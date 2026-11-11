# Pitex verification map

The window path is `gtk`. The bridge path is `drive`. Both are `helpers/verify-pitex.mjs`. The window lists pi and Claude Code.

| Feature | File | What a pass looks like |
| --- | --- | --- |
| Codex | [codex.md](codex.md) | The Settings row is hidden. A stored `codex` is `gtk --provider stale-codex`. `drive --provider codex` still proves the adapter. |
| Claude Code | [claude.md](claude.md) | The window lists Claude Code. The child process is missing the billing and endpoint variables the parent had. |
| Grok | [grok.md](grok.md) | The Settings row is hidden. A stored `grok` is `gtk --provider stale-grok`. `drive --provider grok` still proves the adapter. |
| Antigravity | [agy.md](agy.md) | The window does not list Antigravity. A stored `agy` reads as pi and starts the built-in runtime. The window command is `gtk --provider stale-agy`. |

The fakes are the CLI stand-in. A pass here is not a pass against a vendor login.
