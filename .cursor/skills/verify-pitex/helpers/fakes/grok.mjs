#!/usr/bin/env node
import readline from "node:readline";

import { recordLine, recordNote, recordSpawn } from "./record.mjs";

recordSpawn();

const auth = process.env.FAKE_AUTH || "ok";
const inside = process.env.FAKE_INSIDE;
const outsideFile = process.env.FAKE_OUTSIDE_FILE;
const outsideDir = process.env.FAKE_OUTSIDE_DIR;
const project = process.env.FAKE_ROOT;
const nodeBin = process.env.FAKE_NODE || process.execPath;
const send = (value) => process.stdout.write(`${JSON.stringify(value)}\n`);
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

let promptId = null;
let nextPermission = 42;
let trackedTerminal = null;
let mode = "basic";

function promptText(msg) {
  const prompt = msg.params?.prompt;
  if (!Array.isArray(prompt)) return "";
  return prompt.map((part) => (typeof part?.text === "string" ? part.text : "")).join("\n");
}

function finishPrompt(stopReason) {
  if (promptId == null) return;
  const id = promptId;
  promptId = null;
  send({ jsonrpc: "2.0", id, result: { stopReason } });
}

function initResult() {
  const authMethods = auth === "signed-out" ? [{ id: "xai.api_key" }] : [{ id: "cached_token" }, { id: "xai.api_key" }];
  return {
    protocolVersion: 1,
    agentCapabilities: { loadSession: true },
    authMethods,
    _meta: {
      modelState: {
        currentModelId: "grok-test",
        availableModels: [
          { modelId: "grok-test", name: "Grok Test", reasoningEfforts: ["low", "high"] },
        ],
      },
    },
  };
}

function continueTurn(msg) {
  const outcome = msg.result?.outcome?.outcome;
  if (!outcome) return;
  recordNote({ permissionOutcome: outcome, id: msg.id });
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
        rawOutput: outcome === "selected" ? "ok" : "denied",
      },
    },
  });
  finishPrompt("end_turn");
}

function continueFiles(msg) {
  if (msg.result?.terminalId) {
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

function startPrompt(msg) {
  promptId = msg.id;
  const text = promptText(msg);
  if (text.includes("JAIL")) {
    mode = "files";
    send({ jsonrpc: "2.0", id: 51, method: "fs/read_text_file", params: { sessionId: "sess-1", path: inside } });
    send({ jsonrpc: "2.0", id: 54, method: "fs/read_text_file", params: { sessionId: "sess-1", path: outsideFile } });
    send({ jsonrpc: "2.0", id: 55, method: "fs/write_text_file", params: { sessionId: "sess-1", path: `${project}/from-grok.txt`, content: "written-inside\n" } });
    send({ jsonrpc: "2.0", id: 56, method: "fs/write_text_file", params: { sessionId: "sess-1", path: `${outsideDir}/pwned.txt`, content: "written-outside\n" } });
    send({ jsonrpc: "2.0", id: 52, method: "terminal/create", params: { sessionId: "sess-1", command: "hostname", cwd: outsideDir } });
    const script =
      "const fs=require('fs'); fs.writeFileSync(process.env.FAKE_CWD_FILE, process.cwd()); process.stdout.write('scoped-ok'); Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 600);";
    const quotedNode = `'${String(nodeBin).replaceAll("'", "'\\''")}'`;
    const quotedScript = `'${script.replaceAll("'", "'\\''")}'`;
    send({
      jsonrpc: "2.0",
      id: 53,
      method: "terminal/create",
      params: {
        sessionId: "sess-1",
        command: `${quotedNode} -e ${quotedScript}`,
        cwd: project,
      },
    });
    return;
  }
  if (text.includes("PLAIN")) {
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: { update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Noted" } } },
    });
    finishPrompt("end_turn");
    return;
  }
  send({
    jsonrpc: "2.0",
    method: "session/update",
    params: { update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: text.includes("HOLD") ? "Working" : "Hello" } } },
  });
  if (text.includes("HOLD")) return;
  send({
    jsonrpc: "2.0",
    method: "session/update",
    params: {
      update: {
        sessionUpdate: "tool_call",
        toolCallId: "tool-1",
        kind: "execute",
        status: "pending",
        rawInput: { command: "latexmk main.tex" },
      },
    },
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
}

rl.on("line", (line) => {
  recordLine(line);
  let msg;
  try {
    msg = JSON.parse(line);
  } catch {
    return;
  }
  if (!msg.method) {
    if (mode === "files") continueFiles(msg);
    else continueTurn(msg);
    return;
  }
  if (!Object.prototype.hasOwnProperty.call(msg, "id")) {
    if (msg.method === "session/cancel") finishPrompt("cancelled");
    return;
  }
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
  if (msg.method === "session/new") {
    send({ jsonrpc: "2.0", id: msg.id, result: { sessionId: "sess-1" } });
    return;
  }
  if (msg.method === "session/load") {
    send({
      jsonrpc: "2.0",
      method: "session/update",
      params: { update: { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Restored" } } },
    });
    send({ jsonrpc: "2.0", id: msg.id, result: {} });
    return;
  }
  if (msg.method === "session/prompt") {
    startPrompt(msg);
    return;
  }
  send({ jsonrpc: "2.0", id: msg.id, result: {} });
});
