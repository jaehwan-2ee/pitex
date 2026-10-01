# Building Pitex

Run these commands from the repository root unless noted otherwise.

## Prerequisites (build only)

- **macOS**: Xcode (current release), build via `Mac/Pitex.xcodeproj`
- **Linux** (Ubuntu 24.04):

  ```sh
  sudo apt-get install -y build-essential pkg-config libssl-dev \
      libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
      libvte-2.91-gtk4-dev libpoppler-glib-dev libwebkitgtk-6.0-dev
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

## Equation preview verification

On macOS, use the same Xcode toolchain for the app products and strict Swift 6
checker. From an existing trusted automation launcher with Accessibility and
post-event access, in an owned GUI session:

```sh
export DEVELOPER_DIR=/Applications/Xcode-27.0-RC.app/Contents/Developer
xcodebuild -project Mac/Pitex.xcodeproj -scheme Pitex -configuration Release \
  -derivedDataPath /tmp/pitex-equation-preview-dd build
python3 Tools/check-equation-preview-macos.py \
  /tmp/pitex-equation-preview-dd/Build/Products/Release \
  /tmp/pitex-equation-preview-evidence
```

Use a fresh evidence directory for each changed candidate; do not replay an
unchanged passed run. The launcher, not the ad-hoc checker app, performs genuine
HID motion and public AX reads/whitelisted native settings presses. Missing
trust or controls fails explicitly; do not change TCC, SIP or global preferences
to force a pass. TeX Live and the project's supported compiler must be available
for both distinct exact renders. `PITEX_CHECK_COMPILE_ONLY=1` verifies compilation
only, not native acceptance.

Full runs retain the exact signed checker app, fixture, three generated compiler
inputs and two logs, including on functional failure. Missing copies are reported
separately from the functional result. `CHECK_COMPLETE` proves the exercised
scenario, not unobserved platform paths. The ordinary full checker reports
increased contrast as unverified when that system preference is off; the
separate HC-only native acceptance below exercises it explicitly. The guarded
WebKit process-kill selector belongs only to the throwaway crash checker,
never production.

The scoped HC-only acceptance case `a637` on an owned disposable macOS VM
passed with the same production controller/WebView: ordinary dark → real
system-notification HC dark before any theme change, then HC light. Actual
rendered contrasts were 15.75:1 and 16.29:1 with opaque, visible ink.
The independent guardian restored and verified both public UI and fresh
`NSWorkspace` getters from original OFF/OFF → active ON/ON → final OFF/OFF;
the live checker also observed the ordinary palette returning without an
injected refresh. Only Increase Contrast ON/OFF was pressed. Reduce
Transparency coupled automatically on this VM and reverted with HC OFF,
so the authorized conditional RT-restoration press was not natively needed.
The HC case/outer numeric exit was 0; the guardian's numeric exit was not
separately recorded and must not be inferred from its `RESTORED` receipt.
This scoped test authority is not permission for general preference, TCC,
SIP or private-API changes.

Timeouts consume the same three-failure budget as process deaths; only persisted
enabled off-to-on resets it, not automatic attach, edits, themes or file-type
changes. A pending macOS JavaScript call can retain its replaced WebView/process
until it returns or the app quits; the product has no public forced-kill API.
Exact-render cancellation and per-request cleanup remain independent of the
20-second timeout. Its 4 KiB diagnostic tail is a display limit: shell-escape
child stdout is unbounded in bytes during that interval.

On Ubuntu 24.04 with WebKitGTK 6.0, a display (or Xvfb) and real `pdflatex`,
the new host-only regressions can run without replaying the old renderer
contract:

```sh
cd Linux
PITEX_EQUATION_NEW_HOST_PROOFS_ONLY=1 xvfb-run -a cargo test -p pitex-shell \
  --lib equation_preview::tests::renderer_page_contract -- --ignored --exact --nocapture
```

This exercises a real startup-ready timeout, unsaved root context under a
caller borrow, and canceled same-key TeX work followed by a real PDF/native
image. The timeout regression exposed synchronous WebKit cancellation
re-entering the shared JavaScript completion: both completion paths now take
the callback out and release its `RefCell` borrow before invoking it.
Host-specific helper-path shims or disabled WebKit sandboxes, when required
by a restricted test host, are functional-test qualifications—not shipping
settings or evidence that the production sandbox was exercised.

[Back to the main README](../README.md)
