// Claude Code adapter. Drives the user's installed `claude` CLI through
// @anthropic-ai/claude-agent-sdk `query()` (`pathToClaudeCodeExecutable`).
// Login stays inside that CLI (`claude auth login` / `claude auth status`).
// This adapter never reads credential files and never sets an API key.
// Host variables that would replace that login — API keys, OAuth tokens,
// provider switches, and ANTHROPIC_*BASE_URL overrides — are removed from
// the environment passed to `claude` and to query(). Everything else is kept.
// https://code.claude.com/docs/en/env-vars
//
// SDK 0.3.312: permissionMode "default" keeps manual approvals on
// canUseTool (an omitted mode can start in "auto"). settingSources [] is
// the SDK's isolation mode so filesystem settings cannot pre-approve tools.
// interrupt() and the session AbortController require streaming input.
// Resume passes the session id to options.resume.
//
// Checked against https://code.claude.com/docs/en/agent-sdk/typescript
// and https://code.claude.com/docs/en/cli-reference (`claude auth status`).

import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { accessSync, constants as fsConstants, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";

import type { AdapterHost, AdapterStartOptions, CliAdapter, PiCommand } from "./adapter.ts";
import {
  allowKey,
  approvalPresentation,
  createClaudeStreamState,
  mapClaudeMessage,
  messagesFromState,
  modelIdOfInit,
  modelsFromSdk,
  parseClaudeAuthStatus,
  permissionForDecision,
  permissionModeOfInit,
  sessionIdOf,
  statsFromResult,
  visibleClaudeModels,
  type ClaudeStreamState,
  type Json,
  type ListedClaudeModel,
  type SdkModelInfo,
} from "./claude-map.ts";
import { appendUserMessage, rememberSession, threadIdFor, writeSessionPrelude } from "./session-index.ts";

export const CLAUDE_MISSING =
  "Claude Code was not found. Install the official Claude Code CLI, then run `claude auth login`.";
export const CLAUDE_SIGNED_OUT =
  "Claude Code is not logged in. Run `claude auth login` in a terminal, then try again.";

const SDK_URL = new URL("./vendor/claude-agent-sdk/sdk.mjs", import.meta.url);

export interface ClaudeQueryOptions {
  cwd: string;
  pathToClaudeCodeExecutable: string;
  permissionMode: "default";
  includePartialMessages: boolean;
  settingSources: [];
  canUseTool: (
    toolName: string,
    input: Record<string, unknown>,
    options: CanUseToolOptions
  ) => Promise<Json>;
  abortController?: AbortController;
  model?: string;
  effort?: string;
  systemPrompt: { type: "preset"; preset: "claude_code"; append?: string };
  resume?: string;
  stderr: (data: string) => void;
  strictMcpConfig: boolean;
  mcpServers: Record<string, never>;
  persistSession: boolean;
  env: Record<string, string>;
}

export interface CanUseToolOptions {
  signal: AbortSignal;
  suggestions?: unknown;
  toolUseID?: string;
  requestId?: string;
  title?: string;
  displayName?: string;
  description?: string;
  decisionReason?: string;
  suppressAlwaysAllowRule?: boolean;
}

export interface ClaudeQueryHandle extends AsyncIterable<unknown> {
  interrupt(): Promise<unknown>;
  setModel?(model?: string): Promise<void>;
  setPermissionMode?(mode: string): Promise<void>;
  applyFlagSettings?(settings: Record<string, unknown>): Promise<void>;
  supportedModels(): Promise<SdkModelInfo[]>;
  close(): void;
}

export type ClaudeQueryFn = (args: {
  prompt: AsyncIterable<Json>;
  options: ClaudeQueryOptions;
}) => ClaudeQueryHandle;

export interface ClaudeAdapterDeps {
  query?: ClaudeQueryFn;
  authStatus?: (
    binary: string,
    cwd: string,
    env: NodeJS.ProcessEnv
  ) => Promise<{ stdout: string; code: number | null }>;
}

interface PendingApproval {
  finish: (decision: string) => void;
}

/**
 * Credentials and endpoint overrides that make Claude Code bill an API key
 * or a cloud provider instead of the account from `claude auth login`.
 * Model names and the rest of the host environment stay.
 */
const CLAUDE_BILLING_ENV = new Set([
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_AWS_API_KEY",
  "ANTHROPIC_AWS_WORKSPACE_ID",
  "ANTHROPIC_CUSTOM_HEADERS",
  "ANTHROPIC_FEDERATION_RULE_ID",
  "ANTHROPIC_FOUNDRY_API_KEY",
  "ANTHROPIC_FOUNDRY_AUTH_TOKEN",
  "ANTHROPIC_FOUNDRY_RESOURCE",
  "ANTHROPIC_GOOGLE_CLOUD_WORKSPACE_ID",
  "ANTHROPIC_ORGANIZATION_ID",
  "ANTHROPIC_VERTEX_PROJECT_ID",
  "ANTHROPIC_WORKSPACE_ID",
  "AWS_BEARER_TOKEN_BEDROCK",
  "CLAUDE_BRIDGE_BASE_URL",
  "CLAUDE_BRIDGE_OAUTH_TOKEN",
  "CLAUDE_CODE_API_BASE_URL",
  "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
  "CLAUDE_CODE_HFI_BEARER_TOKEN",
  "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
  "CLAUDE_CODE_OAUTH_TOKEN",
  "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
  "CLAUDE_CODE_SESSIONS_OAUTH_TOKEN",
  "CLAUDE_CODE_USE_ANTHROPIC_AWS",
  "CLAUDE_CODE_USE_ANTHROPIC_GOOGLE_CLOUD",
  "CLAUDE_CODE_USE_BEDROCK",
  "CLAUDE_CODE_USE_FOUNDRY",
  "CLAUDE_CODE_USE_MANTLE",
  "CLAUDE_CODE_USE_VERTEX",
]);

/** `ANTHROPIC_BASE_URL` and the same override on Bedrock, Vertex, Foundry, and AWS. */
function isClaudeBillingOverride(name: string): boolean {
  if (CLAUDE_BILLING_ENV.has(name)) return true;
  return /^ANTHROPIC_(?:[A-Z0-9]+_)*BASE_URL$/.test(name);
}

/** Copy of the host environment with CLI-login overrides removed. */
export function claudeChildEnv(env: NodeJS.ProcessEnv): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(env)) {
    if (typeof value !== "string" || isClaudeBillingOverride(key)) continue;
    out[key] = value;
  }
  return out;
}

