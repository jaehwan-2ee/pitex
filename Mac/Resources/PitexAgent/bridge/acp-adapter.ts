// ACP adapter for official CLIs that speak the Agent Client Protocol on stdio
// (JSON-RPC, one message per line): the Grok CLI (`grok --no-auto-update agent
// stdio`) and the Antigravity CLI (`agy acp`). Per-CLI differences live in
// acp-profiles.ts. Each CLI keeps its own login. This file never reads,
// copies, or parses credential or session files, and it never refreshes tokens.
//
// Protocol: Grok CLI ACP guide, https://docs.x.ai/build/cli/headless-scripting
// (initialize, authenticate with methodId "cached_token", session/new,
// session/prompt, session/update, session/request_permission, session/cancel,
// session/load when advertised, session/set_model). ACP v1 filesystem and
// terminal methods are enabled for Grok and jailed to the project directory.
// agy (1.5.15, measured on the wire) needs no authenticate call, runs its own
// tools, asks through session/request_permission, and lists its sessions with
// session/list. The agy quirks handled here (late history replay after
// session/load, end_turn while a command still runs) follow Paseo's
// Antigravity provider (github.com/getpaseo/paseo, plugins/antigravity-provider).
//
// The adapter shape follows Pitex's Codex adapter, which was adapted from
// T3 Code v0.1.4 (MIT), Copyright (c) 2026 T3 Tools Inc.
// https://github.com/pingdotgg/t3code
// This file is original Pitex code, not a copy of T3's Grok driver.

import { spawn, type ChildProcess, type ChildProcessWithoutNullStreams } from "node:child_process";
import { randomUUID } from "node:crypto";
import { accessSync, existsSync, constants as fsConstants, readFileSync, realpathSync, utimesSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join, resolve } from "node:path";

import type { AdapterHost, AdapterStartOptions, CliAdapter, ModeState, PiCommand } from "./adapter.ts";
import type { AcpProfile } from "./acp-profiles.ts";
import { stopCodexChild } from "./codex-adapter.ts";
import {
  approvalFromPermission,
  approvalKey,
  cancelledPermission,
  catalogFromAcp,
  closeTurn,
  createHistory,
  createStreamState,
  finishHistoryTurn,
  historyMessages,
  mapSessionUpdate,
  permissionActions,
  permissionOptions,
  permissionResult,
  permissionResultForAction,
  pushUserMessage,
  readProjectFile,
  recordSessionUpdate,
  terminalWorkingDirectory,
  visibleModels,
  writeProjectFile,
  type AcpCatalog,
  type AcpModel,
  type HistoryState,
  type Json,
  type PermissionOption,
  type StreamState,
} from "./acp-map.ts";
import { redactSecrets } from "./jsonrpc.ts";
import {
  appendUserMessage,
  loadIndex,
  rememberSession,
  sessionModeFor,
  saveIndex,
  sessionKey,
  foreignSessionMessage,
  sessionBackend,
  threadIdFor,
  writeSessionPrelude,
} from "./session-index.ts";

const PROMPT_TIMEOUT_MS = 30 * 60 * 1000;
const LOAD_TIMEOUT_MS = 120_000;
const LIST_TIMEOUT_MS = 10_000;
/** Most session/list rows read per scan, across pages. */
const SESSION_SCAN_LIMIT = 500;
/** Longest wait for a load replay to go quiet. */
const REPLAY_WAIT_LIMIT_MS = 10_000;
/** After a deferred end_turn and its commands, close this long after the follow-up reply's usage_update. */
const DEFER_SETTLE_MS = 500;
/** ... or this long after the last update while no finished reply has arrived (agy asks the model again first). */
const DEFER_QUIET_LIMIT_MS = 120_000;
const SAFE_SESSION_ID = /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/;

export function findCli(profile: AcpProfile, env: NodeJS.ProcessEnv, platform = process.platform): string | null {
  const explicit = env[profile.pathEnv];
  if (explicit && isExecutable(explicit)) return explicit;
  const directories = (env.PATH ?? "").split(delimiter).filter(Boolean);
  const home = env.HOME || homedir();
  directories.push(join(home, ".local", "bin"), "/usr/local/bin", "/opt/homebrew/bin");
  const base = profile.binary;
  const names = platform === "win32" ? [`${base}.exe`, `${base}.cmd`, base] : [base];
  for (const directory of directories) {
    for (const name of names) {
      const candidate = join(directory, name);
      if (isExecutable(candidate)) return candidate;
    }
  }
  return null;
}

export interface CliLaunchSpec {
  command: string;
  args: string[];
  windowsVerbatimArguments?: boolean;
}

/**
 * The profile's fixed ACP arguments (`grok --no-auto-update agent stdio`,
 * `agy acp --sandbox --disable-telemetry`).
 * Windows CreateProcess cannot run a shebang. A node or bun shebang is
 * started with this process's node. The check reads only the first 128
 * bytes of the executable. A `.cmd` / `.bat` shim is started through
 * `cmd.exe /d /s /c` with those fixed arguments only. `%` and other cmd
 * metacharacters are rejected. `.exe` is spawned directly.
 */
