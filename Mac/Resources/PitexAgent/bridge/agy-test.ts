// Antigravity (`agy acp`) tests for the shared ACP adapter. The fake CLI
// copies the wire shapes measured against agy 1.5.15: no authenticate call,
// config options for model / thought_level / mode, run_command permission
// requests with only allow_once and reject_once, session/list across every
// folder, and history that replays after the session/load response.
// Run with the rest of the bridge suite: node --test Mac/Resources/PitexAgent/bridge/test.ts

import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import type { AdapterHost } from "./adapter.ts";
import { AcpAdapter, cliLaunchSpec, findCli } from "./acp-adapter.ts";
import {
  approvalFromPermission,
  approvalKey,
  catalogFromAcp,
  createStreamState,
  mapSessionUpdate,
  permissionActions,
  permissionResultForAction,
} from "./acp-map.ts";
import { AGY } from "./acp-profiles.ts";
import { importListedSessions, loadIndex, rememberSession, writeSessionPrelude } from "./session-index.ts";

const FAKE_AGY = `#!/usr/bin/env node
import readline from "node:readline";
import { appendFileSync, readFileSync } from "node:fs";

const record = process.env.FAKE_RECORD;
const auth = process.env.FAKE_AUTH || "ok";
const sticky = process.env.FAKE_STICKY_MODE === "1";
const noMode = process.env.FAKE_NO_MODE === "1";
const newDelay = Number(process.env.FAKE_NEW_DELAY_MS || 0);
// FAKE_SESSIONS_FILE is re-read on each session/list, so a test can add a session mid-run.
const listed = () => JSON.parse(process.env.FAKE_SESSIONS_FILE ? readFileSync(process.env.FAKE_SESSIONS_FILE, "utf8") : process.env.FAKE_SESSIONS || "[]");
const send = (value) => process.stdout.write(JSON.stringify(value) + "\\n");
const update = (u) => send({ jsonrpc: "2.0", method: "session/update", params: { sessionId: "s", update: u } });
let mode = process.env.FAKE_START_MODE || "default";
let model = "gemini-x";
let effort = "high";
let created = 0;
let permissionId = 500;
let promptId = null;
let pendingPermission = null;

function configOptions() {
  const options = [{
    id: "model", category: "model", type: "select", currentValue: model,
    options: [{ value: "gemini-x", name: "Gemini X" }, { value: "orch", name: "Orchestrator" }],
  }];
  if (model === "gemini-x") {
    options.push({
      id: "thought_level", category: "thought_level", type: "select", currentValue: effort,
      options: [{ value: "low", name: "Low" }, { value: "high", name: "High" }],
    });
  }
  if (!noMode) options.push({
    id: "mode", category: "mode", type: "select", currentValue: mode,
    options: [{ value: "default" }, { value: "accept-edits" }, { value: "plan" }],
  });
  return options;
}

function sessionResult(sessionId) {
  return {
    sessionId,
    configOptions: configOptions(),
    models: { currentModelId: "gemini-x-high", availableModels: [{ modelId: "gemini-x-high", name: "Gemini X (High)" }] },
    modes: noMode ? undefined : { currentModeId: mode, availableModes: [{ id: "default" }, { id: "accept-edits" }, { id: "plan" }] },
  };
}

function finish(stopReason) {
  if (promptId == null) return;
  const id = promptId;
  promptId = null;
  send({ jsonrpc: "2.0", id, result: { stopReason } });
}

const runCommand = (id, status) => ({
  sessionUpdate: id === "tc-1" ? "tool_call" : "tool_call_update",
  toolCallId: "tc-1",
  name: "run_command",
  kind: "execute",
  title: "touch ran.txt",
  status,
  args: { CommandLine: "touch ran.txt" },
  rawInput: { CommandLine: "touch ran.txt", WaitMsBeforeAsync: 5000 },
});

function prompt(msg) {
  promptId = msg.id;
  const text = msg.params.prompt[0].text;
  if (text.startsWith("slow")) {
    // Measured on agy: after the command ends, the model takes seconds to reply, and a
    // usage_update can come before any reply text.
    update({ ...runCommand("tc-1", "in_progress"), sessionUpdate: "tool_call", toolCallId: "tc-s" });
    finish("end_turn");
    setTimeout(() => {
      update({ ...runCommand("x", "completed"), toolCallId: "tc-s" });
      update({ sessionUpdate: "usage_update", used: 5, size: 100 });
    }, 50);
    setTimeout(() => {
      update({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Late reply" } });
      update({ sessionUpdate: "usage_update", used: 10, size: 100 });
    }, 900);
    return;
  }
  if (text.startsWith("fswrite")) {
    send({ jsonrpc: "2.0", id: 900, method: "fs/write_text_file", params: { sessionId: "s", path: process.cwd() + "/sneaky.txt", content: "x" } });
    finish("end_turn");
    return;
  }
  if (text.startsWith("defer")) {
    update({ ...runCommand("tc-1", "in_progress"), sessionUpdate: "tool_call", toolCallId: "tc-d" });
    update({ sessionUpdate: "tool_call", toolCallId: "tc-daemon", name: "run_command", kind: "execute", status: "in_progress", rawInput: { CommandLine: "serve", IsDaemon: true } });
    finish("end_turn");
    setTimeout(() => {
      update({ ...runCommand("x", "completed"), toolCallId: "tc-d", rawOutput: { output: "built" } });
      update({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Background done" } });
      update({ sessionUpdate: "usage_update", used: 10, size: 100 });
    }, 150);
    return;
  }
  update({ sessionUpdate: "session_info_update", title: "Touch a file" });
  update(runCommand("tc-1", "in_progress"));
  pendingPermission = permissionId++;
  send({
    jsonrpc: "2.0",
    id: pendingPermission,
    method: "session/request_permission",
    params: {
      sessionId: "s",
      toolCall: runCommand("tc-1", "pending"),
      options: [
        { optionId: "allow", name: "Allow", kind: "allow_once" },
        { optionId: "deny", name: "Deny", kind: "reject_once" },
      ],
    },
  });
}

function answered(msg) {
  if (msg.id !== pendingPermission) return;
  pendingPermission = null;
  const chosen = msg.result && msg.result.outcome && msg.result.outcome.optionId;
  if (chosen === "allow") {
    update({ ...runCommand("x", "completed"), rawOutput: { output: "" } });
    update({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Done" } });
  } else {
    update({ ...runCommand("x", "failed") });
    update({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Denied" } });
  }
  finish("end_turn");
}

function handle(msg) {
  const reply = (result) => send({ jsonrpc: "2.0", id: msg.id, result });
  switch (msg.method) {
    case "initialize":
      reply({
        protocolVersion: 1,
        agentCapabilities: {
          loadSession: true,
          promptCapabilities: { image: true },
          sessionCapabilities: { list: {}, load: {}, resume: {}, close: {} },
          _meta: { antigravity: { tier: true, systemPrompt: { append: true } } },
        },
        authMethods: [{ id: "oauth-personal" }, { id: "gemini-api-key" }],
      });
      return;
    case "session/new":
      if (auth === "fail") {
        send({ jsonrpc: "2.0", id: msg.id, error: { code: -32000, message: "Authentication required: token secret-xyz expired" } });
        return;
      }
      created += 1;
      if (newDelay) {
        const id = "new-" + created;
        setTimeout(() => reply(sessionResult(id)), newDelay);
        return;
      }
      reply(sessionResult("new-" + created));
      update({ sessionUpdate: "available_commands_update", availableCommands: [{ name: "plan", description: "Plan first" }] });
      return;
    case "session/load":
      reply(sessionResult(msg.params.sessionId));
      // agy replays history after the load response.
      setTimeout(() => update({ sessionUpdate: "user_message_chunk", content: { type: "text", text: "Earlier question" } }), 60);
      setTimeout(() => update({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Restored answer" } }), 160);
      setTimeout(() => update({ sessionUpdate: "tool_call", toolCallId: "w1", name: "write_file", kind: "edit", status: "in_progress", title: "Write refs.bib", rawInput: { TargetFile: "refs.bib" } }), 170);
      setTimeout(() => update({ sessionUpdate: "tool_call_update", toolCallId: "w1", status: "completed", content: [{ type: "diff", path: "refs.bib", newText: "@misc{x}\\n" }] }), 180);
      return;
    case "session/list": {
      const start = Number(msg.params.cursor || 0);
      const sessions = listed();
      const page = sessions.slice(start, start + 2);
      const result = { sessions: page };
      if (start + 2 < sessions.length) result.nextCursor = String(start + 2);
      reply(result);
      return;
    }
    case "session/set_config_option":
      if (msg.params.configId === "mode" && !sticky) mode = msg.params.value;
      if (msg.params.configId === "model") model = msg.params.value;
      if (msg.params.configId === "thought_level") effort = msg.params.value;
      reply({ configOptions: configOptions() });
      return;
    case "session/prompt":
      prompt(msg);
      return;
    default:
      reply({});
  }
}

readline.createInterface({ input: process.stdin, crlfDelay: Infinity }).on("line", (line) => {
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    return;
  }
  appendFileSync(record, line + "\\n");
  if (!msg.method) {
    answered(msg);
    return;
  }
  if (!Object.prototype.hasOwnProperty.call(msg, "id")) {
    if (msg.method === "session/cancel") finish("cancelled");
    return;
  }
  handle(msg);
});
`;

