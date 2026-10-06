#!/usr/bin/env node
// Verify real helper behavior and the source/payload inventory.
import assert from 'node:assert/strict';
import { readFile, readdir, mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const index = process.argv.indexOf('--package-root');
const root = index < 0 ? join(repo, 'Mac/Resources/PitexAgent') : resolve(process.argv[index + 1]);
const names = ['citation-management', 'humanizer', 'latex-compile', 'latex-doctor', 'scientific-writing', 'scispace', 'submission-preflight', 'texlive-runtime-installer'].sort();
assert.deepEqual((await readdir(join(root, 'skills'), { withFileTypes: true })).filter(e => e.isDirectory()).map(e => e.name).sort(), names);
for (const name of names) {
  const text = await readFile(join(root, 'skills', name, 'SKILL.md'), 'utf8');
  assert.match(text, new RegExp(`^---\\r?\\n[\\s\\S]*?^name: ${name}$`, 'm'));
}
const notices = JSON.parse(await readFile(join(root, 'NOTICE-PROVENANCE.json'), 'utf8'));
assert.deepEqual(notices.components.map(c => c.name).sort(), ['@earendil-works/pi-coding-agent', 'humanizer']);
const indexText = await readFile(join(root, 'THIRD-PARTY-NOTICES.md'), 'utf8');
for (const component of notices.components) {
  const bytes = await readFile(join(root, component.licenseFile));
  assert.equal(component.license, 'MIT');
  assert.ok(bytes.toString('utf8').includes(component.copyright), component.name);
  assert.equal(createHash('sha256').update(bytes).digest('hex'), component.licenseSha256, `${component.name} upstream notice changed`);
  assert.ok(indexText.includes(component.copyright), component.name);
}
const provenance = JSON.parse(await readFile(join(root, 'SKILL-PROVENANCE.json'), 'utf8'));
assert.equal(provenance.license, 'MIT');
for (const [name, skill] of Object.entries(provenance.skills)) {
  assert.match(await readFile(join(root, 'skills', name, 'LICENSE.md'), 'utf8'), /MIT License/);
  for (const [relative, checksum] of Object.entries(skill.upstreamFiles)) {
    const bytes = await readFile(join(root, 'skills', name, relative));
    if (!skill.adaptedFiles.includes(relative)) assert.equal(createHash('sha256').update(bytes).digest('hex'), checksum, `${name}/${relative} changed without attribution`);
  }
}
const { normalizeDOI, lookupDOI, toBibTeX } = await import(pathToFileURL(join(root, 'skills/citation-management/scripts/doi_metadata.mjs')));
assert.equal(normalizeDOI('https://doi.org/10.1038/NPHYS1170'), '10.1038/nphys1170');
assert.throws(() => normalizeDOI('javascript:alert(1)'));
const paper = { DOI: '10.1038/nphys1170', type: 'article-journal', author: [{ family: 'A&B', given: 'Jane' }], title: 'DNA & 50%', issued: { 'date-parts': [[2020]] } };
const requests = [];
assert.equal((await lookupDOI(paper.DOI, async url => { requests.push(url); if (url.includes('doi.org')) return { DOI: '10.1111/wrong' }; return { message: paper }; })).DOI, paper.DOI);
assert.equal(requests.length, 2);
await assert.rejects(lookupDOI(paper.DOI, async () => ({ message: { DOI: '10.1111/wrong' } })));
const bib = toBibTeX(paper);
assert.ok(bib.includes('A\\&B, Jane'));
assert.ok(!bib.includes('textbackslash'));
assert.ok(bib.includes('title = {{DNA \\& 50\\%}}'));

const { inspectProject } = await import(pathToFileURL(join(root, 'skills/submission-preflight/scripts/preflight.mjs')));
const work = await mkdtemp(join(tmpdir(), 'pitex-skills-한글-'));
try {
  await mkdir(join(work, 'chapters'));
  await writeFile(join(work, 'main.tex'), '\\documentclass{article}\n% \\input{missing-comment}\n\\input{chapters/body}\n\\bibliography{refs}\n\\begin{document}\\ref{good} \\cite{valid}\\end{document}\n');
  await writeFile(join(work, 'chapters/body.tex'), '\\label{good}\n\\begin{verbatim}\\ref{not-a-reference}\\end{verbatim}\n\\verb|\\cite{not-a-citation}|');
  await writeFile(join(work, 'refs.bib'), '@article{valid, title={DNA}, author={Author}, year={2020}}');
  const original = await readFile(join(work, 'main.tex'), 'utf8');
  const clean = await inspectProject(work);
  assert.equal(clean.findings.length, 0);
  assert.equal(clean.checkedSources.length, 2);
  assert.ok(clean.manualChecks.length > 0);
  await writeFile(join(work, 'chapters/body.tex'), '\\label{good}\\label{good}\\Cref{missing}\\citep{absent}\\input{lost}\\includegraphics{missing-figure}');
  await writeFile(join(work, 'main.log'), '! Undefined control sequence.\nOverfull \\hbox (1pt too wide)\n');
  const issues = await inspectProject(work);
  for (const code of ['duplicate-label', 'unresolved-reference', 'unresolved-citation', 'missing-source', 'missing-figure', 'compile-error', 'compile-warning']) assert.ok(issues.findings.some(f => f.code === code), code);
  assert.equal(issues.status, 'blockers-found');
  assert.equal(await readFile(join(work, 'main.tex'), 'utf8'), original);
  await assert.rejects(inspectProject(work, '../outside.tex'));
} finally { await rm(work, { recursive: true, force: true }); }
console.log('PASS eight bundled skills, complete Pi/Humanizer MIT notices, source provenance, verified DOI fallback and read-only preflight');
