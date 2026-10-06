#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { cp, mkdtemp, realpath, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const TOOLS_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(TOOLS_DIRECTORY, '..');
const FIXTURE_SOURCE = join(REPOSITORY_ROOT, 'Fixtures', 'projects', 'multifile');
const COMMAND_TIMEOUT_MS = 120_000;
const OUTPUT_LIMIT = 1_000_000;

function containedPath(root, candidate) {
  const rel = relative(root, candidate);
  return rel === '' || (!rel.startsWith(`..${sep}`) && rel !== '..' && !isAbsolute(rel));
}

function appendBounded(current, chunk) {
  if (current.length >= OUTPUT_LIMIT) return current;
  return current + chunk.toString('utf8').slice(0, OUTPUT_LIMIT - current.length);
}

function runCommand(executable, argv, cwd, timeoutMs = COMMAND_TIMEOUT_MS) {
  return new Promise((resolveResult) => {
    let stdout = '';
    let stderr = '';
    let timedOut = false;
    let settled = false;
    const child = spawn(executable, argv, {
      cwd,
      detached: process.platform !== 'win32',
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    child.stdout.on('data', (chunk) => { stdout = appendBounded(stdout, chunk); });
    child.stderr.on('data', (chunk) => { stderr = appendBounded(stderr, chunk); });

    const terminate = (signal) => {
      if (child.pid && process.platform !== 'win32') {
        try { process.kill(-child.pid, signal); return; } catch {}
      }
      try { child.kill(signal); } catch {}
    };

    const timeout = setTimeout(() => {
      timedOut = true;
      terminate('SIGTERM');
      setTimeout(() => terminate('SIGKILL'), 1_000).unref();
    }, timeoutMs);
    timeout.unref();

    const finish = (result) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      resolveResult({ ...result, stdout, stderr, timedOut });
    };

    child.once('error', (error) => finish({ exitCode: null, signal: null, errorCode: error.code ?? 'SPAWN_ERROR' }));
    child.once('close', (exitCode, signal) => finish({ exitCode, signal, errorCode: null }));
  });
}

function requireSuccess(label, result) {
  if (result.exitCode !== 0 || result.timedOut || result.errorCode !== null) {
    throw new Error(`${label} failed`);
  }
}

function firstVersionLine(result) {
  return `${result.stdout}\n${result.stderr}`.split(/\r?\n/u).map((line) => line.trim()).find(Boolean) ?? '';
}

function parseRecords(output, recordKey) {
  const records = [];
  let current = null;
  for (const line of output.split(/\r?\n/u)) {
    const separator = line.indexOf(':');
    if (separator < 1) continue;
    const key = line.slice(0, separator).trim();
    const value = line.slice(separator + 1).trim();
    if (key === recordKey) {
      current = {};
      records.push(current);
    }
    if (current) current[key] = value;
  }
  return records;
}

function normalizedInput(cwd, input) {
  const unquoted = input.replace(/^"|"$/gu, '');
  return resolve(cwd, unquoted);
}

async function requireNonemptyFile(path) {
  const info = await stat(path);
  if (!info.isFile() || info.size === 0) throw new Error(`Expected nonempty output: ${basename(path)}`);
}

async function safeRemove(tempRoot) {
  const canonicalTemp = await realpath(tmpdir());
  const canonicalRoot = await realpath(tempRoot);
  if (dirname(canonicalRoot) !== canonicalTemp || !basename(canonicalRoot).startsWith('texspark-synctex-')) {
    throw new Error('Refusing to remove a directory outside the dedicated temporary root');
  }
  await rm(canonicalRoot, { recursive: true, force: true });
}

async function main() {
  const evidence = {
    schemaVersion: 1,
    fixture: 'multifile',
    tools: [],
    build: { status: 'fail', passes: 0 },
    queries: [],
    cleanup: { tempContained: true, removed: false },
    status: 'pass',
  };
  let failed = false;
  let tempRoot;

  try {
    if (!containedPath(REPOSITORY_ROOT, FIXTURE_SOURCE)) throw new Error('Fixture path escapes repository');
    tempRoot = await mkdtemp(join(tmpdir(), 'texspark-synctex-'));
    const projectRoot = join(tempRoot, 'multifile');
    await cp(FIXTURE_SOURCE, projectRoot, { recursive: true, errorOnExist: true });

    for (const [executable, argv] of [['xelatex', ['--version']], ['synctex', ['--version']]]) {
      const result = await runCommand(executable, argv, projectRoot, 15_000);
      requireSuccess(`${executable} version probe`, result);
      const version = firstVersionLine(result);
      if (version === '') throw new Error(`${executable} returned no version`);
      evidence.tools.push({ executable, version, status: 'pass' });
    }

    const buildArgv = ['-interaction=nonstopmode', '-halt-on-error', '-synctex=1', 'main.tex'];
    for (let pass = 0; pass < 2; pass += 1) {
      const result = await runCommand('xelatex', buildArgv, projectRoot);
      requireSuccess(`xelatex pass ${pass + 1}`, result);
      evidence.build.passes += 1;
    }
    await requireNonemptyFile(join(projectRoot, 'main.pdf'));
    await requireNonemptyFile(join(projectRoot, 'main.synctex.gz'));
    evidence.build.status = 'pass';

    const querySpecifications = [
      { id: 'root', file: 'main.tex', line: 7, column: 1, page: 1 },
      { id: 'intro-child', file: 'sections/intro.tex', line: 2, column: 1, page: 1 },
      { id: 'details-child', file: 'sections/details.tex', line: 2, column: 1, page: 1 },
    ];

    for (const specification of querySpecifications) {
      const inputPath = join(projectRoot, specification.file);
      const viewArgv = ['view', '-i', `${specification.line}:${specification.column}:${inputPath}`, '-o', 'main.pdf'];
      const view = await runCommand('synctex', viewArgv, projectRoot, 30_000);
      requireSuccess(`synctex view ${specification.id}`, view);
      const viewRecords = parseRecords(view.stdout, 'Page');
      if (viewRecords.length !== 1) throw new Error(`synctex view ${specification.id} returned ${viewRecords.length} blocks`);
      const forward = viewRecords[0];
      const page = Number.parseInt(forward.Page, 10);
      const x = Number.parseFloat(forward.x);
      const y = Number.parseFloat(forward.y);
      if (page !== specification.page || !Number.isFinite(x) || !Number.isFinite(y)) {
        throw new Error(`synctex view ${specification.id} returned an invalid block`);
      }

      const editArgv = ['edit', '-o', `${page}:${x}:${y}:main.pdf`];
      const edit = await runCommand('synctex', editArgv, projectRoot, 30_000);
      requireSuccess(`synctex edit ${specification.id}`, edit);
      const editRecords = parseRecords(edit.stdout, 'Input');
      if (editRecords.length !== 1) throw new Error(`synctex edit ${specification.id} returned ${editRecords.length} blocks`);
      const reverse = editRecords[0];
      const reverseLine = Number.parseInt(reverse.Line, 10);
      const reversePath = normalizedInput(projectRoot, reverse.Input);
      if (reversePath !== resolve(inputPath) || reverseLine !== specification.line) {
        throw new Error(`synctex edit ${specification.id} did not map to the exact source location`);
      }

      evidence.queries.push({
        id: specification.id,
        input: specification.file,
        line: specification.line,
        page,
        viewBlocks: 1,
        editBlocks: 1,
        reverseInput: specification.file,
        reverseLine,
        ambiguousResults: 0,
        falsePositiveResults: 0,
        status: 'pass',
      });
    }
  } catch (error) {
    failed = true;
    evidence.error = error instanceof Error ? error.message : String(error);
  } finally {
    if (tempRoot) {
      try {
        await safeRemove(tempRoot);
        evidence.cleanup.removed = true;
      } catch (error) {
        failed = true;
        evidence.cleanup.error = error instanceof Error ? error.message : String(error);
      }
    }
  }

  evidence.status = failed ? 'fail' : 'pass';
  process.stdout.write(`${JSON.stringify(evidence, null, 2)}\n`);
  if (failed) process.exitCode = 1;
}

await main();
