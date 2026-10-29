#!/usr/bin/env node
// pitex-agent-bridge — pi RPC JSONL on stdin/stdout, one official CLI behind it.
// Milestone 1 selects the Codex adapter. The host (Pitex) discovers bun or
// node the same way it launches pi, then runs this file.
//
// PITEX_SESSION_DIR              per-project session folder
// PITEX_DEVELOPER_INSTRUCTIONS   editor instructions forwarded to the CLI
// PITEX_CLI_ADAPTER              adapter id: "codex" (default), "claude", or "grok"
// CODEX_PATH                     optional explicit Codex CLI binary
// CLAUDE_PATH                    optional explicit Claude Code CLI binary
// GROK_PATH                      optional explicit Grok CLI binary

import readline from "node:readline";

import { createAdapter, type AdapterHost, type PiCommand } from "./adapter.ts";

function write(value: unknown): void {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

const host: AdapterHost = {
  emit(event) {
    write(event);
  },
  respond(command, success, data, error, id) {
    const message: Record<string, unknown> = { type: "response", command, success };
    if (data !== undefined) message.data = data;
    if (error) message.error = error;
    if (id) message.id = id;
    write(message);
  },
};

const adapter = await createAdapter(process.env.PITEX_CLI_ADAPTER || "codex");
await adapter.start({
  cwd: process.cwd(),
  sessionDir: process.env.PITEX_SESSION_DIR || process.cwd(),
  developerInstructions: process.env.PITEX_DEVELOPER_INSTRUCTIONS,
  env: process.env,
});

if (adapter.unavailableReason) {
  write({ type: "extension_error", error: adapter.unavailableReason });
}

const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
lines.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;
  let command: PiCommand;
  try {
    command = JSON.parse(trimmed) as PiCommand;
  } catch {
    return;
  }
  if (!command || typeof command.type !== "string") return;
  void adapter.handleCommand(command, host).catch((error: unknown) => {
    const message = error instanceof Error ? error.message : "The bridge command failed.";
    host.respond(command.type, false, undefined, message, command.id);
  });
});

lines.on("close", () => {
  void adapter.stop().finally(() => process.exit(0));
});
