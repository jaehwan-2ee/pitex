// Maps Claude Agent SDK messages onto the pi RPC events the Mac and GTK
// transcript UIs already render.
//
// Checked against @anthropic-ai/claude-agent-sdk 0.3.312
// (https://code.claude.com/docs/en/agent-sdk/typescript):
// SDKPartialAssistantMessage (`stream_event` / BetaRawMessageStreamEvent),
// SDKAssistantMessage, SDKUserMessage tool_result blocks, SDKResultMessage,
// and CanUseTool PermissionResult (`behavior` allow/deny, `updatedPermissions`
// with destination `session`).
//
// This file is original Pitex code written against that SDK reference.

import type { ApprovalAction, ModeState } from "./adapter.ts";
import { cleanDetail, newTurnId, patchDiff, stampTurn, unifiedDiff, type ToolDetail } from "./tool-detail.ts";
import { foldClaudeTodo, sameTodos, TODO_TOOLS, type TodoItem } from "./todos.ts";

export type Json = Record<string, unknown>;

export interface ClaudeStreamState {
  nextContentIndex: number;
  textIndexByKey: Record<string, number>;
  thinkingIndexByKey: Record<string, number>;
  textDraftByKey: Record<string, number>;
  thinkingDraftByKey: Record<string, number>;
  assistantText: string;
  thinkingText: string;
  turnOpen: boolean;
  toolStarted: Record<string, true>;
  toolFinished: Record<string, true>;
  toolNameById: Record<string, string>;
  toolArgsById: Record<string, Json>;
  /** The SDK tool name and full input, for the typed detail. */
  toolRawNameById: Record<string, string>;
  toolInputById: Record<string, Json>;
  turnId: string | null;
  toolJsonById: Record<string, string>;
  /** content_block index → tool id while input JSON is still streaming. */
  toolIndexToId: Record<string, string>;
  transcript: Json[];
  draft: Json[];
  /** The conversation's todo list (`todo_update`); kept across turns. */
  todos: TodoItem[] | null;
}

export function createClaudeStreamState(): ClaudeStreamState {
  return {
    nextContentIndex: 0,
    textIndexByKey: {},
    thinkingIndexByKey: {},
    textDraftByKey: {},
    thinkingDraftByKey: {},
    assistantText: "",
    thinkingText: "",
    turnOpen: false,
    toolStarted: {},
    toolFinished: {},
    toolNameById: {},
    toolArgsById: {},
    toolRawNameById: {},
    toolInputById: {},
    turnId: null,
    toolJsonById: {},
    toolIndexToId: {},
    transcript: [],
    draft: [],
    todos: null,
  };
}

/** Completed messages plus the assistant blocks still streaming. */
export function messagesFromState(state: ClaudeStreamState): Json[] {
  if (state.draft.length === 0) return state.transcript;
  return [...state.transcript, { role: "assistant", content: state.draft.map((block) => ({ ...block })) }];
}

export interface SdkModelInfo {
  value: string;
  displayName: string;
  description?: string;
  supportedEffortLevels?: string[];
}

export interface ListedClaudeModel {
  id: string;
  provider: "claude";
  name: string;
  efforts: string[];
}

export function modelsFromSdk(list: unknown): ListedClaudeModel[] {
  if (!Array.isArray(list)) return [];
  const models: ListedClaudeModel[] = [];
  for (const raw of list) {
    const row = asObject(raw);
    const id = asString(row?.value);
    if (!row || !id) continue;
    const efforts = Array.isArray(row.supportedEffortLevels)
      ? row.supportedEffortLevels.filter((level): level is string => typeof level === "string" && level.length > 0)
      : [];
    models.push({
      id,
      provider: "claude",
      name: asString(row.displayName) ?? id,
      efforts,
    });
  }
  return models;
}

export function visibleClaudeModels(models: ListedClaudeModel[]): { id: string; provider: string; name: string }[] {
  return models.map(({ id, provider, name }) => ({ id, provider, name }));
}

const FILE_TOOLS = new Set(["Edit", "Write", "NotebookEdit", "MultiEdit"]);
const SUMMARY_KEYS = [
  "query",
  "pattern",
  "url",
  "prompt",
  "file_path",
  "path",
  "notebook_path",
  "command",
  "description",
];

function textField(input: Record<string, unknown>, key: string): string {
  const value = input[key];
  return typeof value === "string" ? value.trim() : "";
}

function filePathOf(input: Record<string, unknown>): string {
  for (const key of ["file_path", "path", "notebook_path"]) {
    const value = textField(input, key);
    if (value) return value;
  }
  return "";
}

