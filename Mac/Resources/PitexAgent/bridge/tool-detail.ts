// Typed tool-call detail shared by every bridge backend.
//
// Additive, optional fields on the pi RPC tool_execution_start/update/end
// events. Hosts that do not know them (and pi's own provider, which never
// sends them) keep rendering toolName/args/result as before.
//
// Merge rule for hosts (Paseo timeline-projection): a later event for the
// same toolCallId merges into the row; kind "other" never replaces a typed
// kind; the error text is kept only when the status is failed.

import { randomUUID } from "node:crypto";

export type ToolKind = "shell" | "read" | "edit" | "write" | "search" | "fetch" | "sub_agent" | "think" | "other";
export type ToolStatus = "running" | "completed" | "failed" | "canceled";

export interface ToolDetail {
  kind: ToolKind;
  path?: string;
  command?: string;
  cwd?: string;
  exitCode?: number;
  oldText?: string;
  newText?: string;
  /** Unified diff hunks (no ---/+++ header), produced by the bridge. */
  diff?: string;
  query?: string;
  url?: string;
  numMatches?: number;
  label?: string;
  startLine?: number;
  endLine?: number;
  /** sub_agent: the agent type (Claude's `subagent_type`) and the task it was given. */
  subAgentType?: string;
  description?: string;
  /** sub_agent: tool calls the subagent has started so far (live count). */
  toolUses?: number;
}

type Json = Record<string, unknown>;

/** Cap for oldText/newText on the wire; the diff carries the change. */
export const TEXT_CAP = 4_000;
/** Cap for the diff text; hosts cap again with the same number. */
export const DIFF_CAP = 12_000;
/** LCS table size limit (cells) after trimming the common prefix and suffix. */
const MAX_CELLS = 4_000_000;
/** A line starting with "\" is a meta line for both host diff parsers. */
export const DIFF_TOO_LARGE = "\\ diff too large to show";
const DIFF_TRUNCATED = "\\ diff truncated";

function bounded(text: string, cap: number): string {
  return text.length <= cap ? text : text.slice(0, cap);
}

function splitLines(text: string): string[] {
  if (!text) return [];
  const lines = text.split("\n");
  if (lines[lines.length - 1] === "") lines.pop();
  return lines;
}

/** Cut a long diff at a line boundary and say so. */
export function capDiff(diff: string): string {
  if (diff.length <= DIFF_CAP) return diff;
  const cut = diff.lastIndexOf("\n", DIFF_CAP);
  return `${diff.slice(0, cut > 0 ? cut : DIFF_CAP)}\n${DIFF_TRUNCATED}`;
}

/**
 * Line diff of two texts as unified-diff hunks with `context` lines around
 * each change. LCS over the lines left after trimming the common prefix and
 * suffix; returns DIFF_TOO_LARGE when that table would exceed MAX_CELLS.
 * Identical texts give "".
 */
