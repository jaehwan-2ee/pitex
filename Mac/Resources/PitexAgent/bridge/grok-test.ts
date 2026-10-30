// Grok ACP adapter tests. The fake CLI speaks JSON-RPC on stdio and never
// blocks the readline loop on a reply, so a permission or terminal response
// cannot deadlock the turn.
// Run with the Codex suite: node --test Mac/Resources/PitexAgent/bridge/test.ts

import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, readdirSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import type { AdapterHost } from "./adapter.ts";
import { GROK_SIGNED_OUT, GrokAdapter, findGrok, grokLaunchSpec } from "./grok-adapter.ts";
import {
  approvalFromPermission,
  cancelledPermission,
  catalogFromAcp,
  closeTurn,
  createHistory,
  createStreamState,
  historyMessages,
  mapSessionUpdate,
  permissionResult,
  pushUserMessage,
  readProjectFile,
  recordSessionUpdate,
  resolveInsideProject,
  terminalWorkingDirectory,
  visibleModels,
  writeProjectFile,
} from "./grok-map.ts";

const FAKE_SOURCE = `#!/usr/bin/env node
import readline from "node:readline";
import { appendFileSync } from "node:fs";

const record = process.env.FAKE_RECORD;
const mode = process.env.FAKE_MODE || "basic";
const auth = process.env.FAKE_AUTH || "ok";
const load = process.env.FAKE_LOAD || "yes";
const inside = process.env.FAKE_INSIDE;
const outsideFile = process.env.FAKE_OUTSIDE_FILE;
const outsideDir = process.env.FAKE_OUTSIDE_DIR;
const project = process.env.FAKE_ROOT;
const nodeBin = process.env.FAKE_NODE;
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
const send = (value) => process.stdout.write(JSON.stringify(value) + "\\n");
let promptId = null;
let nextPermission = 42;
let trackedTerminal = null;

function finishPrompt(stopReason) {
  if (promptId == null) return;
  const id = promptId;
  promptId = null;
  send({ jsonrpc: "2.0", id, result: { stopReason } });
}

function initResult() {
  const authMethods = auth === "missing"
    ? [{ id: "xai.api_key" }]
    : [{ id: "cached_token" }, { id: "xai.api_key" }];
  return {
    protocolVersion: 1,
    agentCapabilities: { loadSession: load === "yes" },
    authMethods,
    _meta: {
      modelState: {
        currentModelId: "grok-test",
        availableModels: [{
          modelId: "grok-test",
          name: "Grok Test",
          reasoningEfforts: ["low", "high"],
        }],
      },
    },
  };
}

function continueTurn(msg) {
  const outcome = msg.result && msg.result.outcome && msg.result.outcome.outcome;
  if (!outcome) return;
  if (outcome === "cancelled") {
    finishPrompt("cancelled");
    return;
  }
  send({
    jsonrpc: "2.0",
    method: "session/update",
    params: {
      update: {
        sessionUpdate: "tool_call",
        toolCallId: "tool-1",
        kind: "execute",
        status: "completed",
        rawInput: { command: "latexmk main.tex" },
        rawOutput: "ok",
      },
    },
  });
  finishPrompt("end_turn");
}

function continueFiles(msg) {
  if (msg.result && msg.result.terminalId) {
    trackedTerminal = msg.result.terminalId;
    send({ jsonrpc: "2.0", id: 71, method: "terminal/wait_for_exit", params: { terminalId: trackedTerminal } });
    return;
  }
  if (msg.id === 71 && msg.result && trackedTerminal) {
    send({ jsonrpc: "2.0", id: 70, method: "terminal/output", params: { terminalId: trackedTerminal } });
    return;
  }
  if (msg.id === 70 && msg.result) finishPrompt("end_turn");
}

function handleRequest(msg) {
  if (msg.method === "initialize") {
    send({ jsonrpc: "2.0", id: msg.id, result: initResult() });
    return;
  }
  if (msg.method === "authenticate") {
    if (auth === "fail") {
      send({ jsonrpc: "2.0", id: msg.id, error: { code: -32000, message: "upstream token secret leaked" } });
      return;
    }
    send({ jsonrpc: "2.0", id: msg.id, result: {} });
    return;
  }
  if (msg.method === "x.ai/models/list") {
    send({ jsonrpc: "2.0", id: msg.id, error: { code: -32601, message: "Method not found" } });
    return;
  }
  if (msg.method === "session/new") {
    send({ jsonrpc: "2.0", id: msg.id, result: { sessionId: "sess-1" } });
    return;
  }
  if (msg.method === "session/load") {
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: { update: { sessionUpdate: "user_message_chunk", content: { type: "text", text: "Earlier" } } },
    });
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: { update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Restored" } } },
    });
    send({ jsonrpc: "2.0", id: msg.id, result: {} });
    return;
  }
  if (msg.method === "session/prompt") {
    promptId = msg.id;
    if (mode === "files") {
      send({ jsonrpc: "2.0", id: 51, method: "fs/read_text_file", params: { sessionId: "sess-1", path: inside } });
      send({ jsonrpc: "2.0", id: 54, method: "fs/read_text_file", params: { sessionId: "sess-1", path: outsideFile } });
      send({ jsonrpc: "2.0", id: 52, method: "terminal/create", params: { sessionId: "sess-1", command: "hostname", cwd: outsideDir } });
      send({
        jsonrpc: "2.0",
        id: 53,
        method: "terminal/create",
        params: {
          sessionId: "sess-1",
          command: nodeBin,
          args: ["-e", "process.stdout.write('scoped-ok')"],
          cwd: project,
        },
      });
      return;
    }
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: { update: { sessionUpdate: "user_message_chunk", content: { type: "text", text: "ECHO" } } },
    });
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: { update: { sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "Think" } } },
    });
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: { update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Hello" } } },
    });
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: { update: { sessionUpdate: "plan", entries: [{ content: "Write", status: "pending" }] } },
    });
    const permissionId = nextPermission;
    nextPermission += 1;
    send({
      jsonrpc: "2.0",
      id: permissionId,
      method: "session/request_permission",
      params: {
        sessionId: "sess-1",
        toolCall: {
          toolCallId: "t1",
          title: "Run latexmk",
          kind: "execute",
          rawInput: { command: "latexmk main.tex" },
        },
        options: [
          { optionId: "allow-once", name: "Allow once", kind: "allow_once" },
          { optionId: "allow-always", name: "Always", kind: "allow_always" },
          { optionId: "reject-once", name: "Reject", kind: "reject_once" },
        ],
      },
    });
    return;
  }
  if (msg.method === "session/set_model" || msg.method === "session/set_mode" || msg.method === "session/set_config_option") {
    send({ jsonrpc: "2.0", id: msg.id, result: {} });
    return;
  }
  send({ jsonrpc: "2.0", id: msg.id, result: {} });
}

rl.on("line", (line) => {
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    return;
  }
  appendFileSync(record, line + "\\n");
  if (!msg.method) {
    if (mode === "files") continueFiles(msg);
    else continueTurn(msg);
    return;
  }
  if (!Object.prototype.hasOwnProperty.call(msg, "id")) {
    if (msg.method === "session/cancel") finishPrompt("cancelled");
    return;
  }
  handleRequest(msg);
});
`;

