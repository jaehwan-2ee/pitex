#!/usr/bin/env bash
# Portable behavior check for BUNDLED-VERSIONS.txt generation.
# Runs the verbatim `record_bundled_deps` function extracted from
# bundle-preview-engine-macos.sh against a fake brew/otool (no network, no
# real Homebrew, no macOS). Asserts on the function's output records — a
# keg is only a valid corresponding source when its version equals the
# formula's versions.stable (+ _revision) AND urls.stable URL+SHA exist.
#
#   bash Tools/check-bundle-source-record.sh
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
WORK=$(mktemp -d "${TMPDIR:-/tmp}/bundle-record-check.XXXXXX")
trap 'rm -rf "$WORK"' EXIT

# sed extracts the verbatim shipped function definition.
sed -n '/^record_bundled_deps() {/,/^}/p' \
    "$ROOT/Tools/bundle-preview-engine-macos.sh" > "$WORK/fn.sh"
[ -s "$WORK/fn.sh" ] || { echo "FAIL: could not extract record_bundled_deps" >&2; exit 1; }
. "$WORK/fn.sh"

# Fake tools: brew answers per formula name; otool emits a fixed dylib id.
mkdir -p "$WORK/bin" "$WORK/fakecellar"
cat > "$WORK/bin/brew" <<'BREWEOF'
#!/usr/bin/env bash
case "$3" in
  zlib)      echo '{"formulae":[{"versions":{"stable":"1.3.2"},"revision":0,"installed":[{"version":"1.3.2"}],"urls":{"stable":{"url":"https://zlib.net/zlib-1.3.2.tar.xz","checksum":"abc123"}}}]}' ;;
  harfbuzz)  echo '{"formulae":[{"versions":{"stable":"15.0.0"},"revision":0,"installed":[{"version":"14.5.0"}],"urls":{"stable":{"url":"https://github.com/harfbuzz/harfbuzz/releases/download/15.0.0/harfbuzz-15.0.0.tar.xz","checksum":"dead"}}}]}' ;;
  libpng)    echo '{"formulae":[{"versions":{"stable":"1.6.58"},"revision":1,"installed":[{"version":"1.6.58_1"}],"urls":{"stable":{"url":"https://example/png.tar.xz","checksum":"beef"}}}]}' ;;
  nourl)     echo '{"formulae":[{"versions":{"stable":"2.0"},"revision":0,"installed":[{"version":"2.0"}],"urls":{"stable":{}}}]}' ;;
  emptyurl)  echo '{"formulae":[{"versions":{"stable":"3.0"},"revision":0,"installed":[{"version":"3.0"}],"urls":{"stable":{"url":"","checksum":"beef"}}}]}' ;;
  nosha)     echo '{"formulae":[{"versions":{"stable":"4.0"},"revision":0,"installed":[{"version":"4.0"}],"urls":{"stable":{"url":"https://example/s.tar.xz"}}}]}' ;;
  nobrew)    exit 1 ;;
  *)         echo '{}' ;;
esac
BREWEOF
cat > "$WORK/bin/otool" <<'OTOOLEOF'
#!/usr/bin/env bash
printf 'stub:\n\t/path/x.dylib (compatibility version 1.0.0, current version 9.9.9)\n'
OTOOLEOF
chmod +x "$WORK/bin/brew" "$WORK/bin/otool"
export PATH="$WORK/bin:$PATH"

cellar() { echo "$WORK/fakecellar/Cellar/$1/$2/lib/$3	$3"; }

fails=0
check() { # check <desc> <want-present-regex> <want-absent-regex-or->
    local desc=$1 want=$2 absent=${3:-}
    if ! grep -q "$want" "$WORK/out.txt"; then
        echo "FAIL: $desc — missing /$want/"; fails=1
    elif [ -n "$absent" ] && grep -q "$absent" "$WORK/out.txt"; then
        echo "FAIL: $desc — unexpected /$absent/"; fails=1
    else
        echo "ok: $desc"
    fi
}

# All cases in one run so order/co-mingling is also exercised.
{
    cellar zlib 1.3.2 libz.dylib                       # version+url+sha ok
    cellar harfbuzz 14.5.0 libharfbuzz.dylib           # installed<stable
    cellar libpng 1.6.58_1 libpng.dylib                # revision suffix ok
    cellar nourl 2.0 libn.dylib                        # urls.stable absent
    cellar emptyurl 3.0 libe.dylib                     # url empty
    cellar nosha 4.0 libs.dylib                        # checksum missing
    cellar nobrew 1.0 libn2.dylib                      # brew info fails
    echo "/opt/priv/lib/libTECkit.0.dylib	libTECkit.0.dylib"   # source-built
} > "$WORK/in.tsv"
record_bundled_deps "$WORK/in.tsv" > "$WORK/out.txt"

check "match records source URL+SHA"   "libz.dylib = zlib 1.3.2.*keg" "^MISMATCH zlib"
check "url+sha recorded"               "https://zlib.net/zlib-1.3.2.tar.xz abc123"
check "revision suffix accepted"       "libpng.dylib = libpng 1.6.58_1" "^MISMATCH libpng"
check "installed<stable mismatches"    "^MISMATCH harfbuzz keg=14.5.0 formula-stable=15.0.0"
check "urls.stable absent mismatches"  "^MISMATCH nourl keg=2.0 formula-stable=2.0 url=none"
check "empty url mismatches"           "^MISMATCH emptyurl keg=3.0.*url=none"
check "missing checksum mismatches"    "^MISMATCH nosha keg=4.0.*sha=none"
check "brew info failure mismatches"   "^MISMATCH nobrew keg=1.0 formula-stable=none"
check "source-built records dylib ver" "libTECkit.0.dylib = source-built (dylib current_version 9.9.9"
# every recorded non-source-built line must carry a url+sha pair
if grep -qE '^\s+source: *$|source: https://\S+ *$' "$WORK/out.txt"; then
    echo "FAIL: a record lacks url or sha"; fails=1
fi

[ "$fails" -eq 0 ] || { echo "check-bundle-source-record FAILED" >&2; exit 1; }
echo "check-bundle-source-record: all checks passed"