/** One line for the approval card and the tool row. JSON is only the last resort. */
function compactToolSummary(toolName: string, input: Record<string, unknown>): string {
  if (toolName === "AskUserQuestion") {
    const asked = claudeQuestions(input).map((question) => question.question).join(" · ");
    if (asked) return `${toolName}: ${asked}`;
  }
  for (const key of SUMMARY_KEYS) {
    const value = textField(input, key);
    if (value) return `${toolName}: ${value}`;
  }
  for (const value of Object.values(input)) {
    if (typeof value === "string" && value.trim()) return `${toolName}: ${value.trim()}`;
  }
  const parts: string[] = [];
  for (const [key, value] of Object.entries(input)) {
    if (typeof value === "number" || typeof value === "boolean") parts.push(`${key}=${String(value)}`);
  }
  if (parts.length > 0) return `${toolName}: ${parts.join(", ")}`;
  if (Object.keys(input).length === 0) return "";
  try {
    return JSON.stringify(input);
  } catch {
    return "";
  }
}

export function approvalPresentation(
  toolName: string,
  input: Record<string, unknown>,
  options: { title?: string; displayName?: string; description?: string; decisionReason?: string }
): { kind: "command" | "file" | "permissions"; title: string; detail: string } {
  const command = textField(input, "command");
  const filePath = filePathOf(input);
  const kind = toolName === "Bash" ? "command" : FILE_TOOLS.has(toolName) ? "file" : "permissions";
  // displayName is the raw tool name ("Bash"). Codex and Grok cards use a
  // human title, and the detail is the command or path rather than the
  // model's short description of that action.
  const explicit = options.title?.trim() ?? "";
  const title =
    kind === "command"
      ? "Run command"
      : kind === "file"
        ? "Edit files"
        : explicit && explicit !== toolName
          ? explicit
          : "Allow action";
  const described = options.description?.trim() || options.decisionReason?.trim() || "";
  const detail =
    kind === "command"
      ? command || described || toolName
      : kind === "file"
        ? filePath || described || toolName
        : compactToolSummary(toolName, input) || (described ? `${toolName}: ${described}` : toolName);
  return { kind, title, detail: detail.slice(0, 4000) };
}

/**
 * The permission modes Pitex offers for Claude Code. bypassPermissions, auto
 * and dontAsk are never offered, and a session always starts in default.
 */
export const CLAUDE_MODES: ModeState["options"] = [
  { id: "default", label: "Ask before edits" },
  { id: "acceptEdits", label: "Accept edits" },
  { id: "plan", label: "Plan" },
];

export function isClaudeMode(id: unknown): id is string {
  return CLAUDE_MODES.some((mode) => mode.id === id);
}

/** ExitPlanMode buttons. Approving switches the session to default or, when picked, to acceptEdits. */
export const PLAN_ACTIONS: ApprovalAction[] = [
  { id: "approve_plan", label: "Approve plan", behavior: "allow", variant: "primary" },
  { id: "approve_plan_accept_edits", label: "Approve and accept edits", behavior: "allow", variant: "secondary" },
  { id: "keep_planning", label: "Keep planning", behavior: "deny" },
];

/** Same cap as the host Markdown renderer. */
const PLAN_CAP = 32_000;

export interface ApprovalQuestion {
  id: string;
  header: string;
  question: string;
  options: { label: string; description?: string }[];
  multiSelect: boolean;
  allowOther: boolean;
}

/** AskUserQuestion input → card questions. "Other" is the host's to offer, so every question allows it (as Paseo does). */
export function claudeQuestions(input: Json): ApprovalQuestion[] {
  const raw = Array.isArray(input.questions) ? input.questions : [];
  return raw.flatMap((item, index) => {
    const row = asObject(item);
    const question = asString(row?.question)?.trim();
    if (!row || !question) return [];
    const options = (Array.isArray(row.options) ? row.options : []).flatMap((option) => {
      const entry = asObject(option);
      const label = (typeof option === "string" ? option : asString(entry?.label))?.trim();
      if (!label) return [];
      const description = asString(entry?.description)?.trim();
      return [description ? { label, description } : { label }];
    });
    return [
      {
        id: String(index),
        header: asString(row.header)?.trim() ?? "",
        question,
        options,
        multiSelect: row.multiSelect === true,
        allowOther: true,
      },
    ];
  });
}