export function cliLaunchSpec(profile: AcpProfile, binary: string, platform = process.platform): CliLaunchSpec {
  const direct = { command: binary, args: [...profile.args] };
  if (platform !== "win32") return direct;
  if (/\.exe$/i.test(binary)) return direct;
  if (/\.(cmd|bat)$/i.test(binary)) {
    const quoted = quoteCmdExecutable(binary);
    if (!quoted) throw new Error(`${profile.displayName} path cannot be launched through cmd.exe.`);
    return {
      command: "cmd.exe",
      args: ["/d", "/s", "/c", `${quoted} ${profile.args.join(" ")}`],
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
    return { command: process.execPath, args: [binary, ...profile.args] };
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
  const code = error instanceof AcpRpcError ? error.code : undefined;
  return code === -32601 || /method not found/i.test(message);
}

/** The profile's sign-in message for an auth failure. The CLI's own error text is not forwarded. */
function authFailure(profile: AcpProfile, error: unknown): string | null {
  const message = error instanceof Error ? error.message : "";
  if (profile.keyringLocked && /keyring/i.test(message)) return profile.keyringLocked;
  return /auth|not logged in|log back in/i.test(message) ? profile.signedOut : null;
}

/** Same folder on disk, so `/tmp/x` and its realpath compare equal. */
function canonicalDir(path: string): string {
  try {
    return realpathSync(resolve(path));
  } catch {
    return resolve(path);
  }
}

/**
 * Mirror files for the CLI's own sessions in this project, so the history
 * list (which scans `*.jsonl`) shows them. Each row is `{sessionId, cwd,
 * title, updatedAt}` from ACP session/list. A row is skipped when its cwd is
 * another folder or its id is already in the session index, so a rescan is a
 * no-op. An existing file is never overwritten. Returns the files written.
 */
export function importListedSessions(sessionDir: string, prefix: string, rows: unknown[], projectRoot: string): string[] {
  const root = canonicalDir(projectRoot);
  const index = loadIndex(sessionDir);
  const known = new Set(Object.values(index.sessions).map((record) => record.threadId));
  const written: string[] = [];
  let changed = false;
  for (const raw of rows) {
    const row = asObject(raw);
    const id = asString(row?.sessionId);
    const cwd = asString(row?.cwd);
    if (!id || !cwd || !SAFE_SESSION_ID.test(id) || known.has(id) || canonicalDir(cwd) !== root) continue;
    const path = joinSession(sessionDir, `${prefix}-${id}.jsonl`);
    known.add(id);
    // One index write per scan instead of a rememberSession call per row.
    index.sessions[sessionKey(path)] = { threadId: id, updatedAt: new Date().toISOString() };
    changed = true;
    if (existsSync(path)) continue;
    writeSessionPrelude(path, asString(row?.title)?.trim() || "Untitled conversation", projectRoot);
    const updated = Date.parse(asString(row?.updatedAt) ?? "");
    if (Number.isFinite(updated)) utimesSync(path, new Date(updated), new Date(updated));
    written.push(path);
  }
  if (changed) saveIndex(sessionDir, index);
  return written;
}

/** A foreground command that agy may still be running after it sends end_turn. Daemons never finish. */
function blocksTurnEnd(update: Json): boolean {
  const input = asObject(update.rawInput) ?? asObject(update.args);
  return (update.name === "run_command" || update.kind === "execute") && input?.IsDaemon !== true;
}

class AcpRpcError extends Error {
  readonly code: number;
  constructor(message: string, code = -32000) {
    super(message);
    this.name = "AcpRpcError";
    this.code = code;
  }
}

interface PendingRpc {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timer: NodeJS.Timeout;
}

/**
 * Newline-delimited JSON-RPC for the ACP child.
 * Inbound messages go through the shared secret redaction before dispatch.
 * This class does not read credential files.
 */
class AcpRpc {
  private nextId = 1;
  private buffer = "";
  private pending = new Map<number, PendingRpc>();
  private closed = false;
  private readonly child: ChildProcessWithoutNullStreams;
  private readonly onNotification: (method: string, params: Json) => void;
  private readonly onServerRequest: (method: string, id: string | number, params: Json) => void;
  private readonly onClose?: (message: string) => void;
  private readonly name: string;

  constructor(
    name: string,
    child: ChildProcessWithoutNullStreams,
    onNotification: (method: string, params: Json) => void,
    onServerRequest: (method: string, id: string | number, params: Json) => void,
    onClose?: (message: string) => void
  ) {
    this.name = name;
    this.child = child;
    this.onNotification = onNotification;
    this.onServerRequest = onServerRequest;
    this.onClose = onClose;
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => this.ingest(chunk));
    child.on("exit", () => {
      this.failAll(`${name} exited.`);
      this.onClose?.(`${name} exited.`);
    });
    child.on("error", (error) => {
      this.failAll(error.message);
      this.onClose?.(error.message);
    });
    child.stdin.on("error", (error: NodeJS.ErrnoException) => {
      const message =
        error.code === "EPIPE" ? `${name} is unavailable.` : `${name} is unavailable (${error.message}).`;
      this.failAll(message);
      this.onClose?.(message);
    });
  }

  request(method: string, params: unknown, timeoutMs = 30_000): Promise<unknown> {
    if (this.closed) return Promise.reject(new AcpRpcError(`${this.name} is not running.`));
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new AcpRpcError(`${this.name} timed out waiting for ${method}.`));
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
    this.failAll(`${this.name} stopped.`);
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
        waiter.reject(new AcpRpcError(error.message || `${this.name} request failed.`, error.code ?? -32000));
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
      waiter.reject(new AcpRpcError(message));
      this.pending.delete(id);
    }
  }
}

