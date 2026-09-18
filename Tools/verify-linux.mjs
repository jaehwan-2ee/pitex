#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { readFile, readdir, stat } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { verifyTargetDag } from './verify-target-dag.mjs';
import { verifyRustDag } from './verify-rust-dag.mjs';
import { validateParity } from './validate-parity.mjs';
import { validateXcodeproj } from './validate-xcodeproj.mjs';

const TOOL_DIR = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(TOOL_DIR, '..');
const ignoredDirectories = new Set(['.git', '.gjc', 'node_modules', '.build', 'dist', 'coverage', 'target', '.codegraph']);
function slash(path) { return path.replaceAll(sep, '/'); }
function violation(code, path, message) { return { code, path: slash(path), message }; }

async function walk(root) {
  const output = [];
  async function visit(current) {
    let entries;
    try { entries = await readdir(current, { withFileTypes: true }); } catch (error) { if (error.code === 'ENOENT') return; throw error; }
    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue;
      const child = join(current, entry.name);
      if (entry.isSymbolicLink()) output.push({ path: child, kind: 'symlink' });
      else if (entry.isDirectory()) await visit(child);
      else if (entry.isFile()) output.push({ path: child, kind: 'file' });
    }
  }
  await visit(root); return output;
}

async function parseJsonGate(root) {
  const violations = [];
  const evidence = [];
  for (const entry of await walk(root)) if (entry.kind === 'file' && entry.path.endsWith('.json')) {
    const rel = slash(relative(root, entry.path)); evidence.push(rel);
    try { JSON.parse(await readFile(entry.path, 'utf8')); } catch (error) { violations.push(violation('MALFORMED_JSON', rel, error.message)); }
  }
  return { tool: 'parse-json', status: violations.length ? 'fail' : 'pass', checks: [{ command: 'JSON.parse repository JSON inputs', result: violations.length ? 'fail' : 'pass', evidence }], violations };
}

function probablyText(buffer) {
  if (buffer.includes(0)) return false;
  let control = 0;
  for (const byte of buffer) if (byte < 9 || (byte > 13 && byte < 32)) control += 1;
  return control / Math.max(buffer.length, 1) < 0.01;
}
async function securityScan(root) {
  const violations = [];
  const evidence = [];
  const secretPatterns = [
    ['PRIVATE_KEY', /-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/],
    ['AWS_ACCESS_KEY', /\bAKIA[0-9A-Z]{16}\b/],
    ['GITHUB_TOKEN', /\bgh[opusr]_[A-Za-z0-9]{30,255}\b/],
    ['GENERIC_SECRET', /\b(?:password|passwd|client_secret|api_key|access_token)\s*[:=]\s*["'][^"'\s]{12,}["']/i]
  ];
  const productRoots = new Set(['Mac','Packages','Linux']);
  for (const entry of await walk(root)) {
    const rel = slash(relative(root, entry.path));
    if (entry.kind === 'symlink') { violations.push(violation('SYMLINK', rel, 'symlinks are not accepted by the deterministic source gate')); continue; }
    const metadata = await stat(entry.path);
    if (metadata.size > 5 * 1024 * 1024) { violations.push(violation('UNSCANNED_LARGE_FILE', rel, 'file exceeds the 5 MiB static scan limit')); continue; }
    const buffer = await readFile(entry.path);
    if (!probablyText(buffer)) continue;
    evidence.push(rel);
    const text = buffer.toString('utf8');
    for (const [code, pattern] of secretPatterns) if (pattern.test(text)) violations.push(violation(code, rel, 'possible committed secret'));
    const top = rel.split('/')[0];
    const productRuntime = productRoots.has(top) && !/(?:^|\/)(?:[Tt]ests?)(?:\/|$)/.test(rel);
    if (productRuntime && /ReferenceEvidence\/|reference[-_ ]?(?:capture|download|asset)|extracted[-_ ]asset/i.test(text)) violations.push(violation('REFERENCE_CONTAMINATION', rel, 'product input refers to quarantined reference evidence'));
    if (productRuntime && /Sparkle|SUUpdater|SPU(?:Standard)?Updater|checkForUpdates|automatic[-_ ]?update/i.test(text)) violations.push(violation('UPDATER_EXCLUSION', rel, 'in-app updater implementation/configuration is forbidden'));
  }
  violations.sort((a, b) => `${a.code}:${a.path}`.localeCompare(`${b.code}:${b.path}`));
  return { tool: 'security-contamination-scan', status: violations.length ? 'fail' : 'pass', checks: [{ command: 'bounded text secret/reference/updater scan', result: violations.length ? 'fail' : 'pass', evidence: evidence.sort() }], violations };
}

