---
name: verify-pitex
description: Drive the Pitex GTK window and the pitex-agent-bridge adapters for Codex, Claude Code, Grok, and Antigravity, then keep the evidence. Use when changing AI provider selection, the bridge, or an adapter, or when a provider journey needs a proof.
---

# Verify Pitex

The window is the user surface. Settings, then AI, then Assistant backend lists pi, Claude Code, and Antigravity CLI. The window does not list Codex CLI or Grok CLI. A stored `codex` or `grok` reads as pi and starts the built-in runtime. Codex, Claude Code, Grok, and Antigravity launch `Mac/Resources/PitexAgent/bridge/cli.ts`. This skill drives that window when `Linux/target/debug/pitex` exists. It also drives the bridge on stdin for abort, resume, a missing CLI, a signed-out CLI, the Claude child environment, the Grok file jail, and the Antigravity mode, import, and Always allow rules.

The programs in `helpers/fakes/` stand in for the vendor CLIs. They are not logins. Do not read or copy `~/.codex`, `~/.claude`, `~/.grok`, `~/.gemini`, or any token file. Every drive uses a new `HOME` under `/tmp/pitex-verify/`.

## Launch

Build the window once:

```sh
cargo build -p pitex --manifest-path Linux/Cargo.toml
```

The bridge does not need a build. Node 24 runs `cli.ts` directly. A drive is ready when doctor prints `"worthDriving": true`.

Two Pitex processes share one app id on a session bus. Run the window drive inside its own `dbus-run-session`, which the `gtk` command already does. Do not point it at a display where someone is using Pitex.

## Doctor

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs doctor
```

Doctor checks that the bridge file and the Codex, Claude Code, Grok, and Antigravity fakes exist. It does not start Pitex.

## Drive

Bridge journeys:

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider all
```

`drive --provider all` drives Codex, Claude Code, Grok, and Antigravity. One provider is `--provider codex`, `claude`, `grok`, or `agy`.