async function removeTemp(dir: string): Promise<void> {
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

function collectHost(): {
  host: AdapterHost;
  events: Record<string, unknown>[];
  responses: Record<string, unknown>[];
} {
  const events: Record<string, unknown>[] = [];
  const responses: Record<string, unknown>[] = [];
  const host: AdapterHost = {
    emit(event) {
      events.push(event);
    },
    respond(command, success, data, error, id) {
      responses.push({ command, success, data, error, id });
    },
  };
  return { host, events, responses };
}

async function waitUntil(label: string, ready: () => boolean): Promise<void> {
  const deadline = Date.now() + 5000;
  while (Date.now() < deadline) {
    if (ready()) return;
    await new Promise((resolve) => setTimeout(resolve, 20));
  }
  throw new Error(`timed out waiting for ${label}`);
}

function parseLines(text: string): Record<string, any>[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => JSON.parse(line) as Record<string, any>);
}

interface Boot {
  adapter: GrokAdapter;
  collected: ReturnType<typeof collectHost>;
  dir: string;
  outside: string;
  record: string;
  sessionDir: string;
}

async function boot(options: { mode?: string; auth?: string; load?: string } = {}): Promise<Boot> {
  const dir = realpathSync(mkdtempSync(join(tmpdir(), "pitex-grok-")));
  const outside = realpathSync(mkdtempSync(join(tmpdir(), "pitex-grok-out-")));
  const inside = join(dir, "note.txt");
  writeFileSync(inside, "hello-project");
  writeFileSync(join(outside, "secret.txt"), "super-secret-hostname");
  const binary = join(dir, "grok");
  const record = join(dir, "traffic.jsonl");
  const sessionDir = join(dir, "sessions");
  writeFileSync(binary, FAKE_SOURCE, { mode: 0o755 });
  writeFileSync(record, "");
  const adapter = new GrokAdapter();
  const collected = collectHost();
  await adapter.start({
    cwd: dir,
    sessionDir,
    developerInstructions: "Be terse.",
    env: {
      ...process.env,
      GROK_PATH: binary,
      HOME: dir,
      PATH: process.env.PATH ?? "",
      FAKE_RECORD: record,
      FAKE_MODE: options.mode ?? "basic",
      FAKE_AUTH: options.auth ?? "ok",
      FAKE_LOAD: options.load ?? "yes",
      FAKE_ROOT: dir,
      FAKE_INSIDE: inside,
      FAKE_OUTSIDE_FILE: join(outside, "secret.txt"),
      FAKE_OUTSIDE_DIR: outside,
      FAKE_NODE: process.execPath,
    },
  });
  return { adapter, collected, dir, outside, record, sessionDir };
}