interface Boot {
  adapter: AcpAdapter;
  host: AdapterHost;
  events: Record<string, unknown>[];
  responses: Record<string, unknown>[];
  dir: string;
  project: string;
  sessionDir: string;
  record: string;
  env: NodeJS.ProcessEnv;
}

function prepare(extra: Record<string, string> = {}): Omit<Boot, "adapter" | "host" | "events" | "responses"> {
  const dir = realpathSync(mkdtempSync(join(tmpdir(), "pitex-agy-")));
  const project = join(dir, "project");
  const sessionDir = join(dir, "sessions");
  const binary = join(dir, "agy");
  const record = join(dir, "traffic.jsonl");
  writeFileSync(binary, FAKE_AGY, { mode: 0o755 });
  writeFileSync(record, "");
  mkdirSync(project);
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    AGY_PATH: binary,
    HOME: dir,
    PATH: process.env.PATH ?? "",
    FAKE_RECORD: record,
    ...extra,
  };
  return { dir, project, sessionDir, record, env };
}

async function boot(prepared = prepare()): Promise<Boot> {
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
  const adapter = new AcpAdapter(AGY);
  await adapter.start({
    cwd: prepared.project,
    sessionDir: prepared.sessionDir,
    developerInstructions: "Be terse.",
    env: prepared.env,
  });
  return { ...prepared, adapter, host, events, responses };
}