export function findClaude(env: NodeJS.ProcessEnv, platform = process.platform): string | null {
  const explicit = env.CLAUDE_PATH;
  if (explicit && isExecutable(explicit)) return explicit;
  const directories = (env.PATH ?? "").split(delimiter).filter(Boolean);
  const home = env.HOME || homedir();
  directories.push(join(home, ".local", "bin"), "/usr/local/bin", "/opt/homebrew/bin");
  const names = platform === "win32" ? ["claude.exe", "claude.cmd", "claude"] : ["claude"];
  for (const directory of directories) {
    for (const name of names) {
      const candidate = join(directory, name);
      if (isExecutable(candidate)) return candidate;
    }
  }
  return null;
}

export function defaultAuthStatus(
  binary: string,
  cwd: string,
  env: NodeJS.ProcessEnv
): Promise<{ stdout: string; code: number | null }> {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, ["auth", "status"], {
      cwd,
      env,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    let stdout = "";
    let settled = false;
    const finish = (fn: () => void) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      fn();
    };
    const timer = setTimeout(() => {
      child.kill();
      finish(() => reject(new Error(CLAUDE_SIGNED_OUT)));
    }, 15000);
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      if (stdout.length < 8192) stdout += chunk.slice(0, 8192 - stdout.length);
    });
    child.stderr.on("data", () => {
      // Status diagnostics stay off the pi stream.
    });
    child.once("error", (error) => finish(() => reject(error)));
    child.once("exit", (code) => finish(() => resolve({ stdout, code })));
  });
}

async function defaultQuery(): Promise<ClaudeQueryFn> {
  const sdk = (await import(SDK_URL.href)) as { query: ClaudeQueryFn };
  return (args) => sdk.query(args);
}

export function readPiMessages(sessionPath: string): Json[] {
  let text = "";
  try {
    text = readFileSync(sessionPath, "utf8");
  } catch {
    return [];
  }
  const messages: Json[] = [];
  for (const line of text.split("\n")) {
    if (!line.trim()) continue;
    try {
      const row = JSON.parse(line) as Json;
      if (row.type === "message" && row.message && typeof row.message === "object") {
        messages.push(row.message as Json);
      }
    } catch {
      // A torn last line is ignored; the messages ahead of it still load.
    }
  }
  return messages;
}

