#!/bin/bash
# Bundle the embedded preview engine helpers into a staged .app directory.
#
#   Tools/bundle-preview-engine-macos.sh <BUILD_DIR> <DEST_DIR>
#
#   BUILD_DIR  out-of-repo make output dir (created if missing)
#   DEST_DIR   parent of the PreviewEngine payload, e.g.
#              build/Build/Products/Release/Pitex.app/Contents/Helpers
#
# Produces <DEST_DIR>/PreviewEngine/{pitex-preview,pitex-preview-xetex,lib/}
# plus notices under <DEST_DIR>/../Resources/PreviewEngine/ (the sibling
# Resources dir inside the same .app Contents): every non-system dylib the
# helpers need is copied to lib/ and install names rewritten to @loader_path
# (binaries: @loader_path/lib/x, dylibs: @loader_path/x). All bundled files
# are ad-hoc signed; the script fails if a non-system absolute dylib
# reference remains or a bundled library's license file cannot be located.
# Notices live under Resources, not Helpers: macOS treats Contents/Helpers
# as code, so .txt/.json there would break the outer codesign. TECkit stays
# a shared library so the LGPL replaceability requirement survives.
#
# Dependencies — NEVER installed on a user's machine. Provide them either:
#   a) PKG_CONFIG_PATH=<private prefix>/lib/pkgconfig with .pc files for
#      freetype2 harfbuzz graphite2 libpng zlib icu-uc teckit (the Makefile
#      also honours TECKIT_CFLAGS/TECKIT_LIBS and ICU_PREFIX directly), plus
#      DEP_SEARCH_DIRS=<colon-separated lib dirs> for dylib lookup and
#      DEP_LICENSE_DIR=<dir of license files> for per-library notices; or
#   b) Homebrew on PATH (a private clone works — `brew --prefix` is used,
#      nothing is written outside its prefix). Missing formulae are only
#      auto-installed when PITEX_BUNDLE_INSTALL_DEPS=1, meant for ephemeral
#      CI runners.
# Runs on macOS with the CLT toolchain (cc/c++/make); Xcode is not needed.

set -euo pipefail

BUILD_DIR=${1:?usage: bundle-preview-engine-macos.sh <BUILD_DIR> <DEST_DIR>}
DEST_DIR=${2:?usage: bundle-preview-engine-macos.sh <BUILD_DIR> <DEST_DIR>}
REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
PKG_CONFIG=${PKG_CONFIG:-pkg-config}

# DEST_DIR must be <App>.app/Contents/Helpers so that ../Resources is the
# app's real Resources dir — notices never land outside the bundle.
# realpath (no -e requirement): the dir is created below.
DEST_DIR=$(python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "$DEST_DIR")
[ "$(basename "$DEST_DIR")" = Helpers ] && [ "$(basename "$(dirname "$DEST_DIR")")" = Contents ] \
    || { echo "bundle: DEST_DIR must be <App>.app/Contents/Helpers (got $DEST_DIR)" >&2; exit 2; }
mkdir -p "$DEST_DIR"
[ "$(uname)" = Darwin ] || { echo "bundle-preview-engine-macos.sh: macOS only" >&2; exit 2; }
BREW=$(command -v brew >/dev/null 2>&1 && brew --prefix) || BREW=

# Caller-provided ICU prefix feeds pkg-config before the dependency check.
if [ -n "${ICU_PREFIX:-}" ] && [ -d "$ICU_PREFIX/lib/pkgconfig" ]; then
    export PKG_CONFIG_PATH="$ICU_PREFIX/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
