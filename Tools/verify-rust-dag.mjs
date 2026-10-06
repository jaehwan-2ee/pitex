#!/usr/bin/env node
// Static DAG gate for the Rust port — mirrors verify-target-dag.mjs for the
// Swift packages. Validates that Linux/Cargo.toml workspace members and each
// member's path dependencies match Tools/rust-target-dag.json exactly, that
// the crate graph is acyclic, that portable crates never reach GUI/system
// dependencies, and that the `pitex` binary composes only `pitex-shell`.
import { readFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const TOOL_DIR = dirname(fileURLToPath(import.meta.url));
const DEFAULT_ROOT = resolve(TOOL_DIR, '..');

async function read(path) {
  return await readFile(path, 'utf8');
}

function add(violations, code, path, message) {
  violations.push({ code, path: path.replaceAll(sep, '/'), message });
}

// Minimal TOML-aware-enough dependency scanner: tracks [dependencies],
// [dev-dependencies], [build-dependencies], and target-scoped
// [target.'cfg(...)'.dependencies] sections and captures entries
// of the forms `name = { path = "../x" }`, `name = { version = ... }`, and
// `name.workspace = true`.
function parseCargoToml(source) {
  const sections = { dependencies: [], 'dev-dependencies': [], 'build-dependencies': [], 'target-dependencies': [] };
  let section = null;
  for (const rawLine of source.split('\n')) {
    const line = rawLine.replace(/#.*$/, '').trim();
    const header = line.match(/^\[\s*([^\]]+)\s*\]/);
    if (header) {
      const name = header[1].trim();
      if (Object.prototype.hasOwnProperty.call(sections, name)) section = name;
      else if (/^target\..*\.dependencies$/.test(name)) section = 'target-dependencies';
      else section = null;
      continue;
    }
    if (!section || !line) continue;
    const key = line.match(/^([A-Za-z0-9_-]+)(?:\.\w+)?\s*=/);
    if (!key) continue;
    const pathMatch = line.match(/\bpath\s*=\s*"([^"]+)"/);
    sections[section].push({ name: key[1], path: pathMatch ? pathMatch[1] : null });
  }
  return sections;
}

