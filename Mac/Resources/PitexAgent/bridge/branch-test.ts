// Subagent nesting, fork_session and rewind for Claude Code (M9).

import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

import type { AdapterHost } from "./adapter.ts";
import { ClaudeAdapter, insideProject, type ClaudeQueryFn, type ClaudeQueryOptions, type ClaudeSessionApi } from "./claude-adapter.ts";
import { claudePromptCount, claudeTurnCut, createClaudeStreamState, keepTurnsOf, mapClaudeMessage, replayClaudeMessages, type Json } from "./claude-map.ts";
import { threadIdFor } from "./session-index.ts";

const AGENT = "toolu_agent";

/** The shape a real `claude` 2.1.331 run sent for an Explore subagent with two calls (trimmed). */
function subagentRun(): Json[] {
  const child = (id: string, name: string, input: Json) => ({
    type: "assistant",
    parent_tool_use_id: AGENT,
    message: { model: "claude-haiku-4-5", content: [{ type: "text", text: "Looking around." }, { type: "tool_use", id, name, input }] },
  });
  const result = (id: string, text: string, parent: string | null) => ({
    type: "user",
    parent_tool_use_id: parent,
    message: { role: "user", content: [{ type: "tool_result", tool_use_id: id, content: text }] },
  });
  return [
    {
      type: "assistant",
      parent_tool_use_id: null,
      message: { model: "claude-opus-5-5", content: [{ type: "tool_use", id: AGENT, name: "Agent", input: { description: "Find word after alpha", subagent_type: "Explore", prompt: "Look" } }] },
    },
    { type: "user", parent_tool_use_id: AGENT, message: { role: "user", content: [{ type: "text", text: "Look" }] } },
    { type: "stream_event", parent_tool_use_id: AGENT, event: { type: "content_block_delta", index: 0, delta: { type: "text_delta", text: "sub stream" } } },
    child("toolu_glob", "Glob", { pattern: "**/notes.txt" }),
    result("toolu_glob", "notes.txt", AGENT),
    child("toolu_read", "Read", { file_path: "/p/notes.txt" }),
    result("toolu_read", "1\talpha beta", AGENT),
    result(AGENT, "The word is beta.", null),
    { type: "assistant", parent_tool_use_id: null, message: { model: "claude-opus-5-5", content: [{ type: "text", text: "beta" }] } },
    { type: "result", subtype: "success", result: "beta" },
  ];
}

test("a Claude subagent's calls carry parentToolCallId, its card counts them, and its text stays out of the answer", () => {
  const state = createClaudeStreamState();
  const events = subagentRun().flatMap((message) => mapClaudeMessage(message, state));
  const starts = events.filter((event) => event.type === "tool_execution_start");
  assert.deepEqual(starts.map((event) => [event.toolCallId, event.parentToolCallId]), [
    [AGENT, undefined],
    ["toolu_glob", AGENT],
    ["toolu_read", AGENT],
  ]);
  assert.deepEqual(starts[0].detail, { kind: "sub_agent", label: "Find word after alpha", description: "Find word after alpha", subAgentType: "Explore" });
  const counts = events.filter((event) => event.type === "tool_execution_update" && event.toolCallId === AGENT).map((event) => (event.detail as Json).toolUses);
  assert.deepEqual(counts, [1, 2]);
  const ends = events.filter((event) => event.type === "tool_execution_end");
  assert.deepEqual(ends.map((event) => [event.toolCallId, event.parentToolCallId ?? null]), [
    ["toolu_glob", AGENT],
    ["toolu_read", AGENT],
    [AGENT, null],
  ]);
  assert.equal((ends[2].detail as Json).toolUses, 2);
  const text = events
    .filter((event) => event.type === "message_update")
    .map((event) => (event.assistantMessageEvent as Json).delta ?? "")
    .join("");
  assert.equal(text, "beta", "neither the subagent's text nor its stream reaches the answer");
  // The model of the answer is the main one, not the subagent's.
  assert.equal(state.model, "claude-opus-5-5");
  // get_messages keeps the nesting for a reopened chat.
  const rows = state.transcript.filter((row) => row.role === "toolResult");
  assert.deepEqual(rows.map((row) => [row.toolCallId, row.parentToolCallId ?? null]), [
    ["toolu_glob", AGENT],
    ["toolu_read", AGENT],
    [AGENT, null],
  ]);
  const calls = state.transcript.flatMap((row) => (Array.isArray(row.content) ? (row.content as Json[]) : [])).filter((block) => block.type === "toolCall");
  assert.deepEqual(calls.map((block) => block.parentToolCallId ?? null), [null, AGENT, AGENT]);
});

