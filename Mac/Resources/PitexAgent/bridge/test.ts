// Unit tests for the agent bridge: Codex notification mapping, Antigravity
// stream-json mapping, approval replies, the session index, secret redaction,
// and fake-process round trips.
// Run: node --test Mac/Resources/PitexAgent/bridge/test.ts

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import "./grok-test.ts";

import { createAdapter, type AdapterHost, type PiCommand } from "./adapter.ts";
import { AgyAdapter, agyLaunchSpec, agyStreamArgs, findAgy } from "./agy-adapter.ts";
import {
  AGY_ALWAYS_PROCEED_NOTICE,
  AGY_KEYRING_LOCKED,
  AGY_PERMISSION_LIMIT,
  AGY_SIGNED_OUT,
  approvalIsNotDelivered,
  authFailureReason,
  createAgyStreamState,
  decisionLabel,
  mapAgyLine,
  modelsFromText,
  permissionDenialNotice,
  permissionModeNotice,
} from "./agy-map.ts";
import { CodexAdapter, codexForceKillPlan, codexLaunchSpec, findCodex, stopCodexChild } from "./codex-adapter.ts";
import {
  approvalFromServerRequest,
  approvalRpcReply,
  createStreamState,
  itemsToPiMessages,
  mapNotification,
  modelsFromList,
  visibleModels,
} from "./codex-map.ts";
import { LineJsonRpc, redactSecrets } from "./jsonrpc.ts";
import {
  appendUserMessage,
  rememberSession,
  sessionMessages,
  threadIdFor,
  writeSessionPrelude,
} from "./session-index.ts";
import "./claude-checks.ts";

async function removeTemp(dir: string): Promise<void> {
  // Windows keeps a process's working directory locked until the handle
  // closes, and rm can return EPERM for a moment after exit.
  for (let attempt = 0; attempt < 20; attempt++) {
    try {
      rmSync(dir, { recursive: true, force: true });
      return;
    } catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if ((code !== "EPERM" && code !== "EBUSY") || attempt === 19) throw error;
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }
}

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
    await removeTemp(dir);
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
    await removeTemp(dir);
    await removeTemp(empty);
  }
});

