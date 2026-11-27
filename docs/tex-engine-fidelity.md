# Built-in TeX engine fidelity: baseline and plan

Goal: the built-in engine should produce output nearly equivalent to pdfLaTeX and XeLaTeX without losing its current speed.
This phase measures and plans. It does not change the engine.

## What the built-in engine is

The built-in engine is the embedded editing-preview helper pair in `PreviewEngine/`.

| Part | Where | What it does |
|---|---|---|
| `pitex-preview` | `PreviewEngine/src/bin/pitex-preview.rs`, `src/{linux,macos}/driver/` | Session driver. Speaks the stdio protocol in `PreviewEngine/PROTOCOL.md`, forks checkpointed engine processes, reruns until auxiliary files converge, publishes PDF/SyncTeX snapshots |
| `pitex-preview-xetex` | `src/{linux,macos}/xetex/` | XeTeX 0.999994 core translated to Rust (c2rust, pinned nightly), with backported pdfTeX vocabulary |
| XDV→PDF writer | `src/{linux,macos}/driver/xdv2pdf.rs`, `src/shared/*` | Pitex's own writer: fonts, images, links, outlines, PostScript interpreter, PDF inclusion |
| In-process services | `src/shared/embedded_aux.rs`, `bibliography_*.rs`, `bibtex_style.rs` | BibTeX `.bst` interpreter, Biber-compatible BBL generation, minted lexers |

The helpers implement typesetting, PDF output, bibliography generation and code highlighting themselves.
They delegate only to the user's TeX distribution for data: `kpsewhich`, `.sty`/`.cls`/`.tfm`/font files and hyphenation patterns.
Installed `xelatex`, `pdflatex`, `xdvipdfmx`, BibTeX and Biber are test oracles, not fallbacks (`PreviewEngine/COMPATIBILITY.md`).

The format is always XeLaTeX: `format_name = "xelatex.ini"` at `PreviewEngine/src/linux/xetex/main/main.rs:1858` and `src/macos/xetex/main/main.rs:1806`.
A document marked `% !TEX program = pdflatex` is still typeset with XeLaTeX semantics: TU encoding, OpenType Latin Modern, `\pdfoutput=0` and the `xetex`/`dvipdfmx` package drivers.

Selection:

- Linux: `SettingsStore::live_preview_backend` (`Linux/crates/pitex-shell/src/settings.rs:310`) defaults to `"embedded"`. `Model::embedded_wanted` (`Linux/crates/pitex-shell/src/model.rs:4296`) starts the helper when live compile is on. SSH projects always use it. The helper is found by `embedded_preview::helper_path` (`Linux/crates/pitex-shell/src/embedded_preview.rs:299`).
- macOS: the same setting in `Mac/Sources/Features/SettingsView.swift:108`, checked at `Mac/Sources/Features/EmbeddedPreview.swift:419`. The helper ships in `Pitex.app/Contents/Helpers/PreviewEngine/`.
- Manual **Build** always runs the project compiler (`latexmk`, `pdflatex` or `xelatex`) through `DirectCommandPlan` (`Linux/crates/build-core/src/lib.rs:267`).

Fallbacks: none automatic.
A `failed` event keeps the last good preview on screen.
The user can switch the backend to `"compiler"`, which runs the project toolchain for live preview.

Existing regression coverage is in `Tools/Native` (`preview-regress-*`, 22 tools in the CI step "Differential TeX, PDF, PostScript and auxiliary regressions").
Those tools check features and semantics case by case.
None of them measures whole-document visual fidelity, page-level text or wall time against a stock engine, which is what this benchmark adds.

## Benchmark

`Tools/tex-fidelity.py` is standard-library Python plus poppler (and MuPDF for link counts).
It drives the helper over the same stdio protocol the apps use and compares against `latexmk` with the engine named by the document's `% !TEX program` line.

