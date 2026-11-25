// Per-project map from a Pitex session file to a CLI conversation id
// (Codex thread id, Claude session id, Grok or Antigravity ACP session id).
// The history list keeps scanning *.jsonl; this index is not a jsonl file,
// so it never shows up as its own conversation.

import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export const INDEX_NAME = "codex-session-index.json";

export interface SessionRecord {
  threadId: string;
  model?: string;
  /** Permission mode the user picked for this conversation; absent means default. */
  mode?: string;
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
  mode?: string
): void {
  const index = loadIndex(sessionDir);
  index.sessions[sessionKey(sessionPath)] = {
    threadId,
    ...(model ? { model } : {}),
    ...(mode && mode !== "default" ? { mode } : {}),
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

/** A pi-shaped session head so the existing history list can title the chat. */
export function writeSessionPrelude(sessionPath: string, userText: string, cwd: string): void {
  mkdirSync(dirname(sessionPath), { recursive: true });
  const header = {
    type: "session",
    version: 1,
    id: sessionKey(sessionPath),
    timestamp: new Date().toISOString(),
    cwd,
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
