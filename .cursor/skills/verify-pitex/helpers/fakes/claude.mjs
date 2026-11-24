#!/usr/bin/env node
import { randomUUID } from "node:crypto";
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import readline from "node:readline";

import { recordLine, recordNote, recordSpawn } from "./record.mjs";

recordSpawn();

const args = process.argv.slice(2);
if (args[0] === "auth" && args[1] === "status") {
  const loggedIn = process.env.FAKE_AUTH !== "signed-out";
  process.stdout.write(`${JSON.stringify({ loggedIn })}\n`);
  process.exit(0);
}

/** `--flag value` or `--flag=value` (the SDK passes the second form). */
function flag(name) {
  const joined = args.find((arg) => arg.startsWith(`${name}=`));
  if (joined) return joined.slice(name.length + 1);
  const at = args.indexOf(name);
  return at >= 0 ? args[at + 1] : undefined;
}
const resumeAt = args.indexOf("--resume");
// FAKE_PERSIST=1 (the claude-branch journey): a UUID session saved like Claude Code's own
// transcript, so the SDK's getSessionMessages and forkSession read it, plus file checkpoints.
const persist = process.env.FAKE_PERSIST === "1";
const session = flag("--resume") ?? (resumeAt >= 0 && args[resumeAt + 1] ? args[resumeAt + 1] : persist ? randomUUID() : "sess-claude-1");
const projectDir = join(process.env.CLAUDE_CONFIG_DIR || join(process.env.HOME || ".", ".claude"), "projects", process.cwd().replace(/[^A-Za-z0-9]/g, "-"));
const sessionFile = join(projectDir, `${session}.jsonl`);
const backupFile = join(projectDir, "fake-backups.json");
let lastUuid = flag("--resume-session-at") ?? lastSavedUuid();
let currentPrompt = null;

function lastSavedUuid() {
  if (!persist || !existsSync(sessionFile)) return null;
  const rows = readFileSync(sessionFile, "utf8").split("\n").filter(Boolean).map((line) => JSON.parse(line));
  return rows.findLast((row) => row.uuid)?.uuid ?? null;
}

/** One transcript row in Claude Code's format (main chain only). */
function save(type, message) {
  if (!persist) return null;
  mkdirSync(projectDir, { recursive: true });
  const uuid = randomUUID();
  appendFileSync(sessionFile, `${JSON.stringify({ type, uuid, parentUuid: lastUuid, isSidechain: false, sessionId: session, timestamp: new Date().toISOString(), cwd: process.cwd(), userType: "external", message })}\n`);
  lastUuid = uuid;
  return uuid;
}

const send = (value) => {
  if ((value.type === "assistant" || value.type === "user") && !value.parent_tool_use_id) save(value.type, value.message);
  process.stdout.write(`${JSON.stringify(value)}\n`);
};

function backups() {
  return existsSync(backupFile) ? JSON.parse(readFileSync(backupFile, "utf8")) : [];
}

/** rewind_files: files edited at or after that prompt go back to their content before it. */
function rewindFiles(msg) {
  const order = existsSync(sessionFile)
    ? readFileSync(sessionFile, "utf8").split("\n").filter(Boolean).map((line) => JSON.parse(line)).filter((row) => row.type === "user" && typeof row.message?.content === "string").map((row) => row.uuid)
    : [];
  const from = order.indexOf(msg.request.user_message_id);
  if (from < 0) return { canRewind: false, error: "No file checkpoint found for this message." };
  const restore = new Map();
  for (const row of backups()) if (order.indexOf(row.prompt) >= from && !restore.has(row.path)) restore.set(row.path, row.before);
  if (!msg.request.dry_run) {
    for (const [path, before] of restore) writeFileSync(path, before);
    recordNote({ rewindFiles: [...restore.keys()] });
  }
  return { canRewind: true, filesChanged: [...restore.keys()] };
}
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });

let holding = false;
// SLOW turn in progress: like the real CLI, later user messages wait and each
// gets its own turn (and its own result) after it.
let slowTimer = null;
const waiting = [];

