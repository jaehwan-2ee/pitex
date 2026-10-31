// Claude Code adapter checks: message mapping, approvals, resume, and a
// fake SDK round trip. The real CLI and its login files are never touched.
// Run via: node --test Mac/Resources/PitexAgent/bridge/test.ts

import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import type { AdapterHost } from "./adapter.ts";
import {
  CLAUDE_MISSING,
  CLAUDE_SIGNED_OUT,
  ClaudeAdapter,
  claudeChildEnv,
  findClaude,
  type CanUseToolOptions,
  type ClaudeQueryFn,
  type ClaudeQueryHandle,
  type ClaudeQueryOptions,
} from "./claude-adapter.ts";
import {
  approvalPresentation,
  createClaudeStreamState,
  mapClaudeMessage,
  messagesFromState,
  parseClaudeAuthStatus,
  permissionForDecision,
  type Json,
} from "./claude-map.ts";

const LOCAL_SUGGESTION = [
  {
    type: "addRules",
    rules: [{ toolName: "Bash", ruleContent: "latexmk main.tex" }],
    behavior: "allow",
    destination: "localSettings",
  },
];

test("claude messages map to pi events without repeating streamed text", () => {
  const state = createClaudeStreamState();
  const events = [
    ...mapClaudeMessage(
      {
        type: "stream_event",
        event: { type: "content_block_delta", index: 0, delta: { type: "thinking_delta", thinking: "plan" } },
      },
      state
    ),
    ...mapClaudeMessage(
      {
        type: "stream_event",
        event: { type: "content_block_delta", index: 1, delta: { type: "text_delta", text: "Hello" } },
      },
      state
    ),
    ...mapClaudeMessage(
      {
        type: "assistant",
        message: {
          role: "assistant",
          content: [
            { type: "thinking", thinking: "plan" },
            { type: "text", text: "Hello" },
            { type: "tool_use", id: "t1", name: "Bash", input: { command: "latexmk" } },
          ],
        },
      },
      state
    ),
    ...mapClaudeMessage(
      {
        type: "user",
        message: { role: "user", content: [{ type: "tool_result", tool_use_id: "t1", content: "ok" }] },
      },
      state
    ),
    ...mapClaudeMessage({ type: "auth_status", error: true, output: "must-not-copy" }, state),
    ...mapClaudeMessage({ type: "result", subtype: "error", is_error: true, errors: ["boom"] }, state),
  ];
  assert.deepEqual(messageDeltas(events, "thinking_delta"), ["plan"]);
  assert.deepEqual(messageDeltas(events, "text_delta"), ["Hello"]);
  const started = events.find((event) => event.type === "tool_execution_start");
  const finished = events.find((event) => event.type === "tool_execution_end");
  assert.equal(started?.toolName, "bash");
  assert.deepEqual(started?.args, { command: "latexmk" });
  assert.equal(finished?.toolCallId, "t1");
  assert.equal(finished?.isError, false);
  const end = events.find((event) => event.type === "agent_end");
  const failed = (end?.messages as Json[] | undefined)?.[0];
  assert.equal(failed?.stopReason, "error");
  assert.equal(failed?.errorMessage, "boom");
  assert.equal(JSON.stringify(events).includes("must-not-copy"), false);
  const roles = messagesFromState(state).map((message) => message.role);
  assert.deepEqual(roles, ["assistant", "toolResult"]);
});

test("claude streamed tool input becomes one tool execution", () => {
  const state = createClaudeStreamState();
  const events = [
    ...mapClaudeMessage(
      {
        type: "stream_event",
        event: {
          type: "content_block_start",
          index: 2,
          content_block: { type: "tool_use", id: "t2", name: "Bash", input: {} },
        },
      },
      state
    ),
    ...mapClaudeMessage(
      {
        type: "stream_event",
        event: {
          type: "content_block_delta",
          index: 2,
          delta: { type: "input_json_delta", partial_json: '{"command":"latexmk main.tex"}' },
        },
      },
      state
    ),
    ...mapClaudeMessage(
      { type: "stream_event", event: { type: "content_block_stop", index: 2 } },
      state
    ),
  ];
  assert.equal(events.filter((event) => event.type === "tool_execution_start").length, 1);
  assert.equal(events.at(-1)?.toolName, "bash");
  assert.deepEqual(events.at(-1)?.args, { command: "latexmk main.tex" });
});