```sh
PITEX_BIN=<dir with pitex-preview and pitex-preview-xetex> \
  python3 Tools/tex-fidelity.py --runs 3 --json out.json --markdown out.md
python3 Tools/tex-fidelity.py --self-test
```

The corpus is `Fixtures/tex-fidelity/`: 28 self-written documents and one invented `refs.bib`.
Images come from TeX Live's `mwe` package.

| Area | Documents |
|---|---|
| Classes | `article-basic`, `report-chapters`, `beamer-basic`, `ieeetran-conf`, `thesis-long` (23 pages), `twocolumn`, `korean-oblivoir` |
| Math | `math-ams` (amsmath, amssymb, mathtools, bm), `theorems` (amsthm, cleveref), `xelatex-fontspec` (unicode-math) |
| Floats and tables | `graphics` (PDF, PNG, JPEG, subcaption), `tables` (booktabs, multirow, tabularx, longtable, colortbl) |
| References | `hyperref-refs`, `natbib-bibtex` (BibTeX), `biblatex-biber` (Biber) |
| Packages | `algorithms`, `xcolor-geometry`, `microtype`, `tikz-basic`, `pgfplots`, `siunitx`, `enumitem-lists`, `footnotes-margins`, `listings-code`, `inputenc-babel` |
| Unicode and Korean | `xelatex-fontspec`, `xelatex-multilingual` (polyglossia: Greek, Russian, German), `korean-kotex`, `korean-oblivoir`, `beamer-korean` |

Metrics per document:

| Metric | How |
|---|---|
| Build | Reference: `latexmk` exit status. Built-in: a complete `published` event. Error and `Warning` counts from both logs |
| Pages | `pdfinfo` |
| Text | Per-page `pdftotext`, NFKC and casefold, word-level `difflib` ratio. Median and minimum over pages |
| Raster | Per-page `pdftoppm -r 72 -gray`. Ink F1 with one-pixel tolerance: dark pixels of each page matched against the other page's dilated ink. 1.0 is identical, 0.0 shares nothing. Median and minimum over pages |
| Fonts | `pdffonts`: names with subset tags stripped, and the unembedded count |
| Links, outline | MuPDF `getLinks()` per page; `pdftohtml -xml` outline items |
| Time | Reference cold: `latexmk` from an empty output directory. Reference warm: one more engine pass with converged aux files. Built-in cold: new helper session with the shared format cache until generation 1 completes. Built-in warm: an edit right after `\begin{document}` in the same session, which forces a full body pass. Medians of 3 |

A missing page scores 0 for text and raster.
Identical layouts rendered from different font embeddings score 0.975–1.0 on raster; that is the metric's noise floor.
Text similarity is order-sensitive, so it also drops when superscripts or a `?` node are emitted in a different reading order.

## Baseline

Helpers: Pitex v1.1.7 release packages (built from `75e1919`, current `main`), unmodified.
Raw results: [`baseline-linux.json`](tex-engine-fidelity/baseline-linux.json) and [`baseline-macos.json`](tex-engine-fidelity/baseline-macos.json).

### Linux (comet, Ubuntu 24.04, TeX Live 2023/Debian, pdfTeX 1.40.25)

Format build, once per install: 3.4 s.

