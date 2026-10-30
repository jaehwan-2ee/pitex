// Grok CLI adapter. Speaks `grok --no-auto-update agent stdio` (ACP JSON-RPC
// over stdio) and the user's existing `grok login`. It never reads, copies,
// or parses credential files, and it never refreshes tokens.
//
// Protocol: Grok CLI ACP guide, https://docs.x.ai/build/cli/headless-scripting
// (initialize, authenticate with methodId "cached_token", session/new,
// session/prompt, session/update, session/request_permission, session/cancel,
// session/load when advertised, session/set_model). ACP v1 filesystem and
// terminal methods are enabled and jailed to the project directory.
//
// The adapter shape follows Pitex's Codex adapter, which was adapted from
// T3 Code v0.1.4 (MIT), Copyright (c) 2026 T3 Tools Inc.
// https://github.com/pingdotgg/t3code
// This file is original Pitex code, not a copy of T3's Grok driver.

import { spawn, type ChildProcess, type ChildProcessWithoutNullStreams } from "node:child_process";
import { randomUUID } from "node:crypto";
import { accessSync, constants as fsConstants, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join, resolve } from "node:path";

import type { AdapterHost, AdapterStartOptions, CliAdapter, PiCommand } from "./adapter.ts";
import { stopCodexChild } from "./codex-adapter.ts";
import {
  GROK_CLIENT_CAPABILITIES,
  approvalFromPermission,
  cancelledPermission,
  catalogFromAcp,
  closeTurn,
  createHistory,
  createStreamState,
  finishHistoryTurn,
  historyMessages,
  mapSessionUpdate,
  permissionOptions,
  permissionResult,
  pushUserMessage,
  readProjectFile,
  recordSessionUpdate,
  terminalWorkingDirectory,
  visibleModels,
  writeProjectFile,
  type AcpCatalog,
  type GrokModel,
  type HistoryState,
  type Json,
  type PermissionOption,
  type StreamState,
} from "./grok-map.ts";
import { redactSecrets } from "./jsonrpc.ts";
import {
  appendUserMessage,
  rememberSession,
  threadIdFor,
  writeSessionPrelude,
} from "./session-index.ts";

export const GROK_MISSING =
  "Grok CLI was not found. Install the official Grok CLI, then run `grok login`.";
export const GROK_SIGNED_OUT =
  "Grok CLI is not logged in. Run `grok login` in a terminal, then try again.";
export const GROK_NO_RESUME =
  "This Grok CLI cannot resume a saved session (it does not advertise loadSession). Start a new conversation.";

const GROK_ARGS = ["--no-auto-update", "agent", "stdio"] as const;
const PROMPT_TIMEOUT_MS = 30 * 60 * 1000;
const LOAD_TIMEOUT_MS = 120_000;

export function findGrok(env: NodeJS.ProcessEnv, platform = process.platform): string | null {
  const explicit = env.GROK_PATH;
  if (explicit && isExecutable(explicit)) return explicit;
  const directories = (env.PATH ?? "").split(delimiter).filter(Boolean);
  const home = env.HOME || homedir();
  directories.push(join(home, ".local", "bin"), "/usr/local/bin", "/opt/homebrew/bin");
  const names = platform === "win32" ? ["grok.exe", "grok.cmd", "grok"] : ["grok"];
  for (const directory of directories) {
    for (const name of names) {
      const candidate = join(directory, name);
      if (isExecutable(candidate)) return candidate;
    }
  }
  return null;
}

export interface GrokLaunchSpec {
  command: string;
  args: string[];
  windowsVerbatimArguments?: boolean;
}

/**
 * `grok --no-auto-update agent stdio` as documented for ACP in scripts.
 * Windows CreateProcess cannot run a shebang. A node or bun shebang is
 * started with this process's node. The check reads only the first 128
 * bytes of the executable. A `.cmd` / `.bat` shim is started through
 * `cmd.exe /d /s /c` with those fixed arguments only. `%` and other cmd
 * metacharacters are rejected. `.exe` is spawned directly.
 */
export function grokLaunchSpec(binary: string, platform = process.platform): GrokLaunchSpec {
  const direct = { command: binary, args: [...GROK_ARGS] };
  if (platform !== "win32") return direct;
  if (/\.exe$/i.test(binary)) return direct;
  if (/\.(cmd|bat)$/i.test(binary)) {
    const quoted = quoteCmdExecutable(binary);
    if (!quoted) throw new Error("Grok CLI path cannot be launched through cmd.exe.");
    return {
      command: "cmd.exe",
      args: ["/d", "/s", "/c", `${quoted} --no-auto-update agent stdio`],
      windowsVerbatimArguments: true,
    };
  }
  let header = "";
  try {
    header = readFileSync(binary).subarray(0, 128).toString("utf8");
  } catch {
    return direct;
  }
  if (header.startsWith("#!") && /node|bun/.test(header)) {
    return { command: process.execPath, args: [binary, ...GROK_ARGS] };
  }
  return direct;
}

