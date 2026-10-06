# Embedded preview engine provenance and dependencies

This directory builds the two helpers behind Pitex's **editing preview**:

| Helper | Role | Origin |
|---|---|---|
| `pitex-preview-xetex` | XeTeX engine that checkpoints itself with `fork()` and does all file I/O through the driver | TeXpresso's modified XeTeX (itself the Tectonic port of XeTeX), with the changes listed below |
| `pitex-preview` | Session driver: file overrides from unsaved editor buffers, rollback to checkpoints, XDV→PDF conversion, atomic publication | TeXpresso's session/rollback logic ported off MuPDF, plus a Pitex-authored XDV→PDF writer |

Upstream: <https://github.com/let-def/texpresso> at commit
`e8df7709077b2f86f6e16e6c86ceefb86de06f8d`. `provenance.json` records, for
every file here, whether it was imported (upstream path, upstream and
current SHA-256, modified flag, license notice) or written for Pitex.
`preview-provenance verify` checks that record; `tools/audit-linkage.sh`
additionally inspects the built helpers' shared-library closure and symbols.
This documents the technical boundary; it is not a legal opinion.

This record describes the source checkout. [COMPATIBILITY.md](COMPATIBILITY.md)
lists the checkout's implemented interfaces, tests and policy limits.
Pitex-authored files use the repository `LICENSE` (GNU AGPL 3.0 or later).
Imported files keep their own licenses, and third-party notices remain in
effect.

Final documents are **not** produced here: manual builds keep using the
project's pdflatex/xelatex/lualatex/latexmk configuration. Preview PDFs are
temporary artifacts in a per-session directory and say so in their metadata
(`/Producer (Pitex embedded editing preview)`).

## Rust migration

The XeTeX core, checkpoint workers, session driver, VFS, XDV/PDF writer,
image/font parsers, shared buffers, and boundary regressions are Rust.
The original C implementations are no longer build inputs or present in this
directory or this repository's history. The former Pitex repository has
them at commit `9619780acb7c6a6e7fdaa2d52254a1087fb7f73d` (v2.2.1).

[migration.json](migration.json) records each source C SHA-256 and its Rust translation.
C2Rust 0.22.1 generated the initial code. The translation preserves the
original C ABI, allocation model, unsafe pointer operations, and fork-based
checkpoint behavior. This language migration does not claim to make the
engine memory-safe. The macOS and Linux source trees preserve the respective
platform headers and engine branches. The generated and validated ABI
targets are `aarch64-apple-darwin` (macOS) and
`x86_64-unknown-linux-gnu` (Linux with glibc). The build rejects other
target triples before it compiles or links any engine code.
The exported variadic logging calls
use Rust's C variadic ABI; the package pins nightly-2026-10-04. Builds do not
require C2Rust.

Five later manual translations move OpenType math, layout algorithms,
FreeType metrics, Fontconfig name handling and macOS font metrics into Rust.
They retain the complete original MIT notices from SIL International,
Jonathan Kew, Khaled Hosny and Jiang Jiang, as applicable.
`migration.json` records each source revision, original SHA-256, translated
module and remaining native boundary.

| Source | Rust module | Native source before → current |
|---|---|---|
| `xetex-XeTeXOTMath.cpp` | [opentype_math.rs](src/shared/opentype_math.rs) | C++ file removed |
| `xetex-XeTeXLayoutInterface.cpp` | [font_layout.rs](src/shared/font_layout.rs) | 1,166 → 104 lines |
| `xetex-XeTeXFontInst.cpp` | [font_freetype.rs](src/shared/font_freetype.rs) | 561 → 99 lines |
| `xetex-XeTeXFontMgr_FC.cpp` | [fontconfig_names.rs](src/shared/fontconfig_names.rs) | 386 → 133 lines |
| `xetex-XeTeXFontInst_Mac.cpp` | [font_macos.rs](src/shared/font_macos.rs) | 176 → 59 lines |

