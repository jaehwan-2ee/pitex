# Building Pitex

Run these commands from the repository root unless noted otherwise.

## Prerequisites (build only)

- **macOS**: Xcode (current release), build via `Mac/Pitex.xcodeproj`
- **Linux** (Ubuntu 24.04):

  ```sh
  sudo apt-get install -y build-essential pkg-config libssl-dev \
      libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev \
      libvte-2.91-gtk4-dev libpoppler-glib-dev libwebkitgtk-6.0-dev \
      libteckit-dev libharfbuzz-dev libgraphite2-dev libicu-dev \
      libfreetype-dev libfontconfig-dev libpng-dev zlib1g-dev
  ```

  (the last two lines build the embedded preview helpers).

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

## Embedded preview engine

The editing-preview helpers are plain C/C++ built by
`PreviewEngine/Makefile`, separate from the app. Output always goes to
`BUILD_DIR`, never into the source tree:

```sh
make -C PreviewEngine -j BUILD_DIR=/tmp/pitex-preview-engine
# no teckit.pc? point at the library directly:
#   TECKIT_CFLAGS=-I<prefix>/include TECKIT_LIBS=<prefix>/lib/libTECkit.so.0
```

With TeX Live (`xelatex`/`kpsewhich`) and `pdftotext` on `PATH`, check
microtype compatibility, continuous edits, error recovery and an interrupted
worker before its first checkpoint. The package suite compares 27 fixtures
with real XeLaTeX, including Unicode math scripts, shaped-text copy/search,
Korean, tables, graphics, TikZ/PGFPlots, cross-file macros and existing BBLs:

```sh
PITEX_BIN=/tmp/pitex-preview-engine/bin python3 PreviewEngine/tools/regress-live-updates.py
PITEX_BIN=/tmp/pitex-preview-engine/bin python3 PreviewEngine/tools/regress-tex-compat.py
```

Dependencies (all dynamically linked): freetype2, harfbuzz, graphite2,
libpng, zlib, ICU, TECkit, and fontconfig on Linux. On macOS the Makefile
uses Homebrew when present, or `PKG_CONFIG_PATH` for a private prefix.

- **Running a dev build**: the app finds `pitex-preview` via
  `PITEX_PREVIEW_HELPER`, then next to its executable, then
  `../lib/pitex/` (Linux) or `Contents/Helpers/PreviewEngine/` (macOS). The
  driver starts the sibling `pitex-preview-xetex`:

  ```sh
  PITEX_PREVIEW_HELPER=/tmp/pitex-preview-engine/bin/pitex-preview cargo run -p pitex
  ```

- **Protocol**: [PreviewEngine/PROTOCOL.md](../PreviewEngine/PROTOCOL.md),
  the stdio JSON-lines contract between app and helper. It covers the
  display rules, final-PDF floor, SyncTeX identity and artifact release.
- **Provenance**: any edit under `PreviewEngine/` must be followed by
  regenerating and checking the manifest. Then audit the built helpers'
  link closure and symbols (Linux `ldd`/`dpkg`, macOS `otool`):

  ```sh
  python3 PreviewEngine/tools/provenance.py generate --upstream <texpresso checkout at the recorded commit>
  python3 PreviewEngine/tools/provenance.py verify
  bash PreviewEngine/tools/audit-linkage.sh /tmp/pitex-preview-engine/bin
  ```

- **macOS bundle**: `Tools/bundle-preview-engine-macos.sh <build dir>
  <Pitex.app>/Contents/Helpers` builds the helpers, copies their non-system
  dylibs into `PreviewEngine/lib/` (`@loader_path`), stages the license
  texts and ad-hoc signs everything. Private dependency prefixes are passed
  with `PKG_CONFIG_PATH`, `DEP_SEARCH_DIRS` and `DEP_LICENSE_DIR`. The
  release workflow builds TECkit and HarfBuzz from pinned tarballs.
- **Ubuntu 22.04** builds leave the `embedded-preview` feature off, and
  Windows has no helper; both keep the compiler live preview.

### Change note: macOS CoreText late-font ceiling

Keep the default safe CoreText path and its protective font barriers.
This delivery adds no optional engine optimization or backend cutover:
checkpoint-only replay is not guaranteed before late named platform-font loads.

The accepted B3 run used one **macOS 26.5.2 ARM64 VM**, **Xcode 27 RC /
Swift 6.4** tools, and an existing warm format cache whose before/after
inventory was unchanged. The 12-page `fontspec` + `unicode-math` fixture
used TeX-distribution file defaults; named Menlo 7 pt and Helvetica Neue
18 pt instances first appeared on page 12. Two unsaved edits changed page 3.

| Warm revision | New edit visible in hybrid PDF | All 12 pages coherent | Aux-converged complete PDF |
|---|---:|---:|---:|
| Generation 2 | 379 ms | 1177 ms | 1923 ms |
| Generation 3 | 381 ms | 1145 ms | 1904 ms |

Times are rounded to the nearest millisecond: helper update-write start →
OS-read receipt of the chunk completing the publication JSON. They exclude
the app's 120 ms coalescing window, editor/PDF display and physical-device
latency. One run supplies
no percentiles or general performance guarantee.

The first edit-visible PDFs contained **11 fresh pages and stale page 12**,
with no SyncTeX. Copied page 3 contained the new marker; fresh late-font
page 12 was established only at the coherent 12-page milestone.
Each warm edit observed three root launches: an initial fresh root,
**one** protective font-barrier restart, then an auxiliary convergence
rerun. These are not three checkpoint rollbacks or a barrier per font.
Exact checkpoint totals are unavailable (`null`); an empty decimation
observation list does not mean zero checkpoints.

All 13 publications reported zero TeX errors. Native PDFKit inspection
verified page counts, revision text, the Unicode integral and late-font
markers—not visual fidelity. No crash or watchdog event was observed in
this run only. First-page edits, preamble-only font loading and
all-file-font incrementality were not measured.

Retained evidence directory `coretext-626cvn3t/` holds the full run:
`events.jsonl` preserves OS-read timestamps, and per-generation driver
spans preserve restart/convergence attribution. The runner completed with
exit 0.
Frozen SHA-256 references:

```text
coretext-626cvn3t.tgz       0e8c29d342971215c242b662e1e4fe84ff29d52cc31b35d3e2ff5eb0bae745ea
summary.json              965304bee070ac6052dcdb6267a9561b8c983a9b7aa86be89094456969a4082f
fixture                   10bd7a4d97d226af192a807b732385ad492226aaefafab0976ac748b54685ba4
benchmark-task1-coretext.py d4dd1efd63b6fcebeb677cf4cf69858e8f17e9ef3155895d55eda847330a3de1
run-vm1-b3-retained.sh     c5197f57c208a32c968b5c1187f8f96a8a0f32a94fe3cd1f8cc707c0692846fa
pitex-preview             6c307ab9fdc4afaf984f111c459eb622542d57261c446f4acf6b920181e72f87
pitex-preview-xetex       3917357cfb00db6f46296e1cc6168ad2d2d4c0338355d61056509a25a9a1d0f6
```

[Back to the main README](../README.md)
