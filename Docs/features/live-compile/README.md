# Live compile

[All features](../../README.md) · [Main README](../../../README.md)

To enable live compile, use **Live Compile** in the toolbar or **Settings → TeX Compile → Live Compile**.
The **Preview backend** setting selects the engine for the editing preview.

## Embedded editing preview

This is the default backend on macOS, Ubuntu 24.04, Ubuntu 22.04, and Windows.
On macOS and Ubuntu, a XeTeX engine derived from [TeXpresso](https://github.com/let-def/texpresso) runs in a helper process.
Windows uses a bundled helper with the installed XeLaTeX compiler and an isolated project copy.
It compiles the text in the editor, including unsaved changes.
It does not save source files or write build files into the project.
Editor updates use a 100 ms coalescing interval.

The preview header shows **Editing preview** or **Updating editing preview…** during an engine pass.
After a manual **Build** with no errors, it shows **Final PDF**.
The editing preview runs independently of the project compiler.
The macOS and Ubuntu helper resumes from a safe checkpoint when one is available.
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
| Windows | `%TEMP%\pitex-preview-0\` | Uses the installed TeX distribution's formats and cache. |

Pitex deletes the session folder when the preview stops or the application closes.
After a crash, the next start removes remaining session folders.
The first preview makes engine formats.
A lock puts first starts that occur at the same time in sequence.
Thus, the first preview on a computer can have a delay of some seconds.

The Unix preview uses XeTeX with native PDF and build-service extensions.
It supports the tested PDF links, annotations, bookmarks, forms, and metadata.
Native vertical fonts use vertical metrics.
The helper also processes supported PostScript graphics inside the preview process.

Native services generate bibliography data and supported code highlighting inputs.
They process unsaved project changes without calling external bibliography or highlighting programs.
A project-supplied BBL file takes priority over generated bibliography data.
A frozen minted cache keeps the installed package's behavior.

The preview does not execute arbitrary shell-escape commands.
It reports unsupported operations in preview warnings.

The preview retains source characters for PDF search and copy with the default settings.
This includes shaped ligatures and Unicode math script variants.
Complete font programs can make preview PDFs larger.

See the [engine compatibility record](../../../PreviewEngine/COMPATIBILITY.md) for tested support and remaining differences.
The preview does not claim full XeLaTeX or pdfLaTeX equivalence.
Use **Build** to create the final PDF with the project compiler.

### macOS native-font measurements

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
See the [measurement conditions and limits](../../../Development/README.md#change-note-macos-coretext-late-font-ceiling).

SSH projects use the embedded preview on the local mirror where this engine is available.
Save an SSH project to build its final PDF on the remote device.
Windows also previews unsaved edits locally. Its bundled helper runs the installed XeLaTeX
against an isolated project copy and returns a complete PDF and SyncTeX mapping.
It coalesces updates and cancels obsolete compiler processes; it does not use the
Unix engine's checkpointed intermediate-page publications.
For SSH projects, use **Save** to build the final PDF on the remote device.

## Compiler compatibility preview

This backend uses the project compiler during text entry.
The first edit starts a short delay window.
At the end of this window, Pitex saves the latest edited files from open tabs and compiles them.
For local projects, this backend saves files in the project folder.
For SSH projects on macOS, it saves files in the local mirror.

**Build delay** accepts 200 ms to 10 seconds.
The default is 700 ms.
On macOS, this backend also works on an SSH project's local mirror.
It uses the TeX tools installed on the Mac.
It saves edited sources in the local mirror.
Only **Save**, **Save All**, or **Build** sends these changes to the remote device.
Closing a project or quitting also saves pending changes before closure.
Local preview work pauses while an SSH save commits its source revision.
Text entry does not start automatic remote builds.
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

## Forward and inverse SyncTeX

Forward sync moves from the source to the related PDF location.
The PDF can highlight this location.
Settings can disable the highlight.
Inverse sync moves from the PDF to the related source line.

| Action | macOS | Linux / Windows |
|---|---|---|
| Forward sync | `⌘`-click in the editor, or `⌘⇧J` | `Ctrl`-click in the editor, or `Ctrl+Shift+J` |
| Inverse sync | `⌘`-click in the PDF | `Ctrl`-click in the PDF |