function quoteCmdExecutable(binary: string): string | null {
  if (!binary || /[\r\n"%!^&|<>]/.test(binary)) return null;
  return `"${binary}"`;
}

function isExecutable(path: string): boolean {
  try {
    accessSync(path, fsConstants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function asObject(value: unknown): Json | null {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Json) : null;
}

function asString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function optionalInt(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? Math.trunc(value) : undefined;
}

function isMethodNotFound(error: unknown): boolean {
  const message = error instanceof Error ? error.message : "";
  const code = error instanceof GrokRpcError ? error.code : undefined;
  return code === -32601 || /method not found/i.test(message);
}

function isAuthRequired(error: unknown): boolean {
  const message = error instanceof Error ? error.message : "";
  return /auth/i.test(message);
}

class GrokRpcError extends Error {
  readonly code: number;
  constructor(message: string, code = -32000) {
    super(message);
    this.name = "GrokRpcError";
    this.code = code;
  }
}

interface PendingRpc {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timer: NodeJS.Timeout;
}

/**
 * Newline-delimited JSON-RPC for the Grok ACP child.
 * Inbound messages go through the shared secret redaction before dispatch.
 * This class does not read credential files.
 */
class GrokRpc {
  private nextId = 1;
  private buffer = "";
  private pending = new Map<number, PendingRpc>();
  private closed = false;
  private readonly child: ChildProcessWithoutNullStreams;
  private readonly onNotification: (method: string, params: Json) => void;
  private readonly onServerRequest: (method: string, id: string | number, params: Json) => void;
  private readonly onClose?: (message: string) => void;

  constructor(
    child: ChildProcessWithoutNullStreams,
    onNotification: (method: string, params: Json) => void,
    onServerRequest: (method: string, id: string | number, params: Json) => void,
    onClose?: (message: string) => void
  ) {
    this.child = child;
    this.onNotification = onNotification;
    this.onServerRequest = onServerRequest;
    this.onClose = onClose;
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => this.ingest(chunk));
    child.on("exit", () => {
      this.failAll("Grok CLI exited.");
      this.onClose?.("Grok CLI exited.");
    });
    child.on("error", (error) => {
      this.failAll(error.message);
      this.onClose?.(error.message);
    });
    child.stdin.on("error", (error: NodeJS.ErrnoException) => {
      const message =
        error.code === "EPIPE" ? "Grok CLI is unavailable." : `Grok CLI is unavailable (${error.message}).`;
      this.failAll(message);
      this.onClose?.(message);
    });
  }

  request(method: string, params: unknown, timeoutMs = 30_000): Promise<unknown> {
    if (this.closed) return Promise.reject(new GrokRpcError("Grok CLI is not running."));
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new GrokRpcError(`Grok CLI timed out waiting for ${method}.`));
      }, timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
      this.send({ jsonrpc: "2.0", id, method, params: params ?? {} });
    });
  }

  notify(method: string, params: unknown = {}): void {
    this.send({ jsonrpc: "2.0", method, params });
  }

  respond(id: string | number, result: unknown): void {
    this.send({ jsonrpc: "2.0", id, result });
  }

  fail(id: string | number, message: string): void {
    this.send({ jsonrpc: "2.0", id, error: { code: -32000, message } });
  }

  close(): void {
    this.closed = true;
    this.failAll("Grok CLI stopped.");
  }

  private send(message: unknown): void {
    if (this.closed) return;
    this.child.stdin.write(`${JSON.stringify(message)}\n`);
  }

  private ingest(chunk: string): void {
    this.buffer += chunk;
    while (true) {
      const newline = this.buffer.indexOf("\n");
      if (newline < 0) return;
      const line = this.buffer.slice(0, newline).replace(/\r$/, "");
      this.buffer = this.buffer.slice(newline + 1);
      if (!line.trim()) continue;
      let parsed: unknown;
      try {
        parsed = JSON.parse(line);
      } catch {
        continue;
      }
      this.dispatch(redactSecrets(parsed) as Json);
    }
  }

  private dispatch(message: Json): void {
    const method = typeof message.method === "string" ? message.method : "";
    const params = asObject(message.params) ?? {};
    const hasId = Object.prototype.hasOwnProperty.call(message, "id");
    if (method && hasId) {
      const id = message.id;
      if (typeof id === "string" || typeof id === "number") this.onServerRequest(method, id, params);
      return;
    }
    if (hasId && (Object.prototype.hasOwnProperty.call(message, "result") || message.error)) {
      const id = typeof message.id === "number" ? message.id : Number(message.id);
      const waiter = this.pending.get(id);
      if (!waiter) return;
      clearTimeout(waiter.timer);
      this.pending.delete(id);
      if (message.error && typeof message.error === "object") {
        const error = message.error as { message?: string; code?: number };
        waiter.reject(new GrokRpcError(error.message || "Grok CLI request failed.", error.code ?? -32000));
        return;
      }
      waiter.resolve(message.result);
      return;
    }
    if (method) this.onNotification(method, params);
  }

  private failAll(message: string): void {
    this.closed = true;
    for (const [id, waiter] of this.pending) {
      clearTimeout(waiter.timer);
      waiter.reject(new GrokRpcError(message));
      this.pending.delete(id);
    }
  }
}

