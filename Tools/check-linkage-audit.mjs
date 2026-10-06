#!/usr/bin/env node
// Exercise the real audit with large macOS symbol tables and unsafe fixtures.
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const work = await mkdtemp(join(tmpdir(), 'pitex-linkage-audit-'));
try {
  const commands = join(work, 'commands'), helpers = join(work, 'helpers');
  await mkdir(commands); await mkdir(helpers);
  const executable = (path, source) => writeFile(path, source, { mode: 0o755 });
  for (const name of ['pitex-preview', 'pitex-preview-xetex']) await executable(join(helpers, name), '#!/bin/sh\nexit 0\n');
  await executable(join(commands, 'cargo'), '#!/bin/sh\nexit 0\n');
  await executable(join(commands, 'uname'), '#!/bin/sh\nprintf "Darwin\\n"\n');
  await executable(join(commands, 'otool'), '#!/bin/sh\nprintf "%s:\\n\\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\\n" "$2"\nif [ "$PITEX_AUDIT_CASE" = forbidden-library ]; then printf "\\t@loader_path/libmupdf.dylib (compatibility version 1.0.0)\\n"; fi\n');
  await executable(join(commands, 'nm'), `#!/usr/bin/env node
const fixture = process.env.PITEX_AUDIT_CASE;
if (process.argv[2] === '-gu') {
  if (fixture !== 'missing-import') console.log('                 U _TECkit_ConvertBuffer');
  for (let i = 0; i < 20000; i++) console.log('                 U _external_symbol_' + i);
} else {
  if (fixture === 'defined-teckit') console.log('0000000000000000 T _TECkit_ConvertBuffer');
  if (fixture === 'defined-renderer') console.log('0000000000000000 T _fz_render');
  for (let i = 0; i < 20000; i++) console.log('0000000000000000 T _local_symbol_' + i);
}
`);
  for (const [fixture, expected] of [['valid', null], ['missing-import', 'does not import TECkit_ConvertBuffer'], ['defined-teckit', 'defines TECkit symbols'], ['defined-renderer', 'defines MuPDF/xdvipdfmx symbols'], ['forbidden-library', 'links a forbidden library']]) {
    const result = spawnSync('bash', [join(root, 'PreviewEngine/tools/audit-linkage.sh'), helpers], {
      env: { ...process.env, PATH: commands + ':' + process.env.PATH, PITEX_AUDIT_CASE: fixture },
      encoding: 'utf8', timeout: 30000, maxBuffer: 5_000_000,
    });
    assert.ifError(result.error);
    if (expected === null) assert.equal(result.status, 0, result.stdout + result.stderr);
    else { assert.equal(result.status, 1, result.stdout + result.stderr); assert.ok(result.stdout.includes(expected), fixture); }
    assert.ok(!result.stderr.includes('Broken pipe'), fixture);
  }
} finally { await rm(work, { recursive: true, force: true }); }
console.log('PASS linkage audit accepts large symbol tables and rejects missing imports, static TECkit, renderer symbols and forbidden libraries');
