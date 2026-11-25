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
  /** approval_response: the `actions[].id` the user picked. */
  actionId?: string;
  /** approval_response: question id → the chosen label(s) or the "Other" text. */
  answers?: Record<string, unknown>;
  /** approval_response: feedback for a deny or "Keep planning". */
  message?: string;
  /** set_mode */
  modeId?: string;
}

/** One button on an approval card (`approval_request.actions`). Hosts localize known ids. */
export interface ApprovalAction {
  id: string;
  label: string;
  behavior: "allow" | "deny";
  variant?: "primary" | "secondary" | "danger";
}

/** `get_state.mode` and `mode_changed.mode`: the permission mode and the ones the user may pick. */
export interface ModeState {
  id: string;
  options: { id: string; label: string }[];
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
    case "grok":
    case "agy": {
      const { AcpAdapter } = await import("./acp-adapter.ts");
      const { AGY, GROK } = await import("./acp-profiles.ts");
      return new AcpAdapter(id === "agy" ? AGY : GROK);
    }
    default:
      throw new Error(
        `Unknown CLI adapter "${id}". This Pitex build can drive the Codex CLI, Claude Code, the Grok CLI, or the Antigravity CLI.`
      );
  }
}
