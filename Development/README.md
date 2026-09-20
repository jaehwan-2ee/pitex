# Building Pitex

Run these commands from the repository root unless noted otherwise.

## Prerequisites (build only)

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
  hicolor-icon-theme` (see [Windows README](../Windows/README.md)); NSIS for the installer.
- **Tooling**: Node.js 24+ (`Tools/` validation scripts), Swift 6.3.3+ for running
  `swift test` in `Packages/` on Linux.

## macOS

```sh
xcodebuild -project Mac/Pitex.xcodeproj -scheme Pitex -configuration Release build
```

## Linux

```sh
cd Linux
cargo build --workspace                          # full build (Ubuntu 24.04+)
cargo run -p pitex
cargo build -p pitex --no-default-features       # Ubuntu 22.04 compat build
```

See [Linux README](../Linux/README.md) for details.

## Windows

```sh
bash Windows/build.sh    # under MSYS2 UCRT64 — builds + bundles dist/pitex
```

See [Windows README](../Windows/README.md) for details.

[Back to the main README](../README.md)
