# Grok

Settings, AI, Assistant backend, Grok CLI. The bridge speaks ACP to `grok agent stdio` and keeps file and terminal access inside the project root.

## Sub-features

- Stream text, show the command row, and answer Approve, Deny, and Always allow.
- Abort a held turn with `session/cancel`.
- Start a new conversation and resume the first one.
- Report a missing `grok` binary, and a login with no `cached_token` method, without exiting the bridge.
- Refuse a read and a write outside the project. Allow a write inside the project. Run a terminal whose cwd is the project.

## How to get to it (user POV)

Same Settings path as Codex. Choose Grok CLI (your login). Approve a command in the assistant. The jail is not a separate screen. It is what happens when the CLI asks to read or run something outside the open project.

## Driving it with verify-pitex

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider grok
```

The project contains `note.txt` with `inside-note`. A sibling directory contains `marker.txt`. Pass for the jail checks means the CLI record contains `outside this project` and `inside-note`, `from-grok.txt` exists in the project, `pwned.txt` does not exist outside it, `marker.txt` is unchanged, and `terminal-cwd.txt` contains the project path.

## Gotchas

Resume text from the CLI is the loaded session chunk `Restored`. The pass is that string, or the original prompt, in `get_messages`.

The Grok process cwd in `proc.json` is the project directory.

The in-project terminal request puts the whole shell command in `command` and omits `args`. That is the shape the Grok CLI sends for `terminal/create`.