Window journey, after the binary exists. The clickable provider is `claude`. `agy` is written to the preferences before launch instead of clicked, then runs the same prompt and Approve.

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider claude
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider agy
```

`gtk` opens `main.tex` under Xvfb, presses Ctrl+comma, opens the AI page, and opens the backend dropdown. `02-menu.png` is the open picker. The command then chooses Claude Code, opens the assistant, and sends `Fix the title`. It clicks Approve. Pass means `preferences.json` stores `claude` and the CLI record contains the prompt. Approve is `allow` with `user_temporary`. For `agy`, pass means `preferences.json` still stores `agy`, the record contains the prompt, and the fake noted `"permission":"allow"`.

Tool cards: `gtk --provider claude-toolcards` chooses Claude Code like `claude`, drags the editor/console divider up, and sends `TOOLCARDS`. The fake answers with a Read of `intro.tex` (lines 1-40), an Edit of `main.tex` with a structured patch, and a Bash call that exits 1. The finished turn folds them under "Worked for <1s · 1 edit · 1 command · 1 read"; `04-sent.png` shows that row, `04-fold-open.png` the fold opened on the three collapsed cards, `05-expanded.png` the expanded bodies (diff, command and output), and `06-open-file.png` the editor after the Read card's Open file. Pass means the prompt reached the fake; with `tesseract` installed the titles and the diff are also read from the shots. Without it, look at the shots.

Markdown: `gtk --provider claude-markdown` chooses Claude Code the same way and sends `MARKDOWN`. The fake streams a Markdown answer (heading, list, inline code, inline and display math, a ```latex block, a table, a quote, a project-file link and a web link) and holds for 4 s inside the open ```latex fence. `04-streaming.png` must show that fence as a code box running to the end of the text; `05-rendered.png` is the finished message with typeset math (pdflatex) and the highlighted code block. The run clicks the block's Copy (`06-copied.png`, back to Copy 1.5 s later in `06-copy-reset.png`) and reads the clipboard with `xclip`; `07-scrolled-*.png` wheel down to the table, quote and links; `08-file-link.png` is the editor after clicking the `intro.tex:1` link. Pass means the prompt reached the fake and the clipboard holds the code block. `PITEX_GTK_THEME=dark` seeds the dark theme before launch.

Approvals: `gtk --provider claude-approvals` chooses Claude Code, drags the divider up like `claude-toolcards`, and opens the mode picker on the composer row (`04-mode-picker.png` shows "Ask before edits", `05-mode-open.png` the open list). It picks Plan with the keyboard (`06-mode-plan.png`), sends `QUESTION` (`07-question.png`: header chips, a radio group with descriptions, checkboxes, an Other field, Submit off), types `XeLaTeX` in the first question's Other field and ticks both parts (`08-question-filled.png`), presses Enter in the Other field to submit (`09-question-answered.png`), then sends `PLAN`. `10-plan.png` is the Markdown plan card; the run types feedback (`11-plan-feedback.png`), presses Enter (Keep planning), gets the revised plan (`12-plan-revised.png`), and clicks Approve and accept edits (`13-plan-approved.png`: the picker reads Accept edits on an amber tint). Pass means the fake noted `set_permission_mode` plan, the answers keyed by question text (`XeLaTeX`, `Intro, Method`), a deny carrying the feedback, an allow with `setMode` acceptEdits, and `set_permission_mode` acceptEdits.

Todo list: `gtk --provider claude-todos` chooses Claude Code, drags the divider up, and sends `TODOS`. The fake sends three TodoWrite snapshots 1.5 s apart (all pending; Build the PDF in progress; two completed and Check the log in progress), then answers. `04-todos-created.png` shows "Task list created (3)" and the collapsed checklist above the composer, `05-todos-started.png` "Started: Build the PDF" and `Tasks 0/3 · Building the PDF`, `06-todos-collapsed.png` all five activity rows and `Tasks 2/3` (no step once the turn is over); the run expands the checklist (`07-todos-expanded.png`: two struck-through rows with a check, a half circle on the third), clicks the trash button (`08-new-chat.png`: no checklist), opens the history popover (`09-history.png`) and reopens the chat (`10-resumed.png`: the transcript without TodoWrite rows and the checklist back at 2/3). It then sends `LONGTODOS`: one 24-item list held 4 s (`11-todos-long-running.png`: one "Task list created (24)" row and the expanded checklist capped at about 200 px and scrolling; `12-todos-long-done.png`: the header back to `Tasks 3/24` once the turn ends). Last it opens Settings, turns Antigravity on and Claude Code off (`13-switch-settings.png`), and closes Settings (`14-switched.png`: the switch notice and no checklist). Pass means the prompt reached the fake; look at the shots.

Composer: `gtk --provider claude-composer` chooses Claude Code, drags the divider up, hides the sidebar (its points were measured that way), and sends `SLOW` (the fake holds the turn 16 s; like the real CLI, a message pushed during it gets its own turn and result afterwards). It types two `PLAIN queued …` messages with Return (`04-queued.png`: "Queued (2)" rows with Edit and Delete; no Send now, because Claude reports `steer:false`; the send button stays beside Stop), clicks Edit on the first (`05-editing.png`: its text back in the composer), appends ` edited` and presses Return (`06-requeued.png`: it is now last), then queues `PLAIN queued three` with Ctrl+Return (`07-ctrl-return-queued.png`: three rows, nothing steered). It clicks Stop (`08-stopped.png`: an `interrupted` notice, the queue kept and paused, and every idle row now offers Send now), then Send now on the first row (`09-resumed.png`: `PLAIN queued two`, `PLAIN queued one edited` and `PLAIN queued three` each answered in order, one per turn). Then `see @mai` opens the file list (`10-mention.png`), Tab inserts `@main.tex ` (`11-mention-inserted.png`), and `@mai` followed by a click in the transcript closes the list (`12-mention-closed.png`); Shift+Return grows the composer with its first line in view (`13-multiline.png`); `/` lists Claude's commands with argument hints and without `/model` (`14-slash.png`); `/clear` empties the panel without reaching the backend (`15-cleared.png`); a red PNG put on the clipboard with `xclip` pastes as a chip (`16-image-chip.png`), goes out with `what color is this square?` (`17-image-sent.png`: the thumbnail in the transcript and the fake's `I see 1 image(s): image/png`), and is sent again with no text (`18-image-only.png`). Pass means the fake got the user messages in the order `SLOW`, `PLAIN queued two`, `PLAIN queued one edited`, `PLAIN queued three`, never `/clear`, and two `image/png` blocks, the first with `what color is this square?`. The buttons are measured coordinates (`QUEUE_EDIT_FIRST`, `QUEUE_SEND_NOW_FIRST`, `STOP`).

Usage and errors: `gtk --provider claude-usage` chooses Claude Code, drags the divider up, keeps the sidebar shown, and sends `PLAIN warm up` (`03-first-turn-no-meter.png`: no meter, because Claude reports the model's window only with a turn's result and the bridge does not guess one; `04-1280-send-visible.png`: the whole assistant pane, Send included, fits the 1280 px window), then sends `USAGE`. The fake streams thinking, a sentence, a Read, more text and a 24-line list, with usage at 30 %, 73 % and 93 % of a 200k window, then fails with a rate-limit API error; a second `USAGE` (Retry) answers normally. `05-thinking-streaming.png` is the open Thinking block with a spinner and a grey `30% · 60k/200k` meter; `06-thinking-done-meter-amber.png` the block closed as "Thought for 3s", two separate answer rows around the Read card and an amber 73 % meter; `07-meter-red.png` a red 93 % meter; `08-meter-popover.png` the meter's popover (context, input, cached, output). The run wheels up while the list streams (`09-scrolled-up-jump.png`: the view stays put and "Jump to latest" shows), clicks it (`10-jumped.png`: following again), waits for the failure (`11-error-retry.png`: "Claude Code hit a rate or usage limit…", Details, Retry), opens Details (`12-error-details.png`: the raw API error) and clicks Retry (`13-retried.png`: a new `USAGE` bubble and "Retried fine…", no Retry left). Pass means the fake got `USAGE` twice. The meter, Jump, Details and Retry points are measured (`METER`, `JUMP`, `DETAILS`, `RETRY`; `PITEX_<NAME>_X`/`_Y` override them).

Sign-in: `gtk --provider agy-signin` starts the Antigravity fake signed out (`FAKE_AUTH_FILE` holds `signed-out`), drags the divider up and sends `Fix the title`. `04-sent.png` must show the error row "Sign in to Antigravity in a terminal, then retry." with Details and Retry. The run then writes `ok` to that file (the user signed in) and clicks Retry (`SIGNIN`, measured): `05-retried.png` is the same prompt running on a reconnected agy with its approval card, `06-approved.png` the finished turn. Pass means the prompt reached the fake.

Subagents, folding, rewind and fork: `gtk --provider claude-branch` sets `FAKE_PERSIST=1`, so the Claude fake saves its session in Claude Code's transcript format under the run's `HOME/.claude/projects` (the SDK's `getSessionMessages` and `forkSession` read it) and keeps file checkpoints for `rewind_files`. It sends `SUBAGENT`: a short reply, then an Explore subagent (Agent tool) whose Glob and Read arrive 1.5 s apart with `parent_tool_use_id`, and the final answer. `04-subagent-collapsed.png` is the running card ("Explore", "1 tool"); the run opens it (`05-subagent-expanded.png`: the two nested calls); `06-turn1-folded.png` is the finished turn folded under "Worked for 6s · 1 tool" with the answer in view. `EDITFILE` writes a section into `main.tex` (`07-turn2-done.png`). The first answer's ⋯ opens Fork from here / Rewind to here (`08-message-menu.png`); Rewind lists `main.tex` (`09-rewind-card.png`); Rewind conversation and files runs with the fake's restore held 3 s (`FAKE_REWIND_DELAY_MS`): the run presses Esc, clicks Cancel and sends `MEANWHILE` during it (`09b-rewinding.png`: every card button greyed, Send greyed, `MEANWHILE` still in the composer). It then gives `10-rewound.png` (one turn left, "Rewound the conversation and restored main.tex.", the editor back to the original text). `11-turn1-expanded.png` opens the fold (the subagent's calls still nested), `12-menu-fork.png` and `13-forked.png` fork the chat ("Forked into a new chat…"), and `14-history.png` lists both chats. Pass means `main.tex` had the edit, is back to the original after the rewind, the fake restored exactly one file, and `MEANWHILE` never reached it. Points are measured (`SUBAGENT_CARD`, `FIRST_SUMMARY`, `FIRST_REPLY_MENU`, `MENU_REWIND`, `REWIND_BOTH`, `REWIND_CANCEL`, `FIRST_REPLY_MENU_OPEN`, `MENU_FORK_OPEN`, `HISTORY_SIDEBAR`).

Antigravity has no native fork or rewind: `gtk --provider agy-branch` seeds `agy`, sends `Fix the title` and approves like `agy`, then opens the answer's ⋯ and hovers the greyed Rewind (`07-rewind-disabled.png`: "Antigravity cannot rewind conversations."), and clicks Fork (`08-quoted-fork.png`: a new chat, the notice, and the earlier turn quoted in the composer).

Pane width: `gtk --provider pi-fit` turns pi on and Claude Code and Antigravity off in Settings, then opens the assistant. `04-pi-fit.png` must show pi's long model title ellipsized and the send button inside the 1280 px window.

A stored `codex` or `grok` uses the same window and does not click a menu row.

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider stale-codex
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider stale-grok
```

