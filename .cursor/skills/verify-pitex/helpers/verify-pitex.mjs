#!/usr/bin/env node
import { spawn } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync, readdirSync, existsSync, readlinkSync, rmSync, copyFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(HERE, "../../../..");
const BRIDGE = join(REPO, "Mac/Resources/PitexAgent/bridge/cli.ts");
const EVIDENCE_ROOT = join(REPO, ".cursor/skills/verify-pitex/evidence");
const FAKES = join(HERE, "fakes");
const INSTRUCTIONS = "Pitex verification instructions.";

const PROVIDERS = ["codex", "claude", "grok"];
const GTK_PROVIDERS = [...PROVIDERS, "stale-agy"];
const PATH_ENV = { codex: "CODEX_PATH", claude: "CLAUDE_PATH", grok: "GROK_PATH" };
const FAKE_BIN = { codex: "codex.mjs", claude: "claude.mjs", grok: "grok.mjs" };

const PROBE = {
  ANTHROPIC_API_KEY: "pitex-verify-probe-api-key",
  ANTHROPIC_AUTH_TOKEN: "pitex-verify-probe-auth-token",
  CLAUDE_CODE_OAUTH_TOKEN: "pitex-verify-probe-oauth",
  AWS_BEARER_TOKEN_BEDROCK: "pitex-verify-probe-bedrock",
  CLAUDE_CODE_USE_BEDROCK: "1",
  CLAUDE_CODE_USE_VERTEX: "1",
  CLAUDE_CODE_USE_FOUNDRY: "1",
  ANTHROPIC_VERTEX_PROJECT_ID: "pitex-verify-probe-vertex",
  ANTHROPIC_FOUNDRY_API_KEY: "pitex-verify-probe-foundry",
  ANTHROPIC_BASE_URL: "https://probe.invalid/anthropic",
  ANTHROPIC_BEDROCK_BASE_URL: "https://probe.invalid/bedrock",
  ANTHROPIC_VERTEX_BASE_URL: "https://probe.invalid/vertex",
  ANTHROPIC_FOUNDRY_BASE_URL: "https://probe.invalid/foundry",
  ANTHROPIC_MODEL: "claude-sonnet-keep",
};

const MUST_STRIP = [
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "CLAUDE_CODE_OAUTH_TOKEN",
  "AWS_BEARER_TOKEN_BEDROCK",
  "CLAUDE_CODE_USE_BEDROCK",
  "CLAUDE_CODE_USE_VERTEX",
  "CLAUDE_CODE_USE_FOUNDRY",
  "ANTHROPIC_VERTEX_PROJECT_ID",
  "ANTHROPIC_FOUNDRY_API_KEY",
  "ANTHROPIC_BASE_URL",
  "ANTHROPIC_BEDROCK_BASE_URL",
  "ANTHROPIC_VERTEX_BASE_URL",
  "ANTHROPIC_FOUNDRY_BASE_URL",
];

const MISSING = {
  codex: "Codex CLI was not found",
  claude: "Claude Code was not found",
  grok: "Grok CLI was not found",
};

function usage() {
  console.log(`verify-pitex doctor
verify-pitex drive --provider <codex|claude|grok|all>
verify-pitex cleanup
verify-pitex gtk --provider <codex|claude|grok|stale-agy>`);
}

function check(id, pass, observed) {
  return { id, pass: Boolean(pass), observed: String(observed).slice(0, 800) };
}

function sleep(ms) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, ms));
}

function childPids(pid) {
  try {
    const text = readFileSync(`/proc/${pid}/task/${pid}/children`, "utf8");
    return text.trim().split(/\s+/).filter(Boolean).map((item) => Number(item));
  } catch {
    return [];
  }
}

function descendants(root) {
  const seen = new Set();
  const stack = [root];
  const all = [];
  while (stack.length) {
    const pid = stack.pop();
    if (!pid || seen.has(pid)) continue;
    seen.add(pid);
    all.push(pid);
    for (const child of childPids(pid)) stack.push(child);
  }
  return all;
}

