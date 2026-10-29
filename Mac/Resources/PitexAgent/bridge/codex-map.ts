// Maps Codex app-server notifications and thread items onto the pi RPC
// events the Mac and GTK transcript UIs already render.
//
// Protocol checked against Codex CLI 0.165.0 (`codex app-server generate-ts`):
// item/agentMessage/delta, item/reasoning/summaryTextDelta,
// item/reasoning/textDelta, item/commandExecution/outputDelta,
// item/started, item/completed, turn/started, turn/completed,
// item/commandExecution/requestApproval, item/fileChange/requestApproval,
// execCommandApproval, applyPatchApproval, item/permissions/requestApproval.
//
// Design adapted from T3 Code v0.1.4 (MIT), Copyright (c) 2026 T3 Tools Inc.
// https://github.com/pingdotgg/t3code — CodexAdapterV2 event projection.
// This file is original Pitex code written against that protocol, not a copy.

export type Json = Record<string, unknown>;

export interface StreamState {
  nextContentIndex: number;
  textIndexByItem: Record<string, number>;
  thinkingIndexByItem: Record<string, number>;
  /** Item ids that already emitted summary thinking, so raw text is not doubled. */
  thinkingSummarySeen: Record<string, true>;
  toolStarted: Record<string, true>;
  textByItem: Record<string, string>;
  thinkingParts: string[];
  assistantText: string;
  turnOpen: boolean;
}

export function createStreamState(): StreamState {
  return {
    nextContentIndex: 0,
    textIndexByItem: {},
    thinkingIndexByItem: {},
    thinkingSummarySeen: {},
    toolStarted: {},
    textByItem: {},
    thinkingParts: [],
    assistantText: "",
    turnOpen: false,
  };
}

export interface ApprovalView {
  id: string;
  method: string;
  kind: "command" | "file" | "permissions";
  title: string;
  detail: string;
}

export type ApprovalDecision = "approve" | "deny" | "always_allow";

export interface RpcReply {
  result?: Json;
  error?: { code: number; message: string };
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
  out.push({ type: "agent_start" });
  out.push({ type: "message_start", message: { role: "assistant" } });
}

function commandToolArgs(item: Json): Json {
  return { command: asString(item.command) ?? "" };
}

function fileToolArgs(item: Json): Json {
  const changes = Array.isArray(item.changes) ? item.changes : [];
  const first = asObject(changes[0]);
  const path = asString(first?.path) ?? "";
  return { path, changes };
}

export function fileChangeDetail(item: Json): string {
  const changes = Array.isArray(item.changes) ? item.changes : [];
  return changes
    .map((change) => {
      const row = asObject(change);
      if (!row) return "";
      const kind = asObject(row.kind);
      const kindName = asString(kind?.type) ?? "edit";
      const path = asString(row.path) ?? "";
      const diff = asString(row.diff) ?? "";
      return [kindName, path, diff].filter(Boolean).join("\n");
    })
    .filter(Boolean)
    .join("\n\n");
}

function toolName(item: Json): string {
  switch (item.type) {
    case "commandExecution":
      return "bash";
    case "fileChange":
      return "edit";
    case "mcpToolCall":
      return asString(item.tool) ?? "mcp";
    case "dynamicToolCall":
      return asString(item.tool) ?? "tool";
    case "webSearch":
      return "web_search";
    default:
      return asString(item.type) ?? "tool";
  }
}

function toolArgs(item: Json): Json {
  switch (item.type) {
    case "commandExecution":
      return commandToolArgs(item);
    case "fileChange":
      return fileToolArgs(item);
    case "mcpToolCall":
    case "dynamicToolCall":
      return { name: toolName(item), arguments: item.arguments ?? {} };
    default:
      return { name: toolName(item) };
  }
}

function toolOutput(item: Json): string {
  if (item.type === "fileChange") return fileChangeDetail(item);
  if (typeof item.aggregatedOutput === "string") return item.aggregatedOutput;
  const error = asObject(item.error);
  if (asString(error?.message)) return error?.message as string;
  if (typeof item.result === "string") return item.result;
  return "";
}

