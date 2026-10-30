// Per-CLI adapter. Codex, Claude Code, and Grok are registered.
//
// Later milestones plug in behind this same interface:
// - agy stream-json
//
// Design adapted from T3 Code v0.1.4 (MIT), Copyright (c) 2026 T3 Tools Inc.
// https://github.com/pingdotgg/t3code
// See packages/effect-codex-app-server and
// apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts.
// Pitex does not vendor T3's managed ChatGPT OAuth broker.

export interface PiCommand {
  type: string;
  id?: string;
  message?: string;
  streamingBehavior?: string;
  sessionPath?: string;
  provider?: string;
  modelId?: string;
  level?: string;
  decision?: string;
}

export interface AdapterHost {
  emit(event: Record<string, unknown>): void;
  respond(command: string, success: boolean, data?: unknown, error?: string, id?: string): void;
}

export interface AdapterStartOptions {
  cwd: string;
  sessionDir: string;
  developerInstructions?: string;
  env: NodeJS.ProcessEnv;
}

/**
 * One locally installed official CLI. The host (Pitex) only speaks pi RPC.
 * The adapter owns the CLI process, its login, and the event translation.
 */
export interface CliAdapter {
  readonly id: string;
  readonly displayName: string;
  /** User-facing reason the CLI cannot run, or null after a successful start. */
  readonly unavailableReason: string | null;
  start(options: AdapterStartOptions): Promise<void>;
  handleCommand(command: PiCommand, host: AdapterHost): Promise<void>;
  stop(): Promise<void>;
}

export async function createAdapter(id: string): Promise<CliAdapter> {
  switch (id) {
    case "codex": {
      const { CodexAdapter } = await import("./codex-adapter.ts");
      return new CodexAdapter();
    }
    case "claude": {
      const { ClaudeAdapter } = await import("./claude-adapter.ts");
      return new ClaudeAdapter();
    }
    case "grok": {
      const { GrokAdapter } = await import("./grok-adapter.ts");
      return new GrokAdapter();
    }
    default:
      throw new Error(
        `Unknown CLI adapter "${id}". This Pitex build can drive the Codex CLI, Claude Code, or the Grok CLI.`
      );
  }
}
