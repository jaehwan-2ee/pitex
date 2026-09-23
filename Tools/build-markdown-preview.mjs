#!/usr/bin/env node
// Builds the single self-contained Markdown preview page
// (Mac/Resources/markdown-preview.html) from Assets/markdown-preview:
// inlines the vendor JS/CSS, the glue, and KaTeX's woff2 fonts as data: URIs,
// then stamps the inline script's SHA-256 into the CSP. Output is
// deterministic — no timestamps, no absolute paths.
//   node Tools/build-markdown-preview.mjs          rebuild the resource
//   node Tools/build-markdown-preview.mjs --check  fail if it is stale
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const ASSETS = join(ROOT, 'Assets', 'markdown-preview');
const VENDOR = join(ASSETS, 'vendor');
const OUT = join(ROOT, 'Mac', 'Resources', 'markdown-preview.html');

const SCRIPT_SOURCES = [
  'markdown-it/markdown-it.min.js',
  'katex/katex.min.js',
  'markdown-it-texmath/texmath.js',
  join('..', 'preview.js'),
];

const CSS_SOURCES = [
  'katex/katex.min.css',
  'markdown-it-texmath/texmath.css',
  join('..', 'preview.css'),
];

// Inline the woff2 font files referenced by katex.min.css as data: URIs.
// The .woff/.ttf alternates stay as plain url()s — the CSP only allows data:
// fonts, so they are never fetched once the woff2 entry resolves.
async function inlineFonts(css) {
  const matches = [...css.matchAll(/url\(fonts\/([^)]*\.woff2)\)/g)];
  const fonts = new Map();
  for (const [, name] of matches) {
    if (!fonts.has(name))
      fonts.set(name, await readFile(join(VENDOR, 'katex', 'fonts', name)));
  }
  return css.replace(/url\(fonts\/([^)]*\.woff2)\)/g,
    (_, name) => `url(data:font/woff2;base64,${fonts.get(name).toString('base64')})`);
}

export async function buildHtml() {
  const parts = [];
  for (const src of SCRIPT_SOURCES)
    parts.push(await readFile(join(VENDOR, src), 'utf8'));
  const script = parts.map(p => p.trimEnd()).join('\n');

  const cssParts = [];
  for (const src of CSS_SOURCES)
    cssParts.push(await readFile(join(VENDOR, src), 'utf8'));
  const css = (await inlineFonts(cssParts[0])) + cssParts.slice(1).join('');

  const hash = createHash('sha256').update(script).digest('base64');
  const template = await readFile(join(ASSETS, 'template.html'), 'utf8');
  return template
    .replace('@@CSP_HASH@@', hash)
    .replace('@@CSS@@', () => css)
    .replace('@@SCRIPT@@', () => script);
}

const check = process.argv.includes('--check');
const html = await buildHtml();
if (check) {
  let committed = null;
  try { committed = await readFile(OUT, 'utf8'); } catch { /* missing */ }
  if (committed !== html) {
    console.error('markdown-preview.html is stale — run node Tools/build-markdown-preview.mjs');
    process.exit(1);
  }
  console.log('markdown-preview.html is up to date');
} else {
  await writeFile(OUT, html);
  console.log(`wrote ${OUT} (${html.length} bytes)`);
}