/**
 * Host answers (question id → label, labels, or "Other" text) → AskUserQuestion
 * `answers`, keyed by the full question text and multi-select labels joined
 * with ", " (Paseo's shape). Null when nothing was answered.
 */
export function claudeAnswers(input: Json, answers: unknown): Record<string, string> | null {
  const given = asObject(answers);
  if (!given) return null;
  const out: Record<string, string> = {};
  for (const question of claudeQuestions(input)) {
    const value = given[question.id];
    const parts = (Array.isArray(value) ? value : [value])
      .filter((part): part is string => typeof part === "string")
      .map((part) => part.trim())
      .filter(Boolean);
    if (parts.length > 0) out[question.question] = parts.join(", ");
  }
  return Object.keys(out).length > 0 ? out : null;
}

/** The approval_request event for one canUseTool call: question, plan, or a tool with its typed detail. */
export function claudeApprovalEvent(
  id: string,
  toolName: string,
  input: Json,
  options: { title?: string; displayName?: string; description?: string; decisionReason?: string }
): Json {
  if (toolName === "AskUserQuestion") {
    const questions = claudeQuestions(input);
    if (questions.length > 0) {
      const first = questions[0];
      const choices = first.options.map((option) => option.label).join(" / ");
      return {
        type: "approval_request",
        id,
        kind: "question",
        title: "Claude has a question",
        detail: [first.question, choices].filter(Boolean).join("\n").slice(0, 4000),
        questions,
      };
    }
  }
  if (toolName === "ExitPlanMode") {
    const plan = typeof input.plan === "string" ? input.plan : "";
    return {
      type: "approval_request",
      id,
      kind: "plan",
      title: "Review the plan",
      detail: plan.slice(0, 4000),
      plan: plan.slice(0, PLAN_CAP),
      actions: PLAN_ACTIONS,
    };
  }
  const event: Json = { type: "approval_request", id, ...approvalPresentation(toolName, input, options) };
  const detail = claudeToolDetail(toolName, input);
  if (detail.kind !== "other") event.toolDetail = detail;
  return event;
}

/** Tool-row fields the Mac and Linux transcripts already render without dumping JSON. */
export function toolExecutionFields(name: string, args: Json): { toolName: string; args: Json } {
  if (name === "Bash") {
    const command = textField(args, "command");
    return { toolName: "bash", args: command ? { command } : {} };
  }
  if (FILE_TOOLS.has(name)) {
    const path = filePathOf(args);
    return { toolName: name === "Write" ? "write" : "edit", args: path ? { path } : {} };
  }
  const summary = compactToolSummary(name, args);
  if (!summary || summary.startsWith("{") || summary.startsWith("[")) return { toolName: name, args };
  return { toolName: name, args: { summary } };
}

/** Identity of one tool call for "always allow" during this session only. */
export function allowKey(toolName: string, input: Record<string, unknown>): string {
  const command = typeof input.command === "string" ? input.command : "";
  const filePath =
    typeof input.file_path === "string" ? input.file_path : typeof input.path === "string" ? input.path : "";
  return `${toolName}\n${command}\n${filePath}`;
}

export interface PermissionChoice {
  result: Json;
  remember: boolean;
  rememberKey: string;
}

/**
 * Pitex Approve / Deny / Always allow → SDK PermissionResult.
 * Always allow rewrites every suggestion onto destination `session` so the
 * CLI does not persist a rule into a settings file. When the SDK marks the
 * ask `suppressAlwaysAllowRule`, Always allow is treated as allow once.
 */
export function permissionForDecision(
  decision: string | undefined,
  toolName: string,
  input: Record<string, unknown>,
  suggestions: unknown,
  suppressAlwaysAllow: boolean,
  toolUseID?: string
): PermissionChoice {
  const rememberKey = allowKey(toolName, input);
  const choice = decision === "approve" || decision === "always_allow" || decision === "deny" ? decision : "deny";
  if (choice === "deny") {
    return {
      remember: false,
      rememberKey,
      result: {
        behavior: "deny",
        message: "The user denied this action.",
        ...(toolUseID ? { toolUseID } : {}),
        decisionClassification: "user_reject",
      },
    };
  }
  const always = choice === "always_allow" && !suppressAlwaysAllow;
  const updatedPermissions = always ? sessionScopedPermissions(suggestions, toolName, input) : undefined;
  return {
    remember: always,
    rememberKey,
    result: {
      behavior: "allow",
      updatedInput: input,
      ...(updatedPermissions && updatedPermissions.length > 0 ? { updatedPermissions } : {}),
      ...(toolUseID ? { toolUseID } : {}),
      decisionClassification: always ? "user_permanent" : "user_temporary",
    },
  };
}