class PromptPump implements AsyncIterator<Json> {
  private queue: Json[] = [];
  private waiter: ((result: IteratorResult<Json>) => void) | null = null;
  private closed = false;

  push(text: string): void {
    const message: Json = {
      type: "user",
      message: { role: "user", content: text },
      parent_tool_use_id: null,
    };
    if (this.waiter) {
      const waiter = this.waiter;
      this.waiter = null;
      waiter({ value: message, done: false });
      return;
    }
    this.queue.push(message);
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    const waiter = this.waiter;
    this.waiter = null;
    waiter?.({ value: undefined as unknown as Json, done: true });
  }

  messages(): AsyncIterable<Json> {
    return { [Symbol.asyncIterator]: () => this };
  }

  next(): Promise<IteratorResult<Json>> {
    const queued = this.queue.shift();
    if (queued) return Promise.resolve({ value: queued, done: false });
    if (this.closed) return Promise.resolve({ value: undefined as unknown as Json, done: true });
    return new Promise((resolve) => {
      this.waiter = resolve;
    });
  }
}

export class ClaudeAdapter implements CliAdapter {
  readonly id = "claude";
  readonly displayName = "Claude Code";
  unavailableReason: string | null = null;

  private cwd = process.cwd();
  private sessionDir = "";
  private developerInstructions = "";
  private childEnv: Record<string, string> = {};
  private binary: string | null = null;
  private host: AdapterHost | null = null;
  private query: ClaudeQueryHandle | null = null;
  private pump: PromptPump | null = null;
  private abortController: AbortController | null = null;
  private generation = 0;
  private stream: ClaudeStreamState = createClaudeStreamState();
  private sessionPath: string | null = null;
  private sessionId: string | null = null;
  private persistedTranscript = 0;
  private modelId: string | null = null;
  private modelPinned = false;
  private effort: string | null = null;
  private models: ListedClaudeModel[] = [];
  private allowKeys = new Set<string>();
  private pending = new Map<string, PendingApproval>();
  /** `auth_status` can arrive before the host's first command. */
  private signedOutPending = false;
  private running = false;
  private stats: Json = { tokens: { total: 0 }, cost: 0, contextUsage: {} };
  private readonly deps: ClaudeAdapterDeps;

  constructor(deps: ClaudeAdapterDeps = {}) {
    this.deps = deps;
  }

  async start(options: AdapterStartOptions): Promise<void> {
    this.cwd = options.cwd;
    this.sessionDir = options.sessionDir;
    this.developerInstructions = options.developerInstructions ?? "";
    this.childEnv = claudeChildEnv(options.env);
    try {
      this.binary = findClaude(options.env);
      if (!this.binary) throw new Error(CLAUDE_MISSING);
      const status = await (this.deps.authStatus ?? defaultAuthStatus)(this.binary, this.cwd, this.childEnv);
      if (!parseClaudeAuthStatus(status.stdout, status.code)) throw new Error(CLAUDE_SIGNED_OUT);
      await this.openQuery(null);
      this.unavailableReason = null;
    } catch (error) {
      await this.stop();
      this.unavailableReason = error instanceof Error ? error.message : CLAUDE_MISSING;
    }
  }

