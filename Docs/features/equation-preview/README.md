# Equation preview

[All features](../../README.md) · [Main README](../../../README.md)

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
An equation without a closing delimiter has no preview.
An equation with a blank line before its closing delimiter also has no preview.

The parser collects these macro definitions in document order:

- `\newcommand`, `\renewcommand`, and `\providecommand`.
- `\DeclareMathOperator`.
- `\newenvironment` and `\renewenvironment`.

Definitions can come from `\input`, `\include`, `\subfile`, and `\import` files.
Project-local `\usepackage` style files can also supply definitions.
Unsaved changes in open tabs also apply.
The equation parser does not collect `\def` or `\let` definitions.

## Fast preview

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

## Exact TeX preview

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

## Equation preview settings

**Settings → Editor → Equation Preview** controls these functions:

- Enable the preview.
- Show the preview during text entry.
- Put the preview above or below the equation.
- Select the renderer.
- Set the delay to **Instant**, **80 ms**, or **150 ms**.

On macOS, the preview uses the system's **Increase Contrast** setting.
It does not change the equation or keyboard focus.
On Linux, equation preview needs WebKitGTK 6.0.
The Ubuntu 24.04 and Ubuntu 22.04 packages include this dependency.
Windows equation preview uses WebView2, with the same bundled MathJax renderer.