The recorded original source hashes are:

| Source | SHA-256 |
|---|---|
| OpenType math | `a58bf996f4f23b14d2d52dd1e735d9c0785dab782aef59d6f7bd335c32cdf915` |
| Layout interface | `0887e7bf3ce77c9aee272976f5c7d98ae87309f2e1d6332b9efdad32ad575239` |
| FreeType font instance | `d6487df22bb4e62e3cbe97e004c539e6abad09f00eb3f320bfbfccdca0d8a315` |
| Fontconfig font manager | `bffffa93891c0f81fe5760de09e4a8fb224dddba42bf84137ef276108bc94cd9` |
| macOS font instance | `4e3e0e5b52d70c075769922c8480e449daaf039a56534d67c6128cd1094a7e81` |

The remaining C++ bridges construct and access opaque `FontInst`/`FontMgr`
classes, read SDK records and manage platform resources.
The base `xetex-XeTeXFontMgr.cpp` and macOS
`xetex-XeTeXFontMgr_Mac.mm` also remain native code.
Cargo compiles these layout/platform translation units behind their C ABI.
It compiles no C engine, driver, parser or self-test.
Headers in `xetex/` describe the boundary.
The font libraries and replaceable TECkit library remain shared dependencies.

Outside these helpers, Linux's former 110-line C IBus compatibility shim is
removed. The original app-local implementation is now
[ibus_compat.rs](../Linux/crates/pitex/src/ibus_compat.rs).
It calls the installed IBus/GLib libraries and keeps their platform ABI.

The migration also adapts the modern Rust `VaList` API, keeps SDK ctype
inline functions local to their Rust module, uses the input's native `f64`
for buffer finite-value checks, and represents the two CoreFoundation
constant strings with the verified native `__cfstring` ABI. `src/bin/`
selects the platform source modules and keeps both helper names and the
stdio/checkpoint protocols unchanged.

## What was imported

The paths below identify the original imports. Historical `.c` paths map
to platform Rust modules through `migration.json`; retained headers keep
their upstream paths and notices.

- `xetex/engine`, `xetex/layout`, `xetex/main`, `xetex/include` ←
  `src/engine/{engine,layout,main,include}` — MIT (Tectonic Project), MIT/X11
  (SIL International XeTeX code), MIT (TeXpresso), public domain
  (`xetex-texmfmp.c`, `picohash.h`). Per-file notices are kept unchanged.
- `xetex/common` ← `src/common`, `src/include` — TeXpresso (MIT, root
  `LICENSE` copied to `licenses/TeXpresso-LICENSE.txt`): TeX Live file
  lookup and cache paths.
- `driver/engine_tex.c|h`, `state.c|h`, `fs.c`, `sprotocol.c|h`,
  `myabort.c|h` ← `src/frontend/…` — TeXpresso (MIT): process/snapshot
  management, trace/fence rollback, virtual file system, engine protocol.

## Changes to imported code

- **GPL code removed.** `src/engine/dpx` (xdvipdfmx, GPL-2.0-or-later) is not
  imported. The engine used it only to measure included pictures
  (`xetex-pic.c`) and to initialise a PDF file cache (`xetex-ini.c`);
  those calls now use `shared/pdfread.c` and `shared/imginfo.c`, written for
  Pitex from the PDF, PNG, JPEG/JFIF/Exif and BMP specifications.
- **TECkit not vendored.** The `teckit-*` sources (LGPL-2.1+/CPL-0.5+) are not
  imported; the engine includes the distribution's `<teckit/TECkit_Engine.h>`
  and links the shared `libTECkit` (see `licenses/THIRD-PARTY.md` for the
  replacement instructions). `fontspec`'s `Mapping=tex-text` keeps working.