test("codex.cmd runs its npm script, or cmd.exe with a fixed argument", () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-cmd-"));
  try {
    const root = join(dir, "Program Files");
    const script = join(root, "node_modules", "@openai", "codex", "bin", "codex.js");
    mkdirSync(join(root, "node_modules", "@openai", "codex", "bin"), { recursive: true });
    writeFileSync(script, "");
    const cmd = join(root, "codex.cmd");
    writeFileSync(cmd, `@ECHO off\r\n"%~dp0\\node_modules\\@openai\\codex\\bin\\codex.js" %*\r\n`);
    const spec = codexLaunchSpec(cmd, "win32");
    assert.equal(spec.command, process.execPath);
    assert.deepEqual(spec.args, [script, "app-server"]);
    assert.equal(spec.windowsVerbatimArguments, undefined);

    const bat = join(root, "codex.bat");
    writeFileSync(bat, `@ECHO off\r\n"%dp0%\\node_modules\\@openai\\codex\\bin\\codex.js" %*\r\n`);
    assert.deepEqual(codexLaunchSpec(bat, "win32").args, [script, "app-server"]);

    const opaque = join(root, "opaque.cmd");
    writeFileSync(opaque, "@ECHO off\r\nREM no script path\r\n");
    const fallback = codexLaunchSpec(opaque, "win32");
    assert.equal(fallback.command, "cmd.exe");
    assert.deepEqual(fallback.args, ["/d", "/s", "/c", `"${opaque}" app-server`]);
    assert.equal(fallback.windowsVerbatimArguments, true);

    assert.deepEqual(codexLaunchSpec(cmd, "linux"), { command: cmd, args: ["app-server"] });
    assert.throws(() => codexLaunchSpec(join(root, "bad%PATH%.cmd"), "win32"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("two concurrent approvals are both answered, and abort or new_session denies the rest", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-queue-"));
  const binary = join(dir, "codex");
  const record = join(dir, "replies.jsonl");
  writeFileSync(binary, `#!/usr/bin/env node
import readline from "node:readline";
import { appendFileSync } from "node:fs";
const record = ${JSON.stringify(record)};
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
const send = (value) => process.stdout.write(JSON.stringify(value) + "\\n");
let batch = 0;
rl.on("line", (line) => {
  const msg = JSON.parse(line);
  if (!msg.method) {
    appendFileSync(record, line + "\\n");
    return;
  }
  if (!Object.prototype.hasOwnProperty.call(msg, "id")) return;
  if (msg.method === "initialize") send({ jsonrpc: "2.0", id: msg.id, result: { userAgent: "fake" } });
  else if (msg.method === "account/read") {
    send({ jsonrpc: "2.0", id: msg.id, result: { account: { type: "chatgpt", email: "a@b.c" }, requiresOpenaiAuth: false } });
  } else if (msg.method === "model/list") {
    send({ jsonrpc: "2.0", id: msg.id, result: { data: [], nextCursor: null } });
  } else if (msg.method === "thread/start") {
    send({ jsonrpc: "2.0", id: msg.id, result: { thread: { id: "thread-1" }, model: "gpt-test" } });
  } else if (msg.method === "turn/start") {
    batch += 1;
    const first = batch * 2 + 5;
    send({ jsonrpc: "2.0", id: msg.id, result: { turn: { id: "turn-" + batch } } });
    send({ jsonrpc: "2.0", id: first, method: "item/commandExecution/requestApproval", params: { command: "latexmk", threadId: "thread-1", turnId: "turn-" + batch, itemId: "a" } });
    send({ jsonrpc: "2.0", id: first + 1, method: "item/commandExecution/requestApproval", params: { command: "git status", threadId: "thread-1", turnId: "turn-" + batch, itemId: "b" } });
  } else send({ jsonrpc: "2.0", id: msg.id, result: {} });
});
`, { mode: 0o755 });
  writeFileSync(record, "");
  const adapter = new CodexAdapter();
  const collected = collectHost();
  try {
    await adapter.start({ cwd: dir, sessionDir: dir, env: { ...process.env, CODEX_PATH: binary, PATH: process.env.PATH ?? "" } });
    assert.equal(adapter.unavailableReason, null);
    await adapter.handleCommand({ type: "prompt", id: "p1", message: "Fix it" }, collected.host);
    await waitForCount(collected.events, "approval_request", 2);
    const approvals = collected.events.filter((event) => event.type === "approval_request");
    assert.deepEqual(approvals.map((event) => event.id), ["7", "8"]);
    assert.equal(approvals[0].detail, "latexmk");
    assert.equal(approvals[1].detail, "git status");

    await adapter.handleCommand({ type: "approval_response", id: "7", decision: "approve" }, collected.host);
    await adapter.handleCommand({ type: "approval_response", id: "8", decision: "deny" }, collected.host);
    const answered = await waitForText(record, /"id":8/);
    const decisions = answered
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line))
      .filter((line) => line.result && line.result.decision);
    assert.deepEqual(decisions.map((line) => [line.id, line.result.decision]), [[7, "accept"], [8, "decline"]]);

    await adapter.handleCommand({ type: "prompt", id: "p2", message: "Again" }, collected.host);
    await waitForCount(collected.events, "approval_request", 4);
    const before = readFileSync(record, "utf8");
    await adapter.handleCommand({ type: "abort", id: "stop" }, collected.host);
    const denied = await waitForText(record, /"id":10/);
    const fresh = denied.slice(before.length)
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line))
      .filter((line) => line.result && line.result.decision);
    assert.deepEqual(fresh.map((line) => [line.id, line.result.decision]), [[9, "decline"], [10, "decline"]]);

    await adapter.handleCommand({ type: "prompt", id: "p3", message: "Third" }, collected.host);
    await waitForCount(collected.events, "approval_request", 6);
    const marked = readFileSync(record, "utf8").length;
    await adapter.handleCommand({ type: "new_session", id: "new" }, collected.host);
    const cleared = await waitForText(record, /"id":12/);
    const rest = cleared.slice(marked)
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line))
      .filter((line) => line.result && line.result.decision);
    assert.deepEqual(rest.map((line) => [line.id, line.result.decision]), [[11, "decline"], [12, "decline"]]);
  } finally {
    await adapter.stop();
    await removeTemp(dir);
  }
});

test("stdin EPIPE is reported as an unavailable error", async () => {
  const child = spawn(process.execPath, ["-e", "setInterval(() => {}, 1000)"], {
    stdio: ["pipe", "pipe", "pipe"],
  });
  const messages: string[] = [];
  const rpc = new LineJsonRpc(
    child,
    () => {},
    () => {},
    (message) => messages.push(message)
  );
  const error = new Error("write EPIPE") as NodeJS.ErrnoException;
  error.code = "EPIPE";
  child.stdin.emit("error", error);
  assert.ok(messages.some((message) => message.includes("unavailable")));
  rpc.close();
  child.kill("SIGKILL");
  await new Promise((resolve) => child.once("exit", resolve));
});

