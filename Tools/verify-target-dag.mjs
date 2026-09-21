#!/usr/bin/env node
import { readFile, readdir } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const TOOL_DIR = dirname(fileURLToPath(import.meta.url));
const DEFAULT_ROOT = resolve(TOOL_DIR, '..');

async function json(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

async function filesBelow(path, suffix) {
  const found = [];
  async function visit(current) {
    let entries;
    try { entries = await readdir(current, { withFileTypes: true }); } catch (error) {
      if (error.code === 'ENOENT') return;
      throw error;
    }
    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      const child = join(current, entry.name);
      if (entry.isDirectory()) await visit(child);
      else if (entry.isFile() && child.endsWith(suffix)) found.push(child);
    }
  }
  await visit(path);
  return found;
}

function balancedCalls(source, callPattern) {
  const calls = [];
  for (const match of source.matchAll(callPattern)) {
    const start = match.index;
    let depth = 0;
    let quote = null;
    let escaped = false;
    let end = -1;
    for (let index = source.indexOf('(', start); index < source.length; index += 1) {
      const character = source[index];
      if (quote) {
        if (escaped) escaped = false;
        else if (character === '\\') escaped = true;
        else if (character === quote) quote = null;
        continue;
      }
      if (character === '"' || character === "'") quote = character;
      else if (character === '(') depth += 1;
      else if (character === ')' && --depth === 0) { end = index + 1; break; }
    }
    if (end < 0) throw new Error(`unterminated target declaration at byte ${start}`);
    calls.push(source.slice(start, end));
  }
  return calls;
}