| Document | Engine | Ref ok | Built-in ok (errors) | Pages ref/built-in | Text median/min | Raster median/min | Fonts ref/built-in | Links ref/built-in | Outline ref/built-in | PDF KB ref/built-in | Ref cold/warm s | Built-in cold/warm s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| algorithms | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.999/0.999 | 8/9 | 0/0 | 0/0 | 81/361 | 0.47/0.20 | 0.44/0.11 |
| article-basic | pdflatex | yes | yes (0) | 3/3 | 1.000/1.000 | 0.997/0.997 | 14/14 | 0/0 | 0/0 | 266/2225 | 0.54/0.22 | 0.47/0.12 |
| beamer-basic | pdflatex | yes | yes (0) | 7/7 | 1.000/0.783 | 0.998/0.896 | 7/9 | 121/9 | 2/2 | 91/519 | 1.27/0.57 | 1.32/0.29 |
| beamer-korean | xelatex | yes | yes (0) | 2/2 | 1.000/1.000 | 1.000/1.000 | 6/6 | 0/0 | 0/0 | 11/55091 | 2.24/1.10 | 5.21/0.41 |
| biblatex-biber | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.959/0.959 | 6/6 | 9/9 | 1/1 | 94/668 | 2.29/0.44 | 2.03/2.12 |
| enumitem-lists | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.991/0.991 | 3/3 | 0/0 | 0/0 | 35/146 | 0.25/0.18 | 0.45/0.12 |
| footnotes-margins | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.984/0.984 | 5/5 | 0/0 | 0/0 | 56/345 | 0.27/0.19 | 0.44/0.15 |
| graphics | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 1.000/1.000 | 3/3 | 0/0 | 0/0 | 53/223 | 0.49/0.20 | 0.46/0.07 |
| hyperref-refs | pdflatex | yes | yes (0) | 2/2 | 0.999/0.997 | 0.857/0.734 | 8/10 | 14/14 | 4/4 | 87/544 | 0.71/0.31 | 0.75/0.15 |
| ieeetran-conf | pdflatex | yes | yes (0) | 1/1 | 0.979/0.979 | 0.499/0.499 | 7/9 | 0/0 | 0/0 | 82/505 | 1.27/0.23 | 0.59/0.11 |
| inputenc-babel | pdflatex | yes | yes (0) | 1/1 | 0.773/0.773 | 0.725/0.725 | 1/1 | 0/0 | 0/0 | 35/118 | 0.51/0.22 | 0.53/0.08 |
| korean-kotex | xelatex | yes | yes (0) | 1/1 | 0.966/0.966 | 1.000/1.000 | 10/10 | 0/0 | 0/0 | 34/94857 | 1.25/0.66 | 4.01/0.74 |
| korean-oblivoir | xelatex | yes | yes (0) | 2/2 | 1.000/1.000 | 0.982/0.964 | 7/7 | 5/5 | 4/4 | 38/140150 | 2.20/0.77 | 6.00/0.71 |
| listings-code | pdflatex | yes | yes (0) | 1/1 | 0.709/0.709 | 0.766/0.766 | 3/4 | 0/0 | 0/0 | 38/206 | 0.52/0.22 | 0.52/0.11 |
| math-ams | pdflatex | yes | yes (0) | 1/1 | 0.919/0.919 | 0.983/0.983 | 15/17 | 0/0 | 0/0 | 140/607 | 0.53/0.21 | 0.52/0.12 |
| microtype | pdflatex | yes | yes (2) | 2/2 | 0.909/0.818 | 0.927/0.903 | 2/2 | 0/0 | 0/0 | 55/234 | 0.54/0.23 | 0.54/0.23 |
| natbib-bibtex | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.975/0.975 | 6/6 | 0/0 | 0/0 | 85/670 | 1.14/0.20 | 0.49/0.14 |
| pgfplots | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.994/0.994 | 2/3 | 0/0 | 0/0 | 25/126 | 0.47/0.38 | 0.84/0.12 |
| report-chapters | pdflatex | yes | yes (0) | 7/7 | 1.000/1.000 | 0.998/0.997 | 5/5 | 0/0 | 0/0 | 134/805 | 0.50/0.20 | 0.49/0.14 |
| siunitx | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.998/0.998 | 5/6 | 0/0 | 0/0 | 49/215 | 0.32/0.25 | 0.55/0.07 |
| tables | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.994/0.994 | 2/2 | 0/0 | 0/0 | 33/114 | 0.49/0.20 | 0.51/0.06 |
| theorems | pdflatex | yes | yes (0) | 1/1 | 1.000/1.000 | 0.979/0.979 | 8/9 | 3/3 | 1/1 | 87/389 | 0.68/0.30 | 0.65/0.07 |
| thesis-long | pdflatex | yes | yes (0) | 23/23 | 1.000/0.995 | 0.999/0.991 | 10/10 | 7/7 | 6/6 | 220/1270 | 0.71/0.30 | 0.77/0.11 |
| tikz-basic | pdflatex | yes | yes (0) | 1/1 | 0.875/0.875 | 0.953/0.953 | 2/2 | 0/0 | 0/0 | 23/92 | 0.37/0.28 | 0.61/0.08 |
| twocolumn | pdflatex | yes | yes (0) | 2/2 | 0.991/0.981 | 0.999/0.998 | 9/9 | 0/0 | 0/0 | 137/775 | 0.55/0.24 | 0.54/0.11 |
| xcolor-geometry | pdflatex | yes | yes (0) | 2/2 | 1.000/1.000 | 0.988/0.985 | 2/2 | 0/0 | 0/0 | 30/119 | 0.52/0.22 | 0.53/0.04 |
| xelatex-fontspec | xelatex | yes | yes (0) | 1/1 | 0.980/0.980 | 1.000/1.000 | 7/7 | 0/0 | 0/0 | 21/1389 | 1.18/0.61 | 1.03/0.23 |
| xelatex-multilingual | xelatex | yes | yes (0) | 1/1 | 0.947/0.947 | 0.998/0.998 | 1/1 | 0/0 | 0/0 | 13/590 | 0.95/0.48 | 0.82/0.18 |

