# Native chat: Antigravity backend and chat UI parity

Status: plan, 2026-11-24. Each milestone ships as its own pull request.

## Goal

The owner asked for four things:

- Bring back the Antigravity (`agy`) backend.
- Make it selectable like Claude Code, running on the user's default home.
- Bring pitex's own agent chat UI up to the level of T3 Code, the Claude Code VS Code extension, Paseo and Orca.
- Use that one UI for both Claude Code and Antigravity.

## Findings

### The hidden Antigravity work

- #23 (`ca3834d`) added `Mac/Resources/PitexAgent/bridge/agy-adapter.ts` and `agy-map.ts`. They drive `agy` in headless stream-json mode.
- #28 (`398017a`) hid the backend before any release, because stream-json cannot answer tool approvals (`agy-adapter.ts:317-320` always refuses `approval_response`). The source stayed in the tree.
- `adapter.ts:46` maps `agy` to `codex`, so nothing reaches the agy code today. Its seven tests in `test.ts` still pass against a fake CLI.
- A stored `ai.agentProvider=agy` reads as `pi` on Mac (`SettingsView.swift:189-193`) and on Linux and Windows (`settings.rs`).
- #32, #31 and #35 later rebuilt the provider lists, so a plain revert of #28 does not apply. Every host change has to be rewritten against the current provider tables.

### How the Claude Code backend works today

- The Node bridge (`Mac/Resources/PitexAgent/bridge/cli.ts`) speaks pi RPC JSONL to the host.
- `claude-adapter.ts` runs the vendored Agent SDK 0.3.312 against the user's installed `claude`, with the user's own `HOME` and login.
- Every edit and every mutating command goes through `canUseTool`, because the SDK runs with `permissionMode: "default"` and `settingSources: []`. "Always allow" lasts for the session only.
- Hosts:
  - Mac (SwiftUI): `AgentCoordinator.swift`, `AgentPanel.swift`.
  - Linux (GTK): `pitex-shell` `agent.rs`, `panes.rs`, `app_ui.rs`.
  - Windows: links the same `pitex-shell` crate, so it has the same panel.
- All three platforms have the Claude Code GUI.
- The history popover lists only pitex's own mirror files in `pitex-sessions/<slug>/*.jsonl`. No platform lists Claude Code sessions that were started outside pitex.

### Antigravity over ACP (measured against agy 1.5.15 on Linux)

`agy acp` is an Agent Client Protocol stdio server.

- **Startup.** `session/new` works with the CLI's existing sign-in. No `authenticate` call is needed.
- **Advertised capabilities:**
  - `loadSession`
  - `sessionCapabilities.list`, `resume`, `load` and `close`
  - image prompts
  - `_meta.antigravity.systemPrompt.append`
- **Session options.** `session/new` returns config options:
  - `model`: 11 base ids
  - `thought_level`: low, medium, high or xhigh
  - `mode`: default, accept-edits or plan
- **Session list.** `session/list` returns `{sessionId, cwd, title, updatedAt}` for every conversation in the user's default agy home. pitex lists them over ACP and never reads agy's files.
- **Wire test.** One prompt in a temp project, with every permission rejected:
  - The command `touch ran.txt` raised `session/request_permission` with `allow_once` and `reject_once` (there is no `allow_always`). After the rejection the tool failed and the file was not created.
  - A write to an absolute path outside the project failed without a prompt. Nothing was written.
  - A write inside the project went ahead without a prompt in mode `default`.
  - agy never called the client's `fs/*` or `terminal/*` methods. It runs its own tools.

Paseo drives agy the same way. It uses `agy acp` through a generic ACP runtime (`plugins/antigravity-provider` in github.com/getpaseo/paseo). It imports old conversations with `session/list` and opens them with `session/load`.

Paseo works around two agy quirks:

- agy replays history after the `session/load` response, so Paseo waits for 200 ms with no updates before treating output as live.
- agy sends `end_turn` while background commands are still running, so Paseo keeps the turn open until they finish.

### Safety difference to note