function procFact(pid) {
  let cwd = "";
  let cmdline = [];
  let envPresent = {};
  try {
    cwd = readlinkSync(`/proc/${pid}/cwd`);
  } catch {
    cwd = "";
  }
  try {
    cmdline = readFileSync(`/proc/${pid}/cmdline`).toString("utf8").split("\0").filter(Boolean);
  } catch {
    cmdline = [];
  }
  try {
    const raw = readFileSync(`/proc/${pid}/environ`);
    for (const entry of raw.toString("utf8").split("\0")) {
      const eq = entry.indexOf("=");
      if (eq <= 0) continue;
      const key = entry.slice(0, eq);
      if (!Object.prototype.hasOwnProperty.call(PROBE, key)) continue;
      const value = entry.slice(eq + 1);
      envPresent[key] = value === PROBE[key] ? "probe" : "other";
    }
  } catch {
    envPresent = {};
  }
  return { pid, cwd, cmdline, envPresent };
}

class Bridge {
  constructor(child, evidenceDir) {
    this.child = child;
    this.evidenceDir = evidenceDir;
    this.messages = [];
    this.stderr = "";
    this.sent = [];
    this.procs = [];
    this.buf = "";
    this.pids = new Set([child.pid]);
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      this.buf += chunk;
      let nl;
      while ((nl = this.buf.indexOf("\n")) >= 0) {
        const line = this.buf.slice(0, nl).trim();
        this.buf = this.buf.slice(nl + 1);
        if (!line) continue;
        try {
          this.messages.push(JSON.parse(line));
        } catch {
          this.messages.push({ type: "non-json", line });
        }
      }
    });
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => {
      this.stderr += chunk;
      if (this.stderr.length > 20000) this.stderr = this.stderr.slice(-20000);
    });
  }

  send(message) {
    this.sent.push(message);
    this.child.stdin.write(`${JSON.stringify(message)}\n`);
  }

  sample() {
    for (const pid of descendants(this.child.pid)) {
      this.pids.add(pid);
      const fact = procFact(pid);
      if (fact.cmdline.length === 0 && fact.cwd === "") continue;
      const key = JSON.stringify({ pid: fact.pid, cwd: fact.cwd, cmdline: fact.cmdline, envPresent: fact.envPresent });
      if (!this.procs.some((item) => JSON.stringify(item) === key)) this.procs.push(fact);
    }
    this.writePids();
  }

  writePids() {
    writeFileSync(join(this.evidenceDir, "pids.json"), `${JSON.stringify([...this.pids])}\n`);
  }

  async waitFor(pred, ms, label) {
    const start = Date.now();
    while (Date.now() - start < ms) {
      this.sample();
      if (pred()) return;
      await sleep(40);
    }
    const tail = JSON.stringify(this.messages.slice(-6));
    throw new Error(`timed out waiting for ${label}: ${tail} stderr=${this.stderr.slice(-500)}`);
  }

  responses(command, from) {
    return this.messages.slice(from).filter((message) => message.type === "response" && message.command === command);
  }

  events(type, from) {
    return this.messages.slice(from).filter((message) => message.type === type);
  }

  textFrom(from) {
    let text = "";
    for (const message of this.messages.slice(from)) {
      const event = message.assistantMessageEvent;
      if (message.type === "message_update" && event?.type === "text_delta" && typeof event.delta === "string") {
        text += event.delta;
      }
    }
    return text;
  }

  async stop() {
    if (this.child.exitCode !== null || this.child.killed) return;
    this.child.stdin.end();
    this.child.kill("SIGTERM");
    const pid = this.child.pid;
    await Promise.race([
      new Promise((resolvePromise) => this.child.once("exit", resolvePromise)),
      sleep(1500).then(() => {
        try {
          process.kill(pid, "SIGKILL");
        } catch {
          return;
        }
      }),
    ]);
  }
}

function prepareRun(provider, journey) {
  const id = `${Date.now()}-${provider}-${journey}`;
  const home = join("/tmp", "pitex-verify", id);
  const project = join(home, "project");
  const outside = join(home, "outside");
  const sessions = join(home, "sessions");
  const evidence = join(EVIDENCE_ROOT, id);
  mkdirSync(project, { recursive: true });
  mkdirSync(outside, { recursive: true });
  mkdirSync(sessions, { recursive: true });
  mkdirSync(evidence, { recursive: true });
  writeFileSync(join(project, "main.tex"), "\\documentclass{article}\\begin{document}Title\\end{document}\n");
  writeFileSync(join(project, "note.txt"), "inside-note\n");
  writeFileSync(join(outside, "marker.txt"), "outside-marker\n");
  return { id, home, project, outside, sessions, evidence, provider, journey };
}

