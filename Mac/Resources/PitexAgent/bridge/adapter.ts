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
  /** prompt / steer: pi's ImageContent (base64 `data`). */
  images?: PiImage[];
}

/** pi's `ImageContent`; `type` is optional on the wire. */
export interface PiImage {
  type?: string;
  mimeType: string;
  data: string;
}

/** What `get_state.capabilities` reports: may the host steer a running turn, and send images. */
export interface Capabilities {
  steer: boolean;
  images: boolean;
}

export const MAX_IMAGES = 8;
export const MAX_IMAGE_BYTES = 5 * 1024 * 1024;
const IMAGE_TYPES = new Set(["image/png", "image/jpeg", "image/gif", "image/webp"]);

/** The prompt's images after the caps every backend shares (Claude's own limits); throws a user-facing error. */
export function promptImages(command: PiCommand): PiImage[] {
  const images = Array.isArray(command.images) ? command.images : [];
  if (images.length > MAX_IMAGES) throw new Error(`A message can carry at most ${MAX_IMAGES} images.`);
  for (const image of images) {
    if (!image || typeof image.data !== "string" || !IMAGE_TYPES.has(image.mimeType)) {
      throw new Error("Only PNG, JPEG, GIF and WebP images can be sent.");
    }
    // base64: 4 characters carry 3 bytes.
    if (Math.floor((image.data.length * 3) / 4) > MAX_IMAGE_BYTES) {
      throw new Error("Images must be 5 MB or smaller.");
    }
  }
  return images.map((image) => ({ mimeType: image.mimeType, data: image.data }));
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
