---
name: verify-pitex
description: Drive the Pitex GTK window and the pitex-agent-bridge adapters for Codex, Claude Code, Grok, and Antigravity, then keep the evidence. Use when changing AI provider selection, the bridge, or an adapter, or when a provider journey needs a proof.
---

# Verify Pitex

The window is the user surface. Settings, then AI, then Assistant backend selects pi, Codex, Claude Code, Grok, or Antigravity. For the four CLI providers the app launches `Mac/Resources/PitexAgent/bridge/cli.ts`. This skill drives that window when `Linux/target/debug/pitex` exists, and drives the bridge on stdin for abort, resume, a missing CLI, a signed-out CLI, the Claude child environment, and the Grok file jail.

The programs in `helpers/fakes/` stand in for the vendor CLIs. They are not logins. Do not read or copy `~/.codex`, `~/.claude`, `~/.grok`, or any token file. Every drive uses a new `HOME` under `/tmp/pitex-verify/`.

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

Doctor checks that the bridge file and the four fakes exist. It does not start Pitex.

## Drive

Bridge journeys:

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider all
```

One provider is `--provider codex`, `claude`, `grok`, or `agy`.

Window journey, after the binary exists. The provider is `codex`, `claude`, `grok`, or `agy`:

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider claude
```

`gtk` opens `main.tex` under Xvfb, presses Ctrl+comma, chooses that provider on the AI page, opens the assistant, and sends `Fix the title`. For Codex, Claude Code, and Grok it then clicks Approve. Pass means `preferences.json` stores that provider and the CLI record contains the prompt. Approve is `accept` for Codex, `allow` with `user_temporary` for Claude Code, and `allow-once` for Grok. For Antigravity, pass means the window shows the permission-limit sentence and the CLI record contains no approval response.

On GTK 4.14, AT-SPI lists the frame and no child widgets. The helper then clicks inside the default 720 by 560 Settings window. If that window size changes, the clicks miss and `gtk-result.json` names the failed check.

## Evidence

Each run writes `.cursor/skills/verify-pitex/evidence/<id>/`.

A bridge run stores `checks.json`, `cli-record.jsonl`, `stdout.jsonl`, `stdin.jsonl`, and `proc.json`. `proc.json` records cwd and which probe environment names were present. It does not store secret values.

A window run stores `gtk-result.json`, `preferences.json`, `cli-record.jsonl`, and `01-project.png` through `05-approved.png`.

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
