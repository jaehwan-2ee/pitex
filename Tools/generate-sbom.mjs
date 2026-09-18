#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const slash = (value) => value.replaceAll(sep, '/');
const digest = (value) => createHash('sha256').update(value).digest('hex');
const stable = (value) => JSON.stringify(value, (key, item) => item && typeof item === 'object' && !Array.isArray(item) ? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a.localeCompare(b))) : item, 2) + '\n';

async function optionalText(path) { try { return await readFile(path, 'utf8'); } catch (error) { if (error.code === 'ENOENT') return null; throw error; } }
async function swiftComponents() {
  const output = [];
  for (const packageDir of ['Packages/TexCore', 'Packages/TexApp']) {
    const path = join(ROOT, packageDir, 'Package.swift'); const text = await readFile(path, 'utf8');
    const packageName = text.match(/let\s+package\s*=\s*Package\s*\(\s*name:\s*"([^"]+)"/)?.[1] ?? packageDir.split('/').at(-1);
    output.push({ type: 'application', name: packageName, version: 'source', properties: [{ name: 'pitex:manifest', value: `${packageDir}/Package.swift` }, { name: 'pitex:sha256', value: digest(text) }] });
    for (const match of text.matchAll(/\.(?:target|testTarget|executableTarget)\s*\(\s*name:\s*"([^"]+)"/g)) output.push({ type: 'library', name: match[1], version: 'source', group: packageName, properties: [{ name: 'pitex:swift-target', value: match[0].split('(')[0].slice(1) }] });
    for (const match of text.matchAll(/\.package\s*\(\s*url:\s*"([^"]+)"\s*,\s*(?:from|exact):\s*"([^"]+)"/g)) output.push({ type: 'library', name: match[1].split('/').at(-1).replace(/\.git$/, ''), version: match[2], externalReferences: [{ type: 'vcs', url: match[1] }] });
  }
  const resolvedPath = join(ROOT, 'Mac/Pitex.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved');
  const resolved = await optionalText(resolvedPath);
  if (resolved) for (const pin of JSON.parse(resolved).pins ?? []) output.push({ type: 'library', name: pin.identity, version: pin.state?.version ?? pin.state?.revision ?? 'unresolved', externalReferences: pin.location ? [{ type: 'vcs', url: pin.location }] : undefined, properties: [{ name: 'pitex:swift-resolved', value: slash(relative(ROOT, resolvedPath)) }] });
  return output;
}
function toolVersion(command, args = ['--version']) {
  const result = spawnSync(command, args, { cwd: ROOT, shell: false, encoding: 'utf8', timeout: 5000, maxBuffer: 64 * 1024, env: { PATH: process.env.PATH ?? '', LANG: 'C', LC_ALL: 'C', NO_COLOR: '1' } });
  if (result.error?.code === 'ENOENT') return 'unavailable';
  if (result.error || result.status !== 0) return `error:${result.status ?? result.error?.code ?? 'unknown'}`;
  let version = `${result.stdout}\n${result.stderr}`.trim().split(/\r?\n/)[0].slice(0, 256) || 'unknown';
  version = version.replaceAll(ROOT, '$ROOT');
  if (process.env.HOME) version = version.replaceAll(process.env.HOME, '$HOME');
  return version;
}
async function xcodeComponent() {
  const paths = ['Mac/Config/Base.xcconfig', 'Mac/Pitex.xcodeproj/project.pbxproj']; const properties = [];
  for (const path of paths) { const text = await readFile(join(ROOT, path), 'utf8'); properties.push({ name: `pitex:sha256:${path}`, value: digest(text) }); }
  const config = await readFile(join(ROOT, paths[0]), 'utf8');
  for (const key of ['MACOSX_DEPLOYMENT_TARGET', 'SDKROOT', 'SUPPORTED_PLATFORMS', 'SWIFT_VERSION']) { const value = config.match(new RegExp(`^${key}\\s*=\\s*(.+)$`, 'm'))?.[1]?.trim(); if (value) properties.push({ name: `xcode:${key}`, value }); }
  return { type: 'application', name: 'Pitex-Xcode-source-config', version: 'source', properties: properties.sort((a, b) => a.name.localeCompare(b.name)) };
}
export async function generateSbom() {
  const components = [...await swiftComponents(), await xcodeComponent()];
  for (const command of ['latexmk', 'pdflatex', 'xelatex', 'lualatex', 'synctex']) components.push({ type: 'application', name: command, version: toolVersion(command) });
  components.sort((a, b) => `${a.type}:${a.group ?? ''}:${a.name}:${a.version}`.localeCompare(`${b.type}:${b.group ?? ''}:${b.name}:${b.version}`));
  return { bomFormat: 'CycloneDX', specVersion: '1.6', version: 1, metadata: { component: { type: 'application', name: 'Pitex-source', version: 'provisional-g005' }, properties: [{ name: 'pitex:deterministic', value: 'true' }, { name: 'pitex:source-root', value: '.' }] }, components };
}
async function main() {
  const args = process.argv.slice(2); let output = join(ROOT, 'Evidence/sbom.cdx.json');
  if (args.length) { if (args.length !== 2 || args[0] !== '--output') throw new Error('usage: generate-sbom.mjs [--output PATH]'); output = resolve(ROOT, args[1]); }
  if (relative(ROOT, output).startsWith(`..${sep}`) || relative(ROOT, output) === '..') throw new Error('output must be inside repository root');
  await mkdir(dirname(output), { recursive: true }); await writeFile(output, stable(await generateSbom()), { mode: 0o644 });
  process.stdout.write(`${slash(relative(ROOT, output))}\n`);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