function imageBlocks(msg) {
  const content = msg.message?.content;
  return Array.isArray(content) ? content.filter((part) => part?.type === "image") : [];
}

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
        commands: [
          { name: "clear", description: "Clear conversation history and free up context", argumentHint: "" },
          { name: "model", description: "Set the AI model for Claude Code", argumentHint: "[model]" },
          { name: "compact", description: "Clear conversation history but keep a summary in context", argumentHint: "<optional custom summarization instructions>" },
          { name: "context", description: "Visualize current context usage", argumentHint: "" },
          { name: "cost", description: "Show the total cost and duration of the current session", argumentHint: "" },
        ],
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

// SUBAGENT: a short reply, then an Explore subagent (Agent tool) whose Glob and Read
// arrive 1.5 s apart with parent_tool_use_id, its report, and the final answer.
function subagent() {
  const root = process.env.FAKE_ROOT ?? process.cwd();
  const agent = "toolu_agent";
  const child = (id, name, input) =>
    send({ type: "assistant", session_id: session, parent_tool_use_id: agent, message: { role: "assistant", content: [{ type: "tool_use", id, name, input }] } });
  const result = (id, content, parent) =>
    send({ type: "user", session_id: session, parent_tool_use_id: parent, message: { role: "user", content: [{ type: "tool_result", tool_use_id: id, content }] } });
  streamText("I will ask a subagent to look.");
  send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: "I will ask a subagent to look." }] } });
  send({
    type: "assistant",
    session_id: session,
    message: { role: "assistant", content: [{ type: "tool_use", id: agent, name: "Agent", input: { description: "Find the document title", subagent_type: "Explore", prompt: "Find the title of main.tex" } }] },
  });
  setTimeout(() => {
    child("toolu_glob", "Glob", { pattern: "**/*.tex" });
    result("toolu_glob", `${root}/main.tex`, agent);
  }, 1500);
  setTimeout(() => {
    child("toolu_sub_read", "Read", { file_path: `${root}/main.tex` });
    result("toolu_sub_read", "1\t\\documentclass{article}\\begin{document}Title\\end{document}", agent);
  }, 3000);
  setTimeout(() => {
    result(agent, "The document's title is \"Title\" (main.tex, line 1).", null);
    const reply = "The title of main.tex is **Title**.";
    send({ type: "stream_event", session_id: session, event: { type: "content_block_delta", index: 1, delta: { type: "text_delta", text: reply } } });
    send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: reply }] } });
    finish("success", reply);
  }, Number(process.env.FAKE_SUBAGENT_MS || 6000));
}

// EDITFILE: an Edit of main.tex written to disk with a checkpoint (FAKE_PERSIST), then an answer.
function editFile() {
  const root = process.env.FAKE_ROOT ?? process.cwd();
  const path = `${root}/main.tex`;
  const before = readFileSync(path, "utf8");
  const after = before.replace("Title", "Title\\section{Added by Claude}");
  if (persist) writeFileSync(backupFile, JSON.stringify([...backups(), { prompt: currentPrompt, path, before }]));
  writeFileSync(path, after);
  send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "tool_use", id: "toolu_editfile", name: "Edit", input: { file_path: path, old_string: "Title", new_string: "Title\\section{Added by Claude}" } }] } });
  send({ type: "user", session_id: session, message: { role: "user", content: [{ type: "tool_result", tool_use_id: "toolu_editfile", content: "The file has been updated." }] } });
  const reply = "I added a section to main.tex.";
  streamText(reply);
  send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: reply }] } });
  finish("success", reply);
}

// TODOS: three TodoWrite snapshots 1.5 s apart (all pending, the first in
// progress, two completed and the last in progress), then a short answer.
const TODO_STEPS = [
  ["Build the PDF", "Building the PDF"],
  ["Fix the citation", "Fixing the citation"],
  ["Check the log", "Checking the log"],
];
const TODO_SNAPSHOTS = [
  ["pending", "pending", "pending"],
  ["in_progress", "pending", "pending"],
  ["completed", "completed", "in_progress"],
];

