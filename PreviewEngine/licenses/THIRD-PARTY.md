# Third-party components of the embedded preview helpers

`pitex-preview` and `pitex-preview-xetex` contain source code from TeXpresso
(MIT, `TeXpresso-LICENSE.txt`), the Tectonic XeTeX port (MIT,
`Tectonic-MIT.txt`) and XeTeX (MIT/X11, `XeTeX-SIL-MIT.txt`); two files are
public domain (`xetex/engine/xetex-texmfmp.c`, `xetex/main/picohash.h`).
Everything else in `PreviewEngine/` is Pitex-authored (repository `LICENSE`,
AGPL-3.0-or-later). `PreviewEngine/provenance.json` lists every file.

The in-process PostScript, bibliography and highlighting implementations are
Pitex-authored. Neither GPL/AGPL converters nor external bibliography or
highlighting programs are embedded by these additions. The PostScript font
outline client dynamically uses FreeType under the FTL; it reads runtime
font files from the user's TeX installation without bundling new font data.

## Shared libraries loaded at run time (not vendored)

| Library | License | Notes |
|---|---|---|
| TECkit (`libTECkit`) | LGPL-2.1-or-later OR CPL-0.5-or-later | Unmodified upstream library, dynamically linked; shipped under LGPL-2.1+ (see below) |
| HarfBuzz | MIT ("Old MIT") | OpenType shaping |
| graphite2 | LGPL-2.1-or-later OR MPL-2.0 OR GPL-2.0-or-later | Used under LGPL/MPL; dynamically linked |
| FreeType | FreeType License (FTL) OR GPL-2.0-or-later | Used under the FTL: portions of this software are copyright © The FreeType Project (www.freetype.org). All rights reserved. |
| ICU | Unicode License v3 | Line breaking, bidi, normalization data |
| fontconfig (Linux) | MIT-style (HPND) | System font lookup |
| libpng | PNG Reference Library License v2 | Required by the font/image stack |
| zlib | zlib License | Compression |

Transitive libraries loaded by these (for example GLib, LGPL-2.1-or-later;
PCRE2, BSD-3-Clause; Expat, MIT; bzip2; Brotli, MIT) are likewise shared
distribution libraries. `tools/audit-linkage.sh` prints the complete closure
of a build together with the package each library comes from.

On Ubuntu these are the distribution packages (`libteckit0`,
`libharfbuzz0b`, `libgraphite2-3`, `libfreetype6`, `libicu74`,
`libfontconfig1`, `libpng16-16t64`, `zlib1g`); their full license texts are
in `/usr/share/doc/<package>/copyright`. In the macOS app bundle the libraries
sit in `Contents/Helpers/PreviewEngine/lib/` (code); their license texts and
the engine's provenance live under `Contents/Resources/PreviewEngine/`
(resources, not code) as staged by `Tools/bundle-preview-engine-macos.sh`.

## TECkit (LGPL-2.1-or-later)

TECkit is used unmodified from upstream (SIL International,
<https://github.com/silnrsi/teckit>, source also in every TeX Live source
distribution). The engine calls only its public C API
(`TECkit_CreateConverter`, `TECkit_ConvertBuffer`, `TECkit_ResetConverter`,
`TECkit_DisposeConverter`) through the shared library, so it can be replaced:

- Ubuntu: the engine loads the system `libTECkit.so.0`; installing another
  build of that package or pointing `LD_LIBRARY_PATH` at a compatible
  `libTECkit.so.0` replaces it. Corresponding source:
  `apt source libteckit0` (Ubuntu package, unmodified).
- macOS: replace the `libTECkit` dylib in
  `Contents/Helpers/PreviewEngine/lib/` (the helper refers to it through
  `@loader_path/lib/`) with a compatible build and re-sign the bundle ad hoc
  (`codesign --force --sign - <path>`). Corresponding source for the bundled
  dylib: the pinned tarball `teckit-2.5.13.tar.xz`, sha256
  `3f55cd3670f1ff1a439d5a40071870b9e4ca2be8877a0eb80e24783cb532b380`, built
  with `./configure --with-system-zlib` per `release.yml`.

## Corresponding source for bundled libraries (LGPL/MPL components)

Per LGPL-2.1 §6 / MPL-2.0 §3.2, sources for the copyleft libraries we ship:

- TECkit 2.5.13 (macOS bundle): the pinned tarball above, unmodified.
- HarfBuzz 14.5.0 (macOS bundle): the pinned tarball recorded in
  `provenance.json`'s `runtime_dependencies` table, unmodified.
- Homebrew-built libraries (macOS bundle): `BUNDLED-VERSIONS.txt` sits next
  to this file in the bundle's `Contents/Resources/PreviewEngine/` (macOS
  only — not shipped in the deb). It records, per dylib, the formula name,
  the exact installed version and the formula's pinned source URL + SHA256 —
  generated during bundling from the copied files themselves, so it cannot
  drift from what actually shipped. The named archives are the unmodified
  corresponding sources.
- On Linux these libraries are not bundled — the installed distribution
  packages' own `apt source` archives are the corresponding source.

This is the distribution's own source-availability statement, not a legal
opinion on license compatibility.

## TeX distribution

The helpers do not contain or link TeX Live, MacTeX or Tectonic. The engine
reads the user's installed distribution at run time (located through
`kpsewhich`), exactly like a separately installed TeX engine, and generates
its own format file into the user's cache directory.

ICU i18n also supplies the native bibliography regular-expression and locale
collation operations. It uses the same installed ICU release and notices as
the existing engine Unicode routines.

## Rust migration components

The embedded helpers statically link the Rust standard library and libc
crate (MIT or Apache-2.0) and C2Rust bitfield support (BSD-3-Clause).

Source parsing for the in-process services statically links regex 1.13.1,
regex-automata 0.4.18, regex-syntax 0.8.11, aho-corasick 1.1.5 and memchr
2.8.3, all used under their MIT option. Their complete original MIT texts,
including copyright holders, are the adjacent versioned
`Rust-<crate>-LICENSE-MIT.txt` files. The versions and source checksums are
locked in `PreviewEngine/Cargo.lock`.

PostScript DCT/JPEG streams use jpeg-decoder 0.3.2 under its MIT option,
with optional Rayon disabled. The complete upstream copyright and license
is in `Rust-jpeg-decoder-0.3.2-LICENSE-MIT.txt`.
`Cargo.lock` pins their versions and registry checksums.

C2Rust's derive macro uses proc-macro2, quote, syn, and unicode-ident during
builds. These are build dependencies, not separately linked runtime
libraries. Their original MIT, Apache-2.0, and Unicode notices are included
in the adjacent `Rust-*` files. The C2Rust runtime and derive code share
`C2Rust-BSD-3-Clause.txt`. The standard library's corresponding copyright
record is `Rust-standard-library-COPYRIGHT.html`.
