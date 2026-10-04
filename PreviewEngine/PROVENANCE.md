# Embedded preview engine — provenance and dependency boundary

This directory builds the two helpers behind Pitex's **editing preview**:

| Helper | Role | Origin |
|---|---|---|
| `pitex-preview-xetex` | XeTeX engine that checkpoints itself with `fork()` and does all file I/O through the driver | TeXpresso's modified XeTeX (itself the Tectonic port of XeTeX), with the changes listed below |
| `pitex-preview` | Session driver: file overrides from unsaved editor buffers, rollback to checkpoints, XDV→PDF conversion, atomic publication | TeXpresso's session/rollback logic ported off MuPDF, plus a Pitex-authored XDV→PDF writer |

Upstream: <https://github.com/let-def/texpresso> at commit
`e8df7709077b2f86f6e16e6c86ceefb86de06f8d`. `provenance.json` records, for
every file here, whether it was imported (upstream path, upstream and
current SHA-256, modified flag, license notice) or written for Pitex.
`tools/provenance.py verify` checks that record; `tools/audit-linkage.sh`
additionally inspects the built helpers' shared-library closure and symbols.
This documents the technical boundary; it is not a legal opinion.

Final documents are **not** produced here: manual builds keep using the
project's pdflatex/xelatex/lualatex/latexmk configuration. Preview PDFs are
temporary artifacts in a per-session directory and say so in their metadata
(`/Producer (Pitex embedded editing preview)`).

## What was imported

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
  counts as converged.
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

`driver/main.c` (stdio protocol — `PROTOCOL.md` — publication, backpressure,
convergence reruns, crash/stuck recovery; its update/close handling follows
TeXpresso's `interpret_open`/`interpret_close`), `driver/xdv.c` (XDV page
index), `driver/xdv2pdf.c` (DVI/XDV interpreter, virtual fonts, dvipdfmx-style
specials used by `xetex.def`, `pgfsys-dvipdfmx.def` and hyperref),
`driver/fonts.c` (OpenType/TrueType/TTC, TFM, VF, Type 1 PFB/PFA, `.enc`,
pdfTeX font maps), `driver/images.c` (PNG, JPEG, BMP), `driver/pdfw.c` (PDF
objects, PDF page import as Form XObjects), `driver/json.c`, `driver/tbuf.c`,
`shared/*`. They were written from the published format descriptions (DVI by
Knuth, XeTeX XDV opcodes, VF/TFM, OpenType, Type 1, PDF 1.7, PNG, JPEG, BMP);
no xdvipdfmx/dvipdfmx or MuPDF code was used.

## Build and runtime dependencies

All shared, none vendored: TECkit, HarfBuzz, graphite2, FreeType, ICU, libpng,
zlib, fontconfig (Linux); CoreText/AppKit frameworks on macOS. Licenses and
obligations: `licenses/THIRD-PARTY.md`. A TeX distribution is read at run
time, not linked.

## Verification commands

```sh
python3 PreviewEngine/tools/provenance.py verify
make -C PreviewEngine BUILD_DIR=/abs/out
PreviewEngine/tools/audit-linkage.sh /abs/out/bin
```

Regenerate `provenance.json` after changing any file here:
`python3 PreviewEngine/tools/provenance.py generate --upstream <texpresso checkout at e8df770>`.
