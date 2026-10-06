# Windows editing preview helper

`pitex-preview.exe` implements the stdio protocol in [../PROTOCOL.md](../PROTOCOL.md)
for Windows. It runs one persistent session per workspace and renders unsaved
source buffers using the installed `xelatex` from TeX Live or MiKTeX. The app
supplies the same augmented TeX search path as final builds. XeLaTeX is required
for this preview backend regardless of the project's final build recipe.

The helper maintains a private source snapshot, overlays unsaved buffers,
restores saved/closed buffers from disk, and notices changed or removed files
on every update. Sources and compiler outputs stay in the session directory;
the project is never modified. VCS metadata, `.pitex-live`, `.pitex-preview`,
`node_modules`, and Python environment/cache directories are excluded.
Directory symlinks/junctions are not traversed; file symlinks are copied.
Absolute `\input` paths and source dependencies outside the workspace cannot
receive unsaved overrides. Relative project inputs receive overrides normally.

New generations cancel an active compiler process tree immediately. Windows
Job Objects own compiler children, and the app owns the helper's whole process
tree in a separate kill-on-close Job Object. Compiler processes use
`CREATE_NO_WINDOW`; UTF-8 JSON and native Unicode process arguments preserve
paths containing spaces and non-ASCII characters. A stuck pass is stopped
after 60 seconds. Shell escape is disabled for editing previews.

Windows publishes coherent full-pass PDF snapshots after up to three passes
for cross-reference convergence. Recoverable TeX errors with a finished PDF
publish the error-marked PDF; fatal errors retain the last preview. The helper
rewrites uncompressed (or gzip-compressed) SyncTeX `Input:` records back to
original project paths, including known Windows ANSI and mixed-encoding
snapshot aliases. Ambiguous aliases disable SyncTeX rather than select a
different source file. Rewritten metadata recalculates its byte anchors.
The Windows app stages private ASCII query aliases and restores original
Unicode PDF/source identities before validating navigation results.
Final-build floors, buffer-identity validation, and
publication ownership follow the shared app protocol; at most four unreleased
publications are retained.

The Unix engine uses `fork` checkpoints to roll back within a typesetting pass
and can publish intermediate pages before a pass finishes. The Windows helper
does not provide those checkpoints or within-pass intermediate pages, and a
large project's full pass can take longer. Bibliography tools and user-defined
final build recipes continue to run through the normal final build action.

## Build and validation

No C/C++ compiler, TECkit, ICU, or font-layout development libraries are needed.
The runtime TeX distribution is installed separately, as for final builds.

```sh
cargo build --release --locked --manifest-path PreviewEngine/Windows/Cargo.toml --bin pitex-preview
cargo test --locked --manifest-path PreviewEngine/Windows/Cargo.toml
# Requires xelatex on PATH (TeX Live/TinyTeX: xetex latex-bin cm).
cargo test --locked --manifest-path PreviewEngine/Windows/Cargo.toml --test protocol real_tex -- --ignored
```

The normal tests compile a small native Rust compiler fixture, then exercise
the real helper protocol, unsaved includes, buffer closure, filesystem
changes/deletions, cancellation, bounded publication retention, failure
recovery, Unicode paths, SyncTeX rewriting, and process-tree shutdown.

`PITEX_PREVIEW_TEX_ENGINE` or `--engine` can select an alternative compatible
XeLaTeX executable. `--cache` is accepted for protocol compatibility; the
installed TeX distribution owns its formats and package cache.

## Notices

Pitex is licensed under the GNU AGPL 3.0 or later; see the repository `LICENSE`.
This Windows implementation is original Rust code and does not link the
TeXpresso/XeTeX translated engine or bundle a TeX distribution.

Runtime Rust dependencies: `serde_json`, `serde_core`, `itoa`, `memchr`, `zmij`,
`flate2`, `crc32fast`, `cfg-if`, `miniz_oxide`, `adler2`, `simd-adler32`,
`windows-sys`, and `windows-link` (MIT; several also offer Apache-2.0).
`miniz_oxide` also offers Zlib, `adler2` offers 0BSD, and `memchr` offers
the Unlicense. Dependency copyright and MIT-license texts are in
`THIRD-PARTY-NOTICES.txt`.
Rust standard library uses MIT and Apache-2.0 with bundled third-party notices.
The package stages Rust MIT/Apache-2.0 and standard-library notices from
`PreviewEngine/licenses/` alongside this file.