function todos() {
  TODO_SNAPSHOTS.forEach((statuses, index) => {
    setTimeout(() => {
      const id = `toolu_todo_${index}`;
      const input = { todos: TODO_STEPS.map(([content, activeForm], row) => ({ content, activeForm, status: statuses[row] })) };
      send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "tool_use", id, name: "TodoWrite", input }] } });
      send({
        type: "user",
        session_id: session,
        message: { role: "user", content: [{ type: "tool_result", tool_use_id: id, content: "Todos have been modified successfully." }] },
      });
      if (index < TODO_SNAPSHOTS.length - 1) return;
      setTimeout(() => {
        streamText("Built the PDF and fixed the citation; checking the log next.");
        send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: "Built the PDF and fixed the citation; checking the log next." }] } });
        finish("success", "Built the PDF and fixed the citation; checking the log next.");
      }, 500);
    }, 1500 * index);
  });
}

// LONGTODOS: one 24-item TodoWrite (three done, the fourth in progress), held
// 4 s while the turn runs so the expanded checklist can be seen scrolling.
function longTodos() {
  const id = "toolu_todo_long";
  const todos = Array.from({ length: 24 }, (_, row) => ({
    content: `Step ${row + 1}: check section ${row + 1}`,
    activeForm: `Checking section ${row + 1}`,
    status: row < 3 ? "completed" : row === 3 ? "in_progress" : "pending",
  }));
  send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "tool_use", id, name: "TodoWrite", input: { todos } }] } });
  send({ type: "user", session_id: session, message: { role: "user", content: [{ type: "tool_result", tool_use_id: id, content: "Todos have been modified successfully." }] } });
  setTimeout(() => {
    streamText("Checked three sections.");
    send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: "Checked three sections." }] } });
    finish("success", "Checked three sections.");
  }, 4000);
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

// USAGE: thinking, text, a Read, more text and a long streamed answer, with usage that
// crosses 70 % and then 90 % of a 200k window, ending in a rate-limit error. The
// second USAGE (Retry) answers normally. Timings leave room for screenshots.
let usageRuns = 0;
const at = (ms, fn) => setTimeout(fn, ms);
const streamEvent = (event) => send({ type: "stream_event", session_id: session, event });
const apiMessage = (contextTokens) =>
  streamEvent({
    type: "message_start",
    message: { model: "claude-test", usage: { input_tokens: 400, cache_creation_input_tokens: 600, cache_read_input_tokens: contextTokens - 1000, output_tokens: 1 } },
  });
const delta = (index, delta) => streamEvent({ type: "content_block_delta", index, delta });

function usage() {
  usageRuns += 1;
  if (usageRuns > 1) {
    apiMessage(60_000);
    delta(0, { type: "text_delta", text: "Retried fine: the citation key is now defined." });
    finishUsage("success", "Retried fine: the citation key is now defined.");
    return;
  }
  apiMessage(60_000);
  delta(0, { type: "thinking_delta", thinking: "The user wants the bibliography fixed. " });
  at(1200, () => delta(0, { type: "thinking_delta", thinking: "Read the log first, then the .bib file." }));
  at(3500, () => {
    delta(1, { type: "text_delta", text: "I will read the build log first." });
    streamEvent({ type: "message_delta", usage: { output_tokens: 300 } });
    send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "tool_use", id: "toolu_log", name: "Read", input: { file_path: "main.log" } }] } });
    send({ type: "user", session_id: session, message: { role: "user", content: [{ type: "tool_result", tool_use_id: "toolu_log", content: "Citation `knuth84' undefined." }] } });
    apiMessage(145_000);
    delta(0, { type: "text_delta", text: "The log says the citation `knuth84` is undefined." });
  });
  at(7000, () => {
    apiMessage(185_000);
    delta(0, { type: "text_delta", text: "\n\nHere is what I checked, step by step:\n\n" });
  });
  for (let line = 1; line <= 24; line += 1) {
    at(7000 + line * 450, () => delta(0, { type: "text_delta", text: `${line}. Checked entry ${line} of refs.bib against the log.\n` }));
  }
  at(19_500, () => {
    send({
      type: "assistant",
      session_id: session,
      error: "rate_limit",
      message: { model: "<synthetic>", role: "assistant", content: [{ type: "text", text: 'API Error: 429 {"type":"error","error":{"type":"rate_limit_error","message":"Number of requests has exceeded your rate limit."}}' }] },
    });
    finishUsage("error_during_execution", 'API Error: 429 {"type":"error","error":{"type":"rate_limit_error"}}');
  });
}

