# Apple Foundation Models as a direct provider

A Mac with Apple Intelligence should offer Apple Foundation Models in the assistant picker, next to pi and Claude Code. The chat stays on the Mac. Pitex does not send it through pi.

## What is wrong with the current shape

Every shipped provider is a subprocess. pi is `pi --mode rpc`. Codex, Claude, and Grok are the bundled `pitex-agent-bridge` process with `PITEX_CLI_ADAPTER` set. The host stores that fact as a boolean, `uses_bridge`, and the launch plan has two branches. Anything that is not a bridge falls through to pi, including a stored id the shell does not know.

Apple Foundation Models is not a CLI. It is the `FoundationModels` framework, `SystemLanguageModel`, and `LanguageModelSession`, on macOS 26 or later, on Apple silicon, with Apple Intelligence turned on. A boolean that means "subprocess or other subprocess" cannot name that.

pi can still list an Apple model if the installed pi runtime does. That path is pi's catalog and pi's process. This design does not use it.

## The data shape

A provider is a transport.

```text
AgentTransport = Pi | CliBridge | OnDevice
```

| id | transport | Where it runs |
| --- | --- | --- |
| `pi` | `Pi` | `pi --mode rpc` |
| `codex`, `claude`, `grok` | `CliBridge` | `pitex-agent-bridge` |
| `apple-fm` | `OnDevice` | `LanguageModelSession` inside the Mac app |

Each row also has `selectable`. `SHOW_DIRECT_CODEX_AND_GROK` is false, so `codex` and `grok` stay in the registry for launch plans and read back as `pi`. The raw preference is left on disk until the user picks a row. `normalize_agent_provider` accepts every selectable id. That includes `apple-fm`. Unknown ids, including the retired `agy` id, read as `pi`.

The menu is one function, `visible_agent_providers()`. A row is listed when `selectable` is true and this shell can start the transport. The GTK shell cannot start `OnDevice`, so its menu is `pi` and `claude`. A stored `apple-fm` value stays stored. The menu shows pi until the user picks a row, and the caption is the unavailable sentence. `plan_launch("apple-fm")` returns that sentence and does not start pi.

The Mac picker uses `ShippedAgentProvider.selectableCases` and the same rule. `pi` and `claude` are listed. `apple-fm` is listed only when `AppleFMRuntime.availability()` is `.available`. Otherwise the picker keeps the pi and Claude Code rows and shows that availability sentence. A stored `apple-fm` id is not rewritten to `pi`.

The sentence is `AppleFMHost.unavailableMessage`:

> Apple Foundation Models run in the Pitex Mac app when Apple Intelligence is on. This build cannot start them.

## The session

The panel already consumes pi JSONL events. `AppleFMSession` implements the same `AgentSession` surface as `PiAgentProcess`: `isRunning`, `events`, `send`, and `terminate`. The coordinator holds `any AgentSession`. Ghost completion and commit-message generation stay on pi. They ignore `ai.agentProvider`.

The event vocabulary matches the bridge in `Mac/Resources/PitexAgent/bridge`. That bridge was adapted from T3 Code v0.1.4 (MIT). This provider does not copy a T3 module. No T3 Apple Foundation Models module was found in the tree.

`AppleFMContract` in AppPorts holds the behavior Linux can test. `FoundationModels` is imported only from `Mac/Sources/Features/AppleFMSession.swift`, behind `canImport` and `macOS 26`. AppPorts does not import it, so `swift test` on Linux still builds.

### Availability

`SystemLanguageModel.default.availability` maps to `AppleFMAvailability`.

| System reason | Sentence |
| --- | --- |
| device not eligible | This Mac can't run Apple Intelligence, so Apple Foundation Models are unavailable. |
| Apple Intelligence off | Turn on Apple Intelligence in System Settings, then try again. |
| model not ready | Apple Intelligence is still preparing the on-device model. Try again when it finishes. |
| other | Apple Foundation Models are unavailable. |
| macOS older than 26, or a build without the framework | Apple Foundation Models need macOS 26 or later on Apple silicon. |

The coordinator sets the connection to failed with that sentence and does not create a `LanguageModelSession`.

`Mac/Config/Base.xcconfig` weak-links `FoundationModels`. The app deployment target stays macOS 15. A Mac that does not have the framework still opens. The session code is compiled only when the SDK contains the module.

