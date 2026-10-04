#!/usr/bin/env node
// Fetches and vendors the offline MathJax assets used by the equation
// preview into Assets/equation-preview/vendor/.  Versions and npm tarball
// integrity are pinned below — nothing is downloaded unpinned.  Every
// extracted file is sha256'd into vendor/manifest.json; --check re-verifies
// the committed tree against the manifest without touching the network.
//
//   node Tools/fetch-equation-preview-assets.mjs          fetch/refresh
//   node Tools/fetch-equation-preview-assets.mjs --check  verify offline
//
// Layout contract (consumed by the equation-preview renderer page):
//   vendor/mathjax/tex-svg-nofont.js                     entrypoint bundle
//   vendor/mathjax/mathjax-tex-font/svg.js               TeX font (glyphs inline)
//   vendor/mathjax/input/tex/extensions/*.js             all TeX extensions
//   vendor/mathjax/ui/safe.js                            optional sanitizer
//   vendor/mathjax/LICENSE ; mathjax-tex-font/OFL-1.1.txt
// The page MUST set MathJax.loader.paths.mathjax and .fonts to the
// vendor/mathjax dir and output.font='mathjax-tex' before
// tex-svg-nofont.js runs; the upstream fonts default is cdn.jsdelivr.net —
// see VERSIONS for details.

import { createHash } from 'node:crypto';
import { mkdir, readFile, rm, writeFile, readdir } from 'node:fs/promises';
import { mkdirSync, createWriteStream } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { pipeline } from 'node:stream/promises';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const VENDOR = join(ROOT, 'Assets', 'equation-preview', 'vendor');
const MATHJAX_DIR = join(VENDOR, 'mathjax');
const MANIFEST = join(VENDOR, 'manifest.json');

// Pinned upstream packages.  integrity = npm registry dist.integrity (sha512).
const PACKAGES = [
  {
    name: 'mathjax',
    version: '4.1.3',
    license: 'Apache-2.0',
    url: 'https://registry.npmjs.org/mathjax/-/mathjax-4.1.3.tgz',
    integrity: 'sha512-BN/8Pkgn7G1pIDYJqd9md+JHsE/jydSYbyOZnSdSA0WziuVO8mRxdYiWFumkVVly/8U+hm9DpIIoWuvySverzw==',
    home: 'https://github.com/mathjax/MathJax',
    // Members to vendor, relative to package/ inside the tarball.
    // 'dir:' entries copy every file in that directory (flat).
    members: [
      'tex-svg-nofont.js',
      'dir:input/tex/extensions',
      'ui/safe.js',
      'LICENSE',
    ],
    dest: '',
  },
  {
    name: '@mathjax/mathjax-tex-font',
    version: '4.1.3',
    // package code is Apache-2.0 (npm package.json); glyph data is OFL-1.1
    // (upstream def/mathjax-tex.ts legal declaration) — both are retained.
    license: 'Apache-2.0 AND OFL-1.1',
    url: 'https://registry.npmjs.org/@mathjax/mathjax-tex-font/-/mathjax-tex-font-4.1.3.tgz',
    integrity: 'sha512-9B78brBEmmAwyumaREIyM1gF2HgDamDIXA36UyeXWxX7XrpLdmoSB5NLIf/c6tEA4L6xNrIdMxfgY6YX/kcSNw==',
    home: 'https://github.com/mathjax/MathJax-fonts',
    members: [
      'svg.js',
      'package.json',
    ],
    dest: 'mathjax-tex-font',
  },
];

const sha256 = buf => createHash('sha256').update(buf).digest('hex');
const sha512 = buf => 'sha512-' + createHash('sha512').update(buf).digest('base64');

// The mathjax-tex font data is MathJax-authored (Copyright (c) 2022 MathJax,
// Inc.) under the SIL Open Font License 1.1 per the font package's own
// def/mathjax-tex.ts legal declaration; the npm tarball ships no license
// file, so the canonical OFL-1.1 text is vendored verbatim.
const NOTICES = [
  {
    name: 'SIL OFL-1.1 (mathjax-tex font license)',
    url: 'https://raw.githubusercontent.com/spdx/license-list-data/main/text/OFL-1.1.txt',
    sha256: '8eea8287e5876b539670cadb82e99f9a7afddec6f6730811be1daf25d2e9bcfd',
    dest: 'mathjax/mathjax-tex-font/OFL-1.1.txt',
  },
];