function toolFailed(item: Json): boolean {
  const status = asString(item.status);
  if (status === "failed" || status === "declined") return true;
  if (typeof item.exitCode === "number" && item.exitCode !== 0) return true;
  if (item.success === false) return true;
  return false;
}

function startTool(state: StreamState, item: Json, out: Json[]): void {
  const id = asString(item.id);
  if (!id || state.toolStarted[id]) return;
  state.toolStarted[id] = true;
  ensureTurn(state, out);
  out.push({
    type: "tool_execution_start",
    toolCallId: id,
    toolName: toolName(item),
    args: toolArgs(item),
  });
}

function finishTool(state: StreamState, item: Json, out: Json[]): void {
  const id = asString(item.id);
  if (!id) return;
  startTool(state, item, out);
  const text = toolOutput(item);
  out.push({
    type: "tool_execution_end",
    toolCallId: id,
    isError: toolFailed(item),
    result: toolContent(text),
  });
}

function appendText(state: StreamState, itemId: string, delta: string, out: Json[]): void {
  if (!delta) return;
  ensureTurn(state, out);
  let index = state.textIndexByItem[itemId];
  if (index === undefined) {
    index = state.nextContentIndex++;
    state.textIndexByItem[itemId] = index;
    state.textByItem[itemId] = "";
    out.push(textEvent("text_start", index));
  }
  state.textByItem[itemId] = (state.textByItem[itemId] ?? "") + delta;
  state.assistantText += delta;
  out.push(textEvent("text_delta", index, delta));
}

function appendThinking(state: StreamState, itemId: string, delta: string, out: Json[]): void {
  if (!delta) return;
  ensureTurn(state, out);
  let index = state.thinkingIndexByItem[itemId];
  if (index === undefined) {
    index = state.nextContentIndex++;
    state.thinkingIndexByItem[itemId] = index;
    out.push(textEvent("thinking_start", index));
  }
  state.thinkingParts.push(delta);
  out.push(textEvent("thinking_delta", index, delta));
}

function closeTurn(state: StreamState, turn: Json | null, out: Json[]): void {
  ensureTurn(state, out);
  const error = asObject(turn?.error);
  const failed = asString(turn?.status) === "failed" || asString(turn?.status) === "interrupted";
  const message = asString(error?.message);
  const content: Json[] = [];
  const thinking = state.thinkingParts.join("");
  if (thinking) content.push({ type: "thinking", thinking });
  if (state.assistantText) content.push({ type: "text", text: state.assistantText });
  const assistant: Json = {
    role: "assistant",
    stopReason: failed ? "error" : "stop",
    content,
  };
  if (message) assistant.errorMessage = message;
  out.push({ type: "message_end", message: assistant });
  out.push({ type: "turn_end" });
  const agentEnd: Json = { type: "agent_end" };
  if (failed) agentEnd.messages = [assistant];
  out.push(agentEnd);
  state.turnOpen = false;
  state.assistantText = "";
  state.thinkingParts = [];
  state.textIndexByItem = {};
  state.thinkingIndexByItem = {};
  state.thinkingSummarySeen = {};
  state.textByItem = {};
}

/**
 * Project one app-server notification into zero or more pi events.
 * Unknown methods produce nothing so a newer Codex can add notifications
 * without breaking the transcript.
 */