Summary (Linux):

| Measure | Value |
|---|---|
| Documents that build (reference / built-in) | 28 / 28 |
| Same error count as the reference | 27 / 28 (`microtype`: 2 built-in errors) |
| Same page count | 28 / 28 |
| Raster: median of per-document medians | 0.993 |
| Raster: documents with median ≥ 0.97 | 21 / 28 |
| Raster: documents with every page ≥ 0.90 | 23 / 28 |
| Text: median of per-document medians | 1.000 |
| Text: documents with median ≥ 0.98 | 20 / 28 |
| Same link count | 27 / 28 |
| Unembedded fonts in built-in output | 0 |
| Median time, built-in vs reference, cold | 0.55 s vs 0.54 s (ratio median 1.01) |
| Median time, built-in vs reference, warm | 0.12 s vs 0.23 s (ratio median 0.47) |
| Median PDF size ratio, built-in / reference | 5.7× |

### CI cross-check (GitHub `ubuntu-24.04`, helpers built from this branch's source)

The `preview-engine` job ran the same corpus once (`--runs 1`) with helpers built from source rather than the release package.
Every fidelity number matches the comet table to three decimals except three documents:
`footnotes-margins` raster 0.739 (comet 0.984), `twocolumn` text 0.770/0.541 (comet 0.991/0.981), and `thesis-long` with 22 pages in both reference and built-in output (comet 23).
On the runner the reference PDFs of all three differ in size from comet's and both logs carry one extra warning, so the cause is the runner's TeX Live or fonts rather than the engine (inferred; the warning text is not inspected yet).
Built-in times on the runner are 1.1–1.6× comet's, and its warm times are noisier, which is why the speed gate below runs on comet and the M4.

### macOS (M4 `mini`, TeX Live 2026, pdfTeX 1.40.29)

Same corpus and harness, v1.1.7 helpers copied out of the release DMG, every step under `/tmp/swe-m4lock.py`.
Format build: 4.7 s.

Of the 24 other documents whose reference builds there, all score the same as on Linux to three decimals except `listings-code` (text 0.670, raster 0.758), `math-ams` (text 0.904) and `microtype` (0 built-in errors, text 0.937, raster 0.929). TeX Live 2026 ships newer package versions, so the reference output itself moved.
TeX Live 2026's microtype accepts letterspacing under XeTeX, which removes the two errors.
Built-in cold time is lower than the reference on 19 of 24 documents (median ratio 0.84); warm median ratio is 0.26.