async function shutdown(booted: Boot): Promise<void> {
  await booted.adapter.stop();
  await removeTemp(booted.dir);
  await removeTemp(booted.outside);
}

function responseById(responses: Record<string, unknown>[], id: string): Record<string, unknown> {
  const found = responses.find((response) => response.id === id);
  if (!found) throw new Error(`missing response ${id}: ${JSON.stringify(responses)}`);
  return found;
}

test("ACP session updates become pi transcript events", () => {
  const state = createStreamState();
  const events = [
    ...mapSessionUpdate({ sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "Think" } }, state),
    ...mapSessionUpdate({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Hello" } }, state),
    ...mapSessionUpdate({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: " there" } }, state),
    ...mapSessionUpdate(
      {
        sessionUpdate: "tool_call",
        toolCallId: "c1",
        kind: "execute",
        status: "pending",
        rawInput: { command: "latexmk" },
      },
      state
    ),
    ...mapSessionUpdate(
      { sessionUpdate: "tool_call_update", toolCallId: "c1", status: "in_progress", rawOutput: "ok" },
      state
    ),
    ...mapSessionUpdate(
      {
        sessionUpdate: "tool_call_update",
        toolCallId: "c1",
        kind: "execute",
        status: "completed",
        rawInput: { command: "latexmk" },
        rawOutput: "ok",
      },
      state
    ),
    ...mapSessionUpdate(
      { sessionUpdate: "plan", entries: [{ content: "Write", status: "pending" }] },
      state
    ),
    ...mapSessionUpdate({ sessionUpdate: "future_update", content: { type: "text", text: "nope" } }, state),
    ...closeTurn(state, "end_turn"),
  ];
  assert.deepEqual(
    events.map((event) => event.type).filter((type) => type !== "message_update"),
    [
      "agent_start",
      "message_start",
      "tool_execution_start",
      "tool_execution_update",
      "tool_execution_end",
      "tool_execution_start",
      "tool_execution_update",
      "tool_execution_end",
      "message_end",
      "turn_end",
      "agent_end",
    ]
  );
  const deltas = events
    .map((event) => event.assistantMessageEvent as { type?: string; delta?: string } | undefined)
    .filter((delta) => delta?.type === "thinking_delta" || delta?.type === "text_delta")
    .map((delta) => delta?.delta);
  assert.deepEqual(deltas, ["Think", "Hello", " there"]);
  const tool = events.find((event) => event.type === "tool_execution_start" && event.toolCallId === "c1");
  assert.equal(tool?.toolName, "bash");
  assert.deepEqual(tool?.args, { command: "latexmk" });
  const plan = events.find((event) => event.type === "tool_execution_start" && event.toolCallId === "acp-plan");
  assert.equal(plan?.toolName, "plan");
  const end = events.find((event) => event.type === "message_end");
  assert.equal((end?.message as { stopReason?: string }).stopReason, "stop");
  assert.equal(state.turnOpen, false);
  assert.equal(state.planStarted, false);

  const again = mapSessionUpdate(
    { sessionUpdate: "plan", entries: [{ content: "Next", status: "pending" }] },
    state
  );
  assert.equal(again.some((event) => event.type === "tool_execution_start" && event.toolCallId === "acp-plan"), true);
  const cancelled = closeTurn(state, "cancelled");
  const failed = cancelled.find((event) => event.type === "message_end");
  assert.equal((failed?.message as { stopReason?: string; errorMessage?: string }).stopReason, "error");
  assert.equal((failed?.message as { errorMessage?: string }).errorMessage, "The turn was cancelled.");

  const edit = mapSessionUpdate(
    {
      sessionUpdate: "tool_call",
      toolCallId: "e1",
      kind: "edit",
      status: "pending",
      locations: [{ path: "main.tex" }],
    },
    createStreamState()
  );
  assert.equal(edit[2]?.toolName, "edit");
  assert.deepEqual(edit[2]?.args, { path: "main.tex" });

  const history = createHistory();
  pushUserMessage(history, "Hi");
  recordSessionUpdate(history, { sessionUpdate: "agent_thought_chunk", content: { type: "text", text: "Think" } });
  recordSessionUpdate(history, { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Hello" } });
  recordSessionUpdate(history, {
    sessionUpdate: "tool_call",
    toolCallId: "c1",
    kind: "execute",
    status: "completed",
    rawInput: { command: "ls" },
    rawOutput: "main.tex",
  });
  const messages = historyMessages(history);
  assert.deepEqual(messages[0], { role: "user", content: [{ type: "text", text: "Hi" }] });
  const assistant = messages[1] as { role: string; content: { type: string }[] };
  assert.equal(assistant.role, "assistant");
  assert.deepEqual(
    assistant.content.map((block) => block.type),
    ["thinking", "text", "toolCall"]
  );
  const result = messages[2] as {
    role: string;
    toolCallId: string;
    toolName: string;
    isError: boolean;
    content: { type: string; text: string }[];
  };
  assert.equal(result.role, "toolResult");
  assert.equal(result.toolCallId, "c1");
  assert.equal(result.toolName, "bash");
  assert.equal(result.isError, false);
  assert.deepEqual(result.content, [{ type: "text", text: "main.tex" }]);
});

test("permission decisions select an offered ACP option", () => {
  const options = [
    { optionId: "allow-once", kind: "allow_once" },
    { optionId: "allow-always", kind: "allow_always" },
    { optionId: "reject-once", kind: "reject_once" },
  ];
  const view = approvalFromPermission(42, {
    toolCall: { kind: "execute", title: "Run latexmk", rawInput: { command: "latexmk main.tex" } },
    options,
  });
  assert.equal(view.id, "42");
  assert.equal(view.kind, "command");
  assert.match(view.detail, /latexmk main\.tex/);
  assert.deepEqual(permissionResult("approve", options), { outcome: { outcome: "selected", optionId: "allow-once" } });
  assert.deepEqual(permissionResult("always_allow", options), {
    outcome: { outcome: "selected", optionId: "allow-always" },
  });
  assert.deepEqual(permissionResult("deny", options), { outcome: { outcome: "selected", optionId: "reject-once" } });
  assert.deepEqual(permissionResult("nope", options), { outcome: { outcome: "selected", optionId: "reject-once" } });
  assert.deepEqual(permissionResult("approve", [{ optionId: "always", kind: "allow_always" }]), {
    outcome: { outcome: "selected", optionId: "always" },
  });
  assert.deepEqual(permissionResult("deny", [{ optionId: "once", kind: "allow_once" }]), {
    outcome: { outcome: "cancelled" },
  });
  assert.deepEqual(cancelledPermission(), { outcome: { outcome: "cancelled" } });
});

test("model catalogs come from config options, session config, and model state", () => {
  const config = catalogFromAcp({
    configOptions: [
      {
        id: "model",
        category: "model",
        currentValue: "grok-2",
        options: [
          { value: "grok-2", name: "Grok 2" },
          { value: "grok-3", name: "Grok 3" },
        ],
      },
      {
        id: "thought",
        category: "thought_level",
        currentValue: "high",
        options: [
          { value: "low", name: "Low" },
          { value: "high", name: "High" },
        ],
      },
    ],
  });
  assert.equal(config.modelConfigId, "model");
  assert.equal(config.effortConfigId, "thought");
  assert.equal(config.currentModelId, "grok-2");
  assert.deepEqual(config.efforts, ["low", "high"]);
  assert.equal(config.currentEffort, "high");
  assert.deepEqual(visibleModels(config.models), [
    { id: "grok-2", provider: "grok", name: "Grok 2" },
    { id: "grok-3", provider: "grok", name: "Grok 3" },
  ]);

  const session = catalogFromAcp({
    models: { currentModelId: "grok-4", availableModels: [{ modelId: "grok-4", name: "Grok 4" }] },
    _meta: {
      "x.ai/sessionConfig": {
        options: [
          { id: "grok-4", category: "model", label: "Grok 4", selected: true },
          { id: "low", category: "mode", label: "Low", selected: true },
          { id: "high", category: "mode", label: "High" },
        ],
      },
    },
  });
  assert.equal(session.currentModelId, "grok-4");
  assert.equal(session.effortUsesMode, true);
  assert.equal(session.currentEffort, "low");
  assert.deepEqual(session.efforts, ["low", "high"]);

  const state = catalogFromAcp({
    _meta: {
      modelState: {
        currentModelId: "grok-test",
        availableModels: [{ modelId: "grok-test", name: "Grok Test", reasoningEfforts: ["low", "high"] }],
      },
    },
  });
  assert.equal(state.currentModelId, "grok-test");
  assert.deepEqual(state.efforts, ["low", "high"]);
  assert.deepEqual(visibleModels(state.models), [{ id: "grok-test", provider: "grok", name: "Grok Test" }]);
});

test("filesystem and terminal paths stay inside the project", () => {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "pitex-grok-root-")));
  const outside = realpathSync(mkdtempSync(join(tmpdir(), "pitex-grok-jail-")));
  try {
    writeFileSync(join(outside, "secret.txt"), "hidden-secret");
    symlinkSync(outside, join(root, "link"));
    assert.equal(resolveInsideProject(root, join(root, "link", "secret.txt")), null);
    const escaped = readProjectFile(root, join(root, "link", "secret.txt"));
    assert.equal(escaped.ok, false);
    assert.equal(resolveInsideProject(root, "relative.txt"), null);
    assert.equal(resolveInsideProject(root, `${root}\0note.txt`), null);
    assert.equal(writeProjectFile(root, join(outside, "nope.txt"), "x").ok, false);

    mkdirSync(join(root, "sub"));
    writeFileSync(join(root, "sub", "note.txt"), "a\nb\nc");
    const read = readProjectFile(root, join(root, "sub", "note.txt"), 2, 1);
    assert.equal(read.ok, true);
    if (read.ok) assert.equal(read.content, "b");
    const written = writeProjectFile(root, join(root, "out", "new.txt"), "saved");
    assert.equal(written.ok, true);
    assert.equal(readFileSync(join(root, "out", "new.txt"), "utf8"), "saved");

    assert.equal(terminalWorkingDirectory(root, undefined), realpathSync(root));
    assert.equal(terminalWorkingDirectory(root, "sub"), null);
    assert.equal(terminalWorkingDirectory(root, outside), null);
    assert.equal(terminalWorkingDirectory(root, join(root, "sub")), realpathSync(join(root, "sub")));
  } finally {
    rmSync(root, { recursive: true, force: true });
    rmSync(outside, { recursive: true, force: true });
  }
});

