# Vendored Claude Agent SDK

Pitex ships the JavaScript root bundle of `@anthropic-ai/claude-agent-sdk`
**0.3.312** (Claude Code **2.1.312**) and does not ship Anthropic's native
`claude` binary.

- `sdk.mjs` is the package root entry. That entry inlines its own copies of
  `zod` and `@modelcontextprotocol/sdk`, so the bridge does not install those
  packages.
- Optional platform packages (`@anthropic-ai/claude-agent-sdk-darwin-*`,
  `linux-*`, `win32-*`) are omitted on purpose. They contain Anthropic's
  Claude Code executable. The adapter sets `pathToClaudeCodeExecutable` to
  the `claude` binary already installed on the machine.
- The Mac app and the Linux `.deb` both copy `Mac/Resources/PitexAgent` as a
  folder. This directory is inside that tree, so the bundle is present
  without an `npm install` at launch.
- License: Anthropic PBC, all rights reserved. See `LICENSE.md` and
  <https://code.claude.com/docs/en/legal-and-compliance>. This is not MIT.
  The notice Pitex checks is `licenses/claude-agent-sdk-LICENSE.md`.

To refresh the pin, replace `sdk.mjs` from the published npm tarball of the
same package version, update `sdkMjsSha256` in `bridge/package.json`, and
keep the native platform packages out of this directory.
