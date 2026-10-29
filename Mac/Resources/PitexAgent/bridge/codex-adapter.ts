// Codex CLI adapter. Speaks `codex app-server` JSON-RPC on stdio and the
// user's existing `codex login`. It never reads, copies, or parses
// Codex credential files or token files, and it never refreshes tokens.
//
// Protocol: Codex CLI 0.165.0, `codex app-server generate-ts`.
// approvalPolicy is the stable value "untrusted"; approvalsReviewer is "user".
//
// Design adapted from T3 Code v0.1.4 (MIT), Copyright (c) 2026 T3 Tools Inc.
// https://github.com/pingdotgg/t3code
// packages/effect-codex-app-server and CodexAdapterV2.ts.
// Pitex does not port T3's managed ChatGPT OAuth broker.

import { spawn, type ChildProcess, type ChildProcessWithoutNullStreams } from "node:child_process";
import { accessSync, constants as fsConstants, existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, dirname, isAbsolute, join } from "node:path";
import { randomUUID } from "node:crypto";

import type { AdapterHost, AdapterStartOptions, CliAdapter, PiCommand } from "./adapter.ts";
import {
  approvalFromServerRequest,
  approvalRpcReply,
  createStreamState,
  fileChangeDetail,
  itemsToPiMessages,
  mapNotification,
  modelsFromList,
  turnsToPiMessages,
  visibleModels,
  type Json,
  type ListedModel,
  type StreamState,
} from "./codex-map.ts";
import { LineJsonRpc } from "./jsonrpc.ts";
import {
  appendUserMessage,
  rememberSession,
  threadIdFor,
  writeSessionPrelude,
} from "./session-index.ts";

export const CODEX_MISSING =
  "Codex CLI was not found. Install the official Codex CLI, then run `codex login`.";
export const CODEX_SIGNED_OUT =
  "Codex CLI is not logged in. Run `codex login` in a terminal, then try again.";

interface PendingApproval {
  method: string;
  params: Json;
  rpcId: string | number;
}

export function findCodex(env: NodeJS.ProcessEnv, platform = process.platform): string | null {
  const explicit = env.CODEX_PATH;
  if (explicit && isExecutable(explicit)) return explicit;
  const directories = (env.PATH ?? "").split(delimiter).filter(Boolean);
  const home = env.HOME || homedir();
  directories.push(join(home, ".local", "bin"), "/usr/local/bin", "/opt/homebrew/bin");
  const names = platform === "win32" ? ["codex.exe", "codex.cmd", "codex"] : ["codex"];
  for (const directory of directories) {
    for (const name of names) {
      const candidate = join(directory, name);
      if (isExecutable(candidate)) return candidate;
    }
  }
  return null;
}

export interface CodexLaunchSpec {
  command: string;
  args: string[];
  /** Set when `args` is already a cmd.exe command line. */
  windowsVerbatimArguments?: boolean;
}

/**
 * Windows CreateProcess cannot run a shebang, and current Node rejects a
 * direct spawn of `.cmd` / `.bat` (EINVAL). Prefer `codex.exe`. A cmd shim
 * is the Node script it names, run with this process's node. Otherwise the
 * shim is started through `cmd.exe /d /s /c` with the fixed argument
 * `app-server` only.
 */
export function codexLaunchSpec(binary: string, platform = process.platform): CodexLaunchSpec {
  const direct = { command: binary, args: ["app-server"] };
  if (platform !== "win32") return direct;
  if (/\.exe$/i.test(binary)) return direct;
  if (/\.(cmd|bat)$/i.test(binary)) {
    const script = npmShimScript(binary);
    if (script) return { command: process.execPath, args: [script, "app-server"] };
    const quoted = quoteCmdExecutable(binary);
    if (!quoted) {
      throw new Error("Codex CLI path cannot be launched through cmd.exe.");
    }
    return {
      command: "cmd.exe",
      args: ["/d", "/s", "/c", `${quoted} app-server`],
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
    return { command: process.execPath, args: [binary, "app-server"] };
  }
  return direct;
}

/** npm's cmd shim quotes the real CLI, often under `%~dp0` or `%dp0%`. */
function npmShimScript(cmdPath: string): string | null {
  let text = "";
  try {
    text = readFileSync(cmdPath).subarray(0, 8192).toString("utf8");
  } catch {
    return null;
  }
  const directory = dirname(cmdPath);
  const pattern = /"([^"\r\n]+\.(?:js|mjs|cjs))"/gi;
  for (const match of text.matchAll(pattern)) {
    const candidate = expandShimPath(match[1], directory);
    if (candidate && existsSync(candidate)) return candidate;
  }
  return null;
}