test("findGrok honors GROK_PATH and a Windows cmd shim is quoted", () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-grok-which-"));
  try {
    const binary = join(dir, "grok");
    writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
    assert.equal(findGrok({ GROK_PATH: binary, PATH: "", HOME: dir }), binary);
    assert.equal(findGrok({ PATH: dir, HOME: dir }), binary);
    assert.equal(findGrok({ GROK_PATH: join(dir, "missing"), PATH: "", HOME: dir }), null);
    const exe = join(dir, "grok.exe");
    writeFileSync(exe, "", { mode: 0o755 });
    assert.equal(findGrok({ PATH: dir, HOME: dir }, "win32"), exe);
    const cmd = join(dir, "grok.cmd");
    writeFileSync(cmd, "@ECHO off\r\n");
    const spec = grokLaunchSpec(cmd, "win32");
    assert.equal(spec.command, "cmd.exe");
    assert.deepEqual(spec.args, ["/d", "/s", "/c", `"${cmd}" --no-auto-update agent stdio`]);
    assert.equal(spec.windowsVerbatimArguments, true);
    assert.throws(() => grokLaunchSpec(join(dir, "bad%PATH%.cmd"), "win32"), /cmd\.exe/);
    assert.deepEqual(grokLaunchSpec(binary, "linux"), {
      command: binary,
      args: ["--no-auto-update", "agent", "stdio"],
    });
    assert.deepEqual(grokLaunchSpec(exe, "win32"), {
      command: exe,
      args: ["--no-auto-update", "agent", "stdio"],
    });
    const nodeShim = join(dir, "grok-node");
    writeFileSync(nodeShim, "#!/usr/bin/env node\nconsole.log(1)\n", { mode: 0o755 });
    assert.deepEqual(grokLaunchSpec(nodeShim, "win32"), {
      command: process.execPath,
      args: [nodeShim, "--no-auto-update", "agent", "stdio"],
    });
    assert.deepEqual(grokLaunchSpec(binary, "win32"), {
      command: binary,
      args: ["--no-auto-update", "agent", "stdio"],
    });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("Grok adapter sources do not reference credential files", () => {
  const needles = ["auth.json", ".codex", ".claude", "XAI_API_KEY", "GROK_AUTH", "credentials.json", "~/.grok"];
  for (const name of ["grok-adapter.ts", "grok-map.ts"]) {
    const source = readFileSync(new URL(`./${name}`, import.meta.url), "utf8");
    for (const needle of needles) {
      assert.equal(source.includes(needle), false, `${name} contains ${needle}`);
    }
  }
});

test("a fake Grok CLI round-trips a prompt, an approval, and a cancel", { timeout: 20_000 }, async () => {
  const booted = await boot();
  const { adapter, collected, record, sessionDir, dir } = booted;
  try {
    assert.equal(adapter.unavailableReason, null, adapter.unavailableReason ?? "");
    const init = parseLines(readFileSync(record, "utf8")).find((line) => line.method === "initialize");
    assert.equal(init?.params.protocolVersion, 1);
    assert.deepEqual(init?.params.clientCapabilities, {
      fs: { readTextFile: true, writeTextFile: true },
      terminal: true,
    });
    assert.equal(parseLines(readFileSync(record, "utf8")).some((line) => line.method === "x.ai/models/list"), false);

    await adapter.handleCommand({ type: "get_available_models", id: "models" }, collected.host);
    const models = responseById(collected.responses, "models");
    assert.equal(models.success, true);
    assert.deepEqual((models.data as { models: { id: string; provider: string; name: string }[] }).models, [
      { id: "grok-test", provider: "grok", name: "Grok Test" },
    ]);
    await adapter.handleCommand({ type: "get_available_thinking_levels", id: "levels" }, collected.host);
    assert.deepEqual((responseById(collected.responses, "levels").data as { levels: string[] }).levels, ["low", "high"]);

    await adapter.handleCommand({ type: "prompt", id: "empty", message: "   " }, collected.host);
    assert.equal(responseById(collected.responses, "empty").success, false);

    await adapter.handleCommand({ type: "prompt", id: "p1", message: "Fix the intro" }, collected.host);
    assert.equal(responseById(collected.responses, "p1").success, true);
    const created = parseLines(readFileSync(record, "utf8")).find((line) => line.method === "session/new");
    assert.equal(created?.params.cwd, dir);
    assert.deepEqual(created?.params.mcpServers, []);
    assert.equal(created?.params._meta.rules, "Be terse.");
    assert.match(readFileSync(join(sessionDir, "codex-session-index.json"), "utf8"), /sess-1/);

    await waitUntil("approval 42", () =>
      collected.events.some((event) => event.type === "approval_request" && event.id === "42")
    );
    await adapter.handleCommand({ type: "prompt", id: "busy", message: "too soon" }, collected.host);
    assert.equal(responseById(collected.responses, "busy").success, false);
    assert.match(String(responseById(collected.responses, "busy").error), /still working/);
    await adapter.handleCommand({ type: "steer", id: "steer", message: "change" }, collected.host);
    assert.equal(responseById(collected.responses, "steer").success, false);
    assert.match(String(responseById(collected.responses, "steer").error), /steer/);

    await adapter.handleCommand({ type: "approval_response", id: "42", decision: "always_allow" }, collected.host);
    await waitUntil("first agent_end", () => collected.events.some((event) => event.type === "agent_end"));
    const traffic = parseLines(readFileSync(record, "utf8"));
    const allowed = traffic.find((line) => line.result?.outcome?.optionId === "allow-always");
    assert.equal(allowed?.id, 42);
    const deltas = collected.events
      .map((event) => event.assistantMessageEvent as { type?: string; delta?: string } | undefined)
      .filter((delta) => delta?.type === "thinking_delta" || delta?.type === "text_delta")
      .map((delta) => delta?.delta);
    assert.deepEqual(deltas, ["Think", "Hello"]);

    await adapter.handleCommand({ type: "get_messages", id: "history" }, collected.host);
    const messages = (responseById(collected.responses, "history").data as { messages: { role: string }[] }).messages;
    assert.equal(messages[0]?.role, "user");
    assert.ok(messages.some((message) => message.role === "assistant"));
    assert.ok(messages.some((message) => message.role === "toolResult"));
    const blob = JSON.stringify(messages);
    assert.match(blob, /Hello/);
    assert.match(blob, /Think/);
    assert.doesNotMatch(blob, /ECHO/);
    assert.match(blob, /Fix the intro/);

    await adapter.handleCommand({ type: "set_model", id: "model", modelId: "grok-other" }, collected.host);
    assert.equal(responseById(collected.responses, "model").success, true);
    const setModel = parseLines(readFileSync(record, "utf8")).find((line) => line.method === "session/set_model");
    assert.equal(setModel?.params.modelId, "grok-other");
    assert.equal(setModel?.params.sessionId, "sess-1");
    await adapter.handleCommand({ type: "set_thinking_level", id: "effort", level: "high" }, collected.host);
    assert.equal(responseById(collected.responses, "effort").success, true);
    const setMode = parseLines(readFileSync(record, "utf8")).find((line) => line.method === "session/set_mode");
    assert.equal(setMode?.params.modeId, "high");

    await adapter.handleCommand({ type: "prompt", id: "p2", message: "Second" }, collected.host);
    await waitUntil("approval 43", () =>
      collected.events.some((event) => event.type === "approval_request" && event.id === "43")
    );
    await adapter.handleCommand({ type: "abort", id: "stop" }, collected.host);
    assert.equal(responseById(collected.responses, "stop").success, true);
    await waitUntil("cancel recorded", () => {
      const lines = parseLines(readFileSync(record, "utf8"));
      return lines.some((line) => line.method === "session/cancel") && lines.some((line) => line.id === 43);
    });
    const after = parseLines(readFileSync(record, "utf8"));
    assert.equal(after.find((line) => line.id === 43)?.result.outcome.outcome, "cancelled");
    const cancelled = collected.events.filter((event) => event.type === "agent_end").at(-1);
    const message = (cancelled?.messages as { errorMessage?: string }[] | undefined)?.[0];
    assert.equal(message?.errorMessage, "The turn was cancelled.");
  } finally {
    await shutdown(booted);
  }
});

test("filesystem reads and terminals are jailed to the project", { timeout: 20_000 }, async () => {
  const booted = await boot({ mode: "files" });
  const { adapter, collected, record } = booted;
  try {
    assert.equal(adapter.unavailableReason, null, adapter.unavailableReason ?? "");
    await adapter.handleCommand({ type: "prompt", id: "p1", message: "Read the note" }, collected.host);
    assert.equal(responseById(collected.responses, "p1").success, true);
    await waitUntil("terminal approval", () =>
      collected.events.some((event) => event.type === "approval_request" && event.id === "53")
    );
    const approvals = collected.events.filter((event) => event.type === "approval_request");
    assert.deepEqual(
      approvals.map((event) => event.id),
      ["53"]
    );
    assert.equal(approvals[0]?.kind, "command");
    await adapter.handleCommand({ type: "approval_response", id: "53", decision: "approve" }, collected.host);
    await waitUntil("scoped command finished", () => collected.events.some((event) => event.type === "agent_end"));
    const traffic = readFileSync(record, "utf8");
    assert.match(traffic, /hello-project/);
    assert.doesNotMatch(traffic, /super-secret-hostname/);
    assert.match(traffic, /scoped-ok/);
    assert.match(traffic, /"exitCode":0/);
    const errors = parseLines(traffic).filter((line) => /outside this project/.test(line.error?.message ?? ""));
    assert.deepEqual(
      errors.map((line) => line.id).sort((a, b) => a - b),
      [52, 54]
    );
  } finally {
    await shutdown(booted);
  }
});

test("a missing or signed-out Grok CLI is reported without forwarding auth text", { timeout: 20_000 }, async () => {
  const empty = mkdtempSync(join(tmpdir(), "pitex-grok-empty-"));
  const missing = new GrokAdapter();
  const host = collectHost();
  await missing.start({ cwd: empty, sessionDir: empty, env: { PATH: "", HOME: empty, GROK_PATH: "" } });
  assert.match(missing.unavailableReason ?? "", /grok login/);
  await missing.handleCommand({ type: "prompt", message: "Hi", id: "p" }, host.host);
  assert.equal(host.responses[0]?.success, false);
  assert.match(String(host.responses[0]?.error), /grok login/);
  await removeTemp(empty);

  const unsigned = await boot({ auth: "missing" });
  try {
    assert.match(unsigned.adapter.unavailableReason ?? "", /grok login/);
    assert.equal(readFileSync(unsigned.record, "utf8").includes("authenticate"), false);
  } finally {
    await shutdown(unsigned);
  }

  const failed = await boot({ auth: "fail" });
  try {
    assert.equal(failed.adapter.unavailableReason, GROK_SIGNED_OUT);
    assert.equal((failed.adapter.unavailableReason ?? "").includes("leaked"), false);
    const auth = parseLines(readFileSync(failed.record, "utf8")).find((line) => line.method === "authenticate");
    assert.deepEqual(auth?.params, { methodId: "cached_token", _meta: { headless: true } });
    assert.equal(readFileSync(failed.record, "utf8").includes("xai.api_key"), false);
  } finally {
    await shutdown(failed);
  }
});

test("resume degrades when the CLI does not advertise loadSession", { timeout: 20_000 }, async () => {
  const booted = await boot({ load: "no" });
  const { adapter, collected, record, sessionDir } = booted;
  try {
    assert.equal(adapter.unavailableReason, null, adapter.unavailableReason ?? "");
    await adapter.handleCommand({ type: "prompt", id: "p1", message: "Hello" }, collected.host);
    await waitUntil("approval", () => collected.events.some((event) => event.type === "approval_request"));
    const approval = collected.events.find((event) => event.type === "approval_request");
    await adapter.handleCommand(
      { type: "approval_response", id: String(approval?.id), decision: "approve" },
      collected.host
    );
    await waitUntil("turn", () => collected.events.some((event) => event.type === "agent_end"));
    const sessionPath = join(sessionDir, readdirSync(sessionDir).find((name) => name.endsWith(".jsonl")) ?? "");
    await adapter.handleCommand(
      { type: "switch_session", id: "missing", sessionPath: join(sessionDir, "missing.jsonl") },
      collected.host
    );
    assert.match(String(responseById(collected.responses, "missing").error), /no Grok session/);
    await adapter.handleCommand({ type: "switch_session", id: "saved", sessionPath }, collected.host);
    assert.equal(responseById(collected.responses, "saved").success, false);
    assert.match(String(responseById(collected.responses, "saved").error), /loadSession/);
    assert.equal(
      parseLines(readFileSync(record, "utf8")).some((line) => line.method === "session/load"),
      false
    );
  } finally {
    await shutdown(booted);
  }
});

test("session/load restores a transcript without replaying it into the live stream", { timeout: 20_000 }, async () => {
  const booted = await boot({ load: "yes" });
  const { adapter, collected, record, sessionDir } = booted;
  try {
    assert.equal(adapter.unavailableReason, null, adapter.unavailableReason ?? "");
    await adapter.handleCommand({ type: "prompt", id: "p1", message: "Live prompt xyz" }, collected.host);
    await waitUntil("approval", () => collected.events.some((event) => event.type === "approval_request"));
    const approval = collected.events.find((event) => event.type === "approval_request");
    await adapter.handleCommand(
      { type: "approval_response", id: String(approval?.id), decision: "approve" },
      collected.host
    );
    await waitUntil("turn", () => collected.events.some((event) => event.type === "agent_end"));
    const sessionPath = join(sessionDir, readdirSync(sessionDir).find((name) => name.endsWith(".jsonl")) ?? "");
    const before = collected.events.length;
    await adapter.handleCommand({ type: "new_session", id: "new" }, collected.host);
    assert.equal(responseById(collected.responses, "new").success, true);
    await adapter.handleCommand({ type: "switch_session", id: "sw", sessionPath }, collected.host);
    assert.equal(responseById(collected.responses, "sw").success, true);
    assert.equal(collected.events.length, before);
    const loaded = parseLines(readFileSync(record, "utf8")).find((line) => line.method === "session/load");
    assert.equal(loaded?.params.sessionId, "sess-1");
    assert.equal(loaded?.params._meta.rules, "Be terse.");
    await adapter.handleCommand({ type: "get_messages", id: "history" }, collected.host);
    const blob = JSON.stringify(responseById(collected.responses, "history").data);
    assert.match(blob, /Earlier/);
    assert.match(blob, /Restored/);
    assert.doesNotMatch(blob, /Live prompt xyz/);
  } finally {
    await shutdown(booted);
  }
});
