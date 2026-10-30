#!/usr/bin/env node
import readline from "node:readline";

import { recordLine, recordNote, recordSpawn } from "./record.mjs";

recordSpawn();

const auth = process.env.FAKE_AUTH || "ok";
const send = (value) => process.stdout.write(`${JSON.stringify(value)}\n`);
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

let nextApproval = 42;
let pendingTurn = null;
const turns = [];

function promptText(msg) {
  const input = msg.params?.input;
  if (!Array.isArray(input)) return "";
  return input.map((part) => (typeof part?.text === "string" ? part.text : "")).join("\n");
}

function finishTurn(status) {
  if (!pendingTurn) return;
  const turnId = pendingTurn.turnId;
  pendingTurn = null;
  send({
    jsonrpc: "2.0",
    method: "turn/completed",
    params: { turn: { id: turnId, status } },
  });
}

function beginTurn(msg) {
  const text = promptText(msg);
  const turnId = `turn-${turns.length + 1}`;
  pendingTurn = { id: msg.id, turnId, text };
  turns.push({ text, assistant: "" });
  send({ jsonrpc: "2.0", id: msg.id, result: { turn: { id: turnId } } });
  send({ jsonrpc: "2.0", method: "turn/started", params: { turn: { id: turnId } } });
  if (text.includes("PLAIN")) {
    send({
      jsonrpc: "2.0",
      method: "item/agentMessage/delta",
      params: { itemId: "m-plain", delta: "Noted" },
    });
    turns[turns.length - 1].assistant = "Noted";
    finishTurn("completed");
    return;
  }
  if (text.includes("HOLD")) {
    send({
      jsonrpc: "2.0",
      method: "item/agentMessage/delta",
      params: { itemId: "m-hold", delta: "Working" },
    });
    turns[turns.length - 1].assistant = "Working";
    return;
  }
  send({
    jsonrpc: "2.0",
    method: "item/agentMessage/delta",
    params: { itemId: "m1", delta: "Hello" },
  });
  send({
    jsonrpc: "2.0",
    method: "item/started",
    params: { item: { type: "commandExecution", id: "c1", command: "latexmk main.tex" } },
  });
  const approvalId = nextApproval;
  nextApproval += 1;
  pendingTurn.approvalId = approvalId;
  send({
    jsonrpc: "2.0",
    id: approvalId,
    method: "item/commandExecution/requestApproval",
    params: {
      command: "latexmk main.tex",
      threadId: "thread-1",
      turnId,
      itemId: "c1",
      kind: "command",
    },
  });
  turns[turns.length - 1].assistant = "Hello";
}

function onApprovalReply(msg) {
  if (!pendingTurn || msg.id !== pendingTurn.approvalId) return;
  const decision = msg.result?.decision ?? "";
  recordNote({ approvalDecision: decision, id: msg.id });
  send({
    jsonrpc: "2.0",
    method: "item/completed",
    params: {
      item: {
        type: "commandExecution",
        id: "c1",
        command: "latexmk main.tex",
        aggregatedOutput: decision === "decline" ? "" : "built",
        status: decision === "decline" ? "declined" : "completed",
      },
    },
  });
  finishTurn(decision === "decline" ? "completed" : "completed");
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
    onApprovalReply(msg);
    return;
  }
  if (!Object.prototype.hasOwnProperty.call(msg, "id")) return;
  if (msg.method === "initialize") {
    send({ jsonrpc: "2.0", id: msg.id, result: { userAgent: "pitex-verify-fake" } });
    return;
  }
  if (msg.method === "account/read") {
    if (auth === "signed-out") {
      send({ jsonrpc: "2.0", id: msg.id, result: { account: null, requiresOpenaiAuth: true } });
      return;
    }
    send({
      jsonrpc: "2.0",
      id: msg.id,
      result: {
        account: { type: "chatgpt", email: "verify@example.invalid", planType: "plus" },
        requiresOpenaiAuth: false,
        authToken: "must-not-leak",
      },
    });
    return;
  }
  if (msg.method === "model/list") {
    send({
      jsonrpc: "2.0",
      id: msg.id,
      result: {
        data: [
          {
            id: "gpt-test",
            model: "gpt-test",
            displayName: "GPT Test",
            hidden: false,
            isDefault: true,
            supportedReasoningEfforts: [{ reasoningEffort: "low", description: "Low" }],
            defaultReasoningEffort: "low",
          },
        ],
        nextCursor: null,
      },
    });
    return;
  }
  if (msg.method === "thread/start" || msg.method === "thread/resume") {
    send({
      jsonrpc: "2.0",
      id: msg.id,
      result: { thread: { id: "thread-1", reasoningEffort: "low" }, model: "gpt-test" },
    });
    return;
  }
  if (msg.method === "turn/start") {
    beginTurn(msg);
    return;
  }
  if (msg.method === "turn/interrupt") {
    send({ jsonrpc: "2.0", id: msg.id, result: {} });
    finishTurn("interrupted");
    return;
  }
  if (msg.method === "thread/read") {
    send({
      jsonrpc: "2.0",
      id: msg.id,
      result: {
        thread: {
          id: "thread-1",
          turns: turns.map((turn, index) => ({
            id: `turn-${index + 1}`,
            items: [
              { type: "userMessage", content: [{ type: "text", text: turn.text }] },
              { type: "agentMessage", id: `m-${index}`, text: turn.assistant },
            ],
          })),
        },
      },
    });
    return;
  }
  send({ jsonrpc: "2.0", id: msg.id, result: {} });
});
