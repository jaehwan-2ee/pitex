#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const TOOL_DIR = dirname(fileURLToPath(import.meta.url));
const DEFAULT_ROOT = resolve(TOOL_DIR, '..');
const TAXONOMY = ['active-parity', 'excluded-approved', 'out-of-domain'];
const ID = /^p0\.[a-z][a-z0-9-]*(?:\.[a-z][a-z0-9-]*){2,}$/;
const OUT_OF_DOMAIN = /^p0\.(?:identity|commercial|distribution)\./;
const UPDATER = 'p0.macos.updater.in-app-automatic';

function stripComment(line) {
  let quote = null;
  let escaped = false;
  for (let i = 0; i < line.length; i += 1) {
    const c = line[i];
    if (quote) {
      if (escaped) escaped = false;
      else if (c === '\\' && quote === '"') escaped = true;
      else if (c === quote) quote = null;
    } else if (c === '"' || c === "'") quote = c;
    else if (c === '#' && (i === 0 || /\s/.test(line[i - 1]))) return line.slice(0, i);
  }
  if (quote) throw new Error('unterminated quoted scalar');
  return line;
}

function splitInline(text) {
  const result = [];
  let quote = null;
  let escaped = false;
  let start = 0;
  for (let i = 0; i < text.length; i += 1) {
    const c = text[i];
    if (quote) {
      if (escaped) escaped = false;
      else if (c === '\\' && quote === '"') escaped = true;
      else if (c === quote) quote = null;
    } else if (c === '"' || c === "'") quote = c;
    else if (c === ',') { result.push(text.slice(start, i).trim()); start = i + 1; }
  }
  result.push(text.slice(start).trim());
  return result;
}

function scalar(text) {
  text = text.trim();
  if (text === '') return undefined;
  if (text.startsWith('[')) {
    if (!text.endsWith(']')) throw new Error('unterminated inline array');
    const body = text.slice(1, -1).trim();
    return body === '' ? [] : splitInline(body).map(scalar);
  }
  if (text.startsWith('{') || /[&*!]|^---$|^\.\.\.$/.test(text[0] ?? '')) throw new Error(`unsupported YAML construct: ${text}`);
  if (text.startsWith('"')) return JSON.parse(text);
  if (text.startsWith("'")) {
    if (!text.endsWith("'")) throw new Error('unterminated single-quoted scalar');
    return text.slice(1, -1).replaceAll("''", "'");
  }
  if (text === 'true') return true;
  if (text === 'false') return false;
  if (text === 'null' || text === '~') return null;
  if (/^-?(?:0|[1-9]\d*)(?:\.\d+)?$/.test(text)) return Number(text);
  return text;
}

function colon(text) {
  let quote = null;
  for (let i = 0; i < text.length; i += 1) {
    const c = text[i];
    if (quote) { if (c === quote) quote = null; }
    else if (c === '"' || c === "'") quote = c;
    else if (c === ':' && (i === text.length - 1 || /\s/.test(text[i + 1]))) return i;
  }
  return -1;
}

