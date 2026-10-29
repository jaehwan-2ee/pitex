// Newline-delimited JSON-RPC 2.0 over a child process's stdio.
// Inbound token-shaped fields are dropped before any caller sees them.
// Nothing in this module reads Codex credential files.

import { type ChildProcessWithoutNullStreams } from "node:child_process";

const SECRET_KEYS = new Set([
  "authToken",
  "accessToken",
  "refreshToken",
  "idToken",
  "chatgptAuthTokens",
  "apiKey",
  "api_key",
]);

export function redactSecrets(value: unknown): unknown {
  if (Array.isArray(value)) return value.map((item) => redactSecrets(item));
  if (!value || typeof value !== "object") return value;
  const copy: Record<string, unknown> = {};
  for (const [key, inner] of Object.entries(value as Record<string, unknown>)) {
    if (SECRET_KEYS.has(key)) continue;
    copy[key] = redactSecrets(inner);
  }
  return copy;
}

interface Pending {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timer: NodeJS.Timeout;
}

export class LineJsonRpc {
  private nextId = 1;
  private buffer = "";
  private pending = new Map<number, Pending>();
  private closed = false;
  private readonly child: ChildProcessWithoutNullStreams;
  private readonly onNotification: (method: string, params: Record<string, unknown>) => void;
  private readonly onServerRequest: (
    method: string,
    id: string | number,
    params: Record<string, unknown>
  ) => void;
  private readonly onClose?: (message: string) => void;

  constructor(
    child: ChildProcessWithoutNullStreams,
    onNotification: (method: string, params: Record<string, unknown>) => void,
    onServerRequest: (
      method: string,
      id: string | number,
      params: Record<string, unknown>
    ) => void,
    onClose?: (message: string) => void
  ) {
    this.child = child;
    this.onNotification = onNotification;
    this.onServerRequest = onServerRequest;
    this.onClose = onClose;
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => this.ingest(chunk));
    child.on("exit", () => {
      this.failAll("Codex CLI exited.");
      this.onClose?.("Codex CLI exited.");
    });
    child.on("error", (error) => {
      this.failAll(error.message);
      this.onClose?.(error.message);
    });
    // EPIPE after Codex exits is an 'error' on stdin. Without a listener
    // that event crashes the bridge instead of telling the panel.
    child.stdin.on("error", (error: NodeJS.ErrnoException) => {
      const message =
        error.code === "EPIPE"
          ? "Codex CLI is unavailable."
          : `Codex CLI is unavailable (${error.message}).`;
      this.failAll(message);
      this.onClose?.(message);
    });
  }

  request(method: string, params: unknown, timeoutMs = 30_000): Promise<unknown> {
    if (this.closed) return Promise.reject(new Error("Codex CLI is not running."));
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Codex CLI timed out waiting for ${method}.`));
      }, timeoutMs);
      this.pending.set(id, { resolve, reject, timer });
      this.send({ jsonrpc: "2.0", id, method, params: params ?? {} });
    });
  }

  notify(method: string, params: unknown = {}): void {
    this.send({ jsonrpc: "2.0", method, params });
  }

  respond(id: string | number, result: unknown): void {
    this.send({ jsonrpc: "2.0", id, result });
  }

  fail(id: string | number, message: string): void {
    this.send({ jsonrpc: "2.0", id, error: { code: -32000, message } });
  }

  close(): void {
    this.closed = true;
    this.failAll("Codex CLI stopped.");
  }

  private send(message: unknown): void {
    if (this.closed) return;
    this.child.stdin.write(`${JSON.stringify(message)}\n`);
  }

  private ingest(chunk: string): void {
    this.buffer += chunk;
    while (true) {
      const newline = this.buffer.indexOf("\n");
      if (newline < 0) return;
      const line = this.buffer.slice(0, newline).replace(/\r$/, "");
      this.buffer = this.buffer.slice(newline + 1);
      if (!line.trim()) continue;
      let parsed: unknown;
      try {
        parsed = JSON.parse(line);
      } catch {
        continue;
      }
      const message = redactSecrets(parsed) as Record<string, unknown>;
      this.dispatch(message);
    }
  }

  private dispatch(message: Record<string, unknown>): void {
    const method = typeof message.method === "string" ? message.method : "";
    const params =
      message.params && typeof message.params === "object"
        ? (message.params as Record<string, unknown>)
        : {};
    const hasId = Object.prototype.hasOwnProperty.call(message, "id");
    if (method && hasId) {
      const id = message.id;
      if (typeof id === "string" || typeof id === "number") this.onServerRequest(method, id, params);
      return;
    }
    if (hasId && (Object.prototype.hasOwnProperty.call(message, "result") || message.error)) {
      const id = typeof message.id === "number" ? message.id : Number(message.id);
      const waiter = this.pending.get(id);
      if (!waiter) return;
      clearTimeout(waiter.timer);
      this.pending.delete(id);
      if (message.error && typeof message.error === "object") {
        const error = message.error as { message?: string };
        waiter.reject(new Error(error.message || "Codex CLI request failed."));
        return;
      }
      waiter.resolve(message.result);
      return;
    }
    if (method) this.onNotification(method, params);
  }

  private failAll(message: string): void {
    this.closed = true;
    for (const [id, waiter] of this.pending) {
      clearTimeout(waiter.timer);
      waiter.reject(new Error(message));
      this.pending.delete(id);
    }
  }
}
