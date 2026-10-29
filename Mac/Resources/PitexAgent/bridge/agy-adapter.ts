// Antigravity CLI adapter. Speaks `agy` stream-json on stdio and uses the
// CLI's own login. It never reads, copies, or parses Antigravity credential
// files, application-default credential files, or any other token file.
//
// Launch flags, from the CLI reference and the headless page (docs fetched
// 2026-10-29; changelog head 1.4.10):
//   --input-format stream-json --output-format stream-json
//   --print-timeout 0s   (reference: 0 waits until the turn completes)
//   --sandbox            (containment; no effect when the sandbox is unavailable)
//   --disable-slash-commands
//   optional --model, --effort, --conversation
// Working directory is the project directory (spawn cwd). There is no
// documented system-prompt flag, so editor instructions are prefixed onto
// the first user message of a new conversation.
//
// Not passed, on purpose:
//   --dangerously-skip-permissions  (always-proceed; auto-approves every tool)
//   --mode accept-edits | auto      (auto-approves edits, and auto also runs commands)
//   --mode plan                     (rewrites the task into a planning prefix)
// Headless mode has no interactive permission channel. Tools that need
// approval are denied by the CLI. See agy-map.ts.

import { spawn, type ChildProcess, type ChildProcessWithoutNullStreams } from "node:child_process";
import { accessSync, constants as fsConstants } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";
import { randomUUID } from "node:crypto";

import type { AdapterHost, AdapterStartOptions, CliAdapter, PiCommand } from "./adapter.ts";
import {
  AGY_EFFORTS,
  AGY_MISSING,
  AGY_PERMISSION_LIMIT,
  AGY_SIGNED_OUT,
  approvalIsNotDelivered,
  authFailureReason,
  createAgyStreamState,
  mapAgyLine,
  modelsFromText,
  permissionDenialNotice,
  permissionModeNotice,
  redactDiagnostic,
  safeToken,
  statsFromUsage,
  type AgyModel,
  type AgyStreamState,
  type Json,
} from "./agy-map.ts";
import {
  appendSessionMessage,
  appendUserMessage,
  rememberSession,
  sessionMessages,
  threadIdFor,
  writeSessionPrelude,
} from "./session-index.ts";

export { AGY_MISSING, AGY_SIGNED_OUT, AGY_PERMISSION_LIMIT };

export interface AgyLaunchSpec {
  command: string;
  args: string[];
  windowsVerbatimArguments?: boolean;
}

export function agyStreamArgs(options: {
  model?: string | null;
  effort?: string | null;
  conversationId?: string | null;
}): string[] {
  const args = [
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--print-timeout",
    "0s",
    "--sandbox",
    "--disable-slash-commands",
  ];
  const model = safeToken(options.model);
  const effort = safeToken(options.effort);
  const conversationId = safeToken(options.conversationId);
  if (model) args.push("--model", model);
  if (effort) args.push("--effort", effort);
  if (conversationId) args.push("--conversation", conversationId);
  return args;
}

export function findAgy(env: NodeJS.ProcessEnv, platform = process.platform): string | null {
  const explicit = env.AGY_PATH;
  if (explicit && isExecutable(explicit)) return explicit;
  const directories = (env.PATH ?? "").split(delimiter).filter(Boolean);
  const home = env.HOME || homedir();
  directories.push(join(home, ".local", "bin"), "/usr/local/bin", "/opt/homebrew/bin");
  const names = platform === "win32" ? ["agy.exe", "agy.cmd", "agy"] : ["agy"];
  for (const directory of directories) {
    for (const name of names) {
      const candidate = join(directory, name);
      if (isExecutable(candidate)) return candidate;
    }
  }
  return null;
}

