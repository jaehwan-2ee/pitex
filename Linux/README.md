# Pitex for Linux (Rust)

Native Ubuntu port of the macOS Pitex app. The workspace mirrors the
Swift package DAG one crate per target; `pitex-shell` is the composition root
and `pitex` is the binary.

Two build variants:

- **Full build** (default, `modern-gtk` feature) — Ubuntu 24.04+:
  GTK 4.10+, libadwaita 1.4+, embedded VTE terminal.
- **Compatibility build** (`--no-default-features`) — Ubuntu 22.04:
  GTK 4.6, libadwaita 1.1, and no VTE-GTK4. The bottom-console terminal
  degrades to a read-only transcript; interactive commands (Pi sign-in,
  custom shell commands) are handed to an external terminal emulator
  (`kgx`, `gnome-terminal`, `konsole`, or `xterm`). Everything else —
  editor, PDF preview, SyncTeX, builds, the Pi assistant — is identical.

## System dependencies (Ubuntu 24.04)

```sh
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config libssl-dev \
    libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
    libvte-2.91-gtk4-dev libpoppler-glib-dev
```

## System dependencies (Ubuntu 22.04)

```sh
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config libssl-dev \
    libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
    libpoppler-glib-dev
```

Ubuntu 22.04 ships GTK 4.6 — there is no `libvte-2.91-gtk4-dev` package,
which is why the compat build drops the embedded terminal.

## Build & test

```sh
cargo build --workspace                      # all crates (full build)
cargo test  --workspace                      # unit + contract tests
cargo run   -p pitex                         # launch the GTK app
cargo build -p pitex --no-default-features   # Ubuntu 22.04 compat build
```

`gtk4`/`adw`/`sourceview5`/`vte4` crates need a running display only at
runtime; compilation and tests are headless-safe.

## Debian packages

`crates/pitex/Cargo.toml` carries `[package.metadata.deb]` — build with
[cargo-deb](https://github.com/kornelski/cargo-deb):

```sh
cargo deb --package pitex                      # full build → target/debian/
cargo deb --package pitex --variant ubuntu2204 # 22.04 compat build
```

The `ubuntu2204` variant sets `default-features = false`. Both variants use
`depends = "$auto"`, so `dpkg-shlibdeps` derives the library requirements
from the binary actually built — the compat deb should be produced on (or
in a container of) Ubuntu 22.04 for correct version bounds. The release
workflow does exactly that.

## Desktop integration

`packaging/dev.pitex.app.desktop` registers the app ID (`dev.pitex.app`)
and the LaTeX/BibTeX file associations that mirror the macOS `Info.plist`
document types. Install it to `~/.local/share/applications/` and run
`update-desktop-database` so Open-With activations reach the running
instance. `packaging/dev.pitex.app.png` is the matching icon — install it
to `~/.local/share/icons/hicolor/512x512/apps/dev.pitex.app.png`.

## Gate

`node ../Tools/verify-linux.mjs` runs the full gate including
`cargo test --workspace`; `--skeleton` runs the static checks only.
