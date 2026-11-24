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

Tool cards: `gtk --provider claude-toolcards` chooses Claude Code like `claude`, drags the editor/console divider up, and sends `TOOLCARDS`. The fake answers with a Read of `intro.tex` (lines 1-40), an Edit of `main.tex` with a structured patch, and a Bash call that exits 1. `04-sent.png` shows the three collapsed cards, `05-expanded.png` the expanded bodies (diff, command and output), and `06-open-file.png` the editor after the Read card's Open file. Pass means the prompt reached the fake; with `tesseract` installed the titles and the diff are also read from the shots. Without it, look at the shots.

Markdown: `gtk --provider claude-markdown` chooses Claude Code the same way and sends `MARKDOWN`. The fake streams a Markdown answer (heading, list, inline code, inline and display math, a ```latex block, a table, a quote, a project-file link and a web link) and holds for 4 s inside the open ```latex fence. `04-streaming.png` must show that fence as a code box running to the end of the text; `05-rendered.png` is the finished message with typeset math (pdflatex) and the highlighted code block. The run clicks the block's Copy (`06-copied.png`, back to Copy 1.5 s later in `06-copy-reset.png`) and reads the clipboard with `xclip`; `07-scrolled-*.png` wheel down to the table, quote and links; `08-file-link.png` is the editor after clicking the `intro.tex:1` link. Pass means the prompt reached the fake and the clipboard holds the code block. `PITEX_GTK_THEME=dark` seeds the dark theme before launch.

Approvals: `gtk --provider claude-approvals` chooses Claude Code, drags the divider up like `claude-toolcards`, and opens the mode picker on the composer row (`04-mode-picker.png` shows "Ask before edits", `05-mode-open.png` the open list). It picks Plan with the keyboard (`06-mode-plan.png`), sends `QUESTION` (`07-question.png`: header chips, a radio group with descriptions, checkboxes, an Other field, Submit off), types `XeLaTeX` in the first question's Other field and ticks both parts (`08-question-filled.png`), presses Enter in the Other field to submit (`09-question-answered.png`), then sends `PLAN`. `10-plan.png` is the Markdown plan card; the run types feedback (`11-plan-feedback.png`), presses Enter (Keep planning), gets the revised plan (`12-plan-revised.png`), and clicks Approve and accept edits (`13-plan-approved.png`: the picker reads Accept edits on an amber tint). Pass means the fake noted `set_permission_mode` plan, the answers keyed by question text (`XeLaTeX`, `Intro, Method`), a deny carrying the feedback, an allow with `setMode` acceptEdits, and `set_permission_mode` acceptEdits.

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