function bridgeEnv(run, mode) {
  const env = {
    PATH: dirname(process.execPath),
    HOME: run.home,
    TMPDIR: "/tmp",
    LANG: "C.UTF-8",
    PITEX_SESSION_DIR: run.sessions,
    PITEX_DEVELOPER_INSTRUCTIONS: INSTRUCTIONS,
    FAKE_RECORD: join(run.evidence, "cli-record.jsonl"),
    FAKE_INSIDE: join(run.project, "note.txt"),
    FAKE_OUTSIDE_FILE: join(run.outside, "marker.txt"),
    FAKE_OUTSIDE_DIR: run.outside,
    FAKE_ROOT: run.project,
    FAKE_NODE: process.execPath,
    FAKE_CWD_FILE: join(run.project, "terminal-cwd.txt"),
  };
  for (const [key, value] of Object.entries(PROBE)) env[key] = value;
  if (PATH_ENV[run.provider]) env.PITEX_CLI_ADAPTER = run.provider;
  if ((mode === "ok" || mode === "signed-out") && PATH_ENV[run.provider]) {
    env[PATH_ENV[run.provider]] = join(FAKES, FAKE_BIN[run.provider]);
    if (mode === "signed-out") env.FAKE_AUTH = "signed-out";
  }
  writeFileSync(env.FAKE_RECORD, "");
  return env;
}

function startBridge(run, mode) {
  const env = bridgeEnv(run, mode);
  const child = spawn(process.execPath, [BRIDGE], {
    cwd: run.project,
    env,
    stdio: ["pipe", "pipe", "pipe"],
  });
  const bridge = new Bridge(child, run.evidence);
  bridge.writePids();
  return bridge;
}

function saveExchange(run, bridge, checks) {
  writeFileSync(join(run.evidence, "stdout.jsonl"), `${bridge.messages.map((message) => JSON.stringify(message)).join("\n")}\n`);
  writeFileSync(join(run.evidence, "stdin.jsonl"), `${bridge.sent.map((message) => JSON.stringify(message)).join("\n")}\n`);
  writeFileSync(join(run.evidence, "stderr.txt"), bridge.stderr);
  writeFileSync(join(run.evidence, "proc.json"), `${JSON.stringify(bridge.procs, null, 2)}\n`);
  writeFileSync(join(run.evidence, "checks.json"), `${JSON.stringify(checks, null, 2)}\n`);
  const indexPath = join(run.sessions, "codex-session-index.json");
  if (existsSync(indexPath)) copyFileSync(indexPath, join(run.evidence, "codex-session-index.json"));
}

function recordText(run) {
  const path = join(run.evidence, "cli-record.jsonl");
  return existsSync(path) ? readFileSync(path, "utf8") : "";
}

async function handshake(bridge) {
  const from = bridge.messages.length;
  bridge.send({ type: "get_state", id: "state" });
  bridge.send({ type: "get_available_models", id: "models" });
  bridge.send({ type: "get_session_stats", id: "stats" });
  bridge.send({ type: "get_commands", id: "commands" });
  await bridge.waitFor(() => bridge.responses("get_commands", from).length > 0, 25000, "get_commands");
  return from;
}

async function driveSession(provider) {
  const run = prepareRun(provider, "session");
  const bridge = startBridge(run, "ok");
  const checks = [];
  try {
    const ready = await handshake(bridge);
    const models = bridge.responses("get_available_models", ready).at(-1);
    checks.push(check("models", models?.success === true, JSON.stringify(models?.data ?? models?.error)));
    const first = await oneTurn(bridge, run, {
      message: "Fix the title",
      decision: "approve",
      text: "Hello",
      tool: true,
      approval: true,
    });
    checks.push(...first);
    checks.push(...(await oneTurn(bridge, run, { message: "DENY the build", decision: "deny", text: "Hello", tool: true, approval: true })).filter((item) => item.id === "approval-decision"));
    checks.push(...(await oneTurn(bridge, run, { message: "ALWAYS allow the build", decision: "always_allow", text: "Hello", tool: true, approval: true })).filter((item) => item.id === "approval-decision"));
    if (provider === "grok") checks.push(...(await jailTurn(bridge, run)));
    checks.push(...(await abortTurn(bridge)));
    checks.push(...(await resumeTurn(bridge, run, provider)));
    checks.push(...envChecks(provider, run, bridge));
    checks.push(instructionsCheck(run));
    checks.push(leakCheck(bridge));
  } catch (error) {
    checks.push(check("session-threw", false, error instanceof Error ? error.message : String(error)));
  } finally {
    bridge.sample();
    saveExchange(run, bridge, checks);
    await bridge.stop();
  }
  return { run, checks };
}

