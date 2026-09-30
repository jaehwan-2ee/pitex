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

## Live-compile scheduling benchmark

```sh
bash autoresearch.sh
```

Requires Python 3, `rustc` (1.75+) and `swiftc` (6.3.3+); no package downloads,
TeX installation, display server, or network. Temporary executables are removed
on exit. The harness directly compiles and **executes both production
`LiveCompileScheduler` implementations** against the same input protocol;
every response must agree across Swift and Rust.

The fixed workload covers continuous typing, short bursts, hesitant typing,
and long IME composition at 120/600/1800 ms compiler service times, plus a
remote-style trace at 1600 ms with the existing 1500 ms debounce floor.
Local traces hold the configured idle delay at 700 ms. The virtual host uses
a 150 ms overdue-timer retry and 50 ms cancellation acknowledgement.
These are prescribed inputs, not measured compiler or operating-system times.

- **Primary:** `scheduler_edit_latency_ms` (lower is better), the mean of the
  13 per-trace mean edit-to-accepted-result latencies. Every edit is counted
  until an accepted completion covers its source revision, so starving the
  preview during typing cannot hide behind a fast final-keystroke measurement.
- **Secondary:** pooled edit p95/max, build starts/cancellations, accepted
  results, and virtual compiler busy/wasted milliseconds. `CASE` rows expose
  regressions hidden by the aggregate.
- **Guards:** no overlapping builds, no live start during IME composition,
  final revision eventually accepted, stale/invalidated/cancelled results
  rejected, manual ownership preserved, and no self-triggering rebuild loop.
  Failure exits nonzero; `METRIC` lines appear only after all checks pass.

This measures **scheduling policy**, not actual TeX speed, editor responsiveness,
PDF flicker, viewport preservation, or end-to-end presentation latency.
Compiler cost and the host timer model are fixed so policy comparisons are
reproducible. Do not tune the workload to improve the score; compare secondary
resource costs too. Renderer/build-pipeline changes need a separate real
engine/native-window benchmark before claiming a smoother preview.
[Texifier's live rendering uses its own built-in typesetter](https://www.texifier.com/docs/apps/typesetting/typesetters/texpadtex);
this benchmark is not a Texifier performance comparison.

[Back to the main README](../README.md)
