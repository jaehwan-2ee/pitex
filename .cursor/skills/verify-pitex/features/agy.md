# Antigravity

The window does not list Antigravity. A stored `agy` reads as pi and starts the built-in runtime. Headless stream-json cannot answer tool approvals, so this build does not start that adapter.

The adapter source stays in the repo. `node --test Mac/Resources/PitexAgent/bridge/test.ts` still covers `new AgyAdapter()`.

## Sub-features

- The picker lists pi, Codex, Claude Code, and Grok.
- A saved `agy` still opens the built-in assistant.
- The assistant does not launch `cli.ts`.

## How to get to it (user POV)

Open a `.tex` file. Press Ctrl+comma. Open the AI page. Open Assistant backend. The menu has no Antigravity row. Close Settings and open the assistant. A prompt stays on the built-in runtime.

## Driving it with verify-pitex

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider stale-agy
```

Pass means the menu screenshot is taken while `ai.agentProvider` is still `agy`, the assistant composer is on screen after the prompt, the fake CLI record does not contain `Fix the title`, and no descendant of the Pitex pid has `cli.ts` in its command line. The stored value may still be `agy` or it may be `pi`.

A normal `gtk` run for Codex, Claude Code, or Grok saves `02-menu.png` while the picker is open, before the row click.

## Gotchas

`drive --provider agy` is a usage error. Doctor does not require `helpers/fakes/agy.mjs`. The file stays on disk.
