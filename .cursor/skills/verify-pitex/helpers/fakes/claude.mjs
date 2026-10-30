#!/usr/bin/env node
import readline from "node:readline";

import { recordLine, recordNote, recordSpawn } from "./record.mjs";

recordSpawn();

const args = process.argv.slice(2);
if (args[0] === "auth" && args[1] === "status") {
  const loggedIn = process.env.FAKE_AUTH !== "signed-out";
  process.stdout.write(`${JSON.stringify({ loggedIn })}\n`);
  process.exit(0);
}

const resumeAt = args.indexOf("--resume");
const session = resumeAt >= 0 && args[resumeAt + 1] ? args[resumeAt + 1] : "sess-claude-1";
const send = (value) => process.stdout.write(`${JSON.stringify(value)}\n`);
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

let holding = false;

function userText(msg) {
  const content = msg.message?.content ?? msg.message?.message?.content;
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content.map((part) => (typeof part?.text === "string" ? part.text : "")).join("\n");
  }
  return "";
}

function answerInitialize(msg) {
  send({
    type: "control_response",
    response: {
      subtype: "success",
      request_id: msg.request_id,
      response: {
        models: [
          {
            value: "claude-test",
            displayName: "Claude Test",
            supportedEffortLevels: ["low", "medium", "high"],
          },
        ],
        commands: [],
        agents: [],
      },
    },
  });
  send({
    type: "system",
    subtype: "init",
    session_id: session,
    model: "claude-test",
    permissionMode: "default",
  });
}

function streamText(text) {
  send({
    type: "stream_event",
    session_id: session,
    event: { type: "content_block_delta", index: 0, delta: { type: "text_delta", text } },
  });
}

function askBash() {
  send({
    type: "assistant",
    session_id: session,
    message: {
      role: "assistant",
      content: [
        { type: "text", text: "Hello" },
        { type: "tool_use", id: "toolu_1", name: "Bash", input: { command: "latexmk main.tex" } },
      ],
    },
  });
  send({
    type: "control_request",
    request_id: "perm-1",
    request: {
      subtype: "can_use_tool",
      tool_name: "Bash",
      input: { command: "latexmk main.tex" },
      tool_use_id: "toolu_1",
    },
  });
}

function finish(subtype, result) {
  send({
    type: "result",
    subtype,
    is_error: subtype !== "success",
    session_id: session,
    result,
    usage: { input_tokens: 3, output_tokens: 2 },
    total_cost_usd: 0,
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
  if (msg.type === "control_request" && msg.request?.subtype === "initialize") {
    answerInitialize(msg);
    return;
  }
  if (msg.type === "control_request" && msg.request?.subtype === "interrupt") {
    send({
      type: "control_response",
      response: { subtype: "success", request_id: msg.request_id, response: {} },
    });
    holding = false;
    finish("error", "interrupted");
    return;
  }
  if (msg.type === "control_response") {
    const behavior = msg.response?.response?.behavior ?? msg.response?.behavior;
    recordNote({ permissionBehavior: behavior ?? "unknown" });
    if (behavior === "allow") {
      send({
        type: "user",
        session_id: session,
        message: { role: "user", content: [{ type: "tool_result", tool_use_id: "toolu_1", content: "built" }] },
      });
    }
    finish("success", "Hello");
    return;
  }
  if (msg.type === "user" || msg.message) {
    const text = userText(msg);
    if (text.includes("HOLD")) {
      holding = true;
      streamText("Working");
      return;
    }
    if (text.includes("PLAIN")) {
      streamText("Noted");
      send({
        type: "assistant",
        session_id: session,
        message: { role: "assistant", content: [{ type: "text", text: "Noted" }] },
      });
      finish("success", "Noted");
      return;
    }
    if (text.includes("READONLY")) {
      streamText("pitex-ok");
      send({
        type: "assistant",
        session_id: session,
        message: { role: "assistant", content: [{ type: "text", text: "pitex-ok" }] },
      });
      finish("success", "pitex-ok");
      return;
    }
    if (text.includes("TOUCH")) {
      send({
        type: "assistant",
        session_id: session,
        message: {
          role: "assistant",
          content: [
            { type: "text", text: "Hello" },
            { type: "tool_use", id: "toolu_touch", name: "Bash", input: { command: "touch pitex-ok" } },
          ],
        },
      });
      send({
        type: "control_request",
        request_id: "perm-touch",
        request: {
          subtype: "can_use_tool",
          tool_name: "Bash",
          input: { command: "touch pitex-ok" },
          tool_use_id: "toolu_touch",
        },
      });
      return;
    }
    streamText("Hello");
    askBash();
  }
});

process.on("SIGTERM", () => {
  if (holding) finish("error", "interrupted");
  process.exit(0);
});
