# Codex

Settings, AI, Assistant backend, Codex CLI. The assistant talks to the Codex app-server the bridge spawns.

## Sub-features

- Select Codex and send a prompt from the window.
- Stream assistant text, show the command row, and accept Approve, Deny, and Always allow.
- Abort a turn that has not finished.
- Start a new conversation and resume the first one.
- Report a missing `codex` binary, and a signed-out `account/read`, without exiting the bridge.

## How to get to it (user POV)

Open a `.tex` file. Press Ctrl+comma. Open the AI page. Set Assistant backend to Codex CLI (your login). Close Settings. Open the assistant from the header. Type in Ask about your document and press Return. Approve, Deny, or Always allow on the command banner.

## Driving it with verify-pitex

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs gtk --provider codex
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider codex
```

`gtk` is the window path. `drive` sends the pi commands for Deny, Always allow, abort, new conversation, resume, missing CLI, and signed out. Pass is `gtk-result.json` with `"pass": true` and every object in the Codex `checks` array with `"pass": true`.

## Gotchas

The window clicks assume a 1280 by 800 screen and the default 720 by 560 Settings window. AT-SPI on GTK 4.14 does not list the buttons, so a layout change misses the click.

The fake answers `account/read` with `authToken` set to `must-not-leak`. The pi stdout must not contain that string. The check is `no-token-on-pi-stream`.
