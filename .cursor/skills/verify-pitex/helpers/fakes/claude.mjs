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
      input: { command: "latexmk main.tex", description: "Compile the document" },
      tool_use_id: "toolu_1",
      display_name: "Bash",
      description: "Compile the document",
    },
  });
}

// One Read with a line range (of intro.tex, which the window journey writes),
// one Edit with Claude Code's structuredPatch result, and one Bash call that
// exits 1: the typed tool cards in the panel.
function toolCards() {
  const root = process.env.FAKE_ROOT ?? process.cwd();
  const file = `${root}/main.tex`;
  const call = (id, name, input) =>
    send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "tool_use", id, name, input }] } });
  const result = (id, content, isError, toolUseResult) =>
    send({
      type: "user",
      session_id: session,
      message: { role: "user", content: [{ type: "tool_result", tool_use_id: id, content, is_error: isError }] },
      ...(toolUseResult ? { tool_use_result: toolUseResult } : {}),
    });
  call("toolu_read", "Read", { file_path: `${root}/intro.tex`, offset: 1, limit: 40 });
  result("toolu_read", "1\t\\documentclass{article}", false);
  call("toolu_edit", "Edit", { file_path: file, old_string: "Title", new_string: "A Better Title\n\\maketitle" });
  result("toolu_edit", "The file has been updated.", false, {
    filePath: file,
    structuredPatch: [
      {
        oldStart: 1,
        oldLines: 1,
        newStart: 1,
        newLines: 2,
        lines: [
          "-\\documentclass{article}\\begin{document}Title\\end{document}",
          "+\\documentclass{article}\\begin{document}A Better Title",
          "+\\maketitle\\end{document}",
        ],
      },
    ],
  });
  call("toolu_bash", "Bash", { command: "latexmk -pdf main.tex" });
  result("toolu_bash", "Exit code 1\n! LaTeX Error: File `missing.sty' not found.", true);
  streamText("Done");
  send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: "Done" }] } });
  finish("success", "Done");
}

// A Markdown answer streamed in two parts: the first ends inside an open
// ```latex fence (drawn as an open code box), the rest closes it 4 s later.
const MARKDOWN_ANSWER = [
  "## Plan for the intro\n\n1. Check `main.tex` for the title\n2. Add a **section** with *emphasis*\n   - nested point\n\n",
  "Inline math $E=mc^2$ sits in the text; prices like $5 and $10 stay plain.\n\n```latex\n\\section{Intro} % first section\n",
  "\\begin{equation}\n  a^2 + b^2 = c^2\n\\end{equation}\n```\n\n$$\\int_0^1 x\\,dx$$\n\n",
  "| Command | Purpose |\n|---|---|\n| `\\cite` | references |\n| `\\ref` | **labels** |\n\n> A short quote.\n\n",
  "**Bold with $x^2$ inside**, while echo $FOO$BAR and ![img](x.png) stay text.\n\n",
  "See [intro.tex:1](intro.tex:1) or [the docs](https://example.org).",
];

function markdown() {
  streamText(MARKDOWN_ANSWER[0]);
  streamText(MARKDOWN_ANSWER[1]);
  setTimeout(() => {
    for (const part of MARKDOWN_ANSWER.slice(2)) streamText(part);
    const text = MARKDOWN_ANSWER.join("");
    send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text }] } });
    finish("success", text);
  }, 4000);
}

// APPROVALS: AskUserQuestion (one single-choice and one multi-select question)
// and ExitPlanMode twice: the first is meant for Keep planning with feedback,
// the revised one for Approve plan. Each answer is noted in FAKE_RECORD.
const QUESTIONS = [
  {
    header: "Engine",
    question: "Which TeX engine should the build use?",
    options: [{ label: "pdfLaTeX", description: "Fast and classic" }, { label: "LuaLaTeX", description: "Unicode fonts" }],
    multiSelect: false,
  },
  { header: "Parts", question: "Which parts need work?", options: [{ label: "Intro" }, { label: "Method" }], multiSelect: true },
];
const PLANS = [
  "## Plan\n\n1. Rename the title in `main.tex`\n2. Add a `\\maketitle` line\n3. Build with **latexmk**",
  "## Revised plan\n\n1. Rename the title in `main.tex`\n2. Add an abstract after `\\maketitle`\n3. Build with **latexmk**",
];

function askTool(requestId, toolUseId, name, input) {
  send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "tool_use", id: toolUseId, name, input }] } });
  send({
    type: "control_request",
    request_id: requestId,
    request: { subtype: "can_use_tool", tool_name: name, input, tool_use_id: toolUseId, display_name: name },
  });
}

function approvalAnswer(requestId, response) {
  const answer = response ?? {};
  if (requestId === "perm-question") {
    recordNote({ question: answer.behavior, answers: answer.updatedInput?.answers ?? null });
    streamText("Thanks");
    finish("success", "Thanks");
    return true;
  }
  if (requestId === "perm-plan-1") {
    recordNote({ plan1: answer.behavior, message: answer.message ?? null });
    askTool("perm-plan-2", "toolu_plan_2", "ExitPlanMode", { plan: PLANS[1] });
    return true;
  }
  if (requestId === "perm-plan-2") {
    recordNote({ plan2: answer.behavior, updatedPermissions: answer.updatedPermissions ?? null });
    streamText("Plan approved");
    finish("success", "Plan approved");
    return true;
  }
  return false;
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
  if (msg.type === "control_request" && msg.request?.subtype !== "interrupt") {
    // set_permission_mode, set_model, ...: acknowledge so the SDK call returns.
    if (msg.request?.subtype === "set_permission_mode") recordNote({ setPermissionMode: msg.request.mode });
    send({ type: "control_response", response: { subtype: "success", request_id: msg.request_id, response: {} } });
    return;
  }
  if (msg.type === "control_response" && approvalAnswer(msg.response?.request_id, msg.response?.response)) return;
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
    if (text.includes("MARKDOWN")) {
      markdown();
      return;
    }
    if (text.includes("QUESTION")) {
      askTool("perm-question", "toolu_question", "AskUserQuestion", { questions: QUESTIONS });
      return;
    }
    if (text.includes("PLAN")) {
      streamText("Here is my plan.");
      askTool("perm-plan-1", "toolu_plan_1", "ExitPlanMode", { plan: PLANS[0] });
      return;
    }
    if (text.includes("TOOLCARDS")) {
      toolCards();
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
          input: { command: "touch pitex-ok", description: "Create a marker file" },
          tool_use_id: "toolu_touch",
          display_name: "Bash",
          description: "Create a marker file",
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
