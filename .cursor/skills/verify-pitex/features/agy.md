# Antigravity

Settings, AI, Assistant backend, Antigravity CLI. The bridge runs `agy acp --sandbox --disable-telemetry` in the project folder with the user's own `HOME` and sign-in, and speaks ACP to it.

## Sub-features

- Stream text and show `run_command` as a bash row.
- Ask before a command. Approve sends `allow`, Deny sends `deny`. agy offers no allow_always, so Always allow sends `allow` and the bridge answers the same command again by itself until the next new or switched conversation.
- Switch the session to mode `default` after `session/new` and `session/load`, so agy's own settings cannot leave it in accept-edits.
- List the user's existing agy conversations for this project (ACP `session/list`) in the history list, once each, and replay one on resume.
- Abort a held turn with `session/cancel`.
- Report a missing `agy` binary, and a signed-out CLI, without exiting the bridge.

## How to get to it (user POV)

Open a `.tex` file. Press Ctrl+comma. Open the AI page. Turn on Antigravity CLI. Open the assistant and send a prompt. A command shows an approval card. The history button lists the conversations agy already has for this folder.

## Driving it with verify-pitex

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider agy
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider agy
```

Pass for the agy checks means the CLI record has `session/set_config_option` with mode `default`, the sessions folder has `agy-home-1.jsonl` titled `From the agy home` and no `agy-home-2.jsonl` (another folder), and a repeated `ALWAYS allow the build` prompt raises no approval row while the fake notes one more `"permission":"allow"`.

## Gotchas

The fake starts every session in `accept-edits`. A run that skips the mode switch fails `agy-mode-default`.

Resume text is the replayed chunk `Restored`. It arrives after the `session/load` response, like the real agy.

The fake is not a login. Never point a drive at the real `~/.gemini` folder or copy anything from it.