The session uses a 4096-token window, `AppleFMCatalog.defaultContextTokens`. Apple documents that size for the on-device model. `SystemLanguageModel.contextSize` arrived in the 26.4 SDK. This build does not call it, so an SDK that has `FoundationModels` and not `contextSize` still compiles. The catalog row is id `system`, provider `apple`, name `Apple Foundation Model`. The only thinking level is `off`.

### Prompt budget

The editor envelope can be far larger than 4096 tokens. `AppleFMPromptBudget` gives the user message half of the window, counted as 4 bytes per token, and leaves the other half for instructions, tool schemas, and the reply. A budget under 64 bytes is rejected. A message that fits is sent unchanged. A message that fits after the editor block is replaced with a short omission notice is sent, and the panel gets a non-failed notice. A message that still does not fit keeps a UTF-8 suffix of the user text, with a truncation notice.

### Tools and approval

The model sees three tools.

| Tool | Permission |
| --- | --- |
| `read_file` | Allowed inside the project. No approval. |
| `write_file` | Approval, kind `file`, title `Write file`. |
| `run_command` | Approval, kind `command`, title `Run command`. |

Any other tool name is refused. `approve` runs the action once. `always_allow` remembers that kind for the session. Any other wire decision is a deny. Paths must be relative, without a NUL, a leading slash or backslash, a colon, or a `..` component. Commands run with the project directory as the working directory, after approval, and stop after 30 seconds. Read output is capped at 8000 bytes. Command output is capped at 4000 bytes. A write larger than 256000 bytes is refused.

The panel already queues `approval_request` and answers with `approval_response`. Stop denies queued approvals, then sends `abort`.

### Streaming and cancel

`streamResponse` yields cumulative snapshots. `AppleFMStreamCursor` turns a snapshot that extends the previous text into a `text_delta` for the new suffix. A snapshot that does not extend the previous text updates the stored reply and emits no delta. `message_end` then carries the stored reply, and the panel replaces the row from that message. The turn then emits `turn_end` and `agent_end`. A thrown generation error becomes `agent_end` with `stopReason` `error` and the framework's localized description.

`abort` cancels the generation task, stops a running command, resumes approval waiters as deny, and closes an open turn once. A second prompt while a turn is open fails with a sentence. The model does not start a second generation on top of the first. `new_session` drops the `LanguageModelSession`. `switch_session` answers `cancelled: true` so the host does not load a pi transcript this model does not have. `get_commands` is empty. The host adds `/resume` itself.

A provider switch calls `shutdown` while a turn is open. `shutdown` clears the turn flag, retires `processEpoch`, drops the pending approval, and calls `terminate` on the session. `terminate` cancels the Apple FM generation task. `connect(_:)` watches that epoch for every session. Apple FM does not launch a process, so `connect` mints the token when the caller did not. An event whose token is no longer current is ignored, including a late snapshot from the cancelled stream.

## Why this is not a helper process

A small Swift helper that spoke the same JSONL would keep the host rule "every provider is a subprocess." That rule is the one this change removes. A helper also needs a second Xcode product, a copy phase, and signing. `Tools/validate-xcodeproj.mjs` is strict about the project file. Crash isolation is the real advantage of a helper. `FoundationModels` can still fail the process. This design surfaces `GenerationError` and later `LanguageModelError` as a failed turn. It does not claim the framework cannot crash the app.

## What a Mac with Apple Intelligence still has to prove

Linux CI runs the contract tests and the Rust launch test. The macOS CI job compiles the Swift file when the SDK has `FoundationModels`. This design does not claim an on-device reply was generated.

These still need a Mac with Apple Intelligence:

- The live `availability` value matches the sentences above on a machine that is ineligible, has Apple Intelligence off, or is still downloading the model.
- `LanguageModelSession(tools:instructions:)` and `streamResponse(to:)` compile and stream text snapshots whose `.content` is a `String`.
- Cancelling `streamResponse` stops the turn, and a tool call can wait for approval without blocking the Approve button. If the framework isolates the session to the main actor, the wait has to be checked again.
- `contextSize` on that OS, if it differs from 4096.
- A real `read_file`, an approved `write_file`, and an approved `run_command` against a project directory.