function parseManifest(source) {
  const targets = new Map();
  const calls = balancedCalls(source, /\.(?:target|executableTarget)\s*\(/g);
  for (const call of calls) {
    const name = call.match(/\bname\s*:\s*"([A-Za-z][A-Za-z0-9_]*)"/s)?.[1];
    if (!name) throw new Error('target declaration has no literal name');
    const dependencyRegion = call.match(/\bdependencies\s*:\s*\[([\s\S]*?)\]/)?.[1] ?? '';
    const dependencies = [...dependencyRegion.matchAll(/"([A-Za-z][A-Za-z0-9_]*)"/g)].map((match) => match[1]);
    if (targets.has(name)) throw new Error(`duplicate target declaration: ${name}`);
    targets.set(name, [...new Set(dependencies)].sort());
  }
  return targets;
}

function cycles(graph) {
  const output = [];
  const state = new Map();
  const stack = [];
  function visit(node) {
    if (state.get(node) === 1) {
      const index = stack.indexOf(node);
      output.push([...stack.slice(index), node].join(' -> '));
      return;
    }
    if (state.get(node) === 2) return;
    state.set(node, 1);
    stack.push(node);
    for (const next of graph.get(node) ?? []) if (graph.has(next)) visit(next);
    stack.pop();
    state.set(node, 2);
  }
  for (const node of [...graph.keys()].sort()) visit(node);
  return [...new Set(output)].sort();
}

function add(violations, code, path, message) {
  violations.push({ code, path: path.replaceAll(sep, '/'), message });
}

export async function verifyTargetDag({ root = DEFAULT_ROOT } = {}) {
  root = resolve(root);
  const dagPath = join(root, 'Tools', 'target-dag.json');
  const violations = [];
  let dag;
  try { dag = await json(dagPath); } catch (error) {
    add(violations, 'DAG_MALFORMED', relative(root, dagPath), error.message);
    return report(violations, [relative(root, dagPath)]);
  }
  const names = Object.keys(dag.targets ?? {}).sort();
  const required = ['TexDomain','ProjectCore','DocumentSessionCore','LanguageCore','BuildCore','SyncTeXCore','AICore','GitCore','ParityKit','AppPorts','ProjectFeature','EditorFeature','EditorMacAdapter','BuildFeature','PDFFeature','SettingsFeature','MacPlatform','AppShell'].sort();
  if (JSON.stringify(names) !== JSON.stringify(required)) add(violations, 'DAG_TARGET_SET', 'Tools/target-dag.json', 'target set does not exactly match the approved 18 targets');
  if (dag.compositionRoot !== 'AppShell') add(violations, 'COMPOSITION_ROOT', 'Tools/target-dag.json', 'AppShell must be the sole configured composition root');
  const configured = new Map(names.map((name) => [name, dag.targets[name]?.dependencies ?? []]));
  for (const [name, deps] of configured) {
    if (!Array.isArray(deps) || new Set(deps).size !== deps.length) add(violations, 'DAG_DEPENDENCIES', 'Tools/target-dag.json', `${name} dependencies must be a unique array`);
    for (const dep of deps) if (!configured.has(dep)) add(violations, 'DAG_UNKNOWN_EDGE', 'Tools/target-dag.json', `${name} references unknown target ${dep}`);
  }
  for (const cycle of cycles(configured)) add(violations, 'DAG_CYCLE', 'Tools/target-dag.json', cycle);

  const packageRoots = [...new Set(names.map((name) => dag.targets[name]?.package).filter(Boolean))].sort();
  const actual = new Map();
  const evidence = ['Tools/target-dag.json'];
  for (const packageRoot of packageRoots) {
    const manifestPath = join(root, packageRoot, 'Package.swift');
    evidence.push(`${packageRoot}/Package.swift`);
    try {
      const parsed = parseManifest(await readFile(manifestPath, 'utf8'));
      for (const [name, deps] of parsed) actual.set(name, { deps, packageRoot });
    } catch (error) {
      add(violations, 'MANIFEST_MALFORMED', `${packageRoot}/Package.swift`, error.code === 'ENOENT' ? 'required manifest is missing' : error.message);
    }
  }
  for (const name of names) {
    const target = actual.get(name);
    if (!target) { add(violations, 'TARGET_MISSING', `${dag.targets[name]?.package}/Package.swift`, `${name} is not declared`); continue; }
    if (target.packageRoot !== dag.targets[name].package) add(violations, 'TARGET_PACKAGE', `${target.packageRoot}/Package.swift`, `${name} is declared in the wrong package`);
    const expected = [...configured.get(name)].sort();
    const internalActual = target.deps.filter((dep) => configured.has(dep)).sort();
    if (JSON.stringify(expected) !== JSON.stringify(internalActual)) add(violations, 'EDGE_MISMATCH', `${target.packageRoot}/Package.swift`, `${name}: expected [${expected}], declared [${internalActual}]`);
  }
  for (const [name, target] of actual) if (configured.has(name)) {
    for (const dep of target.deps) if (configured.has(dep) && !configured.get(name).includes(dep)) add(violations, 'FORBIDDEN_EDGE', `${target.packageRoot}/Package.swift`, `${name} -> ${dep}`);
  }

  const appleTargets = new Set(dag.appleOnlyTargets ?? []);
  const appleModules = new Set(dag.appleOnlyModules ?? []);
  const rootMarkers = [];
  for (const packageRoot of packageRoots) {
    for (const file of await filesBelow(join(root, packageRoot, 'Sources'), '.swift')) {
      const rel = relative(root, file).replaceAll(sep, '/');
      evidence.push(rel);
      const afterSources = relative(join(root, packageRoot, 'Sources'), file).split(sep);
      const target = afterSources[0];
      if (!configured.has(target)) { add(violations, 'UNDECLARED_SOURCE_TARGET', rel, `source directory ${target} is not in the approved DAG`); continue; }
      const source = await readFile(file, 'utf8');
      const imports = [...source.matchAll(/^\s*(?:@\w+(?:\([^\n]*\))?\s+)?import\s+(?:class\s+|struct\s+|enum\s+|protocol\s+|func\s+|var\s+|let\s+|typealias\s+)?([A-Za-z][A-Za-z0-9_]*)/gm)].map((match) => match[1]);
      for (const imported of imports) {
        if (appleModules.has(imported) && !appleTargets.has(target)) add(violations, 'APPLE_IMPORT', rel, `${target} may not import ${imported}`);
        if (configured.has(imported) && imported !== target && !configured.get(target).includes(imported)) add(violations, 'UNDECLARED_IMPORT_EDGE', rel, `${target} imports ${imported} without a declared edge`);
      }
      if (!appleTargets.has(target) && /#if\s+(?:os\(macOS\)|canImport\((?:AppKit|SwiftUI|PDFKit|AuthenticationServices)\))/.test(source)) add(violations, 'APPLE_SOURCE_GROWTH', rel, `${target} contains Apple-only conditional source`);
      if (/@main\b|\b(?:final\s+)?(?:class|struct)\s+\w*CompositionRoot\b|\bAppComposition\b/.test(source)) rootMarkers.push({ target, rel });
    }
  }
  for (const file of await filesBelow(join(root, 'Mac', 'Sources'), '.swift')) {
    const rel = relative(root, file).replaceAll(sep, '/');
    evidence.push(rel);
    const source = await readFile(file, 'utf8');
    if (/@main\b|\bAppShell\.make\s*\(/.test(source)) rootMarkers.push({ target: 'AppShell', rel });
  }
  for (const marker of rootMarkers) if (marker.target !== dag.compositionRoot) add(violations, 'DUPLICATE_COMPOSITION_ROOT', marker.rel, `composition marker exists in ${marker.target}`);
  if (rootMarkers.length === 0) add(violations, 'COMPOSITION_ROOT_MISSING', 'Mac/Sources/AppShell', 'no AppShell composition entry point was found');

  return report(violations, [...new Set(evidence)].sort());
}

function report(violations, evidence) {
  violations.sort((a, b) => `${a.code}:${a.path}:${a.message}`.localeCompare(`${b.code}:${b.path}:${b.message}`));
  return {
    tool: 'verify-target-dag',
    status: violations.length === 0 ? 'pass' : 'fail',
    checks: [{ command: 'static Swift manifest/source DAG validation', result: violations.length === 0 ? 'pass' : 'fail', evidence }],
    violations
  };
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const root = process.argv[2] ? resolve(process.argv[2]) : DEFAULT_ROOT;
  try {
    const result = await verifyTargetDag({ root });
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    if (result.status !== 'pass') process.exitCode = 1;
  } catch (error) {
    process.stdout.write(`${JSON.stringify({ tool: 'verify-target-dag', status: 'fail', checks: [], violations: [{ code: 'INTERNAL_ERROR', path: '', message: error.message }] }, null, 2)}\n`);
    process.exitCode = 1;
  }
}