test("claude approval cards show the command, the path, or a short summary", () => {
  const bash = approvalPresentation(
    "Bash",
    { command: "touch /tmp/pitex-claude-card", description: "Create an empty marker file in /tmp" },
    { displayName: "Bash", description: "Create an empty marker file in /tmp" }
  );
  assert.equal(bash.kind, "command");
  assert.equal(bash.title, "Run command");
  assert.equal(bash.detail, "touch /tmp/pitex-claude-card");

  const write = approvalPresentation(
    "Write",
    { file_path: "main.tex", content: "hello" },
    { displayName: "Write", description: "Write main.tex" }
  );
  assert.equal(write.kind, "file");
  assert.equal(write.title, "Edit files");
  assert.equal(write.detail, "main.tex");

  const edit = approvalPresentation(
    "Edit",
    { file_path: "chapters/intro.tex", old_string: "a", new_string: "b" },
    { displayName: "Edit" }
  );
  assert.equal(edit.kind, "file");
  assert.equal(edit.title, "Edit files");
  assert.equal(edit.detail, "chapters/intro.tex");

  const search = approvalPresentation("WebSearch", { query: "pitex latex" }, { displayName: "WebSearch" });
  assert.equal(search.kind, "permissions");
  assert.equal(search.title, "Allow action");
  assert.equal(search.detail, "WebSearch: pitex latex");

  const opaque = approvalPresentation("Mystery", { count: 2 }, { displayName: "Mystery" });
  assert.equal(opaque.title, "Allow action");
  assert.equal(opaque.detail.includes("{"), false);

  const state = createClaudeStreamState();
  const rows = mapClaudeMessage(
    {
      type: "assistant",
      message: {
        role: "assistant",
        content: [
          {
            type: "tool_use",
            id: "b",
            name: "Bash",
            input: { command: "touch /tmp/pitex-claude-card", description: "Create an empty marker file in /tmp" },
          },
          { type: "tool_use", id: "w", name: "Write", input: { file_path: "main.tex", content: "hello" } },
          { type: "tool_use", id: "e", name: "Edit", input: { file_path: "chapters/intro.tex" } },
          { type: "tool_use", id: "s", name: "WebSearch", input: { query: "pitex latex" } },
        ],
      },
    },
    state
  ).filter((event) => event.type === "tool_execution_start");
  assert.deepEqual(
    rows.map((event) => ({ toolName: event.toolName, args: event.args })),
    [
      { toolName: "bash", args: { command: "touch /tmp/pitex-claude-card" } },
      { toolName: "write", args: { path: "main.tex" } },
      { toolName: "edit", args: { path: "chapters/intro.tex" } },
      { toolName: "WebSearch", args: { summary: "WebSearch: pitex latex" } },
    ]
  );
});

test("claude permission decisions stay session scoped", () => {
  const input = { command: "latexmk main.tex" };
  const once = permissionForDecision("approve", "Bash", input, LOCAL_SUGGESTION, false, "id");
  assert.equal(once.remember, false);
  assert.equal(once.result.behavior, "allow");
  assert.equal(once.result.decisionClassification, "user_temporary");
  assert.equal(once.result.updatedPermissions, undefined);

  const always = permissionForDecision("always_allow", "Bash", input, LOCAL_SUGGESTION, false, "id");
  assert.equal(always.remember, true);
  assert.equal(always.result.decisionClassification, "user_permanent");
  const updates = always.result.updatedPermissions as { destination?: string; rules?: { ruleContent?: string }[] }[];
  assert.equal(updates[0].destination, "session");
  assert.equal(updates[0].rules?.[0].ruleContent, "latexmk main.tex");
  assert.equal(JSON.stringify(updates).includes("localSettings"), false);

  const suppressed = permissionForDecision("always_allow", "Bash", input, LOCAL_SUGGESTION, true, "id");
  assert.equal(suppressed.remember, false);
  assert.equal(suppressed.result.decisionClassification, "user_temporary");
  assert.equal(suppressed.result.updatedPermissions, undefined);

  const denied = permissionForDecision("deny", "Bash", input, LOCAL_SUGGESTION, false, "id");
  assert.equal(denied.remember, false);
  assert.equal(denied.result.behavior, "deny");
  assert.equal(denied.result.message, "The user denied this action.");
  assert.equal(denied.result.decisionClassification, "user_reject");
});

test("claude auth status reads only loggedIn", () => {
  assert.equal(
    parseClaudeAuthStatus('{"loggedIn":false,"authToken":"must-not-leak","configDirectory":"/tmp/hidden-home/.claude"}', 0),
    false
  );
  assert.equal(parseClaudeAuthStatus("not json", 0), true);
  assert.equal(parseClaudeAuthStatus('noise {"loggedIn":true,"apiKey":"ignored"} trailing', 1), true);
  assert.equal(parseClaudeAuthStatus("", 1), false);
});

test("claude adapter sources do not reference credential files", () => {
  const forbidden = [
    "auth.json",
    ".claude.json",
    "~/.claude",
    "credentials.json",
    "keychain",
    "Login Keychain",
    ".credentials",
  ];
  for (const name of ["claude-adapter.ts", "claude-map.ts"]) {
    const source = readFileSync(new URL(`./${name}`, import.meta.url), "utf8");
    for (const needle of forbidden) assert.equal(source.includes(needle), false, `${name} mentions ${needle}`);
  }
  const adapter = readFileSync(new URL("./claude-adapter.ts", import.meta.url), "utf8");
  assert.match(adapter, /permissionMode: "default"/);
  assert.match(adapter, /settingSources: \[\]/);
});

