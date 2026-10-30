# Pitex Agent third-party notices

## Pi coding agent 1.4.0

Copyright (c) 2025 Mario Zechner.

Pi is licensed under the MIT License. The complete upstream notice and
license are included in [licenses/pi-MIT.txt](licenses/pi-MIT.txt).
Source: <https://github.com/earendil-works/pi/tree/v1.4.0>.

Pitex installs Pi into an app-managed runtime directory. These notices
identify the version pinned by Pitex; Pi's dependencies retain their own
licenses, and a later user-installed version may include updated notices.

## Humanizer 3.0.0

Copyright (c) 2025 Siqi Chen.

Humanizer is licensed under the MIT License. The complete upstream notice
and license are included in [skills/humanizer/LICENSE.md](skills/humanizer/LICENSE.md).
Source: <https://github.com/blader/humanizer/tree/9862685f575c65a8247f90369951df1b3416e3d6>.

These licenses apply to the named third-party components. Pitex's own
license does not replace their terms. The two K-Dense academic skills have
their notices in their respective skill directories and provenance in
`SKILL-PROVENANCE.json`.

## T3 Code 0.1.4

Copyright (c) 2026 T3 Tools Inc.

The Codex CLI adapter in `bridge/` follows the shape of T3 Code v0.1.4
(MIT): `packages/effect-codex-app-server` and
`apps/server/src/orchestration-v2/Adapters/CodexAdapterV2.ts`. Pitex does
not include T3 Code source and does not include its managed ChatGPT OAuth
broker. The MIT license is in
[licenses/t3code-MIT.txt](licenses/t3code-MIT.txt).
Source: <https://github.com/pingdotgg/t3code/tree/v0.1.4>.

## Claude Agent SDK 0.3.312

© Anthropic PBC. All rights reserved.

`@anthropic-ai/claude-agent-sdk` 0.3.312 is the JavaScript package Pitex
uses to drive the user's installed Claude Code CLI. Pitex vendors only
`bridge/vendor/claude-agent-sdk/sdk.mjs`. The native platform packages,
which contain Anthropic's `claude` binary, are not included. The CLI
itself stays the user's unmodified install. This license is not MIT. The
upstream notice is in
[licenses/claude-agent-sdk-LICENSE.md](licenses/claude-agent-sdk-LICENSE.md).
Source: <https://github.com/anthropics/claude-agent-sdk-typescript/tree/0.3.312>.
