# Embedded engine compatibility

This record describes the native Unix helpers in this source checkout.

The helpers combine a checkpointed XeTeX core with Pitex's own XDV-to-PDF writer.
They render inside the helper processes.
Installed `xelatex`, `pdflatex`, `xdvipdfmx`, Ghostscript, Biber, BibTeX and Pygments are test references, not rendering fallbacks.
Manual **Build** still uses the project's compiler configuration to produce the final PDF.

The initial comparison used XeTeX 0.999995 and pdfTeX 1.40.25 from TeX Live 2023.
The embedded core retains its XeTeX 0.999994 identity and includes documented backports.
Tests establish the behavior of their fixtures and installed reference versions.
They do not establish complete XeLaTeX or pdfLaTeX equivalence across packages, fonts and releases.

## Public command inventory

[pdftex-vocabulary.json](compatibility/pdftex-vocabulary.json) contains exactly 163 unique public names.
It covers the pdfTeX 1.40.29 manual's formal syntax appendix and legacy runtime commands.
The initial pdfTeX 1.40.25 reference exposes 161 of these names.
That reference lacks `ignoreprimitiveerror` and predates `pdfuseptexunderscore`.
The catalogue excludes four extraction artifacts listed in the JSON file.

`preview-audit-pdftex-vocabulary` independently checks source registration and runtime availability.
Its [report](compatibility/pdftex-vocabulary-report.json) records the helper hashes, reference version and observations for one build.
Regenerate it after rebuilding the helpers; an older report cannot describe a newer binary.
Registration or a successful `\meaning` query does not verify argument scanning, typography or output behavior.

Three identity names remain deliberately unavailable: `pdftexbanner`, `pdftexrevision` and `pdftexversion`.
The engine reports its real XeTeX identity instead of claiming to be pdfTeX.
The audit rejects evidence links on these names.

### Semantic evidence

Catalog entries link the regression cases that exercise each command, by tool and case ID.
With `PITEX_EVIDENCE` set, every passing case appends a record to that file.
The record holds the helper hashes, the reference engine the case compared against in the same run and the control sequences in its TeX source.
The audit's `--evidence` option derives one level per command from these records:

| Level | Meaning |
|---|---|
| `tested` | A linked case passed against these helpers and compared the command's result with stock pdfTeX output from the same run |
| `partial` | Linked cases passed, but they check fixed expectations, compare with XeLaTeX, or cover only storage or one configuration |
| `not_evaluated` | No case is linked |

The audit fails when a link has no passing record for the current helper hashes or when the linked case's source does not contain the command.
It also fails when a `tested` link's case made no same-run pdfTeX comparison.
CI runs the regressions with `PITEX_EVIDENCE`, then fails if the committed report records different levels (`--check`).
A link is a reviewed claim about what its case observes.
A `tested` level covers the linked fixtures on the reference installation.
It does not cover every keyword, error path or package interaction.

The current report has 105 tested, 49 partial and 9 not-evaluated commands.
The 9 are the three identity names, `pdfoutput`, `pdfmovechars`, `pdfrunninglinkon`, `pdfrunninglinkoff`, `tracinglostchars` and `tracingstacklevels`.
For 48 commands available in both engines, the native `\meaning` differs from stock pdfTeX.
Some names are aliases, such as `\creationdate` for `\pdfcreationdate`.
Others fall back to display text, such as `[unknown extension!]` for `\pdfcolorstack`, `\jobname` for `\pdfunescapehex` and `[unknown dimen parameter!]` for `\pdfvorigin`.
The evidence levels do not cover `\meaning` or `\show` text.
All 48 have behavioral evidence: 42 are tested and 6 partial.
The stock probe runs `pdftex --ini -etex` without LaTeX, so `input` shows LaTeX's macro only in the native column.

These native behaviors differ from pdfLaTeX and are outside the tested levels:

| Command | Native behavior |
|---|---|
| `pdfoutput` | Stays 0, so XeTeX-aware packages keep their xdvipdfmx paths; pdfLaTeX sets 1 |
| `pdfobj`, `pdfrefobj` | Every object is written, with or without `\immediate` or `\pdfrefobj`; pdfTeX omits unreferenced non-immediate objects |
| `pdfxform`, `pdfpageresources` | Resource text merges only standard resource categories; other keys are dropped |
| `pdfxform` | Form bounding boxes use the box baseline as origin, where pdfTeX uses the lower edge; placement matches |
| `pdfsetmatrix` | One combined `cm` operator replaces pdfTeX's translation pair |
| `pdffilemoddate` | `FORCE_SOURCE_DATE` is ignored; the file's own time is reported |
| `pdftrailerid` | Without `\pdftrailerid`, no `/ID` is written; pdfTeX derives one from the time and file name |
| `pdfdecimaldigits` | Applies to text coordinates; page boxes are written with up to six decimal places |
| `pdfadjustspacing`, `pdffontexpand` | Expansion can choose different paragraph breaks (see below) |

## Implemented interfaces

