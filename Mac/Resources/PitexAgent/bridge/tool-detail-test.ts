// Typed tool detail: the diff function and the per-backend mappings.

import assert from "node:assert/strict";
import test from "node:test";

import { closeTurn, createStreamState, mapSessionUpdate } from "./acp-map.ts";
import { cancelOpenTools, createClaudeStreamState, mapClaudeMessage } from "./claude-map.ts";
import { createStreamState as createCodexState, mapNotification } from "./codex-map.ts";
import { DIFF_CAP, DIFF_TOO_LARGE, unifiedDiff } from "./tool-detail.ts";

type Json = Record<string, unknown>;

test("unifiedDiff: identical, insert, delete, replace, too large", () => {
  assert.equal(unifiedDiff("a\nb\n", "a\nb\n"), "");
  assert.equal(unifiedDiff("a\nb\n", "a\nx\nb\n"), "@@ -1,2 +1,3 @@\n a\n+x\n b");
  assert.equal(unifiedDiff("a\nb\nc\n", "a\nc\n"), "@@ -1,3 +1,2 @@\n a\n-b\n c");
  assert.equal(unifiedDiff("a\nb\nc\n", "a\nB\nc\n"), "@@ -1,3 +1,3 @@\n a\n-b\n+B\n c");
  assert.equal(unifiedDiff("", "hi\n"), "@@ -0,0 +1,1 @@\n+hi");
  assert.equal(unifiedDiff("hi\n", ""), "@@ -1,1 +0,0 @@\n-hi");

  // Two changes far apart give two hunks with three context lines each.
  const old = Array.from({ length: 20 }, (_, i) => `l${i + 1}`).join("\n");
  const next = old.replace("l2\n", "L2\n").replace("l18", "L18");
  const hunks = unifiedDiff(old, next).split("\n").filter((line) => line.startsWith("@@"));
  assert.deepEqual(hunks, ["@@ -1,5 +1,5 @@", "@@ -15,6 +15,6 @@"]);

  const big = (tag: string) => Array.from({ length: 2_100 }, (_, i) => `${tag}${i}`).join("\n");
  assert.equal(unifiedDiff(big("a"), big("b")), DIFF_TOO_LARGE);

  const long = unifiedDiff("", Array.from({ length: 2_000 }, (_, i) => `line ${i}`).join("\n"));
  assert.ok(long.length <= DIFF_CAP + 40);
  assert.ok(long.endsWith("\\ diff truncated"));

  // A huge one-hunk change neither overflows the stack nor builds past the cap.
  const huge = unifiedDiff("", "l\n".repeat(200_000));
  assert.ok(huge.length <= DIFF_CAP + 40 && huge.endsWith("\\ diff truncated"));
  const write = mapClaudeMessage(sdkAssistantTool("big", "Write", { file_path: "/p/x", content: "l\n".repeat(200_000) }), createClaudeStreamState());
  assert.ok(write.some((event) => event.type === "tool_execution_start"));
});

function sdkAssistantTool(id: string, name: string, input: Json): Json {
  return { type: "assistant", message: { content: [{ type: "tool_use", id, name, input }] } };
}

function sdkToolResult(id: string, text: string, isError: boolean, toolUseResult?: Json): Json {
  return {
    type: "user",
    message: { content: [{ type: "tool_result", tool_use_id: id, content: text, is_error: isError }] },
    ...(toolUseResult ? { tool_use_result: toolUseResult } : {}),
  };
}

test("Claude Edit carries old/new text and a diff; the result patch has real line numbers", () => {
  const state = createClaudeStreamState();
  const start = mapClaudeMessage(
    sdkAssistantTool("e1", "Edit", { file_path: "/p/intro.tex", old_string: "Hello\n", new_string: "Hi\nthere\n" }),
    state
  ).find((event) => event.type === "tool_execution_start") as Json;
  assert.equal(start.status, "running");
  assert.equal(typeof start.turnId, "string");
  assert.deepEqual(start.detail, {
    kind: "edit",
    path: "/p/intro.tex",
    oldText: "Hello\n",
    newText: "Hi\nthere\n",
    diff: "@@ -1,1 +1,2 @@\n-Hello\n+Hi\n+there",
  });
  const end = mapClaudeMessage(
    sdkToolResult("e1", "The file has been updated.", false, {
      structuredPatch: [{ oldStart: 10, oldLines: 1, newStart: 10, newLines: 2, lines: ["-Hello", "+Hi", "+there"] }],
    }),
    state
  ).find((event) => event.type === "tool_execution_end") as Json;
  assert.equal(end.status, "completed");
  assert.equal(end.turnId, start.turnId);
  assert.equal((end.detail as Json).diff, "@@ -10,1 +10,2 @@\n-Hello\n+Hi\n+there");
});

