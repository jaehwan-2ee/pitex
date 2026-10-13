# Agent notes

Cloud agents are set up for the Ubuntu 24.04 Linux workspace. macOS (`xcodebuild`), Swift packages, and the Windows MSYS2 build are not installed here.

## Layout

- `Linux/` — Rust workspace. `pitex` is the binary; `pitex-shell` is the composition root. Edition 2021, MSRV 1.75, AGPL-3.0-or-later.
- `PreviewEngine/` — embedded editing-preview helpers. Separate Cargo workspace. `rust-toolchain.toml` pins `nightly-2026-10-04`. Build output goes to `BUILD_DIR`, not the source tree.
- `Windows/`, `Mac/`, `Packages/` — Windows and macOS apps, plus Swift packages `TexCore` and `TexApp`.
- `Tools/` — repository gates (Node) and native checkers (`Tools/Native`).
- `Toolchains.lock.json` — pins Node `24.15.0`, npm `12.0.2`, Swift `6.3.3`, and TeX Live `2023-debian`. It does not pin Rust.

## Commands

Run from the repository root unless noted. CI uses rustup **stable** for `Linux/` (`dtolnay/rust-toolchain@stable`). Use the rustup proxies on `PATH`; a standalone `cargo` will not switch to the PreviewEngine nightly.

```sh
# Build and test the Ubuntu 24.04 workspace (CI rust job).
cargo build --workspace --manifest-path Linux/Cargo.toml
cargo test --workspace --manifest-path Linux/Cargo.toml

# Static Linux gate. Does not run cargo test. Requires Node >= 24.
node Tools/verify-linux.mjs --skeleton
```

The CI `gates` job is the repository lint. It does not run `cargo fmt` or `cargo clippy`.

```sh
node Tools/verify-target-dag.mjs
node Tools/verify-rust-dag.mjs
node Tools/validate-xcodeproj.mjs
node Tools/check-markdown-preview.mjs
node Tools/check-bundled-skills.mjs
node Tools/check-linkage-audit.mjs
node Tools/generate-sbom.mjs
node Tools/verify-release-surface.mjs
bash Tools/check-audit-linkage-paths.sh
ruby Tools/verify-workflow-yaml.rb
bash Tools/check-nightly-publish.sh
bash Tools/check-nightly-gate.sh
```

The Linux workspace test in CI is not `--locked`. `Tools/Native` and `PreviewEngine` commands are `--locked`.

Ubuntu 22.04 has no VTE GTK4 package. That variant is `--no-default-features --features markdown-preview,equation-preview,embedded-preview`.

Ignored GTK tests need a display:

```sh
GSK_RENDERER=cairo xvfb-run -a dbus-run-session -- \
  cargo test -p pitex-shell --manifest-path Linux/Cargo.toml --lib agent_update_tests -- --ignored --test-threads=1
```

WebKitGTK's sandbox needs unprivileged user namespaces. On hosts that set `kernel.apparmor_restrict_unprivileged_userns=1`, CI turns that sysctl off for the ignored WebKit tests. Leave `WEBKIT_DISABLE_SANDBOX` unset.

Embedded helpers:

```sh
make -C PreviewEngine -j BUILD_DIR=/tmp/pitex-preview-engine
make -C PreviewEngine selftest BUILD_DIR=/tmp/pitex-preview-engine
```
