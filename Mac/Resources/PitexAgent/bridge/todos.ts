// The agent's todo list (`todo_update`), from Claude Code's TodoWrite and
// TaskCreate/TaskUpdate/TaskList, an ACP `plan` update, or Codex's
// `turn/plan/updated`. Every event carries the whole list.
//
// The Claude task folding follows Paseo
// (packages/server/src/server/agent/providers/claude/task-state.ts).

export type Json = Record<string, unknown>;

export type TodoStatus = "pending" | "in_progress" | "completed";

export interface TodoItem {
  id?: string;
  text: string;
  status: TodoStatus;
  activeForm?: string;
}

/** Claude tools whose rows the checklist replaces. A failed call still shows its card. */
export const TODO_TOOLS = new Set(["TodoWrite", "TaskCreate", "TaskUpdate", "TaskList"]);

const ITEM_CAP = 100;
const TEXT_CAP = 500;

function asObject(value: unknown): Json | null {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Json) : null;
}

function text(value: unknown): string | undefined {
  if (typeof value === "number" && Number.isFinite(value)) return String(value);
  return typeof value === "string" && value.trim() ? value.trim().slice(0, TEXT_CAP) : undefined;
}

function status(value: unknown): TodoStatus | "deleted" {
  if (value === "completed" || value === "in_progress" || value === "deleted") return value;
  if (value === "inProgress") return "in_progress";
  return "pending";
}

/** One row from any source: Claude `content`/`subject`, ACP `content`, Codex `step`. */
function toItem(value: unknown): TodoItem | null {
  const row = asObject(value);
  if (!row) return null;
  const label = text(row.content) ?? text(row.subject) ?? text(row.step) ?? text(row.text);
  const state = status(row.status);
  if (!label || state === "deleted") return null;
  const id = text(row.id) ?? text(row.taskId);
  const activeForm = text(row.activeForm);
  return { ...(id ? { id } : {}), text: label, status: state, ...(activeForm ? { activeForm } : {}) };
}

/** A list of rows in any of the source shapes; rows without text are dropped. */
export function todoItems(value: unknown): TodoItem[] {
  if (!Array.isArray(value)) return [];
  return value
    .map(toItem)
    .filter((item): item is TodoItem => item !== null)
    .slice(0, ITEM_CAP);
}

/** Valid items from a stored or replayed snapshot, or null when there is none. */
export function storedTodos(value: unknown): TodoItem[] | null {
  return Array.isArray(value) ? todoItems(value) : null;
}

export function sameTodos(a: TodoItem[] | null, b: TodoItem[] | null): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * Fold one finished Claude todo tool call into the current list.
 * TodoWrite replaces it from its input; TaskCreate needs the new id from the structured
 * result or its text (`output`); TaskList replaces it from the result.
 * Returns null when the call does not change the list.
 */
export function foldClaudeTodo(
  current: TodoItem[] | null,
  name: string,
  input: Json,
  result: Json | null,
  output = ""
): TodoItem[] | null {
  if (name === "TodoWrite") return Array.isArray(input.todos) ? todoItems(input.todos) : null;
  if (result?.success === false) return null;
  const list = current ?? [];
  if (name === "TaskCreate") {
    const task = asObject(result?.task);
    // Without the structured result (several results in one message), the result text names the id.
    const id = text(task?.id) ?? text(result?.taskId) ?? /\bTask #(\d+) created\b/.exec(output)?.[1];
    const label = text(task?.subject) ?? text(input.subject);
    if (!id || !label) return null;
    const activeForm = text(input.activeForm);
    const item: TodoItem = { id, text: label, status: "pending", ...(activeForm ? { activeForm } : {}) };
    return [...list.filter((row) => row.id !== id), item].slice(0, ITEM_CAP);
  }
  if (name === "TaskUpdate") {
    const id = text(input.taskId) ?? text(result?.taskId);
    const index = list.findIndex((row) => row.id === id);
    if (!id || index < 0) return null;
    const requested = input.status ?? asObject(result?.statusChange)?.to;
    const next = requested === undefined ? list[index].status : status(requested);
    if (next === "deleted") return list.filter((row) => row.id !== id);
    const label = text(input.subject);
    const activeForm = text(input.activeForm);
    const updated: TodoItem = {
      ...list[index],
      ...(label ? { text: label } : {}),
      ...(activeForm ? { activeForm } : {}),
      status: next,
    };
    return list.map((row, at) => (at === index ? updated : row));
  }
  if (name === "TaskList") {
    return Array.isArray(result?.tasks) ? todoItems(result.tasks).filter((item) => item.id) : null;
  }
  return null;
}

/** The checklist rows of an ACP `plan` session update. */
export function todosFromAcpPlan(update: Json): TodoItem[] {
  return todoItems(update.entries);
}

/** The checklist rows of a Codex `turn/plan/updated` notification. */
export function todosFromCodexPlan(params: Json): TodoItem[] {
  return todoItems(params.plan);
}

/**
 * A replayed pi transcript without the todo tool rows the checklist replaces.
 * A failed call keeps its tool call and result so the error stays visible.
 */
export function withoutTodoTools(messages: Json[]): Json[] {
  const hidden = new Set<string>();
  const failed = new Set<string>();
  for (const message of messages) {
    if (message.role === "toolResult" && message.isError === true && typeof message.toolCallId === "string") {
      failed.add(message.toolCallId);
    }
    if (message.role !== "assistant" || !Array.isArray(message.content)) continue;
    for (const block of message.content) {
      const row = asObject(block);
      if (row?.type === "toolCall" && typeof row.id === "string" && TODO_TOOLS.has(String(row.name))) hidden.add(row.id);
    }
  }
  for (const id of failed) hidden.delete(id);
  if (hidden.size === 0) return messages;
  const out: Json[] = [];
  for (const message of messages) {
    if (message.role === "toolResult" && hidden.has(String(message.toolCallId))) continue;
    if (message.role === "assistant" && Array.isArray(message.content)) {
      const content = message.content.filter((block) => {
        const row = asObject(block);
        return !(row?.type === "toolCall" && hidden.has(String(row.id)));
      });
      if (content.length === 0) continue;
      if (content.length !== message.content.length) {
        out.push({ ...message, content });
        continue;
      }
    }
    out.push(message);
  }
  return out;
}