- **MuPDF/SDL removed.** TeXpresso renders XDV pages into MuPDF display lists
  in an SDL window. That renderer (`src/frontend/renderer.c`, `incdvi.c`,
  `src/dvi/*`, all built on MuPDF, AGPL-3.0/commercial) is not imported;
  `driver/tbuf.c` replaces `fz_buffer`, and the driver writes real PDF files
  instead.
- `driver/engine_tex.c`: generic engine class and editor/SDL notifications
  dropped; incremental XDV page index (`driver/xdv.c`) instead of `incdvi`;
  watchdog/kill hooks; engine sockets marked close-on-exec; the aux-file
  convergence stash is kept across rollbacks (so an edit does not force a
  full rerun when the `.aux` output is unchanged) and on-disk `.aux` equality
  counts as converged. Dead workers at a rollback boundary are discarded,
  including workers interrupted before their first trace/checkpoint.
- **Paragraph-token compatibility** (`xetex_format.h`, `xetex-xetex0.c`,
  `xetex-ini.c`): `\partokencontext` and `\partokenname` semantics follow
  TeX Live's public-domain [`partoken.ch`](https://github.com/TeX-Live/texlive-source/blob/trunk/texk/web2c/partoken.ch)
  (Petr Olsak, 2021). This lets current LaTeX and `microtype` select paragraph
  insertion contexts without undefined-command errors. The integer slot
  originally changed the Pitex format layout to serial 34. The current
  compatibility additions use serial 36. Cache filenames include the serial,
  so the new engine does not load an older incompatible format.
  Unicode delimiter primitives use the table-base expressions rather than
  stale generated offsets, including `\Udelcode` and `\XeTeXdelcode`.
- **Source text in PDFs** (`xetex-ini.c`): new formats enable the existing
  `\XeTeXgenerateactualtext` option by default. The Pitex XDV writer preserves
  the UTF-16 text in opcode 254 as PDF `/ActualText`, so shaped ligatures
  retain their source characters when copied or searched.
- **Process lifetime** (`fork.c`, `engine_tex.c`): a checkpoint parent now
  closes its copies of the new child's socket pair before blocking, so a
  checkpoint chain sees EOF and exits when the driver dies; the exec'd root
  engine gets `PR_SET_PDEATHSIG` on Linux.
- **macOS font barrier** (`xetex-ext.c`, `main.c`, `fork.c`,
  `texpresso_protocol.c`, `sprotocol.[ch]`, `engine_tex.c`): upstream delays
  the first `fork()` until output starts because CoreText cannot load fonts
  in a forked child. A checkpoint child that would load a platform font now
  reports `FNTB` and exits; the driver restarts from a freshly exec'd engine
  and takes no checkpoint before that point, so late `\setmainfont{…}` /
  system-font loads run in an exec'd process.