fi
# Keg-only formulae (icu4c@*, zlib, …) are never linked into $BREW/lib or
# $BREW/lib/pkgconfig — expose every installed keg's pc dir so the
# dependency check and the Makefile find them without extra env wiring.
if [ -n "$BREW" ]; then
    for pcdir in "$BREW"/opt/*/lib/pkgconfig; do
        [ -d "$pcdir" ] && export PKG_CONFIG_PATH="$pcdir${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
    done
fi

dep_check() {
    command -v "$PKG_CONFIG" >/dev/null 2>&1 || { MISSING=" pkg-config"; return; }
    local p
    for p in freetype2 harfbuzz graphite2 libpng zlib icu-uc; do
        "$PKG_CONFIG" --exists "$p" 2>/dev/null || MISSING="$MISSING $p"
    done
    # TECKIT_* env vars bypass pkg-config entirely (see the Makefile).
    if [ -z "${TECKIT_LIBS:-}" ] || [ -z "${TECKIT_CFLAGS:-}" ]; then
        "$PKG_CONFIG" --exists teckit 2>/dev/null || MISSING="$MISSING teckit"
    fi
}

MISSING=
dep_check
if [ -n "$MISSING" ]; then
    if [ "${PITEX_BUNDLE_INSTALL_DEPS:-0}" = 1 ] && [ -n "$BREW" ]; then
        for f in freetype harfbuzz graphite2 icu4c libpng teckit zlib pkgconf; do
            brew list --versions "$f" >/dev/null 2>&1 || brew install "$f"
        done
        # macOS provides libz but not always zlib.pc — use the keg-only
        # Homebrew zlib's .pc when the SDK doesn't ship one.
        if [ -d "$BREW/opt/zlib/lib/pkgconfig" ]; then
            export PKG_CONFIG_PATH="$BREW/opt/zlib/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
        fi
        dep_check
    fi
fi
if [ -n "$MISSING" ]; then
    cat >&2 <<EOF
bundle: missing build dependencies:$MISSING
  Provide a private prefix via PKG_CONFIG_PATH=<prefix>/lib/pkgconfig (and
  DEP_SEARCH_DIRS=<lib dirs> + DEP_LICENSE_DIR=<licenses>), or TECKIT_*
  /ICU_PREFIX overrides. Automatic Homebrew installs are opt-in and meant
  for ephemeral CI only: PITEX_BUNDLE_INSTALL_DEPS=1.
EOF
    exit 2
fi

make -C "$REPO_ROOT/PreviewEngine" BUILD_DIR="$BUILD_DIR"

BIN_DIR="$BUILD_DIR/bin"
PAYLOAD="$DEST_DIR/PreviewEngine"
LIB_DIR="$PAYLOAD/lib"
LIC_DIR="$DEST_DIR/../Resources/PreviewEngine"
mkdir -p "$LIB_DIR" "$LIC_DIR"
cp -f "$BIN_DIR/pitex-preview" "$BIN_DIR/pitex-preview-xetex" "$PAYLOAD/"
chmod 755 "$PAYLOAD/pitex-preview" "$PAYLOAD/pitex-preview-xetex"

WORK=$(mktemp -d "${TMPDIR:-/tmp}/pitex-preview-bundle.XXXXXX")
trap 'rm -rf "$WORK"' EXIT
: > "$WORK/copied.tsv"   # realpath \t basename   (unique per row)
: > "$WORK/licdirs"      # DEP_LICENSE_DIRs already copied
QUEUE="$WORK/queue"      # one dylib realpath per line, FIFO order
: > "$QUEUE"

realpath_of() { python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "$1"; }
realpath_file() {
    local rp; rp=$(realpath_of "$1") || return 1
    [ -f "$rp" ] && printf '%s\n' "$rp" || return 1
}


# Resolve one load-command entry to a realpath on disk; $2 = the file that
# recorded the ref (needed for @loader_path, which resolves relative to the
# referrer's directory — e.g. ICU dylibs sit beside each other in a keg's
# lib/). Prints nothing for system libraries; returns 1 when a non-system
# dep cannot be found. Search order: absolute path, referrer dir for
# @loader_path/@executable_path, $BREW/lib, every Cellar keg's lib dir
# (keg-only formulae never land in $BREW/lib), then DEP_SEARCH_DIRS.
resolve_dep() {
    local dep=$1 referrer=$2 name cand keg d
    case $dep in
        /usr/lib/*|/System/*) return 0 ;;
        /*) realpath_file "$dep" || return 1; return 0 ;;
        @loader_path/*)
            cand="$(dirname "$referrer")/${dep#@loader_path/}"
            [ -f "$cand" ] && { realpath_file "$cand"; return 0; }
            return 1 ;;
        @executable_path/*)
            # Refs relative to the loading executable — the payload dir.
            cand="$PAYLOAD/${dep#@executable_path/}"
            [ -f "$cand" ] && { realpath_file "$cand"; return 0; }
            return 1 ;;
        @rpath/*)
            name=${dep#@rpath/}
            if [ -n "$BREW" ]; then
                for cand in "$BREW/lib/$name" "$BREW/lib/$(basename "$name")"; do
                    [ -f "$cand" ] && { realpath_file "$cand" || continue; return 0; }
                done
                for keg in "$BREW/Cellar"/*/*/; do
                    [ -f "$keg/lib/$name" ] && { realpath_file "$keg/lib/$name" || continue; return 0; }
                done
            fi
            for d in $(echo "${DEP_SEARCH_DIRS:-}" | tr ':' ' '); do
                [ -f "$d/$name" ] && { realpath_file "$d/$name" || continue; return 0; }
            done
            return 1 ;;
    esac
    return 0
}