| Interface | Current implementation | Evidence |
|---|---|---|
| Hyperref links and destinations | Named destinations, wrapped and cross-page links, transformed hit regions, margins and annotation/link identifiers | PDF features, structures and policy tests |
| Bookmarks | Nested outlines with Unicode titles and parent, child and sibling references | PDF feature tests |
| Forms and annotations | AcroForm fields, widget appearance resources, page references and annotation geometry | PDF feature tests |
| PDF objects and resources | Reserved object IDs, reference remapping, streams, reusable forms/images, page resources and named objects | Primitive, query and PDF feature tests |
| PDF document controls | Metadata, viewer/catalog options, page boxes, origins, number precision, trailer IDs and compression | Primitive, string and backend option tests |
| Article threads | Explicit and running threads, cyclic bead references, page bead arrays and thread margins | PDF structure tests |
| Object compression | Object streams and xref streams at configured levels, with PDF-version restrictions | PDF structure and raster tests |
| PDF inclusion policy | Version checks, legacy option consumption, duplicate warnings and compatible mapped-Type1 font substitution | PDF policy tests |
| Draft output | `pdfdraftmode` runs the TeX pass without publishing a new PDF; the app shows a warning | Backend option and application tests |
| String and file queries | Escaping, hashes, file metadata, dates, matching and match-result queries | String and query tests |
| Core controls | Font tags, `nd`/`nc` dimensions and diagnostic stream routing | Core control tests |
| Font typography | Expansion, protrusion, letterspacing, ligature controls, font attributes, font resources and Unicode mappings | Primitive and font typography tests |
| Native vertical fonts | Identity-V fonts, W2/DW2 metrics and OpenType vertical origins | Font layout and PDF tests |
| Bitmap fonts | Runtime PK loading and PDF Type 3 glyphs, with configured mode/resolution and font controls | Bitmap font tests |
| OpenType math | MATH constants, glyph kerns, variants and assemblies through the Rust implementation | Three XeLaTeX metric comparisons and unit tests |
| Font layout and discovery | HarfBuzz/Graphite layout, metric caches, font-name matching, collection indices and Type1 metrics | Layout and discovery tests |
| PostScript/PSTricks | Stack execution, paths, curves, clipping, transforms, opacity, font outlines, images and shading types 1–7 | PostScript raster comparisons |
| PostScript images and color | Image types 1/3/4, masks, planar sources, stream filters, Indexed/CIE spaces and Separation/DeviceN tint transforms | Image/color raster comparisons |
| PostScript VM | Shared composite values, save/restore, local/global allocation, access attributes and file objects | Unit tests and two unsaved generations |
| Bibliography data | In-process AUX/BCF handling and virtual BBL generation from current database contents | Auxiliary tests |
| BibTeX styles | Original stack interpreter runs installed `.bst` programs | Eight stock/custom style comparisons |
| Minted inputs | In-process lexers produce virtual block, inline and included-code inputs | Auxiliary tests |
| Compatibility diagnostics | Unsupported PDF/PostScript/service operations appear in publication warnings and the app | Helper and application tests |

The established 27-document corpus also checks ordinary math/macros, expl3/xparse, Unicode fonts, lists, tables and diagrams.
It includes alpha PNG, CMYK JPEG, TikZ gradients/fading/blending, PDF page imports, prepared BBL files and frozen minted inputs.

## Service and policy limits

Font expansion adjusts line-break capacity, glyph advances and outlines.
Pitex's optimizer can select different paragraph breaks from pdfTeX.
Vertical-font support does not establish pTeX/upTeX paragraph-layout equivalence.
Font discovery depends on the user's installed fonts and platform libraries.

`pdfinclusioncopyfonts` preserves source fonts when copying is enabled.
When copying is disabled, compatible unmodified mapped Type1 programs can replace embedded source programs.
CID/TrueType fonts and incompatible maps retain their embedded programs.
This substitution does not merge imported subsets with the main document's font subsets.
PDF-version mismatch controls can warn, suppress the warning or reject an inclusion.
Duplicate-destination, duplicate-map and page-group warning controls affect diagnostics, not the document's objects.

The PostScript implementation covers the tested language, image, shading and VM operations.
It reports unsupported operators and interfaces.
The color path evaluates supported CIE and tint transforms internally.
Sampled PDF tint functions have finite precision: 257 samples for one component, 33² for two, 17³ for three and 9⁴ for four.
Higher dimensions use two samples per axis, with a 16-component capacity limit.
The 24 image/color fixtures pass both unsaved generations.
The CIE-A fixture differs by one gray level under the explicit 1/255 color-management tolerance.
Other image/color raster means remain below 0.1 on the tested reference installation.
Cubic sampled functions retain their original samples and interpolation settings during output-range normalization.
These results do not establish every PostScript device, resource, filter or color-management behavior.
Fonts and package prologs come from the user's TeX installation.

BibLaTeX formats generated BBL data in TeX.
The services read virtual AUX/BCF control data, expanded resource paths and citation keys, including multiple reference sections.
They process supported sorting, labels and field/type/regular-expression sourcemaps internally.
ICU implements supported Perl-style lookaround, numbered/named captures and backreferences.
Sourcemaps normalize Unicode for Biber-compatible matching and BBL output.
Locale collation consumes emitted BCF locale, case, name and sorting controls.
Executable Perl, recursive or conditional patterns, branch resets and unsupported control options produce warnings.
Prepared project BBL files remain authoritative.
The embedded highlighter uses its own palette and lexers.
Arbitrary Pygments plugins and themes are outside that implementation.
A frozen cache retains the installed minted package's behavior.

