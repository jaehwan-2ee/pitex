# Pitex

Native LaTeX environment for macOS, Linux, and Windows: editor, PDF preview
with SyncTeX, project-aware builds, and an embedded Pi-based AI assistant —
sharing one architecture across platforms.

- **macOS**: SwiftUI/AppKit/TextKit app (`Mac/`), driven by portable SwiftPM
  targets (`Packages/`) that compile and test on Linux.
- **Linux**: GTK4/libadwaita app (`Linux/`), a Cargo workspace that mirrors
  the Swift target DAG one crate per target (`Tools/rust-target-dag.json`).
- **Windows**: GTK4/libadwaita app (`Windows/`) built on the same Rust crates
  — `pitex-shell` runs on the MSYS2 GTK stack and `windows-platform`
  implements the Windows side of the `app-ports` contracts.
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

- **Ubuntu 24.04** (full build — GTK 4.10+, libadwaita 1.4+, VTE-GTK4)
  or **Ubuntu 22.04** (compatibility build — GTK 4.6, libadwaita 1.1,
  no embedded terminal).
- System packages (Ubuntu 24.04):

  ```sh
  sudo apt-get install -y build-essential pkg-config libssl-dev \
      libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
      libvte-2.91-gtk4-dev libpoppler-glib-dev
  ```

- System packages (Ubuntu 22.04 — note there is no VTE-GTK4 package):

  ```sh
  sudo apt-get install -y build-essential pkg-config libssl-dev \
      libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
      libpoppler-glib-dev
  ```

- Rust stable toolchain

### Windows

- **Windows 10+** with **MSYS2** (UCRT64 environment)
- UCRT64 packages: `gcc rust pkgconf gtk4 libadwaita gtksourceview5 poppler
  adwaita-icon-theme hicolor-icon-theme` (see `Windows/README.md`)

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
cargo build --workspace                          # full build (Ubuntu 24.04+)
cargo run -p pitex
cargo build -p pitex --no-default-features       # Ubuntu 22.04 compat build
```

See `Linux/README.md` for details.

### Windows

```sh
bash Windows/build.sh    # under MSYS2 UCRT64 — builds + bundles dist/pitex
```

See `Windows/README.md` for details.

## Download

Prebuilt artifacts are published on the
[Releases](../../releases) page:

- `Pitex-*-macos-arm64.dmg` — unsigned/ad-hoc-signed macOS app; open via
  right-click → Open on first launch (Gatekeeper).
- `Pitex-*-ubuntu24.04-amd64.deb` — full Ubuntu 24.04+ package
  (embedded terminal via VTE-GTK4). Install with
  `sudo apt install ./Pitex-*-ubuntu24.04-amd64.deb`.
- `Pitex-*-ubuntu22.04-amd64.deb` — Ubuntu 22.04 compatibility package.
  The terminal console shows status output and hands interactive commands
  (e.g. Pi sign-in) to an external terminal emulator, since 22.04 has no
  VTE-GTK4.
- `Pitex-*-windows-amd64-setup.exe` — **recommended Windows install**:
  double-click the NSIS installer; it installs to
  `%LOCALAPPDATA%\Programs\Pitex` (no admin rights needed), adds Start
  Menu/Desktop shortcuts, and registers an Add/Remove Programs entry.
- `Pitex-*-windows-amd64.zip` — portable Windows bundle (`pitex.exe` + GTK
  runtime) for users who prefer no installer. Extract anywhere and run
  `pitex\bin\pitex.exe`. Like the 22.04 build, the terminal console hands
  interactive commands to an external terminal (there is no VTE-GTK4 on
  Windows).

### macOS via Homebrew

```sh
brew tap jaehwan-2ee/tap
brew install --cask pitex
```

The cask installs the same signed-ad-hoc DMG content into `/Applications`.
Apps installed this way update through `brew upgrade --cask pitex` — the
in-app updater detects the Caskroom install and defers to brew
automatically.

## Updates

Pitex checks GitHub Releases for a newer tag, downloads the matching
platform asset, and installs it — the same flow on macOS, Linux, and
Windows.

- **Settings → Updates → Check for Updates** — queries the latest release
  and shows whether a newer version exists.
- **Install Update** (appears when an update is available) — downloads and
  installs:
  - *macOS*: replaces `/Applications/Pitex.app` from the DMG and relaunches.
    Homebrew-cask installs go through `brew upgrade --cask pitex` instead;
    without admin rights the DMG opens for a manual drag install.
  - *Linux*: runs `pkexec apt install` on the downloaded deb (polkit
    password prompt), falling back to `sudo apt install` in a terminal
    window. Restart Pitex to finish.
  - *Windows*: downloads the `*-setup.exe` installer and runs it silently
    (`/S`) after the app exits, then relaunches `pitex.exe`. A portable-zip
    install migrates to `%LOCALAPPDATA%\Programs\Pitex` automatically;
    zip-only releases fall back to a staged bundle swap.
- **Automatically download and install updates** — when on, Pitex runs the
  same check→download→install pass on every launch. Stored as
  `pitex.pref.update.autoInstall` on all platforms.

Requirements: network access to `api.github.com` and the release CDN;
`curl` (preinstalled on macOS, Ubuntu, and Windows 10+); on Linux, polkit
or `sudo` for the package install.

## Development

- CI (`.github/workflows/ci.yml`) runs the repository gates, SwiftPM tests,
  the Linux Rust workspace tests (24.04 + 22.04 compat), and the Windows
  workspace build under MSYS2 UCRT64 on every PR.
- Cutting a release: tag `v*` and push — the release workflow builds the
  DMG, both deb variants, and the Windows zip, and attaches them to a
  GitHub Release.
- Report bugs or request features via Issues; changes land via PR.

## License

Pitex is source-available under the
[PolyForm Noncommercial License 1.0.0](LICENSE): free to use, modify, and
share for personal, educational, and other noncommercial purposes.
Commercial use requires a separate license from the author.
