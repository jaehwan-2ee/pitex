// Todo list (`todo_update`) mapping for Claude Code, ACP plans and Codex.
// Run through test.ts: node --test Mac/Resources/PitexAgent/bridge/test.ts

import assert from "node:assert/strict";
import test from "node:test";

import { closeTurn, createHistory, createStreamState, historyMessages, mapSessionUpdate, recordSessionUpdate } from "./acp-map.ts";
import { createClaudeStreamState, mapClaudeMessage, messagesFromState, type Json } from "./claude-map.ts";
import { createStreamState as createCodexState, mapNotification } from "./codex-map.ts";
import { foldClaudeTodo, withoutTodoTools } from "./todos.ts";

function tool(id: string, name: string, input: Json): Json {
  return { type: "assistant", message: { content: [{ type: "tool_use", id, name, input }] } };
}

function result(id: string, text: string, isError = false, structured?: Json): Json {
  return {
    type: "user",
    message: { content: [{ type: "tool_result", tool_use_id: id, content: text, is_error: isError }] },
    ...(structured ? { tool_use_result: structured } : {}),
  };
}

function todoEvents(events: Json[]): unknown[] {
  return events.filter((event) => event.type === "todo_update").map((event) => event.items);
}

test("Claude TodoWrite becomes todo_update snapshots and its rows leave the transcript", () => {
  const state = createClaudeStreamState();
  const pending = [
    { content: "Build the PDF", status: "pending", activeForm: "Building the PDF" },
    { content: "Fix the citation", status: "pending", activeForm: "Fixing the citation" },
  ];
  const events = [
    ...mapClaudeMessage(tool("t1", "TodoWrite", { todos: pending }), state),
    ...mapClaudeMessage(result("t1", "ok"), state),
    ...mapClaudeMessage(tool("t2", "TodoWrite", { todos: [{ ...pending[0], status: "in_progress" }, pending[1]] }), state),
    ...mapClaudeMessage(result("t2", "ok"), state),
    // The same list again changes nothing.
    ...mapClaudeMessage(tool("t3", "TodoWrite", { todos: [{ ...pending[0], status: "in_progress" }, pending[1]] }), state),
    ...mapClaudeMessage(result("t3", "ok"), state),
  ];
  assert.deepEqual(todoEvents(events), [
    [
      { text: "Build the PDF", status: "pending", activeForm: "Building the PDF" },
      { text: "Fix the citation", status: "pending", activeForm: "Fixing the citation" },
    ],
    [
      { text: "Build the PDF", status: "in_progress", activeForm: "Building the PDF" },
      { text: "Fix the citation", status: "pending", activeForm: "Fixing the citation" },
    ],
  ]);
  assert.equal(events.some((event) => String(event.type).startsWith("tool_execution")), false);
  assert.equal(JSON.stringify(withoutTodoTools(messagesFromState(state))).includes("TodoWrite"), false);
});

test("a failed Claude todo call keeps its error card", () => {
  const state = createClaudeStreamState();
  const events = [
    ...mapClaudeMessage(tool("c1", "TaskCreate", { subject: "Build" }), state),
    ...mapClaudeMessage(result("c1", "Task store is locked", true), state),
  ];
  const end = events.find((event) => event.type === "tool_execution_end");
  assert.equal(end?.isError, true);
  assert.equal(end?.status, "failed");
  assert.ok(events.some((event) => event.type === "tool_execution_start" && event.toolName === "TaskCreate"));
  assert.deepEqual(todoEvents(events), []);
  const kept = withoutTodoTools(messagesFromState(state));
  assert.deepEqual(kept.map((message) => message.role), ["assistant", "toolResult"]);
});

test("a rejected TodoWrite list never reaches the checklist", () => {
  const state = createClaudeStreamState();
  const events = [
    ...mapClaudeMessage(tool("w1", "TodoWrite", { todos: [{ content: "A", status: "pending" }] }), state),
    ...mapClaudeMessage(result("w1", "InputValidationError: activeForm is required", true), state),
  ];
  assert.deepEqual(todoEvents(events), []);
  assert.equal(state.todos, null);
  assert.equal(events.find((event) => event.type === "tool_execution_end")?.status, "failed");
});

