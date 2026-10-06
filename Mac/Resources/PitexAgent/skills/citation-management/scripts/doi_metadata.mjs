#!/usr/bin/env node
// Pitex-authored DOI metadata helper. No project mutation or dependencies.
import { pathToFileURL } from 'node:url';

export function normalizeDOI(value) {
  const doi = value.trim().replace(/^doi:\s*/i, '').replace(/^https?:\/\/(?:dx\.)?doi\.org\//i, '');
  if (!/^10\.\d{4,9}\/\S+$/i.test(doi)) throw new Error('Provide a DOI or doi.org URL.');
  return doi.toLowerCase();
}

async function request(url, accept) {
  const response = await fetch(url, {
    headers: { Accept: accept, 'User-Agent': 'Pitex citation-management/1.0' },
    signal: AbortSignal.timeout(15000),
  });
  if (!response.ok) throw new Error(`Metadata service returned HTTP ${response.status}.`);
  const text = await response.text();
  if (text.length > 2_000_000) throw new Error('Metadata response is too large.');
  return JSON.parse(text);
}

export async function lookupDOI(value, load = request) {
  const doi = normalizeDOI(value);
  let metadata;
  try {
    metadata = await load(`https://doi.org/${encodeURIComponent(doi)}`, 'application/vnd.citationstyles.csl+json');
    if (normalizeDOI(metadata.DOI ?? '') !== doi) throw new Error('The returned DOI does not match.');
  } catch {
    const result = await load(`https://api.crossref.org/works/${encodeURIComponent(doi)}`, 'application/json');
    metadata = result.message;
    if (!metadata || normalizeDOI(metadata.DOI ?? '') !== doi) throw new Error('No matching DOI metadata was returned.');
  }
  return metadata;
}

function tex(value) {
  return String(value ?? '').replace(/[\\{}%&#_$]/g, c => ({ '\\': '\\textbackslash{}', '{': '\\{', '}': '\\}', '%': '\\%', '&': '\\&', '#': '\\#', '_': '\\_', '$': '\\$' })[c]);
}

export function toBibTeX(metadata) {
  const authors = (metadata.author ?? []).map(a => a.literal ? `{${tex(a.literal)}}` : [tex(a.family), tex(a.given)].filter(Boolean).join(', ')).join(' and ');
  const year = (metadata.issued ?? metadata.published ?? metadata['published-print'] ?? metadata.created)?.['date-parts']?.[0]?.[0];
  const first = metadata.author?.[0]?.family ?? 'paper';
  const key = `${first.normalize('NFKD').replace(/[^a-zA-Z0-9]/g, '') || 'paper'}${year ?? ''}${normalizeDOI(metadata.DOI).replace(/[^a-z0-9]/g, '').slice(-8)}`;
  const scalar = value => Array.isArray(value) ? value[0] : value;
  const type = metadata.type === 'journal-article' || metadata.type === 'article-journal' ? 'article' : metadata.type === 'book' ? 'book' : 'misc';
  const fields = { author: authors, title: scalar(metadata.title), year,
    [type === 'article' ? 'journal' : 'publisher']: scalar(metadata['container-title']) ?? metadata.publisher,
    volume: metadata.volume, number: metadata.issue, pages: metadata.page, doi: normalizeDOI(metadata.DOI), url: metadata.URL };
  return `@${type}{${key},\n${Object.entries(fields).filter(([, v]) => v !== undefined && v !== '').map(([k, v]) => `  ${k} = {${k === 'author' ? v : k === 'title' ? `{${tex(v)}}` : tex(v)}},`).join('\n')}\n}\n`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2);
  if (args.includes('--help') || !args.some(a => !a.startsWith('--'))) {
    console.log('Usage: node doi_metadata.mjs <DOI-or-URL> [--json]');
    process.exit(args.includes('--help') ? 0 : 2);
  }
  try {
    const metadata = await lookupDOI(args.find(a => !a.startsWith('--')));
    process.stdout.write(args.includes('--json') ? `${JSON.stringify(metadata, null, 2)}\n` : toBibTeX(metadata));
  } catch (error) {
    console.error(`DOI metadata unresolved: ${error.message}`);
    process.exitCode = 1;
  }
}
