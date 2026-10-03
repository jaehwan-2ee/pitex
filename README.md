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
  *(No prebuilt Windows download is currently published; Windows (beta)
  support will return in a later release.)*

## Key features

### Open projects over SSH

**Open a folder on another Mac, Linux PC, or SSH server and work in the
native Pitex editor.** Choose **File → Open via SSH**, select a host from
`~/.ssh/config` or add one in **Settings → SSH**, then browse to the project.
Key/agent authentication must already work without a password prompt.

- **Save → sync** — manual saves, Save All, and **Settings → Editor → Auto
  Save** upload saved changes automatically. Auto Save uses the selected
  2, 5, or 10 second idle delay; there is no separate sync interval.
- **Remote builds and Git** run on the connected device. Build PDFs and
  SyncTeX metadata return to the local preview.
- **Pitex Agent** reads and edits the local project mirror; after an agent
  run, its file changes sync back over SSH. The agent process and its shell
  tools run locally; they do not run in the remote machine's environment.
- **Conflict protection** keeps files changed on both sides for review.
  Offline edits stay in the local mirror for a later sync. The window shows
  the device and sync state; returning to the window refreshes remote edits.
- **Multiple manuscripts** stay distinguishable: duplicate names in Project
  and editor tabs display their folder, such as `manuscript.tex (4_journal)`, and bibliography/PDF
  associations follow the selected manuscript's path.

The sidebar offers **Workspace / Project / TODOs**, with **Workspace** selected
by default. Workspace preserves the opened folder's hierarchy and highlights
the current file in place. Project shows the current manuscript's nested TeX,
BibTeX, and figure dependencies, with its compiled PDF separated below; an
active Markdown document appears as its own project. Switching tabs preserves
Workspace folder expansion. Drag the divider between document structure and
Workspace / Project / TODOs to adjust their heights. Workspace uses plain file
names within the folder tree and fills the available height.

Mirroring excludes VCS/dependency directories (including `.git`,
`node_modules`, and `.venv`) and files of 50 MB or larger. Open projects
inside the mirrored folder when asking the agent to read or edit files.


### Pi-based AI Assistant

The bottom console hosts an embedded **pi** coding agent with a native
chat surface: streamed transcript, thinking/tool-call rows, model picker,
and document-context attachments. The agent edits project files directly;
every proposed edit is revision-bound and can be applied or rejected.

The assistant ships with five built-in skills:

- [Humanizer](https://github.com/blader/humanizer) rewrites AI-sounding prose
  while preserving its meaning and the writer's voice.
- [SciSpace](https://scispace.com/) finds academic papers, gathers abstracts
  and DOI links, and helps organize literature reviews and BibTeX entries.
- **LaTeX Compile** compiles TeX projects using an available TeX runtime.
- **LaTeX Doctor** checks installed TeX tools and diagnoses missing components.
- **TeX Live Runtime Installer** detects existing TeX installations and helps
  set up a TeX Live runtime when needed.

### AI-based autocompletion

Pi suggests inline LaTeX continuations as you type. Press **Tab** to accept
or **Esc** to dismiss; typing or moving the caret also clears the suggestion.
Enable it in **Settings → AI → Inline autocompletion**. It uses the configured
Pi model and runs separately from the assistant conversation.

### Ref / cite helper

Get completion suggestions for `\ref{...}` and `\cite{...}` from labels in
the project's `.tex` files and citation keys in its `.bib` files. The sidebar
also lists labels and bibliography entries for navigation.

### TODOs

Track `% TODO:` and `% DONE:` comments across the project in the sidebar.
Jump to a task's source, add or rename tasks, mark them done, or delete them.

### Symbols Table

Browse LaTeX symbols by category from the editor's symbols button. Select a
glyph to insert its LaTeX command at the caret; hover to see the command.

### Markdown preview

`.md` and `.markdown` files open in the same editor, and the inspector
shows a live rendered preview instead of the PDF column — including
KaTeX math (`$…$`, `$$…$$`, `\[…\]`, `\(…\)`). On macOS and Linux Pitex
also registers as an "Open With" handler for Markdown files; *Settings →
Markdown → Use for .md Files* makes it the default app, like *Use for .tex
Files* does for TeX.

- **Live or on save** — the preview refreshes as you type; turn off
  *Settings → Markdown → Live preview* to render only when the document
  is saved.
- **Scroll sync** — scrolling the editor or the preview keeps the other
  at the same source line (*Settings → Markdown → Sync scrolling*).
- **Preview theme** — *Match app* follows the app appearance, or force
  *Light* / *Dark* (*Settings → Markdown → Preview theme*); font size is
  adjustable there too.
- **Raw HTML, no scripts** — inline HTML renders, but a strict
  content-security policy and navigation lockdown keep embedded scripts,
  event handlers, and `javascript:` links from executing.
- **Links** — `http(s)`/`mailto` links open in the browser; relative file
  links open in the default app. Executable files are refused, missing
  files ignored, and other schemes blocked.

Settings live under **Settings → Markdown** (right after the renamed
**TeX Compile** tab). On Linux the preview needs WebKitGTK 6.0
(`libwebkitgtk-6.0-4`, pulled in by the deb's dependencies). The Ubuntu
22.04 and Windows builds don't include the preview yet — they show a
"not available in this build" note instead.

### Equation preview

Point at an equation — or move the caret into one — and a rendered preview
appears above it (Overleaf-style), without building the document.
Hovering a different equation previews that one without moving the caret;
moving away, leaving the editor, or pressing **Esc** hides it.

- **What counts as math** — `$…$`, `$$…$$`, `\(…\)`, `\[…\]` and the
  `math`, `displaymath`, `equation[*]`, `align[*]`, `gather[*]`,
  `multline[*]`, `flalign[*]`, `alignat[*]`, `eqnarray[*]` environments,
  plus `aligned`, `gathered`, `split`, `cases`, `array` and the `matrix`
  family written on their own. Comments, escaped `\$`, `\verb`, verbatim-like
  environments and `$…$` inside `\text{…}` are read the way TeX reads them;
  an unfinished equation (a blank line before its closer) shows nothing.
- **Your macros** — `\newcommand`, `\renewcommand`, `\providecommand`,
  `\DeclareMathOperator`, `\newenvironment` and `\renewenvironment` apply in
  document order, including definitions from `\input`/`\include`/`\subfile`/
  `\import`ed files and project-local `\usepackage` style files (unsaved
  edits in open tabs count). `\def`/`\let` are not collected.
- **Fast preview** renders locally with a bundled, offline MathJax 4 (TeX
  font) inside a sandboxed web view — no network, no TeX run. It is labeled
  *Fast preview* and is an approximation: package-specific commands MathJax
  does not know show *Preview unavailable*.
- **Exact TeX preview** — *Build → Exact Equation Preview* (`⌥⌘E` /
  `Ctrl+Alt+E`) compiles just that equation with the project's own
  preamble, engine and shell-escape setting, in a private temporary folder
  that is deleted afterwards; it never touches the project's PDF or aux
  files. With *Renderer → Fast + TeX fallback* it also runs automatically
  for equations the fast preview cannot render, once you pause (never per
  keystroke).
  - Supported build commands: `pdflatex`, `xelatex`, `lualatex`, and
    `latexmk` only in its explicit PDF modes (`latexmk -pdf` /
    `-pdfxe` / `-pdflua`) plus an unquoted `-pdflatex=<engine>` override.
    Bare or `.latexmkrc`-driven `latexmk`, quoted/chained overrides
    (e.g. `-pdflatex="…"`), DVI/PS output, conflicting options, and
    custom or Tectonic commands report *Exact preview unavailable*
    rather than guess.

Settings live under **Settings → Editor → Equation Preview**: enable, preview
while typing, above/below, renderer, and delay (Instant / 80 ms / 150 ms).
On macOS the rendered preview updates live with the system's **Increase
Contrast** setting, without changing the equation or taking keyboard focus.
On Linux it needs WebKitGTK 6.0 (Ubuntu 24.04 package); the Ubuntu 22.04
build does not include it.

Fast rendering has a 5-second watchdog (10 seconds for renderer startup).
Three failures in one enabled session stop automatic recovery; turn
**Enable equation preview** off, then on, to try again. Successful renders,
editing, theme changes and file switches do not replenish that budget.
On macOS a truly hung WebKit script can retain its old view/process until
it returns or Pitex quits; replacing the view does not guarantee immediate
process cleanup.

Exact TeX jobs time out after 20 seconds. The error display keeps a 4 KiB
log tail, but stdout from project-enabled shell-escape children is not
byte-capped before that timeout. The log-tail limit is not a memory cap.

### Live compile

Toggle **Live Compile** in the toolbar (or **Settings → TeX Compile → Live
Compile**) and the preview follows your edits. **Preview backend** picks
how:

#### Embedded editing preview (default on macOS and Ubuntu 24.04)

A XeTeX engine derived from [TeXpresso](https://github.com/let-def/texpresso)
runs in a helper process and typesets exactly what the editor holds —
unsaved changes included — without saving your files or writing anything
into the project.

- The preview header says **Editing preview** (with *Updating editing
  preview…* while a pass runs) or **Final PDF** after a successful
  **Build**. Editing feedback streams independently of the final compiler;
  the helper resumes from a safe checkpoint when available. Native-font
  safety barriers can require a fresh engine process instead.
- Partial publications can combine fresh prefix pages with a stale tail
  from the previous pass; they do not mean the whole document is current.
- **Build** still creates the final PDF with the project compiler
  (pdflatex, xelatex, lualatex, latexmk, …). The final PDF is never covered
  by an older draft: it stays on screen until you edit again, and an edit
  typed while the build runs keeps the newer editing preview.
- **SyncTeX** works on an editing preview once it matches the editor text
  (saving does not break the match). A newer source revision or stale-tail
  pages disable mapping until a coherent publication matches the current
  text; the status line says why.
- Changes to files you have not opened (figures, `\input` files, `.bib`)
  are picked up when Pitex notices them: on Linux as soon as a project
  file changes on disk (new files after a rescan or assistant run); on
  macOS at the next edit, save, rescan or assistant run.
- Temporary files: preview PDFs live in a private per-session directory
  (Linux `$XDG_RUNTIME_DIR/pitex-preview/`, macOS `$TMPDIR/pitex-preview/`)
  that is removed when the preview stops or Pitex quits; leftovers from a
  crash are removed on the next start. The engine's TeX file index is
  cached in `~/.cache/pitex/preview-engine` (Linux) or
  `~/Library/Caches/Pitex/preview-engine` (macOS). The first launch on a
  machine generates engine formats; simultaneous first launches serialize
  on a lock, so the first-ever preview may take a few seconds.
- Known differences from the final build: the preview always uses XeTeX
  (pdfTeX/LuaTeX-only documents may differ or fail — switch to the
  compiler backend for those); fonts are embedded whole, so preview PDFs
  are larger; PostScript specials, PDF links/annotations/outlines and
  shell-escape (e.g. `minted`) are not previewed; vertical native-font
  text is shown horizontally.
- **macOS CoreText ceiling**: safe font barriers remain enabled. In one
  warm-format-cache, 12-page VM fixture with named platform fonts first
  used on page 12, helper update-write → publication-receipt times for two
  unsaved page-3 edits were 379/381 ms in hybrid PDFs (11 fresh pages plus
  one stale page, without SyncTeX). All 12 pages were fresh (coherent
  publication) after 1177/1145 ms and aux-converged after 1923/1904 ms,
  rounded to the nearest millisecond. Each edit required a fresh root,
  one font-barrier restart
  and one convergence rerun—not checkpoint-only replay. These are helper
  receipt observations on a macOS 26.5.2 ARM64 VM, not editor/display or
  physical-hardware guarantees. See the
  [conditioned measurement and limits](Development/README.md#change-note-macos-coretext-late-font-ceiling).
- Remote (SSH) projects, the Ubuntu 22.04 package and Windows use the
  compiler backend.

#### Compiler compatibility preview

Rebuilds with the project compiler as you edit. The first edit opens a
short coalescing window; the build then saves the latest open edited
files — there is no shadow copy — and compiles them.

- **Build delay** is configurable from 200 ms to 10 s (default 700 ms).
  Remote projects compile on the connected device and never go below a
  1.5 s effective delay.
  Further typing does not restart this window; an active build or IME
  composition still holds the build slot.
- **Isolated output** — live builds write under `.pitex-live/` inside
  the project root, so regular build artifacts, source discovery, and
  remote sync stay untouched. A failed or superseded run keeps the last
  good PDF on screen.
- **While you keep typing** — a running live build finishes and may update
  the preview with intermediate progress. New edits coalesce into one
  follow-up build rather than repeatedly cancelling useful work. A manual
  build takes priority and is never interrupted by typing; disabling live
  compile or changing the build context (including the resolved main
  document, not just an explicit pin) still retires live work.
- **Custom commands** must keep their output isolated too: add the
  `{outdir}` placeholder (e.g. `latexmk -pdf -outdir={outdir} {file}`)
  or the live run stops with an explanatory message. The shipped
  Tectonic preset is rewritten automatically; manual builds keep their
  existing behavior.
- **Follow cursor** (off by default) moves the PDF to the cursor after a
  successful live build. Forward/inverse SyncTeX keeps working against
  the isolated live artifact — the preview keeps your page, zoom and
  scroll position across rebuilds.

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
| Build / cancel build | `⌘B` or `⇧↩` (in editor) / `⌘.` | `Ctrl+B` or `Shift+Enter` (in editor) / `Ctrl+.` |
| Run custom command | `⌃⌘B` | `Ctrl+Alt+B` |
| Pin build target | `⌘P` | `Ctrl+P` |
| Send selection to assistant | `⌘⇧A` | `Ctrl+Shift+A` |
| Send assistant message | `⌘⏎` | — |
| Toggle line comment | `⌘/` | `Ctrl+/` |
| Save / Save As / Save All | `⌘S` / `⌘⇧S` / `⌘⌥S` | `Ctrl+S` / `Ctrl+Shift+S` / `Ctrl+Alt+S` |
| New / Open / Close | `⌘N` / `⌘O` / `⌘W` | `Ctrl+N` / `Ctrl+O` / `Ctrl+W` |
| Find | `⌘F` | `Ctrl+F` |
| Settings | — | `Ctrl+,` |
| Left Sidebar | `⌘T` | `Ctrl+T` |
| Right Sidebar | `⌘⌥P` | `Ctrl+Alt+P` |
| Bottom panel | — | `Ctrl+Shift+Y` |
| Exact equation preview | `⌥⌘E` | `Ctrl+Alt+E` |

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

### Windows

> **Note:** Windows support is **beta** — expect bugs and instability.
> Please report issues on the
> [issue tracker](../../issues).

Windows (beta) support is paused for now and will return in a later
release — no Windows download is currently published. Building from
source is described in [Windows/README.md](Windows/README.md).

## Prerequisites

Requirements for running the released app — build-time dependencies live
in the [build instructions](Development/README.md).

### All platforms

- A TeX distribution for real document builds — TeX Live, BasicTeX
  (macOS), or MiKTeX (Windows). The editor and PDF preview work without
  one; builds cannot run. The embedded editing preview reads the same
  TeX Live / MacTeX installation (XeTeX formats and fonts included).
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

See the [build instructions](Development/README.md) for platform dependencies and commands.

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
  - *Windows*: no Windows release is currently published, so the update
    check reports that no suitable download is available. When one is
    published again, it installs the `*-setup.exe` silently (`/S`) after
    the app exits, then relaunches `pitex.exe` (a portable-zip install
    migrates to `%LOCALAPPDATA%\Programs\Pitex`; the helpers wait for the
    process to exit and check for install errors before relaunching).
- **Automatically download and install updates** — when on, Pitex runs the
  same check→download→install pass on every launch. Stored as
  `pitex.pref.update.autoInstall` on all platforms.

Requirements: network access to `api.github.com` and the release CDN;
`curl` (preinstalled on macOS, Ubuntu, and Windows 10+); on Linux, polkit
or `sudo` for the package install.

## Development

- CI (`.github/workflows/ci.yml`) runs SwiftPM tests, Linux Rust workspace
  tests (24.04 + 22.04 compat), and the Windows workspace build under
  MSYS2 UCRT64 on every PR.
- Cutting a release: tag `v*` and push — the release workflow builds the
  DMG and both deb variants and attaches them to a GitHub Release. The
  Windows build runs on pull requests and manual runs only, not on tag
  pushes.
- Report bugs or request features via Issues; changes land via PR.

## License

Pitex is source-available under the
[PolyForm Shield License 1.0.0](LICENSE): free to use, modify, and
share, including commercially — but you may not use it to compete with
the project or its author.

The embedded editing preview helpers (`pitex-preview`,
`pitex-preview-xetex`) are built from [PreviewEngine/](PreviewEngine/),
which contains code from TeXpresso (MIT), XeTeX (SIL/MIT-style) and
Tectonic (MIT). They link dynamically to system or bundled libraries,
including TECkit (LGPL-2.1-or-later, used unmodified and replaceable — see
"Corresponding source" in `PreviewEngine/licenses/THIRD-PARTY.md`). Within
the helper binaries themselves, TeXpresso's GPL'd `dpx` component
(xdvipdfmx) and the MuPDF/SDL renderer were removed rather than imported;
dynamically loaded system libraries keep their own licenses.
See [PreviewEngine/PROVENANCE.md](PreviewEngine/PROVENANCE.md)
and [PreviewEngine/licenses/](PreviewEngine/licenses/); the deb installs
them under `/usr/share/doc/pitex/preview-engine/`, the macOS app under
`Contents/Resources/PreviewEngine/`.

The macOS PDF preview column is PDFKit's `PDFView` (a system framework); the
app bundles no third-party PDF viewer code. The copyleft components above
(PreviewEngine) keep their own notices and obligations.