function sessionScopedPermissions(suggestions: unknown, toolName: string, input: Record<string, unknown>): Json[] {
  const updates: Json[] = [];
  if (Array.isArray(suggestions)) {
    for (const raw of suggestions) {
      const row = asObject(raw);
      if (!row || !Array.isArray(row.rules)) continue;
      updates.push({
        type: asString(row.type) ?? "addRules",
        rules: row.rules,
        behavior: asString(row.behavior) ?? "allow",
        destination: "session",
      });
    }
  }
  if (updates.length > 0) return updates;
  return [
    {
      type: "addRules",
      behavior: "allow",
      destination: "session",
      rules: [{ toolName, ...(allowKey(toolName, input) ? { ruleContent: allowKey(toolName, input) } : {}) }],
    },
  ];
}

const SEARCH_TOOLS = new Set(["Grep", "Glob", "WebSearch"]);

function numberField(input: Json, key: string): number | undefined {
  const value = input[key];
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

/** Typed detail for one Claude Code tool call, from its name and input (Paseo's name sets). */
export function claudeToolDetail(name: string, input: Json): ToolDetail {
  const path = filePathOf(input) || undefined;
  if (name === "Bash") return cleanDetail({ kind: "shell", command: textField(input, "command") });
  if (name === "Read") {
    const offset = numberField(input, "offset");
    const limit = numberField(input, "limit");
    const startLine = offset ?? (limit !== undefined ? 1 : undefined);
    const endLine = startLine !== undefined && limit !== undefined ? startLine + limit - 1 : undefined;
    return cleanDetail({ kind: "read", path, startLine, endLine });
  }
  if (name === "Edit") {
    const oldText = typeof input.old_string === "string" ? input.old_string : "";
    const newText = typeof input.new_string === "string" ? input.new_string : "";
    return cleanDetail({ kind: "edit", path, oldText, newText, diff: unifiedDiff(oldText, newText) });
  }
  if (name === "MultiEdit") {
    const edits = Array.isArray(input.edits) ? input.edits : [];
    const diff = edits
      .map((edit) => asObject(edit))
      .map((edit) =>
        edit ? unifiedDiff(asString(edit.old_string) ?? "", asString(edit.new_string) ?? "") : ""
      )
      .filter(Boolean)
      .join("\n");
    return cleanDetail({ kind: "edit", path, diff });
  }
  if (name === "Write") {
    const newText = typeof input.content === "string" ? input.content : "";
    return cleanDetail({ kind: "write", path, newText, diff: unifiedDiff("", newText) });
  }
  if (name === "NotebookEdit") return cleanDetail({ kind: "edit", path });
  if (SEARCH_TOOLS.has(name)) {
    const query = textField(input, name === "WebSearch" ? "query" : "pattern");
    return cleanDetail({ kind: "search", query, path, label: name });
  }
  if (name === "WebFetch") return cleanDetail({ kind: "fetch", url: textField(input, "url") });
  if (name === "Task" || name === "Agent") {
    return cleanDetail({
      kind: "sub_agent",
      label: textField(input, "description") || textField(input, "subagent_type"),
    });
  }
  return { kind: "other", label: name };
}

/** Fields only the tool result knows: exit code, real-line diff, match count. */
function claudeResultDetail(name: string, input: Json, text: string, isError: boolean, result: Json | null): ToolDetail {
  const detail = claudeToolDetail(name, input);
  if (detail.kind === "shell") {
    const match = /^Exit code (-?\d+)/.exec(text.trimStart());
    detail.exitCode = match ? Number(match[1]) : isError ? undefined : 0;
  }
  if ((detail.kind === "edit" || detail.kind === "write") && result) {
    const diff = patchDiff(result.structuredPatch);
    if (diff) detail.diff = diff;
    else if (detail.kind === "write" && typeof result.originalFile === "string") {
      // The raw content: detail.newText is already capped at TEXT_CAP.
      detail.diff = unifiedDiff(result.originalFile, typeof input.content === "string" ? input.content : "");
    }
  }
  if (detail.kind === "search" && result) {
    // Not numFiles: Grep's default files_with_matches mode counts files, not matches.
    detail.numMatches = numberField(result, "numMatches") ?? numberField(result, "numLines");
  }
  return cleanDetail(detail);
}

function asObject(value: unknown): Json | null {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Json) : null;
}