Editing previews do not execute arbitrary `write18`/`ShellEscape` commands.
They report those requests in snapshot warnings.
This is a preview runtime policy.
Generated virtual inputs never overwrite project files or upload SSH mirror files.
An error count of zero does not prove that every requested output feature was reproduced.

## Regression commands

Build the helpers and set `PITEX_BIN` to their `bin` directory.
From the repository root, run each tool with:

```sh
cargo run --locked --quiet --manifest-path Tools/Native/Cargo.toml --bin <name>
```

| Tool | Scope |
|---|---|
| `preview-audit-pdftex-vocabulary` | 163-name inventory and semantic evidence; see below to update the report |
| `preview-regress-tex-compat` | 27-document package corpus |
| `preview-regress-engine-primitives` | 20 document cases plus format-cache migration; 21 checks |
| `preview-regress-engine-queries` | 29 reference query assertions, page/form references and two unsaved generations |
| `preview-regress-core-controls` | Three font-tag, dimension-unit and diagnostic-stream cases |
| `preview-regress-engine-strings` | 13 string, file, date and metadata cases |
| `preview-regress-font-typography` | 20 font-control cases |
| `preview-regress-backend-options` | Image, resource, output, origin and draft policies |
| `preview-regress-bitmap-fonts` | Seven PK/Type 3 font cases |
| `preview-regress-opentype-math` | Three exact XeLaTeX math-metric cases |
| `preview-regress-font-layout` | Four OpenType, cache, vertical and Graphite cases |
| `preview-regress-font-discovery` | Nine Fontconfig cases on Linux; five CoreText/Type1 cases on macOS |
| `preview-regress-pdf-policy` | 15 inclusion, warning and destination-policy cases |
| `preview-regress-pdf-features` | 15 object-graph, stream, geometry and form cases |
| `preview-regress-pdf-structures` | Nine thread/link/compression cases, raster equality and size reduction |
| `preview-regress-postscript` | Drawing, fonts, images and all seven shading types |
| `preview-regress-postscript-color` | Image types 3/4, masks, Indexed/CIE/tint transforms and image-state corrections |
| `preview-regress-postscript-vm` | VM behavior compared across two unsaved generations |
| `preview-regress-auxiliary` | 14 unsaved bibliography/code cases, removals and process traps |
| `preview-regress-bibtex-style` | Eight stock/custom `.bst` styles against BibTeX |
| `preview-regress-bibliography-controls` | Emitted BCF, BBL fields/order, Unicode regex and locale controls against Biber; two unsaved generations and generator traps |
| `preview-regress-pdftex-semantics` | 19 small pdfLaTeX differential cases for commands whose native `\meaning` differs, plus links, literals, paragraph tokens and output parameters |

To regenerate the report, run the regressions with `PITEX_EVIDENCE` set to a new file.
Then pass that file to the audit:

```sh
cargo run --locked --quiet --manifest-path Tools/Native/Cargo.toml --bin preview-audit-pdftex-vocabulary -- --require-native --evidence "$PITEX_EVIDENCE" --output PreviewEngine/compatibility/pdftex-vocabulary-report.json
```

The report must come from the same helper build as the evidence records.

`make -C PreviewEngine selftest` runs the native boundary suite.
Run Rust unit tests from `PreviewEngine` with `cargo test --release --locked --bin pitex-preview`.
PostScript unit tests run in addition to document regressions.
Reference programs run only as test oracles.
CI checks source provenance, upstream notices and the helpers' dynamic dependencies.
The macOS helper job builds against the SDK and compares OpenType math,
layout and CoreText metrics through two unsaved updates per fixture.
The source and license boundary is recorded in [PROVENANCE.md](PROVENANCE.md).

## Implementation references

The implementations use published interfaces and installed package data:
[PDF reference](https://www.iso.org/standard/51502.html),
[PostScript language](https://www.adobe.com/jp/print/postscript/pdfs/PLRM.pdf),
[pdfTeX manual](https://mirrors.ibiblio.org/CTAN/systems/doc/pdftex/manual/pdftex-a.pdf),
[Type 1 font format](https://www.adobe.com/content/dam/acom/en/devnet/font/pdfs/T1_SPEC.pdf),
[FreeType API](https://freetype.org/freetype2/docs/reference/),
[OpenType vertical metrics](https://learn.microsoft.com/en-us/typography/opentype/spec/vmtx),
[OpenType MATH](https://learn.microsoft.com/en-us/typography/opentype/spec/math),
[Adobe glyph naming](https://github.com/adobe-type-tools/agl-specification)
and [XeTeX source](https://github.com/TeX-Live/texlive-source/blob/trunk/texk/web2c/xetexdir/xetex.web).
No GPL/AGPL converter or bibliography implementation was copied into these helpers.
