# Pitex

Native LaTeX environment for macOS and Linux: editor, PDF preview with
SyncTeX, project-aware builds, and an embedded Pi-based AI assistant —
sharing one architecture across platforms.

- **macOS**: SwiftUI/AppKit/TextKit app (`Mac/`), driven by portable SwiftPM
  targets (`Packages/`) that compile and test on Linux.
- **Linux**: GTK4/libadwaita app (`Linux/`), a Cargo workspace that mirrors
  the Swift target DAG one crate per target (`Tools/rust-target-dag.json`).
- **Gates**: `Tools/*.mjs` verify the target DAGs, Xcode project, release
  surface, and cross-language parity contract.

## Key features

### Pi-based AI Assistant

The bottom console hosts an embedded **pi** coding agent with a native
chat surface: streamed transcript, thinking/tool-call rows, model picker,
and document-context attachments. The agent edits project files directly;
every proposed edit is revision-bound and can be applied or rejected.

### Forward / Inverse SyncTeX

Source and PDF stay locked together:

- **Forward Sync** — jump from a source position to the matching PDF
  location (highlighted, toggleable in Settings):
  - macOS: `⌘`-click in the editor, or `⌘⇧J`
  - Linux: `Ctrl`-click in the editor, or `Ctrl+Shift+J`
- **Inverse Sync** — jump back to the exact source line:
  - macOS: `⌘`-click on the PDF
  - Linux: `Ctrl`-click on the PDF

### Shortcuts

| Action | macOS | Linux |
|---|---|---|
| Build / cancel build | `⌘B` / `⌘.` | `Ctrl+B` / `Ctrl+.` |
| Run custom command | `⌃⌘B` | `Ctrl+Alt+B` |
| Pin build target | `⌘P` | `Ctrl+P` |
| Send selection to assistant | `⌘⇧A` | `Ctrl+Shift+A` |
| Send assistant message | `⌘⏎` | — |
| Toggle line comment | `⌘/` | `Ctrl+/` |
| Save / Save As / Save All | `⌘S` / `⌘⇧S` / `⌘⌥S` | `Ctrl+S` / `Ctrl+Shift+S` / `Ctrl+Alt+S` |
| New / Open / Close | `⌘N` / `⌘O` / `⌘W` | `Ctrl+N` / `Ctrl+O` / `Ctrl+W` |
| Find | `⌘F` | `Ctrl+F` |
| Settings | — | `Ctrl+,` |
| Assistant pane | — | `Ctrl+T` |
| Inspector pane | — | `Ctrl+Alt+P` |
| Bottom panel | — | `Ctrl+Shift+Y` |

## Prerequisites

### macOS

- Apple Silicon Mac running **macOS 15+**
- **Xcode** (current release) — build via `Mac/Pitex.xcodeproj`
- A TeX distribution (TeX Live / BasicTeX) for real document builds

### Linux

- **Ubuntu 24.04** (or a distribution shipping GTK 4.10+)
- System packages:

  ```sh
  sudo apt-get install -y build-essential pkg-config libssl-dev \
      libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
      libvte-2.91-gtk4-dev libpoppler-glib-dev
  ```

- Rust stable toolchain

### Tooling

- Node.js 24+ (`Tools/` gates)
- Swift 6.3.3+ for running `swift test` in `Packages/` on Linux

## Build

### macOS

```sh
xcodebuild -project Mac/Pitex.xcodeproj -scheme Pitex -configuration Release build
```

### Linux

```sh
cargo build --workspace   # in Linux/
cargo run -p pitex
```

See `Linux/README.md` for details.

## Download

Prebuilt artifacts are published on the
[Releases](../../releases) page:

- `Pitex-*-macos-arm64.dmg` — unsigned/ad-hoc-signed macOS app; open via
  right-click → Open on first launch (Gatekeeper).
- `Pitex-*-linux-x86_64.AppImage` — Ubuntu 24.04+ AppImage with bundled GTK.

## Development

- CI (`.github/workflows/ci.yml`) runs the repository gates, SwiftPM tests,
  and the Rust workspace tests on every PR.
- Cutting a release: tag `v*` and push — the release workflow builds the
  DMG and AppImage and attaches them to a GitHub Release.
- Report bugs or request features via Issues; changes land via PR.

## License

Pitex is source-available under the
[PolyForm Noncommercial License 1.0.0](LICENSE): free to use, modify, and
share for personal, educational, and other noncommercial purposes.
Commercial use requires a separate license from the author.
