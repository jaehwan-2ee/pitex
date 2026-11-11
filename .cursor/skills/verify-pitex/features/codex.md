# Codex

Settings, AI, Assistant backend, Codex CLI. The assistant talks to the Codex app-server the bridge spawns.

The Settings row is hidden, and a stored `codex` is the `gtk --provider stale-codex` journey. `drive --provider codex` is how this adapter is still proved.

## Sub-features

- A stored `codex` preference starts pi. The window does not send the prompt to the Codex CLI.
- Stream assistant text, show the command row, and accept Approve, Deny, and Always allow.
- Abort a turn that has not finished.
- Start a new conversation and resume the first one.
- Report a missing `codex` binary, and a signed-out `account/read`, without exiting the bridge.

## How to get to it (user POV)

The Assistant backend menu does not list Codex CLI. A stored `codex` preference opens as pi. The bridge adapter still runs when the drive sets `PITEX_CLI_ADAPTER=codex`.

## Driving it with verify-pitex

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider stale-codex
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider codex
```

`stale-codex` is the window path. It does not click a menu row. `drive` sends the pi commands for Deny, Always allow, abort, new conversation, resume, missing CLI, and signed out. Pass is `gtk-result.json` with `"pass": true` and every object in the Codex `checks` array with `"pass": true`.

## Gotchas

The window clicks assume a 1280 by 800 screen and the default 720 by 560 Settings window. AT-SPI on GTK 4.14 does not list the buttons, so a layout change misses the click.

The fake answers `account/read` with `authToken` set to `must-not-leak`. The pi stdout must not contain that string. The check is `no-token-on-pi-stream`.
