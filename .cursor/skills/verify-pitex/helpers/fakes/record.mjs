import { appendFileSync } from "node:fs";

const INTEREST = /^(ANTHROPIC_|CLAUDE_|AWS_BEARER_)/;

export function recordSpawn() {
  const path = process.env.FAKE_RECORD;
  if (!path) return;
  const envPresent = {};
  for (const key of Object.keys(process.env)) {
    if (INTEREST.test(key)) envPresent[key] = true;
  }
  appendFileSync(
    path,
    `${JSON.stringify({
      kind: "spawn",
      pid: process.pid,
      cwd: process.cwd(),
      argv: process.argv.slice(2),
      envPresent,
    })}\n`
  );
}

export function recordLine(line) {
  const path = process.env.FAKE_RECORD;
  if (!path) return;
  appendFileSync(path, `${JSON.stringify({ kind: "stdin", line })}\n`);
}

export function recordNote(note) {
  const path = process.env.FAKE_RECORD;
  if (!path) return;
  appendFileSync(path, `${JSON.stringify({ kind: "note", ...note })}\n`);
}