  async handleCommand(command: PiCommand, host: AdapterHost): Promise<void> {
    this.host = host;
    this.deliverSignedOut();
    if (this.unavailableReason && command.type !== "abort") {
      host.respond(command.type, false, undefined, this.unavailableReason, command.id);
      return;
    }
    try {
      switch (command.type) {
        case "prompt":
        case "steer":
          await this.prompt(command, host);
          return;
        case "abort":
          await this.abort(host, command.id);
          return;
        case "new_session":
          await this.newSession(host, command.id);
          return;
        case "switch_session":
          await this.switchSession(command, host);
          return;
        case "get_messages":
          host.respond("get_messages", true, { messages: messagesFromState(this.stream) }, undefined, command.id);
          return;
        case "get_state":
          host.respond("get_state", true, this.stateData(), undefined, command.id);
          return;
        case "get_available_models":
          host.respond(
            "get_available_models",
            true,
            { models: visibleClaudeModels(this.models) },
            undefined,
            command.id
          );
          return;
        case "get_available_thinking_levels":
          host.respond(
            "get_available_thinking_levels",
            true,
            { levels: this.thinkingLevels() },
            undefined,
            command.id
          );
          return;
        case "set_model":
          await this.setModel(command, host);
          return;
        case "set_thinking_level":
          await this.setEffort(command, host);
          return;
        case "get_session_stats":
          host.respond("get_session_stats", true, this.stats, undefined, command.id);
          return;
        case "get_commands":
          host.respond("get_commands", true, { commands: [] }, undefined, command.id);
          return;
        case "approval_response":
          this.resolveApproval(command);
          host.respond("approval_response", true, {}, undefined, command.id);
          return;
        default:
          host.respond(command.type, false, undefined, `Unsupported command: ${command.type}`, command.id);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Claude Code request failed.";
      host.respond(command.type, false, undefined, publicClaudeError(message), command.id);
    }
  }

  async stop(): Promise<void> {
    this.closeActiveQuery();
    this.running = false;
  }

  private async openQuery(resumeId: string | null): Promise<void> {
    this.closeActiveQuery();
    if (!this.binary) throw new Error(CLAUDE_MISSING);
    const generation = this.generation;
    const controller = new AbortController();
    this.abortController = controller;
    const pump = new PromptPump();
    this.pump = pump;
    const queryFn = this.deps.query ?? (await defaultQuery());
    const query = queryFn({
      prompt: pump.messages(),
      options: this.buildOptions(this.binary, resumeId, controller),
    });
    this.query = query;
    void this.readLoop(generation, query);
    const models = await this.loadModels(query);
    if (generation === this.generation && models.length > 0) {
      this.models = models;
      if (!this.modelId) this.modelId = models[0].id;
    }
  }

  private closeActiveQuery(): void {
    this.generation += 1;
    this.denyOutstanding(true);
    const query = this.query;
    const pump = this.pump;
    const controller = this.abortController;
    this.query = null;
    this.pump = null;
    this.abortController = null;
    pump?.close();
    try {
      controller?.abort();
    } catch {
      // The controller can already be aborted when the CLI exits on its own.
    }
    try {
      query?.close();
    } catch {
      // close() is idempotent when the CLI has already gone.
    }
  }

  private buildOptions(binary: string, resumeId: string | null, controller: AbortController): ClaudeQueryOptions {
    const options: ClaudeQueryOptions = {
      cwd: this.cwd,
      pathToClaudeCodeExecutable: binary,
      permissionMode: "default",
      includePartialMessages: true,
      settingSources: [],
      canUseTool: (toolName, input, toolOptions) => this.onCanUseTool(toolName, input, toolOptions),
      abortController: controller,
      systemPrompt: {
        type: "preset",
        preset: "claude_code",
        ...(this.developerInstructions ? { append: this.developerInstructions } : {}),
      },
      stderr: () => {
        // CLI diagnostics stay off the pi event stream.
      },
      strictMcpConfig: true,
      mcpServers: {},
      persistSession: true,
      env: this.childEnv,
    };
    if (this.modelId) options.model = this.modelId;
    if (this.effort) options.effort = this.effort;
    if (resumeId) options.resume = resumeId;
    return options;
  }

  private async loadModels(query: ClaudeQueryHandle): Promise<ListedClaudeModel[]> {
    const pending = query.supportedModels().catch(() => [] as SdkModelInfo[]);
    let timer: ReturnType<typeof setTimeout> | undefined;
    const listed = await Promise.race([
      pending,
      new Promise<SdkModelInfo[]>((resolve) => {
        timer = setTimeout(() => resolve([]), 10000);
        timer.unref?.();
      }),
    ]);
    if (timer) clearTimeout(timer);
    return modelsFromSdk(listed);
  }

  private async readLoop(generation: number, query: ClaudeQueryHandle): Promise<void> {
    try {
      for await (const message of query) {
        if (generation !== this.generation) return;
        this.onSdkMessage(message);
      }
    } catch (error) {
      if (generation !== this.generation) return;
      const text = error instanceof Error ? error.message : "Claude Code stopped.";
      this.running = false;
      this.host?.emit({ type: "extension_error", error: publicClaudeError(text) });
    }
  }

  private onSdkMessage(message: unknown): void {
    const row = asObject(message);
    if (!row) return;
    if (row.type === "auth_status") {
      if (row.error) this.noteSignedOut();
      return;
    }
    const sessionId = sessionIdOf(message);
    if (sessionId) {
      this.sessionId = sessionId;
      this.remember();
    }
    const initModel = modelIdOfInit(message);
    if (initModel && !this.modelPinned) this.modelId = initModel;
    const mode = permissionModeOfInit(message);
    if (mode && mode !== "default") void this.query?.setPermissionMode?.("default");
    const before = this.stream.transcript.length;
    const events = mapClaudeMessage(message, this.stream);
    this.persistFrom(before);
    const stats = statsFromResult(message);
    if (stats) this.stats = stats;
    if (events.some((event) => event.type === "agent_end")) this.running = false;
    if (!this.host) return;
    for (const event of events) this.host.emit(event);
  }

  private async prompt(command: PiCommand, host: AdapterHost): Promise<void> {
    const message = command.message ?? "";
    if (!message.trim()) {
      host.respond(command.type, false, undefined, "The prompt was empty.", command.id);
      return;
    }
    if (!this.query || !this.pump) await this.openQuery(this.sessionId);
    if (!this.pump) throw new Error(CLAUDE_MISSING);
    this.noteUserMessage(message);
    this.running = true;
    this.pump.push(message);
    host.respond(command.type, true, {}, undefined, command.id);
  }

  private async abort(host: AdapterHost, id?: string): Promise<void> {
    this.denyOutstanding(true);
    try {
      await this.query?.interrupt();
    } catch {
      // The turn may already have finished.
    }
    this.running = false;
    host.respond("abort", true, {}, undefined, id);
  }

  private async newSession(host: AdapterHost, id?: string): Promise<void> {
    this.sessionPath = null;
    this.sessionId = null;
    this.persistedTranscript = 0;
    this.stream = createClaudeStreamState();
    this.allowKeys.clear();
    this.running = false;
    await this.openQuery(null);
    host.respond("new_session", true, {}, undefined, id);
  }

  private async switchSession(command: PiCommand, host: AdapterHost): Promise<void> {
    const sessionPath = command.sessionPath ?? "";
    const threadId = threadIdFor(this.sessionDir, sessionPath);
    if (!threadId) {
      host.respond(
        "switch_session",
        false,
        undefined,
        "That conversation has no Claude Code session to resume.",
        command.id
      );
      return;
    }
    this.allowKeys.clear();
    this.sessionPath = sessionPath;
    this.sessionId = threadId;
    this.stream = createClaudeStreamState();
    this.stream.transcript = readPiMessages(sessionPath);
    this.persistedTranscript = this.stream.transcript.length;
    this.running = false;
    await this.openQuery(threadId);
    host.respond("switch_session", true, { cancelled: false }, undefined, command.id);
  }

  private async setModel(command: PiCommand, host: AdapterHost): Promise<void> {
    if (command.modelId) {
      this.modelId = command.modelId;
      this.modelPinned = true;
    }
    if (this.query?.setModel && this.modelId) {
      try {
        await this.query.setModel(this.modelId);
      } catch {
        // The next query() passes options.model when this process is gone.
      }
    }
    host.respond("set_model", true, {}, undefined, command.id);
  }

  private async setEffort(command: PiCommand, host: AdapterHost): Promise<void> {
    if (command.level) this.effort = command.level;
    if (this.query?.applyFlagSettings && this.effort) {
      try {
        await this.query.applyFlagSettings({ effortLevel: this.effort });
      } catch {
        // options.effort is applied the next time the CLI starts.
      }
    }
    host.respond("set_thinking_level", true, {}, undefined, command.id);
  }

  private noteUserMessage(text: string): void {
    const userMessage: Json = { role: "user", content: [{ type: "text", text }] };
    if (!this.sessionPath) {
      this.sessionPath = joinSession(this.sessionDir, `claude-${randomUUID()}.jsonl`);
      writeSessionPrelude(this.sessionPath, text, this.cwd);
      this.stream.transcript = readPiMessages(this.sessionPath);
      this.persistedTranscript = this.stream.transcript.length;
    } else {
      appendUserMessage(this.sessionPath, text);
      this.stream.transcript.push(userMessage);
      this.persistedTranscript = this.stream.transcript.length;
    }
    this.remember();
  }

  private remember(): void {
    if (!this.sessionPath || !this.sessionId) return;
    rememberSession(this.sessionDir, this.sessionPath, this.sessionId, this.modelId ?? undefined);
  }

  private persistFrom(before: number): void {
    if (!this.sessionPath) return;
    const start = Math.max(before, this.persistedTranscript);
    for (let index = start; index < this.stream.transcript.length; index += 1) {
      appendPiMessage(this.sessionPath, this.stream.transcript[index]);
    }
    this.persistedTranscript = this.stream.transcript.length;
  }

  private stateData(): Json {
    const current = this.models.find((model) => model.id === this.modelId);
    return {
      model: this.modelId
        ? { id: this.modelId, provider: "claude", name: current?.name ?? this.modelId }
        : null,
      thinkingLevel: this.thinkingLevel(),
    };
  }

  private thinkingLevel(): string {
    if (this.effort) return this.effort;
    const levels = this.thinkingLevels();
    if (levels.includes("medium")) return "medium";
    return levels[0] ?? "medium";
  }

  private thinkingLevels(): string[] {
    const current = this.models.find((model) => model.id === this.modelId);
    const levels = current?.efforts.length ? [...current.efforts] : ["low", "medium", "high", "xhigh"];
    const selected = this.effort;
    if (selected && !levels.includes(selected)) levels.unshift(selected);
    return levels;
  }

  private onCanUseTool(
    toolName: string,
    input: Record<string, unknown>,
    options: CanUseToolOptions
  ): Promise<Json> {
    const safeInput = input && typeof input === "object" && !Array.isArray(input) ? input : {};
    const key = allowKey(toolName, safeInput);
    if (this.allowKeys.has(key)) {
      return Promise.resolve({
        behavior: "allow",
        updatedInput: safeInput,
        ...(options.toolUseID ? { toolUseID: options.toolUseID } : {}),
        decisionClassification: "user_permanent",
      });
    }
    const id = options.toolUseID || options.requestId || randomUUID();
    const view = approvalPresentation(toolName, safeInput, options);
    return new Promise((resolve) => {
      let settled = false;
      const finish = (result: Json) => {
        if (settled) return;
        settled = true;
        this.pending.delete(id);
        resolve(result);
      };
      const decide = (decision: string) => {
        const aborted = decision === "abort";
        const choice = permissionForDecision(
          aborted ? "deny" : decision,
          toolName,
          safeInput,
          options.suggestions,
          options.suppressAlwaysAllowRule === true,
          options.toolUseID
        );
        if (choice.remember) this.allowKeys.add(choice.rememberKey);
        finish(aborted ? { ...choice.result, interrupt: true } : choice.result);
      };
      if (options.signal.aborted) {
        decide("abort");
        return;
      }
      options.signal.addEventListener("abort", () => decide("abort"), { once: true });
      this.pending.set(id, { finish: decide });
      this.host?.emit({
        type: "approval_request",
        id,
        kind: view.kind,
        title: view.title,
        detail: view.detail,
      });
    });
  }

  private resolveApproval(command: PiCommand): void {
    const pending = this.pending.get(command.id ?? "");
    if (!pending) return;
    pending.finish(command.decision ?? "deny");
  }

  private noteSignedOut(): void {
    this.signedOutPending = true;
    this.deliverSignedOut();
  }

  private deliverSignedOut(): void {
    if (!this.signedOutPending || !this.host) return;
    this.signedOutPending = false;
    this.host.emit({ type: "extension_error", error: CLAUDE_SIGNED_OUT });
  }

  private denyOutstanding(interrupt: boolean): void {
    const outstanding = [...this.pending.values()];
    this.pending.clear();
    for (const pending of outstanding) pending.finish(interrupt ? "abort" : "deny");
  }
}

function appendPiMessage(sessionPath: string, message: Json): void {
  writeFileSync(sessionPath, `${JSON.stringify({ type: "message", message })}\n`, { flag: "a" });
}

function joinSession(sessionDir: string, name: string): string {
  return sessionDir.endsWith("/") ? sessionDir + name : `${sessionDir}/${name}`;
}

function publicClaudeError(text: string): string {
  if (/not logged in|unauthori[sz]ed|auth login/i.test(text)) return CLAUDE_SIGNED_OUT;
  return text.slice(0, 500);
}

function asObject(value: unknown): Json | null {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Json) : null;
}

function isExecutable(path: string): boolean {
  try {
    accessSync(path, fsConstants.X_OK);
    return true;
  } catch {
    return false;
  }
}
