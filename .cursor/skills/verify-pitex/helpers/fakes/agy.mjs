#!/usr/bin/env node
// Stand-in for `agy acp`: the ACP shapes measured against Antigravity CLI
// 1.5.15. No authenticate method is needed, the session starts in
// accept-edits so the bridge has to switch it to default, run_command asks
// with only allow_once and reject_once, session/list spans every folder, and
// session/load replays history after its response.
import readline from "node:readline";

import { recordLine, recordNote, recordSpawn } from "./record.mjs";

recordSpawn();

const auth = process.env.FAKE_AUTH || "ok";
const project = process.env.FAKE_ROOT || process.cwd();
const send = (value) => process.stdout.write(`${JSON.stringify(value)}\n`);
const update = (u) => send({ jsonrpc: "2.0", method: "session/update", params: { sessionId: "sess", update: u } });
const say = (text) => update({ sessionUpdate: "agent_message_chunk", content: { type: "text", text } });

let mode = "accept-edits";
let created = 0;
let promptId = null;
let permissionId = 70;

function configOptions() {
  return [
    { id: "model", category: "model", type: "select", currentValue: "gemini-test", options: [{ value: "gemini-test", name: "Gemini Test" }] },
    {
      id: "thought_level",
      category: "thought_level",
      type: "select",
      currentValue: "high",
      options: [{ value: "low", name: "Low" }, { value: "high", name: "High" }],
    },
    { id: "mode", category: "mode", type: "select", currentValue: mode, options: [{ value: "default" }, { value: "accept-edits" }, { value: "plan" }] },
  ];
}

function finish(stopReason) {
  if (promptId == null) return;
  const id = promptId;
  promptId = null;
  send({ jsonrpc: "2.0", id, result: { stopReason } });
}

const tool = (status) => ({
  sessionUpdate: status === "in_progress" ? "tool_call" : "tool_call_update",
  toolCallId: "call-1",
  name: "run_command",
  kind: "execute",
  title: "latexmk main.tex",
  status,
  rawInput: { CommandLine: "latexmk main.tex" },
});

function startPrompt(msg) {
  promptId = msg.id;
  const text = (msg.params?.prompt ?? []).map((part) => part?.text ?? "").join("\n");
  if (text.includes("PLAIN")) {
    say("Noted");
    finish("end_turn");
    return;
  }
  say(text.includes("HOLD") ? "Working" : "Hello");
  if (text.includes("HOLD")) return;
  update(tool("in_progress"));
  send({
    jsonrpc: "2.0",
    id: permissionId++,
    method: "session/request_permission",
    params: {
      sessionId: "sess",
      toolCall: { ...tool("pending"), sessionUpdate: "tool_call" },
      options: [
        { optionId: "allow", name: "Allow", kind: "allow_once" },
        { optionId: "deny", name: "Deny", kind: "reject_once" },
      ],
    },
  });
}

function answered(msg) {
  const outcome = msg.result?.outcome;
  if (!outcome) return;
  recordNote({ permission: outcome.optionId ?? outcome.outcome });
  if (outcome.outcome === "cancelled") {
    finish("cancelled");
    return;
  }
  update({ ...tool(outcome.optionId === "allow" ? "completed" : "failed"), rawOutput: { output: "" } });
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
          sessionCapabilities: { list: {}, load: {}, resume: {}, close: {} },
          _meta: { antigravity: { systemPrompt: { append: true } } },
        },
        authMethods: [{ id: "oauth-personal" }],
      });
      return;
    case "session/new":
      if (auth === "signed-out") {
        send({ jsonrpc: "2.0", id: msg.id, error: { code: -32000, message: "Authentication required" } });
        return;
      }
      created += 1;
      mode = "accept-edits";
      reply({ sessionId: `sess-${created}`, configOptions: configOptions() });
      return;
    case "session/load":
      reply({ configOptions: configOptions() });
      setTimeout(() => say("Restored"), 60);
      return;
    case "session/list":
      reply({
        sessions: [
          { sessionId: "home-1", cwd: project, title: "From the agy home", updatedAt: "2026-01-02T03:04:05Z" },
          { sessionId: "home-2", cwd: "/elsewhere", title: "Other project", updatedAt: "2026-01-02T03:04:05Z" },
        ],
      });
      return;
    case "session/set_config_option":
      if (msg.params?.configId === "mode") mode = msg.params.value;
      reply({ configOptions: configOptions() });
      return;
    case "session/prompt":
      startPrompt(msg);
      return;
    default:
      reply({});
  }
}

readline.createInterface({ input: process.stdin, crlfDelay: Infinity }).on("line", (line) => {
  recordLine(line);
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    return;
  }
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