function asString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function textEvent(kind: string, contentIndex: number, delta?: string): Json {
  const assistantMessageEvent: Json = { type: kind, contentIndex };
  if (delta !== undefined) assistantMessageEvent.delta = delta;
  return { type: "message_update", assistantMessageEvent };
}

function toolContent(text: string): Json {
  return { content: [{ type: "text", text }] };
}

function ensureTurn(state: ClaudeStreamState, out: Json[]): void {
  if (state.turnOpen) return;
  state.turnOpen = true;
  state.turnId = newTurnId();
  out.push({ type: "agent_start" });
  out.push({ type: "message_start", message: { role: "assistant" } });
}

function flushDraft(state: ClaudeStreamState): void {
  if (state.draft.length === 0) return;
  state.transcript.push({ role: "assistant", content: state.draft });
  state.draft = [];
  state.textDraftByKey = {};
  state.thinkingDraftByKey = {};
  state.textIndexByKey = {};
  state.thinkingIndexByKey = {};
}

function appendDraftText(state: ClaudeStreamState, key: string, bucket: "text" | "thinking", delta: string): void {
  const indexByKey = bucket === "text" ? state.textDraftByKey : state.thinkingDraftByKey;
  const field = bucket === "text" ? "text" : "thinking";
  const existing = indexByKey[key];
  const block = existing === undefined ? undefined : state.draft[existing];
  if (!block || block.type !== bucket) {
    state.draft.push({ type: bucket, [field]: delta });
    indexByKey[key] = state.draft.length - 1;
    return;
  }
  block[field] = `${block[field] ?? ""}${delta}`;
}

function appendText(state: ClaudeStreamState, key: string, delta: string, out: Json[]): void {
  if (!delta) return;
  ensureTurn(state, out);
  let index = state.textIndexByKey[key];
  if (index === undefined) {
    index = state.nextContentIndex++;
    state.textIndexByKey[key] = index;
    out.push(textEvent("text_start", index));
  }
  state.assistantText += delta;
  out.push(textEvent("text_delta", index, delta));
  appendDraftText(state, key, "text", delta);
}

function appendThinking(state: ClaudeStreamState, key: string, delta: string, out: Json[]): void {
  if (!delta) return;
  ensureTurn(state, out);
  let index = state.thinkingIndexByKey[key];
  if (index === undefined) {
    index = state.nextContentIndex++;
    state.thinkingIndexByKey[key] = index;
    out.push(textEvent("thinking_start", index));
  }
  state.thinkingText += delta;
  out.push(textEvent("thinking_delta", index, delta));
  appendDraftText(state, key, "thinking", delta);
}

function emitFullText(state: ClaudeStreamState, text: string, out: Json[]): void {
  if (!text) return;
  const already = state.assistantText;
  if (text === already) return;
  const rest = already && text.startsWith(already) ? text.slice(already.length) : already ? "" : text;
  if (rest) appendText(state, "assistant", rest, out);
}

function emitFullThinking(state: ClaudeStreamState, text: string, out: Json[]): void {
  if (!text) return;
  const already = state.thinkingText;
  if (text === already) return;
  const rest = already && text.startsWith(already) ? text.slice(already.length) : already ? "" : text;
  if (rest) appendThinking(state, "thinking", rest, out);
}

function upsertToolDraft(state: ClaudeStreamState, id: string): void {
  const name = state.toolNameById[id] ?? "tool";
  const args = state.toolArgsById[id] ?? {};
  const existing = state.draft.find((block) => block.type === "toolCall" && block.id === id);
  if (existing) {
    existing.name = name;
    existing.arguments = args;
    return;
  }
  state.draft.push({ type: "toolCall", id, name, arguments: args });
}

function startTool(state: ClaudeStreamState, id: string, name: string, args: Json, out: Json[]): void {
  const row = toolExecutionFields(name, args);
  state.toolNameById[id] = row.toolName;
  state.toolArgsById[id] = row.args;
  state.toolRawNameById[id] = name;
  state.toolInputById[id] = args;
  upsertToolDraft(state, id);
  if (state.toolStarted[id]) return;
  state.toolStarted[id] = true;
  ensureTurn(state, out);
  // The checklist shows these calls once they succeed; a failed one gets its card in finishTool.
  if (TODO_TOOLS.has(name)) return;
  out.push({
    type: "tool_execution_start",
    toolCallId: id,
    toolName: row.toolName,
    args: row.args,
    detail: claudeToolDetail(name, args),
    status: "running",
  });
}