/** A Windows cmd shim cannot be spawned directly. The argument list is fixed flags plus safe tokens. */
export function agyLaunchSpec(binary: string, streamArgs: string[], platform = process.platform): AgyLaunchSpec {
  if (platform !== "win32" || !/\.(cmd|bat)$/i.test(binary)) {
    return { command: binary, args: streamArgs };
  }
  if (!binary || /[\r\n"%!^&|<>]/.test(binary)) {
    throw new Error("Antigravity CLI path cannot be launched through cmd.exe.");
  }
  const tail = streamArgs.join(" ");
  return {
    command: "cmd.exe",
    args: ["/d", "/s", "/c", `"${binary}" ${tail}`],
    windowsVerbatimArguments: true,
  };
}

function isExecutable(path: string): boolean {
  try {
    accessSync(path, fsConstants.X_OK);
    return true;
  } catch {
    return false;
  }
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

async function stopChild(child: ChildProcess | null, graceMs = 400): Promise<void> {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  child.stdin?.on("error", () => {
    // Closing stdin after the CLI has gone raises EPIPE. Shutdown handles it.
  });
  try {
    child.stdin?.end();
  } catch {
    // The pipe is already closed.
  }
  await waitForChildExit(child, graceMs);
  if (child.exitCode !== null || child.signalCode !== null) return;
  try {
    child.kill();
  } catch {
    // Exit won the race.
  }
  await waitForChildExit(child, graceMs);
  if (child.exitCode !== null || child.signalCode !== null) return;
  if (process.platform === "win32" && child.pid) {
    await new Promise<void>((resolve) => {
      const killer = spawn("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
        windowsHide: true,
        stdio: "ignore",
      });
      killer.once("exit", resolve);
      killer.once("error", () => {
        try {
          child.kill("SIGKILL");
        } catch {
          // Already exited.
        }
        resolve();
      });
    });
  } else {
    try {
      child.kill("SIGKILL");
    } catch {
      // Already exited.
    }
  }
  await waitForChildExit(child, graceMs);
}

interface PendingPrompt {
  message: string;
  commandId?: string;
  host: AdapterHost;
}

export class AgyAdapter implements CliAdapter {
  readonly id = "agy";
  readonly displayName = "Antigravity CLI";
  unavailableReason: string | null = null;

  private cwd = process.cwd();
  private sessionDir = "";
  private developerInstructions = "";
  private env: NodeJS.ProcessEnv = process.env;
  private binary: string | null = null;
  private child: ChildProcessWithoutNullStreams | null = null;
  private generation = 0;
  private stream: AgyStreamState = createAgyStreamState();
  private host: AdapterHost | null = null;
  private models: AgyModel[] = [];
  private modelId: string | null = null;
  private effort: string | null = null;
  private conversationId: string | null = null;
  private launchedModel: string | null = null;
  private launchedEffort: string | null = null;
  private launchedConversation: string | null = null;
  private sessionPath: string | null = null;
  private messages: Json[] = [];
  private sentInstructions = false;
  private permissionMode: string | null = null;
  private policyAnnounced = false;
  private turnActive = false;
  private pending: PendingPrompt[] = [];
  private stdoutBuffer = "";
  private stderrLine = "";
  private stats: Json = { tokens: { total: 0 }, cost: 0, contextUsage: {} };
  private initWait: { resolve: () => void; reject: (error: Error) => void } | null = null;

  async start(options: AdapterStartOptions): Promise<void> {
    this.cwd = options.cwd;
    this.sessionDir = options.sessionDir;
    this.developerInstructions = options.developerInstructions ?? "";
    this.env = options.env;
    try {
      const binary = findAgy(options.env);
      if (!binary) throw new Error(AGY_MISSING);
      this.binary = binary;
      this.models = await this.probeModels(binary);
      await this.startStream();
      this.unavailableReason = null;
    } catch (error) {
      await this.stop();
      this.unavailableReason = error instanceof Error ? error.message : AGY_MISSING;
    }
  }

  async handleCommand(command: PiCommand, host: AdapterHost): Promise<void> {
    this.host = host;
    if (this.unavailableReason && command.type !== "abort") {
      host.respond(command.type, false, undefined, this.unavailableReason, command.id);
      return;
    }
    this.announcePolicy(host);
    try {
      switch (command.type) {
        case "prompt":
          await this.prompt(command, host);
          return;
        case "steer":
          // Stream-json accepts the next user message only after `result`,
          // so a steer waits in the same queue as a prompt.
          await this.prompt({ ...command, type: "prompt" }, host);
          return;
        case "abort":
          this.abort(host, command.id);
          return;
        case "new_session":
          await this.newSession(host, command.id);
          return;
        case "switch_session":
          await this.switchSession(command, host);
          return;
        case "get_messages":
          this.getMessages(command, host);
          return;
        case "get_state":
          host.respond("get_state", true, this.stateData(), undefined, command.id);
          return;
        case "get_available_models":
          host.respond("get_available_models", true, { models: this.models }, undefined, command.id);
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
          this.setModel(command, host);
          return;
        case "set_thinking_level":
          this.setEffort(command, host);
          return;
        case "get_session_stats":
          host.respond("get_session_stats", true, this.stats, undefined, command.id);
          return;
        case "get_commands":
          host.respond("get_commands", true, { commands: [] }, undefined, command.id);
          return;
        case "approval_response": {
          const outcome = approvalIsNotDelivered(command.decision);
          host.respond("approval_response", false, outcome, AGY_PERMISSION_LIMIT, command.id);
          return;
        }
        default:
          host.respond(command.type, false, undefined, `Unsupported command: ${command.type}`, command.id);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : "Antigravity CLI request failed.";
      host.respond(command.type, false, undefined, message, command.id);
    }
  }

  async stop(): Promise<void> {
    this.generation += 1;
    this.initWait?.reject(new Error("Antigravity CLI stopped."));
    this.initWait = null;
    const child = this.child;
    this.child = null;
    this.turnActive = false;
    this.pending = [];
    await stopChild(child);
  }

  private announcePolicy(host: AdapterHost): void {
    if (this.policyAnnounced || this.unavailableReason) return;
    this.policyAnnounced = true;
    host.emit({ type: "extension_error", error: permissionModeNotice(this.permissionMode) });
  }

  private async probeModels(binary: string): Promise<AgyModel[]> {
    const child = spawn(binary, ["models"], {
      cwd: this.cwd,
      env: this.env,
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    let stdout = "";
    let stderr = "";
    child.stdout?.setEncoding("utf8");
    child.stderr?.setEncoding("utf8");
    child.stdout?.on("data", (chunk: string) => {
      if (stdout.length < 1_000_000) stdout += chunk;
    });
    child.stderr?.on("data", (chunk: string) => {
      if (stderr.length < 100_000) stderr += chunk;
    });
    const code = await new Promise<number>((resolve, reject) => {
      let settled = false;
      const finish = (callback: () => void) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        callback();
      };
      const timer = setTimeout(() => {
        child.kill();
        finish(() => reject(new Error(`${AGY_SIGNED_OUT} (\`agy models\` did not answer.)`)));
      }, 20_000);
      child.once("error", (error) => finish(() => reject(error)));
      child.once("exit", (status) => finish(() => resolve(status ?? 1)));
    });
    const auth = authFailureReason(`${stdout}\n${stderr}`);
    if (auth) throw new Error(auth);
    const models = modelsFromText(stdout);
    if (code !== 0 || models.length === 0) {
      const detail = redactDiagnostic(stderr) ?? "";
      throw new Error(
        detail
          ? `Antigravity CLI could not list models. ${detail}`
          : `${AGY_SIGNED_OUT} (\`agy models\` failed.)`
      );
    }
    return models;
  }

  private launchMatches(): boolean {
    return (
      this.child !== null &&
      this.child.exitCode === null &&
      this.child.signalCode === null &&
      this.launchedModel === this.modelId &&
      this.launchedEffort === this.effort &&
      this.launchedConversation === this.conversationId
    );
  }

  private async ensureRunning(): Promise<void> {
    if (this.launchMatches()) return;
    await this.startStream();
  }

  private async startStream(): Promise<void> {
    if (!this.binary) throw new Error(AGY_MISSING);
    const generation = ++this.generation;
    const previous = this.child;
    this.child = null;
    await stopChild(previous);
    if (generation !== this.generation) return;
    const streamArgs = agyStreamArgs({
      model: this.modelId,
      effort: this.effort,
      conversationId: this.conversationId,
    });
    const launch = agyLaunchSpec(this.binary, streamArgs);
    const child = spawn(launch.command, launch.args, {
      cwd: this.cwd,
      env: this.env,
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
      windowsVerbatimArguments: launch.windowsVerbatimArguments === true,
    }) as ChildProcessWithoutNullStreams;
    this.child = child;
    this.launchedModel = this.modelId;
    this.launchedEffort = this.effort;
    this.launchedConversation = this.conversationId;
    this.stdoutBuffer = "";
    this.stderrLine = "";
    this.stream = createAgyStreamState();
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      if (generation !== this.generation) return;
      this.ingest(chunk);
    });
    child.stderr.on("data", (chunk: string) => {
      if (generation !== this.generation) return;
      this.ingestStderr(chunk);
    });
    child.stdin.on("error", () => {
      // EPIPE after the CLI exits is expected during shutdown.
    });
    const ready = new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.initWait = null;
        reject(new Error("Antigravity CLI did not start a stream-json session."));
      }, 20_000);
      this.initWait = {
        resolve: () => {
          clearTimeout(timer);
          resolve();
        },
        reject: (error) => {
          clearTimeout(timer);
          reject(error);
        },
      };
    });
    child.once("exit", () => {
      if (generation !== this.generation) return;
      const auth = authFailureReason(this.stderrLine);
      if (this.initWait) {
        this.initWait.reject(new Error(auth ?? AGY_SIGNED_OUT));
        this.initWait = null;
        return;
      }
      if (this.turnActive) this.failOpenTurn(auth ?? "Antigravity CLI exited.");
      if (this.child === child) this.child = null;
    });
    child.once("error", (error) => {
      if (generation !== this.generation) return;
      this.initWait?.reject(error);
      this.initWait = null;
    });
    try {
      await ready;
    } catch (error) {
      this.initWait = null;
      if (generation === this.generation) {
        this.generation += 1;
        await stopChild(child);
        if (this.child === child) this.child = null;
      }
      throw error;
    }
  }

  private ingest(chunk: string): void {
    this.stdoutBuffer += chunk;
    let newline = this.stdoutBuffer.indexOf("\n");
    while (newline >= 0) {
      const line = this.stdoutBuffer.slice(0, newline).trim();
      this.stdoutBuffer = this.stdoutBuffer.slice(newline + 1);
      if (line) this.onStdoutLine(line);
      newline = this.stdoutBuffer.indexOf("\n");
    }
  }

  private onStdoutLine(line: string): void {
    let parsed: unknown;
    try {
      parsed = JSON.parse(line);
    } catch {
      return;
    }
    const mapped = mapAgyLine(parsed, this.stream);
    if (mapped.permissionMode) this.permissionMode = mapped.permissionMode;
    if (mapped.conversationId) {
      this.conversationId = mapped.conversationId;
      this.launchedConversation = mapped.conversationId;
      if (this.sessionPath) {
        rememberSession(this.sessionDir, this.sessionPath, mapped.conversationId, this.modelId ?? undefined);
      }
    }
    if (mapped.usage) this.stats = statsFromUsage(mapped.usage);
    if (mapped.terminal) {
      for (const message of mapped.messages) {
        if (this.sessionPath) appendSessionMessage(this.sessionPath, message);
        this.messages.push(message);
      }
    }
    if (this.initWait && parsed && typeof parsed === "object" && (parsed as Json).event === "init") {
      const wait = this.initWait;
      this.initWait = null;
      wait.resolve();
    }
    for (const event of mapped.events) this.host?.emit(event);
    if (mapped.terminal) {
      this.turnActive = false;
      void this.pump();
    }
  }

  private ingestStderr(chunk: string): void {
    this.stderrLine = `${this.stderrLine}${chunk}`.slice(-4_000);
    const lines = chunk.split(/\r?\n/);
    for (const line of lines) {
      const auth = authFailureReason(line);
      if (auth && this.initWait) {
        const wait = this.initWait;
        this.initWait = null;
        wait.reject(new Error(auth));
      }
      const notice = permissionDenialNotice(line);
      if (notice) this.host?.emit({ type: "extension_error", error: notice });
    }
  }

  private failOpenTurn(message: string): void {
    this.turnActive = false;
    this.pending = [];
    this.host?.emit({
      type: "agent_end",
      messages: [
        {
          role: "assistant",
          stopReason: "error",
          errorMessage: message,
          content: [],
        },
      ],
    });
  }

  private async prompt(command: PiCommand, host: AdapterHost): Promise<void> {
    const message = command.message ?? "";
    if (!message.trim()) {
      host.respond(command.type, false, undefined, "The prompt was empty.", command.id);
      return;
    }
    if (!this.sessionPath) {
      this.sessionPath = joinSession(this.sessionDir, `agy-${randomUUID()}.jsonl`);
      writeSessionPrelude(this.sessionPath, message, this.cwd);
    } else {
      appendUserMessage(this.sessionPath, message);
    }
    if (this.conversationId && this.sessionPath) {
      rememberSession(this.sessionDir, this.sessionPath, this.conversationId, this.modelId ?? undefined);
    }
    this.messages.push({ role: "user", content: [{ type: "text", text: message }] });
    this.pending.push({ message, commandId: command.id, host });
    await this.pump();
  }

  private pumping = false;

  private async pump(): Promise<void> {
    if (this.pumping) return;
    this.pumping = true;
    try {
      while (!this.turnActive && this.pending.length > 0) {
        const next = this.pending.shift();
        if (!next) return;
        try {
          await this.ensureRunning();
        } catch (error) {
          const message = error instanceof Error ? error.message : "Antigravity CLI could not start.";
          next.host.respond("prompt", false, undefined, message, next.commandId);
          continue;
        }
        if (this.turnActive || !this.child) {
          next.host.respond("prompt", false, undefined, AGY_MISSING, next.commandId);
          continue;
        }
        const wire =
          !this.sentInstructions && this.developerInstructions.trim()
            ? `${this.developerInstructions.trim()}\n\n${next.message}`
            : next.message;
        this.sentInstructions = true;
        this.turnActive = true;
        try {
          this.child.stdin.write(`${JSON.stringify({ event: "user", message: { content: wire } })}\n`);
        } catch (error) {
          this.turnActive = false;
          const message = error instanceof Error ? error.message : "Antigravity CLI did not accept the prompt.";
          next.host.respond("prompt", false, undefined, message, next.commandId);
          continue;
        }
        next.host.respond("prompt", true, {}, undefined, next.commandId);
      }
    } finally {
      this.pumping = false;
    }
  }

  private abort(host: AdapterHost, id?: string): void {
    this.pending = [];
    if (this.turnActive && this.child && this.child.exitCode === null) {
      try {
        this.child.kill("SIGINT");
      } catch {
        // The process can exit between the check and the signal.
      }
    }
    host.respond("abort", true, {}, undefined, id);
  }

  private async newSession(host: AdapterHost, id?: string): Promise<void> {
    this.pending = [];
    this.turnActive = false;
    this.conversationId = null;
    this.sessionPath = null;
    this.messages = [];
    this.sentInstructions = false;
    this.generation += 1;
    const child = this.child;
    this.child = null;
    await stopChild(child);
    host.respond("new_session", true, {}, undefined, id);
  }

  private async switchSession(command: PiCommand, host: AdapterHost): Promise<void> {
    const sessionPath = command.sessionPath ?? "";
    const conversationId = threadIdFor(this.sessionDir, sessionPath);
    if (!conversationId) {
      host.respond(
        "switch_session",
        false,
        undefined,
        "That conversation has no Antigravity conversation to resume.",
        command.id
      );
      return;
    }
    this.pending = [];
    this.turnActive = false;
    this.conversationId = conversationId;
    this.sessionPath = sessionPath;
    this.messages = sessionMessages(sessionPath);
    this.sentInstructions = this.messages.length > 0;
    await this.startStream();
    host.respond("switch_session", true, { cancelled: false }, undefined, command.id);
  }

  private getMessages(command: PiCommand, host: AdapterHost): void {
    if (this.messages.length === 0 && this.sessionPath) {
      this.messages = sessionMessages(this.sessionPath);
    }
    host.respond("get_messages", true, { messages: this.messages }, undefined, command.id);
  }

  private setModel(command: PiCommand, host: AdapterHost): void {
    const model = safeToken(command.modelId);
    if (command.modelId && !model) {
      host.respond("set_model", false, undefined, "That model id cannot be passed to Antigravity CLI.", command.id);
      return;
    }
    this.modelId = model;
    host.respond("set_model", true, {}, undefined, command.id);
  }

  private setEffort(command: PiCommand, host: AdapterHost): void {
    const effort = safeToken(command.level);
    if (command.level && !effort) {
      host.respond(
        "set_thinking_level",
        false,
        undefined,
        "That reasoning effort cannot be passed to Antigravity CLI.",
        command.id
      );
      return;
    }
    if (effort && !AGY_EFFORTS.includes(effort as (typeof AGY_EFFORTS)[number])) {
      host.respond(
        "set_thinking_level",
        false,
        undefined,
        "Antigravity CLI documents reasoning effort as low, medium, high, or max.",
        command.id
      );
      return;
    }
    this.effort = effort;
    host.respond("set_thinking_level", true, {}, undefined, command.id);
  }

  private stateData(): Json {
    const current = this.models.find((model) => model.id === this.modelId);
    return {
      model: current
        ? { id: current.id, provider: "agy", name: current.name }
        : this.modelId
          ? { id: this.modelId, provider: "agy", name: this.modelId }
          : null,
      thinkingLevel: this.effort ?? "medium",
    };
  }

  private thinkingLevels(): string[] {
    const levels: string[] = [...AGY_EFFORTS];
    const selected = this.effort ?? "medium";
    if (!levels.includes(selected)) levels.unshift(selected);
    return levels;
  }
}

function joinSession(sessionDir: string, name: string): string {
  return sessionDir.endsWith("/") || sessionDir.endsWith("\\") ? sessionDir + name : `${sessionDir}/${name}`;
}