export function mapNotification(method: string, params: Json, state: StreamState): Json[] {
  const out: Json[] = [];
  const item = asObject(params.item);
  switch (method) {
    case "turn/started":
      ensureTurn(state, out);
      return out;
    case "item/agentMessage/delta":
      appendText(state, asString(params.itemId) ?? "assistant", asString(params.delta) ?? "", out);
      return out;
    case "item/reasoning/summaryTextDelta":
      state.thinkingSummarySeen[asString(params.itemId) ?? "thinking"] = true;
      appendThinking(state, asString(params.itemId) ?? "thinking", asString(params.delta) ?? "", out);
      return out;
    case "item/reasoning/textDelta": {
      const itemId = asString(params.itemId) ?? "thinking";
      if (state.thinkingSummarySeen[itemId]) return out;
      appendThinking(state, itemId, asString(params.delta) ?? "", out);
      return out;
    }
    case "item/commandExecution/outputDelta": {
      const itemId = asString(params.itemId);
      const delta = asString(params.delta) ?? "";
      if (!itemId || !delta) return out;
      startTool(state, { type: "commandExecution", id: itemId, command: "" }, out);
      out.push({
        type: "tool_execution_update",
        toolCallId: itemId,
        partialResult: toolContent(delta),
      });
      return out;
    }
    case "item/started":
      if (!item) return out;
      if (item.type === "commandExecution" || item.type === "fileChange" || item.type === "mcpToolCall" || item.type === "dynamicToolCall") {
        startTool(state, item, out);
      }
      return out;
    case "item/completed":
      if (!item) return out;
      if (item.type === "agentMessage") {
        const text = asString(item.text) ?? "";
        const itemId = asString(item.id) ?? "assistant";
        const already = state.textByItem[itemId] ?? "";
        if (text && text !== already) {
          const rest = text.startsWith(already) ? text.slice(already.length) : text;
          if (!already) appendText(state, itemId, rest, out);
          else if (rest) appendText(state, itemId, rest, out);
        }
        return out;
      }
      if (item.type === "reasoning") {
        const itemId = asString(item.id) ?? "thinking";
        if (state.thinkingIndexByItem[itemId] === undefined) {
          const summary = Array.isArray(item.summary) ? item.summary.filter((part) => typeof part === "string").join("") : "";
          const content = Array.isArray(item.content) ? item.content.filter((part) => typeof part === "string").join("") : "";
          appendThinking(state, itemId, summary || content, out);
        }
        return out;
      }
      if (
        item.type === "commandExecution" ||
        item.type === "fileChange" ||
        item.type === "mcpToolCall" ||
        item.type === "dynamicToolCall" ||
        item.type === "webSearch"
      ) {
        finishTool(state, item, out);
      }
      return out;
    case "turn/completed":
      closeTurn(state, asObject(params.turn), out);
      return out;
    default:
      return out;
  }
}

function commandLabel(params: Json): string {
  if (Array.isArray(params.command)) {
    const parts = params.command.filter((part): part is string => typeof part === "string" && part.length > 0);
    if (parts.length > 0) return parts.join(" ");
  }
  return asString(params.command) || asString(params.reason) || "Codex wants to run a command.";
}

export function approvalFromServerRequest(
  method: string,
  id: string | number,
  params: Json
): ApprovalView | null {
  const requestId = String(id);
  switch (method) {
    case "item/commandExecution/requestApproval":
    case "execCommandApproval":
      return {
        id: requestId,
        method,
        kind: "command",
        title: "Run command",
        detail: commandLabel(params),
      };
    case "item/fileChange/requestApproval":
    case "applyPatchApproval":
      return {
        id: requestId,
        method,
        kind: "file",
        title: "Edit files",
        detail:
          asString(params.reason) ||
          asString(params.grantRoot) ||
          "Codex wants to change files in this project.",
      };
    case "item/permissions/requestApproval":
      return {
        id: requestId,
        method,
        kind: "permissions",
        title: "Allow permissions",
        detail: asString(params.reason) || "Codex wants extra permissions for this turn.",
      };
    default:
      return null;
  }
}

export function normalizeDecision(decision: string | undefined): ApprovalDecision {
  if (decision === "approve" || decision === "always_allow" || decision === "deny") return decision;
  return "deny";
}

/** JSON-RPC result for a Pitex approve / deny / always-allow choice. */
export function approvalRpcReply(method: string, decision: string | undefined, params: Json): RpcReply {
  const choice = normalizeDecision(decision);
  if (method === "item/permissions/requestApproval") {
    if (choice === "deny") {
      return { error: { code: -32000, message: "The user denied the permission request." } };
    }
    const requested = asObject(params.permissions) ?? {};
    return {
      result: {
        permissions: {
          ...(requested.network ? { network: requested.network } : {}),
          ...(requested.fileSystem ? { fileSystem: requested.fileSystem } : {}),
        },
        scope: choice === "always_allow" ? "session" : "turn",
      },
    };
  }
  if (method === "execCommandApproval" || method === "applyPatchApproval") {
    const legacy =
      choice === "approve" ? "approved" : choice === "always_allow" ? "approved_for_session" : { denied: { rejection: "The user denied the request." } };
    return { result: { decision: legacy } };
  }
  const modern = choice === "approve" ? "accept" : choice === "always_allow" ? "acceptForSession" : "decline";
  return { result: { decision: modern } };
}