# Enqueue a file's non-system dylib refs. $1 = file as it exists on disk
# (binary or copied-dylib source), $2 = referrer basename used as the
# rewrite-map key (map.<key> rows: recorded name \t resolved realpath).
enqueue_deps() {
    local file=$1 key=$2 d resolved base depbase selfrp
    selfrp=$(realpath_of "$file")
    while IFS= read -r d; do
        resolved=$(resolve_dep "$d" "$file") || {
            echo "bundle: unresolved dep $d (of $key)" >&2; exit 1; }
        [ -n "$resolved" ] || continue
        # A dylib's own install name shows up in its otool output — don't
        # record a ref that resolves to the file itself.
        [ "$selfrp" = "$resolved" ] && continue
        printf '%s\t%s\n' "$d" "$resolved" >> "$WORK/map.$key"
        if ! awk -F '\t' -v s="$resolved" '$1==s{f=1} END{exit !f}' "$WORK/copied.tsv"; then
            base=$(basename "$resolved")
            if awk -F '\t' -v b="$base" '$2==b{f=1} END{exit f?0:1}' "$WORK/copied.tsv"; then
                echo "bundle: dylib basename collision: $resolved" >&2; exit 1
            fi
            printf '%s\t%s\n' "$resolved" "$base" >> "$WORK/copied.tsv"
            printf '%s\n' "$resolved" >> "$QUEUE"
        fi
    done < <(otool -L "$file" | awk 'NR>1 {print $1}')
}

# Seed the queue with both binaries' deps, then BFS through copied dylibs.
for bin in pitex-preview pitex-preview-xetex; do
    enqueue_deps "$PAYLOAD/$bin" "$bin"
done
while [ -s "$QUEUE" ]; do
    src=$(head -1 "$QUEUE"); tail -n +2 "$QUEUE" > "$QUEUE.next"; mv "$QUEUE.next" "$QUEUE"
    cp -f "$src" "$LIB_DIR/$(basename "$src")"; chmod 755 "$LIB_DIR/$(basename "$src")"
    enqueue_deps "$src" "$(basename "$src")"
done

# Rewrite: binaries reference @loader_path/lib/<name>; dylibs reference
# @loader_path/<name> and carry id @loader_path/<name> — a sibling-style id
# resolves against the dylib's own directory under dyld and under
# audit-linkage's recursive otool walk.
for bin in pitex-preview pitex-preview-xetex; do
    f="$PAYLOAD/$bin"
    [ -f "$WORK/map.$bin" ] || continue
    while IFS=$'\t' read -r recorded resolved; do
        base=$(awk -F '\t' -v s="$resolved" '$1==s{print $2}' "$WORK/copied.tsv")
        install_name_tool -change "$recorded" "@loader_path/lib/$base" "$f"
    done < "$WORK/map.$bin"