function finishTool(
  state: ClaudeStreamState,
  id: string,
  text: string,
  isError: boolean,
  out: Json[],
  result: Json | null = null,
  canceled = false
): void {
  if (state.toolFinished[id]) return;
  if (!state.toolStarted[id]) {
    startTool(state, id, state.toolRawNameById[id] ?? state.toolNameById[id] ?? "tool", state.toolInputById[id] ?? state.toolArgsById[id] ?? {}, out);
  }
  state.toolFinished[id] = true;
  const name = state.toolRawNameById[id] ?? "tool";
  const input = state.toolInputById[id] ?? {};
  const todo = TODO_TOOLS.has(name);
  // A rejected TodoWrite list never reaches the checklist.
  if (todo && !isError) setTodos(state, foldClaudeTodo(state.todos, name, input, result, text), out);
  const showCard = !todo || (isError && !canceled);
  if (todo && showCard) {
    out.push({ type: "tool_execution_start", toolCallId: id, toolName: name, args: {}, detail: claudeToolDetail(name, input), status: "running" });
  }
  const detail = claudeResultDetail(name, input, text, isError, result);
  const status = canceled ? "canceled" : isError ? "failed" : "completed";
  if (showCard) {
    out.push({
      type: "tool_execution_end",
      toolCallId: id,
      isError,
      result: toolContent(text),
      detail,
      status,
    });
  }
  recordResult(state, id, text, isError, detail, status);
}

function setTodos(state: ClaudeStreamState, items: TodoItem[] | null, out: Json[]): void {
  if (!items || sameTodos(items, state.todos)) return;
  state.todos = items;
  out.push({ type: "todo_update", items });
}

/** The result row keeps the typed card (`detail`, `status`) so a resumed chat draws it again. */
function recordResult(state: ClaudeStreamState, id: string, text: string, isError: boolean, detail: ToolDetail, status: string): void {
  flushDraft(state);
  state.transcript.push({
    role: "toolResult",
    toolCallId: id,
    toolName: state.toolNameById[id] ?? "tool",
    isError,
    content: [{ type: "text", text }],
    detail,
    status,
  });
}

/** End every started, unfinished tool call as canceled (abort, or a turn that closed under it). */
function cancelOpen(state: ClaudeStreamState, out: Json[]): void {
  for (const id of Object.keys(state.toolStarted)) {
    if (!state.toolFinished[id]) finishTool(state, id, "", true, out, null, true);
  }
}

export function cancelOpenTools(state: ClaudeStreamState): Json[] {
  const out: Json[] = [];
  cancelOpen(state, out);
  return stampTurn(out, state.turnId);
}

function resetTurnBuffers(state: ClaudeStreamState): void {
  state.turnOpen = false;
  state.assistantText = "";
  state.thinkingText = "";
  state.textIndexByKey = {};
  state.thinkingIndexByKey = {};
  state.textDraftByKey = {};
  state.thinkingDraftByKey = {};
  state.draft = [];
  state.nextContentIndex = 0;
  state.toolStarted = {};
  state.toolFinished = {};
  state.toolJsonById = {};
  state.toolIndexToId = {};
  state.toolRawNameById = {};
  state.toolInputById = {};
}

function closeTurn(state: ClaudeStreamState, message: Json, out: Json[]): void {
  const failed = message.subtype !== "success" || message.is_error === true;
  if (!state.assistantText && !failed && typeof message.result === "string") {
    appendText(state, "final", message.result, out);
  }
  cancelOpen(state, out);
  flushDraft(state);
  ensureTurn(state, out);
  const errorText = resultError(message);
  const content: Json[] = [];
  if (state.thinkingText) content.push({ type: "thinking", thinking: state.thinkingText });
  if (state.assistantText) content.push({ type: "text", text: state.assistantText });
  const assistant: Json = {
    role: "assistant",
    stopReason: failed ? "error" : "stop",
    content,
  };
  if (errorText) assistant.errorMessage = errorText;
  out.push({ type: "message_end", message: assistant });
  out.push({ type: "turn_end" });
  const agentEnd: Json = { type: "agent_end" };
  if (failed) agentEnd.messages = [assistant];
  out.push(agentEnd);
  resetTurnBuffers(state);
}

function resultError(message: Json): string | null {
  if (message.subtype === "success" && message.is_error !== true) return null;
  if (Array.isArray(message.errors)) {
    const lines = message.errors.filter((line): line is string => typeof line === "string" && line.length > 0);
    if (lines.length > 0) return lines.join("\n");
  }
  if (typeof message.result === "string" && message.result.length > 0) return message.result;
  return "Claude Code stopped before finishing.";
}