test("TaskCreate results that share one message fold from their text", () => {
  const state = createClaudeStreamState();
  const both: Json = {
    type: "user",
    message: {
      content: [
        { type: "tool_result", tool_use_id: "x", content: "Task #1 created successfully: Build" },
        { type: "tool_result", tool_use_id: "y", content: "Task #2 created successfully: Check" },
      ],
    },
  };
  const events = [
    ...mapClaudeMessage({ type: "assistant", message: { content: [
      { type: "tool_use", id: "x", name: "TaskCreate", input: { subject: "Build" } },
      { type: "tool_use", id: "y", name: "TaskCreate", input: { subject: "Check" } },
    ] } }, state),
    ...mapClaudeMessage(both, state),
  ];
  assert.deepEqual(todoEvents(events).at(-1), [
    { id: "1", text: "Build", status: "pending" },
    { id: "2", text: "Check", status: "pending" },
  ]);
  assert.equal(events.some((event) => String(event.type).startsWith("tool_execution")), false);
});

test("Claude TaskCreate, TaskUpdate and TaskList fold into one list", () => {
  const state = createClaudeStreamState();
  const events = [
    ...mapClaudeMessage(tool("a", "TaskCreate", { subject: "Build the PDF", activeForm: "Building the PDF" }), state),
    ...mapClaudeMessage(result("a", "Task #1 created", false, { task: { id: "1", subject: "Build the PDF" } }), state),
    ...mapClaudeMessage(tool("b", "TaskCreate", { subject: "Fix the citation" }), state),
    ...mapClaudeMessage(result("b", "Task #2 created", false, { task: { id: "2", subject: "Fix the citation" } }), state),
    ...mapClaudeMessage(tool("c", "TaskUpdate", { taskId: "1", status: "in_progress" }), state),
    ...mapClaudeMessage(result("c", "Updated", false, { success: true, taskId: "1" }), state),
    ...mapClaudeMessage(tool("d", "TaskUpdate", { taskId: "2", status: "deleted" }), state),
    ...mapClaudeMessage(result("d", "Deleted", false, { success: true, taskId: "2" }), state),
  ];
  const snapshots = todoEvents(events);
  assert.equal(snapshots.length, 4);
  assert.deepEqual(snapshots[2], [
    { id: "1", text: "Build the PDF", status: "in_progress", activeForm: "Building the PDF" },
    { id: "2", text: "Fix the citation", status: "pending" },
  ]);
  assert.deepEqual(snapshots[3], [{ id: "1", text: "Build the PDF", status: "in_progress", activeForm: "Building the PDF" }]);
  assert.deepEqual(
    foldClaudeTodo(state.todos, "TaskList", {}, { tasks: [{ id: "7", subject: "Only", status: "completed" }] }),
    [{ id: "7", text: "Only", status: "completed" }]
  );
  // An update for a task the list does not know changes nothing.
  assert.equal(foldClaudeTodo(state.todos, "TaskUpdate", { taskId: "9", status: "completed" }, null), null);
  assert.equal(events.some((event) => String(event.type).startsWith("tool_execution")), false);
});

test("an ACP plan becomes todo_update, not a plan tool card, live and in history", () => {
  const state = createStreamState();
  const entries = [
    { content: "Read main.tex", status: "completed", priority: "high" },
    { content: "Fix the citation", status: "in_progress", priority: "medium" },
    { content: "Rebuild", status: "pending", priority: "low" },
  ];
  const plan = { sessionUpdate: "plan", entries };
  const events = [...mapSessionUpdate(plan, state), ...closeTurn(state, "end_turn")];
  const items = [
    { text: "Read main.tex", status: "completed" },
    { text: "Fix the citation", status: "in_progress" },
    { text: "Rebuild", status: "pending" },
  ];
  assert.deepEqual(todoEvents(events), [items]);
  assert.equal(events.some((event) => String(event.type).startsWith("tool_execution")), false);

  const history = createHistory();
  recordSessionUpdate(history, { sessionUpdate: "user_message_chunk", content: { type: "text", text: "Fix it" } });
  recordSessionUpdate(history, plan);
  recordSessionUpdate(history, { sessionUpdate: "agent_message_chunk", content: { type: "text", text: "Done" } });
  assert.deepEqual(history.todos, items);
  assert.equal(JSON.stringify(historyMessages(history)).includes("acp-plan"), false);
  assert.deepEqual(historyMessages(history).map((message) => message.role), ["user", "assistant"]);
});

test("a Codex turn/plan/updated becomes todo_update", () => {
  const state = createCodexState();
  const events = mapNotification(
    "turn/plan/updated",
    {
      threadId: "th",
      turnId: "tu",
      explanation: null,
      plan: [
        { step: "Read main.tex", status: "completed" },
        { step: "Fix the citation", status: "inProgress" },
        { step: "Rebuild", status: "pending" },
      ],
    },
    state
  );
  assert.deepEqual(todoEvents(events), [
    [
      { text: "Read main.tex", status: "completed" },
      { text: "Fix the citation", status: "in_progress" },
      { text: "Rebuild", status: "pending" },
    ],
  ]);
});
