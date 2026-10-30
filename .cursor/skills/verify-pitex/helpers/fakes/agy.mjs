#!/usr/bin/env node
import readline from "node:readline";
import { writeSync } from "node:fs";

import { recordLine, recordSpawn } from "./record.mjs";

recordSpawn();

const argv = process.argv.slice(2);
const send = (value) => writeSync(1, `${JSON.stringify(value)}\n`);

if (argv[0] === "models") {
  if (process.env.FAKE_AUTH === "signed-out") {
    writeSync(2, "authentication required\n");
    process.exit(1);
  }
  writeSync(1, "gemini-4-argon-high       Gemini 4 Argon (High)\ngemini-3.8-flash-medium       Gemini 3.8 Flash (Medium)\n");
  process.exit(0);
}

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
  recordLine(line);
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
  const text = typeof msg.message?.content === "string" ? msg.message.content : "";
  if (text.includes("HOLD")) {
    send({
      event: "step_update",
      step_update: { step_index: 1, step_type: "agent_response", state: "ACTIVE", text_delta: "Working" },
    });
    return;
  }
  const reply = text.includes("PLAIN") ? "Noted" : "Hello there\n";
  send({ event: "step_update", step_update: { step_index: 1, step_type: "agent_response", state: "ACTIVE", text_delta: reply } });
  if (!text.includes("PLAIN")) {
    send({
      event: "step_update",
      step_update: {
        step_index: 2,
        step_type: "tool",
        state: "DONE",
        tool_info: {
          name: "run_command",
          parameters: { CommandLine: "echo hello_headless_demo" },
          output: "hello_headless_demo\n",
        },
      },
    });
    writeSync(2, 'a tool required the "command" permission that headless mode cannot prompt for, so it was auto-denied. Add an allow rule.\n');
  }
  send({
    event: "result",
    conversation_id: conversation,
    result: {
      status: "SUCCESS",
      response: reply,
      usage: { input_tokens: 10, output_tokens: 4, thinking_tokens: 3, cache_read_tokens: 1, total_tokens: 14 },
    },
  });
});

process.on("SIGINT", () => {
  recordLine("SIGINT");
  send({ event: "result", result: { status: "INTERRUPTED", error: "interrupted", response: "" } });
  process.exit(0);
});