async function oneTurn(bridge, run, spec) {
  const from = bridge.messages.length;
  bridge.send({ type: "prompt", id: `p-${from}`, message: spec.message });
  await bridge.waitFor(() => bridge.responses("prompt", from).some((message) => message.id === `p-${from}`), 15000, "prompt response");
  const prompt = bridge.responses("prompt", from).find((message) => message.id === `p-${from}`);
  const checks = [check("prompt-accepted", prompt?.success === true, JSON.stringify(prompt))];
  if (spec.approval) {
    await bridge.waitFor(() => bridge.events("approval_request", from).length > 0, 15000, "approval_request");
    const approval = bridge.events("approval_request", from)[0];
    checks.push(check("approval-row", Boolean(approval?.id), JSON.stringify({ id: approval?.id, title: approval?.title, detail: approval?.detail })));
    const text = bridge.textFrom(from);
    checks.push(check("assistant-text", text.includes(spec.text), text));
    const tool = bridge.events("tool_execution_start", from)[0];
    checks.push(check("tool-row", Boolean(tool), JSON.stringify(tool ?? null)));
    bridge.send({ type: "approval_response", id: approval.id, decision: spec.decision });
    await bridge.waitFor(() => bridge.responses("approval_response", from).length > 0, 10000, "approval_response");
    await bridge.waitFor(() => bridge.events("agent_end", from).length > 0, 10000, "agent_end");
    const recorded = recordText(run);
    checks.push(check("approval-decision", decisionSeen(spec.decision, recorded), decisionSnippet(recorded)));
  } else {
    await bridge.waitFor(() => bridge.textFrom(from).includes(spec.text) || bridge.events("agent_end", from).length > 0, 15000, "assistant text");
    checks.push(check("assistant-text", bridge.textFrom(from).includes(spec.text), bridge.textFrom(from)));
    const tool = bridge.events("tool_execution_start", from)[0];
    checks.push(check("tool-row", Boolean(tool), JSON.stringify(tool ?? null)));
    checks.push(check("no-approval-row", bridge.events("approval_request", from).length === 0, String(bridge.events("approval_request", from).length)));
  }
  return checks;
}

function decisionSeen(decision, recorded) {
  if (decision === "approve") return /accept"|allow-once|user_temporary|"allow"/.test(recorded);
  if (decision === "deny") return /decline|reject-once|user_reject|"deny"/.test(recorded);
  return /acceptForSession|allow-always|user_permanent/.test(recorded);
}

function decisionSnippet(recorded) {
  const lines = recorded.split("\n").filter((line) => /decision|optionId|behavior|accept|decline|allow-|reject-/.test(line));
  return lines.slice(-4).join(" | ").slice(0, 500);
}

async function jailTurn(bridge, run) {
  const from = bridge.messages.length;
  bridge.send({ type: "prompt", id: "jail", message: "JAIL the tools" });
  await bridge.waitFor(() => recordText(run).includes("outside this project"), 10000, "jail error");
  await bridge.waitFor(() => bridge.events("approval_request", from).length > 0, 10000, "terminal approval");
  const approval = bridge.events("approval_request", from)[0];
  bridge.send({ type: "approval_response", id: approval.id, decision: "approve" });
  await bridge.waitFor(() => bridge.events("agent_end", from).length > 0, 10000, "jail end");
  bridge.sample();
  const recorded = recordText(run);
  const insideFile = existsSync(join(run.project, "from-grok.txt")) ? readFileSync(join(run.project, "from-grok.txt"), "utf8") : "";
  const outsideWritten = existsSync(join(run.outside, "pwned.txt"));
  const outsideMarker = readFileSync(join(run.outside, "marker.txt"), "utf8");
  const cwdFile = join(run.project, "terminal-cwd.txt");
  const terminalCwd = existsSync(cwdFile) ? readFileSync(cwdFile, "utf8").trim() : "";
  const cwdHit = bridge.procs.some((fact) => fact.cwd === run.project && fact.cmdline.some((part) => part.includes("FAKE_CWD_FILE") || part.includes("scoped-ok")));
  return [
    check("jail-outside-read", recorded.includes("outside this project") && recorded.includes("inside-note"), "record has the inside note and the outside refusal"),
    check("jail-outside-write", insideFile.includes("written-inside") && !outsideWritten, `inside=${JSON.stringify(insideFile)} outsideWritten=${outsideWritten}`),
    check("jail-outside-marker", outsideMarker === "outside-marker\n", outsideMarker),
    check("jail-terminal-cwd", terminalCwd === run.project && recorded.includes("scoped-ok"), `terminalCwd=${terminalCwd} proc=${cwdHit}`),
    check("grok-cwd", bridge.procs.some((fact) => fact.cmdline.some((part) => part.endsWith("grok.mjs")) && fact.cwd === run.project), "grok process cwd"),
  ];
}

