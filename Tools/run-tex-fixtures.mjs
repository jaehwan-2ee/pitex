#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { constants } from 'node:fs';
import { access, cp, mkdtemp, mkdir, readFile, realpath, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const TOOLS_DIRECTORY = dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = resolve(TOOLS_DIRECTORY, '..');
const MATRIX_PATH = join(TOOLS_DIRECTORY, 'tex-build-matrix.json');
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

function firstVersionLine(result) {
  return `${result.stdout}\n${result.stderr}`.split(/\r?\n/u).map((line) => line.trim()).find(Boolean) ?? '';
}

function publicCommandResult(executable, argv, result) {
  return {
    executable,
    argv,
    exitCode: result.exitCode,
    signal: result.signal,
    timedOut: result.timedOut,
    errorCode: result.errorCode,
  };
}

async function assertOutputFiles(directory, names) {
  const output = [];
  for (const name of names) {
    const path = resolve(directory, name);
    if (!containedPath(directory, path)) throw new Error(`Output escapes row directory: ${name}`);
    try {
      const info = await stat(path);
      output.push({ name, present: info.isFile() && info.size > 0 });
    } catch {
      output.push({ name, present: false });
    }
  }
  return output;
}

async function safeRemove(tempRoot) {
  const canonicalTemp = await realpath(tmpdir());
  const canonicalRoot = await realpath(tempRoot);
  const expectedPrefix = `texspark-g003-`;
  if (dirname(canonicalRoot) !== canonicalTemp || !basename(canonicalRoot).startsWith(expectedPrefix)) {
    throw new Error('Refusing to remove a directory outside the dedicated temporary root');
  }
  await rm(canonicalRoot, { recursive: true, force: true });
}

async function main() {
  const evidence = {
    schemaVersion: 1,
    fixture: 'build-matrix',
    tools: [],
    rows: [],
    failureCases: [],
    cleanup: { tempContained: true, removed: false },
    status: 'pass',
  };
  let failed = false;
  let tempRoot;

  try {
    const matrix = JSON.parse(await readFile(MATRIX_PATH, 'utf8'));
    if (matrix.schemaVersion !== 1 || !Array.isArray(matrix.tools) || !Array.isArray(matrix.rows)) {
      throw new Error('Unsupported build matrix schema');
    }

    const fixtureSource = resolve(REPOSITORY_ROOT, matrix.fixture);
    if (!containedPath(REPOSITORY_ROOT, fixtureSource)) throw new Error('Fixture path escapes the repository');
    await access(join(fixtureSource, 'main.tex'), constants.R_OK);
    tempRoot = await mkdtemp(join(tmpdir(), 'texspark-g003-'));

    for (const tool of matrix.tools) {
      const result = await runCommand(tool.executable, tool.versionArgv, tempRoot, 15_000);
      const ok = result.exitCode === 0 && !result.timedOut && firstVersionLine(result) !== '';
      evidence.tools.push({ executable: tool.executable, version: firstVersionLine(result), status: ok ? 'pass' : 'fail' });
      failed ||= !ok;
    }

    for (const row of matrix.rows) {
      const rowRoot = join(tempRoot, 'rows', row.id);
      await mkdir(dirname(rowRoot), { recursive: true });
      await cp(fixtureSource, rowRoot, { recursive: true, errorOnExist: true });
      const cwd = resolve(rowRoot, row.workingDirectory);
      if (!containedPath(rowRoot, cwd)) throw new Error(`Working directory escapes row root: ${row.id}`);
      const passes = [];
      let rowPassed = true;

      for (const pass of row.passPlan) {
        const result = await runCommand(pass.executable, pass.argv, cwd);
        passes.push(publicCommandResult(pass.executable, pass.argv, result));
        if (result.exitCode !== 0 || result.timedOut || result.errorCode !== null) {
          rowPassed = false;
          break;
        }
      }

      const outputs = await assertOutputFiles(cwd, row.expectedOutputs);
      rowPassed &&= outputs.every((output) => output.present);
      evidence.rows.push({ id: row.id, loginShell: row.loginShell === true, status: rowPassed ? 'pass' : 'fail', passes, outputs });
      failed ||= !rowPassed;
    }

    const missing = await runCommand('texspark-g003-intentionally-missing', ['--version'], tempRoot, 5_000);
    const missingPassed = missing.errorCode === 'ENOENT';
    evidence.failureCases.push({ id: 'missing-tool', status: missingPassed ? 'pass' : 'fail', observed: publicCommandResult('texspark-g003-intentionally-missing', ['--version'], missing) });
    failed ||= !missingPassed;

    const malformedRoot = join(tempRoot, 'failure-malformed');
    await mkdir(malformedRoot);
    await writeFile(join(malformedRoot, 'main.tex'), '\\documentclass{article}\n\\begin{document}\n\\undefinedGZeroZeroThreeCommand\n\\end{document}\n', 'utf8');
    const malformed = await runCommand('pdflatex', ['-interaction=nonstopmode', '-halt-on-error', 'main.tex'], malformedRoot, 30_000);
    const malformedPassed = malformed.exitCode !== null && malformed.exitCode !== 0 && !malformed.timedOut;
    evidence.failureCases.push({ id: 'malformed-source', status: malformedPassed ? 'pass' : 'fail', observed: publicCommandResult('pdflatex', ['-interaction=nonstopmode', '-halt-on-error', 'main.tex'], malformed) });
    failed ||= !malformedPassed;

    const cancellation = await runCommand(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], tempRoot, 150);
    const cancellationPassed = cancellation.timedOut && cancellation.exitCode !== 0;
    evidence.failureCases.push({ id: 'bounded-cancellation', status: cancellationPassed ? 'pass' : 'fail', observed: publicCommandResult(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], cancellation) });
    failed ||= !cancellationPassed;
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
        evidence.cleanup.removed = false;
        evidence.cleanup.error = error instanceof Error ? error.message : String(error);
      }
    }
  }

  evidence.status = failed ? 'fail' : 'pass';
  process.stdout.write(`${JSON.stringify(evidence, null, 2)}\n`);
  if (failed) process.exitCode = 1;
}

await main();
