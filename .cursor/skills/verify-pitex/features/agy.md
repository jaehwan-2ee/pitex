# Antigravity

Settings, AI, Assistant backend, Antigravity CLI. The bridge spawns `agy` with `--sandbox` and stream JSON. Headless mode cannot ask for permission.

## Sub-features

- Stream text and show the command row.
- Approve, Deny, and Always allow each return an error and send nothing to `agy`.
- Abort a held turn with SIGINT.
- Start a new conversation and resume the first one.
- Report a missing `agy` binary, and `agy models` exiting with an authentication error, without exiting the bridge.

## How to get to it (user POV)

Same Settings path as Codex. Choose Antigravity CLI (your login). The assistant may show a command row. Approve, Deny, and Always allow do not reach the CLI. The assistant reports that the permission cannot be sent.

## Driving it with verify-pitex

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider agy
```

Pass for `agy-approve`, `agy-deny`, and `agy-always_allow` means each response has `"success": false`, `"sent": false`, and an error that contains `cannot be sent`. Pass for `agy-sandbox` means every stream-json spawn argv contains `--sandbox`, omits `--dangerously-skip-permissions`, omits `--mode`, and the cwd is the project. Pass for `agy-sigint` means the CLI record contains `SIGINT`. Pass for `agy-no-permission-on-stdin` means that record has no `approval_response` and no `control_request`.

## Gotchas

The limit is the product behavior. A journey that expects Approve to run a command is the wrong journey. Do not weaken the checks so a delivered approval counts as a pass.

The window command is `gtk --provider agy`. Pass there is the permission-limit sentence on screen and no approval response in the CLI record.
