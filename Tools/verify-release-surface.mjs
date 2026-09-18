#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { lstat, readFile, readdir, stat } from 'node:fs/promises';
import { dirname, extname, join, relative, resolve, sep } from 'node:path';
import { gunzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const slash = (value) => value.replaceAll(sep, '/');
const ignored = new Set(['.git', '.gjc', '.codegraph', 'node_modules', 'ReferenceEvidence']);
const buildDirectories = new Set(['dist', '.build', 'build', 'DerivedData', 'coverage', 'target']);
const binaryExtensions = new Set(['.o', '.a', '.so', '.dylib', '.class', '.pyc', '.app', '.xctest', '.exe']);
const violations = [];
function fail(code, path, message) { violations.push({ code, path: slash(path), message }); }
function probablyText(buffer) { if (buffer.includes(0)) return false; let controls = 0; for (const byte of buffer.subarray(0, 8192)) if (byte < 9 || (byte > 13 && byte < 32)) controls++; return controls <= Math.max(4, buffer.length * 0.01); }
async function exists(path) { try { return (await stat(path)).isFile(); } catch { return false; } }
async function walk(path, output = []) {
  for (const entry of (await readdir(path, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
    if (ignored.has(entry.name)) continue; const child = join(path, entry.name); const rel = slash(relative(ROOT, child));
    if (entry.isSymbolicLink()) { fail('SYMLINK', rel, 'release source must not contain symlinks'); continue; }
    if (entry.isDirectory()) { if (buildDirectories.has(entry.name)) { fail('BUILD_OUTPUT', rel, 'unexpected build-output directory'); continue; } await walk(child, output); }
    else if (entry.isFile()) output.push({ absolute: child, relative: rel });
  }
  return output;
}
function scanForbidden(files) {
  const product = /^(Mac|Packages|Linux)\//; const client = /^(Mac|Packages|Linux)\//;
  const productPatterns = [
    ['UPDATER_SURFACE', /\b(?:Sparkle|SUUpdater|SPU(?:Standard)?Updater|checkForUpdates)\b/],
    ['PUBLIC_DISTRIBUTION', /\b(?:notarytool|altool|Developer ID Application|releaseChannel|distributionURL)\b/i],
    ['APP_SANDBOX', /(?:ENABLE_APP_SANDBOX\s*=\s*YES|com\.apple\.security\.app-sandbox[\s\S]{0,160}<true\s*\/>)/i],
    ['INTEL_SURFACE', /(?:ARCHS|VALID_ARCHS|SUPPORTED_ARCHS)\s*=\s*[^\n]*(?:x86_64|i386)/i]
  ];
  const secrets = [
    ['PRIVATE_KEY', /-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/], ['AWS_KEY', /\bAKIA[0-9A-Z]{16}\b/], ['TOKEN', /\bgh[opusr]_[A-Za-z0-9]{30,255}\b/], ['ASSIGNED_SECRET', /\b(?:password|passwd|client_secret|api_key|access_token)\s*[:=]\s*["'][^"'\s]{12,}["']/i]
  ];
  for (const file of files) {
    if (binaryExtensions.has(extname(file.relative).toLowerCase())) fail('UNEXPECTED_BINARY', file.relative, 'compiled binary is not a release-source input');
    if (!file.text) continue;
    const testSource = /(?:^|\/)(?:[Tt]ests?)(?:\/|$)/.test(file.relative);
    if (product.test(file.relative) && !testSource) for (const [code, pattern] of productPatterns) if (pattern.test(file.text)) fail(code, file.relative, 'forbidden release surface found in product source/configuration');
    if (client.test(file.relative) && !testSource && /(?:TextField|SecureField)\s*\([\s\S]{0,240}\b(?:API[_ -]?key|apiKey|clientSecret)\b/i.test(file.text)) fail('API_KEY_CLIENT_SURFACE', file.relative, 'client API-key or client-secret input surface is forbidden');
    if (product.test(file.relative) && !testSource && /ReferenceEvidence|reference[-_ ]?(?:capture|download|asset)|copied[-_ ]asset/i.test(file.text)) fail('REFERENCE_CONTAMINATION', file.relative, 'product source/configuration references quarantined reference evidence');
    if (product.test(file.relative) && !testSource) for (const [code, pattern] of secrets) if (pattern.test(file.text)) fail(code, file.relative, 'possible committed secret');
  }
}
async function validateRequired(files, options) {
  for (const file of files.filter((entry) => entry.relative.endsWith('/Package.swift'))) if (/\.package\s*\(\s*url\s*:/.test(file.text ?? '') && !files.some((entry) => entry.relative.endsWith('Package.resolved'))) fail('MISSING_SWIFT_LOCK', file.relative, 'Swift external dependencies require Package.resolved');
  try { const sbom = JSON.parse(await readFile(options.sbom, 'utf8')); if (sbom.bomFormat !== 'CycloneDX' || !Array.isArray(sbom.components) || !sbom.components.length) fail('INVALID_SBOM', slash(relative(ROOT, options.sbom)), 'CycloneDX SBOM has no components'); } catch (error) { fail('MISSING_SBOM', slash(relative(ROOT, options.sbom)), error.code === 'ENOENT' ? 'SBOM is required' : `SBOM is invalid: ${error.message}`); }
  const projectPath = join(ROOT, 'Mac/Pitex.xcodeproj/project.pbxproj'); let project = ''; try { project = await readFile(projectPath, 'utf8'); } catch { fail('MISSING_XCODE_PROJECT', slash(relative(ROOT, projectPath)), 'Xcode source project is required'); }
  const localization = files.filter((entry) => /Mac\/Resources\/[^/]+\.lproj\/(?:Localizable\.strings|Localizable\.xcstrings)$/.test(entry.relative));
  if (!localization.length) fail('MISSING_LOCALIZATION', 'Mac/Resources', 'at least one lproj localization catalog is required');
  const resources = files.filter((entry) => entry.relative.startsWith('Mac/Resources/') && !entry.relative.endsWith('/.DS_Store'));
  for (const resource of resources) { const membershipName = resource.relative.match(/\.lproj\/([^/]+)$/)?.[1] ?? resource.relative.match(/Mac\/Resources\/([^/]+\.xcassets)(?:\/|$)/)?.[1] ?? resource.relative.match(/Mac\/Resources\/([^/]+)$/)?.[1]; if (membershipName && !project.includes(membershipName)) fail('RESOURCE_MEMBERSHIP', resource.relative, 'resource is not referenced by the Xcode project'); }
  const swiftSources = files.filter((entry) => /^(Mac|Packages)\/.*\.swift$/.test(entry.relative)).map((entry) => entry.text).join('\n');
  if (!/\.accessibility(?:Label|Identifier|Hint|Value|Element)\s*\(/.test(swiftSources)) fail('MISSING_ACCESSIBILITY', 'Mac/Sources', 'native source has no explicit accessibility semantics');
  let config = ''; try { config = await readFile(join(ROOT, 'Mac/Config/Base.xcconfig'), 'utf8'); } catch {}
  const deployment = Number(config.match(/^MACOSX_DEPLOYMENT_TARGET\s*=\s*([0-9.]+)/m)?.[1]); if (!Number.isFinite(deployment) || deployment < 15) fail('DEPLOYMENT_TARGET', 'Mac/Config/Base.xcconfig', 'macOS deployment target must be 15 or newer');
  const manifest = await readFile(join(ROOT, 'Parity/manifest.yaml'), 'utf8'); const activeIds = [...manifest.matchAll(/^  - id:\s*(p0\.(project|editor|build|pdf|ai|macos)\.[^\s]+)[\s\S]*?^\s+taxonomy:\s*active-parity$/gm)].map((match) => match[1]);
  const ownedPrefixes = ['p0.project.', 'p0.editor.', 'p0.build.', 'p0.pdf.', 'p0.ai.', 'p0.macos.']; for (const id of activeIds) if (!ownedPrefixes.some((prefix) => id.startsWith(prefix))) fail('MANIFEST_OWNERSHIP', 'Parity/manifest.yaml', `active item lacks explicit owner mapping: ${id}`);
}
function tarNumber(buffer, start, length) { const value = buffer.subarray(start, start + length).toString('ascii').replaceAll('\0', '').trim(); return value ? Number.parseInt(value, 8) : 0; }
async function validateBundle(bundlePath) {
  if (!await exists(bundlePath)) return;
  const rel = slash(relative(ROOT, bundlePath)); const archive = await readFile(bundlePath); if (archive.readUInt32LE(4) !== 0) fail('NONDETERMINISTIC_GZIP', rel, 'gzip mtime must be zero');
  let tar; try { tar = gunzipSync(archive); } catch (error) { fail('INVALID_BUNDLE', rel, error.message); return; }
  const entries = []; for (let offset = 0; offset + 512 <= tar.length;) { const header = tar.subarray(offset, offset + 512); if (header.every((byte) => byte === 0)) break; const name = header.subarray(0, 100).toString('utf8').replace(/\0.*$/, ''); const prefix = header.subarray(345, 500).toString('utf8').replace(/\0.*$/, ''); const path = prefix ? `${prefix}/${name}` : name; const size = tarNumber(header, 124, 12); const mode = tarNumber(header, 100, 8); const uid = tarNumber(header, 108, 8); const gid = tarNumber(header, 116, 8); const mtime = tarNumber(header, 136, 12); if (mode !== 0o644 || uid !== 0 || gid !== 0 || mtime !== 0) fail('NONDETERMINISTIC_TAR', path, 'tar mode/owner/mtime is not normalized'); entries.push(path); offset += 512 + Math.ceil(size / 512) * 512; }
  if (entries.some((entry, index) => index && entries[index - 1].localeCompare(entry, 'en') > 0)) fail('NONDETERMINISTIC_ORDER', rel, 'tar paths are not lexically ordered');
  try { const sidecar = JSON.parse(await readFile(`${bundlePath}.manifest.json`, 'utf8')); const hash = createHash('sha256').update(archive).digest('hex'); if (sidecar.archiveSha256 !== hash || JSON.stringify(sidecar.files?.map((item) => item.path)) !== JSON.stringify(entries)) fail('BUNDLE_MANIFEST_MISMATCH', `${rel}.manifest.json`, 'bundle checksum or included paths differ from manifest'); } catch (error) { fail('MISSING_BUNDLE_MANIFEST', `${rel}.manifest.json`, error.message); }
}
export async function verifyReleaseSurface(options = {}) {
  violations.length = 0; const files = await walk(ROOT); for (const file of files) { const metadata = await stat(file.absolute); if (metadata.size > 5 * 1024 * 1024) { file.text = null; continue; } const bytes = await readFile(file.absolute); file.text = probablyText(bytes) ? bytes.toString('utf8') : null; }
  scanForbidden(files); await validateRequired(files, { sbom: options.sbom ?? join(ROOT, 'Evidence/sbom.cdx.json') }); await validateBundle(options.bundle ?? join(ROOT, 'Artifacts/source-bundle.tar.gz'));
  violations.sort((a, b) => `${a.code}:${a.path}`.localeCompare(`${b.code}:${b.path}`)); return { tool: 'verify-release-surface', status: violations.length ? 'fail' : 'pass', checks: { scannedFiles: files.length, networkUsed: false }, violations: [...violations] };
}
async function main() { const args = process.argv.slice(2); const options = {}; for (let i = 0; i < args.length; i += 2) { if (!['--sbom', '--bundle'].includes(args[i]) || !args[i + 1]) throw new Error('usage: verify-release-surface.mjs [--sbom PATH] [--bundle PATH]'); options[args[i].slice(2)] = resolve(ROOT, args[i + 1]); } const report = await verifyReleaseSurface(options); process.stdout.write(`${JSON.stringify(report, null, 2)}\n`); if (report.status !== 'pass') process.exitCode = 1; }
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
