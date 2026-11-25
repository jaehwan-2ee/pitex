// usage_update from Claude, ACP and Codex; agent_end outcome and error codes;
// one assistant row per text segment. Run through test.ts.

import assert from "node:assert/strict";
import test from "node:test";

import { errorCode, type AdapterHost } from "./adapter.ts";
import { closeTurn as acpCloseTurn, createStreamState as acpState, mapSessionUpdate } from "./acp-map.ts";
import { claudeModelName, createClaudeStreamState, mapClaudeMessage, messagesFromState, replayClaudeMessages } from "./claude-map.ts";
import { CodexAdapter } from "./codex-adapter.ts";
import { createStreamState as codexState, mapNotification } from "./codex-map.ts";

type Json = Record<string, unknown>;

const stream = (event: Json) => ({ type: "stream_event", event });
const textDelta = (index: number, text: string) => stream({ type: "content_block_delta", index, delta: { type: "text_delta", text } });
const of = (events: Json[], type: string) => events.filter((event) => event.type === type);

test("errorCode words signed-out, rate-limit, network and context-full provider errors", () => {
  assert.equal(errorCode("Invalid API key · Please run /login"), "signed_out");
  assert.equal(errorCode("OAuth token has expired. Please obtain a new token."), "signed_out");
  assert.equal(errorCode("Not logged in"), "signed_out");
  assert.equal(errorCode('API Error: 429 {"type":"rate_limit_error"}'), "rate_limited");
  assert.equal(errorCode("Claude AI usage limit reached|1760000000"), "rate_limited");
  assert.equal(errorCode("RESOURCE_EXHAUSTED: quota exceeded"), "rate_limited");
  assert.equal(errorCode("fetch failed: getaddrinfo ENOTFOUND api.anthropic.com"), "network");
  assert.equal(errorCode("Request timed out"), "network");
  assert.equal(errorCode("Prompt is too long: 210000 tokens > 200000 maximum"), "context_full");
  assert.equal(errorCode("Your credit balance is too low"), "other");
});

test("claude usage is pushed from stream usage and the result's context window and cost", () => {
  const state = createClaudeStreamState();
  const events = [
    ...mapClaudeMessage({ type: "system", subtype: "init", model: "claude-sonnet-4-5-20250929" }, state),
    ...mapClaudeMessage(
      stream({
        type: "message_start",
        message: { model: "claude-sonnet-4-5-20250929", usage: { input_tokens: 10, cache_creation_input_tokens: 990, cache_read_input_tokens: 75_000, output_tokens: 1 } },
      }),
      state
    ),
    ...mapClaudeMessage(textDelta(0, "Hi"), state),
    ...mapClaudeMessage(stream({ type: "message_delta", usage: { output_tokens: 500 } }), state),
    ...mapClaudeMessage(
      {
        type: "result",
        subtype: "success",
        is_error: false,
        result: "Hi",
        total_cost_usd: 0.42,
        usage: { input_tokens: 10, output_tokens: 500 },
        modelUsage: {
          "claude-haiku-4-5": { outputTokens: 3, contextWindow: 200_000 },
          "claude-sonnet-4-5-20250929": { outputTokens: 500, contextWindow: 1_000_000 },
        },
      },
      state
    ),
  ];
  const usage = of(events, "usage_update");
  assert.deepEqual(usage[0], {
    type: "usage_update",
    inputTokens: 1000,
    cachedInputTokens: 75_000,
    outputTokens: 1,
    contextUsed: 76_001,
    turnId: usage[0].turnId,
  });
  assert.equal(typeof usage[0].turnId, "string");
  assert.equal(usage[1].outputTokens, 500);
  assert.equal(usage[1].contextUsed, 76_500);
  const last = usage.at(-1) as Json;
  assert.equal(last.contextMax, 1_000_000, "modelUsage of the answering model, not a guessed window");
  assert.equal(last.costUsd, 0.42);
  assert.ok(events.indexOf(last) < events.findIndex((event) => event.type === "agent_end"));
  assert.equal(events.find((event) => event.type === "agent_end")?.outcome, "completed");
  assert.equal((events.find((event) => event.type === "message_start")?.message as Json).modelName, "Claude Sonnet 4.5");

  // A new chat on the same model knows its window from the first usage on.
  const next = createClaudeStreamState();
  next.model = "claude-sonnet-4-5-20250929";
  const first = of(mapClaudeMessage(stream({ type: "message_start", message: { usage: { input_tokens: 10, output_tokens: 1 } } }), next), "usage_update");
  assert.equal(first[0]?.contextMax, 1_000_000);
});