interface PendingApproval {
  kind: "permission" | "terminal";
  rpcId: string | number;
  options: PermissionOption[];
  terminal?: Json;
  /** approvalKey of the request, for the session-only "Always allow" fallback. */
  key?: string;
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

/**
 * `terminal/create` arrives in two shapes. A filled `args` list is the
 * program plus its argv, and those strings are not a shell line. An empty
 * list means `command` is the whole shell command, which is what the Grok
 * CLI sends (`/bin/bash -lc 'echo pitex-ok'`).
 */
export function terminalSpawnSpec(
  command: string,
  args: readonly string[],
  platform = process.platform
): { command: string; args: string[]; verbatim: boolean } {
  if (args.length > 0) return { command, args: [...args], verbatim: false };
  if (platform === "win32") {
    return { command: "cmd.exe", args: ["/d", "/s", "/c", `"${command}"`], verbatim: true };
  }
  return { command: "/bin/sh", args: ["-c", command], verbatim: false };
}

/** SIGKILL the terminal's process group. The shell and a background child share that group. */
export function killProcessGroup(child: ChildProcess): void {
  const pid = child.pid;
  if (pid && process.platform !== "win32") {
    try {
      process.kill(-pid, "SIGKILL");
      return;
    } catch {
    }
  }
  try {
    child.kill("SIGKILL");
  } catch {
    // Already exited.
  }
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

export class AcpAdapter implements CliAdapter {
  readonly id: string;
  readonly displayName: string;
  readonly profile: AcpProfile;
  unavailableReason: string | null = null;

  private rpc: AcpRpc | null = null;
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
  private models: AcpModel[] = [];
  private efforts: string[] = [];
  private modelConfigId: string | null = null;
  private effortConfigId: string | null = null;
  private effortUsesMode = false;
  private modeConfigId: string | null = null;
  private currentMode: string | null = null;
  /** The CLI's permission modes (catalog), filtered to profile.modes for the picker. */
  private modeOptions: ModeState["options"] = [];
  /** Mode the user picked for this conversation; null means profile.forceMode. */
  private selectedMode: string | null = null;
  private loadSession = false;
  private listSessions = false;
  private systemPromptAppend = false;
  private pending = new Map<string, PendingApproval>();
  private terminals = new Map<string, LiveTerminal>();
  private terminalAlways = false;
  /** Session-only "Always allow" for a CLI that offers no allow_always option. */
  private allowRules = new Set<string>();
  private commands: Json[] = [];
  private running = false;
  /** A prompt is waiting for session/new or the mode check before it is sent. */
  private starting = false;
  private turn = 0;
  /** Prompts sent while a turn runs. Each one starts after the turn before it ends. */
  private queue: string[] = [];
  /** Foreground commands still running in this turn (deferTurnOnRunningTools). */
  private openTools = new Set<string>();
  private deferredStop: string | null = null;
  private deferTimer: NodeJS.Timeout | null = null;
  /** Reply after the last tool: idle, streaming text, or finished by a usage_update. */
  private reply: "idle" | "open" | "settled" = "idle";
  /** Tool calls that went through session/request_permission (CLIs that run their own commands). */
  private asked = new Set<string>();
  private noticeShown = false;
  private unaskedWarned = false;
  private ignoreUpdates = false;
  private suppressUserChunks = false;
  private suppressLive = false;
  private lastReplayAt = 0;
  private acceptClose = false;
  private stopped = false;
  private stats: Json = { tokens: { total: 0 }, cost: 0, contextUsage: {} };

  constructor(profile: AcpProfile) {
    this.profile = profile;
    this.id = profile.id;
    this.displayName = profile.displayName;
  }

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
      this.unavailableReason = error instanceof Error ? error.message : this.profile.missing;
    }
  }