`stale-codex` writes `{"ai.agentProvider":"codex"}` before launch. `stale-grok` writes `grok`. Each command opens Settings, the AI page, and the backend dropdown. It does not click a menu row. It dismisses the menu, closes Settings, opens the assistant, types `Fix the title`, and presses Return. It does not click Approve. Pass means the menu shot was taken while the stored value was still that id. The saved value afterwards is that id or `pi`. The sent screenshot shows the assistant composer. The fake CLI record does not contain `Fix the title`. No descendant of the Pitex pid has `cli.ts` in its command line. The composer check reads that screenshot with `tesseract`.

`gtk --provider codex` and `gtk --provider grok` are usage errors. The bridge journey is `drive --provider codex` or `drive --provider grok`. The stored-value journey is `gtk --provider stale-codex` or `gtk --provider stale-grok`.

On GTK 4.14, AT-SPI lists the frame and no child widgets. The helper then clicks inside the default 720 by 560 Settings window. If that window size changes, the clicks miss and `gtk-result.json` names the failed check.

## Evidence

Each run writes `.cursor/skills/verify-pitex/evidence/<id>/`.

A bridge run stores `checks.json`, `cli-record.jsonl`, `stdout.jsonl`, `stdin.jsonl`, and `proc.json`. `proc.json` records cwd and which probe environment names were present. It does not store secret values.

A window run stores `gtk-result.json`, `preferences.json`, `cli-record.jsonl`, `01-project.png`, and `02-menu.png`. An approve run also stores the later shots through `05-approved.png`.

`drive --provider all` also writes `evidence/summary.json`.

A failed check is a finding. Leave a product failure failed. Do not edit the bridge from this skill to turn it green.

## Cleanup

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs cleanup
```

Cleanup signals only pids listed in `evidence/*/pids.json` whose command line is the bridge, a fake, or this helper. The evidence directory stays. Scratch homes under `/tmp/pitex-verify/` can be removed by hand. Do not kill Pitex by process name.

## Helpers

`helpers/verify-pitex.mjs` is the command above. `helpers/gtk-drive.py` is the window script. `helpers/fakes/*.mjs` are the CLIs the bridge execs. The helper sets `CODEX_PATH`, `CLAUDE_PATH`, `GROK_PATH`, or `AGY_PATH` to those files.

Keep the map current with `/maintain-verification-skill` when the app changes. The map is `features/`.
