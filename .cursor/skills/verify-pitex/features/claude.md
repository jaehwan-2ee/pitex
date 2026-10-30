# Claude Code

Settings, AI, Assistant backend, Claude Code. The bridge spawns `claude` and strips billing and endpoint variables from that child.

## Sub-features

- Stream text, show the Bash row, and answer Approve, Deny, and Always allow.
- Abort a held turn.
- Start a new conversation and resume the first one.
- Report a missing `claude` binary, and `claude auth status` logged out, without exiting the bridge.
- Drop the probe billing and endpoint variables from the child environment while the bridge process still has them.

## How to get to it (user POV)

Same Settings path as Codex. Choose Claude Code (your login). The assistant banner is Approve, Deny, or Always allow for a Bash command.

## Driving it with verify-pitex

```sh
node .cursor/skills/verify-pitex/helpers/verify-pitex.mjs drive --provider claude
```

The parent environment contains probe values for `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`, the Bedrock, Vertex, and Foundry flags and keys, and every `ANTHROPIC_*BASE_URL` the bridge knows. Pass for `env-strip` means those names except bare `AUTH_TOKEN` are absent on every `claude.mjs` process, and present on the bridge process. `proc.json` records the names, not the values.

## Gotchas

`bare-auth-token` fails. The Claude child still has bare `AUTH_TOKEN`. `ANTHROPIC_AUTH_TOKEN` is absent. Leave the check failed. Do not edit the adapter from a verification change.

`gtk --provider claude` is implemented. The menu row is the third item. The recorded proof for the window is Codex. Re-run `gtk` before treating the Claude window path as proven.