function spawnRows(run) {
  const rows = [];
  for (const line of recordText(run).split("\n")) {
    if (!line.trim()) continue;
    try {
      const row = JSON.parse(line);
      if (row.kind === "spawn") rows.push(row);
    } catch {
      continue;
    }
  }
  return rows;
}

async function abortTurn(bridge) {
  const from = bridge.messages.length;
  bridge.send({ type: "prompt", id: "hold", message: "HOLD this turn" });
  await bridge.waitFor(() => bridge.textFrom(from).includes("Working") || bridge.responses("prompt", from).some((message) => message.success === false), 15000, "hold text");
  bridge.send({ type: "abort", id: "abort" });
  await bridge.waitFor(() => bridge.responses("abort", from).length > 0, 10000, "abort");
  const reply = bridge.responses("abort", from)[0];
  return [check("abort", reply?.success === true, JSON.stringify(reply))];
}

async function resumeTurn(bridge, run, provider) {
  const indexPath = join(run.sessions, "codex-session-index.json");
  if (!existsSync(indexPath)) return [check("session-index", false, "missing codex-session-index.json")];
  const before = JSON.parse(readFileSync(indexPath, "utf8"));
  const firstPath = Object.keys(before.sessions ?? {})[0];
  const threadId = before.sessions?.[firstPath]?.threadId;
  const checks = [check("session-index", Boolean(firstPath && threadId), JSON.stringify(before.sessions))];
  const fromNew = bridge.messages.length;
  bridge.send({ type: "new_session", id: "new" });
  await bridge.waitFor(() => bridge.responses("new_session", fromNew).length > 0, 15000, "new_session");
  const created = bridge.responses("new_session", fromNew)[0];
  checks.push(check("new-session", created?.success === true, JSON.stringify(created)));
  bridge.send({ type: "prompt", id: "plain", message: "PLAIN second conversation" });
  await bridge.waitFor(() => bridge.events("agent_end", fromNew).length > 0 || bridge.textFrom(fromNew).includes("Noted"), 20000, "second conversation");
  const after = JSON.parse(readFileSync(indexPath, "utf8"));
  const paths = Object.keys(after.sessions ?? {});
  checks.push(check("second-session-file", paths.length >= 2 || paths.some((path) => path !== firstPath), paths.join(",")));
  const fromSwitch = bridge.messages.length;
  bridge.send({ type: "switch_session", id: "resume", sessionPath: firstPath });
  await bridge.waitFor(() => bridge.responses("switch_session", fromSwitch).length > 0, 15000, "switch_session");
  const resumed = bridge.responses("switch_session", fromSwitch)[0];
  checks.push(check("resume", resumed?.success === true, JSON.stringify(resumed)));
  bridge.send({ type: "get_messages", id: "history" });
  await bridge.waitFor(() => bridge.responses("get_messages", fromSwitch).length > 0, 10000, "get_messages");
  const history = JSON.stringify(bridge.responses("get_messages", fromSwitch)[0]?.data ?? {});
  const restored = provider === "grok" ? history.includes("Restored") || history.includes("Fix the title") : history.includes("Fix the title") || history.includes("Hello");
  checks.push(check("resumed-messages", restored, history.slice(0, 400)));
  return checks;
}

