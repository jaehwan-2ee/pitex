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

The parent environment contains probe values for `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`, the Bedrock, Vertex, and Foundry flags and keys, and every `ANTHROPIC_*BASE_URL` the bridge knows. Pass for `env-strip` means those names are absent on every `claude.mjs` process and present on the bridge process. `proc.json` records the names, not the values.

The window command is `gtk --provider claude`.

## Gotchas

Claude Code runs a built-in set of read-only Bash commands, including `echo`, without an approval row. The fake does that when the prompt contains `READONLY`, and the result text is `pitex-ok`. A prompt that contains `TOUCH` asks before `touch pitex-ok` and does not finish until the decision. A file write takes the same approval path in the adapter.

Bare `AUTH_TOKEN` is out of scope. The Claude Code environment list and the authentication page name `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, and `CLAUDE_CODE_OAUTH_TOKEN` as the credentials. They also name the Bedrock, Vertex, and Foundry switches and the `ANTHROPIC_*BASE_URL` overrides. The authentication page says those credentials apply to the CLI and to the Agent SDK. Neither page names a variable that is only `AUTH_TOKEN`. This journey asserts the named credentials and endpoint overrides.

https://code.claude.com/docs/en/env-vars

https://code.claude.com/docs/en/authentication