interface PendingApproval {
  kind: "permission" | "terminal";
  rpcId: string | number;
  options: PermissionOption[];
  terminal?: Json;
}

interface LiveTerminal {
  id: string;
  child: ChildProcess;
  output: string;
  truncated: boolean;
  limit: number;
  exited: boolean;
  exitCode: number | null;
  signal: NodeJS.Signals | null;
  done: Promise<void>;
  settle: () => void;
}

function authMethodIds(init: Json): string[] {
  const methods = Array.isArray(init.authMethods) ? init.authMethods : [];
  const ids: string[] = [];
  for (const method of methods) {
    if (typeof method === "string" && method.length > 0) {
      ids.push(method);
      continue;
    }
    const row = asObject(method);
    const id = asString(row?.id) ?? asString(row?.methodId);
    if (id) ids.push(id);
  }
  return ids;
}

function rememberTerminalOutput(term: LiveTerminal, chunk: Buffer): void {
  term.output += chunk.toString("utf8");
  const size = Buffer.byteLength(term.output);
  if (size <= term.limit) return;
  term.truncated = true;
  const bytes = Buffer.from(term.output);
  let offset = Math.max(0, bytes.length - term.limit);
  while (offset < bytes.length && (bytes[offset] & 0xc0) === 0x80) offset += 1;
  term.output = bytes.subarray(offset).toString("utf8");
}

function terminalEnv(base: NodeJS.ProcessEnv, requested: unknown): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = { ...base };
  if (!Array.isArray(requested)) return env;
  for (const entry of requested) {
    const row = asObject(entry);
    const name = asString(row?.name);
    if (!name || typeof row?.value !== "string") continue;
    env[name] = row.value;
  }
  return env;
}

export class GrokAdapter implements CliAdapter {
  readonly id = "grok";
  readonly displayName = "Grok CLI";
  unavailableReason: string | null = null;

  private rpc: GrokRpc | null = null;
  private child: ChildProcessWithoutNullStreams | null = null;
  private cwd = process.cwd();
  private sessionDir = "";
  private developerInstructions = "";
  private env: NodeJS.ProcessEnv = process.env;
  private stream: StreamState = createStreamState();
  private history: HistoryState = createHistory();
  private host: AdapterHost | null = null;
  private sessionId: string | null = null;
  private sessionPath: string | null = null;
  private modelId: string | null = null;
  private modelTouched = false;
  private effort: string | null = null;
  private effortTouched = false;
  private models: GrokModel[] = [];
  private efforts: string[] = [];
  private modelConfigId: string | null = null;
  private effortConfigId: string | null = null;
  private effortUsesMode = false;
  private loadSession = false;
  private pending = new Map<string, PendingApproval>();
  private terminals = new Map<string, LiveTerminal>();
  private terminalAlways = false;
  private commands: Json[] = [];
  private running = false;
  private suppressUserChunks = false;
  private suppressLive = false;
  private acceptClose = false;
  private stopped = false;
  private stats: Json = { tokens: { total: 0 }, cost: 0, contextUsage: {} };

  async start(options: AdapterStartOptions): Promise<void> {
    this.cwd = options.cwd;
    this.sessionDir = options.sessionDir;
    this.developerInstructions = options.developerInstructions ?? "";
    this.env = options.env;
    this.stopped = false;
    try {
      await this.connect(options.env);
      this.unavailableReason = null;
    } catch (error) {
      await this.stop();
      this.unavailableReason = error instanceof Error ? error.message : GROK_MISSING;
    }
  }