  private async connect(env: NodeJS.ProcessEnv): Promise<void> {
    const profile = this.profile;
    const binary = findCli(profile, env);
    if (!binary) throw new Error(profile.missing);
    this.acceptClose = true;
    const launch = cliLaunchSpec(profile, binary);
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
      // CLI diagnostics stay off the pi event stream.
    });
    this.rpc = new AcpRpc(
      profile.displayName,
      child,
      (method, params) => this.onNotification(method, params),
      (method, id, params) => this.onServerRequest(method, id, params),
      (message) => this.markClosed(message)
    );
    const init = asObject(
      await this.rpc.request("initialize", {
        protocolVersion: 1,
        clientInfo: { name: "pitex", title: "Pitex", version: "0.1.0" },
        clientCapabilities: profile.clientCapabilities,
      })
    );
    if (!init) throw new Error(profile.missing);
    const version = init.protocolVersion;
    if (version !== undefined && version !== 1 && version !== "1") {
      throw new Error(`${profile.displayName} speaks an ACP version Pitex does not support.`);
    }
    const capabilities = asObject(init.agentCapabilities);
    this.loadSession = capabilities?.loadSession === true;
    const listing = asObject(capabilities?.sessionCapabilities)?.list;
    this.listSessions = listing !== undefined && listing !== null && listing !== false;
    const antigravity = asObject(asObject(capabilities?._meta)?.antigravity);
    this.systemPromptAppend = asObject(antigravity?.systemPrompt)?.append === true;
    if (profile.auth === "cached_token") {
      // Official ACP example: CLI login is methodId "cached_token".
      // Pitex does not select an API-key method and does not read token files.
      if (!authMethodIds(init).includes("cached_token")) throw new Error(profile.signedOut);
      try {
        await this.rpc.request("authenticate", { methodId: "cached_token", _meta: { headless: true } });
      } catch {
        throw new Error(profile.signedOut);
      }
    }
    this.mergeCatalog(catalogFromAcp(init, profile));
    if (profile.modelSource === "all" && this.models.length === 0) {
      try {
        this.mergeCatalog(catalogFromAcp(await this.rpc.request("x.ai/models/list", {}, 3_000), profile));
      } catch {
        // The extension is optional. Session setup can still advertise models.
      }
    }
    // The host's model-menu check runs with a throwaway session folder; it only needs the catalog.
    if (env.PITEX_CATALOG_PROBE !== "1") await this.importSessions();
    // Config-option models arrive only with a session. A session that never
    // gets a prompt is not saved by agy, so this leaves nothing in its history.
    if (profile.modelSource === "config") await this.openSession(null);
  }

  private markClosed(message: string): void {
    if (!this.acceptClose) return;
    this.acceptClose = false;
    this.unavailableReason = message;
    this.running = false;
    this.dropPending();
    this.host?.emit({ type: "extension_error", error: message });
  }

  async handleCommand(command: PiCommand, host: AdapterHost): Promise<void> {
    this.host = host;
    if (this.profile.notice && !this.noticeShown && !this.unavailableReason) {
      this.noticeShown = true;
      host.emit({ type: "extension_error", error: this.profile.notice });
    }
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
          // ACP has no steer. Mid-turn text waits in the queue like a typed follow-up.
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
          host.respond(
            "get_messages",
            true,
            { messages: historyMessages(this.history), ...(this.history.todos ? { todos: this.history.todos } : {}) },
            undefined,
            command.id
          );
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
        case "set_mode":
          await this.setMode(command, host);
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
      const message = error instanceof Error ? error.message : `${this.displayName} request failed.`;
      host.respond(command.type, false, undefined, message, command.id);
    }
  }

  async stop(): Promise<void> {
    this.stopped = true;
    this.acceptClose = false;
    this.clearDeferred();
    this.queue = [];
    this.killTerminals();
    const child = this.child;
    this.dropPending();
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
    if (this.running || this.starting) {
      this.queue.push(message);
      host.respond("prompt", true, { queued: true }, undefined, command.id);
      return;
    }
    await this.startTurn(message);
    host.respond("prompt", true, {}, undefined, command.id);
  }

  private async startTurn(message: string): Promise<void> {
    const turn = ++this.turn;
    this.starting = true;
    try {
      if (!this.sessionId) await this.openSession(null);
      // The CLI's mode can change mid-session (`/plan`, its settings, an update dropped after an
      // abort), so set the selected one before every turn instead of trusting the cached value.
      else if (this.profile.forceMode) await this.forceMode();
    } finally {
      if (turn === this.turn) this.starting = false;
    }
    // Stop, a new session, or a switch arrived while the session was starting.
    if (turn !== this.turn) return;
    if (!this.sessionPath) {
      this.sessionPath = joinSession(this.sessionDir, `${this.profile.id}-${randomUUID()}.jsonl`);
      writeSessionPrelude(this.sessionPath, message, this.cwd);
      if (this.sessionId) this.remember();
    } else {
      appendUserMessage(this.sessionPath, message);
    }
    pushUserMessage(this.history, message);
    this.suppressUserChunks = true;
    this.ignoreUpdates = false;
    this.running = true;
    this.openTools.clear();
    this.reply = "idle";
    const pending = this.rpc?.request(
      "session/prompt",
      {
        sessionId: this.sessionId,
        prompt: [{ type: "text", text: message }],
      },
      PROMPT_TIMEOUT_MS
    );
    void pending
      ?.then((result) => {
        if (turn !== this.turn) return;
        this.endTurn(asString(asObject(result)?.stopReason));
      })
      .catch((error: unknown) => {
        if (turn !== this.turn) return;
        const signedOut = authFailure(this.profile, error);
        if (signedOut) {
          this.unavailableReason = signedOut;
          this.queue = [];
          this.completeTurn("refusal", signedOut);
          return;
        }
        const text = error instanceof Error ? error.message : `${this.displayName} request failed.`;
        this.completeTurn("refusal", text);
      });
  }

  /** agy can send end_turn while a foreground command still runs; the turn stays open until it ends. */
  private endTurn(stopReason: string | null): void {
    if (this.profile.deferTurnOnRunningTools && stopReason === "end_turn" && this.openTools.size > 0) {
      this.deferredStop = stopReason;
      return;
    }
    this.completeTurn(stopReason);
  }

  private trackTool(update: Json): void {
    const id = asString(update.toolCallId);
    if (!id) return;
    const status = asString(update.status);
    if (status === "completed" || status === "failed") this.openTools.delete(id);
    else if (blocksTurnEnd(update)) {
      this.openTools.add(id);
      this.reply = "idle";
    }
  }

  /**
   * After a deferred end_turn, agy asks the model again once the commands end. Close
   * DEFER_SETTLE_MS after the usage_update that follows that reply's text, or after
   * DEFER_QUIET_LIMIT_MS of quiet when no reply comes (Paseo's antigravity provider does the same).
   */
  private settleDeferred(): void {
    if (this.deferTimer) clearTimeout(this.deferTimer);
    this.deferTimer = null;
    if (this.deferredStop === null || this.openTools.size > 0) return;
    const stop = this.deferredStop;
    const wait = this.reply === "settled" ? DEFER_SETTLE_MS : DEFER_QUIET_LIMIT_MS;
    this.deferTimer = setTimeout(() => this.completeTurn(stop), wait);
  }

  private clearDeferred(): void {
    if (this.deferTimer) clearTimeout(this.deferTimer);
    this.deferTimer = null;
    this.deferredStop = null;
    this.openTools.clear();
    this.reply = "idle";
  }

  /** Cancels a prompt still waiting for its session; true when one was waiting. */
  private cancelStart(): boolean {
    const waiting = this.starting;
    this.starting = false;
    this.turn += 1;
    return waiting;
  }

  private async abort(host: AdapterHost, id?: string): Promise<void> {
    const waiting = this.cancelStart();
    this.queue = [];
    this.ignoreUpdates = true;
    this.denyOutstanding("cancelled");
    await this.killTerminals();
    if (this.rpc && this.sessionId) this.rpc.notify("session/cancel", { sessionId: this.sessionId });
    if (this.running) this.completeTurn("cancelled");
    else if (waiting || this.stream.turnOpen) {
      for (const event of closeTurn(this.stream, "cancelled")) host.emit(event);
    }
    host.respond("abort", true, {}, undefined, id);
  }

  private async newSession(host: AdapterHost, id?: string): Promise<void> {
    this.cancelStart();
    this.queue = [];
    this.ignoreUpdates = true;
    this.denyOutstanding("cancelled");
    await this.killTerminals();
    if (this.rpc && this.sessionId && this.running) {
      this.rpc.notify("session/cancel", { sessionId: this.sessionId });
    }
    if (this.running) this.completeTurn("cancelled");
    this.sessionId = null;
    this.sessionPath = null;
    this.history = createHistory();
    this.stream = createStreamState();
    this.terminalAlways = false;
    this.allowRules.clear();
    this.asked.clear();
    // A new conversation starts in the profile's mode; accept-edits is picked again per chat.
    this.selectedMode = null;
    this.emitMode();
    // Sessions started in a terminal since the last scan join the history list here.
    await this.importSessions();
    host.respond("new_session", true, {}, undefined, id);
  }

  private async switchSession(command: PiCommand, host: AdapterHost): Promise<void> {
    this.ignoreUpdates = false;
    const sessionPath = command.sessionPath ?? "";
    const owner = sessionBackend(sessionPath);
    if (owner && owner !== this.id) {
      host.respond("switch_session", false, undefined, foreignSessionMessage(owner), command.id);
      return;
    }
    const sessionId = threadIdFor(this.sessionDir, sessionPath);
    if (!sessionId) {
      host.respond(
        "switch_session",
        false,
        undefined,
        `That conversation has no ${this.displayName} session to resume.`,
        command.id
      );
      return;
    }
    if (!this.loadSession) {
      host.respond("switch_session", false, undefined, this.profile.noResume, command.id);
      return;
    }
    this.cancelStart();
    this.queue = [];
    if (this.running && this.rpc && this.sessionId) {
      this.denyOutstanding("cancelled");
      this.rpc.notify("session/cancel", { sessionId: this.sessionId });
      this.completeTurn("cancelled");
    }
    this.allowRules.clear();
    this.asked.clear();
    const saved = sessionModeFor(this.sessionDir, sessionPath);
    this.selectedMode = saved && this.profile.modes.includes(saved) ? saved : null;
    await this.openSession(sessionId);
    this.sessionPath = sessionPath;
    this.emitMode();
    host.respond("switch_session", true, { cancelled: false }, undefined, command.id);
  }

  private async setModel(command: PiCommand, host: AdapterHost): Promise<void> {
    this.modelTouched = true;
    this.modelId = command.modelId || this.modelId;
    if (this.sessionId && this.modelId) await this.pushModel(this.modelId);
    this.remember();
    host.respond("set_model", true, {}, undefined, command.id);
  }

  private async setEffort(command: PiCommand, host: AdapterHost): Promise<void> {
    if (this.profile.modelSource === "config" && this.sessionId && !this.effortConfigId) {
      host.respond("set_thinking_level", false, undefined, "This model has no thinking level.", command.id);
      return;
    }
    this.effortTouched = true;
    this.effort = command.level || this.effort;
    if (this.sessionId && this.effort) await this.pushEffort(this.effort);
    host.respond("set_thinking_level", true, {}, undefined, command.id);
  }

  private async setMode(command: PiCommand, host: AdapterHost): Promise<void> {
    const modeId = command.modeId ?? "";
    if (!this.modeState()?.options.some((option) => option.id === modeId)) {
      host.respond("set_mode", false, undefined, `${this.displayName} cannot use that permission mode.`, command.id);
      return;
    }
    const previous = this.selectedMode;
    this.selectedMode = modeId;
    try {
      if (this.sessionId) await this.forceMode();
    } catch (error) {
      this.selectedMode = previous;
      throw error;
    }
    this.remember();
    this.emitMode();
    host.respond("set_mode", true, { mode: this.modeState() }, undefined, command.id);
  }

  /** The picker state, or null when this profile has no user-selectable modes or the CLI listed none. */
  private modeState(): ModeState | null {
    const options = this.modeOptions.filter((option) => this.profile.modes.includes(option.id));
    const id = this.selectedMode ?? this.profile.forceMode;
    if (options.length === 0 || !id) return null;
    return { id, options };
  }

  private emitMode(): void {
    const mode = this.modeState();
    if (mode) this.host?.emit({ type: "mode_changed", mode });
  }

  private remember(): void {
    if (!this.sessionPath || !this.sessionId) return;
    rememberSession(this.sessionDir, this.sessionPath, this.sessionId, this.modelId ?? undefined, this.selectedMode ?? undefined);
  }

  private async openSession(resumeId: string | null): Promise<void> {
    if (!this.rpc) throw new Error(this.profile.missing);
    if (resumeId) {
      if (!this.loadSession) throw new Error(this.profile.noResume);
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
        this.mergeCatalog(catalogFromAcp(result, this.profile));
        await this.waitForReplay();
      } catch (error) {
        const signedOut = authFailure(this.profile, error);
        if (signedOut) throw new Error(signedOut);
        throw error;
      } finally {
        this.suppressLive = false;
      }
      if (this.profile.forceMode) await this.forceMode();
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
      const signedOut = authFailure(this.profile, error);
      if (signedOut) throw new Error(signedOut);
      throw error;
    }
    const created = asObject(result);
    const id = asString(created?.sessionId);
    if (!id) throw new Error(`${this.displayName} did not return a session id.`);
    this.sessionId = id;
    this.mergeCatalog(catalogFromAcp(result, this.profile));
    if (this.profile.forceMode) await this.forceMode();
    if (this.modelTouched && this.modelId) await this.pushModel(this.modelId);
    if (this.effortTouched && this.effort) await this.pushEffort(this.effort);
  }

  /** agy replays history after the session/load response. Wait until it has been quiet for replayQuietMs. */
  private async waitForReplay(): Promise<void> {
    const quiet = this.profile.replayQuietMs;
    if (quiet <= 0) return;
    const deadline = Date.now() + REPLAY_WAIT_LIMIT_MS;
    this.lastReplayAt = Date.now();
    let wait = quiet;
    while (wait > 0 && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, wait));
      wait = this.lastReplayAt + quiet - Date.now();
    }
  }

  /** Switch to the profile's permission mode and check that the CLI reports it. */
  private async forceMode(): Promise<void> {
    const mode = this.selectedMode ?? this.profile.forceMode;
    if (!mode || !this.rpc || !this.sessionId) return;
    if (this.modeConfigId) {
      const result = await this.rpc.request("session/set_config_option", {
        sessionId: this.sessionId,
        configId: this.modeConfigId,
        value: mode,
      });
      this.mergeCatalog(catalogFromAcp(result, this.profile));
    } else if (this.currentMode && this.currentMode !== mode) {
      await this.rpc.request("session/set_mode", { sessionId: this.sessionId, modeId: mode });
      this.currentMode = mode;
    }
    if (this.currentMode === null) {
      throw new Error(`${this.displayName} did not report its permission mode, so Pitex did not send the prompt.`);
    }
    if (this.currentMode !== mode) {
      throw new Error(
        `${this.displayName} stayed in its "${this.currentMode}" mode instead of "${mode}", so Pitex did not send the prompt.`
      );
    }
  }

  private sessionMeta(): Json {
    if (!this.developerInstructions) return {};
    if (this.profile.instructions === "rules") return { _meta: { rules: this.developerInstructions } };
    // shortcut: an agy that does not advertise systemPrompt.append gets no editor instructions; prefix the first prompt if one ever ships.
    if (!this.systemPromptAppend) return {};
    return { _meta: { antigravity: { systemPrompt: { append: this.developerInstructions } } } };
  }

  private async pushModel(modelId: string): Promise<void> {
    if (!this.rpc || !this.sessionId) return;
    if (this.profile.modelSource === "config") {
      if (!this.modelConfigId) return;
      const result = await this.rpc.request("session/set_config_option", {
        sessionId: this.sessionId,
        configId: this.modelConfigId,
        value: modelId,
      });
      // The reply lists the new model's thought_level, or none (Orchestrator).
      this.mergeCatalog(catalogFromAcp(result, this.profile));
      return;
    }
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
        const result = await this.rpc.request("session/set_config_option", {
          sessionId: this.sessionId,
          configId: this.effortConfigId,
          value: level,
        });
        if (this.profile.modelSource === "config") this.mergeCatalog(catalogFromAcp(result, this.profile));
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

  private async importSessions(): Promise<void> {
    if (!this.profile.importSessions || !this.listSessions || !this.rpc) return;
    const rows: unknown[] = [];
    let cursor: string | null = null;
    try {
      do {
        const params: Json = { cwd: this.projectRoot() };
        if (cursor) params.cursor = cursor;
        const page = asObject(await this.rpc.request("session/list", params, LIST_TIMEOUT_MS));
        const sessions = Array.isArray(page?.sessions) ? page.sessions : [];
        rows.push(...sessions);
        cursor = asString(page?.nextCursor);
      } while (cursor && rows.length < SESSION_SCAN_LIMIT);
    } catch {
      // The history list is a convenience. A failed scan leaves it as it was.
      return;
    }
    try {
      importListedSessions(this.sessionDir, this.profile.id, rows.slice(0, SESSION_SCAN_LIMIT), this.projectRoot());
    } catch {
      // An unwritable session folder must not stop the chat.
    }
  }

  private mergeCatalog(catalog: AcpCatalog): void {
    if (catalog.modeConfigId) this.modeConfigId = catalog.modeConfigId;
    if (catalog.currentMode) this.currentMode = catalog.currentMode;
    if (catalog.modes.length > 0) this.modeOptions = catalog.modes;
    if (this.profile.modelSource === "config") {
      // Each config payload is the whole picture: the model list, and the
      // thought_level of the selected model or none at all.
      if (!catalog.modelConfigId) return;
      this.models = catalog.models;
      this.modelConfigId = catalog.modelConfigId;
      if (catalog.currentModelId && (!this.modelTouched || !this.models.some((model) => model.id === this.modelId))) {
        this.modelId = catalog.currentModelId;
      }
      this.efforts = catalog.efforts;
      this.effortConfigId = catalog.effortConfigId;
      if (!this.effortConfigId) this.effort = null;
      else if (!this.effortTouched || !this.effort || !this.efforts.includes(this.effort)) {
        this.effort = catalog.currentEffort ?? this.efforts[0] ?? null;
      }
      return;
    }
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
        ? { id: this.modelId, provider: this.profile.id, name: current?.name ?? this.modelId }
        : null,
      thinkingLevel: this.effort ?? (this.profile.modelSource === "config" ? null : "medium"),
      ...(this.modeState() ? { mode: this.modeState() } : {}),
    };
  }

  private thinkingLevels(): string[] {
    if (this.profile.modelSource === "config") return [...this.efforts];
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
    this.clearDeferred();
    // A request the agent left open when its turn ended can no longer be answered.
    this.denyOutstanding("cancelled");
    finishHistoryTurn(this.history);
    const message = errorMessage ?? (stopReason === "refusal" ? `${this.displayName} refused the turn.` : undefined);
    for (const event of closeTurn(this.stream, stopReason, message)) {
      this.host?.emit(event);
    }
    const next = this.queue.shift();
    if (next === undefined) return;
    void this.startTurn(next).catch((error: unknown) => {
      this.queue = [];
      const text = error instanceof Error ? error.message : `${this.displayName} request failed.`;
      this.host?.emit({ type: "extension_error", error: text });
    });
  }

  private onNotification(method: string, params: Json): void {
    if (this.ignoreUpdates) return;
    if (method !== "session/update") return;
    const update = asObject(params.update);
    if (!update) return;
    const kind = update.sessionUpdate;
    if (kind === "usage_update") this.noteUsage(update);
    if (kind === "available_commands_update") this.noteCommands(update);
    if (kind === "config_option_update" || kind === "current_mode_update") {
      this.mergeCatalog(catalogFromAcp(update, this.profile));
    }
    if (this.suppressLive) this.lastReplayAt = Date.now();
    // A reply that arrives after its turn closed must not open a host turn that nothing ends.
    if (!this.running && !this.suppressLive) return;
    const skipUser = this.suppressUserChunks && kind === "user_message_chunk";
    if (!skipUser) recordSessionUpdate(this.history, update);
    if (this.running && !this.suppressLive) this.noteUnasked(update);
    if (this.running && this.profile.deferTurnOnRunningTools) {
      if (kind === "tool_call" || kind === "tool_call_update") this.trackTool(update);
      else if (kind === "agent_message_chunk") this.reply = "open";
      else if (kind === "agent_thought_chunk" && this.reply !== "open") this.reply = "idle";
      else if (kind === "usage_update" && this.reply === "open") this.reply = "settled";
      this.settleDeferred();
    }
    if (this.suppressLive || !this.host) return;
    for (const event of mapSessionUpdate(update, this.stream)) {
      this.host.emit(event);
    }
  }

  /**
   * A CLI that runs its own commands asks through session/request_permission. A command that
   * finishes without one means the CLI's own settings skipped Pitex's approval; say so once.
   */
  private noteUnasked(update: Json): void {
    if (this.unaskedWarned || this.profile.clientCapabilities.terminal === true) return;
    if (update.kind !== "execute" || update.status !== "completed") return;
    const id = asString(update.toolCallId);
    if (!id || this.asked.has(id)) return;
    this.unaskedWarned = true;
    this.host?.emit({
      type: "extension_error",
      error: `${this.displayName} ran a command without asking Pitex first. Check its own permission settings (for example an always-proceed setting).`,
    });
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
        return { name, description: asString(row?.description) ?? "", source: this.profile.id };
      })
      .filter((command): command is { name: string; description: string; source: string } => command !== null);
  }

  private onServerRequest(method: string, id: string | number, params: Json): void {
    // Only the file and terminal methods this profile advertised are served.
    if (!this.advertises(method)) method = `unadvertised ${method}`;
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
        this.rpc?.fail(id, `Pitex cannot answer this ${this.displayName} request.`);
        this.host?.emit({
          type: "extension_error",
          error: `Skipped an unsupported ${this.displayName} request (${method}).`,
        });
    }
  }

  private advertises(method: string): boolean {
    const caps = this.profile.clientCapabilities;
    const fs = asObject(caps.fs);
    if (method === "fs/read_text_file") return fs?.readTextFile === true;
    if (method === "fs/write_text_file") return fs?.writeTextFile === true;
    if (method.startsWith("terminal/")) return caps.terminal === true;
    return true;
  }

  private askPermission(id: string | number, params: Json): void {
    const toolCallId = asString(asObject(params.toolCall)?.toolCallId);
    if (toolCallId) this.asked.add(toolCallId);
    const options = permissionOptions(params);
    const key = approvalKey(params);
    if (this.allowRules.has(key)) {
      this.rpc?.respond(id, permissionResult("approve", options));
      return;
    }
    const approval = approvalFromPermission(id, params, this.displayName);
    this.pending.set(String(id), { kind: "permission", rpcId: id, options, key });
    this.host?.emit({
      type: "approval_request",
      ...approval,
      actions: permissionActions(options, key !== ""),
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
    const launch = terminalSpawnSpec(command, args);
    const child = spawn(launch.command, launch.args, {
      cwd,
      env: terminalEnv(this.env, env),
      shell: false,
      detached: true,
      windowsHide: true,
      windowsVerbatimArguments: launch.verbatim,
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

  /** Settles every open request without the user; the host drops those cards on approval_resolved. */
  private denyOutstanding(reason: "cancelled" | "deny"): void {
    const outstanding = [...this.pending.values()];
    this.dropPending(reason === "cancelled" ? "cancelled" : "denied");
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

  private dropPending(outcome: "cancelled" | "denied" = "cancelled"): void {
    const ids = [...this.pending.keys()];
    this.pending.clear();
    for (const id of ids) this.host?.emit({ type: "approval_resolved", id, outcome });
  }

  private resolveApproval(command: PiCommand): void {
    const id = command.id ?? "";
    const pending = this.pending.get(id);
    if (!pending || !this.rpc) return;
    this.pending.delete(id);
    if (pending.kind === "permission") {
      // No allow_always on offer (agy): allow once now, and let this session repeat the answer.
      const offersAlways = pending.options.some((option) => option.kind === "allow_always");
      const always = command.actionId ? command.actionId === "always_allow" : command.decision === "always_allow";
      if (always && !offersAlways && pending.key) this.allowRules.add(pending.key);
      this.rpc.respond(pending.rpcId, permissionResultForAction(command.actionId, command.decision, pending.options));
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

  private async killTerminals(): Promise<void> {
    const waits: Promise<void>[] = [];
    for (const term of this.terminals.values()) {
      if (term.exited) continue;
      killProcessGroup(term.child);
      waits.push(
        new Promise((resolve) => {
          const timer = setTimeout(resolve, 500);
          term.child.once("exit", () => {
            clearTimeout(timer);
            resolve();
          });
        })
      );
    }
    this.terminals.clear();
    await Promise.all(waits);
  }
}

function joinSession(sessionDir: string, name: string): string {
  return sessionDir.endsWith("/") ? sessionDir + name : `${sessionDir}/${name}`;
}