async function run(command, args, cwd) {
  return await new Promise((resolveResult) => {
    const child = spawn(command, args, { cwd, shell: false, stdio: 'ignore', env: { ...process.env, CI: '1', NO_COLOR: '1' } });
    child.once('error', (error) => resolveResult({ unavailable: error.code === 'ENOENT', exitCode: null, signal: null, error: error.message }));
    child.once('close', (exitCode, signal) => {
      resolveResult({ unavailable: false, exitCode, signal });
    });
  });
}
function commandReport(name, command, args, cwd, result, required = true) {
  const rel = slash(relative(ROOT, cwd)) || '.';
  const commandText = [command, ...args].join(' ');
  const failed = result.unavailable ? required : result.exitCode !== 0;
  const status = result.unavailable ? (required ? 'fail' : 'skip') : (failed ? 'fail' : 'pass');
  const violations = [];
  if (result.unavailable && required) violations.push(violation('REQUIRED_TOOL_UNAVAILABLE', rel, `${command} is required for the full gate`));
  else if (!result.unavailable && result.exitCode !== 0) violations.push(violation('COMMAND_FAILED', rel, `${commandText} exited ${result.exitCode}${result.signal ? ` (${result.signal})` : ''}`));
  return { tool: name, status, checks: [{ command: commandText, result: status, evidence: [rel] }], violations };
}
function skipped(name, command, reason, evidence) {
  return { tool: name, status: 'skip', checks: [{ command, result: 'skip', evidence, reason }], violations: [] };
}

async function compileTestGates(mode) {
  if (mode === 'skeleton') return [
    skipped('swift-tests', 'swift test', 'skeleton mode runs static G001 checks only; future Swift tests are explicit', ['Packages/TexCore/Package.swift','Packages/TexApp/Package.swift']),
    skipped('rust-tests', 'cargo test --workspace', 'skeleton mode runs static G001 checks only; future Rust tests are explicit', ['Linux/Cargo.toml'])
  ];
  const reports = [];
  for (const packagePath of ['Packages/TexCore','Packages/TexApp']) {
    let exists = false;
    try { exists = (await stat(join(ROOT, packagePath, 'Package.swift'))).isFile(); } catch {}
    if (!exists) { reports.push({ tool: `swift-tests:${packagePath}`, status: 'fail', checks: [{ command: 'swift test', result: 'fail', evidence: [`${packagePath}/Package.swift`] }], violations: [violation('PACKAGE_MISSING', `${packagePath}/Package.swift`, 'full gate requires this Swift package')] }); continue; }
    reports.push(commandReport(`swift-tests:${packagePath}`, 'swift', ['test'], join(ROOT, packagePath), await run('swift', ['test'], join(ROOT, packagePath))));
  }
  let rustWorkspace = false;
  try { rustWorkspace = (await stat(join(ROOT, 'Linux', 'Cargo.toml'))).isFile(); } catch {}
  if (!rustWorkspace) {
    reports.push({ tool: 'rust-tests', status: 'fail', checks: [{ command: 'cargo test --workspace', result: 'fail', evidence: ['Linux/Cargo.toml'] }], violations: [violation('WORKSPACE_MISSING', 'Linux/Cargo.toml', 'full gate requires the Rust workspace')] });
  } else {
    reports.push(commandReport('rust-tests', 'cargo', ['test', '--workspace'], join(ROOT, 'Linux'), await run('cargo', ['test', '--workspace'], join(ROOT, 'Linux'))));
  }
  return reports;
}

async function main() {
  const args = process.argv.slice(2);
  const unknown = args.filter((arg) => arg !== '--skeleton');
  const mode = args.includes('--skeleton') ? 'skeleton' : 'full';
  const reports = [];
  if (unknown.length) reports.push({ tool: 'arguments', status: 'fail', checks: [], violations: [violation('UNKNOWN_ARGUMENT', '', unknown.join(', '))] });
  const major = Number(process.versions.node.split('.')[0]);
  reports.push({ tool: 'runtime', status: major >= 24 ? 'pass' : 'fail', checks: [{ command: 'node runtime version >= 24', result: major >= 24 ? 'pass' : 'fail', evidence: ['Tools/verify-linux.mjs'] }], violations: major >= 24 ? [] : [violation('NODE_VERSION', 'Tools/verify-linux.mjs', `Node 24+ required; found ${process.versions.node}`)] });
  for (const action of [verifyTargetDag, verifyRustDag, validateParity, validateXcodeproj]) {
    try { reports.push(await action({ root: ROOT })); } catch (error) { reports.push({ tool: action.name, status: 'fail', checks: [], violations: [violation('INTERNAL_ERROR', '', error.message)] }); }
  }
  reports.push(await parseJsonGate(ROOT));
  reports.push(await securityScan(ROOT));
  reports.push(...await compileTestGates(mode));
  const status = reports.some((report) => report.status === 'fail') ? 'fail' : 'pass';
  const output = { tool: 'verify-linux', mode, status, fullGateClaimed: mode === 'full', reports };
  process.stdout.write(`${JSON.stringify(output, null, 2)}\n`);
  if (status !== 'pass') process.exitCode = 1;
}
try { await main(); } catch (error) {
  process.stdout.write(`${JSON.stringify({ tool: 'verify-linux', mode: process.argv.includes('--skeleton') ? 'skeleton' : 'full', status: 'fail', fullGateClaimed: !process.argv.includes('--skeleton'), reports: [{ tool: 'orchestrator', status: 'fail', checks: [], violations: [violation('INTERNAL_ERROR', '', error.message)] }] }, null, 2)}\n`);
  process.exitCode = 1;
}