function workspaceMembers(source) {
  const block = source.match(/^\s*members\s*=\s*\[([\s\S]*?)\]/m);
  if (!block) return null;
  return [...block[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
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

export async function verifyRustDag({ root = DEFAULT_ROOT } = {}) {
  root = resolve(root);
  const rustRoot = join(root, 'Linux');
  const dagPath = join(root, 'Tools', 'rust-target-dag.json');
  const violations = [];
  const evidence = ['Tools/rust-target-dag.json', 'Linux/Cargo.toml'];

  let dag;
  try { dag = JSON.parse(await read(dagPath)); } catch (error) {
    add(violations, 'DAG_MALFORMED', relative(root, dagPath), error.message);
    return report(violations, evidence);
  }

  // ── Workspace member set ────────────────────────────────────────────────
  const configured = new Map(
    Object.entries(dag.targets ?? {}).map(([name, target]) => [
      name,
      target?.dependencies ?? []
    ])
  );
  const memberDirs = new Map(); // crate name -> directory
  let members = null;
  try {
    members = workspaceMembers(await read(join(rustRoot, 'Cargo.toml')));
  } catch (error) {
    add(violations, 'MANIFEST_MALFORMED', 'Linux/Cargo.toml', error.code === 'ENOENT' ? 'workspace manifest is missing' : error.message);
  }
  if (members) {
    for (const dir of members) memberDirs.set(dir.split('/').pop(), dir);
    const expected = [...configured.keys()].sort();
    const actual = [...memberDirs.keys()].sort();
    if (JSON.stringify(expected) !== JSON.stringify(actual)) {
      add(violations, 'DAG_TARGET_SET', 'Linux/Cargo.toml', `workspace members [${actual}] do not match the approved ${expected.length} crates`);
    }
  } else {
    add(violations, 'DAG_TARGET_SET', 'Linux/Cargo.toml', 'workspace members could not be parsed');
  }

  for (const [name, deps] of configured) {
    if (!Array.isArray(deps) || new Set(deps).size !== deps.length) {
      add(violations, 'DAG_DEPENDENCIES', 'Tools/rust-target-dag.json', `${name} dependencies must be a unique array`);
    }
    for (const dep of deps) if (!configured.has(dep)) {
      add(violations, 'DAG_UNKNOWN_EDGE', 'Tools/rust-target-dag.json', `${name} references unknown crate ${dep}`);
    }
  }
  for (const cycle of cycles(configured)) add(violations, 'DAG_CYCLE', 'Tools/rust-target-dag.json', cycle);

  // ── Per-crate edge validation ───────────────────────────────────────────
  const guiDeps = new Set(dag.guiDependencies ?? []);
  const nonPortable = new Set(dag.nonPortableTargets ?? []);
  const externalEdges = new Map(
    Object.entries(dag.externalPathDependencies ?? {}).map(([name, deps]) => [name, deps ?? []])
  );
  for (const [name, deps] of externalEdges) {
    if (!configured.has(name)) add(violations, 'DAG_UNKNOWN_EDGE', 'Tools/rust-target-dag.json', `externalPathDependencies references unknown crate ${name}`);
    if (new Set(deps).size !== deps.length) add(violations, 'DAG_DEPENDENCIES', 'Tools/rust-target-dag.json', `${name} externalPathDependencies must be a unique array`);
    for (const dep of deps) if (configured.has(dep)) add(violations, 'DAG_DEPENDENCIES', 'Tools/rust-target-dag.json', `${name}: ${dep} is a workspace member and must use targets dependencies instead`);
  }
  for (const [name, dir] of [...memberDirs.entries()].sort()) {
    const manifest = join(rustRoot, dir, 'Cargo.toml');
    const rel = `Linux/${dir}/Cargo.toml`;
    evidence.push(rel);
    let parsed;
    try { parsed = parseCargoToml(await read(manifest)); } catch (error) {
      add(violations, 'MANIFEST_MALFORMED', rel, error.code === 'ENOENT' ? 'crate manifest is missing' : error.message);
      continue;
    }
    const expected = [...(configured.get(name) ?? [])].sort();
    const external = new Set(externalEdges.get(name) ?? []);
    const internal = [...parsed.dependencies, ...parsed['target-dependencies']]
      .filter((dep) => dep.path?.startsWith('../'))
      .map((dep) => dep.path.split('/').pop());
    const uniqueInternal = [...new Set(internal)].sort();
    const workspaceInternal = uniqueInternal.filter((edge) => !external.has(edge));
    if (internal.length !== uniqueInternal.length) {
      add(violations, 'DUPLICATE_EDGE', rel, `${name} declares a duplicate internal dependency`);
    }
    for (const edge of external) {
      if (!uniqueInternal.includes(edge)) add(violations, 'EXTERNAL_EDGE_MISSING', rel, `${name}: allowed external path dependency ${edge} is not declared`);
    }
    if (JSON.stringify(expected) !== JSON.stringify(workspaceInternal)) {
      add(violations, 'EDGE_MISMATCH', rel, `${name}: expected [${expected}], declared [${workspaceInternal}]`);
    }
    for (const edge of workspaceInternal) {
      if (!configured.has(edge)) add(violations, 'DAG_UNKNOWN_EDGE', rel, `${name} depends on non-workspace path crate ${edge}`);
      else if (!configured.get(name).includes(edge)) add(violations, 'FORBIDDEN_EDGE', rel, `${name} -> ${edge}`);
    }

    if (!nonPortable.has(name)) {
      // Portable crates must not reach GUI/system crates — directly via any
      // dependency section, or transitively via an edge to a non-portable
      // crate (mirrors the Swift appleOnlyModules confinement check).
      for (const section of Object.values(parsed)) {
        for (const dep of section) {
          if (guiDeps.has(dep.name)) add(violations, 'GUI_DEPENDENCY', rel, `portable crate ${name} depends on ${dep.name}`);
          if (dep.path?.startsWith('../') && nonPortable.has(dep.path.split('/').pop())) {
            add(violations, 'PORTABILITY_EDGE', rel, `portable crate ${name} depends on non-portable crate ${dep.path.split('/').pop()}`);
          }
        }
      }
    }
  }

  // ── Composition root: the `pitex` binary composes only `pitex-shell` ────
  const appTarget = dag.appTarget;
  if (appTarget && configured.has(appTarget)) {
    const appDeps = configured.get(appTarget);
    if (appDeps.length !== 1 || appDeps[0] !== dag.compositionRoot) {
      add(violations, 'COMPOSITION_ROOT', 'Tools/rust-target-dag.json', `${appTarget} must depend solely on ${dag.compositionRoot}`);
    }
  }

  return report(violations, [...new Set(evidence)].sort());
}

function report(violations, evidence) {
  violations.sort((a, b) => `${a.code}:${a.path}:${a.message}`.localeCompare(`${b.code}:${b.path}:${b.message}`));
  return {
    tool: 'verify-rust-dag',
    status: violations.length === 0 ? 'pass' : 'fail',
    checks: [{ command: 'static Rust manifest DAG validation', result: violations.length === 0 ? 'pass' : 'fail', evidence }],
    violations
  };
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const root = process.argv[2] ? resolve(process.argv[2]) : DEFAULT_ROOT;
  try {
    const result = await verifyRustDag({ root });
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    if (result.status !== 'pass') process.exitCode = 1;
  } catch (error) {
    process.stdout.write(`${JSON.stringify({ tool: 'verify-rust-dag', status: 'fail', checks: [], violations: [{ code: 'INTERNAL_ERROR', path: '', message: error.message }] }, null, 2)}\n`);
    process.exitCode = 1;
  }
}