test("Claude Bash with a nonzero exit is a failed shell card with the exit code", () => {
  const state = createClaudeStreamState();
  mapClaudeMessage(sdkAssistantTool("b1", "Bash", { command: "latexmk -pdf main.tex" }), state);
  const end = mapClaudeMessage(sdkToolResult("b1", "Exit code 12\nLaTeX failed", true), state).find(
    (event) => event.type === "tool_execution_end"
  ) as Json;
  assert.equal(end.status, "failed");
  assert.equal(end.isError, true);
  assert.deepEqual(end.detail, { kind: "shell", command: "latexmk -pdf main.tex", exitCode: 12 });
});

test("Claude Read, Grep, WebFetch and Task map to their kinds; TodoWrite becomes a todo list", () => {
  const state = createClaudeStreamState();
  const events: Json[] = [];
  events.push(...mapClaudeMessage(sdkAssistantTool("r", "Read", { file_path: "/p/main.tex", offset: 10, limit: 31 }), state));
  events.push(...mapClaudeMessage(sdkAssistantTool("g", "Grep", { pattern: "\\cite" }), state));
  events.push(...mapClaudeMessage(sdkAssistantTool("f", "WebFetch", { url: "https://ctan.org/pkg/x", prompt: "p" }), state));
  events.push(...mapClaudeMessage(sdkAssistantTool("t", "Task", { description: "Check refs", prompt: "p" }), state));
  events.push(...mapClaudeMessage(sdkAssistantTool("w", "TodoWrite", { todos: [] }), state));
  events.push(...mapClaudeMessage(sdkToolResult("w", "ok", false), state));
  const byId = Object.fromEntries(
    events.filter((event) => event.type === "tool_execution_start").map((event) => [event.toolCallId, event.detail])
  );
  assert.deepEqual(byId.r, { kind: "read", path: "/p/main.tex", startLine: 10, endLine: 40 });
  assert.deepEqual(byId.g, { kind: "search", query: "\\cite", label: "Grep" });
  assert.deepEqual(byId.f, { kind: "fetch", url: "https://ctan.org/pkg/x" });
  assert.deepEqual(byId.t, { kind: "sub_agent", label: "Check refs" });
  assert.equal(byId.w, undefined);
  assert.deepEqual(events.find((event) => event.type === "todo_update")?.items, []);
  // files_with_matches mode reports only numFiles: that is not a match count.
  mapClaudeMessage(sdkAssistantTool("g2", "Grep", { pattern: "x" }), state);
  const filesOnly = mapClaudeMessage(sdkToolResult("g2", "a.tex", false, { numFiles: 2 }), state).find(
    (event) => event.type === "tool_execution_end"
  ) as Json;
  assert.equal((filesOnly.detail as Json).numMatches, undefined);
  const grepEnd = mapClaudeMessage(sdkToolResult("g", "a.tex\nb.tex", false, { numFiles: 2, numMatches: 12 }), state).find(
    (event) => event.type === "tool_execution_end"
  ) as Json;
  assert.equal((grepEnd.detail as Json).numMatches, 12);
});

test("Claude abort cancels every open tool call once", () => {
  const state = createClaudeStreamState();
  mapClaudeMessage(sdkAssistantTool("a", "Bash", { command: "sleep 10" }), state);
  mapClaudeMessage(sdkAssistantTool("b", "Read", { file_path: "/p/x.tex" }), state);
  mapClaudeMessage(sdkToolResult("b", "text", false), state);
  const canceled = cancelOpenTools(state);
  assert.deepEqual(
    canceled.map((event) => [event.type, event.toolCallId, event.status]),
    [["tool_execution_end", "a", "canceled"]]
  );
  // The interrupt's late result for the same call is ignored.
  assert.deepEqual(mapClaudeMessage(sdkToolResult("a", "Interrupted", true), state), []);
});

const AGY_PATH = "/proj/hello.txt";