  private async connect(env: NodeJS.ProcessEnv): Promise<void> {
    const binary = findGrok(env);
    if (!binary) throw new Error(GROK_MISSING);
    this.acceptClose = true;
    const launch = grokLaunchSpec(binary);
    const child = spawn(launch.command, launch.args, {
      cwd: this.cwd,
      env,
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
      windowsVerbatimArguments: launch.windowsVerbatimArguments === true,
    }) as ChildProcessWithoutNullStreams;
    this.child = child;
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", () => {
      // Grok diagnostics stay off the pi event stream.
    });
    this.rpc = new GrokRpc(
      child,
      (method, params) => this.onNotification(method, params),
      (method, id, params) => this.onServerRequest(method, id, params),
      (message) => this.markClosed(message)
    );
    const init = asObject(
      await this.rpc.request("initialize", {
        protocolVersion: 1,
        clientInfo: { name: "pitex", title: "Pitex", version: "0.1.0" },
        clientCapabilities: GROK_CLIENT_CAPABILITIES,
      })
    );
    if (!init) throw new Error(GROK_MISSING);
    const version = init.protocolVersion;
    if (version !== undefined && version !== 1 && version !== "1") {
      throw new Error("Grok CLI speaks an ACP version Pitex does not support.");
    }
    const capabilities = asObject(init.agentCapabilities);
    this.loadSession = capabilities?.loadSession === true;
    // Official ACP example: CLI login is methodId "cached_token".
    // Pitex does not select an API-key method and does not read token files.
    if (!authMethodIds(init).includes("cached_token")) throw new Error(GROK_SIGNED_OUT);
    try {
      await this.rpc.request("authenticate", { methodId: "cached_token", _meta: { headless: true } });
    } catch {
      throw new Error(GROK_SIGNED_OUT);
    }
    this.mergeCatalog(catalogFromAcp(init));
    if (this.models.length === 0) {
      try {
        this.mergeCatalog(catalogFromAcp(await this.rpc.request("x.ai/models/list", {}, 3_000)));
      } catch {
        // The extension is optional. Session setup can still advertise models.
      }
    }
  }

  private markClosed(message: string): void {
    if (!this.acceptClose) return;
    this.acceptClose = false;
    this.unavailableReason = message;
    this.running = false;
    this.host?.emit({ type: "extension_error", error: message });
  }

  async handleCommand(command: PiCommand, host: AdapterHost): Promise<void> {
    this.host = host;
    if (this.unavailableReason && command.type !== "abort") {
      host.respond(command.type, false, undefined, this.unavailableReason, command.id);
      return;
    }
    try {
      switch (command.type) {
        case "prompt":
          await this.prompt(command, host);
          return;
        case "steer":
          if (this.running) {
            host.respond(
              "steer",
              false,
              undefined,
              "Grok CLI cannot steer an in-progress turn. Wait for it to finish or stop it.",
              command.id
            );
            return;
          }
          await this.prompt({ ...command, type: "prompt" }, host);
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
          host.respond("get_messages", true, { messages: historyMessages(this.history) }, undefined, command.id);
          return;
        case "get_state":
          host.respond("get_state", true, this.stateData(), undefined, command.id);
          return;
        case "get_available_models":
          host.respond("get_available_models", true, { models: visibleModels(this.models) }, undefined, command.id);
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
          host.respond("get_commands", true, { commands: this.commands }, undefined, command.id);
          return;
        case "approval_response":
          this.resolveApproval(command);
          host.respond("approval_response", true, {}, undefined, command.id);
          return;
        default:
          host.respond(command.type, false, undefined, `Unsupported command: ${command.type}`, command.id);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Grok CLI request failed.";
      host.respond(command.type, false, undefined, message, command.id);
    }
  }

  async stop(): Promise<void> {
    this.stopped = true;
    this.acceptClose = false;
    this.killTerminals();
    const child = this.child;
    this.pending.clear();
    this.rpc?.close();
    this.child = null;
    this.rpc = null;
    if (!child) return;
    try {
      await stopCodexChild(child);
    } catch {
      // Stop still finishes when the final signal cannot be delivered.
    }
  }

  private async prompt(command: PiCommand, host: AdapterHost): Promise<void> {
    const message = command.message ?? "";
    if (!message.trim()) {
      host.respond("prompt", false, undefined, "The prompt was empty.", command.id);
      return;
    }
    if (this.running) {
      host.respond("prompt", false, undefined, "Grok CLI is still working on the current turn.", command.id);
      return;
    }
    if (!this.sessionId) await this.openSession(null);
    if (!this.sessionPath) {
      this.sessionPath = joinSession(this.sessionDir, `grok-${randomUUID()}.jsonl`);
      writeSessionPrelude(this.sessionPath, message, this.cwd);
      if (this.sessionId) rememberSession(this.sessionDir, this.sessionPath, this.sessionId, this.modelId ?? undefined);
    } else {
      appendUserMessage(this.sessionPath, message);
    }
    pushUserMessage(this.history, message);
    this.suppressUserChunks = true;
    this.running = true;
    const pending = this.rpc?.request(
      "session/prompt",
      {
        sessionId: this.sessionId,
        prompt: [{ type: "text", text: message }],
      },
      PROMPT_TIMEOUT_MS
    );
    host.respond("prompt", true, {}, undefined, command.id);
    void pending
      ?.then((result) => {
        const stop = asString(asObject(result)?.stopReason);
        this.completeTurn(stop);
      })
      .catch((error: unknown) => {
        if (isAuthRequired(error)) {
          this.unavailableReason = GROK_SIGNED_OUT;
          this.completeTurn("refusal", GROK_SIGNED_OUT);
          return;
        }
        const text = error instanceof Error ? error.message : "Grok CLI request failed.";
        this.completeTurn("refusal", text);
      });
  }

  private async abort(host: AdapterHost, id?: string): Promise<void> {
    this.denyOutstanding("cancelled");
    this.killTerminals();
    if (this.rpc && this.sessionId) this.rpc.notify("session/cancel", { sessionId: this.sessionId });
    if (this.running) this.completeTurn("cancelled");
    host.respond("abort", true, {}, undefined, id);
  }

  private async newSession(host: AdapterHost, id?: string): Promise<void> {
    this.denyOutstanding("cancelled");
    this.killTerminals();
    if (this.rpc && this.sessionId && this.running) {
      this.rpc.notify("session/cancel", { sessionId: this.sessionId });
    }
    if (this.running) this.completeTurn("cancelled");
    this.sessionId = null;
    this.sessionPath = null;
    this.history = createHistory();
    this.stream = createStreamState();
    this.terminalAlways = false;
    host.respond("new_session", true, {}, undefined, id);
  }

  private async switchSession(command: PiCommand, host: AdapterHost): Promise<void> {
    const sessionPath = command.sessionPath ?? "";
    const sessionId = threadIdFor(this.sessionDir, sessionPath);
    if (!sessionId) {
      host.respond("switch_session", false, undefined, "That conversation has no Grok session to resume.", command.id);
      return;
    }
    if (!this.loadSession) {
      host.respond("switch_session", false, undefined, GROK_NO_RESUME, command.id);
      return;
    }
    if (this.running && this.rpc && this.sessionId) {
      this.denyOutstanding("cancelled");
      this.rpc.notify("session/cancel", { sessionId: this.sessionId });
      this.completeTurn("cancelled");
    }
    await this.openSession(sessionId);
    this.sessionPath = sessionPath;
    host.respond("switch_session", true, { cancelled: false }, undefined, command.id);
  }

  private async setModel(command: PiCommand, host: AdapterHost): Promise<void> {
    this.modelTouched = true;
    this.modelId = command.modelId || this.modelId;
    if (this.sessionId && this.modelId) await this.pushModel(this.modelId);
    if (this.sessionPath && this.sessionId) {
      rememberSession(this.sessionDir, this.sessionPath, this.sessionId, this.modelId ?? undefined);
    }
    host.respond("set_model", true, {}, undefined, command.id);
  }

  private async setEffort(command: PiCommand, host: AdapterHost): Promise<void> {
    this.effortTouched = true;
    this.effort = command.level || this.effort;
    if (this.sessionId && this.effort) await this.pushEffort(this.effort);
    host.respond("set_thinking_level", true, {}, undefined, command.id);
  }

  private async openSession(resumeId: string | null): Promise<void> {
    if (!this.rpc) throw new Error(GROK_MISSING);
    if (resumeId) {
      if (!this.loadSession) throw new Error(GROK_NO_RESUME);
      this.suppressLive = true;
      this.history = createHistory();
      this.stream = createStreamState();
      try {
        const result = await this.rpc.request(
          "session/load",
          {
            sessionId: resumeId,
            cwd: this.projectRoot(),
            mcpServers: [],
            ...this.sessionMeta(),
          },
          LOAD_TIMEOUT_MS
        );
        this.sessionId = resumeId;
        this.mergeCatalog(catalogFromAcp(result));
      } catch (error) {
        if (isAuthRequired(error)) throw new Error(GROK_SIGNED_OUT);
        throw error;
      } finally {
        this.suppressLive = false;
      }
      return;
    }
    let result: unknown;
    try {
      result = await this.rpc.request("session/new", {
        cwd: this.projectRoot(),
        mcpServers: [],
        ...this.sessionMeta(),
      });
    } catch (error) {
      if (isAuthRequired(error)) throw new Error(GROK_SIGNED_OUT);
      throw error;
    }
    const created = asObject(result);
    const id = asString(created?.sessionId);
    if (!id) throw new Error("Grok CLI did not return a session id.");
    this.sessionId = id;
    this.mergeCatalog(catalogFromAcp(result));
    if (this.modelTouched && this.modelId) await this.pushModel(this.modelId);
    if (this.effortTouched && this.effort) await this.pushEffort(this.effort);
  }

  private sessionMeta(): Json {
    if (!this.developerInstructions) return {};
    return { _meta: { rules: this.developerInstructions } };
  }

  private async pushModel(modelId: string): Promise<void> {
    if (!this.rpc || !this.sessionId) return;
    try {
      await this.rpc.request("session/set_model", { sessionId: this.sessionId, modelId });
    } catch (error) {
      if (this.modelConfigId && isMethodNotFound(error)) {
        await this.rpc.request("session/set_config_option", {
          sessionId: this.sessionId,
          configId: this.modelConfigId,
          value: modelId,
        });
        return;
      }
      if (isMethodNotFound(error)) return;
      throw error;
    }
  }

  private async pushEffort(level: string): Promise<void> {
    if (!this.rpc || !this.sessionId) return;
    try {
      if (this.effortConfigId) {
        await this.rpc.request("session/set_config_option", {
          sessionId: this.sessionId,
          configId: this.effortConfigId,
          value: level,
        });
        return;
      }
      if (this.effortUsesMode || this.efforts.length > 0) {
        await this.rpc.request("session/set_mode", { sessionId: this.sessionId, modeId: level });
      }
    } catch (error) {
      if (isMethodNotFound(error)) return;
      throw error;
    }
  }

  private mergeCatalog(catalog: AcpCatalog): void {
    for (const model of catalog.models) {
      const existing = this.models.find((item) => item.id === model.id);
      if (!existing) this.models.push(model);
      else {
        if (model.name && model.name !== model.id) existing.name = model.name;
        if (model.efforts.length > 0) existing.efforts = model.efforts;
        if (model.isDefault) existing.isDefault = true;
      }
    }
    if (!this.modelTouched && catalog.currentModelId) this.modelId = catalog.currentModelId;
    if (!this.modelId) {
      const preferred = this.models.find((model) => model.isDefault) ?? this.models[0];
      if (preferred) this.modelId = preferred.id;
    }
    if (catalog.efforts.length > 0) this.efforts = catalog.efforts;
    else if (this.modelId) {
      const current = this.models.find((model) => model.id === this.modelId);
      if (current && current.efforts.length > 0) this.efforts = current.efforts;
    }
    if (!this.effortTouched && catalog.currentEffort) this.effort = catalog.currentEffort;
    if (!this.effort) this.effort = this.efforts[0] ?? "medium";
    if (catalog.modelConfigId) this.modelConfigId = catalog.modelConfigId;
    if (catalog.effortConfigId) this.effortConfigId = catalog.effortConfigId;
    if (catalog.effortUsesMode) this.effortUsesMode = true;
  }

  private stateData(): Json {
    const current = this.models.find((model) => model.id === this.modelId);
    return {
      model: this.modelId
        ? { id: this.modelId, provider: "grok", name: current?.name ?? this.modelId }
        : null,
      thinkingLevel: this.effort ?? "medium",
    };
  }

  private thinkingLevels(): string[] {
    const levels = this.efforts.length > 0 ? [...this.efforts] : [this.effort ?? "medium"];
    const selected = this.effort ?? levels[0];
    if (selected && !levels.includes(selected)) levels.unshift(selected);
    return levels;
  }

  private projectRoot(): string {
    return resolve(this.cwd);
  }

  private completeTurn(stopReason: string | null, errorMessage?: string): void {
    if (this.stopped || !this.running) return;
    this.running = false;
    this.suppressUserChunks = false;
    finishHistoryTurn(this.history);
    for (const event of closeTurn(this.stream, stopReason, errorMessage)) {
      this.host?.emit(event);
    }
  }

  private onNotification(method: string, params: Json): void {
    if (method !== "session/update") return;
    const update = asObject(params.update);
    if (!update) return;
    if (update.sessionUpdate === "usage_update") this.noteUsage(update);
    if (update.sessionUpdate === "available_commands_update") this.noteCommands(update);
    if (update.sessionUpdate === "config_option_update") this.mergeCatalog(catalogFromAcp(update));
    const skipUser = this.suppressUserChunks && update.sessionUpdate === "user_message_chunk";
    if (!skipUser) recordSessionUpdate(this.history, update);
    if (this.suppressLive || !this.host) return;
    for (const event of mapSessionUpdate(update, this.stream)) {
      this.host.emit(event);
    }
  }

  private noteUsage(update: Json): void {
    const used =
      typeof update.used === "number"
        ? update.used
        : typeof update.totalTokens === "number"
          ? update.totalTokens
          : 0;
    const size = typeof update.size === "number" ? update.size : null;
    const cost = asObject(update.cost);
    const amount = typeof cost?.amount === "number" ? cost.amount : 0;
    this.stats = {
      tokens: { total: used },
      cost: amount,
      contextUsage: size ? { tokens: used, contextWindow: size, percent: size ? (used / size) * 100 : null } : {},
    };
  }

  private noteCommands(update: Json): void {
    const commands = Array.isArray(update.availableCommands) ? update.availableCommands : [];
    this.commands = commands
      .map((command) => {
        const row = asObject(command);
        const name = asString(row?.name);
        if (!name) return null;
        return { name, description: asString(row?.description) ?? "", source: "grok" };
      })
      .filter((command): command is { name: string; description: string; source: string } => command !== null);
  }

  private onServerRequest(method: string, id: string | number, params: Json): void {
    switch (method) {
      case "session/request_permission":
        this.askPermission(id, params);
        return;
      case "fs/read_text_file":
        this.readFile(id, params);
        return;
      case "fs/write_text_file":
        this.writeFile(id, params);
        return;
      case "terminal/create":
        this.createTerminal(id, params);
        return;
      case "terminal/output":
        this.terminalOutput(id, params);
        return;
      case "terminal/wait_for_exit":
        this.terminalWait(id, params);
        return;
      case "terminal/kill":
        this.terminalKill(id, params, false);
        return;
      case "terminal/release":
        this.terminalKill(id, params, true);
        return;
      default:
        this.rpc?.fail(id, "Pitex cannot answer this Grok request.");
        this.host?.emit({
          type: "extension_error",
          error: `Skipped an unsupported Grok request (${method}).`,
        });
    }
  }

  private askPermission(id: string | number, params: Json): void {
    const approval = approvalFromPermission(id, params);
    this.pending.set(String(id), {
      kind: "permission",
      rpcId: id,
      options: permissionOptions(params),
    });
    this.host?.emit({
      type: "approval_request",
      id: approval.id,
      kind: approval.kind,
      title: approval.title,
      detail: approval.detail,
    });
  }

  private readFile(id: string | number, params: Json): void {
    const path = asString(params.path);
    if (!path) {
      this.rpc?.fail(id, "The file path was missing.");
      return;
    }
    const result = readProjectFile(this.projectRoot(), path, optionalInt(params.line), optionalInt(params.limit));
    if (!result.ok) this.rpc?.fail(id, result.error);
    else this.rpc?.respond(id, { content: result.content });
  }

  private writeFile(id: string | number, params: Json): void {
    const path = asString(params.path);
    if (!path || typeof params.content !== "string") {
      this.rpc?.fail(id, "The file write was missing a path or text.");
      return;
    }
    const result = writeProjectFile(this.projectRoot(), path, params.content);
    if (!result.ok) this.rpc?.fail(id, result.error);
    else this.rpc?.respond(id, {});
  }

  private createTerminal(id: string | number, params: Json): void {
    const cwd = terminalWorkingDirectory(this.projectRoot(), params.cwd);
    if (!cwd) {
      this.rpc?.fail(id, "That path is outside this project.");
      return;
    }
    const command = asString(params.command);
    if (!command) {
      this.rpc?.fail(id, "The terminal command was missing.");
      return;
    }
    if (this.terminalAlways) {
      this.spawnTerminal(id, params, cwd, command);
      return;
    }
    const args = Array.isArray(params.args)
      ? params.args.filter((part): part is string => typeof part === "string")
      : [];
    this.pending.set(String(id), { kind: "terminal", rpcId: id, options: [], terminal: params });
    this.host?.emit({
      type: "approval_request",
      id: String(id),
      kind: "command",
      title: "Run command",
      detail: [command, ...args].join(" "),
    });
  }

  private spawnTerminal(id: string | number, params: Json, cwd: string, command: string): void {
    const args = Array.isArray(params.args)
      ? params.args.filter((part): part is string => typeof part === "string")
      : [];
    const limit = optionalInt(params.outputByteLimit);
    const term = this.startTerminal(command, args, cwd, limit && limit > 0 ? limit : 1_000_000, params.env);
    this.rpc?.respond(id, { terminalId: term.id });
  }

  private startTerminal(
    command: string,
    args: string[],
    cwd: string,
    limit: number,
    env: unknown
  ): LiveTerminal {
    let settle = () => {};
    const done = new Promise<void>((resolve) => {
      settle = resolve;
    });
    const child = spawn(command, args, {
      cwd,
      env: terminalEnv(this.env, env),
      shell: false,
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    const term: LiveTerminal = {
      id: `term-${randomUUID()}`,
      child,
      output: "",
      truncated: false,
      limit,
      exited: false,
      exitCode: null,
      signal: null,
      done,
      settle,
    };
    const finish = (code: number | null, signal: NodeJS.Signals | null) => {
      if (term.exited) return;
      term.exited = true;
      term.exitCode = code;
      term.signal = signal;
      term.settle();
    };
    child.stdout?.on("data", (chunk: Buffer) => rememberTerminalOutput(term, chunk));
    child.stderr?.on("data", (chunk: Buffer) => rememberTerminalOutput(term, chunk));
    child.on("error", (error) => {
      rememberTerminalOutput(term, Buffer.from(error.message));
      finish(127, null);
    });
    child.on("exit", (code, signal) => finish(code, signal));
    this.terminals.set(term.id, term);
    return term;
  }

  private terminalOutput(id: string | number, params: Json): void {
    const term = this.terminals.get(asString(params.terminalId) ?? "");
    if (!term) {
      this.rpc?.fail(id, "That terminal is not running.");
      return;
    }
    const result: Json = { output: term.output, truncated: term.truncated };
    if (term.exited) result.exitStatus = { exitCode: term.exitCode, signal: term.signal };
    this.rpc?.respond(id, result);
  }

  private terminalWait(id: string | number, params: Json): void {
    const term = this.terminals.get(asString(params.terminalId) ?? "");
    if (!term) {
      this.rpc?.fail(id, "That terminal is not running.");
      return;
    }
    void term.done.then(() => {
      this.rpc?.respond(id, { exitCode: term.exitCode, signal: term.signal });
    });
  }

  private terminalKill(id: string | number, params: Json, release: boolean): void {
    const key = asString(params.terminalId) ?? "";
    const term = this.terminals.get(key);
    if (term && !term.exited) {
      try {
        term.child.kill();
      } catch {
        // The process can exit between the check and the signal.
      }
    }
    if (release) this.terminals.delete(key);
    this.rpc?.respond(id, {});
  }

  private denyOutstanding(reason: "cancelled" | "deny"): void {
    const outstanding = [...this.pending.values()];
    this.pending.clear();
    if (!this.rpc) return;
    for (const pending of outstanding) {
      if (pending.kind === "permission") {
        this.rpc.respond(
          pending.rpcId,
          reason === "cancelled" ? cancelledPermission() : permissionResult("deny", pending.options)
        );
      } else {
        this.rpc.fail(pending.rpcId, "The turn was cancelled.");
      }
    }
  }

  private resolveApproval(command: PiCommand): void {
    const id = command.id ?? "";
    const pending = this.pending.get(id);
    if (!pending || !this.rpc) return;
    this.pending.delete(id);
    if (pending.kind === "permission") {
      this.rpc.respond(pending.rpcId, permissionResult(command.decision, pending.options));
      return;
    }
    const decision = command.decision === "approve" || command.decision === "always_allow" ? command.decision : "deny";
    if (decision === "deny") {
      this.rpc.fail(pending.rpcId, "The user denied the terminal request.");
      return;
    }
    if (decision === "always_allow") this.terminalAlways = true;
    const params = pending.terminal ?? {};
    const cwd = terminalWorkingDirectory(this.projectRoot(), params.cwd);
    const commandText = asString(params.command);
    if (!cwd || !commandText) {
      this.rpc.fail(pending.rpcId, "That path is outside this project.");
      return;
    }
    this.spawnTerminal(pending.rpcId, params, cwd, commandText);
  }

  private killTerminals(): void {
    for (const term of this.terminals.values()) {
      if (term.exited) continue;
      try {
        term.child.kill();
      } catch {
        // Already exited.
      }
    }
    this.terminals.clear();
  }
}

function joinSession(sessionDir: string, name: string): string {
  return sessionDir.endsWith("/") ? sessionDir + name : `${sessionDir}/${name}`;
}
