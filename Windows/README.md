# Pitex for Windows

The Windows build reuses the same Rust codebase as the Linux app: the
portable domain/feature crates are unchanged, `pitex-shell` runs on GTK 4 /
libadwaita through MSYS2, and `windows-platform` supplies the Windows
counterparts of the `app-ports` contracts (file capabilities, processes,
workspace opening, default-editor registration, logging, PDF coordinates,
clock, UUID, environment bundle).

## Layout

- `crates/windows-platform` — Windows adapters for the `app-ports` traits:
  `taskkill /T` process-tree termination, native Windows shell document opening,
  Explorer `reveal`, `HKCU\Software\Classes` default-editor registration,
  stderr logging, `std`-only clock/UUID.
- `crates/pitex-windows` — the `pitex` binary. It links `pitex-shell` with
  `modern-gtk` and without `vte` (there is no VTE-GTK4 on Windows), so the
  terminal pane embeds the `terminal-core` engine (ConPTY + DrawingArea) —
  same shape as the Ubuntu 22.04 build; an external terminal (`wt`, then
  conhost `cmd /k`) is only the fallback if the shell can't be spawned.
- `build.sh` — MSYS2 UCRT64 build + bundling script.

## Features

The Windows beta includes the same editor, project tree, TODOs, symbols,
Git and SSH tools, AI assistant, PDF preview, and SyncTeX as Ubuntu.
Its embedded terminal uses ConPTY with application mouse/focus reports,
IME input, selection, and clipboard paste.

Markdown and fast equation previews use Microsoft Edge WebView2 Evergreen.
Both renderers are bundled and work offline after the runtime is installed.
Equation hover/caret previews share the Ubuntu parser, document macros,
settings, cache, and exact project-context TeX renderer.

Live editing preview passes unsaved changes to the bundled `pitex-preview.exe`.
The helper runs your installed XeLaTeX against an isolated project snapshot,
then publishes a complete PDF and SyncTeX mapping. Original project files
stay untouched until you save. Obsolete compiler processes are cancelled.
This Windows helper uses complete compiler passes; the Unix helper uses
checkpointed XeTeX and can publish intermediate pages.

## Build prerequisites (MSYS2 UCRT64)

```sh
pacman -S --needed \
  mingw-w64-ucrt-x86_64-gcc \
  mingw-w64-ucrt-x86_64-rust \
  mingw-w64-ucrt-x86_64-pkgconf \
  mingw-w64-ucrt-x86_64-gtk4 \
  mingw-w64-ucrt-x86_64-libadwaita \
  mingw-w64-ucrt-x86_64-gtksourceview5 \
  mingw-w64-ucrt-x86_64-poppler \
  mingw-w64-ucrt-x86_64-librsvg \
  mingw-w64-ucrt-x86_64-adwaita-icon-theme \
  mingw-w64-ucrt-x86_64-hicolor-icon-theme
```

## Build and bundle

```sh
bash Windows/build.sh
```

produces `Windows/dist/pitex/`:

- `bin/pitex.exe`, `bin/pitex-preview.exe`, `WebView2Loader.dll`, plus every DLL the app and the GTK stack load
  (resolved with `ldd`, including dynamically loaded pixbuf/immodule/media
  modules).
- `share/icons/{Adwaita,hicolor}` — the `-symbolic` icon set the UI uses.
- `share/glib-2.0/schemas` — bundled GTK settings.
- `share/doc/pitex` — helper and MathJax/font license notices.
- `etc/fonts/fonts.conf` — a minimal config pointing at `C:\Windows\Fonts`;
  `main()` exports it as `FONTCONFIG_FILE` before GTK initializes because
  MSYS2's stock config names `/ucrt64/...` paths that do not exist off-MSYS2.

Zip `dist/pitex` to ship it; the folder is portable (no installer, no
registry writes — default-editor registration only happens when the user
asks for it in settings).

## Installer

`pitex.nsi` builds the recommended per-user installer
(`Pitex-*-windows-amd64-setup.exe`) from the same bundle:

```sh
makensis -DVERSION=1.0.2 -DOUTFILE=Pitex-setup.exe pitex.nsi   # from Windows/
```

It installs to `%LOCALAPPDATA%\Programs\Pitex` (no admin rights), adds
Start Menu/Desktop shortcuts, writes an Add/Remove Programs entry with an
uninstaller, and supports `/S` silent installs — which is exactly what the
in-app updater drives: it waits for the app to exit, runs `setup.exe /S`,
and relaunches the new exe. The release workflow produces both the
installer and the portable zip.

## Runtime requirements

- Windows 10 version 1809 or newer.
- Microsoft Edge WebView2 Evergreen for Markdown and equation previews.
  The loader DLL is bundled; install the [Evergreen runtime](https://developer.microsoft.com/microsoft-edge/webview2/) if necessary.
- TeX Live or MiKTeX with XeLaTeX for editing previews. MiKTeX must have
  the document packages installed without an interactive prompt. Final
  builds continue to use the compiler selected in settings.
- Bun, or Node.js and npm, for the Pi assistant.

## Notes

- TeX discovery checks TeX Live under `C:\texlive\<year>\bin\{windows,win32}`
  and MiKTeX under Program Files / `%LOCALAPPDATA%\Programs`.
- The Pi agent runtime installs into the app-local agent directory and is
  launched through Bun/Node like on the other platforms; script shims
  (`.cmd`/`.bat`/`.ps1`) route through `cmd`/`powershell`.
- Authentication (Pi `/login`) runs in the embedded terminal's shell —
  an external terminal (Windows Terminal, else conhost) is the fallback
  when no embedded shell is running.