done
while IFS=$'\t' read -r src base; do
    dest="$LIB_DIR/$base"
    install_name_tool -id "@loader_path/$base" "$dest"
    [ -f "$WORK/map.$base" ] || continue
    while IFS=$'\t' read -r recorded resolved; do
        depbase=$(awk -F '\t' -v s="$resolved" '$1==s{print $2}' "$WORK/copied.tsv")
        install_name_tool -change "$recorded" "@loader_path/$depbase" "$dest"
    done < "$WORK/map.$base"
done < "$WORK/copied.tsv"

# Nothing but system libs and @loader_path may remain.
fail=0
for f in "$PAYLOAD/pitex-preview" "$PAYLOAD/pitex-preview-xetex" "$LIB_DIR"/*; do
    [ -e "$f" ] || continue
    while IFS= read -r d; do
        case $d in
            /usr/lib/*|/System/*|@loader_path/*) ;;
            *) echo "bundle: unbundled ref $d in $(basename "$f")" >&2; fail=1 ;;
        esac
    done < <(otool -L "$f" | awk 'NR>1 {print $1}')
done
[ "$fail" -eq 0 ] || { echo "bundle: refusing to ship unresolved refs" >&2; exit 1; }

# Per-library licenses. Homebrew kegs: LICENSE*/COPYING*/share/doc straight
# from the keg (<brew>/Cellar/<formula>/<ver>/lib/<dylib>). Non-Homebrew
# private prefixes: DEP_LICENSE_DIR's contents land once under
# licenses/external/ — set it so every bundled dylib's notice ships. No
# identifiable license source → abort: shipping binaries without their
# notices is not an option.
while IFS=$'\t' read -r src base; do
    case $src in
        */Cellar/*/*/lib/*)
            keg=${src%%/lib/*}
            fname=$(basename "$(dirname "$keg")")
            out="$LIC_DIR/$fname"
            mkdir -p "$out"
            found=0
            for lic in "$keg"/LICENSE* "$keg"/LICENCE* "$keg"/COPYING* "$keg"/NOTICE* "$keg"/COPYRIGHT*; do
                if [ -f "$lic" ]; then cp -f "$lic" "$out/"; found=1; fi
            done
            if [ -d "$keg/share/doc" ]; then cp -Rf "$keg/share/doc" "$out/doc"; found=1; fi
            [ "$found" -eq 1 ] || { echo "bundle: no license files in keg $keg for $base" >&2; exit 1; }
            ;;
        *)
            if [ -n "${DEP_LICENSE_DIR:-}" ] && [ -d "$DEP_LICENSE_DIR" ]; then
                if ! grep -qxF "$DEP_LICENSE_DIR" "$WORK/licdirs" 2>/dev/null; then
                    mkdir -p "$LIC_DIR/external"
                    cp -Rf "$DEP_LICENSE_DIR/." "$LIC_DIR/external/"
                    echo "$DEP_LICENSE_DIR" >> "$WORK/licdirs"
                fi
            else
                echo "bundle: cannot locate license for non-Homebrew dylib $src (set DEP_LICENSE_DIR)" >&2
                exit 1
            fi ;;
    esac
done < "$WORK/copied.tsv"

# Repo-side notices: engine provenance + every bundled license text.
cp -f "$REPO_ROOT"/PreviewEngine/licenses/* "$LIC_DIR/"
for p in PROVENANCE.md provenance.json; do
    if [ ! -f "$REPO_ROOT/PreviewEngine/$p" ]; then
        echo "bundle: PreviewEngine/$p missing — required in the bundle" >&2
        exit 1
    fi
    cp -f "$REPO_ROOT/PreviewEngine/$p" "$LIC_DIR/$p"
done

# BUNDLED-VERSIONS.txt — the corresponding-source record for every bundled
# dylib, generated from the actual copied files (never hardcoded versions).
# Homebrew dylibs: formula name + installed keg version, verified to equal
# the formula's current `versions.stable` (+`_<revision>` when nonzero) —
# installed[].version alone does NOT prove urls.stable matches the keg, so
# any mismatch or missing metadata fails the bundle rather than recording a
# false source correspondence. Source-built dylibs (private prefix): the
# version recorded in the dylib itself, with provenance.json's
# source_availability as the pinned-source reference.
# Extracted for testability: the probe evals this function verbatim.
record_bundled_deps() {
    while IFS=$'\t' read -r src base; do
        case $src in
            */Cellar/*/lib/*)
                keg=${src%%/lib/*}
                kegver=$(basename "$keg")
                formula=$(basename "$(dirname "$keg")")
                info=$(brew info --json=v2 "$formula" 2>/dev/null || true)
                line=$(printf '%s' "$info" | python3 -c 'import json,sys
try:
  f=json.load(sys.stdin)["formulae"][0]
  v=f.get("versions",{}).get("stable",""); r=f.get("revision",0) or 0
  u=f.get("urls",{}).get("stable",{})
  print(v, r, u.get("url",""), u.get("checksum",""), sep="|")
except Exception: print("")' 2>/dev/null)
                IFS='|' read -r stable_ver revision src_url src_sha <<EOF
$line
EOF
                exp="$stable_ver"; [ "${revision:-0}" != "0" ] && exp="$stable_ver""_$revision"
                # versions.stable equality alone is not source availability:
                # the pinned source URL AND its checksum must also be known.
                if [ -z "$stable_ver" ] || [ "$kegver" != "$exp" ] || [ -z "$src_url" ] || [ -z "$src_sha" ]; then
                    echo "MISMATCH $formula keg=$kegver formula-stable=${exp:-none} url=${src_url:-none} sha=${src_sha:-none}"
                    continue
                fi
                printf '%s = %s %s (keg %s)\n    source: %s %s\n' \
                    "$base" "$formula" "$kegver" "$keg" "$src_url" "$src_sha"
                ;;
            *)
                cur=$(otool -L "$src" | awk 'NR==2{gsub(/[()]/,""); print $NF}')
                printf '%s = source-built (dylib current_version %s; %s)\n    source: pinned archive in provenance.json runtime_dependencies.source_availability\n' \
                    "$base" "${cur:-unknown}" "$src"
                ;;
        esac
    done < "$1"
}
{
    echo "# Bundled shared libraries and their corresponding-source record."
    echo "# Generated from the actual copied files on every bundle run."
    echo "# License expressions: provenance.json 'runtime_dependencies'."
    echo
    record_bundled_deps "$WORK/copied.tsv"
} > "$LIC_DIR/BUNDLED-VERSIONS.txt"
if grep -q "^MISMATCH" "$LIC_DIR/BUNDLED-VERSIONS.txt"; then
    echo "bundle: Homebrew keg not at the formula's stable version, so urls.stable cannot be trusted as its corresponding source:" >&2
    grep "^MISMATCH" "$LIC_DIR/BUNDLED-VERSIONS.txt" >&2
    echo "bundle: run 'brew upgrade <formula>' to match the formula, or build/pin the exact source archive instead (see DEP_LICENSE_DIR path for source-built deps)." >&2
    exit 1
fi

# Ad-hoc sign every bundled binary so Gatekeeper sees a consistent bundle
# (the outer `codesign --deep --force --sign -` on the .app re-seals).
for f in "$PAYLOAD/pitex-preview" "$PAYLOAD/pitex-preview-xetex" "$LIB_DIR"/*; do
    [ -e "$f" ] || continue
    codesign --force --sign - "$f"
done

echo "bundled preview engine -> $PAYLOAD (+ notices -> $LIC_DIR)"