const SAVED: Json[] = [
  { type: "user", uuid: "meta", is_meta: true, message: { role: "user", content: "Caveat" } },
  { type: "user", uuid: "u1", message: { role: "user", content: "First" } },
  { type: "assistant", uuid: "a1", message: { role: "assistant", content: [{ type: "text", text: "One" }] } },
  { type: "user", uuid: "u2", message: { role: "user", content: [{ type: "text", text: "Second" }] } },
  { type: "assistant", uuid: "a2t", message: { role: "assistant", content: [{ type: "tool_use", id: "e", name: "Edit", input: {} }] } },
  { type: "user", uuid: "r2", message: { role: "user", content: [{ type: "tool_result", tool_use_id: "e", content: "ok" }] } },
  { type: "assistant", uuid: "a2", message: { role: "assistant", content: [{ type: "text", text: "Two" }] } },
];

test("turn cuts find the last kept message and the first dropped prompt", () => {
  assert.deepEqual(claudeTurnCut(SAVED, 0), { keepUuid: "meta", dropPromptUuid: "u1" });
  assert.deepEqual(claudeTurnCut(SAVED, 1), { keepUuid: "a1", dropPromptUuid: "u2" });
  assert.deepEqual(claudeTurnCut(SAVED, 2), { keepUuid: "a2", dropPromptUuid: null });
  const pi: Json[] = [{ role: "user" }, { role: "assistant" }, { role: "user" }, { role: "toolResult" }];
  assert.deepEqual(keepTurnsOf(pi, 1), pi.slice(0, 2));
  assert.deepEqual(keepTurnsOf(pi, 0), []);
  assert.deepEqual(keepTurnsOf(pi, 5), pi);
});

test("a slash command is a turn, as the live chat shows it, so cuts after it land on the right prompt", () => {
  const rows: Json[] = [
    { type: "user", uuid: "u1", message: { role: "user", content: "hi" } },
    { type: "assistant", uuid: "a1", message: { role: "assistant", content: [{ type: "text", text: "Hello" }] } },
    // Claude Code 2.1.331's record of "/review main.tex", then the expanded prompt it sent (meta).
    { type: "user", uuid: "c2", message: { role: "user", content: "<command-message>review is running…</command-message>\n<command-name>/review</command-name>\n<command-args>main.tex</command-args>" } },
    { type: "user", uuid: "m2", is_meta: true, message: { role: "user", content: [{ type: "text", text: "Review main.tex …" }] } },
    { type: "assistant", uuid: "a2", message: { role: "assistant", content: [{ type: "text", text: "Looks fine" }] } },
    { type: "user", uuid: "u3", message: { role: "user", content: "do X" } },
    { type: "assistant", uuid: "a3", message: { role: "assistant", content: [{ type: "text", text: "Did X" }] } },
    { type: "user", uuid: "o3", message: { role: "user", content: "<local-command-stdout>ok</local-command-stdout>" } },
  ];
  assert.equal(claudePromptCount(rows), 3);
  assert.deepEqual(claudeTurnCut(rows, 2), { keepUuid: "a2", dropPromptUuid: "u3" }, "Rewind on \"do X\" drops it");
  const shown = replayClaudeMessages(rows).transcript.filter((row) => row.role === "user");
  assert.deepEqual(shown.map((row) => (row.content as Json[])[0].text), ["hi", "/review main.tex", "do X"]);
});

