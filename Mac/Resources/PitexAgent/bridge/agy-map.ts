// Maps Antigravity CLI (`agy`) stream-json events onto the pi RPC events
// the Mac and GTK transcript UIs already render.
//
// Protocol checked against the public docs on 2026-10-29, for the CLI
// whose changelog head that day is 1.4.10:
//   https://antigravity.google/docs/cli/headless/
//   https://antigravity.google/docs/cli/reference/
//   https://antigravity.google/docs/cli/modes/
//   https://antigravity.google/docs/permissions/
//   https://antigravity.google/docs/install/
//   https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md
//
// Stream shape (headless, "Streaming JSON" and "Stream prompts from stdin"):
//   stdout NDJSON events: init, step_update, result
//   stdin user event: {"event":"user","message":{"content":"<text>"}}
//   step_type values the docs name: user_input, agent_response, tool, checkpoint
//   tool_info: {name, parameters, output, error:{type, message}}
//   subagent_info.subagents[]: type_name, role, conversation_id, log_uri, workspace_uris
//   result status: SUCCESS, ERROR, CANCELED, INTERRUPTED, INVALID, WAITING, RUNNING
//   usage: input_tokens, output_tokens, thinking_tokens, cache_read_tokens, total_tokens
//
// The published stream has no reasoning-text field. thinking_tokens is only
// a count on usage. This mapper does not invent thinking deltas.
//
// Headless mode has no interactive permission prompt (headless, "Permissions
// in headless mode"). control_request and control_response end the session
// with ERROR and exit 2. This file therefore never builds a stdin reply for
// Approve / Always allow / Deny.
//
// Original Pitex mapping against those docs. It is not a copy of T3 Code.

export type Json = Record<string, unknown>;

export const AGY_MISSING =
  "Antigravity CLI was not found. Install the official Antigravity CLI, then run `agy` and sign in.";
export const AGY_SIGNED_OUT =
  "Antigravity CLI is not logged in. Run `agy` in a terminal and sign in, then try again.";
export const AGY_KEYRING_LOCKED =
  "Antigravity CLI could not read the system keyring. Unlock it, run `agy` in a terminal and sign in, then try again.";

/**
 * Headless stream-json cannot accept a permission reply. Pitex does not
 * pass `--dangerously-skip-permissions`. Workspace reads and writes follow
 * the CLI's own default policy; other tools that need approval are denied
 * by the CLI.
 */
export const AGY_PERMISSION_LIMIT =
  "Antigravity CLI has no interactive permission prompt in stream-json mode. Pitex does not auto-approve tools. Shell commands and other actions that need approval are denied unless you allow them in `agy`. Reading and writing files in this project follows the CLI's default policy. Approve, Always allow, and Deny in Pitex cannot be sent to this CLI.";

export const AGY_ALWAYS_PROCEED_NOTICE =
  "Antigravity CLI reported permission mode always-proceed from its own settings. Pitex did not pass --dangerously-skip-permissions. Change that mode in `agy` if tools should not run on their own.";

/** CLI reference: effort runs from low through max. The headless page names low, medium, and high. */
export const AGY_EFFORTS = ["low", "medium", "high", "max"] as const;

export interface AgyUsage {
  input: number;
  output: number;
  thinking: number;
  cacheRead: number;
  total: number;
}

export interface AgyToolRecord {
  name: string;
  args: Json;
  output: string;
  isError: boolean;
  finished: boolean;
}

export interface AgyStreamState {
  nextContentIndex: number;
  textByStep: Record<string, string>;
  textIndexByStep: Record<string, number>;
  assistantText: string;
  tools: Record<string, AgyToolRecord>;
  turnOpen: boolean;
}

export interface AgyMapped {
  events: Json[];
  /** Completed pi messages for this line. Set when a turn reaches a terminal result. */
  messages: Json[];
  conversationId: string | null;
  permissionMode: string | null;
  /** Cumulative usage from a terminal result. Step usage is per-step and is ignored. */
  usage: AgyUsage | null;
  terminal: boolean;
  failure: string | null;
}

export function createAgyStreamState(): AgyStreamState {
  return {
    nextContentIndex: 0,
    textByStep: {},
    textIndexByStep: {},
    assistantText: "",
    tools: {},
    turnOpen: false,
  };
}