export function parseRepositoryYaml(source) {
  if (source.includes('\t') || source.includes('\r')) throw new Error('tabs and CR characters are not accepted');
  const tokens = source.split('\n').map((raw, index) => ({ raw: stripComment(raw).trimEnd(), line: index + 1 }))
    .filter(({ raw }) => raw.trim() !== '')
    .map(({ raw, line }) => {
      const indent = raw.length - raw.trimStart().length;
      if (indent % 2 !== 0) throw new Error(`line ${line}: indentation must use two-space steps`);
      return { indent, text: raw.trimStart(), line };
    });
  if (tokens.length === 0) throw new Error('empty YAML document');

  function parseBlock(index, indent) {
    const array = tokens[index]?.text.startsWith('- ');
    const value = array ? [] : {};
    while (index < tokens.length && tokens[index].indent === indent) {
      const token = tokens[index];
      if (token.text.startsWith('- ') !== array) throw new Error(`line ${token.line}: mixed mapping and sequence`);
      if (array) {
        const itemText = token.text.slice(2).trim();
        if (itemText === '') {
          if (tokens[index + 1]?.indent !== indent + 2) throw new Error(`line ${token.line}: empty sequence item`);
          const nested = parseBlock(index + 1, indent + 2); value.push(nested.value); index = nested.index;
        } else {
          const split = colon(itemText);
          if (split < 0) { value.push(scalar(itemText)); index += 1; }
          else {
            const object = {};
            const key = itemText.slice(0, split).trim();
            if (!/^[A-Za-z_][A-Za-z0-9_-]*$/.test(key)) throw new Error(`line ${token.line}: invalid key`);
            const rest = itemText.slice(split + 1).trim();
            object[key] = scalar(rest);
            index += 1;
            if (index < tokens.length && tokens[index].indent === indent + 2) {
              const nested = parseBlock(index, indent + 2);
              if (Array.isArray(nested.value)) {
                if (object[key] !== undefined) throw new Error(`line ${token.line}: scalar cannot have children`);
                object[key] = nested.value;
              } else {
                for (const [childKey, childValue] of Object.entries(nested.value)) {
                  if (Object.hasOwn(object, childKey)) throw new Error(`line ${token.line}: duplicate key ${childKey}`);
                  object[childKey] = childValue;
                }
              }
              index = nested.index;
            }
            value.push(object);
          }
        }
      } else {
        const split = colon(token.text);
        if (split < 1) throw new Error(`line ${token.line}: expected key: value`);
        const key = token.text.slice(0, split).trim();
        if (!/^[A-Za-z_][A-Za-z0-9_-]*$/.test(key) || Object.hasOwn(value, key)) throw new Error(`line ${token.line}: invalid or duplicate key ${key}`);
        const rest = token.text.slice(split + 1).trim();
        value[key] = scalar(rest);
        index += 1;
        if (value[key] === undefined) {
          if (tokens[index]?.indent !== indent + 2) throw new Error(`line ${token.line}: key ${key} requires a nested value`);
          const nested = parseBlock(index, indent + 2); value[key] = nested.value; index = nested.index;
        }
      }
      if (index < tokens.length && tokens[index].indent < indent) break;
      if (index < tokens.length && tokens[index].indent > indent) throw new Error(`line ${tokens[index].line}: unexpected indentation`);
    }
    return { value, index };
  }
  if (tokens[0].indent !== 0) throw new Error('document root must not be indented');
  const parsed = parseBlock(0, 0);
  if (parsed.index !== tokens.length || Array.isArray(parsed.value)) throw new Error('manifest root must be one mapping');
  return parsed.value;
}

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value && typeof value === 'object') return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
  return JSON.stringify(value);
}
function sha256(text) { return createHash('sha256').update(text).digest('hex'); }
function add(list, code, path, message) { list.push({ code, path: path.replaceAll(sep, '/'), message }); }
function object(value) { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function exactKeys(value, required, path, violations) {
  if (!object(value)) { add(violations, 'STRUCTURE', path, 'expected an object'); return false; }
  const missing = required.filter((key) => !Object.hasOwn(value, key));
  if (missing.length) add(violations, 'REQUIRED_FIELD', path, `missing: ${missing.join(', ')}`);
  const unexpected = Object.keys(value).filter((key) => !required.includes(key));
  if (unexpected.length) add(violations, 'UNEXPECTED_FIELD', path, `unexpected: ${unexpected.join(', ')}`);
  return missing.length === 0 && unexpected.length === 0;
}
function validUrl(value) { try { const url = new URL(value); return url.protocol === 'https:' && url.hostname === 'texspark.io'; } catch { return false; } }

function validateManifest(manifest, violations) {
  exactKeys(manifest, ['schema_version','manifest_id','status','completeness_claimed','reference','taxonomy','items'], 'Parity/manifest.yaml', violations);
  if (manifest.schema_version !== '1.0.0' || manifest.manifest_id !== 'p0') add(violations, 'MANIFEST_IDENTITY', 'Parity/manifest.yaml', 'schema_version 1.0.0 and manifest_id p0 are required');
  if (manifest.status !== 'provisional' || manifest.completeness_claimed !== false) add(violations, 'P0_COMPLETENESS', 'Parity/manifest.yaml', 'P0 must remain provisional and must not claim completeness');
  if (JSON.stringify(manifest.taxonomy) !== JSON.stringify(TAXONOMY)) add(violations, 'TAXONOMY', 'Parity/manifest.yaml', 'taxonomy must be the closed ordered three-value taxonomy');
  const ref = manifest.reference;
  if (!object(ref) || ref.product !== 'TexSpark' || ref.version !== 'v0.10.2' || ref.release_date !== '2026-08-28' || ref.documented_source !== 'https://texspark.io/category/changelog/' || ref.downloadable_build?.sha256 !== 'unresolved' || ref.downloadable_build?.resolution_phase !== 'final-transition-r1' || ref.black_box_inventory?.status !== 'unresolved' || ref.black_box_inventory?.resolution_phase !== 'final-transition-r1') add(violations, 'REFERENCE_IDENTITY', 'Parity/manifest.yaml', 'provisional reference identity/resolution fields are malformed');
  if (!Array.isArray(manifest.items) || manifest.items.length === 0) { add(violations, 'MANIFEST_ITEMS', 'Parity/manifest.yaml', 'items must be a non-empty array'); return new Map(); }
  const items = new Map();
  for (const [index, item] of manifest.items.entries()) {
    const path = `Parity/manifest.yaml#items/${index}`;
    exactKeys(item, ['id','title','taxonomy','provisional','source_urls','rationale'], path, violations);
    if (!ID.test(item?.id ?? '')) add(violations, 'STABLE_ID', path, 'invalid stable P0 ID');
    if (items.has(item?.id)) add(violations, 'DUPLICATE_ID', path, item.id); else items.set(item?.id, item);
    if (!TAXONOMY.includes(item?.taxonomy)) add(violations, 'TAXONOMY', path, 'unknown taxonomy');
    if (item?.provisional !== true || typeof item?.title !== 'string' || item.title.length === 0 || typeof item?.rationale !== 'string' || item.rationale.length === 0) add(violations, 'ITEM_FIELDS', path, 'title, rationale, and provisional flag are required');
    if (!Array.isArray(item?.source_urls) || item.source_urls.length === 0 || new Set(item.source_urls).size !== item.source_urls.length || item.source_urls.some((url) => !validUrl(url))) add(violations, 'PROVENANCE_URL', path, 'source_urls must be unique https://texspark.io URLs');
    if (item?.taxonomy === 'excluded-approved' && item.id !== UPDATER) add(violations, 'SOLE_EXCLUSION', path, 'only in-app automatic update may be excluded');
    if (item?.taxonomy === 'out-of-domain' && !OUT_OF_DOMAIN.test(item.id)) add(violations, 'OUT_OF_DOMAIN', path, 'out-of-domain IDs must be identity, commercial, or distribution concerns');
    if (item?.taxonomy === 'active-parity' && (OUT_OF_DOMAIN.test(item.id) || item.id === UPDATER)) add(violations, 'ACTIVE_SEMANTICS', path, 'active item uses a reserved non-active ID');
  }
  const excluded = [...items.values()].filter((item) => item.taxonomy === 'excluded-approved');
  if (excluded.length !== 1 || excluded[0]?.id !== UPDATER) add(violations, 'SOLE_EXCLUSION', 'Parity/manifest.yaml', 'exactly one excluded-approved updater item is required');
  return items;
}

function validateEvidence(index, items, violations) {
  exactKeys(index, ['schema_version','manifest_id','status','completeness_claimed','entries'], 'Parity/evidence-index.json', violations);
  if (index.schema_version !== '1.0.0' || index.manifest_id !== 'p0' || index.status !== 'provisional' || index.completeness_claimed !== false) add(violations, 'EVIDENCE_IDENTITY', 'Parity/evidence-index.json', 'evidence index must be provisional P0 without completeness claim');
  if (!Array.isArray(index.entries)) { add(violations, 'EVIDENCE_ENTRIES', 'Parity/evidence-index.json', 'entries must be an array'); return; }
  const seen = new Set();
  const categories = new Set(['linux-portable-contract','linux-integration','final-mac-native','final-mac-filesystem','final-mac-auth','final-mac-accessibility']);
  const phases = new Set(['linux-p0','final-mac-r1','final-mac-e2e']);
  const environments = new Set(['canonical-linux','physical-mac-min','physical-mac-latest','private-backend']);
  for (const [i, entry] of index.entries.entries()) {
    const path = `Parity/evidence-index.json#entries/${i}`;
    exactKeys(entry, ['parity_id','taxonomy','provisional','provenance','disposition','planned_evidence'], path, violations);
    const item = items.get(entry?.parity_id);
    if (!item) add(violations, 'UNKNOWN_EVIDENCE_ID', path, entry?.parity_id ?? 'missing parity_id');
    if (seen.has(entry?.parity_id)) add(violations, 'DUPLICATE_EVIDENCE', path, entry.parity_id); seen.add(entry?.parity_id);
    if (item && entry.taxonomy !== item.taxonomy) add(violations, 'TAXONOMY_MISMATCH', path, 'evidence taxonomy differs from manifest');
    if (entry?.provisional !== true) add(violations, 'P0_COMPLETENESS', path, 'P0 evidence must be provisional');
    const p = entry?.provenance;
    if (!object(p) || !Array.isArray(p.source_urls) || p.source_urls.length === 0 || new Set(p.source_urls).size !== p.source_urls.length || p.source_urls.some((url) => !validUrl(url)) || p.observed_date !== '2026-08-30' || p.method !== 'public-web' || p.status !== 'provisional') add(violations, 'EVIDENCE_PROVENANCE', path, 'invalid public-web provenance');
    const planned = entry?.planned_evidence;
    if (!Array.isArray(planned)) add(violations, 'PLANNED_EVIDENCE', path, 'planned_evidence must be an array');
    else if (entry.taxonomy === 'active-parity') {
      if (entry.disposition !== 'planned' || planned.length === 0) add(violations, 'ACTIVE_EVIDENCE', path, 'active parity requires planned evidence');
      for (const proof of planned) if (!object(proof) || !categories.has(proof.category) || !phases.has(proof.phase) || typeof proof.artifact_kind !== 'string' || proof.artifact_kind.length === 0 || !environments.has(proof.environment) || proof.status !== 'planned') add(violations, 'PLANNED_EVIDENCE', path, 'invalid planned evidence record');
    } else if (entry.disposition !== 'not-applicable' || planned.length !== 0) add(violations, 'NON_ACTIVE_EVIDENCE', path, 'excluded/out-of-domain entries must be not-applicable with no planned evidence');
  }
  for (const id of items.keys()) if (!seen.has(id)) add(violations, 'EVIDENCE_COVERAGE', 'Parity/evidence-index.json', `missing evidence entry for ${id}`);
}

function validateLedger(source, items, violations) {
  const lines = source.split('\n').filter((line) => line.trim() !== '');
  if (lines.length === 0) { add(violations, 'LEDGER_EMPTY', 'Parity/observations/ledger.jsonl', 'ledger must contain at least one event'); return; }
  let previous = null;
  const eventIds = new Set();
  for (let i = 0; i < lines.length; i += 1) {
    const path = `Parity/observations/ledger.jsonl:${i + 1}`;
    let event;
    try { event = JSON.parse(lines[i]); } catch (error) { add(violations, 'LEDGER_JSON', path, error.message); continue; }
    exactKeys(event, ['event_id','event_type','status','provisional','parity_ids','reference','source','environment','evidence_method','observer','evidence','expected_result','classification','review','supersedes_event_id','chain'], path, violations);
    if (typeof event.event_id !== 'string' || event.event_id.length === 0 || eventIds.has(event.event_id)) add(violations, 'LEDGER_EVENT_ID', path, 'event_id must be non-empty and unique'); eventIds.add(event.event_id);
    if (event.chain?.previous_event_sha256 !== previous) add(violations, 'LEDGER_PREDECESSOR', path, `expected ${previous ?? 'null'}`);
    if (!Array.isArray(event.parity_ids) || new Set(event.parity_ids).size !== event.parity_ids.length || event.parity_ids.some((id) => !items.has(id))) add(violations, 'LEDGER_PARITY_IDS', path, 'parity_ids must uniquely identify manifest items');
    const classifications = event.classification?.taxonomy_values;
    if (!Array.isArray(classifications) || classifications.some((value) => !TAXONOMY.includes(value)) || event.parity_ids?.some((id) => !classifications.includes(items.get(id)?.taxonomy)) || typeof event.classification?.rationale !== 'string') add(violations, 'LEDGER_TAXONOMY', path, 'classification must cover the taxonomy of every parity ID');
    if (event.event_type !== 'initial-public-observation' || event.status !== 'provisional' || event.provisional !== true || event.reference?.product !== 'TexSpark' || event.reference?.version !== 'v0.10.2' || event.reference?.release_date !== '2026-08-28' || event.reference?.downloadable_build_sha256 !== 'unresolved' || event.reference?.black_box_inventory !== 'unresolved-until-final-transition-r1' || !validUrl(event.source?.url) || typeof event.source?.locator !== 'string' || event.source?.observed_date !== '2026-08-30' || event.environment?.platform !== 'canonical-linux' || event.environment?.reference_build_executed !== false || event.evidence_method !== 'public-web' || typeof event.observer !== 'string' || typeof event.expected_result !== 'string' || event.review?.status !== 'pending-final-transition-r1' || typeof event.review?.reviewer !== 'string') add(violations, 'LEDGER_PROVENANCE', path, 'required provisional reference/source/environment/review provenance is incomplete');
    if (event.evidence?.kind !== 'normalized-observation-summary' || typeof event.evidence?.summary !== 'string' || !/^[a-f0-9]{64}$/.test(event.evidence?.sha256 ?? '') || sha256(event.evidence?.summary ?? '') !== event.evidence?.sha256) add(violations, 'LEDGER_EVIDENCE_HASH', path, 'normalized observation summary SHA-256 is invalid');
    if (event.supersedes_event_id !== null && !eventIds.has(event.supersedes_event_id)) add(violations, 'LEDGER_SUPERSESSION', path, 'supersedes_event_id must identify an earlier event');
    const chain = event.chain;
    if (chain?.algorithm !== 'sha256' || typeof chain.canonicalization !== 'string' || typeof chain.digest_rule !== 'string' || !/^[a-f0-9]{64}$/.test(chain.event_sha256 ?? '')) add(violations, 'LEDGER_CHAIN', path, 'invalid chain declaration');
    else {
      const material = structuredClone(event);
      delete material.chain.event_sha256;
      const digest = sha256(canonical(material));
      if (chain.canonicalization !== 'recursive-key-sorted-json-utf8' || chain.digest_rule !== 'SHA-256 of UTF-8 JSON with object keys recursively sorted, no insignificant whitespace, and chain.event_sha256 omitted; chain.previous_event_sha256 remains included' || digest !== chain.event_sha256) add(violations, 'LEDGER_HASH', path, 'event hash does not match the declared deterministic canonicalization');
      previous = chain.event_sha256;
    }
  }
}

export async function validateParity({ root = DEFAULT_ROOT } = {}) {
  root = resolve(root);
  const violations = [];
  const evidence = ['Parity/manifest.yaml','Parity/schema/manifest.schema.json','Parity/evidence-index.json','Parity/schema/evidence.schema.json','Parity/observations/ledger.jsonl','ReferenceEvidence/policy.json'];
  let manifest = {};
  let index = {};
  let ledger = '';
  for (const schema of ['Parity/schema/manifest.schema.json','Parity/schema/evidence.schema.json']) try {
    const document = JSON.parse(await readFile(join(root, schema), 'utf8'));
    if (document.$schema !== 'https://json-schema.org/draft/2020-12/schema' || document.type !== 'object' || !object(document.$defs)) add(violations, 'SCHEMA_CONTRACT', schema, 'schema must be a draft 2020-12 object schema with definitions');
  } catch (error) { add(violations, 'SCHEMA_JSON', schema, error.code === 'ENOENT' ? 'required schema is missing' : error.message); }
  try { manifest = parseRepositoryYaml(await readFile(join(root, 'Parity/manifest.yaml'), 'utf8')); } catch (error) { add(violations, 'MANIFEST_YAML', 'Parity/manifest.yaml', error.code === 'ENOENT' ? 'required manifest is missing' : error.message); }
  try { index = JSON.parse(await readFile(join(root, 'Parity/evidence-index.json'), 'utf8')); } catch (error) { add(violations, 'EVIDENCE_JSON', 'Parity/evidence-index.json', error.code === 'ENOENT' ? 'required evidence index is missing' : error.message); }
  try { ledger = await readFile(join(root, 'Parity/observations/ledger.jsonl'), 'utf8'); } catch (error) { add(violations, 'LEDGER_READ', 'Parity/observations/ledger.jsonl', error.code === 'ENOENT' ? 'required ledger is missing' : error.message); }
  try {
    const policy = JSON.parse(await readFile(join(root, 'ReferenceEvidence/policy.json'), 'utf8'));
    const prohibited = new Set(policy?.prohibitedUses ?? []);
    if (!object(policy) || policy.status !== 'quarantined' || policy.root !== 'ReferenceEvidence/' || policy.enforcement?.mode !== 'fail-closed' || policy.archiveRules?.excludeFromProductArchives !== true || policy.archiveRules?.excludeFromWorkspaceBundles !== true || policy.exportConditions?.rawCaptureExportToProductTree !== false || !prohibited.has('product-source') || !prohibited.has('product-resource') || !prohibited.has('test-fixture') || !prohibited.has('build-input')) add(violations, 'QUARANTINE_POLICY', 'ReferenceEvidence/policy.json', 'quarantine policy does not fail closed against product, fixture, build, and archive use');
  } catch (error) { add(violations, 'QUARANTINE_POLICY', 'ReferenceEvidence/policy.json', error.code === 'ENOENT' ? 'required quarantine policy is missing' : error.message); }
  const items = validateManifest(manifest, violations);
  validateEvidence(index, items, violations);
  validateLedger(ledger, items, violations);
  violations.sort((a, b) => `${a.code}:${a.path}:${a.message}`.localeCompare(`${b.code}:${b.path}:${b.message}`));
  return { tool: 'validate-parity', status: violations.length === 0 ? 'pass' : 'fail', checks: [{ command: 'static parity manifest/schema/evidence/ledger validation', result: violations.length === 0 ? 'pass' : 'fail', evidence }], violations };
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  try {
    const result = await validateParity({ root: process.argv[2] ? resolve(process.argv[2]) : DEFAULT_ROOT });
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    if (result.status !== 'pass') process.exitCode = 1;
  } catch (error) {
    process.stdout.write(`${JSON.stringify({ tool: 'validate-parity', status: 'fail', checks: [], violations: [{ code: 'INTERNAL_ERROR', path: '', message: error.message }] }, null, 2)}\n`); process.exitCode = 1;
  }
}