| Rule | Claude Code in pitex | Antigravity in pitex |
| --- | --- | --- |
| Shell commands | Asks | Asks |
| Writes inside the project | Asks | Does not ask in mode `default` (agy's own policy) |
| Writes outside the project | Asks | Refused by agy |

pitex keeps agy in mode `default` and shows every request agy sends. The edits agy makes inside the project are shown as file-edit cards.

## Feature gap table

Sources:

- **T3:** T3 Code 0.1.11 bundle.
- **VS Code:** the Claude Code extension's public docs.
- **Paseo:** the getpaseo/paseo app source.
- **Orca:** Orca 1.4.271 mobile-web chat bundle.

Columns:

- **pitex CC:** pitex's Claude Code chat today (Mac / Linux).
- **agy after M1:** what the Antigravity backend can do once milestone 1 lands.
- **P / PR:** priority, and the PR that closes the gap.

| Feature | T3 | VS Code | Paseo | Orca | pitex CC | agy after M1 | P / PR |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Streaming transcript | yes | yes | yes, paced reveal | yes | yes / yes | yes | done |
| Tool cards, expand/collapse | yes, density modes | yes | yes, grouped per turn | yes | Mac yes, Linux always open | same as Claude | P0, M2 |
| Typed tool details (shell exit code, read range, search hits) | yes | yes | yes | yes | name guessed from args | same | P0, M2 |
| Diffs and file edits | diff panel | native diff, accept/reject | unified or line diff | diff card | raw text only | same | P0, M2 |
| Markdown, code blocks, copy | yes | yes | yes | yes | plain text | same | P0, M3 |
| Math in messages | no | no | off in production | no | plain text | same | P0, M3 |
| Approval prompts | yes | yes, permission modes | yes, option list | yes, modes | Approve / Deny / Always | real prompts | done; modes in M4 |
| Ask-user questions | yes | yes | yes | yes | no answer path | n/a | P0, M4 |
| Plan mode review | yes | yes | yes | yes | no | agy `/plan` text only | P1, M4 |
| Plan / todo checklist | yes | yes | yes | yes | generic tool card | ACP plan as card | P1, M5 |
| Interrupt / stop | yes | yes | yes | yes | yes / yes | yes | done |
| Session list, resume | yes | yes | yes, plus import | yes | pitex-started chats only | agy home sessions for this project | P1, M6 for Claude |
| Model picker | searchable | yes | yes | yes | yes / yes | yes | done |
| Effort picker | yes | thinking toggle | per model | yes | yes / yes | yes, per model | done |
| Mode picker (default, accept edits, plan) | yes | yes | yes | yes | no | no | P1, M4 |
| Queued messages, steer | edit, reorder | yes | queue, steer, interrupt | edit, pause | sent, not shown | queued in bridge | P1, M7 |
| @file mentions | yes | yes, line ranges | yes | yes | no | no | P1, M7 |
| Image attachments | yes | yes | yes | yes | paths as text | paths as text | P1, M7 |
| Slash command menu | yes | yes | yes | yes | Claude list empty | agy commands | P1, M7 |
| Context and usage meter | yes | partial | yes, warning colors | yes | one text line; Linux has no limits | context from `usage_update` | P2, M8 |
| Thinking blocks | yes | yes | collapsed | collapsed | Mac collapsible, Linux open | yes | P2, M8 |
| Error states, retry | yes | yes | yes, retry | yes, continue | notices, no retry button | same | P2, M8 |
| Scroll pinning, jump to bottom | yes | yes | yes | yes | always forced | same | P2, M8 |
| Subagents | grouped | yes | grouped, drill in | grouped | generic card | generic card | P3, M9 |
| Fork, rewind, checkpoints | yes | yes | yes | conversation only | no | no | P3, M9 |

## Milestones

Each milestone is one draft PR branched from `origin/main`. Claude Code goes first, and Antigravity shares the same UI.

| PR | Change | Depends on |
| --- | --- | --- |
| M1 | Antigravity over ACP: selectable backend, approvals, default-home session import | none |
| M2 | Typed transcript events from the bridge (tool detail, status, turn ids), tool cards with status and diffs on Mac and Linux | M1, for the shared ACP mapping |
| M3 | Markdown, code blocks with copy, and math in assistant messages | none |
| M4 | Approvals: question cards, plan review, permission mode picker | M2 |
| M5 | Plan and todo checklist | M2 |
| M6 | Claude Code default-home session import through the SDK's `listSessions` and `getSessionMessages` | M1, for the shared import path |
| M7 | Composer: visible queue, steer, @file mentions, image attachments, Claude slash commands | M2 |
| M8 | Context meter, usage push, thinking and scroll behaviour, retry | M2 |
| M9 | Subagent grouping, turn folding, fork and rewind | M2, M7 |

### Verification for every PR

- **Linux:**
  - `node --test` in `Mac/Resources/PitexAgent/bridge`.
  - `cargo test --workspace --manifest-path Linux/Cargo.toml`.
  - The ignored GTK tests under `xvfb-run`.
  - A live drive of the GTK app.
- **macOS:** run in the coordinator's VM. Each PR lists exact steps.
