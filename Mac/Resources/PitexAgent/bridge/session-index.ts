// Per-project map from a Pitex session file to a CLI conversation id
// (Codex thread id, Claude session id, Grok or Antigravity ACP session id).
// The history list keeps scanning *.jsonl; this index is not a jsonl file,
// so it never shows up as its own conversation.

import { existsSync, mkdirSync, readFileSync, realpathSync, renameSync, statSync, utimesSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { storedTodos, type TodoItem } from "./todos.ts";

export const INDEX_NAME = "codex-session-index.json";

export interface SessionRecord {
  threadId: string;
  model?: string;
  /** Permission mode the user picked for this conversation; absent means default. */
  mode?: string;
  /** The last todo list, for CLIs whose own history does not replay it (Claude Code, Codex). */
  todos?: TodoItem[];
  updatedAt: string;
}

export interface SessionIndex {
  version: 1;
  sessions: Record<string, SessionRecord>;
}

export function emptyIndex(): SessionIndex {
  return { version: 1, sessions: {} };
}

export function indexPath(sessionDir: string): string {
  return join(sessionDir, INDEX_NAME);
}

export function loadIndex(sessionDir: string): SessionIndex {
  try {
    const parsed = JSON.parse(readFileSync(indexPath(sessionDir), "utf8")) as SessionIndex;
    if (!parsed || parsed.version !== 1 || !parsed.sessions || typeof parsed.sessions !== "object") {
      return emptyIndex();
    }
    return parsed;
  } catch {
    return emptyIndex();
  }
}

export function saveIndex(sessionDir: string, index: SessionIndex): void {
  mkdirSync(sessionDir, { recursive: true });
  const target = indexPath(sessionDir);
  const tmp = `${target}.${process.pid}.tmp`;
  writeFileSync(tmp, `${JSON.stringify(index, null, 2)}\n`, "utf8");
  renameSync(tmp, target);
}

export function sessionKey(sessionPath: string): string {
  return resolve(sessionPath);
}

export function rememberSession(
  sessionDir: string,
  sessionPath: string,
  threadId: string,
  model?: string,
  mode?: string,
  todos?: TodoItem[] | null
): void {
  const index = loadIndex(sessionDir);
  index.sessions[sessionKey(sessionPath)] = {
    threadId,
    ...(model ? { model } : {}),
    ...(mode && mode !== "default" ? { mode } : {}),
    ...(todos && todos.length > 0 ? { todos } : {}),
    updatedAt: new Date().toISOString(),
  };
  saveIndex(sessionDir, index);
}

export function threadIdFor(sessionDir: string, sessionPath: string): string | null {
  const record = loadIndex(sessionDir).sessions[sessionKey(sessionPath)];
  return record?.threadId || null;
}

export function sessionModeFor(sessionDir: string, sessionPath: string): string | null {
  return loadIndex(sessionDir).sessions[sessionKey(sessionPath)]?.mode || null;
}

export function sessionTodosFor(sessionDir: string, sessionPath: string): TodoItem[] | null {
  return storedTodos(loadIndex(sessionDir).sessions[sessionKey(sessionPath)]?.todos);
}

/** A pi-shaped session head so the existing history list can title the chat. */
export function writeSessionPrelude(sessionPath: string, userText: string, cwd: string): void {
  mkdirSync(dirname(sessionPath), { recursive: true });
  const source = sessionBackend(sessionPath);
  const header = {
    type: "session",
    version: 1,
    id: sessionKey(sessionPath),
    timestamp: new Date().toISOString(),
    cwd,
    // The backend that owns the conversation; hosts badge history rows from the file prefix.
    ...(source ? { source } : {}),
  };
  const message = {
    type: "message",
    message: { role: "user", content: [{ type: "text", text: userText }] },
  };
  writeFileSync(sessionPath, `${JSON.stringify(header)}\n${JSON.stringify(message)}\n`, "utf8");
}

export function appendSessionMessage(sessionPath: string, message: Record<string, unknown>): void {
  const line = { type: "message", message };
  writeFileSync(sessionPath, `${JSON.stringify(line)}\n`, { encoding: "utf8", flag: "a" });
}

export function appendUserMessage(sessionPath: string, userText: string): void {
  appendSessionMessage(sessionPath, {
    role: "user",
    content: [{ type: "text", text: userText }],
  });
}

/** Bridge backends whose session files are named `<id>-<uuid>.jsonl`. */
const BRIDGE_BACKENDS = ["claude", "codex", "grok", "agy"];

/** The bridge backend that wrote a session file, or null for any other name (pi's own files). */
export function sessionBackend(sessionPath: string): string | null {
  const name = sessionPath.split(/[\\/]/).pop() ?? "";
  return BRIDGE_BACKENDS.find((id) => name.startsWith(`${id}-`)) ?? null;
}

export function foreignSessionMessage(owner: string): string {
  return `That conversation was started with another assistant (${owner}). Choose it in Settings to continue the conversation.`;
}

/** Most CLI sessions mirrored per scan (newest first). */
export const SESSION_SCAN_LIMIT = 500;
const SAFE_SESSION_ID = /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/;

/** Same folder on disk, so `/tmp/x` and its realpath compare equal. */
function canonicalDir(path: string): string {
  try {
    return realpathSync(resolve(path));
  } catch {
    return resolve(path);
  }
}

/**
 * Mirror files for the CLI's own sessions in this project, so the history
 * list (which scans `*.jsonl`) shows them. Each row is `{sessionId, cwd,
 * title, updatedAt}` (ACP session/list, or Claude's listSessions mapped to
 * it). A row is skipped when its cwd is another folder; a row already in the
 * session index only moves its file's mtime forward to the CLI's last
 * activity, so a rescan writes nothing. An existing file is never
 * overwritten. Returns the files written.
 */
export function importListedSessions(sessionDir: string, prefix: string, rows: unknown[], projectRoot: string): string[] {
  const root = canonicalDir(projectRoot);
  const index = loadIndex(sessionDir);
  const pathById = new Map(Object.entries(index.sessions).map(([path, record]) => [record.threadId, path]));
  const written: string[] = [];
  let changed = false;
  for (const raw of rows) {
    const row = raw && typeof raw === "object" ? (raw as Record<string, unknown>) : {};
    const id = typeof row.sessionId === "string" ? row.sessionId : "";
    const cwd = typeof row.cwd === "string" ? row.cwd : "";
    if (!id || !cwd || !SAFE_SESSION_ID.test(id) || canonicalDir(cwd) !== root) continue;
    const updated = Date.parse(typeof row.updatedAt === "string" ? row.updatedAt : "");
    const known = pathById.get(id);
    if (known) {
      // Continued in a terminal since the last scan: the row moves up the list.
      touchForward(known, updated);
      continue;
    }
    const path = join(sessionDir, `${prefix}-${id}.jsonl`);
    pathById.set(id, path);
    // One index write per scan instead of a rememberSession call per row.
    index.sessions[sessionKey(path)] = { threadId: id, updatedAt: new Date().toISOString() };
    changed = true;
    if (existsSync(path)) continue;
    const title = typeof row.title === "string" ? row.title.trim() : "";
    writeSessionPrelude(path, title || "Untitled conversation", projectRoot);
    if (Number.isFinite(updated)) utimesSync(path, new Date(updated), new Date(updated));
    written.push(path);
  }
  if (changed) saveIndex(sessionDir, index);
  return written;
}

function touchForward(path: string, updated: number): void {
  if (!Number.isFinite(updated)) return;
  try {
    if (statSync(path).mtimeMs < updated) utimesSync(path, new Date(updated), new Date(updated));
  } catch {
    // The mirror was deleted; the index entry stays so it is not re-imported.
  }
}
