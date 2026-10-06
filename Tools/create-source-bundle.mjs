#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { lstat, mkdir, readFile, readdir, rename, writeFile } from 'node:fs/promises';
import { dirname, extname, join, relative, resolve, sep } from 'node:path';
import { gzipSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const TOP_LEVEL = ['Mac', 'Linux', 'Windows', 'Docs', 'Fixtures', 'Packages', 'Parity', 'Tools'];
const SKIP_DIRS = new Set(['.git', '.gjc', 'node_modules', 'dist', '.build', 'build', 'target', 'DerivedData', 'coverage', 'ReferenceEvidence', 'Evidence', 'Artifacts']);
const SKIP_EXTENSIONS = new Set(['.db', '.sqlite', '.sqlite3', '.log', '.tmp', '.d', '.o', '.a', '.so', '.dylib', '.class', '.pyc', '.DS_Store']);
const ALLOWED_BINARY = new Set(['.png', '.jpg', '.jpeg', '.pdf', '.icns', '.zip']);
const slash = (value) => value.replaceAll(sep, '/');
const sha256 = (value) => createHash('sha256').update(value).digest('hex');
const stable = (value) => JSON.stringify(value, null, 2) + '\n';
function insideRoot(path) { const rel = relative(ROOT, path); return rel !== '..' && !rel.startsWith(`..${sep}`) && !resolve(path).startsWith(`${ROOT}${sep}.git${sep}`); }
function excludedName(name) { return /(^|\.)env($|\.)|secret|credential|private[-_.]?key|\.pem$|\.p12$|\.mobileprovision$/i.test(name); }
function probablyBinary(buffer) { if (buffer.includes(0)) return true; let controls = 0; for (const byte of buffer.subarray(0, 8192)) if (byte < 9 || (byte > 13 && byte < 32)) controls++; return controls > Math.max(4, buffer.length * 0.01); }
async function gather() {
  const files = [];
  async function visit(path) {
    if (!insideRoot(path)) throw new Error(`outside-root path refused: ${path}`);
    const metadata = await lstat(path);
    if (metadata.isSymbolicLink()) throw new Error(`symlink refused: ${slash(relative(ROOT, path))}`);
    if (metadata.isDirectory()) {
      const entries = await readdir(path, { withFileTypes: true });
      for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name, 'en'))) {
        if (SKIP_DIRS.has(entry.name)) continue;
        if (excludedName(entry.name)) continue;
        await visit(join(path, entry.name));
      }
      return;
    }
    if (!metadata.isFile()) return;
    const rel = slash(relative(ROOT, path)); const extension = extname(rel).toLowerCase();
    if (SKIP_EXTENSIONS.has(extension) || excludedName(rel.split('/').at(-1))) return;
    const data = await readFile(path);
    if (probablyBinary(data) && !ALLOWED_BINARY.has(extension)) throw new Error(`unexpected binary refused: ${rel}`);
    files.push({ path: rel, data });
  }
  for (const name of TOP_LEVEL) { const path = join(ROOT, name); try { await visit(path); } catch (error) { if (error.code !== 'ENOENT') throw error; } }
  for (const name of ['.gitignore', 'LICENSE', 'Toolchains.lock.json', 'Package.resolved', 'package-lock.json']) { const path = join(ROOT, name); try { await visit(path); } catch (error) { if (error.code !== 'ENOENT') throw error; } }
  return files.sort((a, b) => a.path.localeCompare(b.path, 'en'));
}
function octal(value, length) { const text = value.toString(8); if (text.length > length - 1) throw new Error(`tar field overflow: ${value}`); return `${text.padStart(length - 1, '0')}\0`; }
function put(header, offset, length, text) { const data = Buffer.from(text); if (data.length > length) throw new Error(`tar path or field too long: ${text}`); data.copy(header, offset); }
function splitTarPath(path) {
  if (Buffer.byteLength(path) <= 100) return ['', path];
  for (let index = path.lastIndexOf('/'); index > 0; index = path.lastIndexOf('/', index - 1)) { const prefix = path.slice(0, index); const name = path.slice(index + 1); if (Buffer.byteLength(prefix) <= 155 && Buffer.byteLength(name) <= 100) return [prefix, name]; }
  throw new Error(`tar path too long: ${path}`);
}
function tarHeader(path, size) {
  const [prefix, name] = splitTarPath(path); const header = Buffer.alloc(512);
  put(header, 0, 100, name); put(header, 100, 8, octal(0o644, 8)); put(header, 108, 8, octal(0, 8)); put(header, 116, 8, octal(0, 8)); put(header, 124, 12, octal(size, 12)); put(header, 136, 12, octal(0, 12));
  header.fill(0x20, 148, 156); header[156] = '0'.charCodeAt(0); put(header, 257, 6, 'ustar\0'); put(header, 263, 2, '00'); put(header, 265, 32, 'root'); put(header, 297, 32, 'root'); put(header, 329, 8, octal(0, 8)); put(header, 337, 8, octal(0, 8)); put(header, 345, 155, prefix);
  const checksum = [...header].reduce((sum, byte) => sum + byte, 0); put(header, 148, 8, `${checksum.toString(8).padStart(6, '0')}\0 `); return header;
}
function createTar(files) { const chunks = []; for (const file of files) { chunks.push(tarHeader(file.path, file.data.length), file.data); const padding = (512 - file.data.length % 512) % 512; if (padding) chunks.push(Buffer.alloc(padding)); } chunks.push(Buffer.alloc(1024)); return Buffer.concat(chunks); }
export async function createSourceBundle(outputPath) {
  const files = await gather(); const tar = createTar(files); const archive = gzipSync(tar, { level: 9, mtime: 0 });
  // Node/zlib reserves these header bytes for mtime. Set them explicitly so output is independent of runtime defaults.
  archive.fill(0, 4, 8);
  const manifest = { schemaVersion: 1, format: 'ustar+gzip', normalization: { order: 'UTF-8 lexical', fileMode: '0644', uid: 0, gid: 0, mtime: 0, gzipMtime: 0 }, archiveSha256: sha256(archive), files: files.map((file) => ({ path: file.path, size: file.data.length, sha256: sha256(file.data) })) };
  await mkdir(dirname(outputPath), { recursive: true }); const temporary = `${outputPath}.tmp-${process.pid}`; await writeFile(temporary, archive, { mode: 0o644 }); await rename(temporary, outputPath);
  await writeFile(`${outputPath}.sha256`, `${manifest.archiveSha256}  ${outputPath.split(sep).at(-1)}\n`, { mode: 0o644 }); await writeFile(`${outputPath}.manifest.json`, stable(manifest), { mode: 0o644 });
  return manifest;
}
async function main() {
  const args = process.argv.slice(2); let output = join(ROOT, 'Artifacts/source-bundle.tar.gz');
  if (args.length) { if (args.length !== 2 || args[0] !== '--output') throw new Error('usage: create-source-bundle.mjs [--output PATH]'); output = resolve(ROOT, args[1]); }
  if (!insideRoot(output)) throw new Error('output must be inside repository root');
  const manifest = await createSourceBundle(output); process.stdout.write(`${slash(relative(ROOT, output))} ${manifest.archiveSha256}\n`);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main().catch((error) => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
