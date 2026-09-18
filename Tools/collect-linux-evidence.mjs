#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const OUTPUT_LIMIT = 64 * 1024;
const slash = (value) => value.replaceAll(sep, '/');
const stable = (value) => JSON.stringify(value, null, 2) + '\n';
function display(command, args) { return [command, ...args].map((part) => /^[A-Za-z0-9_./:@+-]+$/.test(part) ? part : JSON.stringify(part)).join(' '); }
function sanitize(text) {
  let output = text.replaceAll(ROOT, '$ROOT'); const home = process.env.HOME; if (home) output = output.replaceAll(home, '$HOME');
  return output.replace(/(password|passwd|client_secret|api_key|access_token)(\s*[:=]\s*)[^\s"']+/gi, '$1$2[REDACTED]').replace(/-----BEGIN [^-]+ PRIVATE KEY-----[\s\S]*?-----END [^-]+ PRIVATE KEY-----/g, '[REDACTED PRIVATE KEY]');
}
async function run(spec) {
  const started = performance.now(); const stdoutHash = createHash('sha256'); const stderrHash = createHash('sha256'); const stdout = []; const stderr = []; let stdoutBytes = 0; let stderrBytes = 0;
  const append = (chunks, chunk, current) => { const remaining = OUTPUT_LIMIT - current; if (remaining > 0) chunks.push(chunk.subarray(0, remaining)); return current + chunk.length; };
  return await new Promise((finish) => {
    const child = spawn(spec.command, spec.args, { cwd: join(ROOT, spec.cwd), shell: false, env: { ...process.env, CI: '1', NO_COLOR: '1', LANG: 'C', LC_ALL: 'C', TZ: 'UTC' }, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => { stdoutHash.update(chunk); stdoutBytes = append(stdout, chunk, stdoutBytes); }); child.stderr.on('data', (chunk) => { stderrHash.update(chunk); stderrBytes = append(stderr, chunk, stderrBytes); });
    let spawnError;
    child.once('error', (error) => { spawnError = error; });
    child.once('close', (exitCode, signal) => finish({ id: spec.id, category: spec.category, command: display(spec.command, spec.args), cwd: spec.cwd, required: true, status: spawnError?.code === 'ENOENT' ? 'fail' : exitCode === 0 ? 'pass' : 'fail', exitCode, signal, durationMs: Math.round(performance.now() - started), unavailable: spawnError?.code === 'ENOENT', error: spawnError ? sanitize(spawnError.message).slice(0, 1024) : undefined, stdout: { sha256: stdoutHash.digest('hex'), bytes: stdoutBytes, capturedBytes: Math.min(stdoutBytes, OUTPUT_LIMIT), truncated: stdoutBytes > OUTPUT_LIMIT, text: sanitize(Buffer.concat(stdout).toString('utf8')) }, stderr: { sha256: stderrHash.digest('hex'), bytes: stderrBytes, capturedBytes: Math.min(stderrBytes, OUTPUT_LIMIT), truncated: stderrBytes > OUTPUT_LIMIT, text: sanitize(Buffer.concat(stderr).toString('utf8')) } }));
  });
}
const COMMANDS = [
  { id: 'release-surface-scan', category: 'static-validator', command: 'node', args: ['Tools/verify-release-surface.mjs'], cwd: '.' },
  { id: 'swift-core-tests', category: 'test', command: 'swift', args: ['test'], cwd: 'Packages/TexCore' },
  { id: 'swift-app-tests', category: 'test', command: 'swift', args: ['test'], cwd: 'Packages/TexApp' },
  { id: 'tex-matrix', category: 'fixture', command: 'node', args: ['Tools/run-tex-fixtures.mjs'], cwd: '.' },
  { id: 'synctex-fixture', category: 'fixture', command: 'node', args: ['Tools/run-synctex-fixture.mjs'], cwd: '.' },
  { id: 'target-dag-validator', category: 'static-validator', command: 'node', args: ['Tools/verify-target-dag.mjs'], cwd: '.' },
  { id: 'rust-dag-validator', category: 'static-validator', command: 'node', args: ['Tools/verify-rust-dag.mjs'], cwd: '.' },
  { id: 'rust-tests', category: 'test', command: 'cargo', args: ['test', '--workspace'], cwd: 'Linux' },
  { id: 'parity-validator', category: 'static-validator', command: 'node', args: ['Tools/validate-parity.mjs'], cwd: '.' },
  { id: 'xcode-source-validator', category: 'static-validator', command: 'node', args: ['Tools/validate-xcodeproj.mjs'], cwd: '.' }
];
const VERSION_COMMANDS = [
  { id: 'node', command: 'node', args: ['--version'], cwd: '.' }, { id: 'npm', command: 'npm', args: ['--version'], cwd: '.' }, { id: 'swift', command: 'swift', args: ['--version'], cwd: '.' },
  { id: 'cargo', command: 'cargo', args: ['--version'], cwd: '.' }, { id: 'rustc', command: 'rustc', args: ['--version'], cwd: '.' },
  { id: 'latexmk', command: 'latexmk', args: ['--version'], cwd: '.' }, { id: 'pdflatex', command: 'pdflatex', args: ['--version'], cwd: '.' }, { id: 'xelatex', command: 'xelatex', args: ['--version'], cwd: '.' }, { id: 'lualatex', command: 'lualatex', args: ['--version'], cwd: '.' }, { id: 'synctex', command: 'synctex', args: ['help'], cwd: '.' }
];
export async function collectLinuxEvidence() {
  const toolVersions = [];
  for (const spec of VERSION_COMMANDS) { const result = await run({ ...spec, category: 'tool-version' }); toolVersions.push({ tool: spec.id, status: result.status, unavailable: result.unavailable, exitCode: result.exitCode, stdout: result.stdout, stderr: result.stderr }); }
  const commands = []; for (const spec of COMMANDS) commands.push(await run(spec));
  const failures = [...commands.filter((item) => item.status !== 'pass'), ...toolVersions.filter((item) => item.status !== 'pass')];
  return { schemaVersion: 1, reportKind: 'provisional-linux-p0', platform: { os: process.platform, architecture: process.arch }, provisional: true, completenessClaimed: false, deterministicFields: 'The collector adds no wall-clock timestamps, hostnames, usernames, environment values, or absolute paths; command output and measured durations remain observed evidence.', outputCaptureLimitBytes: OUTPUT_LIMIT, status: failures.length ? 'fail' : 'pass', toolVersions, commands, appleOnlySlots: [
    { id: 'macos-xcode-build', status: 'skipped', requiredForTerminalEvidence: true, reason: 'Apple Xcode is unavailable on canonical Linux; no build result is inferred.' },
    { id: 'macos-ui-accessibility', status: 'skipped', requiredForTerminalEvidence: true, reason: 'AppKit, PDFKit, Keychain, VoiceOver, menus, panels, and native document workflows require physical Mac evidence.' },
    { id: 'macos-signing-notarization', status: 'skipped', requiredForTerminalEvidence: false, reason: 'Public distribution, signing, and notarization are out of domain.' }
  ] };
}
async function main() {
  const args = process.argv.slice(2); let output = join(ROOT, 'Evidence/linux-p0.json');
  if (args.length) { if (args.length !== 2 || args[0] !== '--output') throw new Error('usage: collect-linux-evidence.mjs [--output PATH]'); output = resolve(ROOT, args[1]); }
  const rel = relative(ROOT, output); if (rel === '..' || rel.startsWith(`..${sep}`)) throw new Error('output must be inside repository root');
  const report = await collectLinuxEvidence(); await mkdir(dirname(output), { recursive: true }); await writeFile(output, stable(report), { mode: 0o644 }); process.stdout.write(`${slash(relative(ROOT, output))} ${report.status}\n`); if (report.status !== 'pass') process.exitCode = 1;
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
