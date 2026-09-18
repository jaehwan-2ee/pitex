# Pitex for Linux (Rust)

Native Ubuntu 24.04 port of the macOS Pitex app. The workspace mirrors the
Swift package DAG one crate per target; `pitex-shell` is the composition root
and `pitex` is the binary.

## System dependencies (Ubuntu 24.04)

```sh
sudo apt-get update
sudo apt-get install -y \
    build-essential pkg-config libssl-dev \
    libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
    libvte-2.91-gtk4-dev libpoppler-glib-dev
```

## Build & test

```sh
cargo build --workspace          # all crates
cargo test  --workspace          # unit + contract tests
cargo run   -p pitex             # launch the GTK app
```

`gtk4`/`adw`/`sourceview5`/`vte4` crates need a running display only at
runtime; compilation and tests are headless-safe.

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