test("stop force-kills a child that ignores the graceful signal", async () => {
  assert.deepEqual(codexForceKillPlan(4242, "win32"), {
    command: "taskkill",
    args: ["/PID", "4242", "/T", "/F"],
  });
  assert.deepEqual(codexForceKillPlan(4242, "linux"), { signal: "SIGKILL" });
  const child = spawn(
    process.execPath,
    ["-e", 'process.on("SIGTERM", () => {}); process.stdout.write("ready\\n"); setInterval(() => {}, 500);'],
    { stdio: ["pipe", "pipe", "pipe"] }
  );
  await new Promise<void>((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("child did not start")), 3000);
    child.stdout.on("data", (chunk) => {
      if (String(chunk).includes("ready")) {
        clearTimeout(timer);
        resolve();
      }
    });
    child.once("exit", () => {
      clearTimeout(timer);
      reject(new Error("child exited before ready"));
    });
  });
  await stopCodexChild(child, 50);
  assert.ok(child.exitCode !== null || child.signalCode !== null);
  if (process.platform !== "win32") assert.equal(child.signalCode, "SIGKILL");
});

async function waitForCount(events: Record<string, unknown>[], type: string, count: number): Promise<void> {
  const deadline = Date.now() + 3000;
  while (Date.now() < deadline) {
    if (events.filter((event) => event.type === type).length >= count) return;
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  throw new Error(`timed out waiting for ${count} ${type} events: ${JSON.stringify(events.map((event) => event.type))}`);
}

function textDeltas(events: Record<string, unknown>[]): string[] {
  return events
    .map((event) => event.assistantMessageEvent as { type?: string; delta?: string } | undefined)
    .filter((delta) => delta?.type === "text_delta")
    .map((delta) => delta?.delta ?? "");
}

test("agy stream-json maps the documented turn without inventing thinking text", () => {
  const state = createAgyStreamState();
  const init = mapAgyLine(
    {
      event: "init",
      conversation_id: "9ec58bfd-4d67-4f5e-83a5-9d907e9c6b1f",
      init: { cwd: "/home/user/project", tools: ["run_command"], permission_mode: "request-review" },
    },
    state
  );
  assert.equal(init.conversationId, "9ec58bfd-4d67-4f5e-83a5-9d907e9c6b1f");
  assert.equal(init.permissionMode, "request-review");
  assert.equal(init.events.length, 0);

  const ignored = [
    mapAgyLine({ event: "step_update", step_update: { step_index: 0, state: "DONE", step_type: "user_input" } }, state),
    mapAgyLine(
      {
        event: "step_update",
        step_update: {
          step_index: 1,
          state: "DONE",
          step_type: "checkpoint",
          usage: { total_tokens: 123, input_tokens: 1, output_tokens: 1, thinking_tokens: 1, cache_read_tokens: 0 },
        },
      },
      state
    ),
    mapAgyLine({ event: "future_event" }, state),
    mapAgyLine({ event: "result", result: { status: "RUNNING" } }, state),
  ];
  assert.deepEqual(
    ignored.flatMap((row) => row.events),
    []
  );
  assert.equal(ignored[1].usage, null);
  assert.equal(ignored[3].terminal, false);

  const active = mapAgyLine(
    {
      event: "step_update",
      step_update: { step_index: 2, state: "ACTIVE", step_type: "agent_response", text_delta: "apple" },
    },
    state
  );
  const done = mapAgyLine(
    {
      event: "step_update",
      step_update: {
        step_index: 2,
        state: "DONE",
        step_type: "agent_response",
        text_delta: "\n",
        usage: { input_tokens: 30384, output_tokens: 4, thinking_tokens: 0, cache_read_tokens: 0, total_tokens: 30388 },
      },
    },
    state
  );
  assert.equal(done.usage, null);
  const started = mapAgyLine(
    {
      event: "step_update",
      step_update: {
        step_index: 4,
        state: "ACTIVE",
        step_type: "tool",
        tool_name: "run_command",
        tool_info: { name: "run_command", parameters: { CommandLine: "echo hello_headless_demo" } },
      },
    },
    state
  );
  const finished = mapAgyLine(
    {
      event: "step_update",
      step_update: {
        step_index: 4,
        state: "DONE",
        step_type: "tool",
        tool_name: "run_command",
        tool_info: {
          name: "run_command",
          parameters: { CommandLine: "echo hello_headless_demo" },
          output: "hello_headless_demo\r\n",
        },
      },
    },
    state
  );
  const repeated = mapAgyLine(
    {
      event: "step_update",
      step_update: {
        step_index: 4,
        state: "DONE",
        step_type: "tool",
        tool_info: {
          name: "run_command",
          parameters: { CommandLine: "echo hello_headless_demo" },
          output: "hello_headless_demo\r\n",
        },
      },
    },
    state
  );
  assert.equal(repeated.events.length, 0);
  const result = mapAgyLine(
    {
      event: "result",
      result: {
        status: "SUCCESS",
        response: "apple\n",
        usage: { input_tokens: 10, output_tokens: 4, thinking_tokens: 7, cache_read_tokens: 1, total_tokens: 14 },
      },
    },
    state
  );
  const events = [...active.events, ...done.events, ...started.events, ...finished.events, ...result.events];
  assert.deepEqual(textDeltas(events), ["apple", "\n"]);
  assert.equal(
    events.some((event) => (event.assistantMessageEvent as { type?: string } | undefined)?.type === "thinking_delta"),
    false
  );
  assert.equal(started.events.filter((event) => event.type === "tool_execution_end").length, 0);
  assert.equal(finished.events.filter((event) => event.type === "tool_execution_start").length, 0);
  const toolStart = events.find((event) => event.type === "tool_execution_start");
  assert.equal(toolStart?.toolName, "bash");
  assert.equal((toolStart?.args as { command: string }).command, "echo hello_headless_demo");
  const toolEnd = events.find((event) => event.type === "tool_execution_end");
  assert.equal((toolEnd?.result as { content: { text: string }[] }).content[0].text, "hello_headless_demo\r\n");
  assert.equal(result.usage?.thinking, 7);
  assert.equal(result.usage?.total, 14);
  assert.equal(result.failure, null);
  assert.equal((result.messages[0].content as { type: string; text?: string }[])[0].text, "apple\n");

  const prefixed = createAgyStreamState();
  mapAgyLine(
    { event: "step_update", step_update: { step_index: 2, state: "ACTIVE", step_type: "agent_response", text_delta: "apple" } },
    prefixed
  );
  const full = mapAgyLine(
    {
      event: "step_update",
      step_update: { step_index: 2, state: "DONE", step_type: "agent_response", text_delta: "apple\n" },
    },
    prefixed
  );
  assert.deepEqual(textDeltas(full.events), ["\n"]);

  const sub = mapAgyLine(
    {
      event: "step_update",
      step_update: {
        step_index: 5,
        state: "DONE",
        subagent_info: {
          subagents: [
            {
              type_name: "researcher",
              role: "search",
              conversation_id: "sub-1",
              log_uri: "/secret/path",
              workspace_uris: ["/secret/ws"],
            },
          ],
        },
      },
    },
    createAgyStreamState()
  );
  const dumped = JSON.stringify(sub.events);
  assert.equal(dumped.includes("/secret/path"), false);
  assert.equal(dumped.includes("/secret/ws"), false);
  assert.match(dumped, /researcher/);

  const failures: [string, RegExp][] = [
    ["ERROR", /boom/],
    ["WAITING", /waiting/i],
    ["INTERRUPTED", /interrupted/i],
  ];
  for (const [status, pattern] of failures) {
    const mapped = mapAgyLine(
      { event: "result", result: { status, error: status === "ERROR" ? "boom" : undefined } },
      createAgyStreamState()
    );
    assert.equal(mapped.terminal, true);
    assert.match(mapped.failure ?? "", pattern);
  }
});

test("agy approvals are not delivered and launch stays in the restrictive mode", () => {
  assert.equal(permissionModeNotice("always-proceed"), AGY_ALWAYS_PROCEED_NOTICE);
  assert.equal(permissionModeNotice(null), AGY_PERMISSION_LIMIT);
  assert.equal(permissionModeNotice("request-review"), AGY_PERMISSION_LIMIT);
  assert.deepEqual(approvalIsNotDelivered("approve"), { sent: false, choice: "once" });
  assert.deepEqual(approvalIsNotDelivered("always_allow"), { sent: false, choice: "always" });
  assert.equal(decisionLabel("deny"), "deny");
  assert.equal(decisionLabel("nope"), "deny");
  assert.equal(approvalIsNotDelivered("deny").sent, false);

  const denial =
    'a tool required the "command" permission that headless mode cannot prompt for, so it was auto-denied. Add an allow rule.';
  assert.match(permissionDenialNotice(denial) ?? "", /auto-denied/);
  assert.match(permissionDenialNotice(denial) ?? "", /does not auto-approve/);
  assert.equal(permissionDenialNotice("progress"), null);

  assert.equal(authFailureReason("authentication required"), AGY_SIGNED_OUT);
  assert.equal(authFailureReason("Authentication expired. Please log back in."), AGY_SIGNED_OUT);
  assert.equal(authFailureReason("secret keyring is locked"), AGY_KEYRING_LOCKED);
  assert.match(authFailureReason("GEMINI_API_KEY is not set") ?? "", /Gemini API key/);

  const models = modelsFromText(
    "# models\nAvailable models\ngemini-4-argon-high       Gemini 4 Argon (High)\ngemini-3.8-flash-medium       Gemini 3.8 Flash (Medium)\ngemini-4-argon-high       duplicate\n"
  );
  assert.deepEqual(
    models.map((model) => model.id),
    ["gemini-4-argon-high", "gemini-3.8-flash-medium"]
  );
  assert.equal(models[0].name, "Gemini 4 Argon (High)");
  assert.equal(models[0].provider, "agy");

  const args = agyStreamArgs({ model: "gemini-4-argon-high", effort: "high", conversationId: "conv-1" });
  for (const flag of ["stream-json", "--print-timeout", "0s", "--sandbox", "--disable-slash-commands", "--model", "--effort", "--conversation"]) {
    assert.ok(args.includes(flag), flag);
  }
  assert.equal(args.includes("--dangerously-skip-permissions"), false);
  assert.equal(args.includes("--mode"), false);
  assert.equal(args.includes("accept-edits"), false);
  assert.equal(args.includes("auto"), false);
  const unsafe = agyStreamArgs({ model: "bad model", effort: "xhigh;rm", conversationId: "../x" });
  assert.equal(unsafe.includes("--model"), false);
  assert.equal(unsafe.includes("--effort"), false);
  assert.equal(unsafe.includes("--conversation"), false);

  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-agy-which-"));
  try {
    const binary = join(dir, "agy");
    writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
    assert.equal(findAgy({ AGY_PATH: binary, PATH: "", HOME: dir }), binary);
    assert.equal(findAgy({ PATH: dir, HOME: dir }), binary);
    assert.equal(findAgy({ AGY_PATH: join(dir, "missing"), PATH: "", HOME: dir }), null);
    const exe = join(dir, "agy.exe");
    writeFileSync(exe, "", { mode: 0o755 });
    assert.equal(findAgy({ PATH: dir, HOME: dir }, "win32"), exe);
    const cmd = join(dir, "agy.cmd");
    const spec = agyLaunchSpec(cmd, ["--sandbox"], "win32");
    assert.equal(spec.command, "cmd.exe");
    assert.deepEqual(spec.args.slice(0, 3), ["/d", "/s", "/c"]);
    assert.match(spec.args[3], /agy\.cmd" --sandbox/);
    assert.equal(spec.windowsVerbatimArguments, true);
    assert.throws(() => agyLaunchSpec(join(dir, "bad%name.cmd"), ["--sandbox"], "win32"));
    assert.deepEqual(agyLaunchSpec(binary, ["--sandbox"], "linux"), { command: binary, args: ["--sandbox"] });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("agy adapter sources do not reference credential files", () => {
  const forbidden = [
    "auth.json",
    ".gemini",
    "gcloud",
    "application_default_credentials",
    "adc.json",
    ".codex",
    ".claude",
    "credentials.json",
    "credentials.db",
  ];
  for (const name of ["agy-adapter.ts", "agy-map.ts"]) {
    const source = readFileSync(new URL(`./${name}`, import.meta.url), "utf8");
    for (const needle of forbidden) {
      assert.equal(source.includes(needle), false, `${name} mentions ${needle}`);
    }
  }
});

function writeFakeAgy(binary: string, record: string): void {
  writeFileSync(
    binary,
    `#!/usr/bin/env node
import readline from "node:readline";
import { appendFileSync, writeSync } from "node:fs";
const record = ${JSON.stringify(record)};
const argv = process.argv.slice(2);
const send = (value) => writeSync(1, JSON.stringify(value) + "\\n");
if (argv[0] === "models") {
  if (process.env.FAKE_AGY_AUTH === "missing") {
    writeSync(2, "authentication required\\n");
    process.exit(1);
  }
  writeSync(1, "gemini-4-argon-high       Gemini 4 Argon (High)\\ngemini-3.8-flash-medium       Gemini 3.8 Flash (Medium)\\n");
  process.exit(0);
}
appendFileSync(record, JSON.stringify({ argv, cwd: process.cwd() }) + "\\n");
const conversationIndex = argv.indexOf("--conversation");
const conversation = conversationIndex >= 0 ? argv[conversationIndex + 1] : "conv-1";
send({
  event: "init",
  conversation_id: conversation,
  init: { cwd: process.cwd(), tools: ["run_command"], permission_mode: "request-review" },
});
let turns = 0;
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on("line", (line) => {
  appendFileSync(record, line + "\\n");
  let msg = null;
  try {
    msg = JSON.parse(line);
  } catch {
    msg = null;
  }
  if (!msg || msg.event !== "user") {
    send({ event: "result", result: { status: "ERROR", error: "unsupported" } });
    return;
  }
  turns += 1;
  if (process.env.FAKE_AGY_HOLD === "1" && turns === 1) {
    send({
      event: "step_update",
      step_update: { step_index: 1, step_type: "agent_response", state: "ACTIVE", text_delta: "Working" },
    });
    return;
  }
  send({ event: "step_update", step_update: { step_index: 0, step_type: "user_input", state: "DONE" } });
  send({ event: "step_update", step_update: { step_index: 1, step_type: "agent_response", state: "ACTIVE", text_delta: "Hello" } });
  send({ event: "step_update", step_update: { step_index: 1, step_type: "agent_response", state: "DONE", text_delta: " there\\n" } });
  send({
    event: "step_update",
    step_update: {
      step_index: 2,
      step_type: "tool",
      state: "DONE",
      tool_info: {
        name: "run_command",
        parameters: { CommandLine: "echo hello_headless_demo" },
        output: "hello_headless_demo\\n",
      },
      usage: { input_tokens: 99, output_tokens: 0, thinking_tokens: 0, cache_read_tokens: 0, total_tokens: 99 },
    },
  });
  if (turns === 1) {
    writeSync(2, 'a tool required the "command" permission that headless mode cannot prompt for, so it was auto-denied. Add an allow rule.\\n');
  }
  send({
    event: "result",
    conversation_id: conversation,
    result: {
      status: "SUCCESS",
      response: "Hello there\\n",
      usage: { input_tokens: 10, output_tokens: 4, thinking_tokens: 3, cache_read_tokens: 1, total_tokens: 14 },
    },
  });
});
rl.on("close", () => process.exit(0));
process.on("SIGINT", () => {
  appendFileSync(record, "SIGINT\\n");
  send({ event: "result", result: { status: "INTERRUPTED", error: "interrupted", response: "" } });
  process.exit(0);
});
`,
    { mode: 0o755 }
  );
}

function agyRecord(path: string): { spawns: { argv: string[]; cwd: string }[]; stdin: { event?: string; message?: { content?: string } }[]; raw: string } {
  const raw = readFileSync(path, "utf8");
  const spawns: { argv: string[]; cwd: string }[] = [];
  const stdin: { event?: string; message?: { content?: string } }[] = [];
  for (const line of raw.split("\n")) {
    if (!line.trim() || line === "SIGINT") continue;
    const parsed = JSON.parse(line) as { argv?: string[]; cwd?: string; event?: string; message?: { content?: string } };
    if (parsed.argv && parsed.cwd) spawns.push({ argv: parsed.argv, cwd: parsed.cwd });
    else stdin.push(parsed);
  }
  return { spawns, stdin, raw };
}

test("a fake agy process round-trips a prompt, a resume, and a model change", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-agy-"));
  const binary = join(dir, "agy");
  const record = join(dir, "record.jsonl");
  const sessionDir = join(dir, "sessions");
  mkdirSync(sessionDir);
  writeFakeAgy(binary, record);
  writeFileSync(record, "");
  const adapter = new AgyAdapter();
  const collected = collectHost();
  try {
    await adapter.start({
      cwd: dir,
      sessionDir,
      developerInstructions: "Pitex instructions.",
      env: { ...process.env, AGY_PATH: binary, FAKE_RECORD: record },
    });
    assert.equal(adapter.unavailableReason, null);
    await adapter.handleCommand({ type: "get_available_models" }, collected.host);
    const models = collected.responses.at(-1);
    assert.equal(models?.success, true);
    assert.deepEqual(
      (models?.data as { models: { id: string }[] }).models.map((model) => model.id),
      ["gemini-4-argon-high", "gemini-3.8-flash-medium"]
    );
    assert.match(String(collected.events[0].error), /does not auto-approve/);

    await adapter.handleCommand({ type: "prompt", id: "p1", message: "Fix it" }, collected.host);
    assert.equal(collected.responses.find((response) => response.id === "p1")?.success, true);
    await waitForCount(collected.events, "agent_end", 1);
    assert.equal(textDeltas(collected.events).join(""), "Hello there\n");
    const toolStart = collected.events.find((event) => event.type === "tool_execution_start");
    assert.equal(toolStart?.toolName, "bash");
    assert.equal((toolStart?.args as { command: string }).command, "echo hello_headless_demo");
    assert.equal(collected.events.some((event) => event.type === "approval_request"), false);
    assert.match(collected.events.map((event) => String(event.error ?? "")).join("\n"), /auto-denied/);

    await adapter.handleCommand({ type: "approval_response", id: "a1", decision: "always_allow" }, collected.host);
    const approval = collected.responses.find((response) => response.command === "approval_response");
    assert.equal(approval?.success, false);
    assert.equal(approval?.error, AGY_PERMISSION_LIMIT);
    assert.deepEqual(approval?.data, { sent: false, choice: "always" });

    await adapter.handleCommand({ type: "get_messages" }, collected.host);
    const history = collected.responses.find((response) => response.command === "get_messages");
    const messages = (history?.data as { messages: { role: string; content?: { text?: string }[] }[] }).messages;
    assert.deepEqual(
      messages.map((message) => message.role),
      ["user", "assistant", "toolResult"]
    );
    assert.equal(messages[0].content?.[0].text, "Fix it");
    assert.match(JSON.stringify(messages[1].content), /Hello there/);
    const index = JSON.parse(readFileSync(join(sessionDir, "codex-session-index.json"), "utf8")) as {
      sessions: Record<string, { threadId: string }>;
    };
    const sessionPath = Object.keys(index.sessions)[0];
    assert.equal(index.sessions[sessionPath].threadId, "conv-1");
    const stored = sessionMessages(sessionPath);
    assert.equal(stored[0].role, "user");
    assert.equal((stored[1] as { content?: { text?: string }[] }).content?.[0].text?.includes("Hello there"), true);

    await adapter.handleCommand({ type: "get_session_stats" }, collected.host);
    const stats = collected.responses.find((response) => response.command === "get_session_stats");
    assert.equal((stats?.data as { tokens: { thinking: number } }).tokens.thinking, 3);

    await adapter.handleCommand({ type: "set_thinking_level", level: "xhigh", id: "bad-effort" }, collected.host);
    assert.equal(collected.responses.find((response) => response.id === "bad-effort")?.success, false);
    await adapter.handleCommand({ type: "set_thinking_level", level: "high", id: "effort" }, collected.host);
    assert.equal(collected.responses.find((response) => response.id === "effort")?.success, true);
    await adapter.handleCommand({ type: "set_model", modelId: "gemini-3.8-flash-medium", id: "model" }, collected.host);
    assert.equal(collected.responses.find((response) => response.id === "model")?.success, true);
    await adapter.handleCommand({ type: "prompt", id: "p2", message: "Next" }, collected.host);
    await waitForCount(collected.events, "agent_end", 2);

    let recorded = agyRecord(record);
    assert.ok(recorded.stdin.every((line) => line.event === "user"));
    assert.equal(recorded.stdin[0].message?.content?.startsWith("Pitex instructions.\n\nFix it"), true);
    assert.equal(recorded.stdin[1].message?.content, "Next");
    assert.equal(recorded.raw.includes("control_request"), false);
    assert.equal(recorded.raw.includes("control_response"), false);
    assert.equal(recorded.spawns[0].cwd, dir);
    assert.ok(recorded.spawns.every((spawned) => spawned.argv.includes("--sandbox") && spawned.argv.includes("stream-json")));
    assert.equal(recorded.spawns.some((spawned) => spawned.argv.includes("--dangerously-skip-permissions")), false);
    assert.equal(recorded.spawns.some((spawned) => spawned.argv.includes("--mode")), false);
    assert.ok(
      recorded.spawns.some(
        (spawned) => spawned.argv.includes("--model") && spawned.argv.includes("gemini-3.8-flash-medium") && spawned.argv.includes("--effort") && spawned.argv.includes("high")
      )
    );

    await adapter.handleCommand({ type: "switch_session", sessionPath: join(sessionDir, "missing.jsonl"), id: "missing" }, collected.host);
    assert.equal(collected.responses.find((response) => response.id === "missing")?.success, false);
    await adapter.handleCommand({ type: "switch_session", sessionPath, id: "resume" }, collected.host);
    const resumed = collected.responses.find((response) => response.id === "resume");
    assert.equal(resumed?.success, true);
    assert.deepEqual(resumed?.data, { cancelled: false });
    recorded = agyRecord(record);
    assert.ok(recorded.spawns.some((spawned) => spawned.argv.includes("--conversation") && spawned.argv.includes("conv-1")));

    await adapter.handleCommand({ type: "get_messages", id: "history-2" }, collected.host);
    const replay = collected.responses.find((response) => response.id === "history-2");
    const replayed = (replay?.data as { messages: { role: string; content?: { text?: string }[] }[] }).messages;
    assert.equal(replayed[0].content?.[0].text, "Fix it");
    await adapter.handleCommand({ type: "prompt", id: "empty", message: "   " }, collected.host);
    assert.equal(collected.responses.find((response) => response.id === "empty")?.success, false);
  } finally {
    await adapter.stop();
    await removeTemp(dir);
  }
});

test("agy abort sends SIGINT and does not write a permission reply", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-agy-abort-"));
  const binary = join(dir, "agy");
  const record = join(dir, "record.jsonl");
  writeFakeAgy(binary, record);
  writeFileSync(record, "");
  const adapter = new AgyAdapter();
  const collected = collectHost();
  try {
    await adapter.start({
      cwd: dir,
      sessionDir: dir,
      env: { ...process.env, AGY_PATH: binary, FAKE_AGY_HOLD: "1", FAKE_RECORD: record },
    });
    assert.equal(adapter.unavailableReason, null);
    await adapter.handleCommand({ type: "prompt", id: "hold", message: "Hold" }, collected.host);
    await waitForCount(collected.events, "message_update", 2);
    await adapter.handleCommand({ type: "abort", id: "stop" }, collected.host);
    assert.equal(collected.responses.find((response) => response.id === "stop")?.success, true);
    if (process.platform !== "win32") {
      const recorded = await waitForText(record, /SIGINT/);
      assert.match(recorded, /SIGINT/);
      await waitForCount(collected.events, "agent_end", 1);
      const end = collected.events.find((event) => event.type === "agent_end");
      assert.match(JSON.stringify(end?.messages), /interrupted/);
    }
    assert.equal(readFileSync(record, "utf8").includes("control_"), false);
  } finally {
    await adapter.stop();
    await removeTemp(dir);
  }
});

test("a missing or signed-out Antigravity CLI is reported without exiting", async () => {
  const empty = mkdtempSync(join(tmpdir(), "pitex-bridge-agy-empty-"));
  const missing = new AgyAdapter();
  await missing.start({ cwd: empty, sessionDir: empty, env: { PATH: "", HOME: empty, AGY_PATH: "" } });
  assert.match(missing.unavailableReason ?? "", /Install the official Antigravity CLI/);
  assert.match(missing.unavailableReason ?? "", /agy/);
  const host = collectHost();
  await missing.handleCommand({ type: "prompt", message: "Hi", id: "p" }, host.host);
  assert.equal(host.responses[0].success, false);
  assert.match(String(host.responses[0].error), /agy/);

  const dir = mkdtempSync(join(tmpdir(), "pitex-bridge-agy-signed-out-"));
  const binary = join(dir, "agy");
  writeFakeAgy(binary, join(dir, "record.jsonl"));
  const signedOut = new AgyAdapter();
  try {
    await signedOut.start({
      cwd: dir,
      sessionDir: dir,
      env: { ...process.env, AGY_PATH: binary, FAKE_AGY_AUTH: "missing" },
    });
    assert.match(signedOut.unavailableReason ?? "", /not logged in/);
    assert.match(signedOut.unavailableReason ?? "", /agy/);
    const signedHost = collectHost();
    await signedOut.handleCommand({ type: "prompt", message: "Hi", id: "p" }, signedHost.host);
    assert.equal(signedHost.responses[0].success, false);
    assert.match(String(signedHost.responses[0].error), /not logged in/);
  } finally {
    await signedOut.stop();
    await missing.stop();
    await removeTemp(dir);
    await removeTemp(empty);
  }
});

test("createAdapter selects agy and rejects an unknown id", async () => {
  const agy = await createAdapter("agy");
  assert.equal(agy.id, "agy");
  assert.equal(agy.displayName, "Antigravity CLI");
  const grok = await createAdapter("grok");
  assert.equal(grok.id, "grok");
  const claude = await createAdapter("claude");
  assert.equal(claude.id, "claude");
  await assert.rejects(() => createAdapter("nope"), /Unknown CLI adapter/);
});