function blockText(content: unknown): string {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .map((part) => {
      if (typeof part === "string") return part;
      const row = asObject(part);
      return row?.type === "text" && typeof row.text === "string" ? row.text : "";
    })
    .filter(Boolean)
    .join("\n");
}

function contentBlocks(message: Json): Json[] {
  const inner = asObject(message.message);
  const content = inner?.content;
  if (!Array.isArray(content)) return [];
  return content.map((part) => asObject(part)).filter((part): part is Json => part !== null);
}

function applyStreamEvent(state: ClaudeStreamState, event: Json, out: Json[]): void {
  const index = typeof event.index === "number" ? String(event.index) : "0";
  if (event.type === "content_block_start") {
    const block = asObject(event.content_block);
    if (!block) return;
    if (block.type === "tool_use") {
      const id = asString(block.id);
      const name = asString(block.name) ?? "tool";
      if (!id) return;
      state.toolNameById[id] = name;
      state.toolRawNameById[id] = name;
      state.toolIndexToId[index] = id;
      const input = asObject(block.input) ?? {};
      state.toolArgsById[id] = input;
      state.toolJsonById[id] = "";
      if (Object.keys(input).length > 0) startTool(state, id, name, input, out);
    }
    return;
  }
  if (event.type === "content_block_delta") {
    const delta = asObject(event.delta);
    if (!delta) return;
    if (delta.type === "text_delta") {
      appendText(state, `text:${index}`, asString(delta.text) ?? "", out);
      return;
    }
    if (delta.type === "thinking_delta") {
      appendThinking(state, `thinking:${index}`, asString(delta.thinking) ?? "", out);
      return;
    }
    if (delta.type === "input_json_delta") {
      const id = state.toolIndexToId[index];
      if (!id) return;
      state.toolJsonById[id] = (state.toolJsonById[id] ?? "") + (typeof delta.partial_json === "string" ? delta.partial_json : "");
    }
    return;
  }
  if (event.type === "content_block_stop") {
    const id = state.toolIndexToId[index];
    if (!id || state.toolStarted[id]) return;
    let args: Json = {};
    const raw = state.toolJsonById[id] ?? "";
    if (raw) {
      try {
        const parsed = JSON.parse(raw) as unknown;
        args = asObject(parsed) ?? {};
      } catch {
        return;
      }
    }
    startTool(state, id, state.toolNameById[id] ?? "tool", args, out);
  }
}

function applyAssistant(state: ClaudeStreamState, message: Json, out: Json[]): void {
  for (const block of contentBlocks(message)) {
    if (block.type === "text") {
      emitFullText(state, typeof block.text === "string" ? block.text : "", out);
      continue;
    }
    if (block.type === "thinking") {
      emitFullThinking(state, typeof block.thinking === "string" ? block.thinking : "", out);
      continue;
    }
    if (block.type === "tool_use") {
      const id = asString(block.id);
      const name = asString(block.name) ?? "tool";
      if (!id) continue;
      startTool(state, id, name, asObject(block.input) ?? {}, out);
    }
  }
}

function applyUser(state: ClaudeStreamState, message: Json, out: Json[]): void {
  const results = contentBlocks(message).filter((block) => block.type === "tool_result");
  // `tool_use_result` is the structured result of the message's one tool call.
  const structured = results.length === 1 ? asObject(message.tool_use_result) : null;
  for (const block of results) {
    const id = asString(block.tool_use_id);
    if (!id) continue;
    finishTool(state, id, blockText(block.content), block.is_error === true, out, structured);
  }
}

/**
 * Project one SDK message into zero or more pi events.
 * Unknown message types produce nothing so a newer CLI can add events
 * without breaking the transcript. `auth_status` is ignored here; its
 * `output` lines are never copied onto the pi stream.
 */
export function mapClaudeMessage(message: unknown, state: ClaudeStreamState): Json[] {
  const row = asObject(message);
  if (!row) return [];
  const out: Json[] = [];
  switch (row.type) {
    case "stream_event": {
      const event = asObject(row.event);
      if (event) applyStreamEvent(state, event, out);
      break;
    }
    case "assistant":
      applyAssistant(state, row, out);
      break;
    case "user":
      applyUser(state, row, out);
      break;
    case "result":
      closeTurn(state, row, out);
      break;
  }
  return stampTurn(out, state.turnId);
}

