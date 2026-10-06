#!/usr/bin/env node
// Pitex-authored, read-only static manuscript checker.
import { readFile, realpath, stat } from 'node:fs/promises';
import { dirname, extname, isAbsolute, relative, resolve } from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { pathToFileURL } from 'node:url';

export function sourceText(text) {
  let result = text.replace(/\\begin\{(verbatim\*?|Verbatim|lstlisting|minted)\}[\s\S]*?\\end\{\1\}/g, s => s.replace(/[^\n]/g, ' '));
  result = result.replace(/\\verb\*?([^\w\s])[^\n]*?\1/g, s => ' '.repeat(s.length));
  return result.split('\n').map(line => {
    for (let i = 0; i < line.length; i++) if (line[i] === '%') {
      let escapes = 0; for (let j = i - 1; j >= 0 && line[j] === '\\'; j--) escapes++;
      if (escapes % 2 === 0) return line.slice(0, i);
    }
    return line;
  }).join('\n');
}

export async function inspectProject(project, main = 'main.tex', log, chktex = false) {
  const root = await realpath(project);
  const findings = [], manual = [], unavailable = [];
  const visited = new Set(), bibs = new Set(), labels = new Map(), keys = new Map(), references = [], citations = [];
  const inside = path => { const r = relative(root, path); return r !== '..' && !r.startsWith('../') && !r.startsWith('..\\') && !isAbsolute(r); };
  async function locate(base, name, extensions) {
    for (const extension of extensions) {
      const path = resolve(base, name + extension);
      if (!inside(path)) continue;
      try {
        const actual = await realpath(path);
        if (inside(actual) && (await stat(actual)).isFile()) return actual;
      } catch { /* report missing after exhausting candidates */ }
    }
    return null;
  }
  const display = path => relative(root, path).replaceAll('\\', '/');
  const finding = (severity, code, file, line, message) => findings.push({ severity, code, file: display(file), line, message });
  function register(map, key, file, line, code) {
    if (map.has(key)) finding('error', code, file, line, `${key} also appears in ${map.get(key).file}:${map.get(key).line}`);
    else map.set(key, { file: display(file), line });
  }
  async function visit(file) {
    if (visited.has(file)) return;
    visited.add(file);
    const info = await stat(file);
    if (info.size > 5_000_000) throw new Error(`Source is too large: ${display(file)}`);
    const text = sourceText(await readFile(file, 'utf8'));
    const pattern = /\\(input|include|subfile|bibliography|addbibresource|includegraphics|graphicspath|label|(?:eq|page|auto|v|c)?ref|(?:auto|paren|text|smart|no)?cite[pt]?|bibitem)\*?\s*(?:\[[^\]]*\]\s*)*\{([^{}]*)\}/gi;
    for (const match of text.matchAll(pattern)) {
      const command = match[1].toLowerCase(), value = match[2].trim();
      const line = text.slice(0, match.index).split('\n').length;
      if (/\\|#/.test(value) || command === 'graphicspath') {
        manual.push(`${display(file)}:${line}: inspect dynamic ${command} argument`); continue;
      }
      if (command === 'label' || command === 'bibitem') {
        register(command === 'label' ? labels : keys, value, file, line, command === 'label' ? 'duplicate-label' : 'duplicate-citation-key');
      } else if (command.endsWith('ref')) {
        for (const key of value.split(',').map(s => s.trim()).filter(Boolean)) references.push({ key, file, line });
      } else if (/cite[pt]?$/.test(command)) {
        for (const key of value.split(',').map(s => s.trim()).filter(s => s && s !== '*')) citations.push({ key, file, line });
      } else {
        const names = command === 'bibliography' ? value.split(',').map(s => s.trim()) : [value];
        for (const name of names) {
          const kind = ['bibliography', 'addbibresource'].includes(command) ? 'bib' : command === 'includegraphics' ? 'figure' : 'source';
          const extensions = extname(name) ? [''] : kind === 'figure' ? ['', '.pdf', '.png', '.jpg', '.jpeg', '.eps', '.svg'] : ['', kind === 'bib' ? '.bib' : '.tex'];
          const target = await locate(dirname(file), name, extensions) ?? await locate(root, name, extensions);
          if (!target) {
            // Graphic search paths and generated figures cannot be resolved
            // solely from this conservative static scanner.
            finding(kind === 'figure' ? 'warning' : 'error', `missing-${kind}`, file, line, `Cannot resolve ${name} inside the project`);
          } else if (kind === 'source') await visit(target);
          else if (kind === 'bib') bibs.add(target);
        }
      }
    }
  }
  const entry = await locate(root, main, extname(main) ? [''] : ['', '.tex']);
  if (!entry) throw new Error('The selected main source must exist inside the project.');
  await visit(entry);
  for (const bib of bibs) {
    if ((await stat(bib)).size > 5_000_000) throw new Error(`Bibliography is too large: ${display(bib)}`);
    const text = await readFile(bib, 'utf8');
    for (const m of text.matchAll(/@(?!comment\b|string\b|preamble\b)[a-zA-Z]+\s*[({]\s*([^,\s]+)\s*,/g)) {
      register(keys, m[1], bib, text.slice(0, m.index).split('\n').length, 'duplicate-citation-key');
    }
  }
  for (const ref of references) if (!labels.has(ref.key)) finding('warning', 'unresolved-reference', ref.file, ref.line, `No static label for ${ref.key}`);
  for (const cite of citations) if (!keys.has(cite.key)) finding('warning', 'unresolved-citation', cite.file, cite.line, `No static bibliography entry for ${cite.key}`);
  const logfile = log ? await locate(root, log, ['']) : await locate(dirname(entry), entry.replace(/\.tex$/i, '.log'), ['']);
  if (log && !logfile) throw new Error('The selected log must exist inside the project.');
  if (logfile) {
    const text = await readFile(logfile, 'utf8');
    for (const [index, line] of text.split(/\r?\n/).entries()) {
      if (/^!|LaTeX Error:|Emergency stop|Fatal error/i.test(line)) finding('error', 'compile-error', logfile, index + 1, line.trim());
      else if (/undefined|Overfull \\[hv]box|Rerun to get|Label\(s\) may have changed/i.test(line)) finding('warning', 'compile-warning', logfile, index + 1, line.trim());
    }
    manual.push('Check the log timestamp and final PDF: an existing log may describe an earlier source revision.');
  } else manual.push('No compiler log checked; perform an authorized final build before submission.');
  if (chktex) {
    try {
      const { stdout, stderr } = await promisify(execFile)('chktex', ['-q', '-f%f:%l:%c:%n:%m\n', display(entry)], { cwd: root, timeout: 30000, maxBuffer: 2_000_000 });
      if (stdout.trim()) findings.push({ severity: 'warning', code: 'chktex', file: display(entry), line: null, message: stdout.trim() });
      if (stderr.trim()) unavailable.push(`ChkTeX: ${stderr.trim()}`);
    } catch (error) {
      if (error.stdout?.trim()) findings.push({ severity: 'warning', code: 'chktex', file: display(entry), line: null, message: error.stdout.trim() });
      else unavailable.push(`ChkTeX unavailable or interrupted: ${error.code ?? error.message}`);
    }
  }
  manual.push('Venue-specific page limits, anonymization, declarations, and final PDF appearance require manual review.');
  return { root, main: display(entry), checkedSources: [...visited].map(display), checkedBibliographies: [...bibs].map(display), findings, manualChecks: [...new Set(manual)], unavailableTools: unavailable,
    status: findings.some(f => f.severity === 'error') ? 'blockers-found' : findings.length ? 'warnings-found' : 'static-checks-passed' };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2), options = {};
  try {
    for (let i = 0; i < args.length; i++) {
      const arg = args[i];
      if (['--root', '--main', '--log'].includes(arg)) { if (!args[i + 1] || args[i + 1].startsWith('--')) throw new Error(`${arg} needs a value`); options[arg.slice(2)] = args[++i]; }
      else if (['--json', '--chktex', '--help'].includes(arg)) options[arg.slice(2)] = true;
      else throw new Error(`Unknown argument: ${arg}`);
    }
    if (options.help) console.log('Usage: node preflight.mjs --root <folder> [--main main.tex] [--log main.log] [--chktex] [--json]');
    else {
      const report = await inspectProject(options.root ?? process.cwd(), options.main, options.log, options.chktex);
      console.log(options.json ? JSON.stringify(report, null, 2) : `${report.status}: ${report.main}\n${report.findings.map(f => `${f.severity}: ${f.file}:${f.line ?? ''}: ${f.message}`).join('\n')}\n${report.manualChecks.join('\n')}`);
      if (report.status === 'blockers-found') process.exitCode = 1;
    }
  } catch (error) { console.error(error.message); process.exitCode = 2; }
}
