# Pitex for Windows

The Windows build reuses the same Rust codebase as the Linux app: the
portable domain/feature crates are unchanged, `pitex-shell` runs on GTK 4 /
libadwaita through MSYS2, and `windows-platform` supplies the Windows
counterparts of the `app-ports` contracts (file capabilities, processes,
workspace opening, default-editor registration, logging, PDF coordinates,
clock, UUID, environment bundle).

## Layout

- `crates/windows-platform` — Windows adapters for the `app-ports` traits:
  `taskkill /T` process-tree termination, `cmd /c start` document opening,
  Explorer `reveal`, `HKCU\Software\Classes` default-editor registration,
  stderr logging, `std`-only clock/UUID.
- `crates/pitex-windows` — the `pitex` binary. It links `pitex-shell` with
  `modern-gtk` and without `vte` (there is no VTE-GTK4 on Windows), so the
  terminal pane embeds the `terminal-core` engine (ConPTY + DrawingArea) —
  same shape as the Ubuntu 22.04 build; an external terminal (`wt`, then
  conhost `cmd /k`) is only the fallback if the shell can't be spawned.
- `build.sh` — MSYS2 UCRT64 build + bundling script.

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
  mingw-w64-ucrt-x86_64-adwaita-icon-theme \
  mingw-w64-ucrt-x86_64-hicolor-icon-theme
```

## Build and bundle

```sh
bash Windows/build.sh
```

produces `Windows/dist/pitex/`:

- `bin/pitex.exe` plus every DLL the exe and the GTK stack load
  (resolved with `ldd`, including dynamically loaded pixbuf/immodule/media
  modules).
- `share/icons/{Adwaita,hicolor}` — the `-symbolic` icon set the UI uses.
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
makensis -DVERSION=1.0.4 -DOUTFILE=Pitex-setup.exe pitex.nsi   # from Windows/
```

It installs to `%LOCALAPPDATA%\Programs\Pitex` (no admin rights), adds
Start Menu/Desktop shortcuts, writes an Add/Remove Programs entry with an
uninstaller, and supports `/S` silent installs — which is exactly what the
in-app updater drives: it waits for the app to exit, runs `setup.exe /S`,
and relaunches the new exe. The release workflow produces both the
installer and the portable zip.

## Notes

- TeX discovery checks TeX Live under `C:\texlive\<year>\bin\{windows,win32}`
  and MiKTeX under Program Files / `%LOCALAPPDATA%\Programs`.
- The Pi agent runtime installs into the app-local agent directory and is
  launched through Bun/Node like on the other platforms; script shims
  (`.cmd`/`.bat`/`.ps1`) route through `cmd`/`powershell`.
- Authentication (Pi `/login`) runs in the embedded terminal's shell —
  an external terminal (Windows Terminal, else conhost) is the fallback
  when no embedded shell is running.