export function unifiedDiff(oldText: string, newText: string, context = 3): string {
  const a = splitLines(oldText);
  const b = splitLines(newText);
  let pre = 0;
  while (pre < a.length && pre < b.length && a[pre] === b[pre]) pre++;
  let suf = 0;
  while (suf < a.length - pre && suf < b.length - pre && a[a.length - 1 - suf] === b[b.length - 1 - suf]) suf++;
  const n = a.length - pre - suf;
  const m = b.length - pre - suf;
  if (n === 0 && m === 0) return "";
  if ((n + 1) * (m + 1) > MAX_CELLS) return DIFF_TOO_LARGE;

  // lcs[i*(m+1)+j] = LCS length of a[pre+i..] and b[pre+j..] (middle part only).
  const width = m + 1;
  const lcs = new Uint32Array((n + 1) * width);
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      lcs[i * width + j] =
        a[pre + i] === b[pre + j]
          ? lcs[(i + 1) * width + j + 1] + 1
          : Math.max(lcs[(i + 1) * width + j], lcs[i * width + j + 1]);
    }
  }
  const ops: { op: " " | "-" | "+"; text: string }[] = [];
  for (let k = 0; k < pre; k++) ops.push({ op: " ", text: a[k] });
  let i = 0;
  let j = 0;
  while (i < n || j < m) {
    if (i < n && j < m && a[pre + i] === b[pre + j]) {
      ops.push({ op: " ", text: a[pre + i] });
      i++;
      j++;
    } else if (j >= m || (i < n && lcs[(i + 1) * width + j] >= lcs[i * width + j + 1])) {
      ops.push({ op: "-", text: a[pre + i] });
      i++;
    } else {
      ops.push({ op: "+", text: b[pre + j] });
      j++;
    }
  }
  for (let k = a.length - suf; k < a.length; k++) ops.push({ op: " ", text: a[k] });

  const out: string[] = [];
  let size = 0;
  let index = 0;
  // Stop once past DIFF_CAP: capDiff cuts there anyway, so a huge change costs no more.
  while (index < ops.length && size <= DIFF_CAP) {
    while (index < ops.length && ops[index].op === " ") index++;
    if (index >= ops.length) break;
    const start = Math.max(0, index - context);
    // Extend the hunk while the next change is within 2*context lines.
    let end = index;
    let quiet = 0;
    for (let k = index; k < ops.length; k++) {
      if (ops[k].op === " ") {
        quiet++;
        if (quiet > 2 * context) break;
      } else {
        quiet = 0;
        end = k;
      }
    }
    const stop = Math.min(ops.length, end + 1 + context);
    let oldBefore = 0;
    let newBefore = 0;
    for (let k = 0; k < start; k++) {
      if (ops[k].op !== "+") oldBefore++;
      if (ops[k].op !== "-") newBefore++;
    }
    let oldCount = 0;
    let newCount = 0;
    const headerAt = out.push("") - 1;
    for (let k = start; k < stop; k++) {
      if (ops[k].op !== "+") oldCount++;
      if (ops[k].op !== "-") newCount++;
      const line = `${ops[k].op}${ops[k].text}`;
      out.push(line); // one at a time: spreading a huge hunk overflows the call stack
      size += line.length + 1;
    }
    const oldStart = oldCount ? oldBefore + 1 : oldBefore;
    const newStart = newCount ? newBefore + 1 : newBefore;
    out[headerAt] = `@@ -${oldStart},${oldCount} +${newStart},${newCount} @@`;
    size += out[headerAt].length + 1;
    index = stop;
  }
  return capDiff(out.join("\n"));
}

/** Claude Code's `structuredPatch` hunks (real file line numbers) as diff text. */
export function patchDiff(hunks: unknown): string {
  if (!Array.isArray(hunks) || hunks.length === 0) return "";
  const out: string[] = [];
  for (const hunk of hunks) {
    if (!hunk || typeof hunk !== "object") continue;
    const h = hunk as Json;
    const lines = Array.isArray(h.lines) ? h.lines.filter((line): line is string => typeof line === "string") : [];
    out.push(`@@ -${Number(h.oldStart) || 0},${Number(h.oldLines) || 0} +${Number(h.newStart) || 0},${Number(h.newLines) || 0} @@`);
    for (const line of lines) out.push(line);
  }
  return capDiff(out.join("\n"));
}

/** Drop empty and undefined fields and cap the long texts. */
export function cleanDetail(detail: ToolDetail): ToolDetail {
  const out: Json = {};
  for (const [key, value] of Object.entries(detail)) {
    if (value === undefined || value === null || value === "") continue;
    if (typeof value === "number" && !Number.isFinite(value)) continue;
    out[key] = value;
  }
  if (typeof out.oldText === "string") out.oldText = bounded(out.oldText, TEXT_CAP);
  if (typeof out.newText === "string") out.newText = bounded(out.newText, TEXT_CAP);
  if (typeof out.diff === "string") out.diff = capDiff(out.diff);
  if (typeof out.command === "string") out.command = bounded(out.command, TEXT_CAP);
  return out as unknown as ToolDetail;
}

export function newTurnId(): string {
  return randomUUID();
}

const TURN_EVENTS = new Set([
  "agent_start",
  "turn_end",
  "message_start",
  "message_update",
  "message_end",
  "tool_execution_start",
  "tool_execution_update",
  "tool_execution_end",
  "todo_update",
]);

/** Put `turnId` on the turn, message and tool events of one batch. */
export function stampTurn<T extends Json>(events: T[], turnId: string | null): T[] {
  if (!turnId) return events;
  for (const event of events) {
    if (typeof event.type === "string" && TURN_EVENTS.has(event.type) && event.turnId === undefined) {
      (event as Json).turnId = turnId;
    }
  }
  return events;
}
