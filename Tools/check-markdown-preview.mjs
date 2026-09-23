#!/usr/bin/env node
// Contract check for the Markdown preview renderer: rebuilds
// Mac/Resources/markdown-preview.html in --check mode, loads the vendor libs
// + preview.js in a vm context, and asserts on rendered samples and the CSP.
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const VENDOR = join(ROOT, 'Assets', 'markdown-preview', 'vendor');
const OUT = join(ROOT, 'Mac', 'Resources', 'markdown-preview.html');

let failures = 0;
function check(name, ok) {
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}`);
  if (!ok) failures++;
}

execFileSync(process.execPath, [join(ROOT, 'Tools', 'build-markdown-preview.mjs'), '--check'],
  { stdio: 'inherit' });

const ctx = vm.createContext({
  // markdown-it's punycode table is base64; browsers provide these, vm doesn't.
  atob: s => Buffer.from(s, 'base64').toString('binary'),
  btoa: s => Buffer.from(s, 'binary').toString('base64'),
});
for (const src of [
  'markdown-it/markdown-it.min.js',
  'katex/katex.min.js',
  'markdown-it-texmath/texmath.js',
  join('..', 'preview.js'),
])
  vm.runInContext(await readFile(join(VENDOR, src), 'utf8'), ctx, { filename: src });

const render = vm.runInContext('renderMarkdown', ctx);
check('renderMarkdown exposed', typeof render === 'function');

const html = render([
  '# Heading',
  '',
  'para with ~~strike~~ and $a^2$ inline',
  '',
  '- item one',
  '- item two',
  '',
  '| a | b |',
  '|---|---|',
  '| 1 | 2 |',
  '',
  '$$\\int_0^1 x\\,dx$$',
  '',
  '```math',
  'x + y',
  '```',
  '',
  '\\[x\\]',
  '',
  'inline \\(y\\) here',
  '',
  '$\\frac{$',
  '',
  '<details><summary>x</summary>y</details>',
  '',
  '<script>alert(1)</script>',
  '',
  '[bad](javascript:alert(1)) [good](https://example.com)',
  '',
  '![img](pics/rel.png)',
  '',
  '> quoted',
  '',
  '---',
].join('\n'));

check('heading renders', /<h1[^>]*>Heading<\/h1>/.test(html));
check('list renders', /<li[^>]*>item one<\/li>/.test(html));
check('GFM table renders', html.includes('<table') && html.includes('<td>1</td>'));
check('~~strike~~ renders', html.includes('<s>strike</s>'));
check('inline $a^2$ renders katex', html.includes('class="katex"'));
check('$$ display math renders', html.includes('katex-display'));
check('```math fence renders display math', render('```math\nx+y\n```').includes('katex-display'));
check('\\[x\\] renders display math', render('\\[x\\]').includes('katex-display'));
check('\\(y\\) renders inline math', render('\\(y\\)').includes('class="katex"'));
check('bad formula keeps katex error span', html.includes('katex-error'));
check('raw HTML passes through', html.includes('<details><summary>x</summary>y</details>'));
check('data-line on heading', /<h1 data-line="0">/.test(html));
check('data-line on paragraph', /<p data-line="2">/.test(html));
check('data-line on list item', /<li data-line="4">/.test(html));
check('data-line on table', /<table data-line="7">/.test(html));
check('data-line on display math', /<section data-line="11">/.test(html));
check('data-line on math fence', /<section data-line="13">/.test(html));
check('data-line on html block', /<div data-line="23">/.test(html));
check('data-line on blockquote', /<blockquote data-line="31">/.test(html));
check('data-line on hr', /<hr data-line="33">/.test(html));
check('javascript: link gets no href', !html.includes('href="javascript:'));
check('https link kept', html.includes('href="https://example.com"'));
check('relative image src kept', html.includes('src="pics/rel.png"'));

const page = await readFile(OUT, 'utf8');
const csp = page.match(/<meta http-equiv="Content-Security-Policy"\s+content="([^"]+)"/);
check('CSP meta present', !!csp);
const scriptSrc = csp?.[1].match(/script-src ([^;]+)/)?.[1] ?? '';
check('script-src uses a sha256 hash', /^'sha256-[A-Za-z0-9+/=]+'$/.test(scriptSrc));
check('script-src has no unsafe-inline', !scriptSrc.includes('unsafe-inline'));

const bundle = page.match(/<script>([\s\S]*)<\/script>/)?.[1] ?? '';
const hash = createHash('sha256').update(bundle).digest('base64');
check('CSP hash matches the inline script bundle', scriptSrc === `'sha256-${hash}'`);
// Raw <script> may appear in the rendered HTML — the CSP is what blocks it.
check('raw script tag passes through (CSP blocks it)', html.includes('<script>alert(1)</script>'));
check('raw script tag absent from page bundle', !bundle.includes('<script>alert(1)'));

if (failures) {
  console.error(`${failures} check(s) failed`);
  process.exit(1);
}
console.log('markdown preview checks passed');
