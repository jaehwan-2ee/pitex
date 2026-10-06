#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const slash = (value) => value.replaceAll(sep, '/');
const sha256 = (value) => createHash('sha256').update(value).digest('hex');
const stable = (value) => JSON.stringify(value, null, 2) + '\n';
const PREFIX_MAP = [
  ['p0.project.', 'project-workspace', ['Packages/TexCore/Sources/ProjectCore', 'Packages/TexCore/Sources/DocumentSessionCore', 'Packages/TexApp/Sources/ProjectFeature'], ['Packages/TexCore/Tests/TexCoreTests/ProjectWorkspaceTests.swift', 'Packages/TexCore/Tests/TexCoreTests/DocumentSessionPersistenceTests.swift'], ['swift-core-tests', 'swift-app-tests']],
  ['p0.editor.', 'editor-language', ['Packages/TexCore/Sources/LanguageCore', 'Packages/TexApp/Sources/EditorFeature', 'Packages/TexApp/Sources/EditorMacAdapter'], ['Packages/TexCore/Tests/TexCoreTests/LanguageIndexTests.swift', 'Packages/TexCore/Tests/TexCoreTests/UnicodeCoordinateTests.swift', 'Packages/TexApp/Tests/TexAppTests/EditorAITransactionTests.swift'], ['swift-core-tests', 'swift-app-tests']],
  ['p0.build.', 'build-pipeline', ['Packages/TexCore/Sources/BuildCore', 'Packages/TexApp/Sources/BuildFeature', 'Tools/run-tex-fixtures.mjs', 'Tools/tex-build-matrix.json'], ['Packages/TexCore/Tests/TexCoreTests/BuildOrchestratorTests.swift', 'Packages/TexCore/Tests/TexCoreTests/ProcessRunnerTests.swift', 'Fixtures/process'], ['swift-core-tests', 'swift-app-tests', 'tex-matrix']],
  ['p0.pdf.', 'pdf-synctex', ['Packages/TexCore/Sources/SyncTeXCore', 'Packages/TexApp/Sources/PDFFeature', 'Tools/run-synctex-fixture.mjs'], ['Packages/TexCore/Tests/TexCoreTests/SyncTeXQueryTests.swift', 'Fixtures/synctex'], ['swift-core-tests', 'swift-app-tests', 'synctex-fixture']],
  ['p0.ai.', 'ai-assistant', ['Packages/TexCore/Sources/AICore', 'Mac/Sources/Features'], ['Packages/TexCore/Tests/TexCoreTests/RevisionBoundEditTests.swift', 'Packages/TexApp/Tests/TexAppTests/EditorAITransactionTests.swift', 'Linux/crates/pitex-shell/tests'], ['swift-core-tests', 'swift-app-tests', 'rust-tests']],
  ['p0.macos.', 'native-app', ['Mac/Sources', 'Mac/Resources', 'Mac/Config', 'Packages/TexApp/Sources/MacPlatform', 'Packages/TexApp/Sources/AppShell'], ['Packages/TexApp/Tests'], ['swift-app-tests', 'xcode-source-validator']]
];
function manifestItems(text) {
  const matches = [...text.matchAll(/^  - id:\s*([^\s]+)([\s\S]*?)(?=^  - id:|(?![\s\S]))/gm)];
  return matches.map((match) => ({ id: match[1], taxonomy: match[2].match(/^\s+taxonomy:\s*([^\s]+)$/m)?.[1], provisional: match[2].match(/^\s+provisional:\s*(true|false)$/m)?.[1] !== 'false' }));
}
async function filesUnder(relativePath) {
  const absolute = join(ROOT, relativePath); const output = [];
  async function visit(path) { const metadata = await stat(path); if (metadata.isDirectory()) { for (const name of (await readdir(path)).sort()) await visit(join(path, name)); } else if (metadata.isFile()) output.push(slash(relative(ROOT, path))); }
  try { await visit(absolute); } catch (error) { if (error.code !== 'ENOENT') throw error; }
  return output;
}
async function checksums(paths) {
  const files = []; for (const path of paths) files.push(...await filesUnder(path));
  return await Promise.all([...new Set(files)].sort().map(async (path) => ({ path, sha256: sha256(await readFile(join(ROOT, path))) })));
}
function reportChecks(report, ids) {
  const byId = new Map((report.commands ?? []).map((command) => [command.id, command]));
  return ids.map((id) => { const command = byId.get(id); return command ? { id, status: command.status, command: command.command, stdoutSha256: command.stdout?.sha256, stderrSha256: command.stderr?.sha256 } : { id, status: 'missing' }; });
}
export async function createReceipt({ manifestPath, indexPath, reportPath }) {
  const [manifestBytes, indexBytes, reportBytes] = await Promise.all([readFile(manifestPath), readFile(indexPath), readFile(reportPath)]); const items = manifestItems(manifestBytes.toString('utf8')); const index = JSON.parse(indexBytes); const report = JSON.parse(reportBytes);
  const indexById = new Map((index.entries ?? []).map((entry) => [entry.parity_id, entry])); const manifestIds = new Set(items.map((item) => item.id));
  const errors = [];
  for (const item of items) { const indexed = indexById.get(item.id); if (!indexed) errors.push(`manifest item missing from index: ${item.id}`); else if (indexed.taxonomy !== item.taxonomy) errors.push(`taxonomy mismatch: ${item.id}`); }
  for (const id of indexById.keys()) if (!manifestIds.has(id)) errors.push(`index item missing from manifest: ${id}`);
  if (report.reportKind !== 'provisional-linux-p0' || report.provisional !== true || report.completenessClaimed !== false) errors.push('Linux report is not an explicitly provisional P0 report');
  const attachments = [];
  for (const item of items.filter((entry) => entry.taxonomy === 'active-parity')) {
    const mapping = PREFIX_MAP.find(([prefix]) => item.id.startsWith(prefix)); if (!mapping) { errors.push(`no explicit implementation mapping: ${item.id}`); continue; }
    const [, owner, sources, tests, commandIds] = mapping;
    attachments.push({ parityId: item.id, taxonomy: item.taxonomy, provisional: true, completenessClaimed: false, implementationOwner: owner, sourceChecksums: await checksums(sources), testChecksums: await checksums(tests), evidenceChecksums: [{ path: slash(relative(ROOT, reportPath)), sha256: sha256(reportBytes) }], portableChecks: reportChecks(report, commandIds), macOnlyPlannedSlot: item.id.startsWith('p0.macos.') || item.id === 'p0.editor.native.text-input' || item.id.startsWith('p0.pdf.preview.') ? { status: 'planned', terminalEvidenceAttached: false } : undefined });
  }
  if (errors.length) throw new Error(errors.join('\n'));
  const preserved = items.filter((item) => item.taxonomy !== 'active-parity').map((item) => ({ parityId: item.id, taxonomy: item.taxonomy, disposition: indexById.get(item.id)?.disposition, unchanged: true }));
  return { schemaVersion: 1, receiptKind: 'p0-evidence-attachment', status: report.status, provisional: true, completenessClaimed: false, terminalEvidenceClaimed: false, inputs: [{ path: slash(relative(ROOT, manifestPath)), sha256: sha256(manifestBytes) }, { path: slash(relative(ROOT, indexPath)), sha256: sha256(indexBytes) }, { path: slash(relative(ROOT, reportPath)), sha256: sha256(reportBytes) }], attachments, preservedSemantics: { updaterExclusion: preserved.filter((item) => item.taxonomy === 'excluded-approved'), outOfDomain: preserved.filter((item) => item.taxonomy === 'out-of-domain') }, appleOnlySlots: report.appleOnlySlots ?? [] };
}
async function main() {
  const args = process.argv.slice(2); const values = { '--manifest': 'Parity/manifest.yaml', '--index': 'Parity/evidence-index.json', '--linux-report': 'Evidence/linux-p0.json', '--output': 'Evidence/p0-update-receipt.json' };
  for (let index = 0; index < args.length; index += 2) { if (!(args[index] in values) || !args[index + 1]) throw new Error('usage: update-p0-evidence.mjs [--manifest PATH] [--index PATH] [--linux-report PATH] [--output PATH]'); values[args[index]] = args[index + 1]; }
  const resolved = Object.fromEntries(Object.entries(values).map(([key, value]) => [key, resolve(ROOT, value)])); for (const path of Object.values(resolved)) { const rel = relative(ROOT, path); if (rel === '..' || rel.startsWith(`..${sep}`)) throw new Error('all paths must be inside repository root'); }
  const receipt = await createReceipt({ manifestPath: resolved['--manifest'], indexPath: resolved['--index'], reportPath: resolved['--linux-report'] }); await mkdir(dirname(resolved['--output']), { recursive: true }); await writeFile(resolved['--output'], stable(receipt), { mode: 0o644 }); process.stdout.write(`${slash(relative(ROOT, resolved['--output']))} ${receipt.status}\n`); if (receipt.status !== 'pass') process.exitCode = 1;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