test("claude child env drops API-key and billing overrides", () => {
  const env = claudeChildEnv({
    PATH: "/usr/bin",
    HOME: "/home/me",
    PITEX_TEST: "1",
    ANTHROPIC_MODEL: "claude-sonnet-4-5",
    ANTHROPIC_API_KEY: "user-supplied",
    ANTHROPIC_AUTH_TOKEN: "user-supplied",
    CLAUDE_CODE_OAUTH_TOKEN: "user-supplied",
    ANTHROPIC_BASE_URL: "https://gateway.example/anthropic",
    ANTHROPIC_BEDROCK_BASE_URL: "https://bedrock.example",
    ANTHROPIC_VERTEX_BASE_URL: "https://vertex.example",
    ANTHROPIC_FOUNDRY_BASE_URL: "https://foundry.example",
    ANTHROPIC_AWS_API_KEY: "user-supplied",
    AWS_BEARER_TOKEN_BEDROCK: "user-supplied",
    CLAUDE_CODE_USE_BEDROCK: "1",
    CLAUDE_CODE_USE_VERTEX: "1",
    emptyValue: "",
    skipped: undefined,
  });
  assert.equal(env.PATH, "/usr/bin");
  assert.equal(env.HOME, "/home/me");
  assert.equal(env.PITEX_TEST, "1");
  assert.equal(env.ANTHROPIC_MODEL, "claude-sonnet-4-5");
  assert.equal(env.emptyValue, "");
  const removed = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_BEDROCK_BASE_URL",
    "ANTHROPIC_VERTEX_BASE_URL",
    "ANTHROPIC_FOUNDRY_BASE_URL",
    "ANTHROPIC_AWS_API_KEY",
    "AWS_BEARER_TOKEN_BEDROCK",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "skipped",
  ];
  for (const name of removed) assert.equal(Object.hasOwn(env, name), false, name);
});

test("findClaude honors CLAUDE_PATH and PATH", () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-claude-which-"));
  const cmdDir = mkdtempSync(join(tmpdir(), "pitex-claude-cmd-"));
  try {
    const binary = join(dir, "claude");
    writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
    assert.equal(findClaude({ CLAUDE_PATH: binary, PATH: "", HOME: dir }), binary);
    assert.equal(findClaude({ PATH: dir, HOME: dir }), binary);
    assert.equal(findClaude({ CLAUDE_PATH: join(dir, "missing"), PATH: "", HOME: dir }), null);
    writeFileSync(join(cmdDir, "claude.cmd"), "@echo off\r\n", { mode: 0o755 });
    assert.equal(findClaude({ PATH: cmdDir, HOME: cmdDir }, "linux"), null);
    assert.equal(findClaude({ PATH: cmdDir, HOME: cmdDir }, "win32"), join(cmdDir, "claude.cmd"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
    rmSync(cmdDir, { recursive: true, force: true });
  }
});

test("vendored claude agent sdk matches the pinned digest", () => {
  const pkg = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")) as {
    claudeAgentSdk: { version: string; sdkMjsSha256: string; optionalNativeBinaries: string };
  };
  const bytes = readFileSync(new URL("./vendor/claude-agent-sdk/sdk.mjs", import.meta.url));
  assert.equal(createHash("sha256").update(bytes).digest("hex"), pkg.claudeAgentSdk.sdkMjsSha256);
  assert.equal(pkg.claudeAgentSdk.version, "0.3.312");
  assert.equal(pkg.claudeAgentSdk.optionalNativeBinaries, "omitted");
  const names = readdirSync(fileURLToPath(new URL("./vendor/claude-agent-sdk/", import.meta.url))).sort();
  assert.deepEqual(names, ["LICENSE.md", "PITEX.md", "README.md", "package.json", "sdk.mjs"]);
});

test("vendored sdk root bundle loads without native binaries", async () => {
  const sdk = (await import(new URL("./vendor/claude-agent-sdk/sdk.mjs", import.meta.url).href)) as {
    query?: unknown;
  };
  assert.equal(typeof sdk.query, "function");
});

