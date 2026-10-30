# Pitex verification map

The window path is `gtk`. The bridge path is `drive`. Both are `helpers/verify-pitex.mjs`.

| Feature | File | What a pass looks like |
| --- | --- | --- |
| Codex | [codex.md](codex.md) | Settings shows Codex CLI, the assistant prints Hello, Approve reaches the CLI |
| Claude Code | [claude.md](claude.md) | The child process is missing the billing and endpoint variables the parent had |
| Grok | [grok.md](grok.md) | A file outside the project is refused, and the terminal cwd is the project |
| Antigravity | [agy.md](agy.md) | The window does not list Antigravity. A stored `agy` reads as pi and starts the built-in runtime. The window command is `gtk --provider stale-agy`. |

The fakes are the CLI stand-in. A pass here is not a pass against a vendor login.
