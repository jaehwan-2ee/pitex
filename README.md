# Pitex

Pitex is a native LaTeX application for macOS, Linux, and Windows.
It has a text editor, a PDF preview with SyncTeX, project builds, and a Pi AI assistant.

| Platform | Application structure | Status |
|---|---|---|
| macOS | SwiftUI, AppKit, and TextKit in `Mac/`. Portable SwiftPM targets in `Packages/`. | Available for Apple Silicon. The SwiftPM targets also compile on Linux. |
| Linux | GTK4 and libadwaita in `Linux/`. One Rust crate for each Swift target. | Available for Ubuntu 24.04 and 22.04. `Tools/rust-target-dag.json` gives the target relationships. |
| Windows | GTK4 and libadwaita in `Windows/`, with the same Rust crates. | Beta support is on hold. No Windows download is available at this time. |

The Windows application uses the MSYS2 GTK stack.
Its `pitex-shell` crate supplies the shell.
Its `windows-platform` crate implements the Windows interfaces in `app-ports`.
The Windows beta can have errors and stability problems.

## Features

### Projects through SSH

Pitex can open a project on a different Mac, Linux computer, or SSH server.
The editor runs on your computer.

1. Make sure that SSH key or agent authentication works without a password prompt.
2. Select **File → Open via SSH**.
3. Select a host from `~/.ssh/config`.
4. Browse to the project folder.

To add a host, use **Settings → SSH**.

Manual saves, **Save All**, and **Auto Save** upload saved changes to the remote device.
**Settings → Editor → Auto Save** sets an idle delay of 2, 5, or 10 seconds.
The upload uses this delay.
There is no other sync interval.

Builds and Git commands run on the remote device.
Pitex transfers the PDF and SyncTeX data to the local preview.
The Pi assistant reads and edits the local project mirror.
After an assistant run, Pitex uploads its file changes through SSH.
The assistant process and its shell tools run on your computer.
They use your local environment.

If a file changes on the two devices, Pitex keeps the changes for your review.
Offline changes stay in the local mirror until the next sync.
The window shows the device and sync status.
When the window becomes active again, Pitex checks for remote changes.

Duplicate manuscript names show their folder in the Project sidebar and editor tabs.
For example, a tab can show `manuscript.tex (4_journal)`.
Bibliography and PDF associations use the selected manuscript's path.

The mirror excludes version-control and dependency folders, such as `.git`, `node_modules`, and `.venv`.
It also excludes files of 50 MB or more.
For assistant access, open the project in the mirror folder.

### Workspace sidebar

The sidebar has **Workspace**, **Project**, and **TODOs** tabs.
**Workspace** is the default tab.
It shows the opened folder's structure and highlights the current file.
File names stay in their folders.
The tree uses the available sidebar height.

**Project** shows the current manuscript's TeX, BibTeX, and figure dependencies in a nested tree.
It shows the compiled PDF independently below the tree.
An active Markdown document has its own project entry.
Tab changes keep the expanded Workspace folders.

To change the panel heights, drag the divider below the document structure.

### Pi AI assistant

The bottom console contains the **pi** coding agent and a native chat interface.
It shows streamed text, thinking and tool-call rows, a model selector, and document attachments.
The assistant can edit project files.
Each proposed edit applies to a specified file revision.
You can accept or reject the edit.

The assistant includes these five skills:

| Skill | Function |
|---|---|
| [Humanizer](https://github.com/blader/humanizer) | Changes AI-style text and keeps its meaning and the author's style. |
| [SciSpace](https://scispace.com/) | Finds academic papers, abstracts, and DOI links. Helps prepare literature reviews and BibTeX entries. |
| LaTeX Compile | Compiles a TeX project with an available TeX runtime. |
| LaTeX Doctor | Checks TeX tools and finds missing components. |
| TeX Live Runtime Installer | Finds existing TeX installations and helps install a TeX Live runtime when necessary. |

### Inline autocompletion

Pi can show a suggested LaTeX continuation at the caret.
This function uses the configured Pi model.
It runs independently of the assistant conversation.

1. Enable **Settings → AI → Inline autocompletion**.
2. To accept a suggestion, press **Tab**.
3. To remove a suggestion, press **Esc**.

A text change or caret movement also removes the suggestion.

### References, citations, and tasks

Pitex suggests labels for `\ref{...}` and citation keys for `\cite{...}`.
It reads labels from the project's `.tex` files and citation keys from its `.bib` files.
The sidebar also gives access to labels and bibliography entries.

The **TODOs** sidebar shows `% TODO:` and `% DONE:` comments from the project.
You can go to a task's source, add a task, rename it, mark it done, or delete it.

### Symbols Table

The editor's symbols button opens the **Symbols Table**.
The table shows LaTeX symbols in categories.
To insert a command at the caret, select its symbol.
To see a command, point at its symbol.

### Markdown preview

Pitex opens `.md` and `.markdown` files in the text editor.
The inspector shows a Markdown preview in place of the PDF column.
The preview includes KaTeX math with `$…$`, `$$…$$`, `\[…\]`, and `\(…\)`.

On macOS and Linux, Pitex is also an **Open With** application for Markdown files.
To make it the default, select **Settings → Markdown → Use for .md Files**.
The **Use for .tex Files** setting supplies the same function for TeX files.

| Setting | Function |
|---|---|
| **Live preview** | Updates the preview during text entry. If you disable it, the preview updates on save. |
| **Sync scrolling** | Keeps the editor and preview at the same source line during a scroll. |
| **Preview theme** | Uses **Match app**, **Light**, or **Dark**. **Match app** uses the application appearance. |
| Font size | Sets the preview font size. |

These settings are in **Settings → Markdown**, after the **TeX Compile** tab.

The preview shows raw HTML.
Its content-security policy blocks scripts, event handlers, and `javascript:` links.
HTTP, HTTPS, and `mailto` links open in the browser.
Relative file links open in the default application.
Pitex blocks executable files and other URL schemes.
It ignores links to missing files.

On Linux, this preview needs WebKitGTK 6.0 (`libwebkitgtk-6.0-4`).
The Ubuntu 24.04 and Ubuntu 22.04 packages include Markdown preview with this dependency.
Windows source builds use WebView2 for Markdown preview.
Windows downloads remain unavailable.

### Equation preview

When you point at an equation or put the caret in it, Pitex shows a preview above the equation.
A document build is not necessary.
You can point at a different equation without moving the caret.
The preview closes when the pointer moves away, the pointer leaves the editor, or you press **Esc**.

The equation parser accepts these delimiters and environments:

- `$…$`, `$$…$$`, `\(…\)`, and `\[…\]`.
- `math`, `displaymath`, `equation[*]`, `align[*]`, `gather[*]`, `multline[*]`, `flalign[*]`, `alignat[*]`, and `eqnarray[*]`.
- Separate `aligned`, `gathered`, `split`, `cases`, `array`, and `matrix` family environments.

The parser uses TeX rules for comments, escaped `\$`, `\verb`, and verbatim environments.
It also finds `$…$` in `\text{…}`.
An equation without a closing delimiter with a blank line before its closing delimiter has no preview.

The parser collects these macro definitions in document order:

- `\newcommand`, `\renewcommand`, and `\providecommand`.
- `\DeclareMathOperator`.
- `\newenvironment` and `\renewenvironment`.

Definitions can come from `\input`, `\include`, `\subfile`, and `\import` files.
Project-local `\usepackage` style files can also supply definitions.
Unsaved changes in open tabs also apply.
The equation parser does not collect `\def` or `\let` definitions.

#### Fast preview

**Fast preview** uses the included MathJax 4 renderer and TeX font in a sandboxed web view.
It uses no network connection or TeX process.
The result is an approximation.
If MathJax cannot interpret a package command, the preview shows **Preview unavailable**.

The renderer has a 5-second timeout and a 10-second startup timeout.
After three failures in one enabled session, automatic recovery stops.
To start a new recovery session, disable **Enable equation preview**.
Then enable it again.
Previews with no errors, text changes, theme changes, and file changes do not reset this failure count.

On macOS, a stalled WebKit script can keep its view and process until it returns or Pitex closes.
A replacement view does not always cause process removal immediately.

#### Exact TeX preview

To compile only the current equation, select **Build → Exact Equation Preview** (`⌥⌘E` on macOS, `Ctrl+Alt+E` on Linux).
This preview uses the project's preamble, engine, and shell-escape setting.
It uses a private temporary folder and deletes the folder after the job.
It does not change the project's PDF or auxiliary files.

**Renderer → Fast + TeX fallback** also starts this job if the fast renderer cannot show an equation.
The job starts after a pause in text entry.
It does not start for each keystroke.

Exact preview accepts these build commands:

- `pdflatex`, `xelatex`, and `lualatex`.
- `latexmk` with an explicit `-pdf`, `-pdfxe`, or `-pdflua` mode.
- An unquoted `-pdflatex=<engine>` override with these explicit `latexmk` modes.

These commands or options cause an **Exact preview unavailable** message:

- `latexmk` without an explicit PDF mode, including modes selected only through `.latexmkrc`.
- Quoted or chained overrides, such as `-pdflatex="…"`.
- DVI or PostScript output, or conflicting options.
- Custom commands and Tectonic commands.

Exact TeX jobs have a 20-second timeout.
The error display keeps the last 4 KiB of the log.
The timeout does not limit the output byte count from child processes with project-enabled shell escape.
The log limit does not limit memory use.

#### Equation preview settings

**Settings → Editor → Equation Preview** controls these functions:

- Enable the preview.
- Show the preview during text entry.
- Put the preview above or below the equation.
- Select the renderer.
- Set the delay to **Instant**, **80 ms**, or **150 ms**.

On macOS, the preview uses the system's **Increase Contrast** setting.
It does not change the equation or keyboard focus.
On Linux, equation preview needs WebKitGTK 6.0 in the Ubuntu 24.04 package.
The Ubuntu 22.04 build does not include this preview.

### Live compile

To enable live compile, use **Live Compile** in the toolbar or **Settings → TeX Compile → Live Compile**.
The **Preview backend** setting selects the engine for the editing preview.

#### Embedded editing preview

This is the default backend on macOS and Ubuntu 24.04.
A XeTeX engine derived from [TeXpresso](https://github.com/let-def/texpresso) runs in a helper process.
It compiles the text in the editor, including unsaved changes.
It does not save source files or write build files into the project.
Editor updates use a 100 ms coalescing interval.

The preview header shows **Editing preview** or **Updating editing preview…** during an engine pass.
After a manual **Build** with no errors, it shows **Final PDF**.
The editing preview runs independently of the project compiler.
The helper resumes from a safe checkpoint when one is available.
Native-font safety barriers can make a new engine process necessary.

A preview that is not complete can contain current pages at the start and older pages at the end.
A coherent publication contains only current pages.
A manual **Build** uses the project compiler, such as `pdflatex`, `xelatex`, `lualatex`, or `latexmk`.
An editing preview from an earlier source revision does not replace the final PDF.
A new edit can replace the final PDF.
An edit during a manual build keeps the editing preview for the new source revision.

SyncTeX works when the editing preview matches the text in the editor.
A save does not break this match.
New source changes or pages from an earlier pass disable mapping until a coherent publication matches the current text.
The status line gives the cause.

Pitex also checks changes to closed files, such as figures, `\input` files, and `.bib` files.
On Linux, it checks existing project files when they change on disk.
A rescan or assistant run finds new files.
On macOS, an edit, save, rescan, or assistant run supplies the next check.

Preview PDFs use private session folders:

| Platform | Temporary folder | TeX file index cache |
|---|---|---|
| Linux | `$XDG_RUNTIME_DIR/pitex-preview/` | `~/.cache/pitex/preview-engine` |
| macOS | `$TMPDIR/pitex-preview/` | `~/Library/Caches/Pitex/preview-engine` |

Pitex deletes the session folder when the preview stops or the application closes.
After a crash, the next start removes remaining session folders.
The first preview makes engine formats.
A lock puts first starts that occur at the same time in sequence.
Thus, the first preview on a computer can have a delay of some seconds.

The embedded preview always uses XeTeX.
For documents that need pdfTeX or LuaTeX commands, select the compiler backend.
The preview embeds complete fonts, which can make the PDF larger.

With the default settings, the preview keeps source characters for PDF search and copy.
This includes shaped ligatures and Unicode math script variants.

The preview reads existing `.bbl` files for bibliography data.
For new bibliography data, run BibTeX or Biber through your project build workflow.

The preview does not run shell-escape commands, such as the commands that generate `minted` output.
PostScript specials, PDF links, annotations, and outlines do not show in the preview.
Vertical text with native fonts shows horizontally.

##### macOS native-font measurements

CoreText safety barriers stay enabled.
A 12-page test document first used named platform fonts on page 12.
The format cache was warm.
Two unsaved edits on page 3 gave these helper measurements:

| Publication | Edit 1 | Edit 2 | PDF state |
|---|---|---|---|
| Not complete | 379 ms | 381 ms | 11 current pages and one older page. No SyncTeX mapping. |
| Coherent | 1177 ms | 1145 ms | All 12 pages current. |
| Auxiliary convergence | 1923 ms | 1904 ms | Auxiliary data stable after one more pass. |

Each edit needed a new root process, one font-barrier restart, and one convergence pass.
These times measure the interval from helper update write to publication receipt, rounded to the nearest millisecond.
The test used a macOS 26.5.2 ARM64 virtual machine.
These results do not measure editor display time or guarantee performance on physical hardware.
See the [measurement conditions and limits](Development/README.md#change-note-macos-coretext-late-font-ceiling).

SSH projects, Ubuntu 22.04, and Windows use the compiler backend.

#### Compiler compatibility preview

This backend uses the project compiler during text entry.
The first edit starts a short delay window.
At the end of this window, Pitex saves the latest edited files from open tabs and compiles them.
This backend does not use a source copy.

**Build delay** accepts 200 ms to 10 seconds.
The default is 700 ms.
SSH projects compile on the remote device with a minimum delay of 1.5 seconds.
Further text entry does not restart the delay window.
An active build or IME composition holds the next build.

Live builds write output into `.pitex-live/` in the project root.
Regular build files, source discovery, and remote sync do not include live output.
A run with errors or a run for an earlier source revision keeps the last good PDF on screen.

An active live build completes before the next live build starts.
New edits cause one more build with the latest changes.
A manual build starts before the next live build.
Text entry does not stop a manual build.
If you disable live compile or change the build context, Pitex stops live work.
The build context includes the resolved main document and an explicit target pin.

For a custom live command, include the `{outdir}` placeholder for the live output folder.
For example:

```sh
latexmk -pdf -outdir={outdir} {file}
```

Without this placeholder, the live run stops and shows the cause.
Pitex changes the included Tectonic preset automatically for live output.
Manual builds keep their existing command behavior.

**Follow cursor** is off by default.
If you enable it, a live build with no errors moves the PDF to the caret's source location.
Forward and inverse SyncTeX use the live artifact.
The preview keeps the page, zoom, and scroll position across builds.

### Forward and inverse SyncTeX

Forward sync moves from the source to the related PDF location.
The PDF can highlight this location.
Settings can disable the highlight.
Inverse sync moves from the PDF to the related source line.

| Action | macOS | Linux |
|---|---|---|
| Forward sync | `⌘`-click in the editor, or `⌘⇧J` | `Ctrl`-click in the editor, or `Ctrl+Shift+J` |
| Inverse sync | `⌘`-click in the PDF | `Ctrl`-click in the PDF |

### Keyboard shortcuts

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

## Download and installation

The [Releases](../../releases) page supplies the application packages.

### macOS

The recommended installation method uses Homebrew.

1. Add the tap:

   ```sh
   brew tap jaehwan-2ee/tap
   ```

2. Install the application:

   ```sh
   brew install --cask pitex
   ```

Homebrew installs Pitex in `/Applications`.
The command `brew upgrade --cask pitex` installs updates.
The application updater finds a Caskroom installation and uses Homebrew automatically.

For a DMG installation:

1. Download `Pitex-*-macos-arm64.dmg`.
2. Open the DMG.
3. Drag `Pitex.app` to `/Applications`.
4. On the first start, right-click the application.
5. Select **Open**.

The application uses ad-hoc signing and has no distribution signature.
Gatekeeper can make this first-start procedure necessary.

### Linux

Select the package for your Ubuntu version.
Install it with the applicable command:

```sh
sudo apt install ./Pitex-*-ubuntu24.04-amd64.deb   # Ubuntu 24.04+
sudo apt install ./Pitex-*-ubuntu22.04-amd64.deb   # Ubuntu 22.04
```

APT also installs the necessary GTK libraries.
The Ubuntu 22.04 package uses a compatibility console because VTE-GTK4 is unavailable there.
This console shows status output.
Interactive commands, such as Pi sign-in, use an external terminal emulator.

APT can show an unsandboxed-download notice if the `_apt` user cannot read the package folder.
For example, this can occur in `~/Downloads`.
The notice does not stop the installation.
To prevent the notice, move the package to `/tmp` before installation.

### Windows

Windows beta support is on hold until a subsequent release.
No Windows package is available at this time.
The beta can have errors and stability problems.
Use the [issue tracker](../../issues) to report problems.
See [Windows build instructions](Windows/README.md) for a source build.

## Runtime requirements

These requirements apply to the released application.
See the [build instructions](Development/README.md) for development dependencies.

| Requirement | Function |
|---|---|
| TeX Live, BasicTeX on macOS, or MiKTeX on Windows | Necessary for document builds. The editor and PDF preview can operate without a TeX distribution. |
| TeX Live or MacTeX, with XeTeX formats and fonts | Supplies TeX files for the embedded editing preview. |
| Bun, or Node.js with npm | Installs the Pi agent runtime in the application's support folder on the first start. |
| Internet access | Necessary for agent installation, update checks, and downloads. |

Without Bun or Node.js with npm, the assistant is unavailable.
The other application functions can operate.

| Platform | Other requirements |
|---|---|
| macOS | Apple Silicon and macOS 15 or later. Homebrew is necessary only for a tap installation. |
| Linux | Ubuntu 24.04 or 22.04, with the matching package. APT supplies the GTK runtime dependencies. Updates need polkit (`pkexec`) or `sudo`. |
| Windows | Windows 10 version 1803 or later. The updater uses the included `curl` and `tar` tools. |

## Build from source

See the [build instructions](Development/README.md) for platform dependencies and build commands.

## Application updates

Pitex checks GitHub Releases for a release tag with a higher version.
It downloads the asset for the current platform and installs the update.

1. Select **Settings → Updates → Check for Updates**.
2. If an update is available, select **Install Update**.

The update procedure depends on the installation method:

| Installation | Update procedure |
|---|---|
| macOS, Homebrew | Updates the tap with `brew update`. Runs `brew upgrade --cask pitex`. Checks the installed application version before restart. |
| macOS, DMG | Prepares the new bundle before replacement of `/Applications/Pitex.app`. Then restarts Pitex. Without administrator access, opens the DMG for manual installation. |
| Linux | Runs `pkexec apt install`. If necessary, opens a terminal for `sudo apt install`. Checks the package version before a restart request. |
| Windows | Reports that no applicable download is available while Windows releases are on hold. |

A stale Homebrew cask or upgrade with errors causes an error message.
You can try the update again.
Complete a Linux installation in its external terminal before restart.

When Windows releases become available, the updater uses the `*-setup.exe` asset.
It waits for Pitex to close, runs the installer with `/S`, and starts `pitex.exe` again.
A portable ZIP installation moves to `%LOCALAPPDATA%\Programs\Pitex`.
The helpers wait for process exit and check installation errors before restart.

**Automatically download and install updates** starts the same update procedure at each application start.
All platforms store this preference as `pitex.pref.update.autoInstall`.

Updates need network access to `api.github.com` and the release CDN.
They also need `curl`, which macOS, Ubuntu, and Windows 10 or later include.
Linux package installation needs polkit or `sudo`.

## Development and contributions

CI (`.github/workflows/ci.yml`) runs these checks for each pull request:

- SwiftPM tests.
- Linux Rust workspace tests for Ubuntu 24.04 and the Ubuntu 22.04 compatibility build.
- A Windows workspace build with MSYS2 UCRT64.

A pushed `v*` tag starts the release workflow.
The workflow builds the DMG and the two Ubuntu packages and attaches them to a GitHub Release.
Windows builds run only for pull requests and manual workflow runs.
They do not run for release tags.

Use [Issues](../../issues) for problems and feature requests.
Use pull requests for code changes.

## License

Pitex source code is available with the [PolyForm Shield License 1.0.0](LICENSE).
You can use, modify, and distribute Pitex, including commercial use.
You must not use Pitex to compete with this project or its author.

The embedded helpers are `pitex-preview` and `pitex-preview-xetex`.
Their source is in [PreviewEngine/](PreviewEngine/).
It includes TeXpresso (MIT), XeTeX (SIL/MIT-style), and Tectonic (MIT) code.
The helpers link dynamically to system or included libraries.

TECkit uses LGPL-2.1-or-later.
The helpers use it without modification, and you can replace the library.
See **Corresponding source** in [THIRD-PARTY.md](PreviewEngine/licenses/THIRD-PARTY.md).
The helper binaries contain no TeXpresso GPL `dpx` component (xdvipdfmx) or MuPDF/SDL renderer.
Dynamically loaded system libraries keep their own licenses.

See [PROVENANCE.md](PreviewEngine/PROVENANCE.md) and the [license notices](PreviewEngine/licenses/).
The Ubuntu package installs the notices in `/usr/share/doc/pitex/preview-engine/`.
The macOS application includes them in `Contents/Resources/PreviewEngine/`.

The macOS PDF column uses the system PDFKit `PDFView`.
Pitex includes no third-party PDF viewer code.
Copyleft components keep their own notices and license obligations.