`biblatex-biber` fails on the M4 (gap 2): text 0.401, raster 0.133, while the same helper scores 1.000 / 0.959 against TeX Live 2023 on Linux.

Three documents could not be compared on the M4 because of the machine, not the engine:
`korean-kotex`, `korean-oblivoir` and `beamer-korean` need Noto CJK KR, which is not installed.
The TeX Live 2026 `biber` is a universal binary whose wrapper fails to start (`extracting arm64 binary with lipo failed`).
The TeX Live tree is root-owned, so the run uses an arm64 copy outside it: `lipo /usr/local/texlive/2026/bin/universal-darwin/biber -thin arm64 -output /tmp/pitex-fidelity/biber-arm64/biber`, prepended to `PATH` for the benchmark only (biber 2.21 starts).
MuPDF is not installed, so links are not counted there.
Nothing in the owner's installation was changed.

The Mac helper shares the writer and engine sources with Linux apart from platform font discovery (CoreText instead of Fontconfig).
The matching numbers say the Mac path does not need its own gap list today; gap 2 follows the TeX Live release, not the platform.

## Gaps

Ranked by documents affected times severity.
Effort: S ≤ 3 days, M 1–2 weeks, L 3–6 weeks.
"Measured" means observed in this run; "inferred" means read from source and not yet confirmed by a fix.