function claudeChildKeys(run, bridge) {
  const present = {};
  for (const row of spawnRows(run)) {
    for (const key of Object.keys(row.envPresent ?? {})) present[key] = true;
  }
  for (const fact of bridge.procs) {
    if (!fact.cmdline.some((part) => part.includes("claude.mjs"))) continue;
    for (const key of Object.keys(fact.envPresent)) present[key] = true;
  }
  return present;
}

function envChecks(provider, run, bridge) {
  if (provider !== "claude") return [check("env-strip", true, "not the claude child")];
  const present = claudeChildKeys(run, bridge);
  const parentHadProbe = bridge.procs.some((fact) => fact.cmdline.some((part) => part.endsWith("cli.ts")) && fact.envPresent.ANTHROPIC_API_KEY === "probe");
  const leaked = MUST_STRIP.filter((key) => present[key]);
  const sawSpawn = spawnRows(run).length > 0;
  return [
    check("claude-parent-had-probes", parentHadProbe, parentHadProbe ? "bridge environ contained the probe keys" : "bridge probe keys were not observed"),
    check("claude-spawn-observed", sawSpawn, `child keys: ${Object.keys(present).join(",")}`),
    check("env-strip", leaked.length === 0 && sawSpawn && parentHadProbe, leaked.length ? `still present: ${leaked.join(",")}` : "billing and endpoint keys absent on the claude child"),
  ];
}

function instructionsCheck(run) {
  const recorded = recordText(run);
  return check("developer-instructions", recorded.includes(INSTRUCTIONS), recorded.includes(INSTRUCTIONS) ? "present on CLI stdin" : "absent from CLI stdin");
}

function leakCheck(bridge) {
  const blob = JSON.stringify(bridge.messages);
  return check("no-token-on-pi-stream", !blob.includes("must-not-leak") && !blob.includes("pitex-verify-probe"), "pi stdout");
}

async function driveShort(provider, mode) {
  const run = prepareRun(provider, mode);
  const bridge = startBridge(run, mode === "missing" ? "missing" : "signed-out");
  const checks = [];
  const needle = mode === "missing" ? MISSING[provider] : "not logged in";
  try {
    await bridge.waitFor(() => bridge.messages.some((message) => message.type === "extension_error" || message.type === "response"), 20000, "startup status");
    const from = bridge.messages.length;
    bridge.send({ type: "prompt", id: "missing-prompt", message: "Hi" });
    await bridge.waitFor(() => bridge.responses("prompt", from).length > 0 || bridge.events("extension_error", 0).length > 0, 10000, "prompt after status");
    const blob = JSON.stringify(bridge.messages);
    const reply = bridge.responses("prompt", from)[0];
    const failedPrompt = reply ? reply.success === false && String(reply.error ?? "").includes(needle) : blob.includes(needle);
    checks.push(check(mode, blob.includes(needle) && failedPrompt, blob.slice(0, 500)));
    checks.push(check("bridge-stays-up", bridge.child.exitCode === null, `exit=${bridge.child.exitCode}`));
    checks.push(leakCheck(bridge));
  } catch (error) {
    checks.push(check(mode, false, error instanceof Error ? error.message : String(error)));
  } finally {
    bridge.sample();
    saveExchange(run, bridge, checks);
    await bridge.stop();
  }
  return { run, checks };
}

function summarize(provider, cli, parts) {
  const checks = parts.flatMap((part) => part.checks);
  return {
    provider,
    cli,
    pass: checks.every((item) => item.pass),
    checks,
    evidence: parts.map((part) => part.run.evidence),
  };
}

async function driveProvider(provider) {
  const session = await driveSession(provider);
  const missing = await driveShort(provider, "missing");
  const signedOut = await driveShort(provider, "signed-out");
  return summarize(provider, "fake", [session, missing, signedOut]);
}