async function shutdown(booted: Boot): Promise<void> {
  await booted.adapter.stop();
  rmSync(booted.dir, { recursive: true, force: true });
}

function traffic(record: string): Record<string, any>[] {
  return readFileSync(record, "utf8")
    .split("\n")
    .filter((line) => line.trim())
    .map((line) => JSON.parse(line) as Record<string, any>);
}

function response(responses: Record<string, unknown>[], id: string): Record<string, unknown> {
  const found = responses.find((item) => item.id === id);
  if (!found) throw new Error(`missing response ${id}: ${JSON.stringify(responses)}`);
  return found;
}

async function waitUntil(label: string, ready: () => boolean, ms = 5000): Promise<void> {
  const deadline = Date.now() + ms;
  while (Date.now() < deadline) {
    if (ready()) return;
    await new Promise((resolve) => setTimeout(resolve, 15));
  }
  throw new Error(`timed out waiting for ${label}`);
}

const count = (events: Record<string, unknown>[], type: string) => events.filter((event) => event.type === type).length;

test("agy tool calls read as pi bash, write, edit, and read cards", () => {
  const state = createStreamState();
  const start = (update: Record<string, unknown>) =>
    mapSessionUpdate({ sessionUpdate: "tool_call", status: "in_progress", ...update }, state).find(
      (event) => event.type === "tool_execution_start"
    );
  const bash = start({ toolCallId: "a", name: "run_command", kind: "execute", rawInput: { CommandLine: "latexmk main.tex" } });
  assert.equal(bash?.toolName, "bash");
  assert.deepEqual(bash?.args, { command: "latexmk main.tex" });
  const write = start({ toolCallId: "b", name: "write_file", kind: "edit", rawInput: { Path: "/p/a.tex", Content: "x" } });
  assert.equal(write?.toolName, "write");
  assert.deepEqual(write?.args, { path: "/p/a.tex" });
  const edit = start({ toolCallId: "c", name: "replace_file_content", kind: "edit", rawInput: { Path: "/p/b.tex" } });
  assert.equal(edit?.toolName, "edit");
  const read = start({ toolCallId: "d", name: "view_file", kind: "read", rawInput: { Path: "/p/c.tex" } });
  assert.equal(read?.toolName, "read");
  assert.deepEqual(read?.args, { path: "/p/c.tex" });

  const permission = {
    toolCall: { toolCallId: "a", name: "run_command", kind: "execute", title: "touch ran.txt", rawInput: { CommandLine: "touch ran.txt" } },
  };
  const view = approvalFromPermission(7, permission, AGY.displayName);
  assert.equal(view.kind, "command");
  assert.equal(view.detail, "touch ran.txt");
  assert.equal(approvalKey(permission), approvalKey({ toolCall: { ...permission.toolCall, toolCallId: "z" } }));
  assert.notEqual(approvalKey(permission), approvalKey({ toolCall: { ...permission.toolCall, rawInput: { CommandLine: "rm -rf x" } } }));
  const argsOnly = (command: string) => ({ toolCall: { name: "run_command", kind: "execute", args: { CommandLine: command } } });
  assert.notEqual(approvalKey(argsOnly("touch a")), approvalKey(argsOnly("rm -rf b")), "args-only requests keep the command in the key");
  assert.equal(approvalKey({ toolCall: { name: "run_command", kind: "execute", title: "Run" } }), "", "no command and no path: no reusable key");

  const catalog = catalogFromAcp(
    {
      configOptions: [
        { id: "model", category: "model", currentValue: "gemini-x", options: [{ value: "gemini-x", name: "Gemini X" }] },
        { id: "mode", category: "mode", currentValue: "accept-edits", options: [] },
      ],
      models: { currentModelId: "gemini-x-high", availableModels: [{ modelId: "gemini-x-high" }] },
    },
    AGY
  );
  assert.deepEqual(catalog.models.map((model) => [model.id, model.provider]), [["gemini-x", "agy"]]);
  assert.equal(catalog.currentModelId, "gemini-x");
  assert.deepEqual(catalog.efforts, []);
  assert.equal(catalog.currentMode, "accept-edits");
});