| # | Gap | Documents | Symptom | Root cause | Effort | Expected gain | Kind |
|---|---|---|---|---|---|---|---|
| 1 | pdfLaTeX documents run as XeLaTeX | `ieeetran-conf`, `inputenc-babel`, `listings-code`, `hyperref-refs`, `microtype`; every pdfLaTeX document without `\usepackage[T1]{fontenc}` | Wrong font family (Times → Latin Modern, raster 0.499); missing glyphs Ł ź ř € © ° “ ” (text 0.773); bold typewriter instead of CM typewriter; different line breaks from OT1/CM vs TU/LM widths (page raster 0.734); microtype tracking and letterspacing refused (2 errors on TL 2023) | Format is hard-coded to `xelatex.ini` (`PreviewEngine/src/linux/xetex/main/main.rs:1858`, `src/macos/xetex/main/main.rs:1806`). LaTeX sees `\XeTeXversion`, selects TU/lmr and Unicode input, `\pdfoutput` stays 0 (`COMPATIBILITY.md`). Measured | L | 5 documents from 0.50–0.93 raster to ≥ 0.97 (inferred) | Fixable: the core already has TFM fonts, `\XeTeXinputencoding bytes` and 154 of 163 pdfTeX primitives with evidence; a pdfTeX-personality format hides the XeTeX identity |
| 2 | Biber BBL syntax rejected by current biblatex | `biblatex-biber` on TeX Live 2026 (M4); every biblatex document on that release | Bibliography prints raw name fields (`author1hash=…family=Knuth, familyi=K., given=…`); text 0.401, raster 0.133. Same document on TeX Live 2023: 1.000 / 0.959 | The in-process BBL writer emits names as one-line `{{hash=…}{family={…}, familyi={…}, given={}, …}}` with empty parts (`src/shared/embedded_aux.rs:1197`). Biber 2.21 writes `{{un=0,uniquepart=base,hash=…}{family={…}, … givenun=0}}` and omits empty parts; biblatex 3.21 parses only that form, biblatex in TeX Live 2023 tolerates both. Measured | S | biblatex documents on TeX Live 2026 from unreadable to ≥ 0.96 raster | Fixable |
| 3 | Beamer navigation links dropped | `beamer-basic`, any Beamer deck | 9 links instead of 121 (real XeLaTeX also gives 121); 3 of 17 annotations per frame survive | Breakable annotation specials (`pdf:bann`/`pdf:eann`) handled at `src/linux/driver/xdv2pdf.rs:4431`; the navigation-bar links inside headline/footline boxes are not emitted. Exact branch not located | S–M | Link parity on all Beamer output | Fixable |
| 4 | Copied math text wrong | `korean-kotex` (∑ copies as X), `math-ams`, `xelatex-fontspec` (∫ variant lost) | Text 0.92–0.97 while raster is 0.98–1.0 | Type 1 fonts with a built-in encoding (CMEX10) get ToUnicode from character codes rather than glyph names; OpenType MATH variant glyphs have no cmap or GSUB path to a code point (`src/linux/driver/fonts.rs:549` `set_unicode`, `:859` `parse_script_unicode`). Inferred | S | Text ≥ 0.98 on math documents | Fixable |
| 5 | Fonts embedded whole, no subsetting | All; worst on CJK | PDFs 5.7× larger at the median, 55–140 MB for Korean (reference 11–38 KB). Korean cold builds 4.0–6.0 s vs 1.25–2.24 s; the first publication wrote 97 MB | Deliberate shortcut at `src/linux/driver/xdv2pdf.rs:24` ("fonts are embedded whole (no subsetting)"). The same face is listed several times in one PDF. Measured size and bytes written; the cold-time link is inferred | M–L | Size parity; Korean cold time toward XeLaTeX (2–3× faster, hypothesis; informational since Korean is out of scope) | Fixable |
| 6 | Bibliography documents slow on every edit | `biblatex-biber` (and any biblatex document) | Warm edit 2.12 s, slower than the 0.44 s reference pass and the built-in cold build | Each body edit regenerates the BBL in process and reruns passes to convergence even when citation keys and databases did not change (`src/shared/embedded_aux.rs`). Inferred from timing | M | Warm ≈ 0.2–0.4 s on bibliography documents | Fixable |
| 7 | Slow cold start on small package-heavy documents | `pgfplots`, `siunitx`, `tikz-basic`, `enumitem-lists`, `footnotes-margins` | Cold 0.44–0.84 s vs reference 0.25–0.47 s | Each session rebuilds the TeX file index (about 1,900 `ls-R` precedence lines in `driver.log` per session) and reruns until aux files are byte-stable. Inferred | S | −0.2 s per cold session | Fixable |
| 8 | Beamer title-box shadow | `beamer-basic` frame 1 | Page raster 0.896, softer shadow | pgf fading and soft-mask transparency path in the writer. Not located | S–M | Frame 1 to ≥ 0.97 | Fixable |
| 9 | Paragraph breaks with font expansion | `microtype` | Raster 0.927, text 0.909 | Pitex's expansion optimizer can choose other breaks than pdfTeX (`COMPATIBILITY.md`, "Service and policy limits"). Partly masked by gap 1 | M | ≥ 0.97 after gap 1 | Semi-fundamental: pdfTeX's exact expansion arithmetic must be matched, not approximated |
| 10 | URL link borders not drawn by poppler | `biblatex-biber` | Raster 0.959; the cyan URL boxes appear only in the reference rendering | `/Border` and `/C` match the reference. The difference is elsewhere in the annotation (indirect `/Annots` array or key set). Not located | S | ≥ 0.98 | Fixable; viewer-dependent |
| 11 | Duplicate-destination warnings | `thesis-long` | Warning "duplicate destination ignored; the first definition is kept"; output matches | Writer reports what hyperref's `xetex` driver emits under XeLaTeX semantics; disappears with gap 1 (inferred) | S | Clean warnings | Fixable |

Not gaps, despite low numbers:

- `tikz-basic` text 0.875: one `?` node is extracted in another order on an eight-word page.
- `xelatex-multilingual` text 0.947: the reference font lacks ∀, ∈ and ℝ; the built-in copies them through ActualText and the reference drops them.
- `xelatex-fontspec` ligatures and small caps: the built-in copies the source text, the reference copies glyph names. NFKC and casefold remove that.

