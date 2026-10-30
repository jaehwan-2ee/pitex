// Per-project map from a Pitex session file to a CLI conversation id
// (Codex thread id, Claude session id, Grok session id, Antigravity
// conversation id). The history list keeps scanning *.jsonl; this index is not
// a jsonl file, so it never shows up as its own conversation.

import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export const INDEX_NAME = "codex-session-index.json";

export interface SessionRecord {
  threadId: string;
  model?: string;
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
  model?: string
): void {
  const index = loadIndex(sessionDir);
  index.sessions[sessionKey(sessionPath)] = {
    threadId,
    ...(model ? { model } : {}),
    updatedAt: new Date().toISOString(),
  };
  saveIndex(sessionDir, index);
}

export function threadIdFor(sessionDir: string, sessionPath: string): string | null {
  const record = loadIndex(sessionDir).sessions[sessionKey(sessionPath)];
  return record?.threadId || null;
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

/** Messages this bridge wrote into a session file. Skips the session header. */
export function sessionMessages(sessionPath: string): Record<string, unknown>[] {
  let text = "";
  try {
    text = readFileSync(sessionPath, "utf8");
  } catch {
    return [];
  }
  const messages: Record<string, unknown>[] = [];
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      const parsed = JSON.parse(trimmed) as { type?: string; message?: Record<string, unknown> };
      if (parsed.type === "message" && parsed.message && typeof parsed.message === "object") {
        messages.push(parsed.message);
      }
    } catch {
      // A torn final line is ignored; the next append rewrites nothing before it.
    }
  }
  return messages;
}
