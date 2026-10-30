---
name: verify-pitex
description: Drive the Pitex GTK window and the pitex-agent-bridge adapters for Codex, Claude Code, and Grok, then keep the evidence. Use when changing AI provider selection, the bridge, or an adapter, or when a provider journey needs a proof.
---

# Verify Pitex

The window is the user surface. Settings, then AI, then Assistant backend lists pi, Codex, Claude Code, and Grok. The window does not list Antigravity. A stored `agy` reads as pi and starts the built-in runtime. Codex, Claude Code, and Grok launch `Mac/Resources/PitexAgent/bridge/cli.ts`. This skill drives that window when `Linux/target/debug/pitex` exists. It also drives the bridge on stdin for abort, resume, a missing CLI, a signed-out CLI, the Claude child environment, and the Grok file jail.

The adapter source stays in the tree. `node --test Mac/Resources/PitexAgent/bridge/test.ts` still covers `new AgyAdapter()`.

The programs in `helpers/fakes/` stand in for the vendor CLIs. They are not logins. Do not read or copy `~/.codex`, `~/.claude`, `~/.grok`, or any token file. Every drive uses a new `HOME` under `/tmp/pitex-verify/`. `helpers/fakes/agy.mjs` stays on disk. Doctor does not require it.

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

Doctor checks that the bridge file and the Codex, Claude Code, and Grok fakes exist. It does not start Pitex.

## Drive

Bridge journeys:

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider all
```

`drive --provider all` drives Codex, Claude Code, and Grok only. One provider is `--provider codex`, `claude`, or `grok`. `drive --provider agy` is a usage error.

Window journey, after the binary exists. The provider is `codex`, `claude`, or `grok`.

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider claude
```

`gtk` opens `main.tex` under Xvfb, presses Ctrl+comma, opens the AI page, and opens the backend dropdown. `02-menu.png` is the open picker. The command then chooses that provider, opens the assistant, and sends `Fix the title`. It clicks Approve. Pass means `preferences.json` stores that provider and the CLI record contains the prompt. Approve is `accept` for Codex, `allow` with `user_temporary` for Claude Code, and `allow-once` for Grok.

A stored `agy` uses the same window.

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider stale-agy
```

That command writes `{"ai.agentProvider":"agy"}` before launch. It opens Settings, the AI page, and the backend dropdown, and it does not click a menu row. It dismisses the menu, closes Settings, opens the assistant, types `Fix the title`, and presses Return. It does not click Approve. Pass means the menu shot was taken while the stored value was still `agy`, the sent screenshot shows the assistant composer, the fake CLI record does not contain `Fix the title`, and no descendant of the Pitex pid has `cli.ts` in its command line. The composer check reads that screenshot with `tesseract`.

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

`helpers/verify-pitex.mjs` is the command above. `helpers/gtk-drive.py` is the window script. `helpers/fakes/*.mjs` are the CLIs the bridge execs. The helper sets `CODEX_PATH`, `CLAUDE_PATH`, or `GROK_PATH` to those files.

Keep the map current with `/maintain-verification-skill` when the app changes. The map is `features/`.