test("only paths inside the project count as inside", () => {
  const root = mkdtempSync(join(tmpdir(), "pitex-inside-"));
  try {
    assert.equal(insideProject(root, join(root, "main.tex")), true);
    assert.equal(insideProject(root, "chapters/new.tex"), true, "a file the rewind would recreate");
    assert.equal(insideProject(root, join(root, "..", "elsewhere.tex")), false);
    assert.equal(insideProject(root, "/etc/hosts"), false);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

/** A streaming query that answers each prompt with one text reply, and records rewindFiles calls. */
class FakeQuery {
  rewinds: [string, unknown][] = [];
  private queue: unknown[] = [];
  private waiter: ((result: IteratorResult<unknown>) => void) | null = null;
  private ended = false;
  readonly options: ClaudeQueryOptions;
  private readonly changed: () => string[];
  constructor(options: ClaudeQueryOptions, changed: () => string[]) {
    this.options = options;
    this.changed = changed;
  }
  push(message: unknown): void {
    if (this.ended) return;
    if (this.waiter) {
      const waiter = this.waiter;
      this.waiter = null;
      waiter({ value: message, done: false });
    } else this.queue.push(message);
  }
  next(): Promise<IteratorResult<unknown>> {
    const queued = this.queue.shift();
    if (queued !== undefined) return Promise.resolve({ value: queued, done: false });
    if (this.ended) return Promise.resolve({ value: undefined, done: true });
    return new Promise((resolve) => (this.waiter = resolve));
  }
  [Symbol.asyncIterator](): AsyncIterator<unknown> {
    return this;
  }
  async interrupt(): Promise<void> {}
  async supportedModels() {
    return [];
  }
  async rewindFiles(id: string, options?: { dryRun?: boolean }) {
    this.rewinds.push([id, options]);
    // Like Claude Code 2.1.331: only the dry run lists the files.
    return options?.dryRun ? { canRewind: true, filesChanged: this.changed() } : { canRewind: true };
  }
  close(): void {
    this.ended = true;
    this.waiter?.({ value: undefined, done: true });
    this.waiter = null;
  }
}

function fakeClaude(opened: FakeQuery[], changed: () => string[], hold: { on: boolean }): ClaudeQueryFn {
  return ({ prompt, options }) => {
    const query = new FakeQuery(options, changed);
    opened.push(query);
    query.push({ type: "system", subtype: "init", session_id: options.resume ?? "sess-1", model: "claude-sonnet-4-5" });
    void (async () => {
      for await (const message of prompt) {
        const text = String(((message as Json).message as Json).content);
        query.push({ type: "assistant", message: { content: [{ type: "text", text: `Re: ${text}` }] } });
        if (!hold.on) query.push({ type: "result", subtype: "success" });
      }
    })();
    return query as unknown as ReturnType<ClaudeQueryFn>;
  };
}

function collect(): { host: AdapterHost; events: Json[]; responses: Json[] } {
  const events: Json[] = [];
  const responses: Json[] = [];
  return {
    events,
    responses,
    host: {
      emit: (event) => void events.push(event),
      respond: (command, success, data, error, id) => void responses.push({ command, success, data, error, id }),
    },
  };
}

async function until(predicate: () => boolean): Promise<void> {
  const deadline = Date.now() + 3000;
  while (!predicate()) {
    if (Date.now() > deadline) throw new Error("timed out");
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}

async function twoTurns(changed: () => string[] = () => []) {
  const dir = mkdtempSync(join(tmpdir(), "pitex-claude-branch-"));
  const binary = join(dir, "claude");
  writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
  const opened: FakeQuery[] = [];
  const hold = { on: false };
  const forks: [string, Json][] = [];
  const saved: Record<string, Json[]> = { "sess-1": SAVED, "forked-1": SAVED.slice(0, 3) };
  const sessions: ClaudeSessionApi = {
    listSessions: async () => [],
    getSessionMessages: async (id) => saved[id] ?? [],
    forkSession: async (id, options) => {
      forks.push([id, options as unknown as Json]);
      return { sessionId: "forked-1" };
    },
  };
  const adapter = new ClaudeAdapter({ query: fakeClaude(opened, changed, hold), sessions, authStatus: async () => ({ stdout: '{"loggedIn":true}', code: 0 }) });
  const collected = collect();
  await adapter.start({ cwd: dir, sessionDir: dir, env: { PATH: "", HOME: dir, CLAUDE_PATH: binary } });
  for (const message of ["First", "Second"]) {
    const ends = collected.events.filter((event) => event.type === "agent_end").length;
    await adapter.handleCommand({ type: "prompt", id: message, message }, collected.host);
    await until(() => collected.events.filter((event) => event.type === "agent_end").length > ends);
  }
  const reply = (command: string) => [...collected.responses].reverse().find((item) => item.command === command) as Json;
  return { dir, adapter, opened, hold, forks, saved, collected, reply };
}

test("fork_session copies the first turns into a new Claude session and switches to it; the original stays", async () => {
  const { dir, adapter, opened, forks, collected, reply } = await twoTurns();
  try {
    await adapter.handleCommand({ type: "get_state", id: "s" }, collected.host);
    assert.deepEqual((reply("get_state").data as Json).capabilities, { steer: false, images: true, fork: true, rewind: { conversation: true, files: true } });
    const original = readFileSync(join(dir, (await import("node:fs")).readdirSync(dir).find((name) => name.startsWith("claude-") && name.endsWith(".jsonl"))!), "utf8");

    // The host shows a third prompt the bridge never saw (a crash restart): no cut at all.
    await adapter.handleCommand({ type: "fork_session", id: "fx", keepTurns: 1, turns: 3 }, collected.host);
    assert.equal(reply("fork_session").success, false);
    assert.match(String(reply("fork_session").error), /no longer matches/);
    assert.deepEqual(forks, []);

    await adapter.handleCommand({ type: "fork_session", id: "f", keepTurns: 1, turns: 2 }, collected.host);
    const forked = reply("fork_session");
    assert.equal(forked.success, true, String(forked.error));
    assert.deepEqual(forks, [["sess-1", { dir, upToMessageId: "a1" }]]);
    const path = (forked.data as Json).sessionPath as string;
    assert.equal(resolve(path), join(dir, "claude-forked-1.jsonl"));
    const mirror = readFileSync(path, "utf8");
    assert.match(mirror, /First/);
    assert.doesNotMatch(mirror, /Second/);
    assert.equal(threadIdFor(dir, path), "forked-1");
    assert.equal(opened.at(-1)?.options.resume, "forked-1", "the next turn continues the fork");
    const kept = readFileSync(join(dir, (await import("node:fs")).readdirSync(dir).find((name) => name.startsWith("claude-") && name !== "claude-forked-1.jsonl" && name.endsWith(".jsonl"))!), "utf8");
    assert.equal(kept, original, "the original mirror is untouched");

    await adapter.handleCommand({ type: "fork_session", id: "f0", keepTurns: 0 }, collected.host);
    assert.equal(reply("fork_session").success, false, "nothing to fork before the first message");
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

test("rewind lists files first, restores them and the conversation, and refuses while a turn runs", async () => {
  let changed: string[] = [];
  const { dir, adapter, opened, hold, collected, reply } = await twoTurns(() => changed);
  changed = [join(dir, "main.tex")];
  try {
    await adapter.handleCommand({ type: "rewind", id: "d", keepTurns: 1, files: true, dryRun: true }, collected.host);
    assert.deepEqual(reply("rewind").data, { filesChanged: changed, conversation: false, dryRun: true });
    assert.deepEqual(opened.at(-1)?.rewinds, [["u2", { dryRun: true }]], "a dry run restores nothing");

    await adapter.handleCommand({ type: "rewind", id: "r", keepTurns: 1, files: true, conversation: true }, collected.host);
    assert.equal(reply("rewind").success, true, String(reply("rewind").error));
    assert.deepEqual(reply("rewind").data, { filesChanged: changed, conversation: true });
    const before = opened.at(-2)!;
    assert.deepEqual(before.rewinds.slice(-2), [["u2", { dryRun: true }], ["u2", undefined]]);
    assert.equal(opened.at(-1)?.options.resumeSessionAt, "a1", "the conversation resumes after the first turn");
    assert.equal(opened.at(-1)?.options.enableFileCheckpointing, true);
    await adapter.handleCommand({ type: "get_messages", id: "m" }, collected.host);
    const roles = ((reply("get_messages").data as Json).messages as Json[]).map((row) => row.role);
    assert.deepEqual(roles, ["user", "assistant"]);

    // The next prompt branches from the rewind point; later restarts resume from its own end.
    await adapter.handleCommand({ type: "prompt", id: "p3", message: "Third" }, collected.host);
    await until(() => collected.events.filter((event) => event.type === "agent_end").length === 3);
    assert.equal(JSON.parse(readFileSync(join(dir, "codex-session-index.json"), "utf8")).sessions[Object.keys(JSON.parse(readFileSync(join(dir, "codex-session-index.json"), "utf8")).sessions)[0]].resumeAt, undefined);

    hold.on = true;
    await adapter.handleCommand({ type: "prompt", id: "p4", message: "Fourth" }, collected.host);
    await adapter.handleCommand({ type: "rewind", id: "busy", keepTurns: 1, files: true, conversation: true }, collected.host);
    assert.equal(reply("rewind").success, false);
    assert.match(String(reply("rewind").error), /Wait for the current turn/);
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

test("rewind refuses, and restores nothing, when Claude changed a file outside the project", async () => {
  const { dir, adapter, opened, collected, reply } = await twoTurns(() => ["/etc/hosts", "main.tex"]);
  try {
    await adapter.handleCommand({ type: "rewind", id: "r", keepTurns: 1, files: true, conversation: true }, collected.host);
    assert.equal(reply("rewind").success, false);
    assert.match(String(reply("rewind").error), /inside the project.*\/etc\/hosts/);
    assert.deepEqual(opened.at(-1)?.rewinds, [["u2", { dryRun: true }]], "only the dry run ran");
    assert.equal(opened.at(-1)?.options.resumeSessionAt, undefined, "the conversation was not rewound either");
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

test("an Antigravity invoke_subagent call is a subagent card (ACP sends no parent ids, so nothing nests)", async () => {
  const { createStreamState, mapSessionUpdate } = await import("./acp-map.ts");
  const state = createStreamState();
  const events = mapSessionUpdate(
    {
      sessionUpdate: "tool_call",
      toolCallId: "call_1",
      name: "invoke_subagent",
      status: "in_progress",
      kind: "other",
      title: "Run invoke_subagent",
      rawInput: { Prompt: "Find the word after alpha", Role: "File Lookup Researcher", TypeName: "research" },
    },
    state
  ) as Json[];
  const start = events.find((event) => event.type === "tool_execution_start");
  assert.deepEqual(start?.detail, { kind: "sub_agent", label: "File Lookup Researcher", description: "File Lookup Researcher", subAgentType: "research" });
  assert.equal(start?.parentToolCallId, undefined);
});