Fundamental limits, outside this corpus:
LuaLaTeX-only documents (`luacode`, `luaotfload` features) need a Lua engine.
Shell-escape tools (`minted` without a frozen cache, `gnuplottex`, `svg` through Inkscape) are refused by preview policy.
Exact pdfTeX paragraph output with font expansion (gap 9) needs pdfTeX's arithmetic, which is a port, not a fix.
Nothing measured here requires a new TeX implementation; the core is XeTeX.

## Definition of done: "nearly equivalent"

Measured with `Tools/tex-fidelity.py` on the CI `preview-engine` runner (TeX Live 2023/Debian), on comet, and on the M4 with TeX Live 2026, over this corpus plus the round 3 additions.
Korean and CJK are out of scope by owner decision (2026-11-27): `korean-kotex`, `korean-oblivoir` and `beamer-korean` stay in the corpus and the report as informational rows and do not count toward any gate.
The "Today" column covers the other 25 documents on Linux.
Gap 9 documents may carry a written exemption.

| Gate | Threshold | Today (Linux) |
|---|---|---|
| Build | 100% of documents that build with the reference build with the same error count | 24 / 25 |
| Pages | 100% equal page count | 25 / 25 |
| Raster | Every document median ≥ 0.97 and every page ≥ 0.90; corpus median ≥ 0.99 | 18 / 25 and 20 / 25; 0.991 |
| Text | Every document median ≥ 0.98; none below 0.95 | 18 / 25 and 18 / 25 |
| Links and outline | Equal counts on 100% | 24 / 25 links, 25 / 25 outline |
| Fonts | 0 unembedded | 0 |
| Speed | No document's built-in cold or warm median slower than this baseline by more than 5%; corpus median not slower | Baseline (this report) |
| TeX Live drift | The same gates hold on the newest TeX Live release (TeX Live 2026 on the M4 today) | `biblatex-biber` fails (gap 2) |

Speed is gated on comet and the M4, not on shared CI runners: median of 5, the same helper build on both sides of the comparison, `-j 4`, under the memory scope.
CI keeps a loose guard of +25% on the built-in/reference time ratio, because runner noise is larger than 5%.

## Round plan

Every engine change touches both `src/linux` and `src/macos`; 41 files differ between the twin trees.
Each PR below owns a file set, so PRs in one round do not edit the same files.

### Round 1 (parallel)

| PR | Gap | Owns | Done when |
|---|---|---|---|
| 1A pdfTeX personality | 1, 11 | `src/{linux,macos}/xetex/main/main.rs`, a new `PreviewEngine/format/pitex-pdflatex.ini`, `src/bin/pitex-preview.rs` (reads `% !TEX program`, passes the format to the engine) | `ieeetran-conf`, `inputenc-babel`, `listings-code`, `hyperref-refs` reach raster ≥ 0.97; `microtype` has 0 errors; XeLaTeX documents unchanged |
| 1B Link annotations | 3, 10 | `src/{linux,macos}/driver/xdv2pdf.rs`, special-handling section only (`pdf:bann`/`eann`/`ann`, around lines 4400–4700) | `beamer-basic` 121/121 links; `biblatex-biber` raster ≥ 0.98 |
| 1C OpenType ToUnicode | 4 (OpenType part) | `src/{linux,macos}/driver/fonts.rs`, `src/shared/font_output.rs`, `src/shared/opentype_math.rs` | `xelatex-fontspec` text ≥ 0.99 |
| 1E Biber BBL syntax | 2 | `src/shared/embedded_aux.rs` BBL writer (around line 1197) | `biblatex-biber` on TeX Live 2026 matches the TeX Live 2023 numbers; TeX Live 2023 unchanged |
| 1D Fidelity gate | — | `Tools/tex-fidelity.py` (`--check` against committed thresholds), `Fixtures/tex-fidelity/`, `.github/workflows/ci.yml` | CI fails on any regression from the committed baseline |

### Round 2 (parallel, after round 1 merges)