function expandShimPath(raw: string, directory: string): string | null {
  let body = raw;
  const prefix = body.match(/^(?:%~dp0|%dp0%)/i);
  if (prefix) body = body.slice(prefix[0].length);
  body = body.replace(/^[\\/]+/, "").replace(/\\/g, "/");
  if (!body || body.includes("..")) return null;
  if (isAbsolute(body)) return body;
  return join(directory, body);
}

/**
 * Quote one executable for `cmd /d /s /c`. The command line is only that
 * path plus the fixed word `app-server`. `%` and other cmd metacharacters
 * are rejected so the path cannot expand or chain commands.
 */
function quoteCmdExecutable(binary: string): string | null {
  if (!binary || /[\r\n"%!^&|<>]/.test(binary)) return null;
  return `"${binary}"`;
}

/** How to finish a child that ignored the graceful stop. */
export function codexForceKillPlan(
  pid: number,
  platform = process.platform
): { command: string; args: string[] } | { signal: "SIGKILL" } {
  if (platform === "win32" && Number.isInteger(pid) && pid > 0) {
    return { command: "taskkill", args: ["/PID", String(pid), "/T", "/F"] };
  }
  return { signal: "SIGKILL" };
}

export async function stopCodexChild(child: ChildProcess, graceMs = 2000): Promise<void> {
  child.stdin?.on("error", () => {
    // Closing stdin after the CLI has gone raises EPIPE. Shutdown handles it.
  });
  if (child.exitCode !== null || child.signalCode !== null) return;
  try {
    child.stdin?.end();
  } catch {
    // The pipe is already closed when the CLI exits on its own.
  }
  try {
    child.kill();
  } catch {
    // Exit won the race with the graceful signal.
  }
  await waitForChildExit(child, graceMs);
  if (child.exitCode !== null || child.signalCode !== null) return;
  try {
    await forceKillChild(child);
  } catch {
    // The process can exit between the check and the signal.
  }
  await waitForChildExit(child, graceMs);
}

function waitForChildExit(child: ChildProcess, graceMs: number): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve();
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, graceMs);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

function forceKillChild(child: ChildProcess): Promise<void> {
  const plan = codexForceKillPlan(child.pid ?? 0);
  if ("signal" in plan) {
    try {
      child.kill(plan.signal);
    } catch {
      // Already exited.
    }
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    let settled = false;
    const done = () => {
      if (settled) return;
      settled = true;
      resolve();
    };
    const killer = spawn(plan.command, plan.args, { windowsHide: true, stdio: "ignore" });
    killer.once("exit", done);
    killer.once("error", () => {
      try {
        child.kill("SIGKILL");
      } catch {
        // Already exited.
      }
      done();
    });
  });
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

export class CodexAdapter implements CliAdapter {
  readonly id = "codex";
  readonly displayName = "Codex CLI";
  unavailableReason: string | null = null;

  private rpc: LineJsonRpc | null = null;
  private child: ChildProcessWithoutNullStreams | null = null;
  private cwd = process.cwd();
  private sessionDir = "";
  private developerInstructions = "";
  private stream: StreamState = createStreamState();
  private host: AdapterHost | null = null;
  private threadId: string | null = null;
  private turnId: string | null = null;
  private sessionPath: string | null = null;
  private modelId: string | null = null;
  private effort: string | null = null;
  private models: ListedModel[] = [];
  private pending = new Map<string, PendingApproval>();
  private fileDetails = new Map<string, string>();
  private running = false;
  private acceptClose = false;

  async start(options: AdapterStartOptions): Promise<void> {
    this.cwd = options.cwd;
    this.sessionDir = options.sessionDir;
    this.developerInstructions = options.developerInstructions ?? "";
    try {
      await this.connect(options.env);
      this.unavailableReason = null;
    } catch (error) {
      await this.stop();
      this.unavailableReason = error instanceof Error ? error.message : CODEX_MISSING;
    }
  }

  private async connect(env: NodeJS.ProcessEnv): Promise<void> {
    const binary = findCodex(env);
    if (!binary) throw new Error(CODEX_MISSING);
    this.acceptClose = true;
    const launch = codexLaunchSpec(binary);
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
      // Codex logs stay off the pi event stream. They are not credentials,
      // and they are not forwarded to the transcript.
    });
    this.rpc = new LineJsonRpc(
      child,
      (method, params) => this.onNotification(method, params),
      (method, id, params) => this.onServerRequest(method, id, params),
      (message) => this.markClosed(message)
    );
    await this.rpc.request("initialize", {
      clientInfo: { name: "pitex", title: "Pitex", version: "0.1.0" },
      capabilities: { experimentalApi: false, requestAttestation: false },
    });
    this.rpc.notify("initialized", {});
    // refreshToken stays false: Pitex must not ask Codex to mint or return tokens.
    // account/read reports the login Codex already has. getAuthStatus is not
    // called: that response can carry a token, and Pitex never wants one.
    const account = asObject(
      await this.rpc.request("account/read", { refreshToken: false })
    );
    if (!account?.account || typeof account.account !== "object") {
      throw new Error(CODEX_SIGNED_OUT);
    }
    await this.refreshModels();
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
          await this.prompt({ ...command, type: "prompt", streamingBehavior: "followUp" }, host);
          return;
        case "abort":
          this.denyOutstandingApprovals();
          await this.abort(host, command.id);
          return;
        case "new_session":
          this.denyOutstandingApprovals();
          this.threadId = null;
          this.turnId = null;
          this.sessionPath = null;
          this.running = false;
          host.respond("new_session", true, {}, undefined, command.id);
          return;
        case "switch_session":
          await this.switchSession(command, host);
          return;
        case "get_messages":
          await this.getMessages(command, host);
          return;
        case "get_state":
          host.respond("get_state", true, this.stateData(), undefined, command.id);
          return;
        case "get_available_models":
          host.respond(
            "get_available_models",
            true,
            { models: visibleModels(this.models) },
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
          this.modelId = command.modelId || this.modelId;
          host.respond("set_model", true, {}, undefined, command.id);
          return;
        case "set_thinking_level":
          this.effort = command.level || this.effort;
          host.respond("set_thinking_level", true, {}, undefined, command.id);
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
      const message = error instanceof Error ? error.message : "Codex CLI request failed.";
      host.respond(command.type, false, undefined, message, command.id);
    }
  }

  async stop(): Promise<void> {
    this.acceptClose = false;
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

  private stats: Json = { tokens: { total: 0 }, cost: 0, contextUsage: {} };

  private async prompt(command: PiCommand, host: AdapterHost): Promise<void> {
    const message = command.message ?? "";
    if (!message.trim()) {
      host.respond("prompt", false, undefined, "The prompt was empty.", command.id);
      return;
    }
    if (!this.threadId) await this.openThread(null);
    if (!this.sessionPath) {
      this.sessionPath = joinSession(this.sessionDir, `codex-${randomUUID()}.jsonl`);
      writeSessionPrelude(this.sessionPath, message, this.cwd);
      if (this.threadId) rememberSession(this.sessionDir, this.sessionPath, this.threadId, this.modelId ?? undefined);
    } else {
      appendUserMessage(this.sessionPath, message);
    }
    const input = [{ type: "text", text: message, text_elements: [] }];
    if (this.running && this.turnId && command.streamingBehavior === "followUp") {
      await this.rpc?.request("turn/steer", {
        threadId: this.threadId,
        expectedTurnId: this.turnId,
        input,
      });
      host.respond("prompt", true, {}, undefined, command.id);
      return;
    }
    const started = asObject(
      await this.rpc?.request("turn/start", {
        threadId: this.threadId,
        input,
        approvalPolicy: "untrusted",
        approvalsReviewer: "user",
        sandboxPolicy: {
          type: "workspaceWrite",
          writableRoots: [this.cwd],
          networkAccess: false,
          excludeTmpdirEnvVar: false,
          excludeSlashTmp: false,
        },
        ...(this.modelId ? { model: this.modelId } : {}),
        ...(this.effort ? { effort: this.effort } : {}),
      })
    );
    const turn = asObject(started?.turn);
    this.turnId = typeof turn?.id === "string" ? turn.id : this.turnId;
    this.running = true;
    host.respond("prompt", true, {}, undefined, command.id);
  }

  private async abort(host: AdapterHost, id?: string): Promise<void> {
    if (this.rpc && this.threadId && this.turnId) {
      try {
        await this.rpc.request("turn/interrupt", { threadId: this.threadId, turnId: this.turnId });
      } catch {
        // The turn may already have finished.
      }
    }
    this.running = false;
    host.respond("abort", true, {}, undefined, id);
  }

  private async switchSession(command: PiCommand, host: AdapterHost): Promise<void> {
    const sessionPath = command.sessionPath ?? "";
    const threadId = threadIdFor(this.sessionDir, sessionPath);
    if (!threadId) {
      host.respond(
        "switch_session",
        false,
        undefined,
        "That conversation has no Codex thread to resume.",
        command.id
      );
      return;
    }
    await this.openThread(threadId);
    this.sessionPath = sessionPath;
    host.respond("switch_session", true, { cancelled: false }, undefined, command.id);
  }

  private async getMessages(command: PiCommand, host: AdapterHost): Promise<void> {
    if (!this.threadId || !this.rpc) {
      host.respond("get_messages", true, { messages: [] }, undefined, command.id);
      return;
    }
    const read = asObject(
      await this.rpc.request("thread/read", { threadId: this.threadId, includeTurns: true })
    );
    const thread = asObject(read?.thread);
    let messages = turnsToPiMessages(Array.isArray(thread?.turns) ? thread.turns : []);
    if (messages.length === 0) {
      const items = await this.pageItems();
      messages = itemsToPiMessages(items);
    }
    host.respond("get_messages", true, { messages }, undefined, command.id);
  }

  private async pageItems(): Promise<unknown[]> {
    if (!this.rpc || !this.threadId) return [];
    const items: unknown[] = [];
    let cursor: string | null = null;
    for (let page = 0; page < 20; page += 1) {
      const response = asObject(
        await this.rpc.request("thread/items/list", {
          threadId: this.threadId,
          ...(cursor ? { cursor } : {}),
          limit: 100,
          sortDirection: "asc",
        })
      );
      const data = Array.isArray(response?.data) ? response.data : [];
      for (const entry of data) {
        const row = asObject(entry);
        if (row?.item) items.push(row.item);
      }
      cursor = typeof response?.nextCursor === "string" ? response.nextCursor : null;
      if (!cursor || data.length === 0) break;
    }
    return items;
  }

  private async openThread(resumeId: string | null): Promise<void> {
    if (!this.rpc) throw new Error(CODEX_MISSING);
    const params: Json = {
      cwd: this.cwd,
      approvalPolicy: "untrusted",
      approvalsReviewer: "user",
      sandbox: "workspace-write",
      ...(this.modelId ? { model: this.modelId } : {}),
      ...(this.developerInstructions ? { developerInstructions: this.developerInstructions } : {}),
    };
    const method = resumeId ? "thread/resume" : "thread/start";
    const result = asObject(
      await this.rpc.request(method, resumeId ? { threadId: resumeId, ...params } : params)
    );
    const thread = asObject(result?.thread);
    const id = typeof thread?.id === "string" ? thread.id : resumeId;
    if (!id) throw new Error("Codex CLI did not return a thread id.");
    this.threadId = id;
    if (typeof result?.model === "string") this.modelId = result.model;
    if (typeof thread?.reasoningEffort === "string") this.effort = thread.reasoningEffort;
    this.turnId = null;
    this.running = false;
  }

  private async refreshModels(): Promise<void> {
    if (!this.rpc) return;
    const collected: ListedModel[] = [];
    let cursor: string | null = null;
    for (let page = 0; page < 10; page += 1) {
      const result = await this.rpc.request("model/list", {
        ...(cursor ? { cursor } : {}),
        limit: 100,
        includeHidden: false,
      });
      const batch = modelsFromList(result);
      collected.push(...batch);
      const root = asObject(result);
      cursor = typeof root?.nextCursor === "string" ? root.nextCursor : null;
      if (!cursor) break;
    }
    this.models = collected;
    if (!this.modelId) {
      const preferred = collected.find((model) => model.isDefault && !model.hidden) ?? collected.find((model) => !model.hidden);
      if (preferred) {
        this.modelId = preferred.id;
        this.effort = this.effort ?? preferred.defaultEffort;
      }
    }
  }

  private stateData(): Json {
    const current = this.models.find((model) => model.id === this.modelId);
    return {
      model: current
        ? { id: current.id, provider: "codex", name: current.name }
        : this.modelId
          ? { id: this.modelId, provider: "codex", name: this.modelId }
          : null,
      thinkingLevel: this.effort ?? current?.defaultEffort ?? "medium",
    };
  }

  private thinkingLevels(): string[] {
    const current = this.models.find((model) => model.id === this.modelId);
    const levels = current?.efforts.length ? [...current.efforts] : ["low", "medium", "high"];
    const selected = this.effort ?? current?.defaultEffort ?? levels[0];
    if (selected && !levels.includes(selected)) levels.unshift(selected);
    return levels;
  }

  private onNotification(method: string, params: Json): void {
    if (method === "thread/tokenUsage/updated") {
      const usage = asObject(params.tokenUsage);
      const total = asObject(usage?.total);
      const tokens = typeof total?.totalTokens === "number" ? total.totalTokens : 0;
      const window = typeof usage?.modelContextWindow === "number" ? usage.modelContextWindow : null;
      this.stats = {
        tokens: { total: tokens },
        cost: 0,
        contextUsage: window
          ? { tokens, contextWindow: window, percent: window ? (tokens / window) * 100 : null }
          : {},
      };
    }
    const item = asObject(params.item);
    if (item?.type === "fileChange") {
      const itemId = typeof item.id === "string" ? item.id : null;
      if (itemId) this.fileDetails.set(itemId, fileChangeDetail(item));
    }
    if (!this.host) return;
    for (const event of mapNotification(method, params, this.stream)) {
      if (event.type === "agent_end") this.running = false;
      if (event.type === "turn/started") this.running = true;
      this.host.emit(event);
    }
    if (method === "turn/started") {
      const turn = asObject(params.turn);
      if (typeof turn?.id === "string") this.turnId = turn.id;
      this.running = true;
    }
  }

  private onServerRequest(method: string, id: string | number, params: Json): void {
    if (method === "account/chatgptAuthTokens/refresh" || method === "attestation/generate") {
      this.rpc?.fail(id, "Pitex does not handle Codex token refreshes. Run `codex login`.");
      return;
    }
    const approval = approvalFromServerRequest(method, id, params);
    if (!approval) {
      this.rpc?.fail(id, "Pitex cannot answer this Codex request.");
      this.host?.emit({
        type: "extension_error",
        error: `Skipped an unsupported Codex request (${method}).`,
      });
      return;
    }
    const itemId = typeof params.itemId === "string" ? params.itemId : null;
    const fileDetail = itemId ? this.fileDetails.get(itemId) : undefined;
    const detail = fileDetail ? `${approval.detail}\n\n${fileDetail}` : approval.detail;
    this.pending.set(String(id), { method, params, rpcId: id });
    this.host?.emit({
      type: "approval_request",
      id: approval.id,
      kind: approval.kind,
      title: approval.title,
      detail,
    });
  }

  /** Decline every Codex request still waiting, then drop the map. */
  private denyOutstandingApprovals(): void {
    const outstanding = [...this.pending.values()];
    this.pending.clear();
    if (!this.rpc) return;
    for (const pending of outstanding) {
      const reply = approvalRpcReply(pending.method, "deny", pending.params);
      if (reply.error) this.rpc.fail(pending.rpcId, reply.error.message);
      else this.rpc.respond(pending.rpcId, reply.result ?? {});
    }
  }

  private resolveApproval(command: PiCommand): void {
    const id = command.id ?? "";
    const pending = this.pending.get(id);
    if (!pending || !this.rpc) return;
    this.pending.delete(id);
    const reply = approvalRpcReply(pending.method, command.decision, pending.params);
    if (reply.error) this.rpc.fail(pending.rpcId, reply.error.message);
    else this.rpc.respond(pending.rpcId, reply.result ?? {});
  }
}

function joinSession(sessionDir: string, name: string): string {
  return sessionDir.endsWith("/") ? sessionDir + name : `${sessionDir}/${name}`;
}