async function download(url, dest) {
  const res = await fetch(url, { redirect: 'follow' });
  if (!res.ok) throw new Error(`GET ${url} -> ${res.status}`);
  await pipeline(res.body, createWriteStream(dest));
  return readFile(dest);
}

function extract(tgz, dir) {
  mkdirSync(dir, { recursive: true });
  // system tar (bsdtar/gtar both fine for plain ustar extraction)
  execFileSync('tar', ['-xzf', tgz, '-C', dir], { stdio: 'pipe' });
}

async function collect(dir, prefix, out) {
  for (const ent of await readdir(join(dir, prefix), { withFileTypes: true })) {
    const rel = prefix ? `${prefix}/${ent.name}` : ent.name;
    if (ent.isDirectory()) await collect(dir, rel, out);
    else if (ent.isFile()) out.push(rel);
  }
}

async function copyIn(base, rel, pkg, files) {
  const dest = join(MATHJAX_DIR, pkg.dest, rel);
  await mkdir(dirname(dest), { recursive: true });
  const data = await readFile(join(base, rel));
  await writeFile(dest, data);
  files[join('mathjax', pkg.dest, rel).replace(/\\/g, '/')] = sha256(data);
}

async function fetchAll() {
  const files = {};
  const tmp = join(tmpdir(), `pitex-mathjax-${process.pid}`);
  await mkdir(tmp, { recursive: true });
  // wipe the owned tree first so stale files can't survive a refresh
  await rm(MATHJAX_DIR, { recursive: true, force: true });
  try {
    for (const pkg of PACKAGES) {
      const tgz = join(tmp, pkg.name.split('/').pop() + '.tgz');
      const buf = await download(pkg.url, tgz);
      const got = sha512(buf);
      if (got !== pkg.integrity)
        throw new Error(`${pkg.name}@${pkg.version} integrity mismatch\n  want ${pkg.integrity}\n  got  ${got}\n  Refusing to vendor unpinned bytes — re-pin deliberately.`);
      const xdir = join(tmp, pkg.name.replace(/[@/]/g, '_'));
      extract(tgz, xdir);
      const base = join(xdir, 'package');
      for (const member of pkg.members) {
        if (member.startsWith('dir:')) {
          const sub = member.slice(4);
          const list = [];
          await collect(base, sub, list);
          for (const rel of list) await copyIn(base, rel, pkg, files);
        } else {
          await copyIn(base, member, pkg, files);
        }
      }
      console.log(`vendored ${pkg.name}@${pkg.version} (integrity ok)`);
    }

    for (const n of NOTICES) {
      const buf = await download(n.url, join(tmp, n.dest.split('/').pop()));
      const got = sha256(buf);
      if (got !== n.sha256)
        throw new Error(`${n.name} hash mismatch\n  want ${n.sha256}\n  got  ${got}`);
      const dest = join(VENDOR, n.dest);
      await mkdir(dirname(dest), { recursive: true });
      await writeFile(dest, buf);
      files[n.dest] = got;
      console.log(`vendored ${n.name}`);
    }
    const manifest = {
      generatedBy: 'Tools/fetch-equation-preview-assets.mjs',
      packages: PACKAGES.map(({ name, version, license, url, integrity, home }) =>
        ({ name, version, license, url, integrity, home })),
      files,
    };
    await writeFile(MANIFEST, JSON.stringify(manifest, null, 2) + '\n');
    console.log(`wrote ${MANIFEST} (${Object.keys(files).length} files)`);
  } finally {
    await rm(tmp, { recursive: true, force: true });
  }
}

async function checkAll() {
  const manifest = JSON.parse(await readFile(MANIFEST, 'utf8'));
  let bad = 0;
  const seen = new Set();
  for (const [rel, want] of Object.entries(manifest.files)) {
    seen.add(rel);
    let data;
    try { data = await readFile(join(VENDOR, rel)); }
    catch { console.error(`missing ${rel}`); bad++; continue; }
    if (sha256(data) !== want) { console.error(`hash mismatch ${rel}`); bad++; }
  }
  const extra = [];
  await collect(VENDOR, 'mathjax', extra);
  for (const rel of extra)
    if (!seen.has(rel)) { console.error(`untracked file ${rel}`); bad++; }
  if (bad) { console.error(`${bad} problem(s) — run node Tools/fetch-equation-preview-assets.mjs`); process.exit(1); }
  console.log(`vendor tree OK (${seen.size} files, ${manifest.packages.map(p => `${p.name}@${p.version}`).join(', ')})`);
}

process.argv.includes('--check') ? await checkAll() : await fetchAll();