function userText(item: Json): string {
  const content = Array.isArray(item.content) ? item.content : [];
  return content
    .map((part) => {
      const row = asObject(part);
      return row?.type === "text" && typeof row.text === "string" ? row.text : "";
    })
    .filter(Boolean)
    .join("\n");
}

function pushToolMessage(messages: Json[], blocks: Json[], results: Json[], item: Json): void {
  const id = asString(item.id) ?? `tool-${messages.length}-${blocks.length}`;
  blocks.push({
    type: "toolCall",
    id,
    name: toolName(item),
    arguments: toolArgs(item),
  });
  results.push({
    role: "toolResult",
    toolCallId: id,
    isError: toolFailed(item),
    content: [{ type: "text", text: toolOutput(item) }],
  });
}

/** Rebuild a pi `get_messages` payload from Codex turns (or a flat item list). */
export function itemsToPiMessages(items: unknown[]): Json[] {
  const messages: Json[] = [];
  let blocks: Json[] = [];
  let results: Json[] = [];
  const flush = () => {
    if (blocks.length > 0) messages.push({ role: "assistant", content: blocks });
    messages.push(...results);
    blocks = [];
    results = [];
  };
  for (const raw of items) {
    const item = asObject(raw);
    if (!item) continue;
    switch (item.type) {
      case "userMessage": {
        flush();
        const text = userText(item);
        if (text) messages.push({ role: "user", content: [{ type: "text", text }] });
        break;
      }
      case "reasoning": {
        const summary = Array.isArray(item.summary) ? item.summary.filter((part) => typeof part === "string").join("\n") : "";
        const content = Array.isArray(item.content) ? item.content.filter((part) => typeof part === "string").join("\n") : "";
        const thinking = summary || content;
        if (thinking) blocks.push({ type: "thinking", thinking });
        break;
      }
      case "agentMessage": {
        const text = asString(item.text);
        if (text) blocks.push({ type: "text", text });
        break;
      }
      case "commandExecution":
      case "fileChange":
      case "mcpToolCall":
      case "dynamicToolCall":
      case "webSearch":
        pushToolMessage(messages, blocks, results, item);
        break;
      default:
        break;
    }
  }
  flush();
  return messages;
}

export function turnsToPiMessages(turns: unknown[]): Json[] {
  const items: unknown[] = [];
  for (const raw of turns) {
    const turn = asObject(raw);
    const turnItems = Array.isArray(turn?.items) ? turn.items : [];
    items.push(...turnItems);
  }
  return itemsToPiMessages(items);
}

export interface ModelDescriptor {
  id: string;
  provider: string;
  name: string;
}

export interface ListedModel extends ModelDescriptor {
  hidden: boolean;
  isDefault: boolean;
  efforts: string[];
  defaultEffort: string;
}

export function modelsFromList(payload: unknown): ListedModel[] {
  const root = asObject(payload);
  const data = Array.isArray(root?.data) ? root.data : [];
  const models: ListedModel[] = [];
  for (const raw of data) {
    const row = asObject(raw);
    const id = asString(row?.id);
    if (!row || !id) continue;
    const efforts = Array.isArray(row.supportedReasoningEfforts)
      ? row.supportedReasoningEfforts
          .map((option) => asString(asObject(option)?.reasoningEffort))
          .filter((effort): effort is string => Boolean(effort))
      : [];
    models.push({
      id,
      provider: "codex",
      name: asString(row.displayName) ?? id,
      hidden: row.hidden === true,
      isDefault: row.isDefault === true,
      efforts,
      defaultEffort: asString(row.defaultReasoningEffort) ?? efforts[0] ?? "medium",
    });
  }
  return models;
}

export function visibleModels(models: ListedModel[]): ModelDescriptor[] {
  return models.filter((model) => !model.hidden).map(({ id, provider, name }) => ({ id, provider, name }));
}