function finishUsage(subtype, result) {
  send({
    type: "result",
    subtype,
    is_error: subtype !== "success",
    session_id: session,
    result,
    // The SDK reads `errors` on every error result, as the real CLI sends it.
    ...(subtype === "success" ? {} : { errors: [result] }),
    total_cost_usd: 0.0731,
    usage: { input_tokens: 1200, output_tokens: 900 },
    modelUsage: { "claude-test": { outputTokens: 900, contextWindow: 200_000 } },
  });
}

function finish(subtype, result) {
  send({
    type: "result",
    subtype,
    is_error: subtype !== "success",
    session_id: session,
    result,
    ...(subtype === "success" ? {} : { errors: [result] }),
    usage: { input_tokens: 3, output_tokens: 2 },
    total_cost_usd: 0,
    // Like the real CLI, the model's window arrives only with the result.
    modelUsage: { "claude-test": { outputTokens: 2, contextWindow: 200_000 } },
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
    if (slowTimer) clearTimeout(slowTimer);
    slowTimer = null;
    waiting.length = 0;
    finish("error", "interrupted");
    return;
  }
  if (msg.type === "control_request" && msg.request?.subtype === "rewind_files") {
    const answer = () => send({ type: "control_response", response: { subtype: "success", request_id: msg.request_id, response: rewindFiles(msg) } });
    // FAKE_REWIND_DELAY_MS holds the real (not dry-run) restore, so a drive can see the rewind in flight.
    const delay = msg.request.dry_run ? 0 : Number(process.env.FAKE_REWIND_DELAY_MS || 0);
    if (delay > 0) setTimeout(answer, delay);
    else answer();
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
    currentPrompt = save("user", { role: "user", content: text });
    const images = imageBlocks(msg);
    recordNote({ user: text.split("\n").at(-1), images: images.map((part) => part.source?.media_type) });
    if (slowTimer) {
      waiting.push(text.split("\n").at(-1));
      return;
    }
    if (text.includes("SLOW")) {
      streamText("Working slowly");
      slowTimer = setTimeout(() => {
        slowTimer = null;
        send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: "Done slowly" }] } });
        finish("success", "Done slowly");
        for (const later of waiting.splice(0)) {
          const reply = `Late turn for: ${later}`;
          streamText(reply);
          send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: reply }] } });
          finish("success", reply);
        }
      }, Number(process.env.FAKE_SLOW_MS || 8000));
      return;
    }
    if (images.length > 0) {
      const reply = `I see ${images.length} image(s): ${images.map((part) => part.source?.media_type).join(", ")}`;
      streamText(reply);
      send({ type: "assistant", session_id: session, message: { role: "assistant", content: [{ type: "text", text: reply }] } });
      finish("success", reply);
      return;
    }
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
    if (text.includes("USAGE")) {
      usage();
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
    if (text.includes("LONGTODOS")) {
      longTodos();
      return;
    }
    if (text.includes("TODOS")) {
      todos();
      return;
    }
    if (text.includes("TOOLCARDS")) {
      toolCards();
      return;
    }
    if (text.includes("SUBAGENT")) {
      subagent();
      return;
    }
    if (text.includes("EDITFILE")) {
      editFile();
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