test("missing claude CLI reports the login hint", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-claude-missing-"));
  let calls = 0;
  const adapter = new ClaudeAdapter({
    query: () => {
      calls += 1;
      throw new Error("query should not run");
    },
  });
  const collected = collect();
  try {
    await adapter.start({ cwd: dir, sessionDir: dir, env: { PATH: "", HOME: dir } });
    assert.equal(adapter.unavailableReason, CLAUDE_MISSING);
    assert.match(adapter.unavailableReason ?? "", /claude auth login/);
    assert.equal(calls, 0);
    await adapter.handleCommand({ type: "prompt", id: "p", message: "Hi" }, collected.host);
    const reply = response(collected.responses, "prompt");
    assert.equal(reply.success, false);
    assert.equal(reply.error, CLAUDE_MISSING);
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

test("signed-out claude does not leak auth status fields", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-claude-signed-out-"));
  const binary = join(dir, "claude");
  const marker = join(dir, "ran");
  writeFileSync(binary, `#!/bin/sh\nprintf ran > '${marker}'\n`, { mode: 0o755 });
  let calls = 0;
  const adapter = new ClaudeAdapter({
    query: () => {
      calls += 1;
      throw new Error("query should not run");
    },
    authStatus: async () => ({
      stdout: '{"loggedIn":false,"authToken":"must-not-leak","configDirectory":"/tmp/hidden-home/.claude"}',
      code: 0,
    }),
  });
  const collected = collect();
  try {
    await adapter.start({
      cwd: dir,
      sessionDir: dir,
      env: { PATH: "", HOME: dir, CLAUDE_PATH: binary },
    });
    assert.equal(adapter.unavailableReason, CLAUDE_SIGNED_OUT);
    assert.equal(calls, 0);
    assert.equal(existsSync(marker), false);
    await adapter.handleCommand({ type: "prompt", id: "p", message: "Hi" }, collected.host);
    const reply = response(collected.responses, "prompt");
    assert.equal(reply.success, false);
    assert.equal(reply.error, CLAUDE_SIGNED_OUT);
    const blob = JSON.stringify({ reason: adapter.unavailableReason, events: collected.events, responses: collected.responses });
    assert.equal(blob.includes("must-not-leak"), false);
    assert.equal(blob.includes("hidden-home"), false);
    assert.equal(blob.includes("configDirectory"), false);
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

test("claude auth_status errors do not copy CLI output", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-claude-auth-event-"));
  const binary = join(dir, "claude");
  writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
  const adapter = new ClaudeAdapter({
    query: ({ prompt }) => {
      const query = new ScriptedQuery();
      void drain(prompt);
      query.push({
        type: "auth_status",
        error: true,
        output: "sk-secret-output-value",
        configDirectory: "/tmp/hidden-home/.claude",
      });
      return query;
    },
    authStatus: async () => ({ stdout: '{"loggedIn":true}', code: 0 }),
  });
  const collected = collect();
  try {
    await adapter.start({
      cwd: dir,
      sessionDir: dir,
      env: { PATH: "", HOME: dir, CLAUDE_PATH: binary },
    });
    await adapter.handleCommand({ type: "get_state", id: "s" }, collected.host);
    await waitUntil(
      () => collected.events.some((event) => event.type === "extension_error"),
      "auth status error"
    );
    const failure = collected.events.find((event) => event.type === "extension_error");
    assert.equal(failure?.error, CLAUDE_SIGNED_OUT);
    const blob = JSON.stringify(collected.events);
    assert.equal(blob.includes("sk-secret-output-value"), false);
    assert.equal(blob.includes("hidden-home"), false);
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

test("claude fake query round trip approves, resumes, and starts a clean session", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-claude-round-"));
  const binary = join(dir, "claude");
  const marker = join(dir, "ran");
  writeFileSync(binary, `#!/bin/sh\nprintf ran > '${marker}'\n`, { mode: 0o755 });
  const opened: Opened[] = [];
  const adapter = new ClaudeAdapter({
    query: scriptedQuery(opened, "script"),
    authStatus: async (_binary, _cwd, env) => {
      assert.equal(env.PITEX_TEST, "1");
      assert.equal(env.ANTHROPIC_API_KEY, undefined);
      assert.equal(env.ANTHROPIC_AUTH_TOKEN, undefined);
      assert.equal(env.CLAUDE_CODE_OAUTH_TOKEN, undefined);
      assert.equal(env.ANTHROPIC_BASE_URL, undefined);
      return {
        stdout: '{"loggedIn":true,"authToken":"must-not-leak","configDirectory":"/tmp/hidden-home/.claude"}',
        code: 0,
      };
    },
  });
  const collected = collect();
  try {
    await adapter.start({
      cwd: dir,
      sessionDir: dir,
      developerInstructions: "Be brief",
      env: {
        PATH: "",
        HOME: dir,
        CLAUDE_PATH: binary,
        PITEX_TEST: "1",
        ANTHROPIC_API_KEY: "user-supplied",
        ANTHROPIC_AUTH_TOKEN: "user-supplied",
        CLAUDE_CODE_OAUTH_TOKEN: "user-supplied",
        ANTHROPIC_BASE_URL: "https://gateway.example/anthropic",
      },
    });
    assert.equal(adapter.unavailableReason, null);
    assert.equal(existsSync(marker), false);
    await waitUntil(() => opened[0]?.query.modesSet.includes("default") === true, "permission mode reset");
    const first = opened[0].options;
    assert.equal(first.permissionMode, "default");
    assert.deepEqual(first.settingSources, []);
    assert.equal(first.cwd, dir);
    assert.equal(first.pathToClaudeCodeExecutable, binary);
    assert.equal(first.resume, undefined);
    assert.equal(first.strictMcpConfig, true);
    assert.deepEqual(first.mcpServers, {});
    assert.equal(first.persistSession, true);
    assert.equal(first.includePartialMessages, true);
    assert.equal(first.systemPrompt.preset, "claude_code");
    assert.equal(first.systemPrompt.append, "Be brief");
    assert.equal(first.env.PITEX_TEST, "1");
    assert.equal(first.env.HOME, dir);
    assert.equal(Object.hasOwn(first.env, "ANTHROPIC_API_KEY"), false);
    assert.equal(Object.hasOwn(first.env, "ANTHROPIC_AUTH_TOKEN"), false);
    assert.equal(Object.hasOwn(first.env, "CLAUDE_CODE_OAUTH_TOKEN"), false);
    assert.equal(Object.hasOwn(first.env, "ANTHROPIC_BASE_URL"), false);
    assert.equal("allowedTools" in first, false);
    assert.equal("additionalDirectories" in first, false);

    await adapter.handleCommand({ type: "get_available_models", id: "models" }, collected.host);
    const models = (response(collected.responses, "get_available_models").data as { models: { id: string; provider: string }[] }).models;
    assert.ok(models.some((model) => model.id === "claude-sonnet-4-5" && model.provider === "claude"));
    await adapter.handleCommand({ type: "get_state", id: "state" }, collected.host);
    await adapter.handleCommand({ type: "get_available_thinking_levels", id: "levels" }, collected.host);
    const level = (response(collected.responses, "get_state").data as { thinkingLevel: string }).thinkingLevel;
    const levels = (response(collected.responses, "get_available_thinking_levels").data as { levels: string[] }).levels;
    assert.equal(level, "medium");
    assert.ok(levels.includes(level));
    assert.equal(new Set(levels).size, levels.length);

    await adapter.handleCommand({ type: "set_model", id: "model", modelId: "claude-opus-4-7" }, collected.host);
    assert.deepEqual(opened[0].query.modelsSet, ["claude-opus-4-7"]);
    await adapter.handleCommand({ type: "prompt", id: "p1", message: "Fix the intro" }, collected.host);
    assert.equal(response(collected.responses, "prompt").success, true);
    await waitUntil(() => collected.events.some((event) => event.type === "approval_request"), "approval");
    const approval = collected.events.find((event) => event.type === "approval_request");
    assert.equal(approval?.kind, "command");
    assert.equal(approval?.title, "Run command");
    assert.equal(approval?.detail, "latexmk main.tex");
    await adapter.handleCommand(
      { type: "approval_response", id: String(approval?.id), decision: "always_allow" },
      collected.host
    );
    await waitUntil(() => collected.events.some((event) => event.type === "agent_end"), "agent end");
    assert.deepEqual(messageDeltas(collected.events, "text_delta"), ["Hello"]);
    assert.deepEqual(messageDeltas(collected.events, "thinking_delta"), ["plan"]);
    assert.equal(collected.events.filter((event) => event.type === "approval_request").length, 1);
    assert.equal(collected.events.filter((event) => event.type === "tool_execution_start").length, 1);
    assert.equal(collected.events.filter((event) => event.type === "tool_execution_end").length, 1);
    const allowed = opened[0].query.permissions[0];
    const repeated = opened[0].query.permissions[1];
    assert.equal(allowed.behavior, "allow");
    assert.equal(allowed.decisionClassification, "user_permanent");
    const updates = allowed.updatedPermissions as { destination?: string; rules?: { ruleContent?: string }[] }[];
    assert.equal(updates[0].destination, "session");
    assert.equal(updates[0].rules?.[0].ruleContent, "latexmk main.tex");
    assert.equal(JSON.stringify(updates).includes("localSettings"), false);
    assert.equal(repeated.behavior, "allow");
    assert.equal(repeated.decisionClassification, "user_permanent");
    assert.equal(repeated.updatedPermissions, undefined);
    await adapter.handleCommand({ type: "get_messages", id: "msgs" }, collected.host);
    const messages = (response(collected.responses, "get_messages").data as { messages: { role: string }[] }).messages;
    assert.deepEqual(messages.map((message) => message.role), ["user", "assistant", "toolResult"]);
    await adapter.handleCommand({ type: "get_session_stats", id: "stats" }, collected.host);
    const stats = response(collected.responses, "get_session_stats").data as { tokens: { total: number }; cost: number };
    assert.equal(stats.tokens.total, 7);
    assert.equal(stats.cost, 0.01);
    const index = readFileSync(join(dir, "codex-session-index.json"), "utf8");
    assert.match(index, /sess-1/);
    assert.equal(index.includes("must-not-leak"), false);

    const session = claudeSession(dir);
    await adapter.handleCommand({ type: "switch_session", id: "sw", sessionPath: session }, collected.host);
    assert.equal(response(collected.responses, "switch_session").success, true);
    assert.equal(opened[1].options.resume, "sess-1");
    assert.equal(opened[1].options.permissionMode, "default");
    assert.deepEqual(opened[1].options.settingSources, []);
    assert.equal(opened[1].options.cwd, dir);
    assert.equal(opened[1].options.pathToClaudeCodeExecutable, binary);
    assert.equal(opened[1].options.model, "claude-opus-4-7");
    await adapter.handleCommand({ type: "prompt", id: "p2", message: "Again" }, collected.host);
    await waitUntil(
      () => collected.events.filter((event) => event.type === "approval_request").length >= 2,
      "second approval"
    );
    const again = collected.events.filter((event) => event.type === "approval_request")[1];
    await adapter.handleCommand({ type: "approval_response", id: String(again.id), decision: "deny" }, collected.host);
    await adapter.handleCommand({ type: "new_session", id: "new" }, collected.host);
    assert.equal(response(collected.responses, "new_session").success, true);
    assert.equal(opened[2].options.resume, undefined);
    assert.equal(opened[2].options.permissionMode, "default");
    assert.deepEqual(opened[2].options.settingSources, []);
    assert.equal(opened[2].options.cwd, dir);

    const blob = JSON.stringify({ events: collected.events, responses: collected.responses, permissions: opened.map((item) => item.query.permissions) });
    assert.equal(blob.includes("must-not-leak"), false);
    assert.equal(blob.includes("hidden-home"), false);
    assert.equal(blob.includes("configDirectory"), false);
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

test("a mutating Bash command and a file write wait for approval", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-claude-mutate-"));
  const binary = join(dir, "claude");
  writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
  const opened: Opened[] = [];
  const adapter = new ClaudeAdapter({
    query: scriptedQuery(opened, "mutate"),
    authStatus: async () => ({ stdout: '{"loggedIn":true}', code: 0 }),
  });
  const collected = collect();
  try {
    await adapter.start({
      cwd: dir,
      sessionDir: dir,
      env: { PATH: "", HOME: dir, CLAUDE_PATH: binary },
    });
    await adapter.handleCommand({ type: "prompt", id: "p", message: "touch pitex-ok" }, collected.host);
    await waitUntil(
      () => collected.events.some((event) => event.type === "approval_request" && event.id === "toolu_touch"),
      "touch approval"
    );
    assert.equal(opened[0].query.permissions.length, 0);
    const touch = collected.events.find((event) => event.id === "toolu_touch");
    assert.equal(touch?.kind, "command");
    assert.equal(touch?.title, "Run command");
    assert.equal(touch?.detail, "touch pitex-ok");
    await new Promise((resolve) => setTimeout(resolve, 40));
    assert.equal(opened[0].query.permissions.length, 0);
    assert.equal(collected.events.some((event) => event.type === "agent_end"), false);
    await adapter.handleCommand({ type: "approval_response", id: "toolu_touch", decision: "approve" }, collected.host);
    await waitUntil(
      () => collected.events.some((event) => event.type === "approval_request" && event.id === "toolu_write"),
      "write approval"
    );
    assert.equal(opened[0].query.permissions.length, 1);
    assert.equal(opened[0].query.permissions[0].behavior, "allow");
    const write = collected.events.find((event) => event.id === "toolu_write");
    assert.equal(write?.kind, "file");
    assert.equal(write?.title, "Edit files");
    assert.equal(write?.detail, "pitex-ok.txt");
    assert.equal(collected.events.some((event) => event.type === "agent_end"), false);
    await adapter.handleCommand({ type: "approval_response", id: "toolu_write", decision: "deny" }, collected.host);
    await waitUntil(() => collected.events.some((event) => event.type === "agent_end"), "turn end");
    assert.equal(opened[0].query.permissions[1].behavior, "deny");
    assert.equal(existsSync(join(dir, "pitex-ok.txt")), false);
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

interface ClaudeFakeMessage {
  type?: string;
  result?: string;
  request?: { subtype?: string; input?: { command?: string } };
}

function claudeFakeTurn(
  fake: string,
  prompt: string,
  drive?: (session: {
    messages: ClaudeFakeMessage[];
    send(value: unknown): void;
    waitFor(label: string, ready: (messages: ClaudeFakeMessage[]) => boolean): Promise<void>;
  }) => Promise<void>
): Promise<{ messages: ClaudeFakeMessage[] }> {
  const child = spawn(process.execPath, [fake], { stdio: ["pipe", "pipe", "pipe"] });
  const messages: ClaudeFakeMessage[] = [];
  let buffer = "";
  const session = {
    messages,
    send(value: unknown) {
      child.stdin.write(`${JSON.stringify(value)}\n`);
    },
    waitFor(label: string, ready: (rows: ClaudeFakeMessage[]) => boolean) {
      return new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => {
          clearInterval(interval);
          reject(new Error(`timed out waiting for ${label}: ${JSON.stringify(messages)}`));
        }, 2500);
        const interval = setInterval(() => {
          if (!ready(messages)) return;
          clearTimeout(timer);
          clearInterval(interval);
          resolve();
        }, 10);
      });
    },
  };
  child.stdout.on("data", (chunk: Buffer) => {
    buffer += chunk.toString("utf8");
    const lines = buffer.split("\n");
    buffer = lines.pop() ?? "";
    for (const line of lines) {
      if (!line.trim()) continue;
      messages.push(JSON.parse(line) as ClaudeFakeMessage);
    }
  });
  session.send({
    type: "user",
    message: { role: "user", content: [{ type: "text", text: prompt }] },
  });
  const done = new Promise<{ messages: ClaudeFakeMessage[] }>((resolve, reject) => {
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`timed out: ${JSON.stringify(messages)}`));
    }, 3000);
    const finish = () => {
      clearTimeout(timer);
      child.kill();
      resolve({ messages });
    };
    if (!drive) {
      const interval = setInterval(() => {
        if (messages.some((message) => message.type === "result" || message.request?.subtype === "can_use_tool")) {
          clearInterval(interval);
          finish();
        }
      }, 10);
      return;
    }
    drive(session).then(() => {
      const interval = setInterval(() => {
        if (!messages.some((message) => message.type === "result")) return;
        clearInterval(interval);
        finish();
      }, 10);
    }, (error: Error) => {
      clearTimeout(timer);
      child.kill();
      reject(error);
    });
  });
  return done.finally(() => {
    child.kill();
  });
}

test("the Claude fake runs echo without asking and waits before touch", async () => {
  const fake = fileURLToPath(new URL("../../../../.cursor/skills/verify-pitex/helpers/fakes/claude.mjs", import.meta.url));
  const readonly = await claudeFakeTurn(fake, "READONLY echo pitex-ok");
  assert.equal(
    readonly.messages.some((message) => message.request?.subtype === "can_use_tool"),
    false
  );
  assert.equal(
    readonly.messages.some((message) => message.type === "result" && message.result === "pitex-ok"),
    true
  );

  const touch = await claudeFakeTurn(fake, "TOUCH pitex-ok", async (session) => {
    await session.waitFor(
      "touch permission",
      (messages) =>
        messages.some((message) => message.request?.subtype === "can_use_tool" && message.request?.input?.command === "touch pitex-ok")
    );
    await new Promise((resolve) => setTimeout(resolve, 40));
    assert.equal(session.messages.some((message) => message.type === "result"), false);
    session.send({
      type: "control_response",
      response: { subtype: "success", request_id: "perm-touch", response: { behavior: "deny" } },
    });
  });
  assert.equal(touch.messages.some((message) => message.type === "result"), true);
});

test("abort interrupts the claude turn and denies the pending tool", async () => {
  const dir = mkdtempSync(join(tmpdir(), "pitex-claude-abort-"));
  const binary = join(dir, "claude");
  writeFileSync(binary, "#!/bin/sh\nexit 0\n", { mode: 0o755 });
  const opened: Opened[] = [];
  const adapter = new ClaudeAdapter({
    query: scriptedQuery(opened, "hold"),
    authStatus: async () => ({ stdout: '{"loggedIn":true}', code: 0 }),
  });
  const collected = collect();
  try {
    await adapter.start({
      cwd: dir,
      sessionDir: dir,
      env: { PATH: "", HOME: dir, CLAUDE_PATH: binary },
    });
    await adapter.handleCommand({ type: "prompt", id: "p", message: "Stop this" }, collected.host);
    await waitUntil(() => collected.events.some((event) => event.type === "approval_request"), "approval");
    await adapter.handleCommand({ type: "abort", id: "stop" }, collected.host);
    await waitUntil(() => opened[0].query.permissions.length > 0, "denied tool");
    assert.equal(response(collected.responses, "abort").success, true);
    assert.equal(opened[0].query.interruptCount, 1);
    assert.equal(opened[0].query.permissions[0].behavior, "deny");
    assert.equal(opened[0].query.permissions[0].interrupt, true);
    assert.equal(opened[0].query.permissions[0].message, "The user denied this action.");
    assert.equal(collected.events.filter((event) => event.type === "approval_request").length, 1);
  } finally {
    await adapter.stop();
    rmSync(dir, { recursive: true, force: true });
  }
});

class ScriptedQuery implements ClaudeQueryHandle {
  permissions: Json[] = [];
  modelsSet: string[] = [];
  modesSet: string[] = [];
  interruptCount = 0;
  private queue: unknown[] = [];
  private waiter: ((result: IteratorResult<unknown>) => void) | null = null;
  private ended = false;

  push(message: unknown): void {
    if (this.ended) return;
    if (this.waiter) {
      const waiter = this.waiter;
      this.waiter = null;
      waiter({ value: message, done: false });
      return;
    }
    this.queue.push(message);
  }

  next(): Promise<IteratorResult<unknown>> {
    const queued = this.queue.shift();
    if (queued !== undefined) return Promise.resolve({ value: queued, done: false });
    if (this.ended) return Promise.resolve({ value: undefined, done: true });
    return new Promise((resolve) => {
      this.waiter = resolve;
    });
  }

  [Symbol.asyncIterator](): AsyncIterator<unknown> {
    return this;
  }

  async interrupt(): Promise<void> {
    this.interruptCount += 1;
  }

  async setModel(model?: string): Promise<void> {
    if (model) this.modelsSet.push(model);
  }

  async setPermissionMode(mode: string): Promise<void> {
    this.modesSet.push(mode);
  }

  async supportedModels() {
    return [
      {
        value: "claude-sonnet-4-5",
        displayName: "Sonnet",
        description: "balanced",
        supportedEffortLevels: ["low", "medium", "high"],
      },
    ];
  }

  close(): void {
    if (this.ended) return;
    this.ended = true;
    const waiter = this.waiter;
    this.waiter = null;
    waiter?.({ value: undefined, done: true });
  }
}

interface Opened {
  options: ClaudeQueryOptions;
  query: ScriptedQuery;
}

function scriptedQuery(opened: Opened[], mode: "script" | "hold" | "mutate"): ClaudeQueryFn {
  return ({ prompt, options }) => {
    const query = new ScriptedQuery();
    opened.push({ options, query });
    query.push({
      type: "system",
      subtype: "init",
      session_id: "sess-1",
      model: "claude-sonnet-4-5",
      permissionMode: "acceptEdits",
    });
    void runTurns(prompt, options, query, mode);
    return query;
  };
}

async function runTurns(
  prompt: AsyncIterable<Json>,
  options: ClaudeQueryOptions,
  query: ScriptedQuery,
  mode: "script" | "hold" | "mutate"
): Promise<void> {
  try {
    for await (const _message of prompt) {
      if (mode === "hold") {
        query.permissions.push(await options.canUseTool("Bash", { command: "latexmk main.tex" }, toolOptions("toolu_abort")));
        continue;
      }
      if (mode === "mutate") {
        const touch = await options.canUseTool(
          "Bash",
          { command: "touch pitex-ok", description: "Create a marker file" },
          toolOptions("toolu_touch", undefined, "Bash", "Create a marker file")
        );
        query.permissions.push(touch);
        if (touch.behavior !== "allow") {
          query.push({ type: "result", subtype: "success", result: "denied", session_id: "sess-1" });
          continue;
        }
        const written = await options.canUseTool(
          "Write",
          { file_path: "pitex-ok.txt", content: "pitex-ok\n" },
          toolOptions("toolu_write", undefined, "Write", "Write the marker")
        );
        query.permissions.push(written);
        query.push({
          type: "result",
          subtype: "success",
          result: written.behavior === "allow" ? "wrote" : "denied",
          session_id: "sess-1",
        });
        continue;
      }
      query.push(streamDelta(0, { type: "thinking_delta", thinking: "plan" }));
      query.push(streamDelta(1, { type: "text_delta", text: "Hello" }));
      const first = await options.canUseTool(
        "Bash",
        { command: "latexmk main.tex" },
        toolOptions("toolu_1", LOCAL_SUGGESTION)
      );
      query.permissions.push(first);
      if (first.behavior === "deny") continue;
      query.permissions.push(
        await options.canUseTool("Bash", { command: "latexmk main.tex" }, toolOptions("toolu_2"))
      );
      query.push({
        type: "assistant",
        message: {
          role: "assistant",
          content: [
            { type: "thinking", thinking: "plan" },
            { type: "text", text: "Hello" },
            { type: "tool_use", id: "toolu_1", name: "Bash", input: { command: "latexmk main.tex" } },
          ],
        },
      });
      query.push({
        type: "user",
        message: {
          role: "user",
          content: [{ type: "tool_result", tool_use_id: "toolu_1", content: "ok" }],
        },
      });
      query.push({
        type: "result",
        subtype: "success",
        result: "Hello",
        session_id: "sess-1",
        usage: { input_tokens: 3, output_tokens: 4 },
        total_cost_usd: 0.01,
      });
    }
  } catch {
    // Closing the prompt iterator ends the turn driver.
  }
}

async function drain(prompt: AsyncIterable<unknown>): Promise<void> {
  try {
    for await (const _message of prompt) {
      // Tests that never prompt still have to release the iterator on stop.
    }
  } catch {
    // Closing the prompt iterator ends the drain.
  }
}

function streamDelta(index: number, delta: Json): Json {
  return { type: "stream_event", event: { type: "content_block_delta", index, delta } };
}

function bareTool(id: string): CanUseToolOptions {
  return { signal: new AbortController().signal, toolUseID: id };
}

function toolOptions(
  id: string,
  suggestions?: unknown,
  displayName = "Bash",
  description = "Compile the document"
): CanUseToolOptions {
  return {
    signal: new AbortController().signal,
    toolUseID: id,
    suggestions,
    displayName,
    description,
  };
}

function messageDeltas(events: Record<string, unknown>[], kind: string): string[] {
  return events.flatMap((event) => {
    if (event.type !== "message_update") return [];
    const inner = event.assistantMessageEvent as { type?: string; delta?: string } | undefined;
    if (!inner || inner.type !== kind || typeof inner.delta !== "string") return [];
    return [inner.delta];
  });
}

function claudeSession(dir: string): string {
  const name = readdirSync(dir).find((entry) => entry.startsWith("claude-") && entry.endsWith(".jsonl"));
  if (!name) throw new Error(`no Claude session in ${readdirSync(dir).join(", ")}`);
  return join(dir, name);
}

function collect(): { host: AdapterHost; events: Record<string, unknown>[]; responses: Record<string, unknown>[] } {
  const events: Record<string, unknown>[] = [];
  const responses: Record<string, unknown>[] = [];
  return {
    events,
    responses,
    host: {
      emit(event) {
        events.push(event);
      },
      respond(command, success, data, error, id) {
        responses.push({ command, success, data, error, id });
      },
    },
  };
}

function response(responses: Record<string, unknown>[], command: string): Record<string, unknown> {
  const found = [...responses].reverse().find((item) => item.command === command);
  if (!found) throw new Error(`no ${command} response in ${responses.map((item) => item.command).join(", ")}`);
  return found;
}

async function waitUntil(predicate: () => boolean, label: string): Promise<void> {
  const deadline = Date.now() + 3000;
  while (Date.now() < deadline) {
    if (predicate()) return;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error(`timed out waiting for ${label}`);
}
