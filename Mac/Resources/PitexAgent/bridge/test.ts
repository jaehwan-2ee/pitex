// Unit tests for the Codex bridge: notification mapping, approval replies,
// the session index, secret redaction, and a fake app-server round trip.
// Run: node --test Mac/Resources/PitexAgent/bridge/test.ts

import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import type { AdapterHost, PiCommand } from "./adapter.ts";
import { CodexAdapter, codexLaunchSpec, findCodex } from "./codex-adapter.ts";
import {
  approvalFromServerRequest,
  approvalRpcReply,
  createStreamState,
  itemsToPiMessages,
  mapNotification,
  modelsFromList,
  visibleModels,
} from "./codex-map.ts";
import { redactSecrets } from "./jsonrpc.ts";
import {
  appendUserMessage,
  rememberSession,
  threadIdFor,
  writeSessionPrelude,
} from "./session-index.ts";

async function waitForText(path: string, pattern: RegExp): Promise<string> {
  const deadline = Date.now() + 3000;
  let text = "";
  while (Date.now() < deadline) {
    text = readFileSync(path, "utf8");
    if (pattern.test(text)) return text;
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  return text;
}

function types(events: Record<string, unknown>[]): string[] {
  return events.map((event) => String(event.type));
}

test("app-server notifications become pi transcript events", () => {
  const state = createStreamState();
  const events = [
    ...mapNotification("turn/started", { turn: { id: "t1" } }, state),
    ...mapNotification("item/reasoning/summaryTextDelta", { itemId: "r1", delta: "plan" }, state),
    ...mapNotification("item/reasoning/textDelta", { itemId: "r1", delta: "hidden" }, state),
    ...mapNotification("item/agentMessage/delta", { itemId: "m1", delta: "Hello" }, state),
    ...mapNotification("item/agentMessage/delta", { itemId: "m1", delta: " there" }, state),
    ...mapNotification("item/started", { item: { type: "commandExecution", id: "c1", command: "latexmk" } }, state),
    ...mapNotification("item/commandExecution/outputDelta", { itemId: "c1", delta: "ok" }, state),
    ...mapNotification("item/completed", {
      item: { type: "commandExecution", id: "c1", command: "latexmk", aggregatedOutput: "ok", exitCode: 0, status: "completed" },
    }, state),
    ...mapNotification("item/completed", {
      item: {
        type: "fileChange",
        id: "f1",
        changes: [{ path: "main.tex", kind: { type: "update" }, diff: "@@ -1 +1 @@\n-a\n+b" }],
        status: "completed",
      },
    }, state),
    ...mapNotification("turn/completed", { turn: { id: "t1", status: "completed", error: null } }, state),
    ...mapNotification("server/futureNotification", {}, state),
  ];
  assert.deepEqual(types(events).filter((type) => type !== "message_update"), [
    "agent_start",
    "message_start",
    "tool_execution_start",
    "tool_execution_update",
    "tool_execution_end",
    "tool_execution_start",
    "tool_execution_end",
    "message_end",
    "turn_end",
    "agent_end",
  ]);
  const deltas = events
    .map((event) => event.assistantMessageEvent as { type?: string; delta?: string } | undefined)
    .filter((delta) => delta?.type === "thinking_delta" || delta?.type === "text_delta")
    .map((delta) => delta?.delta);
  assert.deepEqual(deltas, ["plan", "Hello", " there"]);
  const tool = events.find((event) => event.type === "tool_execution_end" && event.toolCallId === "f1");
  const result = tool?.result as { content: { text: string }[] };
  assert.match(result.content[0].text, /main\.tex/);
  assert.equal(state.turnOpen, false);
});

test("approval decisions map onto Codex replies", () => {
  const command = approvalFromServerRequest( "item/commandExecution/requestApproval", 7, { command: "latexmk main.tex" });
  assert.equal(command?.id, "7");
  assert.equal(command?.kind, "command");
  assert.equal(command?.detail, "latexmk main.tex");
  assert.deepEqual(approvalRpcReply(command!.method, "approve", {}).result, { decision: "accept" });
  assert.deepEqual(approvalRpcReply(command!.method, "always_allow", {}).result, { decision: "acceptForSession" });
  assert.deepEqual(approvalRpcReply(command!.method, "deny", {}).result, { decision: "decline" });
  assert.deepEqual(approvalRpcReply(command!.method, "nope", {}).result, { decision: "decline" });

  const legacy = approvalFromServerRequest("execCommandApproval", "req-1", { command: ["latexmk", "main.tex"] });
  assert.equal(legacy?.detail, "latexmk main.tex");
  assert.deepEqual(approvalRpcReply("execCommandApproval", "always_allow", {}).result, {
    decision: "approved_for_session",
  });
  assert.deepEqual(approvalRpcReply("applyPatchApproval", "deny", {}).result, {
    decision: { denied: { rejection: "The user denied the request." } },
  });

  const permissions = approvalFromServerRequest("item/permissions/requestApproval", 3, {
    reason: "network",
    permissions: { network: { enabled: true } },
  });
  assert.equal(permissions?.kind, "permissions");
  assert.deepEqual(approvalRpcReply(permissions!.method, "approve", { permissions: { network: { enabled: true } } }).result, {
    permissions: { network: { enabled: true } },
    scope: "turn",
  });
  assert.equal(approvalRpcReply(permissions!.method, "deny", {}).error?.message.includes("denied"), true);
  assert.equal(approvalFromServerRequest("account/chatgptAuthTokens/refresh", 1, {}), null);
});

test("thread history rebuilds pi messages and the model list hides hidden rows", () => {
  const messages = itemsToPiMessages([
    { type: "userMessage", content: [{ type: "text", text: "Fix the intro" }] },
    { type: "reasoning", id: "r", summary: ["plan"], content: [] },
    { type: "agentMessage", id: "m", text: "Done." },
    { type: "commandExecution", id: "c", command: "ls", aggregatedOutput: "main.tex", exitCode: 0 },
  ]);
  assert.deepEqual(messages.map((message) => message.role), ["user", "assistant", "toolResult"]);
  const models = modelsFromList({
    data: [
      {
        id: "hidden-one",
        displayName: "Hidden",
        hidden: true,
        isDefault: false,
        supportedReasoningEfforts: [],
        defaultReasoningEffort: "low",
      },
      {
        id: "gpt-test",
        displayName: "GPT Test",
        hidden: false,
        isDefault: true,
        supportedReasoningEfforts: [{ reasoningEffort: "low", description: "Low" }],
        defaultReasoningEffort: "low",
      },
    ],
    nextCursor: null,
  });
  assert.deepEqual(visibleModels(models), [{ id: "gpt-test", provider: "codex", name: "GPT Test" }]);
  assert.equal(models[1].defaultEffort, "low");
});

test("session index maps a Pitex file to a Codex thread and stays out of the history list", () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-index-"));
  try {
    const session = join(dir, "chat.jsonl");
    writeSessionPrelude(session, "<editor-context>\nx\n</editor-context>\n\n한글 질문", "/proj");
    appendUserMessage(session, "second");
    rememberSession(dir, session, "thread-42", "gpt-test");
    assert.equal(threadIdFor(dir, session), "thread-42");
    assert.equal(threadIdFor(dir, join(dir, "missing.jsonl")), null);
    const names = readFileSync(join(dir, "codex-session-index.json"), "utf8");
    assert.match(names, /thread-42/);
    assert.equal(join(dir, "codex-session-index.json").endsWith(".jsonl"), false);
    const lines = readFileSync(session, "utf8").trim().split("\n");
    assert.equal(lines.length, 3);
    assert.equal(JSON.parse(lines[0]).type, "session");
    assert.match(JSON.parse(lines[1]).message.content[0].text, /한글 질문/);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("token-shaped fields are dropped before any caller sees them", () => {
  const cleaned = redactSecrets({
    account: { type: "chatgpt", email: "a@b.c" },
    authToken: "secret-token",
    nested: [{ accessToken: "nope", ok: true }],
  }) as { authToken?: string; nested: { accessToken?: string; ok: boolean }[] };
  assert.equal(cleaned.authToken, undefined);
  assert.equal(cleaned.nested[0].accessToken, undefined);
  assert.equal(cleaned.nested[0].ok, true);
});

test("bridge sources do not read Codex credential files", () => {
  for (const name of ["cli.ts", "codex-adapter.ts", "codex-map.ts", "jsonrpc.ts", "session-index.ts", "adapter.ts"]) {
    const source = readFileSync(new URL(`./${name}`, import.meta.url), "utf8");
    assert.equal(source.includes("auth.json"), false, name);
    assert.equal(source.includes("\"getAuthStatus\""), false, name);
  }
  const adapter = readFileSync(new URL("./codex-adapter.ts", import.meta.url), "utf8");
  assert.match(adapter, /refreshToken: false/);
  assert.match(adapter, /approvalPolicy: "untrusted"/);
});

test("findCodex honors CODEX_PATH and ignores a missing binary", () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-which-"));
  try {
    const binary = join(dir, "codex");
    writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
    assert.equal(findCodex({ CODEX_PATH: binary, PATH: "", HOME: dir }), binary);
    assert.equal(findCodex({ PATH: dir, HOME: dir }), binary);
    assert.equal(findCodex({ CODEX_PATH: join(dir, "missing"), PATH: "", HOME: dir }), null);
    const exe = join(dir, "codex.exe");
    writeFileSync(exe, "", { mode: 0o755 });
    assert.equal(findCodex({ PATH: dir, HOME: dir }, "win32"), exe);
    const shim = join(dir, "shim-codex");
    writeFileSync(shim, "#!/usr/bin/env node\n");
    const spec = codexLaunchSpec(shim, "win32");
    assert.equal(spec.command, process.execPath);
    assert.deepEqual(spec.args, [shim, "app-server"]);
    assert.deepEqual(codexLaunchSpec(exe, "win32"), { command: exe, args: ["app-server"] });
    assert.deepEqual(codexLaunchSpec(shim, "linux"), { command: shim, args: ["app-server"] });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

function collectHost(): { host: AdapterHost; events: Record<string, unknown>[]; responses: Record<string, unknown>[]; waitFor: (type: string) => Promise<Record<string, unknown>> } {
  const events: Record<string, unknown>[] = [];
  const responses: Record<string, unknown>[] = [];
  let notify: (() => void) | null = null;
  const poke = () => {
    notify?.();
  };
  const host: AdapterHost = {
    emit(event) {
      events.push(event);
      poke();
    },
    respond(command, success, data, error, id) {
      responses.push({ command, success, data, error, id });
      poke();
    },
  };
  return {
    host,
    events,
    responses,
    waitFor(type: string) {
      const found = events.find((event) => event.type === type);
      if (found) return Promise.resolve(found);
      return new Promise((resolve, reject) => {
        const timer = setTimeout(
          () => reject(new Error(`timed out waiting for ${type}: ${JSON.stringify({ events, responses })}`)),
          3000
        );
        const check = () => {
          const match = events.find((event) => event.type === type);
          if (!match) return;
          clearTimeout(timer);
          notify = null;
          resolve(match);
        };
        notify = check;
        check();
      });
    },
  };
}

test("a fake app-server round-trips a prompt, an approval, and a resumed transcript", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-fake-"));
  const binary = join(dir, "codex");
  const record = join(dir, "replies.jsonl");
  const sessionDir = join(dir, "sessions");
  writeFileSync(binary, `#!/usr/bin/env node
import readline from "node:readline";
import { appendFileSync } from "node:fs";
const record = ${JSON.stringify(record)};
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
const send = (value) => process.stdout.write(JSON.stringify(value) + "\\n");
rl.on("line", (line) => {
  const msg = JSON.parse(line);
  if (!msg.method) {
    appendFileSync(record, line + "\\n");
    return;
  }
  if (!Object.prototype.hasOwnProperty.call(msg, "id")) return;
  if (msg.method === "initialize") send({ jsonrpc: "2.0", id: msg.id, result: { userAgent: "fake" } });
  else if (msg.method === "account/read") {
    send({ jsonrpc: "2.0", id: msg.id, result: { account: { type: "chatgpt", email: "a@b.c", planType: "plus" }, requiresOpenaiAuth: false, authToken: "must-not-leak" } });
  } else if (msg.method === "model/list") {
    send({ jsonrpc: "2.0", id: msg.id, result: { data: [{ id: "gpt-test", model: "gpt-test", displayName: "GPT Test", hidden: false, isDefault: true, supportedReasoningEfforts: [{ reasoningEffort: "low", description: "Low" }], defaultReasoningEffort: "low" }], nextCursor: null } });
  } else if (msg.method === "thread/start" || msg.method === "thread/resume") {
    send({ jsonrpc: "2.0", id: msg.id, result: { thread: { id: "thread-1", reasoningEffort: "low" }, model: "gpt-test" } });
  } else if (msg.method === "turn/start") {
    send({ jsonrpc: "2.0", id: msg.id, result: { turn: { id: "turn-1" } } });
    send({ jsonrpc: "2.0", method: "turn/started", params: { turn: { id: "turn-1" } } });
    send({ jsonrpc: "2.0", method: "item/agentMessage/delta", params: { itemId: "m1", delta: "Hello" } });
    send({ jsonrpc: "2.0", id: 42, method: "item/commandExecution/requestApproval", params: { command: "latexmk main.tex", threadId: "thread-1", turnId: "turn-1", itemId: "c1", kind: "command", startedAtMs: 1, environmentId: null } });
  } else if (msg.method === "thread/read") {
    send({ jsonrpc: "2.0", id: msg.id, result: { thread: { id: "thread-1", turns: [{ id: "turn-1", items: [
      { type: "userMessage", content: [{ type: "text", text: "Fix it" }] },
      { type: "agentMessage", id: "m1", text: "Hello" }
    ] }] } } });
  } else send({ jsonrpc: "2.0", id: msg.id, result: {} });
});
`, { mode: 0o755 });
  writeFileSync(record, "");
  const adapter = new CodexAdapter();
  const collected = collectHost();
  try {
    await adapter.start({
      cwd: dir,
      sessionDir,
      env: { ...process.env, CODEX_PATH: binary, PATH: process.env.PATH ?? "", FAKE_RECORD: record },
    });
    assert.equal(adapter.unavailableReason, null);
    await adapter.handleCommand({ type: "get_available_models" }, collected.host);
    const models = collected.responses.at(-1);
    assert.equal(models?.success, true);
    assert.deepEqual((models?.data as { models: { id: string }[] }).models.map((model) => model.id), ["gpt-test"]);

    await adapter.handleCommand({ type: "prompt", id: "p1", message: "Fix it" }, collected.host);
    const prompt = collected.responses.find((response) => response.command === "prompt");
    assert.equal(prompt?.success, true);
    const approval = await collected.waitFor("approval_request");
    assert.equal(approval.id, "42");
    assert.equal(approval.detail, "latexmk main.tex");
    assert.equal(JSON.stringify(collected.events).includes("must-not-leak"), false);

    await adapter.handleCommand({ type: "approval_response", id: "42", decision: "always_allow" }, collected.host);
    const recorded = await waitForText(record, /acceptForSession/);
    assert.match(recorded, /"id":42/);
    assert.match(recorded, /acceptForSession/);

    await adapter.handleCommand({ type: "get_messages" }, collected.host);
    const history = collected.responses.find((response) => response.command === "get_messages");
    const messages = (history?.data as { messages: { role: string }[] }).messages;
    assert.deepEqual(messages.map((message) => message.role), ["user", "assistant"]);
    const files = readFileSync(join(sessionDir, "codex-session-index.json"), "utf8");
    assert.match(files, /thread-1/);
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a missing or signed-out Codex CLI is reported without exiting", async () => {
  const missing = new CodexAdapter();
  const empty = mkdtempSync(join(tmpdir(), "pitex-bridge-empty-"));
  await missing.start({ cwd: empty, sessionDir: empty, env: { PATH: "", HOME: empty, CODEX_PATH: "" } });
  assert.match(missing.unavailableReason ?? "", /codex login/);
  const host = collectHost();
  await missing.handleCommand({ type: "prompt", message: "Hi", id: "p" }, host.host);
  assert.equal(host.responses[0].success, false);
  assert.match(String(host.responses[0].error), /codex login/);

  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-signed-out-"));
  const binary = join(dir, "codex");
  writeFileSync(binary, `#!/usr/bin/env node
import readline from "node:readline";
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
const send = (value) => process.stdout.write(JSON.stringify(value) + "\\n");
rl.on("line", (line) => {
  const msg = JSON.parse(line);
  if (msg.method === "initialize") send({ jsonrpc: "2.0", id: msg.id, result: {} });
  else if (msg.method === "account/read") send({ jsonrpc: "2.0", id: msg.id, result: { account: null, requiresOpenaiAuth: true } });
  else if (msg.id != null) send({ jsonrpc: "2.0", id: msg.id, result: {} });
});
`, { mode: 0o755 });
  const signedOut = new CodexAdapter();
  try {
    await signedOut.start({ cwd: dir, sessionDir: dir, env: { ...process.env, CODEX_PATH: binary } });
    assert.match(signedOut.unavailableReason ?? "", /not logged in/);
  } finally {
    await signedOut.stop();
    rmSync(dir, { recursive: true, force: true });
    rmSync(empty, { recursive: true, force: true });
  }
});
