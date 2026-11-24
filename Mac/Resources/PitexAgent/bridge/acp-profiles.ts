// Per-CLI settings for the shared ACP adapter (acp-adapter.ts). Each profile
// names one official CLI that speaks the Agent Client Protocol on stdio and
// signs in through its own login. Pitex never reads that CLI's credential
// or session files.

export interface AcpProfile {
  /** Adapter id (PITEX_CLI_ADAPTER) and the provider label on models and commands. */
  id: string;
  displayName: string;
  /** Executable base name looked up on PATH and in the usual install folders. */
  binary: string;
  /** Environment variable that names an explicit binary. */
  pathEnv: string;
  args: readonly string[];
  /**
   * `cached_token`: `initialize` must offer that method and Pitex calls `authenticate`.
   * `cli`: no authenticate call; the CLI uses its own sign-in, and an auth error
   * from session/new, session/load or session/prompt reads as signed out.
   */
  auth: "cached_token" | "cli";
  clientCapabilities: Record<string, unknown>;
  /**
   * `all`: config options, `models.availableModels`, and the `x.ai/models/list` probe.
   * `config`: only the `model` and `thought_level` config options. Effort is
   * re-read after every model change and hidden when the model has none.
   */
  modelSource: "all" | "config";
  /** `rules`: `_meta.rules`. `antigravity`: `_meta.antigravity.systemPrompt.append`, when `initialize` advertises it. */
  instructions: "rules" | "antigravity";
  /** Permission mode forced (and checked) after session/new and session/load, so the CLI cannot run in accept-edits. */
  forceMode: string | null;
  /** Mirror the CLI's own sessions for this project into the history list (session/list). */
  importSessions: boolean;
  /** Replayed history after session/load stays out of the live stream until this much quiet. */
  replayQuietMs: number;
  /** Keep the turn open after end_turn while a foreground command is still running. */
  deferTurnOnRunningTools: boolean;
  /** Shown once per bridge run, on the first host command, when the CLI is available. */
  notice?: string;
  missing: string;
  signedOut: string;
  keyringLocked?: string;
  noResume: string;
}

export const GROK: AcpProfile = {
  id: "grok",
  displayName: "Grok CLI",
  binary: "grok",
  pathEnv: "GROK_PATH",
  args: ["--no-auto-update", "agent", "stdio"],
  auth: "cached_token",
  // Filesystem reads and writes are implemented and jailed to the project.
  // terminal/create refuses a cwd outside the project and waits for the
  // approval row before the process starts.
  clientCapabilities: { fs: { readTextFile: true, writeTextFile: true }, terminal: true },
  modelSource: "all",
  instructions: "rules",
  forceMode: null,
  importSessions: false,
  replayQuietMs: 0,
  deferTurnOnRunningTools: false,
  missing: "Grok CLI was not found. Install the official Grok CLI, then run `grok login`.",
  signedOut: "Grok CLI is not logged in. Run `grok login` in a terminal, then try again.",
  noResume:
    "This Grok CLI cannot resume a saved session (it does not advertise loadSession). Start a new conversation.",
};

/**
 * `agy acp` (Antigravity CLI 1.5 or later). agy runs its own tools and asks
 * through session/request_permission, so Pitex advertises no fs or terminal
 * methods. `--sandbox` keeps the CLI's own containment on. Pitex never
 * passes `--dangerously-skip-permissions`.
 */
export const AGY: AcpProfile = {
  id: "agy",
  displayName: "Antigravity CLI",
  binary: "agy",
  pathEnv: "AGY_PATH",
  args: ["acp", "--sandbox", "--disable-telemetry"],
  auth: "cli",
  clientCapabilities: {},
  modelSource: "config",
  instructions: "antigravity",
  forceMode: "default",
  importSessions: true,
  replayQuietMs: 200,
  deferTurnOnRunningTools: true,
  // Measured on agy 1.5.15 in mode default: write_file inside the project runs without
  // session/request_permission, and ACP offers no stricter mode. Say so instead of hiding it.
  notice:
    "Antigravity CLI asks before it runs a command, but it edits files in this project without asking. That follows agy's own permission settings, which Pitex cannot change.",
  missing:
    "Antigravity CLI was not found. Install the official Antigravity CLI, then run `agy` and sign in.",
  signedOut: "Antigravity CLI is not logged in. Run `agy` in a terminal and sign in, then try again.",
  keyringLocked:
    "Antigravity CLI could not read the system keyring. Unlock it, run `agy` in a terminal and sign in, then try again.",
  noResume:
    "This Antigravity CLI cannot resume a saved session (it does not advertise loadSession). Start a new conversation.",
};