| PR | Gap | Owns | Done when |
|---|---|---|---|
| 2A Font subsetting and deduplication | 5, 4 (Type 1 part) | `src/{linux,macos}/driver/xdv2pdf.rs` font-emission section (around lines 5250–5600), new `src/shared/font_subset.rs` | Median size ratio ≤ 1.5× and no document slower; `math-ams` text ≥ 0.98 |
| 2B Bibliography warm path | 6 | `src/shared/embedded_aux.rs`, `src/shared/bibliography_*.rs` | `biblatex-biber` warm ≤ 0.5 s with identical output |
| 2C Session start and rerun policy | 7 | `src/{linux,macos}/driver/main.rs`, `src/{linux,macos}/driver/fs.rs` | Small-document cold within 5% of the reference |
| 2D Shading and soft masks | 8 | `src/shared/postscript_color.rs` or the pgf soft-mask path it leads to | `beamer-basic` frame 1 ≥ 0.97 |

2A and 1B share `xdv2pdf.rs` but different sections; 2A waits for 1B, so they never run in parallel.
2B and 1E share `embedded_aux.rs`; 2B starts after 1E merges.
2C edits `driver/main.rs`, which 1A does not touch.

### Round 3

- Corpus growth: `acmart`, `elsarticle`, `llncs`, `revtex4-2`, KOMA-Script, `memoir`, `tcolorbox`, `chemfig`, `minted` with a frozen cache.
- Run the benchmark against the newest TeX Live in CI as well as Ubuntu's 2023 packages; gap 2 shows that package updates alone can break the built-in path.
- Gap 9 (pdfTeX expansion arithmetic) if microtype documents still miss the raster gate.
- Tighten the CI gate to the full definition of done and add a macOS runner with the CJK fonts.

## Running the benchmark

Linux needs no source build. comet has no TECkit development headers, so `make -C PreviewEngine` fails there; the release package's helpers run as they are (the runtime `libTECkit.so.0` is installed):

```sh
gh release download v1.1.7 -R jaehwan-2ee/pitex -p 'Pitex-v1.1.7-ubuntu24.04-amd64.deb'
dpkg-deb -x Pitex-v1.1.7-ubuntu24.04-amd64.deb pkg   # extracts only, installs nothing
systemd-run --user --scope -p MemoryMax=6G \
  env PITEX_BIN=pkg/usr/lib/pitex python3 Tools/tex-fidelity.py --runs 3 --json linux.json
```

From a source build, set `PITEX_BIN` to `$BUILD_DIR/bin` after `make -C PreviewEngine BUILD_DIR=...`.

macOS (M4 `mini`): the installed `Pitex.app` may be older than the release under test.
Attach the release DMG read-only (`hdiutil attach -nobrowse -readonly`), copy `Pitex.app/Contents/Helpers/PreviewEngine/` including its `lib/` directory, detach, and point `PITEX_BIN` at the copy.
Wrap each run in `python3 /tmp/swe-m4lock.py`.
On that machine Noto CJK KR and `mutool` are missing, so the Korean documents have no reference and links are not counted.
The TeX Live 2026 `biber` wrapper fails to start; put an arm64 copy first on `PATH` (`lipo /usr/local/texlive/2026/bin/universal-darwin/biber -thin arm64 -output /tmp/pitex-fidelity/biber-arm64/biber`).
A permanent fix needs root: replace the universal binary with that arm64 copy, or try `sudo tlmgr update --self && sudo tlmgr update biber` (untested; the wrapper bug may persist in the update).

The script needs `latexmk`, `pdflatex`, `xelatex`, `bibtex`, `biber` and poppler (`pdfinfo`, `pdftotext`, `pdftoppm`, `pdffonts`, `pdftohtml`) on `PATH`; `mutool` is optional.
Runs are sequential so timings do not compete; a full `--runs 3` pass takes about 15 minutes on comet.
CI runs the benchmark once per push in the `preview-engine` job and attaches the table to the step summary and the `tex-fidelity` artifact.
