---
name: verify-pitex
description: Drive the Pitex GTK window and the pitex-agent-bridge adapters for Codex, Claude Code, and Grok, then keep the evidence. Use when changing AI provider selection, the bridge, or an adapter, or when a provider journey needs a proof.
---

# Verify Pitex

The window is the user surface. Settings, then AI, then Assistant backend lists pi and Claude Code. The window does not list Codex CLI, Grok CLI, or Antigravity. A stored `codex`, `grok`, or `agy` reads as pi and starts the built-in runtime. Codex, Claude Code, and Grok launch `Mac/Resources/PitexAgent/bridge/cli.ts`. This skill drives that window when `Linux/target/debug/pitex` exists. It also drives the bridge on stdin for abort, resume, a missing CLI, a signed-out CLI, the Claude child environment, and the Grok file jail.

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

Window journey, after the binary exists. The clickable provider is `claude`.

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider claude
```

`gtk` opens `main.tex` under Xvfb, presses Ctrl+comma, opens the AI page, and opens the backend dropdown. `02-menu.png` is the open picker. The command then chooses Claude Code, opens the assistant, and sends `Fix the title`. It clicks Approve. Pass means `preferences.json` stores `claude` and the CLI record contains the prompt. Approve is `allow` with `user_temporary`.

A stored `agy`, `codex`, or `grok` uses the same window and does not click a menu row.

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider stale-agy
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider stale-codex
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider stale-grok
```

`stale-agy` writes `{"ai.agentProvider":"agy"}` before launch. `stale-codex` writes `codex`. `stale-grok` writes `grok`. Each command opens Settings, the AI page, and the backend dropdown. It does not click a menu row. It dismisses the menu, closes Settings, opens the assistant, types `Fix the title`, and presses Return. It does not click Approve. Pass means the menu shot was taken while the stored value was still that id. The saved value afterwards is that id or `pi`. The sent screenshot shows the assistant composer. The fake CLI record does not contain `Fix the title`. No descendant of the Pitex pid has `cli.ts` in its command line. The composer check reads that screenshot with `tesseract`.

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

`helpers/verify-pitex.mjs` is the command above. `helpers/gtk-drive.py` is the window script. `helpers/fakes/*.mjs` are the CLIs the bridge execs. The helper sets `CODEX_PATH`, `CLAUDE_PATH`, or `GROK_PATH` to those files.

Keep the map current with `/maintain-verification-skill` when the app changes. The map is `features/`.
