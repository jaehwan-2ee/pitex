# Pitex

Native LaTeX environment for macOS, Linux, and Windows: editor, PDF preview
with SyncTeX, project-aware builds, and an embedded Pi-based AI assistant —
sharing one architecture across platforms.

- **macOS**: SwiftUI/AppKit/TextKit app (`Mac/`), driven by portable SwiftPM
  targets (`Packages/`) that compile and test on Linux.
- **Linux**: GTK4/libadwaita app (`Linux/`), a Cargo workspace that mirrors
  the Swift target DAG one crate per target (`Tools/rust-target-dag.json`).
- **Windows** *(beta)*: GTK4/libadwaita app (`Windows/`) built on the same
  Rust crates — `pitex-shell` runs on the MSYS2 GTK stack and
  `windows-platform` implements the Windows side of the `app-ports`
  contracts. Windows support is still beta-quality and may be unstable.
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

## Download

Prebuilt artifacts are published on the
[Releases](../../releases) page.

### macOS — Homebrew (recommended)

```sh
brew tap jaehwan-2ee/tap
brew install --cask pitex
```

The cask installs Pitex into `/Applications`. Apps installed this way also
update through `brew upgrade --cask pitex` — the in-app updater detects
the Caskroom install and defers to brew automatically.

Alternatively download `Pitex-*-macos-arm64.dmg`, open it, and drag
Pitex.app to `/Applications` — it is unsigned/ad-hoc-signed, so open via
right-click → Open on first launch (Gatekeeper).

### Linux — deb package

Pick the package matching your Ubuntu release and install it with apt
(pulls in the GTK dependencies automatically):

```sh
sudo apt install ./Pitex-*-ubuntu24.04-amd64.deb   # Ubuntu 24.04+
sudo apt install ./Pitex-*-ubuntu22.04-amd64.deb   # Ubuntu 22.04
```

The 22.04 package is a compatibility build: the terminal console shows
status output and hands interactive commands (e.g. Pi sign-in) to an
external terminal emulator, since 22.04 has no VTE-GTK4.

> **Note:** `apt` may print
> `N: Download is performed unsandboxed as root as file '…' couldn't be
> accessed by user '_apt'` when the `.deb` sits in a directory `_apt`
> can't read (e.g. `~/Downloads`). This is a harmless notice, not an
> error — the install still proceeds. If it bothers you, move the file
> to `/tmp` first.

### Windows — installer (recommended)

> **Note:** Windows support is **beta** — expect bugs and instability.
> Please report issues on the
> [issue tracker](../../issues).

Download `Pitex-*-windows-amd64-setup.exe` and double-click it — a
per-user NSIS installer puts Pitex in `%LOCALAPPDATA%\Programs\Pitex`
(no admin rights needed), adds Start Menu/Desktop shortcuts, and
registers an Add/Remove Programs entry.

`Pitex-*-windows-amd64.zip` is the portable alternative (`pitex.exe` +
GTK runtime): extract anywhere and run `pitex\bin\pitex.exe`. Like the
22.04 build, the terminal console hands interactive commands to an
external terminal (there is no VTE-GTK4 on Windows).

## Prerequisites

Requirements for running the released app — build-time dependencies live
under **Build** below.

### All platforms

- A TeX distribution for real document builds — TeX Live, BasicTeX
  (macOS), or MiKTeX (Windows). The editor and PDF preview work without
  one; builds cannot run.
- **Bun or Node.js + npm** — the AI assistant downloads its `pi` agent
  runtime into the app-local support folder on first launch. Without a
  package manager the assistant stays unavailable; everything else works.
- Internet access for the agent install, update checks, and downloads.

### macOS

- Apple Silicon Mac running **macOS 15+**
- Homebrew only if installing via the tap (see above)

### Linux

- **Ubuntu 24.04** or **Ubuntu 22.04**, matching the deb you install —
  the package's apt dependencies cover the GTK runtime libraries
- polkit (`pkexec`) or `sudo` for installing updates through Settings

### Windows

- **Windows 10** version 1803 or newer (the updater uses the in-box
  `curl` and `tar`)

## Build

### Prerequisites (build only)

- **macOS**: Xcode (current release), build via `Mac/Pitex.xcodeproj`
- **Linux** (Ubuntu 24.04):

  ```sh
  sudo apt-get install -y build-essential pkg-config libssl-dev \
      libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
      libvte-2.91-gtk4-dev libpoppler-glib-dev
  ```

  Ubuntu 22.04 (no VTE-GTK4 package exists there):

  ```sh
  sudo apt-get install -y build-essential pkg-config libssl-dev \
      libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
      libpoppler-glib-dev
  ```

  plus a Rust stable toolchain.
- **Windows**: MSYS2 with UCRT64 packages `gcc rust pkgconf gtk4
  libadwaita gtksourceview5 poppler adwaita-icon-theme
  hicolor-icon-theme` (see `Windows/README.md`); NSIS for the installer.
- **Tooling**: Node.js 24+ (`Tools/` gates), Swift 6.3.3+ for running
  `swift test` in `Packages/` on Linux.

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

## Updates

Pitex checks GitHub Releases for a newer tag, downloads the matching
platform asset, and installs it — the same flow on macOS, Linux, and
Windows.

- **Settings → Updates → Check for Updates** — queries the latest release
  and shows whether a newer version exists.
- **Install Update** (appears when an update is available) — downloads and
  installs:
  - *macOS*: replaces `/Applications/Pitex.app` from the DMG and relaunches.
    Homebrew-cask installs refresh the tap with `brew update`, run
    `brew upgrade --cask pitex`, and verify the installed app version before
    relaunching. A stale cask or failed upgrade shows an error so you can
    retry. DMG installs stage the new bundle before replacing the old one;
    without admin rights the DMG opens for a manual drag install.
  - *Linux*: runs `pkexec apt install` on the downloaded deb (polkit
    password prompt), falling back to `sudo apt install` in a terminal
    window. The in-app install verifies the installed package version before
    asking you to restart Pitex. Terminal installs must finish there first.
  - *Windows*: downloads the `*-setup.exe` installer and runs it silently
    (`/S`) after the app exits, then relaunches `pitex.exe`. A portable-zip
    install migrates to `%LOCALAPPDATA%\Programs\Pitex` automatically;
    zip-only releases fall back to a staged bundle swap. Both helpers wait
    for the app process to exit and check installation errors before
    relaunching. Existing custom installer locations are preserved; helper
    failures open an error log instead of silently relaunching the old app.
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
[PolyForm Shield License 1.0.0](LICENSE): free to use, modify, and
share, including commercially — but you may not use it to compete with
the project or its author.