test("agy write_file with a diff block becomes a write card with a diff; edit_file an edit card", () => {
  const state = createStreamState();
  const write = {
    sessionUpdate: "tool_call_update",
    toolCallId: "w",
    name: "write_file",
    status: "completed",
    kind: "edit",
    title: "File hello.txt",
    rawInput: { Content: "hi\n", Overwrite: true, Path: AGY_PATH },
    rawOutput: {},
    content: [{ type: "diff", path: AGY_PATH, newText: "hi\n" }],
  };
  const events = mapSessionUpdate(write, state);
  const end = events.find((event) => event.type === "tool_execution_end") as Json;
  assert.equal(end.status, "completed");
  assert.equal(typeof end.turnId, "string");
  assert.deepEqual(end.detail, {
    kind: "write",
    path: AGY_PATH,
    label: "File hello.txt",
    newText: "hi\n",
    diff: "@@ -0,0 +1,1 @@\n+hi",
  });
  const edit = mapSessionUpdate(
    {
      ...write,
      toolCallId: "e",
      name: "edit_file",
      rawInput: { Path: AGY_PATH, Replacement: "hello", Target: "hi" },
      content: [{ type: "diff", path: AGY_PATH, oldText: "hi\n", newText: "hello\n" }],
    },
    state
  ).find((event) => event.type === "tool_execution_end") as Json;
  assert.equal((edit.detail as Json).kind, "edit");
  assert.equal((edit.detail as Json).diff, "@@ -1,1 +1,1 @@\n-hi\n+hello");
});

test("agy run_command is a shell card; a call open at cancel ends as canceled", () => {
  const state = createStreamState();
  const start = mapSessionUpdate(
    {
      sessionUpdate: "tool_call",
      toolCallId: "c",
      name: "run_command",
      kind: "execute",
      status: "in_progress",
      rawInput: { CommandLine: "latexmk -pdf main.tex", Cwd: "/proj" },
    },
    state
  ).find((event) => event.type === "tool_execution_start") as Json;
  assert.equal(start.status, "running");
  assert.deepEqual(start.detail, { kind: "shell", cwd: "/proj", label: "run_command", command: "latexmk -pdf main.tex" });
  const closed = closeTurn(state, "cancelled");
  const end = closed.find((event) => event.type === "tool_execution_end") as Json;
  assert.equal(end.status, "canceled");
  assert.equal(end.turnId, start.turnId);
});

test("Codex fileChange carries its diff; commandExecution its exit code; interrupt cancels open items", () => {
  const state = createCodexState();
  const diff = "@@ -1,1 +1,1 @@\n-a\n+b";
  const file = mapNotification(
    "item/completed",
    { item: { type: "fileChange", id: "f", status: "completed", changes: [{ path: "/p/a.tex", kind: { type: "update" }, diff }] } },
    state
  ).find((event) => event.type === "tool_execution_end") as Json;
  assert.deepEqual(file.detail, { kind: "edit", path: "/p/a.tex", diff });
  const added = mapNotification(
    "item/completed",
    { item: { type: "fileChange", id: "n", status: "completed", changes: [{ path: "/p/b.bib", kind: { type: "add" }, diff: "@x{y}\n" }] } },
    state
  ).find((event) => event.type === "tool_execution_end") as Json;
  assert.deepEqual(added.detail, { kind: "write", path: "/p/b.bib", diff: "@@ -0,0 +1,1 @@\n+@x{y}" });
  // Several files: headers dropped, a diff --git line before each file.
  const multi = mapNotification(
    "item/completed",
    {
      item: {
        type: "fileChange", id: "m", status: "completed",
        changes: [
          { path: "/p/a.tex", kind: { type: "update" }, diff: "--- a/a.tex\n+++ b/a.tex\n" + diff },
          { path: "/p/c.tex", kind: { type: "update" }, diff: "--- a/c.tex\n+++ b/c.tex\n" + diff },
        ],
      },
    },
    state
  ).find((event) => event.type === "tool_execution_end") as Json;
  assert.equal(
    (multi.detail as Json).diff,
    `diff --git a//p/a.tex b//p/a.tex\n${diff}\ndiff --git a//p/c.tex b//p/c.tex\n${diff}`
  );
  const shell = mapNotification(
    "item/completed",
    { item: { type: "commandExecution", id: "s", command: "make", cwd: "/p", status: "completed", exitCode: 2, aggregatedOutput: "err" } },
    state
  ).find((event) => event.type === "tool_execution_end") as Json;
  assert.equal(shell.status, "failed");
  assert.deepEqual(shell.detail, { kind: "shell", command: "make", cwd: "/p", exitCode: 2 });

  mapNotification("item/started", { item: { type: "commandExecution", id: "open", command: "sleep 9" } }, state);
  const closed = mapNotification("turn/completed", { turn: { status: "interrupted" } }, state);
  assert.deepEqual(
    closed.filter((event) => event.type === "tool_execution_end").map((event) => [event.toolCallId, event.status]),
    [["open", "canceled"]]
  );
  assert.ok(closed.filter((event) => event.type !== "agent_end").every((event) => typeof event.turnId === "string"));
});