- Standard SyncTeX output (TeXpresso's `/<tag>` extension records disabled),
  cache directory overridable with `PITEX_PREVIEW_CACHE`.
- `xetex/common/texlive_provider.c`: `doc/` and `source/` trees are no longer
  indexed (they shadowed real inputs such as `tex/latex/graphics-cfg/color.cfg`).

## Pitex-authored components

Compatibility additions under `src/shared/` are original Pitex Rust code:
PDF document objects, object streams, threads, inclusion policies, vertical
font metrics and PK bitmap output; an in-process PostScript interpreter
with image/color/shading and VM support; and bibliography/highlighting
services. They read installed TeX package data through the existing file
provider. They do not vendor xdvipdfmx, Ghostscript, Biber, BibTeX or Pygments
implementation code and do not run those programs to render previews.
The PostScript font client uses the same replaceable FreeType shared library
already required by the font stack. Runtime font programs remain in the
user's TeX distribution. The tests run reference tools only as oracles.

The `ignoreprimitiveerror` backport follows the MIT/X11 XeTeX source's
documented integer parameter and bit-1 behavior. PDF primitive parsing and
font expansion are original implementations; no pdfTeX source was copied.
The public pdfTeX vocabulary inventory contains 163 names. Availability
probing records real engine identity and does not assert full semantic
equivalence. The three pdfTeX identity commands remain deliberately
unavailable. Behavioral tests are listed in `COMPATIBILITY.md`.
Format serialization and every platform's derived table offsets change
together, and cache filenames select the matching format serial.

The original Pitex-authored components below now have Rust implementations
under `src/{linux,macos}/` and `src/shared/`. Their historical source names
remain useful for reading the migration record:

`driver/main.c` (stdio protocol — `PROTOCOL.md` — publication, backpressure,
convergence reruns, crash/stuck recovery; its update/close handling follows
TeXpresso's `interpret_open`/`interpret_close`), `driver/xdv.c` (XDV page
index), `driver/xdv2pdf.c` (DVI/XDV interpreter, virtual fonts, dvipdfmx-style
specials used by `xetex.def`, `pgfsys-dvipdfmx.def` and hyperref),
`driver/fonts.c` (OpenType/TrueType/TTC, TFM, VF, Type 1 PFB/PFA, `.enc`,
pdfTeX font maps, Unicode mapping for OpenType `ssty` script alternates), `driver/images.c` (PNG, JPEG, BMP), `driver/pdfw.c` (PDF
objects, PDF page import as Form XObjects), `driver/json.c`, `driver/tbuf.c`,
`shared/*`. They were written from the published format descriptions (DVI by
Knuth, XeTeX XDV opcodes, VF/TFM, OpenType, Type 1, PDF 1.7, PNG, JPEG, BMP);
no xdvipdfmx/dvipdfmx or MuPDF code was used.

## Build and runtime dependencies

Native libraries are shared, none vendored: TECkit, HarfBuzz, graphite2, FreeType, ICU, libpng,
zlib, fontconfig (Linux); CoreText/AppKit frameworks on macOS. The Rust
standard library, libc crate, and C2Rust bitfield helper are statically linked.
The bibliography sourcemap implementation also links the MIT-licensed regex
dependency family. PostScript DCT streams link jpeg-decoder 0.3.2 under its
MIT option, with optional Rayon disabled. `Cargo.lock` records their exact
versions and registry checksums.
Their full original notices are included in `licenses/`.
Licenses and distribution obligations are recorded in
[THIRD-PARTY.md](licenses/THIRD-PARTY.md).
A TeX distribution is read at runtime, not linked.

This boundary applies to the embedded helpers. The Linux GUI application's
existing Poppler PDF viewer is a separate GPL component and remains unchanged.
The helper linkage audit must not be read as a license audit of the whole app.

## Verification commands

```sh
cargo run --locked --quiet --manifest-path Tools/Native/Cargo.toml --bin preview-provenance -- verify
make -C PreviewEngine BUILD_DIR=/abs/out
make -C PreviewEngine selftest BUILD_DIR=/abs/out
PreviewEngine/tools/audit-linkage.sh /abs/out/bin
```

Run the Rust interpreter/unit tests from `PreviewEngine` with
`cargo test --release --locked --bin pitex-preview`.
Run the regressions with `PITEX_EVIDENCE=/abs/evidence.jsonl` (a new file),
then the native vocabulary audit against the same helper pair:

```sh
PITEX_BIN=/abs/out/bin cargo run --locked --quiet --manifest-path Tools/Native/Cargo.toml --bin preview-audit-pdftex-vocabulary -- --require-native --evidence /abs/evidence.jsonl --output PreviewEngine/compatibility/pdftex-vocabulary-report.json
```

The report records build-specific binary hashes and the evidence levels that
those passing cases support. Regenerate it after the final helper build.
Registration, runtime presence, document behavior and license provenance are
separate checks.

Regenerate `provenance.json` after changing any file here:
`cargo run --locked --quiet --manifest-path Tools/Native/Cargo.toml --bin preview-provenance -- generate --upstream <texpresso checkout at e8df770>`.
