// Maps ACP notifications (Grok CLI, Antigravity CLI) onto the pi RPC events
// the Mac and GTK transcript UIs already render.
//
// Protocol checked against the Grok CLI ACP guide
// (https://docs.x.ai/build/cli/headless-scripting), ACP v1
// (https://agentclientprotocol.com/protocol/v1/prompt-turn), and the wire of
// `agy acp` 1.5.15: session/update variants agent_message_chunk,
// agent_thought_chunk, tool_call, tool_call_update, and plan;
// session/request_permission option kinds allow_once, allow_always,
// reject_once, reject_always. agy tool calls carry `name` (run_command,
// write_file, ...) and capitalised arguments (CommandLine, Path, Content).
//
// The event shape follows Pitex's Codex adapter, which was adapted from
// T3 Code v0.1.4 (MIT), Copyright (c) 2026 T3 Tools Inc.
// https://github.com/pingdotgg/t3code
// This file is original Pitex code, not a copy of T3's Grok driver.

import { existsSync, mkdirSync, readFileSync, realpathSync, statSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";

import type { AcpProfile } from "./acp-profiles.ts";
import { markOutcome, type ApprovalAction, type ErrorCode, type ModeState } from "./adapter.ts";
import { cleanDetail, newTurnId, stampTurn, unifiedDiff, type ToolDetail, type ToolKind } from "./tool-detail.ts";
import { todosFromAcpPlan, type TodoItem } from "./todos.ts";

export type Json = Record<string, unknown>;

export interface StreamState {
  nextContentIndex: number;
  textIndexByKey: Record<string, number>;
  thinkingIndexByKey: Record<string, number>;
  toolStarted: Record<string, true>;
  toolEnded: Record<string, true>;
  thinkingParts: string[];
  assistantText: string;
  turnOpen: boolean;
  turnId: string | null;
}

export function createStreamState(): StreamState {
  return {
    nextContentIndex: 0,
    textIndexByKey: {},
    thinkingIndexByKey: {},
    toolStarted: {},
    toolEnded: {},
    thinkingParts: [],
    assistantText: "",
    turnOpen: false,
    turnId: null,
  };
}

export interface ApprovalView {
  id: string;
  kind: "command" | "file" | "permissions";
  title: string;
  detail: string;
  toolDetail?: ToolDetail;
}

export interface PermissionOption {
  optionId: string;
  kind: string;
  name?: string;
}

export interface AcpModel {
  id: string;
  provider: string;
  name: string;
  isDefault: boolean;
  efforts: string[];
}

export interface AcpCatalog {
  models: AcpModel[];
  currentModelId: string | null;
  efforts: string[];
  currentEffort: string | null;
  /** Standard ACP config option id for session/set_config_option. */
  modelConfigId: string | null;
  effortConfigId: string | null;
  /** Grok publishes reasoning effort as a session mode (`session/set_mode`). */
  effortUsesMode: boolean;
  /** Permission mode config option (agy: default, accept-edits, plan). */
  modeConfigId: string | null;
  currentMode: string | null;
  /** The CLI's permission modes, from the mode config option or `modes.availableModes`. */
  modes: ModeState["options"];
}

export function emptyCatalog(): AcpCatalog {
  return {
    models: [],
    currentModelId: null,
    efforts: [],
    currentEffort: null,
    modelConfigId: null,
    effortConfigId: null,
    effortUsesMode: false,
    modeConfigId: null,
    currentMode: null,
    modes: [],
  };
}

type HistoryItem =
  | { kind: "text"; text: string }
  | { kind: "thinking"; text: string }
  | { kind: "tool"; id: string };

interface ToolRecord {
  name: string;
  args: Json;
  output: string;
  isError: boolean;
  /** A completed or failed status arrived; a call left open when the turn ended was canceled. */
  done: boolean;
  /** The typed card for a replayed transcript (`toolResult.detail`). */
  detail: ToolDetail;
}

export interface HistoryState {
  messages: Json[];
  timeline: HistoryItem[];
  tools: Record<string, ToolRecord>;
  /** The last `plan` update (live or replayed by session/load), for the checklist. */
  todos: TodoItem[] | null;
}

export function createHistory(): HistoryState {
  return { messages: [], timeline: [], tools: {}, todos: null };
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

function ensureTurn(state: StreamState, out: Json[]): void {
  if (state.turnOpen) return;
  state.turnOpen = true;
  state.turnId = newTurnId();
  out.push({ type: "agent_start" });
  out.push({ type: "message_start", message: { role: "assistant" } });
}

function chunkText(update: Json): string {
  const content = asObject(update.content);
  if (content && typeof content.text === "string") return content.text;
  if (typeof update.content === "string") return update.content;
  return "";
}

function bounded(text: string, limit: number): string {
  if (text.length <= limit) return text;
  return text.slice(0, limit);
}

function commandText(tool: Json): string {
  // agy puts the input in `args`, `rawInput`, or both.
  const raw = tool.rawInput ?? tool.args;
  if (typeof raw === "string") return raw;
  if (Array.isArray(raw)) {
    return raw.filter((part): part is string => typeof part === "string" && part.length > 0).join(" ");
  }
  const obj = asObject(raw);
  if (!obj) return "";
  if (typeof obj.CommandLine === "string") return obj.CommandLine;
  if (Array.isArray(obj.command)) {
    return obj.command.filter((part): part is string => typeof part === "string" && part.length > 0).join(" ");
  }
  if (typeof obj.command === "string") {
    const args = Array.isArray(obj.args)
      ? obj.args.filter((part): part is string => typeof part === "string" && part.length > 0)
      : [];
    return [obj.command, ...args].join(" ");
  }
  return "";
}

function locationText(tool: Json): string {
  if (!Array.isArray(tool.locations)) return "";
  return tool.locations
    .map((location) => asString(asObject(location)?.path) ?? "")
    .filter(Boolean)
    .join("\n");
}

function contentBlocksText(value: unknown): string {
  if (!Array.isArray(value)) return "";
  return value
    .map((block) => {
      const row = asObject(block);
      if (!row) return "";
      if (row.type === "content") return chunkText({ content: row.content });
      if (row.type === "diff") {
        const path = asString(row.path) ?? "";
        const next = typeof row.newText === "string" ? row.newText : "";
        return [path, next].filter(Boolean).join("\n");
      }
      if (row.type === "terminal") {
        const id = asString(row.terminalId);
        return id ? `terminal ${id}` : "";
      }
      return "";
    })
    .filter(Boolean)
    .join("\n");
}

function rawOutputText(value: unknown): string {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (value && typeof value === "object") {
    try {
      return JSON.stringify(value);
    } catch {
      return "";
    }
  }
  return "";
}

function toolOutputText(update: Json): string {
  const parts = [contentBlocksText(update.content), rawOutputText(update.rawOutput)].filter(Boolean);
  return bounded(parts.join("\n"), 8_000);
}

function toolName(update: Json): string {
  const name = asString(update.name);
  // agy names (run_command, write_file, view_file, ...) become the pi names the cards know.
  const raw = asObject(update.rawInput) ?? asObject(update.args);
  if (typeof raw?.CommandLine === "string") return "bash";
  if (name && typeof raw?.Path === "string") {
    if (update.kind === "read") return "read";
    if (update.kind === "edit") return name === "write_file" ? "write" : "edit";
  }
  if (name) return name;
  switch (update.kind) {
    case "execute":
      return "bash";
    case "edit":
    case "delete":
    case "move":
      return "edit";
    case "read":
      return "read";
    case "search":
      return "search";
    case "fetch":
      return "fetch";
    case "think":
      return "think";
    default:
      return "tool";
  }
}

function toolArgs(update: Json): Json {
  const raw = asObject(update.rawInput) ?? asObject(update.args);
  if (raw && (typeof raw.command === "string" || typeof raw.path === "string" || Array.isArray(raw.command))) {
    return raw;
  }
  const command = commandText(update);
  if (command) return { command };
  const path =
    asString(raw?.Path) ?? asString(asObject(Array.isArray(update.locations) ? update.locations[0] : null)?.path);
  if (path) return { path };
  if (raw) return raw;
  const title = asString(update.title);
  return title ? { title } : {};
}

// agy tool names; other agents fall back to the ACP `kind`.
const AGY_TOOL_KINDS: Record<string, ToolKind> = {
  run_command: "shell",
  write_file: "write",
  create_file: "write",
  write_to_file: "write",
  edit_file: "edit",
  replace_file_content: "edit",
  multi_replace_file_content: "edit",
  view_file: "read",
  read_file: "read",
  grep_search: "search",
  find_by_name: "search",
  codebase_search: "search",
  search_web: "search",
  read_url_content: "fetch",
  // agy 1.5 runs a subagent as one call; ACP reports none of its own calls and no parent id.
  invoke_subagent: "sub_agent",
};

const ACP_KINDS: Record<string, ToolKind> = {
  execute: "shell",
  read: "read",
  edit: "edit",
  delete: "edit",
  move: "edit",
  search: "search",
  fetch: "fetch",
  think: "think",
};

function firstString(raw: Json | null, keys: string[]): string | undefined {
  for (const key of keys) {
    const value = asString(raw?.[key]);
    if (value) return value;
  }
  return undefined;
}

function firstNumber(raw: Json | null, keys: string[]): number | undefined {
  for (const key of keys) {
    const value = raw?.[key];
    if (typeof value === "number" && Number.isFinite(value)) return value;
  }
  return undefined;
}

/** Typed detail for one ACP tool_call / tool_call_update (Paseo's ACP kind mapping plus agy names). */
export function acpToolDetail(update: Json): ToolDetail {
  const raw = asObject(update.rawInput) ?? asObject(update.args);
  const name = asString(update.name) ?? "";
  let kind: ToolKind = AGY_TOOL_KINDS[name] ?? ACP_KINDS[asString(update.kind) ?? ""] ?? "other";
  const diffBlock = Array.isArray(update.content)
    ? update.content.map((block) => asObject(block)).find((block) => block?.type === "diff") ?? null
    : null;
  const location = asObject(Array.isArray(update.locations) ? update.locations[0] : null);
  const detail: ToolDetail = {
    kind,
    path:
      firstString(raw, ["Path", "AbsolutePath", "TargetFile", "File", "path", "file_path", "filePath"]) ??
      asString(location?.path) ??
      asString(diffBlock?.path) ??
      undefined,
    cwd: firstString(raw, ["Cwd", "cwd"]),
    query: firstString(raw, ["Query", "query", "Pattern", "pattern", "SearchPattern"]),
    url: firstString(raw, ["Url", "url"]),
    startLine: firstNumber(raw, ["StartLine", "startLine", "line"]),
    endLine: firstNumber(raw, ["EndLine", "endLine"]),
    exitCode: firstNumber(asObject(update.rawOutput), ["exitCode", "exit_code", "ExitCode"]),
    label: asString(update.title) ?? (name || undefined),
  };
  if (kind === "shell") detail.command = commandText(update);
  if (kind === "sub_agent") {
    detail.description = firstString(raw, ["Role", "Description", "description"]);
    detail.subAgentType = firstString(raw, ["TypeName", "subagent_type"]);
    detail.label = detail.description ?? detail.subAgentType ?? detail.label;
  }
  if (diffBlock && typeof diffBlock.newText === "string") {
    const oldText = typeof diffBlock.oldText === "string" ? diffBlock.oldText : null;
    kind = oldText === null ? "write" : "edit";
    detail.kind = kind;
    detail.newText = diffBlock.newText;
    if (oldText !== null) detail.oldText = oldText;
    detail.diff = unifiedDiff(oldText ?? "", diffBlock.newText);
  }
  return cleanDetail(detail);
}

function approvalShape(tool: Json): { kind: ApprovalView["kind"]; title: string } {
  const kind = asString(tool.kind);
  if (kind === "execute" || commandText(tool)) return { kind: "command", title: "Run command" };
  if (kind === "edit" || kind === "delete" || kind === "move") return { kind: "file", title: "Edit files" };
  return { kind: "permissions", title: asString(tool.title) || "Allow action" };
}

function appendText(state: StreamState, key: string, delta: string, out: Json[]): void {
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
}

function appendThinking(state: StreamState, key: string, delta: string, out: Json[]): void {
  if (!delta) return;
  ensureTurn(state, out);
  let index = state.thinkingIndexByKey[key];
  if (index === undefined) {
    index = state.nextContentIndex++;
    state.thinkingIndexByKey[key] = index;
    out.push(textEvent("thinking_start", index));
  }
  state.thinkingParts.push(delta);
  out.push(textEvent("thinking_delta", index, delta));
}

function startTool(state: StreamState, id: string, update: Json, out: Json[]): void {
  if (state.toolStarted[id]) return;
  state.toolStarted[id] = true;
  ensureTurn(state, out);
  out.push({
    type: "tool_execution_start",
    toolCallId: id,
    toolName: toolName(update),
    args: toolArgs(update),
    detail: acpToolDetail(update),
    status: "running",
  });
}

function updateTool(state: StreamState, id: string, update: Json, out: Json[]): void {
  const text = toolOutputText(update);
  if (!text) return;
  startTool(state, id, update, out);
  out.push({
    type: "tool_execution_update",
    toolCallId: id,
    partialResult: toolContent(text),
    detail: acpToolDetail(update),
    status: "running",
  });
}

function endTool(state: StreamState, id: string, update: Json, failed: boolean, out: Json[]): void {
  if (state.toolEnded[id]) return;
  startTool(state, id, update, out);
  state.toolEnded[id] = true;
  out.push({
    type: "tool_execution_end",
    toolCallId: id,
    isError: failed,
    result: toolContent(toolOutputText(update)),
    detail: acpToolDetail(update),
    status: failed ? "failed" : "completed",
  });
}

/**
 * Project one `session/update` payload into pi events.
 * Unknown `sessionUpdate` values produce nothing so a newer Grok can add
 * notifications without breaking the transcript.
 */
export function mapSessionUpdate(update: Json, state: StreamState): Json[] {
  return stampTurn(mapUpdate(update, state), state.turnId);
}

function mapUpdate(update: Json, state: StreamState): Json[] {
  const out: Json[] = [];
  const sessionUpdate = asString(update.sessionUpdate) ?? "";
  switch (sessionUpdate) {
    case "agent_message_chunk":
      appendText(state, asString(update.messageId) ?? "assistant", chunkText(update), out);
      return out;
    case "agent_thought_chunk":
      appendThinking(state, asString(update.messageId) ?? "thought", chunkText(update), out);
      return out;
    case "tool_call":
    case "tool_call_update": {
      const id = asString(update.toolCallId);
      if (!id) return out;
      const status = asString(update.status) ?? (sessionUpdate === "tool_call" ? "pending" : "");
      if (status === "completed") endTool(state, id, update, false, out);
      else if (status === "failed") endTool(state, id, update, true, out);
      else {
        startTool(state, id, update, out);
        updateTool(state, id, update, out);
      }
      return out;
    }
    case "plan":
      // Every plan update is the whole list (ACP v1 agent-plan).
      out.push({ type: "todo_update", items: todosFromAcpPlan(update) });
      return out;
    default:
      return out;
  }
}

/** Close the open pi turn. `stopReason` is the ACP prompt result. */
export function closeTurn(state: StreamState, stopReason: string | null, errorMessage?: string, code?: ErrorCode): Json[] {
  const out: Json[] = [];
  ensureTurn(state, out);
  // A call still open when the turn closes was cut off: abort, cancel, or an early end.
  for (const id of Object.keys(state.toolStarted)) {
    if (state.toolEnded[id]) continue;
    state.toolEnded[id] = true;
    out.push({
      type: "tool_execution_end",
      toolCallId: id,
      isError: true,
      result: toolContent(""),
      status: "canceled",
    });
  }
  const failed = stopReason === "cancelled" || stopReason === "refusal";
  const content: Json[] = [];
  const thinking = state.thinkingParts.join("");
  if (thinking) content.push({ type: "thinking", thinking });
  if (state.assistantText) content.push({ type: "text", text: state.assistantText });
  const assistant: Json = {
    role: "assistant",
    stopReason: failed ? "error" : "stop",
    content,
  };
  const message = errorMessage || (stopReason === "cancelled" ? "The turn was cancelled." : stopReason === "refusal" ? "The CLI refused the turn." : "");
  if (message && failed) assistant.errorMessage = message;
  out.push({ type: "message_end", message: assistant });
  out.push({ type: "turn_end" });
  const agentEnd: Json = { type: "agent_end" };
  if (failed) agentEnd.messages = [assistant];
  markOutcome(agentEnd, stopReason === "cancelled" ? "canceled" : failed ? "failed" : "completed", message, code);
  out.push(agentEnd);
  state.turnOpen = false;
  state.assistantText = "";
  state.thinkingParts = [];
  state.textIndexByKey = {};
  state.thinkingIndexByKey = {};
  state.toolStarted = {};
  state.toolEnded = {};
  return stampTurn(out, state.turnId);
}

export function approvalFromPermission(id: string | number, params: Json, agentName = "The agent"): ApprovalView {
  const tool = asObject(params.toolCall) ?? {};
  const shape = approvalShape(tool);
  const title = asString(tool.title);
  const command = commandText(tool);
  const detail = [command, locationText(tool), title && title !== shape.title && title !== command ? title : "", toolOutputText(tool)]
    .filter(Boolean)
    .join("\n");
  const toolDetail = acpToolDetail(tool);
  return {
    id: String(id),
    kind: shape.kind,
    title: shape.title,
    detail: bounded(detail || `${agentName} wants to use a tool.`, 4_000),
    ...(toolDetail.kind !== "other" ? { toolDetail } : {}),
  };
}

const ACTION_KINDS: Record<string, Pick<ApprovalAction, "behavior" | "variant" | "label">> = {
  allow_once: { behavior: "allow", variant: "primary", label: "Allow once" },
  allow_always: { behavior: "allow", variant: "secondary", label: "Always allow" },
  reject_once: { behavior: "deny", variant: "danger", label: "Deny" },
  reject_always: { behavior: "deny", variant: "danger", label: "Always deny" },
};

/**
 * Card buttons for session/request_permission, one per option kind the agent
 * offered (the id is the kind; the answer maps it back to that optionId).
 * `sessionAlways` adds Pitex's session-only "always_allow" when the agent
 * offers no allow_always of its own (agy).
 */
export function permissionActions(options: PermissionOption[], sessionAlways: boolean): ApprovalAction[] {
  const actions: ApprovalAction[] = [];
  for (const kind of Object.keys(ACTION_KINDS)) {
    const option = options.find((item) => item.kind === kind);
    if (!option) continue;
    const shape = ACTION_KINDS[kind];
    actions.push({ id: kind, ...shape, label: option.name?.trim() || shape.label });
  }
  if (sessionAlways && !options.some((option) => option.kind === "allow_always")) {
    actions.splice(1, 0, { id: "always_allow", label: "Always allow (this chat)", behavior: "allow", variant: "secondary" });
  }
  return actions;
}

/** The ACP result for a card button: the offered option of that kind, else the legacy decision. */
export function permissionResultForAction(actionId: string | undefined, decision: string | undefined, options: PermissionOption[]): Json {
  const match = actionId ? options.find((option) => option.kind === actionId) : undefined;
  if (match) return { outcome: { outcome: "selected", optionId: match.optionId } };
  return permissionResult(actionId === "always_allow" ? "always_allow" : decision, options);
}

/**
 * Same tool and same command or path. Keys the session-only "Always allow" fallback.
 * Empty when the request names neither, so one grant can never cover every call of a tool.
 */
export function approvalKey(params: Json): string {
  const tool = asObject(params.toolCall) ?? {};
  const command = commandText(tool);
  const path = locationText(tool) || asString(toolArgs(tool).path) || "";
  if (!command && !path) return "";
  return JSON.stringify([toolName(tool), command, path]);
}

export function permissionOptions(params: Json): PermissionOption[] {
  const options = Array.isArray(params.options) ? params.options : [];
  const parsed: PermissionOption[] = [];
  for (const raw of options) {
    const row = asObject(raw);
    const optionId = asString(row?.optionId);
    const kind = asString(row?.kind);
    if (!optionId || !kind) continue;
    const name = asString(row?.name);
    parsed.push(name ? { optionId, kind, name } : { optionId, kind });
  }
  return parsed;
}

export type ApprovalDecision = "approve" | "deny" | "always_allow";

export function normalizeDecision(decision: string | undefined): ApprovalDecision {
  if (decision === "approve" || decision === "always_allow" || decision === "deny") return decision;
  return "deny";
}

/**
 * ACP permission result. The selected `optionId` is one the agent offered.
 * `allow_always` is preferred for "Always allow"; `allow_once` is the
 * fallback when the agent did not offer a remembered grant. A deny with no
 * reject option becomes `cancelled`, which ACP defines as "do not run it".
 */
export function permissionResult(decision: string | undefined, options: PermissionOption[]): Json {
  const choice = normalizeDecision(decision);
  const order =
    choice === "approve"
      ? ["allow_once", "allow_always"]
      : choice === "always_allow"
        ? ["allow_always", "allow_once"]
        : ["reject_once", "reject_always"];
  for (const kind of order) {
    const match = options.find((option) => option.kind === kind);
    if (match) return { outcome: { outcome: "selected", optionId: match.optionId } };
  }
  return { outcome: { outcome: "cancelled" } };
}

export function cancelledPermission(): Json {
  return { outcome: { outcome: "cancelled" } };
}

function addModel(catalog: AcpCatalog, model: AcpModel): void {
  const existing = catalog.models.find((item) => item.id === model.id);
  if (!existing) {
    catalog.models.push(model);
    return;
  }
  if (model.name && model.name !== model.id) existing.name = model.name;
  if (model.efforts.length > 0) existing.efforts = model.efforts;
  if (model.isDefault) existing.isDefault = true;
}

function effortNames(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  const names: string[] = [];
  for (const item of value) {
    if (typeof item === "string" && item.length > 0) {
      names.push(item);
      continue;
    }
    const row = asObject(item);
    const name = asString(row?.id) ?? asString(row?.value) ?? asString(row?.reasoningEffort) ?? asString(row?.name);
    if (name) names.push(name);
  }
  return names;
}

function modelFromEntry(row: Json, provider: string): AcpModel | null {
  const id = asString(row.modelId) ?? asString(row.id) ?? asString(row.value);
  if (!id) return null;
  const meta = asObject(row._meta);
  const efforts = [
    ...effortNames(row.reasoningEfforts),
    ...effortNames(row.supportedReasoningEfforts),
    ...effortNames(meta?.reasoningEfforts),
  ];
  return {
    id,
    provider,
    name: asString(row.name) ?? asString(row.displayName) ?? asString(row.label) ?? id,
    isDefault: row.isDefault === true || row.selected === true || row.default === true,
    efforts,
  };
}

function modelRows(value: unknown): Json[] {
  if (Array.isArray(value)) {
    return value.map((item) => asObject(item)).filter((item): item is Json => item !== null);
  }
  const obj = asObject(value);
  if (!obj) return [];
  for (const key of ["availableModels", "models", "data"]) {
    if (Array.isArray(obj[key])) return modelRows(obj[key]);
  }
  return [];
}

function selectValues(options: unknown): { value: string; name: string }[] {
  if (!Array.isArray(options)) return [];
  const values: { value: string; name: string }[] = [];
  for (const raw of options) {
    const row = asObject(raw);
    if (!row) continue;
    if (Array.isArray(row.options)) {
      values.push(...selectValues(row.options));
      continue;
    }
    const value = asString(row.value) ?? asString(row.id) ?? asString(row.modelId);
    if (!value) continue;
    values.push({ value, name: asString(row.name) ?? asString(row.label) ?? value });
  }
  return values;
}

function absorbConfigOptions(options: unknown, catalog: AcpCatalog, provider: string): void {
  if (!Array.isArray(options)) return;
  for (const raw of options) {
    const row = asObject(raw);
    if (!row) continue;
    const category = asString(row.category);
    const id = asString(row.id);
    if (category === "model" || id === "model") {
      if (id) catalog.modelConfigId = id;
      const current = typeof row.currentValue === "string" ? row.currentValue : null;
      for (const option of selectValues(row.options)) {
        addModel(catalog, {
          id: option.value,
          provider,
          name: option.name,
          isDefault: option.value === current,
          efforts: [],
        });
      }
      if (current) catalog.currentModelId = current;
    }
    if (category === "thought_level" && id) {
      catalog.effortConfigId = id;
      catalog.efforts = selectValues(row.options).map((option) => option.value);
      if (typeof row.currentValue === "string") catalog.currentEffort = row.currentValue;
    }
    if (category === "mode" && id) {
      catalog.modeConfigId = id;
      if (typeof row.currentValue === "string") catalog.currentMode = row.currentValue;
      catalog.modes = selectValues(row.options).map((option) => ({ id: option.value, label: option.name }));
    }
  }
}

function absorbFlatSessionConfig(options: unknown, catalog: AcpCatalog, provider: string): void {
  if (!Array.isArray(options)) return;
  for (const raw of options) {
    const row = asObject(raw);
    if (!row) continue;
    const category = asString(row.category);
    const id = asString(row.id) ?? asString(row.value);
    if (!id) continue;
    const name = asString(row.label) ?? asString(row.name) ?? id;
    if (category === "model") {
      addModel(catalog, {
        id,
        provider,
        name,
        isDefault: row.selected === true,
        efforts: effortNames(row.reasoningEfforts),
      });
      if (row.selected === true) catalog.currentModelId = id;
    } else if (category === "mode" || category === "thought_level") {
      if (!catalog.efforts.includes(id)) catalog.efforts.push(id);
      if (row.selected === true) catalog.currentEffort = id;
      if (category === "mode") catalog.effortUsesMode = true;
    }
  }
}

function absorbModelState(state: Json | null, catalog: AcpCatalog, provider: string): void {
  if (!state) return;
  for (const row of modelRows(state.availableModels ?? state.models ?? state)) {
    const model = modelFromEntry(row, provider);
    if (model) addModel(catalog, model);
  }
  const current = asString(state.currentModelId) ?? asString(state.defaultModelId);
  if (current) catalog.currentModelId = current;
  const efforts = effortNames(state.reasoningEfforts);
  if (efforts.length > 0 && catalog.efforts.length === 0) catalog.efforts = efforts;
}

/**
 * Models and reasoning efforts from initialize, session/new, or `x.ai/models/list`.
 * A `config` profile reads only config options: agy's `models.availableModels`
 * folds the effort into the id (`gemini-4-neon-high`) and would mix two id schemes.
 */
export function catalogFromAcp(payload: unknown, profile: Pick<AcpProfile, "id" | "modelSource">): AcpCatalog {
  const catalog = emptyCatalog();
  const root = asObject(payload);
  if (!root) return catalog;
  const provider = profile.id;
  absorbConfigOptions(root.configOptions, catalog, provider);
  const modes = asObject(root.modes);
  if (!catalog.currentMode && asString(modes?.currentModeId)) catalog.currentMode = asString(modes?.currentModeId);
  if (asString(root.currentModeId)) catalog.currentMode = asString(root.currentModeId);
  const availableModes = modes?.availableModes;
  if (catalog.modes.length === 0 && Array.isArray(availableModes)) {
    catalog.modes = availableModes.flatMap((raw) => {
      const row = asObject(raw);
      const id = asString(row?.id);
      return id ? [{ id, label: asString(row?.name) ?? id }] : [];
    });
  }
  if (profile.modelSource === "config") return catalog;
  const modelObject = asObject(root.models);
  for (const row of modelRows(modelObject ?? root.models ?? root.availableModels ?? root.data)) {
    const model = modelFromEntry(row, provider);
    if (model) addModel(catalog, model);
  }
  const current =
    asString(modelObject?.currentModelId) ?? asString(root.currentModelId) ?? asString(root.defaultModelId);
  if (current) catalog.currentModelId = current;
  const meta = asObject(root._meta);
  if (meta) {
    absorbModelState(asObject(meta.modelState), catalog, provider);
    const sessionConfig = asObject(meta["x.ai/sessionConfig"]);
    absorbFlatSessionConfig(sessionConfig?.options, catalog, provider);
  }
  absorbModelState(asObject(root.modelState), catalog, provider);
  if (catalog.efforts.length === 0 && catalog.currentModelId) {
    const currentModel = catalog.models.find((model) => model.id === catalog.currentModelId);
    if (currentModel && currentModel.efforts.length > 0) catalog.efforts = [...currentModel.efforts];
  }
  if (catalog.currentModelId) {
    const match = catalog.models.find((model) => model.id === catalog.currentModelId);
    if (match) match.isDefault = true;
  }
  return catalog;
}

export function visibleModels(models: AcpModel[]): { id: string; provider: string; name: string }[] {
  return models.map(({ id, provider, name }) => ({ id, provider, name }));
}

function appendTimelineText(history: HistoryState, kind: "text" | "thinking", delta: string): void {
  if (!delta) return;
  const last = history.timeline[history.timeline.length - 1];
  if (last && last.kind === kind) {
    last.text += delta;
    return;
  }
  history.timeline.push({ kind, text: delta });
}

function commitTimeline(history: HistoryState): void {
  if (history.timeline.length === 0) return;
  const blocks: Json[] = [];
  const results: Json[] = [];
  for (const item of history.timeline) {
    if (item.kind === "thinking" && item.text) blocks.push({ type: "thinking", thinking: item.text });
    if (item.kind === "text" && item.text) blocks.push({ type: "text", text: item.text });
    if (item.kind === "tool") {
      const tool = history.tools[item.id];
      if (!tool) continue;
      blocks.push({ type: "toolCall", id: item.id, name: tool.name, arguments: tool.args });
      results.push({
        role: "toolResult",
        toolCallId: item.id,
        toolName: tool.name,
        isError: tool.isError || !tool.done,
        content: [{ type: "text", text: tool.output }],
        detail: tool.detail,
        status: tool.isError ? "failed" : tool.done ? "completed" : "canceled",
      });
    }
  }
  if (blocks.length > 0) history.messages.push({ role: "assistant", content: blocks });
  history.messages.push(...results);
  history.timeline = [];
  history.tools = {};
}

export function pushUserMessage(history: HistoryState, text: string): void {
  if (!text) return;
  commitTimeline(history);
  history.messages.push({ role: "user", content: [{ type: "text", text }] });
}

function rememberTool(history: HistoryState, update: Json): void {
  const id = asString(update.toolCallId);
  if (!id) return;
  const output = toolOutputText(update);
  const failed = update.status === "failed";
  const done = failed || update.status === "completed";
  const existing = history.tools[id];
  const detail = acpToolDetail(update);
  if (!existing) {
    history.tools[id] = { name: toolName(update), args: toolArgs(update), output, isError: failed, done, detail };
    history.timeline.push({ kind: "tool", id });
    return;
  }
  // The hosts' live merge: later fields fill in, and an untyped detail never replaces a typed one.
  existing.detail =
    detail.kind === "other" && existing.detail.kind !== "other"
      ? { ...detail, ...existing.detail }
      : { ...existing.detail, ...detail };
  if (output) existing.output = output;
  if (failed) existing.isError = true;
  if (update.status === "completed") existing.isError = false;
  if (done) existing.done = true;
  if (asString(update.name)) existing.name = toolName(update);
  const args = toolArgs(update);
  if (Object.keys(args).length > 0) existing.args = args;
}

/** Fold one session update into the `get_messages` transcript. */
export function recordSessionUpdate(history: HistoryState, update: Json): void {
  switch (update.sessionUpdate) {
    case "user_message_chunk": {
      const text = chunkText(update);
      if (!text) return;
      commitTimeline(history);
      const last = history.messages[history.messages.length - 1];
      if (last && last.role === "user") {
        const content = Array.isArray(last.content) ? last.content : [];
        const block = asObject(content[0]);
        if (block && block.type === "text" && typeof block.text === "string") {
          block.text += text;
          return;
        }
      }
      history.messages.push({ role: "user", content: [{ type: "text", text }] });
      return;
    }
    case "agent_message_chunk":
      appendTimelineText(history, "text", chunkText(update));
      return;
    case "agent_thought_chunk":
      appendTimelineText(history, "thinking", chunkText(update));
      return;
    case "tool_call":
    case "tool_call_update":
      rememberTool(history, update);
      return;
    case "plan":
      history.todos = todosFromAcpPlan(update);
      return;
    default:
      return;
  }
}

export function finishHistoryTurn(history: HistoryState): void {
  commitTimeline(history);
}

export function historyMessages(history: HistoryState): Json[] {
  if (history.timeline.length === 0) return history.messages;
  const copy = createHistory();
  copy.messages = history.messages.map((message) => structuredClone(message));
  copy.timeline = structuredClone(history.timeline);
  copy.tools = structuredClone(history.tools);
  commitTimeline(copy);
  return copy.messages;
}

export interface ScopedText {
  ok: true;
  content: string;
}

export interface ScopedError {
  ok: false;
  error: string;
}

/**
 * Absolute path that stays inside `root` after symlink resolution.
 * Relative paths and paths that escape the project are rejected.
 */
export function resolveInsideProject(root: string, requested: string): string | null {
  if (!requested || requested.includes("\0") || !isAbsolute(requested)) return null;
  let rootReal: string;
  try {
    rootReal = realpathSync(resolve(root));
  } catch {
    return null;
  }
  const candidate = resolve(requested);
  const missing: string[] = [];
  let existing = candidate;
  while (!existsSync(existing)) {
    const parent = dirname(existing);
    if (parent === existing) return null;
    missing.push(existing.slice(parent.length).replace(/^[/\\]/, ""));
    existing = parent;
  }
  let realExisting: string;
  try {
    realExisting = realpathSync(existing);
  } catch {
    return null;
  }
  const rebuilt = missing.reduceRight((dir, name) => resolve(dir, name), realExisting);
  const fromRoot = relative(rootReal, rebuilt);
  if (fromRoot.startsWith("..") || isAbsolute(fromRoot)) return null;
  return rebuilt;
}

function sliceLines(content: string, line: number | undefined, limit: number | undefined): string {
  if (line === undefined && limit === undefined) return content;
  const lines = content.split("\n");
  const start = Math.max(0, (line ?? 1) - 1);
  const end = limit !== undefined ? start + Math.max(0, limit) : lines.length;
  return lines.slice(start, end).join("\n");
}

export function readProjectFile(
  root: string,
  requested: string,
  line?: number,
  limit?: number
): ScopedText | ScopedError {
  const path = resolveInsideProject(root, requested);
  if (!path) return { ok: false, error: "That path is outside this project." };
  try {
    if (!statSync(path).isFile()) return { ok: false, error: "That path is not a file." };
    const content = readFileSync(path, "utf8");
    return { ok: true, content: sliceLines(content, line, limit) };
  } catch (error) {
    const message = error instanceof Error ? error.message : "The file could not be read.";
    return { ok: false, error: message };
  }
}

export function writeProjectFile(root: string, requested: string, content: string): ScopedText | ScopedError {
  const path = resolveInsideProject(root, requested);
  if (!path) return { ok: false, error: "That path is outside this project." };
  try {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, content, "utf8");
    return { ok: true, content: "" };
  } catch (error) {
    const message = error instanceof Error ? error.message : "The file could not be written.";
    return { ok: false, error: message };
  }
}

/** Directory for `terminal/create`. Omitted cwd means the project root. */
export function terminalWorkingDirectory(root: string, cwd: unknown): string | null {
  const requested = typeof cwd === "string" && cwd.length > 0 ? cwd : resolve(root);
  if (!isAbsolute(requested)) return null;
  const path = resolveInsideProject(root, requested);
  if (!path) return null;
  try {
    if (!statSync(path).isDirectory()) return null;
  } catch {
    return null;
  }
  return path;
}