test("claude subagent messages leave the main turn's usage and model alone", () => {
  const state = createClaudeStreamState();
  state.model = "claude-opus-4-1";
  mapClaudeMessage({ type: "assistant", message: { model: "claude-opus-4-1", usage: { input_tokens: 120_000, output_tokens: 6 }, content: [] } }, state);
  const sub = mapClaudeMessage(
    {
      type: "assistant",
      parent_tool_use_id: "task-1",
      message: { model: "claude-haiku-4-5", usage: { input_tokens: 12_000, output_tokens: 40 }, content: [] },
    },
    state
  );
  assert.equal(of(sub, "usage_update").length, 0);
  assert.equal(state.usage.contextUsed, 120_006);
  assert.equal(state.model, "claude-opus-4-1");
});

test("claude text, a tool, then more text: message_end holds only the last segment", () => {
  const state = createClaudeStreamState();
  const tool = { type: "tool_use", id: "t1", name: "Read", input: { file_path: "main.tex" } };
  const events = [
    ...mapClaudeMessage(textDelta(0, "First."), state),
    ...mapClaudeMessage({ type: "assistant", message: { content: [{ type: "text", text: "First." }] } }, state),
    ...mapClaudeMessage({ type: "assistant", message: { content: [tool] } }, state),
    ...mapClaudeMessage({ type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t1", content: "x" }] } }, state),
    // No stream for the second message: the whole block arrives once.
    ...mapClaudeMessage({ type: "assistant", message: { content: [{ type: "text", text: "Second." }] } }, state),
    ...mapClaudeMessage({ type: "result", subtype: "success", is_error: false, result: "Second." }, state),
  ];
  const starts = of(events, "message_update").filter((event) => (event.assistantMessageEvent as Json).type === "text_start");
  assert.equal(starts.length, 2, "two text rows");
  const deltas = of(events, "message_update").map((event) => (event.assistantMessageEvent as Json).delta).filter(Boolean);
  assert.deepEqual(deltas, ["First.", "Second."]);
  const end = events.find((event) => event.type === "message_end")?.message as Json;
  assert.deepEqual(end.content, [{ type: "text", text: "Second." }]);
  const rows = messagesFromState(state).map((message) => (message.content as Json[]).map((block) => block.text ?? block.type).join());
  assert.deepEqual(rows, ["First.,toolCall", "x", "Second."]);
});

test("claude API errors end the turn failed with a code; Stop ends it canceled", () => {
  const state = createClaudeStreamState();
  const failed = [
    ...mapClaudeMessage(
      {
        type: "assistant",
        error: "rate_limit",
        message: { model: "<synthetic>", content: [{ type: "text", text: "API Error: Request rejected (429)" }] },
      },
      state
    ),
    ...mapClaudeMessage({ type: "result", subtype: "success", is_error: true, result: "API Error: Request rejected (429)" }, state),
  ];
  assert.equal(of(failed, "message_update").length, 0, "the error text is not an answer");
  const end = failed.find((event) => event.type === "agent_end") as Json;
  assert.deepEqual([end.outcome, end.code, end.error], ["failed", "rate_limited", "API Error: Request rejected (429)"]);

  const signedOut = mapClaudeMessage(
    { type: "result", subtype: "success", is_error: true, result: "Invalid API key · Please run /login" },
    state
  ).find((event) => event.type === "agent_end") as Json;
  assert.deepEqual([signedOut.outcome, signedOut.code], ["failed", "signed_out"]);

  mapClaudeMessage(textDelta(0, "Working"), state);
  state.canceled = true;
  const stopped = mapClaudeMessage(
    { type: "result", subtype: "error_during_execution", is_error: true, errors: ["[Request interrupted by user]"] },
    state
  ).find((event) => event.type === "agent_end") as Json;
  assert.equal(stopped.outcome, "canceled");
  assert.equal(stopped.code, undefined);
});

test("replayed claude answers carry the real model name", () => {
  assert.equal(claudeModelName("claude-opus-4-1-20250805"), "Claude Opus 4.1");
  assert.equal(claudeModelName("claude-sonnet-4-20250514"), "Claude Sonnet 4");
  assert.equal(claudeModelName("claude-sonnet-4-5-20250929[1m]"), "Claude Sonnet 4.5 (1M)");
  assert.equal(claudeModelName("gpt-5"), "gpt-5");
  const replay = replayClaudeMessages([
    { type: "user", message: { content: "hi" } },
    { type: "assistant", message: { model: "claude-opus-4-1-20250805", content: [{ type: "text", text: "Hello" }] } },
  ]);
  assert.equal(messagesFromState(replay)[1].modelName, "Claude Opus 4.1");
});

test("ACP turn ends: cancelled is canceled, a refusal is failed with its code", () => {
  const state = acpState();
  mapSessionUpdate({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: "x" } }, state);
  const canceled = acpCloseTurn(state, "cancelled").find((event) => event.type === "agent_end") as Json;
  assert.equal(canceled.outcome, "canceled");
  const signedOut = acpCloseTurn(state, "refusal", "Sign in to Antigravity", "signed_out").find((event) => event.type === "agent_end") as Json;
  assert.deepEqual([signedOut.outcome, signedOut.code, signedOut.error], ["failed", "signed_out", "Sign in to Antigravity"]);
  const network = acpCloseTurn(state, "refusal", "connect ECONNREFUSED 127.0.0.1").find((event) => event.type === "agent_end") as Json;
  assert.equal(network.code, "network");
  assert.equal((acpCloseTurn(state, "end_turn").find((event) => event.type === "agent_end") as Json).outcome, "completed");
});

test("codex token counts become usage_update and a failed turn carries its code", () => {
  const events: Json[] = [];
  const host: AdapterHost = { emit: (event) => events.push(event), respond: () => {} };
  const adapter = new CodexAdapter() as unknown as { host: AdapterHost; onNotification(method: string, params: Json): void };
  adapter.host = host;
  adapter.onNotification("thread/tokenUsage/updated", {
    tokenUsage: {
      total: { totalTokens: 9000, inputTokens: 8000, cachedInputTokens: 100, outputTokens: 1000, reasoningOutputTokens: 300 },
      last: { totalTokens: 4000, inputTokens: 3500, cachedInputTokens: 50, outputTokens: 500, reasoningOutputTokens: 120 },
      modelContextWindow: 272_000,
    },
  });
  const usage = events.find((event) => event.type === "usage_update") as Json;
  assert.deepEqual(
    [usage.inputTokens, usage.cachedInputTokens, usage.outputTokens, usage.thinkingTokens, usage.contextUsed, usage.contextMax],
    [3500, 50, 500, 120, 4000, 272_000]
  );

  const state = codexState();
  mapNotification("turn/started", { turn: { id: "t1" } }, state);
  const failed = mapNotification(
    "turn/completed",
    { turn: { id: "t1", status: "failed", error: { message: "context exceeded", codexErrorInfo: "contextWindowExceeded" } } },
    state
  ).find((event) => event.type === "agent_end") as Json;
  assert.deepEqual([failed.outcome, failed.code], ["failed", "context_full"]);
  mapNotification("turn/started", { turn: { id: "t2" } }, state);
  const stopped = mapNotification("turn/completed", { turn: { id: "t2", status: "interrupted" } }, state).find(
    (event) => event.type === "agent_end"
  ) as Json;
  assert.equal(stopped.outcome, "canceled");
});