function driveGtk(provider) {
  if (!GTK_PROVIDERS.includes(provider)) return Promise.resolve(2);
  const binary = join(REPO, "Linux/target/debug/pitex");
  if (!existsSync(binary)) {
    console.log(JSON.stringify({ pass: false, observed: `missing ${binary}` }));
    return Promise.resolve(2);
  }
  const run = prepareRun(provider, "gtk");
  const env = {
    ...bridgeEnv(run, "ok"),
    HOME: run.home,
    XDG_CONFIG_HOME: join(run.home, ".config"),
    XDG_DATA_HOME: join(run.home, ".local/share"),
    XDG_CACHE_HOME: join(run.home, ".cache"),
    XDG_RUNTIME_DIR: join(run.home, "runtime"),
    GSK_RENDERER: "cairo",
    PITEX_BIN: binary,
    PITEX_GTK_EVIDENCE: run.evidence,
    PITEX_GTK_PROJECT: run.project,
    PITEX_GTK_PROVIDER: provider,
    DISPLAY: process.env.DISPLAY || "",
    PATH: `${dirname(process.execPath)}:/usr/bin:/bin`,
  };
  mkdirSync(env.XDG_RUNTIME_DIR, { recursive: true, mode: 0o700 });
  const child = spawn("xvfb-run", ["-a", "-s", "-screen 0 1280x800x24", "dbus-run-session", "--", "python3", join(HERE, "gtk-drive.py")], {
    env,
    stdio: "inherit",
  });
  return new Promise((resolvePromise) => {
    child.on("exit", (code) => {
      const resultPath = join(run.evidence, "gtk-result.json");
      if (existsSync(resultPath)) console.log(readFileSync(resultPath, "utf8"));
      resolvePromise(code ?? 1);
    });
  });
}

function doctor() {
  const bridgeOk = existsSync(BRIDGE);
  const nodeOk = process.versions.node;
  const fakes = PROVIDERS.map((provider) => existsSync(join(FAKES, FAKE_BIN[provider])));
  const report = {
    worthDriving: bridgeOk && fakes.every(Boolean),
    node: nodeOk,
    bridge: BRIDGE,
    bridgeExists: bridgeOk,
    fakesOk: fakes.every(Boolean),
    head: "",
  };
  try {
    report.head = readFileSync(join(REPO, ".git/HEAD"), "utf8").trim();
  } catch {
    report.head = "";
  }
  console.log(JSON.stringify(report, null, 2));
  return report.worthDriving ? 0 : 1;
}

function cleanup() {
  if (!existsSync(EVIDENCE_ROOT)) return 0;
  let killed = 0;
  for (const name of readdirSync(EVIDENCE_ROOT)) {
    const pidFile = join(EVIDENCE_ROOT, name, "pids.json");
    if (!existsSync(pidFile)) continue;
    const pids = JSON.parse(readFileSync(pidFile, "utf8"));
    for (const pid of pids) {
      let cmdline = "";
      try {
        cmdline = readFileSync(`/proc/${pid}/cmdline`).toString("utf8");
      } catch {
        continue;
      }
      if (!cmdline.includes("cli.ts") && !cmdline.includes("verify-pitex") && !cmdline.includes("/fakes/")) continue;
      try {
        process.kill(pid, "SIGTERM");
        killed += 1;
      } catch {
        continue;
      }
    }
  }
  console.log(JSON.stringify({ killed, evidence: EVIDENCE_ROOT }));
  return 0;
}

function parseArgs(argv) {
  const out = { command: argv[0] ?? "doctor", provider: "all" };
  for (let i = 1; i < argv.length; i += 1) {
    if (argv[i] === "--provider") out.provider = argv[i + 1];
  }
  return out;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.command === "doctor") process.exit(doctor());
  if (args.command === "cleanup") process.exit(cleanup());
  if (args.command === "gtk") {
    const provider = args.provider === "all" ? "codex" : args.provider;
    if (!GTK_PROVIDERS.includes(provider)) {
      usage();
      process.exit(2);
    }
    process.exit(await driveGtk(provider));
  }
  if (args.command !== "drive") {
    usage();
    process.exit(2);
  }
  mkdirSync(EVIDENCE_ROOT, { recursive: true });
  const providers = args.provider === "all" ? PROVIDERS : [args.provider];
  if (providers.some((provider) => !PROVIDERS.includes(provider))) {
    usage();
    process.exit(2);
  }
  const reports = [];
  for (const provider of providers) reports.push(await driveProvider(provider));
  const summaryPath = join(EVIDENCE_ROOT, "summary.json");
  writeFileSync(summaryPath, `${JSON.stringify(reports, null, 2)}\n`);
  for (const report of reports) {
    const failed = report.checks.filter((item) => !item.pass).map((item) => item.id);
    console.log(`${report.pass ? "PASS" : "FAIL"} ${report.provider} fake ${failed.join(",")}`);
  }
  process.exit(reports.every((report) => report.pass) ? 0 : 1);
}

main();