test("findCli and cliLaunchSpec use AGY_PATH and the fixed acp arguments", () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-agy-which-"));
  try {
    const binary = join(dir, "agy");
    writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
    assert.equal(findCli(AGY, { AGY_PATH: binary, PATH: "", HOME: dir }), binary);
    assert.equal(findCli(AGY, { PATH: dir, HOME: dir }), binary);
    assert.equal(findCli(AGY, { AGY_PATH: join(dir, "missing"), PATH: "", HOME: dir }), null);
    assert.deepEqual(cliLaunchSpec(AGY, binary, "linux"), { command: binary, args: ["acp", "--sandbox", "--disable-telemetry"] });
    const cmd = join(dir, "agy.cmd");
    writeFileSync(cmd, "@ECHO off\r\n");
    assert.deepEqual(cliLaunchSpec(AGY, cmd, "win32").args, ["/d", "/s", "/c", `"${cmd}" acp --sandbox --disable-telemetry`]);
    assert.throws(() => cliLaunchSpec(AGY, join(dir, "bad%x%.cmd"), "win32"), /Antigravity CLI path/);
    const shim = join(dir, "agy-node");
    writeFileSync(shim, "#!/usr/bin/env node\n", { mode: 0o755 });
    assert.deepEqual(cliLaunchSpec(AGY, shim, "win32"), {
      command: process.execPath,
      args: [shim, "acp", "--sandbox", "--disable-telemetry"],
    });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("ACP adapter sources do not reference credential or session files", () => {
  const needles = [
    ".gemini",
    "antigravity-cli",
    "oauth_creds",
    "auth.json",
    "credentials.json",
    "application_default_credentials",
    "adc.json",
    "GEMINI_API_KEY",
    ".codex",
    ".claude",
    "~/.grok",
    "XAI_API_KEY",
  ];
  for (const name of ["acp-adapter.ts", "acp-map.ts", "acp-profiles.ts"]) {
    const source = readFileSync(new URL(`./${name}`, import.meta.url), "utf8");
    for (const needle of needles) assert.equal(source.includes(needle), false, `${name} mentions ${needle}`);
  }
});

test("agy starts without authenticate, forces mode default, and reads models from config options", { timeout: 20_000 }, async () => {
  const booted = await boot(prepare({ FAKE_START_MODE: "accept-edits" }));
  const { adapter, host, responses, record } = booted;
  try {
    assert.equal(adapter.unavailableReason, null, adapter.unavailableReason ?? "");
    const lines = traffic(record);
    assert.deepEqual(lines.find((line) => line.method === "initialize")?.params.clientCapabilities, {});
    assert.equal(lines.some((line) => line.method === "authenticate"), false);
    const created = lines.find((line) => line.method === "session/new");
    assert.equal(created?.params.cwd, booted.project);
    assert.deepEqual(created?.params._meta, { antigravity: { systemPrompt: { append: "Be terse." } } });
    const forced = lines.find((line) => line.method === "session/set_config_option");
    assert.deepEqual(
      { configId: forced?.params.configId, value: forced?.params.value },
      { configId: "mode", value: "default" }
    );

    await adapter.handleCommand({ type: "get_available_models", id: "models" }, host);
    assert.deepEqual((response(responses, "models").data as { models: unknown[] }).models, [
      { id: "gemini-x", provider: "agy", name: "Gemini X" },
      { id: "orch", provider: "agy", name: "Orchestrator" },
    ]);
    await adapter.handleCommand({ type: "get_state", id: "state" }, host);
    assert.deepEqual(response(responses, "state").data, {
      model: { id: "gemini-x", provider: "agy", name: "Gemini X" },
      thinkingLevel: "high",
      mode: {
        id: "default",
        options: [
          { id: "default", label: "default" },
          { id: "accept-edits", label: "accept-edits" },
          { id: "plan", label: "plan" },
        ],
      },
    });
    await adapter.handleCommand({ type: "get_available_thinking_levels", id: "levels" }, host);
    assert.deepEqual((response(responses, "levels").data as { levels: string[] }).levels, ["low", "high"]);

    await adapter.handleCommand({ type: "set_model", id: "orch", modelId: "orch" }, host);
    assert.equal(response(responses, "orch").success, true);
    const setModel = traffic(record).filter((line) => line.method === "session/set_config_option").at(-1);
    assert.deepEqual([setModel?.params.configId, setModel?.params.value], ["model", "orch"]);
    await adapter.handleCommand({ type: "get_available_thinking_levels", id: "none" }, host);
    assert.deepEqual((response(responses, "none").data as { levels: string[] }).levels, []);
    await adapter.handleCommand({ type: "set_thinking_level", id: "refused", level: "high" }, host);
    assert.equal(response(responses, "refused").success, false);
    await adapter.handleCommand({ type: "set_model", id: "back", modelId: "gemini-x" }, host);
    await adapter.handleCommand({ type: "get_available_thinking_levels", id: "again" }, host);
    assert.deepEqual((response(responses, "again").data as { levels: string[] }).levels, ["low", "high"]);
    await adapter.handleCommand({ type: "get_commands", id: "commands" }, host);
    assert.deepEqual((response(responses, "commands").data as { commands: unknown[] }).commands, [
      { name: "plan", description: "Plan first", source: "agy" },
    ]);
  } finally {
    await shutdown(booted);
  }

  const sticky = await boot(prepare({ FAKE_START_MODE: "accept-edits", FAKE_STICKY_MODE: "1" }));
  try {
    assert.match(sticky.adapter.unavailableReason ?? "", /accept-edits/);
    await sticky.adapter.handleCommand({ type: "prompt", id: "p", message: "hi" }, sticky.host);
    assert.equal(response(sticky.responses, "p").success, false);
    assert.equal(traffic(sticky.record).some((line) => line.method === "session/prompt"), false);
  } finally {
    await shutdown(sticky);
  }

  const silent = await boot(prepare({ FAKE_NO_MODE: "1" }));
  try {
    assert.match(silent.adapter.unavailableReason ?? "", /did not report its permission mode/);
  } finally {
    await shutdown(silent);
  }
});

test("agy approvals: deny, allow once, and a session-only always allow without allow_always", { timeout: 20_000 }, async () => {
  const booted = await boot();
  const { adapter, host, events, responses, record } = booted;
  const answers = () => traffic(record).filter((line) => !line.method && line.result?.outcome);
  try {
    await adapter.handleCommand({ type: "prompt", id: "p1", message: "touch it" }, host);
    await waitUntil("approval 1", () => count(events, "approval_request") === 1);
    const approval = events.find((event) => event.type === "approval_request");
    assert.equal(approval?.kind, "command");
    assert.equal(approval?.detail, "touch ran.txt");
    const card = events.find((event) => event.type === "tool_execution_start");
    assert.equal(card?.toolName, "bash");
    assert.deepEqual(
      events.filter((event) => event.type === "extension_error").map((event) => event.error),
      [AGY.notice],
      "the in-project edit policy is stated once"
    );

    await adapter.handleCommand({ type: "prompt", id: "queued", message: "touch it" }, host);
    assert.deepEqual(response(responses, "queued").data, { queued: true });
    await adapter.handleCommand({ type: "approval_response", id: String(approval?.id), decision: "deny" }, host);
    await waitUntil("approval 2 from the queued prompt", () => count(events, "approval_request") === 2);
    assert.equal(answers()[0]?.result.outcome.optionId, "deny");
    const denied = events.find((event) => event.type === "tool_execution_end");
    assert.equal(denied?.isError, true);
    assert.equal(count(events, "agent_end"), 1);

    const second = events.filter((event) => event.type === "approval_request")[1];
    await adapter.handleCommand({ type: "approval_response", id: String(second?.id), decision: "always_allow" }, host);
    await waitUntil("turn 2", () => count(events, "agent_end") === 2);
    assert.equal(answers()[1]?.result.outcome.optionId, "allow");

    await adapter.handleCommand({ type: "prompt", id: "p3", message: "touch it" }, host);
    await waitUntil("turn 3", () => count(events, "agent_end") === 3);
    assert.equal(count(events, "approval_request"), 2, "the same command is allowed again without asking");
    assert.equal(answers()[2]?.result.outcome.optionId, "allow");

    await adapter.handleCommand({ type: "new_session", id: "fresh" }, host);
    await adapter.handleCommand({ type: "prompt", id: "p4", message: "touch it" }, host);
    await waitUntil("asked again in a new session", () => count(events, "approval_request") === 3);
    await adapter.handleCommand({ type: "abort", id: "stop" }, host);
    assert.equal(count(events, "extension_error"), 1, "every command here went through an approval");
    assert.equal(count(events, "agent_start"), count(events, "agent_end"));
  } finally {
    await shutdown(booted);
  }
});

test("ACP permission options become card actions and map back to the offered option", () => {
  const all = [
    { optionId: "o1", kind: "allow_once", name: "Yes" },
    { optionId: "o2", kind: "allow_always", name: "Always" },
    { optionId: "o3", kind: "reject_once" },
    { optionId: "o4", kind: "reject_always" },
  ];
  assert.deepEqual(permissionActions(all, true), [
    { id: "allow_once", label: "Yes", behavior: "allow", variant: "primary" },
    { id: "allow_always", label: "Always", behavior: "allow", variant: "secondary" },
    { id: "reject_once", label: "Deny", behavior: "deny", variant: "danger" },
    { id: "reject_always", label: "Always deny", behavior: "deny", variant: "danger" },
  ]);
  const agy = [
    { optionId: "allow", kind: "allow_once", name: "Allow" },
    { optionId: "deny", kind: "reject_once", name: "Deny" },
  ];
  assert.deepEqual(permissionActions(agy, true).map((action) => action.id), ["allow_once", "always_allow", "reject_once"]);
  assert.deepEqual(permissionActions(agy, false).map((action) => action.id), ["allow_once", "reject_once"], "no key, no session rule");
  assert.deepEqual(permissionResultForAction("reject_always", undefined, all), { outcome: { outcome: "selected", optionId: "o4" } });
  assert.deepEqual(permissionResultForAction("always_allow", undefined, agy), { outcome: { outcome: "selected", optionId: "allow" } });
  assert.deepEqual(permissionResultForAction(undefined, "deny", agy), { outcome: { outcome: "selected", optionId: "deny" } });
});

test("agy cards carry actions and the tool detail; abort settles an open card with approval_resolved", { timeout: 20_000 }, async () => {
  const booted = await boot();
  const { adapter, host, events, record } = booted;
  const answers = () => traffic(record).filter((line) => !line.method && line.result?.outcome);
  try {
    await adapter.handleCommand({ type: "prompt", id: "p1", message: "touch it" }, host);
    await waitUntil("approval", () => count(events, "approval_request") === 1);
    const approval = events.find((event) => event.type === "approval_request");
    assert.deepEqual((approval?.actions as { id: string }[]).map((action) => action.id), ["allow_once", "always_allow", "reject_once"]);
    assert.deepEqual(approval?.toolDetail, { kind: "shell", label: "touch ran.txt", command: "touch ran.txt" });
    await adapter.handleCommand({ type: "approval_response", id: String(approval?.id), decision: "deny", actionId: "reject_once" }, host);
    await waitUntil("turn 1", () => count(events, "agent_end") === 1);
    assert.equal(answers()[0]?.result.outcome.optionId, "deny");
    assert.equal(count(events, "approval_resolved"), 0, "the host's own answer needs no resolved event");

    await adapter.handleCommand({ type: "prompt", id: "p2", message: "touch it" }, host);
    await waitUntil("approval 2", () => count(events, "approval_request") === 2);
    const second = events.filter((event) => event.type === "approval_request")[1];
    await adapter.handleCommand({ type: "abort", id: "stop" }, host);
    assert.deepEqual(events.find((event) => event.type === "approval_resolved"), {
      type: "approval_resolved",
      id: String(second.id),
      outcome: "cancelled",
    });
  } finally {
    await shutdown(booted);
  }
});

test("agy set_mode forces the picked mode before each prompt, a new chat goes back to default, and bypass is refused", { timeout: 20_000 }, async () => {
  const booted = await boot();
  const { adapter, host, events, responses, record } = booted;
  const modeSets = () =>
    traffic(record)
      .filter((line) => line.method === "session/set_config_option" && line.params.configId === "mode")
      .map((line) => line.params.value);
  try {
    await adapter.handleCommand({ type: "set_mode", id: "bad", modeId: "bypassPermissions" }, host);
    assert.equal(response(responses, "bad").success, false);
    await adapter.handleCommand({ type: "set_mode", id: "edits", modeId: "accept-edits" }, host);
    assert.equal(response(responses, "edits").success, true);
    assert.equal(modeSets().at(-1), "accept-edits");
    assert.deepEqual((events.find((event) => event.type === "mode_changed")?.mode as { id: string }).id, "accept-edits");
    await adapter.handleCommand({ type: "get_state", id: "state" }, host);
    assert.equal((response(responses, "state").data as { mode: { id: string } }).mode.id, "accept-edits");

    const before = modeSets().length;
    await adapter.handleCommand({ type: "prompt", id: "p1", message: "touch it" }, host);
    await waitUntil("approval", () => count(events, "approval_request") === 1);
    assert.deepEqual(modeSets().slice(before), ["accept-edits"], "the picked mode, not default, before the prompt");
    const approval = events.find((event) => event.type === "approval_request");
    await adapter.handleCommand({ type: "approval_response", id: String(approval?.id), decision: "approve", actionId: "allow_once" }, host);
    await waitUntil("turn", () => count(events, "agent_end") === 1);

    await adapter.handleCommand({ type: "set_mode", id: "back", modeId: "default" }, host);
    assert.equal(modeSets().at(-1), "default");
    await adapter.handleCommand({ type: "set_mode", id: "edits2", modeId: "accept-edits" }, host);
    await adapter.handleCommand({ type: "new_session", id: "fresh" }, host);
    const changes = events.filter((event) => event.type === "mode_changed").map((event) => (event.mode as { id: string }).id);
    assert.deepEqual(changes, ["accept-edits", "default", "accept-edits", "default"]);
    await adapter.handleCommand({ type: "prompt", id: "p2", message: "touch it" }, host);
    await waitUntil("approval 2", () => count(events, "approval_request") === 2);
    assert.equal(modeSets().at(-1), "default", "the new session starts in default");
  } finally {
    await shutdown(booted);
  }
});

test("agy keeps the turn open while a foreground command still runs after end_turn", { timeout: 20_000 }, async () => {
  const booted = await boot();
  const { adapter, host, events } = booted;
  try {
    await adapter.handleCommand({ type: "prompt", id: "p", message: "defer build" }, host);
    await new Promise((resolve) => setTimeout(resolve, 80));
    assert.equal(count(events, "agent_end"), 0, "end_turn arrived while run_command was in progress");
    await waitUntil("deferred end", () => count(events, "agent_end") === 1);
    const end = events.findIndex((event) => event.type === "agent_end");
    const text = events.findIndex((event) => JSON.stringify(event).includes("Background done"));
    assert.ok(text >= 0 && text < end);
    assert.ok(
      events.some((event) => event.type === "extension_error" && /ran a command without asking/.test(String(event.error))),
      "a command that finished without a permission request is reported"
    );
  } finally {
    await shutdown(booted);
  }
});

test("agy waits for a slow follow-up reply, and nothing arrives after the turn closed", { timeout: 20_000 }, async () => {
  const booted = await boot();
  const { adapter, host, events } = booted;
  try {
    await adapter.handleCommand({ type: "prompt", id: "p", message: "slow build" }, host);
    await new Promise((resolve) => setTimeout(resolve, 600));
    assert.equal(count(events, "agent_end"), 0, "a usage_update before any reply text does not close the turn");
    await waitUntil("late reply closes the turn", () => count(events, "agent_end") === 1);
    const end = events.findIndex((event) => event.type === "agent_end");
    const text = events.findIndex((event) => JSON.stringify(event).includes("Late reply"));
    assert.ok(text >= 0 && text < end);
    assert.equal(count(events, "agent_start"), 1);
  } finally {
    await shutdown(booted);
  }
});

test("a prompt sent while the session starts is queued, and Stop drops it", { timeout: 20_000 }, async () => {
  const booted = await boot(prepare({ FAKE_NEW_DELAY_MS: "300" }));
  const { adapter, host, events, responses, record } = booted;
  try {
    await adapter.handleCommand({ type: "new_session", id: "fresh" }, host);
    const first = adapter.handleCommand({ type: "prompt", id: "a", message: "touch it" }, host);
    await new Promise((resolve) => setTimeout(resolve, 50));
    await adapter.handleCommand({ type: "prompt", id: "b", message: "touch it" }, host);
    assert.deepEqual(response(responses, "b").data, { queued: true });
    await adapter.handleCommand({ type: "abort", id: "stop" }, host);
    await first;
    await new Promise((resolve) => setTimeout(resolve, 100));
    const lines = traffic(record);
    assert.equal(lines.filter((line) => line.method === "session/new").length, 2, "boot plus one for the prompt");
    assert.equal(lines.some((line) => line.method === "session/prompt"), false, "Stop dropped the waiting prompt");
    assert.equal(count(events, "agent_start"), count(events, "agent_end"));

    await adapter.handleCommand({ type: "prompt", id: "c", message: "fswrite" }, host);
    await waitUntil("fswrite turn", () => count(events, "agent_end") === 2);
    const refused = traffic(record).find((line) => line.id === 900 && !line.method);
    assert.ok(refused?.error, "fs/write_text_file is refused when fs was not advertised");
    assert.equal(existsSync(join(booted.project, "sneaky.txt")), false);
  } finally {
    await shutdown(booted);
  }
});

test("agy sessions for this project are mirrored once into the history list", { timeout: 20_000 }, () => {
  const dir = realpathSync(mkdtempSync(join(tmpdir(), "pitex-agy-import-")));
  try {
    const sessionDir = join(dir, "sessions");
    const project = join(dir, "project");
    const known = join(sessionDir, "agy-known.jsonl");
    writeSessionPrelude(known, "started in pitex", project);
    rememberSession(sessionDir, known, "known");
    const rows = [
      { sessionId: "s-1", cwd: project, title: "Fix the intro", updatedAt: "2026-01-02T03:04:05Z" },
      { sessionId: "s-2", cwd: `${project}/`, title: "", updatedAt: "bad" },
      { sessionId: "s-3", cwd: join(dir, "other"), title: "Elsewhere", updatedAt: "2026-01-02T03:04:05Z" },
      { sessionId: "known", cwd: project, title: "Already here", updatedAt: "2026-01-02T03:04:05Z" },
      { sessionId: "../escape", cwd: project, title: "Bad id", updatedAt: "2026-01-02T03:04:05Z" },
    ];
    const written = importListedSessions(sessionDir, "agy", rows, project);
    assert.deepEqual(written.map((path) => path.split("/").pop()), ["agy-s-1.jsonl", "agy-s-2.jsonl"]);
    const first = readFileSync(join(sessionDir, "agy-s-1.jsonl"), "utf8");
    assert.match(first, /Fix the intro/);
    assert.match(readFileSync(join(sessionDir, "agy-s-2.jsonl"), "utf8"), /Untitled conversation/);
    assert.equal(statSync(join(sessionDir, "agy-s-1.jsonl")).mtime.toISOString(), "2026-01-02T03:04:05.000Z");
    assert.equal(importListedSessions(sessionDir, "agy", rows, project).length, 0);
    const ids = Object.values(loadIndex(sessionDir).sessions).map((record) => record.threadId).sort();
    assert.deepEqual(ids, ["known", "s-1", "s-2"]);
    assert.deepEqual(readdirSync(sessionDir).filter((name) => name.endsWith(".jsonl")).sort(), [
      "agy-known.jsonl",
      "agy-s-1.jsonl",
      "agy-s-2.jsonl",
    ]);
    assert.equal(existsSync(join(dir, "escape.jsonl")), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("agy session/list import pages, survives a restart, and a picked session replays quietly", { timeout: 20_000 }, async () => {
  const prepared = prepare();
  const rows = [
    { sessionId: "old-1", cwd: prepared.project, title: "Old one", updatedAt: "2026-01-01T00:00:00Z" },
    { sessionId: "far", cwd: prepared.dir, title: "Other folder", updatedAt: "2026-01-01T00:00:00Z" },
    { sessionId: "old-2", cwd: prepared.project, title: "Old two", updatedAt: "2026-01-03T00:00:00Z" },
  ];
  const listFile = join(prepared.dir, "listed.json");
  writeFileSync(listFile, JSON.stringify(rows));
  prepared.env.FAKE_SESSIONS_FILE = listFile;
  const first = await boot(prepared);
  const mirrors = () => readdirSync(prepared.sessionDir).filter((name) => name.endsWith(".jsonl")).sort();
  try {
    assert.deepEqual(mirrors(), ["agy-old-1.jsonl", "agy-old-2.jsonl"]);
    const lists = traffic(prepared.record).filter((line) => line.method === "session/list");
    assert.deepEqual(lists.map((line) => line.params.cursor ?? null), [null, "2"]);
  } finally {
    await first.adapter.stop();
  }

  const second = await boot(prepared);
  const { adapter, host, events, responses, record } = second;
  try {
    assert.deepEqual(mirrors(), ["agy-old-1.jsonl", "agy-old-2.jsonl"]);
    await adapter.handleCommand({ type: "get_state", id: "first" }, host);
    const before = events.length;
    await adapter.handleCommand(
      { type: "switch_session", id: "sw", sessionPath: join(prepared.sessionDir, "agy-old-2.jsonl") },
      host
    );
    assert.equal(response(responses, "sw").success, true);
    // Replayed history stays out of the live stream; only the restored mode is announced.
    assert.deepEqual(events.slice(before).map((event) => event.type), ["mode_changed"]);
    const loaded = traffic(record).find((line) => line.method === "session/load");
    assert.equal(loaded?.params.sessionId, "old-2");
    assert.deepEqual(loaded?.params._meta, { antigravity: { systemPrompt: { append: "Be terse." } } });
    const modes = traffic(record).filter((line) => line.method === "session/set_config_option" && line.params.configId === "mode");
    assert.equal(modes.at(-1)?.params.sessionId, "old-2");
    await adapter.handleCommand({ type: "get_messages", id: "history" }, host);
    const blob = JSON.stringify(response(responses, "history").data);
    assert.match(blob, /Earlier question/);
    assert.match(blob, /Restored answer/);
    // The replayed tool call keeps its typed card.
    const messages = (response(responses, "history").data as { messages: Record<string, unknown>[] }).messages;
    const written = messages.find((message) => message.role === "toolResult");
    assert.equal((written?.detail as Record<string, unknown>)?.kind, "write");
    assert.match(String((written?.detail as Record<string, unknown>)?.diff), /\+@misc\{x\}/);
    assert.equal(written?.status, "completed");

    // A session started in a terminal meanwhile appears on refresh_sessions, once.
    writeFileSync(listFile, JSON.stringify([...rows, { sessionId: "new-1", cwd: prepared.project, title: "From the terminal", updatedAt: "2026-01-04T00:00:00Z" }]));
    await adapter.handleCommand({ type: "refresh_sessions", id: "r1" }, host);
    assert.deepEqual(response(responses, "r1").data, { imported: 1 });
    await adapter.handleCommand({ type: "refresh_sessions", id: "r2" }, host);
    assert.deepEqual(response(responses, "r2").data, { imported: 0 });
    assert.deepEqual(mirrors(), ["agy-new-1.jsonl", "agy-old-1.jsonl", "agy-old-2.jsonl"]);
    writeFileSync(listFile, JSON.stringify(rows));

    await adapter.handleCommand({ type: "prompt", id: "p", message: "touch it" }, host);
    await waitUntil("approval", () => count(events, "approval_request") === 1);
    await adapter.handleCommand({ type: "abort", id: "stop" }, host);
    await adapter.handleCommand({ type: "new_session", id: "fresh" }, host);
    assert.deepEqual(mirrors(), ["agy-new-1.jsonl", "agy-old-1.jsonl", "agy-old-2.jsonl"]);
    writeFileSync(join(prepared.sessionDir, "claude-x.jsonl"), "");
    await adapter.handleCommand(
      { type: "switch_session", id: "foreign", sessionPath: join(prepared.sessionDir, "claude-x.jsonl") },
      host
    );
    assert.match(String(response(responses, "foreign").error), /another assistant \(claude\)/);
  } finally {
    await shutdown(second);
  }
});

test("a missing or signed-out Antigravity CLI is reported without forwarding its error text", { timeout: 20_000 }, async () => {
  const empty = mkdtempSync(join(tmpdir(), "pitex-agy-empty-"));
  try {
    const missing = new AcpAdapter(AGY);
    await missing.start({ cwd: empty, sessionDir: empty, env: { PATH: "", HOME: empty, AGY_PATH: "" } });
    assert.equal(missing.unavailableReason, AGY.missing);
    const responses: Record<string, unknown>[] = [];
    await missing.handleCommand({ type: "prompt", id: "p", message: "Hi" }, {
      emit() {},
      respond(command, success, data, error, id) {
        responses.push({ command, success, error, id });
      },
    });
    assert.equal(responses[0]?.success, false);
    assert.match(String(responses[0]?.error), /run `agy` and sign in/);
  } finally {
    rmSync(empty, { recursive: true, force: true });
  }

  const signedOut = await boot(prepare({ FAKE_AUTH: "fail" }));
  try {
    assert.equal(signedOut.adapter.unavailableReason, AGY.signedOut);
    assert.equal((signedOut.adapter.unavailableReason ?? "").includes("secret-xyz"), false);
    assert.equal(traffic(signedOut.record).some((line) => line.method === "authenticate"), false);
  } finally {
    await shutdown(signedOut);
  }
});