export function emptyMapped(): AgyMapped {
  return {
    events: [],
    messages: [],
    conversationId: null,
    permissionMode: null,
    usage: null,
    terminal: false,
    failure: null,
  };
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

function ensureTurn(state: AgyStreamState, out: Json[]): void {
  if (state.turnOpen) return;
  state.turnOpen = true;
  out.push({ type: "agent_start" });
  out.push({ type: "message_start", message: { role: "assistant" } });
}

function appendText(state: AgyStreamState, stepKey: string, delta: string, out: Json[]): void {
  if (!delta) return;
  const already = state.textByStep[stepKey] ?? "";
  let piece = delta;
  if (already) {
    if (delta === already) return;
    if (delta.startsWith(already)) piece = delta.slice(already.length);
  }
  if (!piece) return;
  ensureTurn(state, out);
  state.textByStep[stepKey] = already + piece;
  let index = state.textIndexByStep[stepKey];
  if (index === undefined) {
    index = state.nextContentIndex++;
    state.textIndexByStep[stepKey] = index;
    out.push(textEvent("text_start", index));
  }
  state.assistantText += piece;
  out.push(textEvent("text_delta", index, piece));
}

function presentTool(name: string, parameters: Json): { toolName: string; args: Json } {
  // The headless page's tool example is run_command with parameters.CommandLine.
  if (name === "run_command") {
    const command = asString(parameters.CommandLine) ?? "";
    return { toolName: "bash", args: { command, ...parameters } };
  }
  return { toolName: name || "tool", args: parameters };
}

function startTool(state: AgyStreamState, id: string, toolName: string, args: Json, out: Json[]): void {
  if (state.tools[id]) return;
  state.tools[id] = { name: toolName, args, output: "", isError: false, finished: false };
  ensureTurn(state, out);
  out.push({
    type: "tool_execution_start",
    toolCallId: id,
    toolName,
    args,
  });
}

function finishTool(
  state: AgyStreamState,
  id: string,
  toolName: string,
  args: Json,
  output: string,
  isError: boolean,
  out: Json[]
): void {
  if (state.tools[id]?.finished) return;
  startTool(state, id, toolName, args, out);
  const tool = state.tools[id];
  if (!tool) return;
  tool.name = toolName || tool.name;
  if (Object.keys(args).length > 0) tool.args = args;
  tool.output = output;
  tool.isError = isError;
  tool.finished = true;
  out.push({
    type: "tool_execution_end",
    toolCallId: id,
    isError,
    result: toolContent(output),
  });
}

function toolPayload(info: Json | null, toolName: string): { args: Json; output: string; isError: boolean; name: string } {
  const parameters = asObject(info?.parameters) ?? {};
  const presented = presentTool(toolName, parameters);
  const error = asObject(info?.error);
  const message = asString(error?.message);
  const output = typeof info?.output === "string" ? info.output : "";
  const payload = { name: presented.toolName, args: presented.args, output, isError: false };
  if (message && output) return { ...payload, output: `${output}\n${message}`, isError: true };
  if (message) return { ...payload, output: message, isError: true };
  return payload;
}

function mapToolStep(state: AgyStreamState, step: Json, out: Json[]): void {
  const index = typeof step.step_index === "number" ? String(step.step_index) : "tool";
  const info = asObject(step.tool_info);
  const canonical = asString(info?.name) ?? asString(step.tool_name) ?? "tool";
  const payload = toolPayload(info, canonical);
  const id = `agy-tool-${index}`;
  const active = step.state === "ACTIVE";
  if (active) {
    startTool(state, id, payload.name, payload.args, out);
    return;
  }
  finishTool(state, id, payload.name, payload.args, payload.output, payload.isError, out);
}

function mapSubagents(state: AgyStreamState, step: Json, out: Json[]): void {
  const info = asObject(step.subagent_info);
  const rows = Array.isArray(info?.subagents) ? info.subagents : [];
  const index = typeof step.step_index === "number" ? String(step.step_index) : "sub";
  rows.forEach((raw, offset) => {
    const row = asObject(raw);
    if (!row) return;
    const id = `agy-sub-${index}-${offset}`;
    const name = asString(row.type_name) ?? "subagent";
    // log_uri and workspace_uris are local paths. They stay off the transcript.
    const args: Json = {};
    const role = asString(row.role);
    const conversationId = asString(row.conversation_id);
    if (role) args.role = role;
    if (conversationId) args.conversationId = conversationId;
    const summary = [name, role, conversationId].filter(Boolean).join(" ");
    if (step.state === "ACTIVE") startTool(state, id, name, args, out);
    else finishTool(state, id, name, args, summary, false, out);
  });
}

/**
 * `result.response` is the whole turn's free text. Streamed `text_delta`
 * fragments are already in `assistantText`, so a matching response must
 * not be appended again. A response that extends the streamed text adds
 * only the suffix.
 */
function appendResponse(state: AgyStreamState, response: string, out: Json[]): void {
  if (!response) return;
  const have = state.assistantText;
  if (!have) {
    appendText(state, "result", response, out);
    return;
  }
  if (response === have || have.startsWith(response)) return;
  if (response.startsWith(have)) {
    appendText(state, "result", response.slice(have.length), out);
    return;
  }
  appendText(state, "result", response, out);
}

function resetTurn(state: AgyStreamState): void {
  state.turnOpen = false;
  state.assistantText = "";
  state.textByStep = {};
  state.textIndexByStep = {};
  state.tools = {};
  state.nextContentIndex = 0;
}

function failureForStatus(status: string, error: string | null): string | null {
  if (status === "SUCCESS") return null;
  if (status === "WAITING") {
    return (
      error ||
      "Antigravity CLI stopped while waiting for input. Stream-json has no interactive prompt, so Pitex cannot answer it."
    );
  }
  if (status === "INTERRUPTED" || status === "CANCELED") {
    return error || "Antigravity CLI interrupted the turn.";
  }
  return error || `Antigravity CLI ended the turn (${status}).`;
}

function closeTurn(state: AgyStreamState, failure: string | null, out: Json[]): Json[] {
  ensureTurn(state, out);
  const content: Json[] = [];
  if (state.assistantText) content.push({ type: "text", text: state.assistantText });
  const results: Json[] = [];
  for (const [id, tool] of Object.entries(state.tools)) {
    content.push({ type: "toolCall", id, name: tool.name, arguments: tool.args });
    results.push({
      role: "toolResult",
      toolCallId: id,
      isError: tool.isError,
      content: [{ type: "text", text: tool.output }],
    });
  }
  const assistant: Json = {
    role: "assistant",
    stopReason: failure ? "error" : "stop",
    content,
  };
  if (failure) assistant.errorMessage = failure;
  out.push({ type: "message_end", message: assistant });
  out.push({ type: "turn_end" });
  const agentEnd: Json = { type: "agent_end" };
  if (failure) agentEnd.messages = [assistant];
  out.push(agentEnd);
  const messages: Json[] = [];
  if (content.length > 0 || failure) messages.push(assistant);
  messages.push(...results);
  resetTurn(state);
  return messages;
}

function usageFrom(value: unknown): AgyUsage | null {
  const usage = asObject(value);
  if (!usage) return null;
  const total = usage.total_tokens;
  if (typeof total !== "number") return null;
  return {
    input: typeof usage.input_tokens === "number" ? usage.input_tokens : 0,
    output: typeof usage.output_tokens === "number" ? usage.output_tokens : 0,
    thinking: typeof usage.thinking_tokens === "number" ? usage.thinking_tokens : 0,
    cacheRead: typeof usage.cache_read_tokens === "number" ? usage.cache_read_tokens : 0,
    total,
  };
}

function conversationIdOf(event: Json, payload: Json | null): string | null {
  return asString(event.conversation_id) ?? asString(payload?.conversation_id);
}

/**
 * Project one stdout line into pi events. Unknown event names produce
 * nothing so a newer CLI can add events without breaking the transcript.
 */
export function mapAgyLine(line: unknown, state: AgyStreamState): AgyMapped {
  const event = asObject(line);
  if (!event) return emptyMapped();
  const kind = asString(event.event);
  if (kind === "init") {
    const init = asObject(event.init);
    return {
      ...emptyMapped(),
      conversationId: conversationIdOf(event, init),
      permissionMode: asString(init?.permission_mode),
    };
  }
  if (kind === "step_update") {
    const step = asObject(event.step_update);
    const mapped = emptyMapped();
    mapped.conversationId = conversationIdOf(event, step);
    if (!step) return mapped;
    const stepType = asString(step.step_type) ?? "";
    if (stepType === "agent_response") {
      appendText(state, String(step.step_index ?? "assistant"), asString(step.text_delta) ?? "", mapped.events);
    } else if (stepType === "tool" || step.tool_info) {
      mapToolStep(state, step, mapped.events);
    }
    if (step.subagent_info) mapSubagents(state, step, mapped.events);
    return mapped;
  }
  if (kind === "result") {
    const result = asObject(event.result);
    const mapped = emptyMapped();
    mapped.conversationId = conversationIdOf(event, result);
    const status = asString(result?.status) ?? "ERROR";
    if (status === "RUNNING") return mapped;
    mapped.terminal = true;
    mapped.usage = usageFrom(result?.usage);
    mapped.failure = failureForStatus(status, asString(result?.error));
    const response = typeof result?.response === "string" ? result.response : "";
    if (status === "SUCCESS" || response) appendResponse(state, response, mapped.events);
    mapped.messages = closeTurn(state, mapped.failure, mapped.events);
    return mapped;
  }
  return emptyMapped();
}

export function permissionModeNotice(mode: string | null): string {
  if (mode === "always-proceed") return AGY_ALWAYS_PROCEED_NOTICE;
  return AGY_PERMISSION_LIMIT;
}

export type ApprovalChoice = "once" | "always" | "deny";

/** Pitex decisions are once (`approve`), always (`always_allow`), or deny. */
export function decisionLabel(decision: string | undefined): ApprovalChoice {
  if (decision === "approve") return "once";
  if (decision === "always_allow") return "always";
  return "deny";
}

/**
 * No stdin message exists for a permission decision. `control_response`
 * ends the session (headless, "Unsupported messages"), and
 * `--dangerously-skip-permissions` would auto-approve every tool.
 */
export function approvalIsNotDelivered(decision: string | undefined): { sent: false; choice: ApprovalChoice } {
  return { sent: false, choice: decisionLabel(decision) };
}

/** Drop a diagnostic line that is carrying credential material. */
export function redactDiagnostic(line: string): string | null {
  const trimmed = line.trim();
  if (!trimmed) return null;
  if (/bearer\s+[a-z0-9\-._~+/]+=*/i.test(trimmed)) return null;
  if (/eyJ[a-zA-Z0-9_-]{10,}/.test(trimmed)) return null;
  if (/(api[_-]?key|access[_-]?token|refresh[_-]?token|authorization)\s*[:=]/i.test(trimmed)) return null;
  return trimmed.length > 500 ? `${trimmed.slice(0, 500)}…` : trimmed;
}

/**
 * Headless permission notices go to stderr and name the tool plus how to
 * allow it. The exact sentence is not specified; match the documented
 * words (denied, cannot prompt, allow rule) rather than a private prefix.
 */
export function permissionDenialNotice(line: string): string | null {
  const text = redactDiagnostic(line);
  if (!text) return null;
  if (!/(denied|cannot prompt|allow rule|allow-rule)/i.test(text)) return null;
  if (!/(permission|tool|command|headless)/i.test(text)) return null;
  return `${text} ${AGY_PERMISSION_LIMIT}`;
}

/**
 * Login failures the CLI prints itself. There is no documented
 * machine-readable auth-status command, and Pitex does not read
 * credential files to find out.
 */
export function authFailureReason(text: string): string | null {
  if (/authentication required/i.test(text)) return AGY_SIGNED_OUT;
  if (/authentication expired/i.test(text) || /please log back in/i.test(text)) return AGY_SIGNED_OUT;
  if (/not logged in/i.test(text)) return AGY_SIGNED_OUT;
  if (/secret keyring is locked/i.test(text)) return AGY_KEYRING_LOCKED;
  if (/GEMINI_API_KEY/i.test(text) && /not set/i.test(text)) {
    return "Antigravity CLI expects a Gemini API key in the environment and did not find one. Export it in your shell, or run `agy` and sign in.";
  }
  return null;
}

export interface AgyModel {
  id: string;
  provider: "agy";
  name: string;
}

/** `agy models` prints `slug` then a gap then a display name (headless, "Select a model"). */
export function modelsFromText(text: string): AgyModel[] {
  const models: AgyModel[] = [];
  const seen = new Set<string>();
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith("#")) continue;
    const split = line.split(/\s{2,}/);
    const id = split[0]?.trim() ?? "";
    if (!/^[A-Za-z0-9][A-Za-z0-9._:-]*$/.test(id)) continue;
    if (seen.has(id)) continue;
    seen.add(id);
    const name = split.slice(1).join(" ").trim() || id;
    models.push({ id, provider: "agy", name });
  }
  return models;
}

export function safeToken(value: string | null | undefined): string | null {
  if (!value) return null;
  return /^[A-Za-z0-9][A-Za-z0-9._:-]*$/.test(value) ? value : null;
}

export function statsFromUsage(usage: AgyUsage): Json {
  return {
    tokens: {
      total: usage.total,
      input: usage.input,
      output: usage.output,
      thinking: usage.thinking,
      cacheRead: usage.cacheRead,
    },
    cost: 0,
    contextUsage: {},
  };
}