/** CLI bookkeeping that Claude Code stores as user text but never shows as a prompt. */
const HIDDEN_USER_TEXT = /^(\[Request interrupted|<command-|<local-command-)/;

/**
 * A saved conversation (the SDK's getSessionMessages: `{type, message, is_meta?}` rows) as the
 * same transcript the live stream builds: typed tool results, thinking, and the todo list,
 * with no events. Calls that never got a result end as canceled.
 */
export function replayClaudeMessages(messages: unknown[]): ClaudeStreamState {
  const state = createClaudeStreamState();
  const none: Json[] = [];
  for (const raw of messages) {
    const row = asObject(raw);
    if (!row || row.is_meta === true) continue;
    if (row.type === "assistant") {
      for (const block of contentBlocks(row)) {
        if (block.type === "text" && asString(block.text)) state.draft.push({ type: "text", text: block.text });
        if (block.type === "thinking" && asString(block.thinking)) state.draft.push({ type: "thinking", thinking: block.thinking });
        const id = asString(block.id);
        if (block.type === "tool_use" && id) startTool(state, id, asString(block.name) ?? "tool", asObject(block.input) ?? {}, none);
      }
      continue;
    }
    if (row.type !== "user") continue;
    if (contentBlocks(row).some((block) => block.type === "tool_result")) {
      applyUser(state, row, none);
      continue;
    }
    const text = blockText(asObject(row.message)?.content).trim();
    if (!text || HIDDEN_USER_TEXT.test(text)) continue;
    cancelOpen(state, none);
    flushDraft(state);
    state.transcript.push({ role: "user", content: [{ type: "text", text }] });
  }
  cancelOpen(state, none);
  flushDraft(state);
  resetTurnBuffers(state);
  return state;
}

/**
 * The SDK's listSessions rows (`{sessionId, summary, lastModified, customTitle?, firstPrompt?}`)
 * as importListedSessions rows, newest first. The SDK already scoped them to `projectRoot`.
 */
export function claudeSessionRows(listed: unknown, projectRoot: string, limit: number): Json[] {
  const rows = (Array.isArray(listed) ? listed : []).map((item) => asObject(item)).filter((row): row is Json => row !== null);
  return rows
    .filter((row) => typeof row.lastModified === "number")
    .sort((a, b) => (b.lastModified as number) - (a.lastModified as number))
    .slice(0, limit)
    .map((row) => ({
      sessionId: row.sessionId,
      cwd: projectRoot,
      title: asString(row.customTitle) ?? asString(row.firstPrompt) ?? asString(row.summary) ?? "",
      updatedAt: new Date(row.lastModified as number).toISOString(),
    }));
}

export function sessionIdOf(message: unknown): string | null {
  const row = asObject(message);
  return asString(row?.session_id);
}

export function modelIdOfInit(message: unknown): string | null {
  const row = asObject(message);
  if (row?.type !== "system" || row.subtype !== "init") return null;
  return asString(row.model);
}

/** The permission mode on a system init (launch) or status (Claude switched by itself) message. */
export function reportedPermissionMode(message: unknown): { mode: string; init: boolean } | null {
  const row = asObject(message);
  if (row?.type !== "system" || (row.subtype !== "init" && row.subtype !== "status")) return null;
  const mode = asString(row.permissionMode);
  return mode ? { mode, init: row.subtype === "init" } : null;
}

export function statsFromResult(message: unknown): Json | null {
  const row = asObject(message);
  if (row?.type !== "result") return null;
  const usage = asObject(row.usage);
  const input = typeof usage?.input_tokens === "number" ? usage.input_tokens : 0;
  const output = typeof usage?.output_tokens === "number" ? usage.output_tokens : 0;
  return {
    tokens: { total: input + output },
    cost: typeof row.total_cost_usd === "number" ? row.total_cost_usd : 0,
    contextUsage: {},
  };
}

/** `claude auth status` prints JSON. Only the `loggedIn` boolean is read. */
export function parseClaudeAuthStatus(stdout: string, exitCode: number | null): boolean {
  const trimmed = stdout.trim();
  const start = trimmed.indexOf("{");
  const end = trimmed.lastIndexOf("}");
  if (start >= 0 && end > start) {
    try {
      const parsed = JSON.parse(trimmed.slice(start, end + 1)) as { loggedIn?: unknown };
      if (typeof parsed.loggedIn === "boolean") return parsed.loggedIn;
    } catch {
      // The CLI also exits 0 when the account is logged in.
    }
  }
  return exitCode === 0;
}
