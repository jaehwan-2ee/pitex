# Pitex verification map

The window path is `gtk`. The bridge path is `drive`. Both are `helpers/verify-pitex.mjs`.

| Feature | File | What a pass looks like |
| --- | --- | --- |
| Codex | [codex.md](codex.md) | Settings shows Codex CLI, the assistant prints Hello, Approve reaches the CLI |
| Claude Code | [claude.md](claude.md) | The child process is missing the billing and endpoint variables the parent had |
| Grok | [grok.md](grok.md) | A file outside the project is refused, and the terminal cwd is the project |
| Antigravity | [agy.md](agy.md) | Approve, Deny, and Always allow return the permission-limit error and the CLI stdin has no approval |

The fakes are the CLI stand-in. A pass here is not a pass against a vendor login.
