#!/usr/bin/env bash
# Audit the embedded preview helpers' dependency boundary.
#
#   PreviewEngine/tools/audit-linkage.sh <dir containing pitex-preview*> [notices dir]
#
# 1. Source: preview-provenance verify (every file accounted for, upstream notices
#    retained, no GPL dpx / vendored TECkit / MuPDF renderer sources).
# 2. Link closure: every shared library the helpers load (ldd on Linux,
#    recursive otool -L on macOS) is listed with its origin; MuPDF, SDL,
#    Ghostscript, Poppler or xdvipdfmx libraries fail the audit.
# 3. Symbols (supplementary): no MuPDF (fz_), xdvipdfmx (dpx_/pdf_doc_)
#    definitions; TECkit_* must be imported from the shared library, never
#    defined inside the engine.
# 4. Notices dir (optional, e.g. Contents/Resources/PreviewEngine in a macOS
#    bundle — licenses and provenance must NOT sit under Contents/Helpers,
#    which macOS treats as code): requires PROVENANCE.md, provenance.json and
#    the four repo license texts to be present.
# Evidence of the technical boundary only — not a legal determination.
set -euo pipefail

BIN=${1:?usage: audit-linkage.sh <bin dir> [notices dir]}
NOTICES=${2:-}
HERE=$(cd "$(dirname "$0")" && pwd)
FAIL=0
fail() { echo "FAIL: $*"; FAIL=1; }

if [ -n "$NOTICES" ]; then
    echo "== notices: $NOTICES"
    for f in PROVENANCE.md provenance.json THIRD-PARTY.md TeXpresso-LICENSE.txt Tectonic-MIT.txt XeTeX-SIL-MIT.txt BUNDLED-VERSIONS.txt; do
        [ -f "$NOTICES/$f" ] || fail "notices missing $f"
    done
fi

echo "== source provenance"
cargo run --locked --quiet --manifest-path "$HERE/../../Tools/Native/Cargo.toml" --bin preview-provenance -- verify || FAIL=1

FORBIDDEN_LIB='mupdf|libSDL|libgs\.|ghostscript|poppler|dvipdfm'
for exe in pitex-preview pitex-preview-xetex; do
  path="$BIN/$exe"
  [ -x "$path" ] || { fail "$path missing"; continue; }
  echo "== link closure: $exe"
  case "$(uname)" in
    Linux)
      ldd "$path" | awk '{print $1, $3}' | while read -r name file; do
        [ -n "$name" ] || continue
        pkg=$(dpkg -S "$(readlink -f "${file:-/nonexistent}")" 2>/dev/null | head -1 | cut -d: -f1 || true)
        [ -n "$pkg" ] || pkg=$(dpkg -S "${file:-/nonexistent}" 2>/dev/null | head -1 | cut -d: -f1 || true)
        printf '  %-32s %-28s %s\n' "$name" "${pkg:-?}" "${file:-}"
      done
      if ldd "$path" | grep -Ei "$FORBIDDEN_LIB" >/dev/null; then fail "$exe links a forbidden library"; fi
      defined=$(nm -C --defined-only "$path" 2>/dev/null || true)
      undefined=$(nm -D --undefined-only "$path" 2>/dev/null || true)
      ;;
    Darwin)
      seen=""
      queue="$path"
      while [ -n "$queue" ]; do
        cur=${queue%% *}; queue=${queue#"$cur"}; queue=${queue# }
        case " $seen " in *" $cur "*) continue ;; esac
        seen="$seen $cur"
        otool -L "$cur" | tail -n +2 | awk '{print $1}' | while read -r lib; do
          printf '  %-60s <- %s\n' "$lib" "$(basename "$cur")"
        done
        for lib in $(otool -L "$cur" | tail -n +2 | awk '{print $1}'); do
          case "$lib" in
            /usr/lib/*|/System/*) ;;
            @loader_path/*) queue="$queue $(dirname "$cur")/${lib#@loader_path/}" ;;
            @executable_path/*) queue="$queue $BIN/${lib#@executable_path/}" ;;
            /*) queue="$queue $lib" ;;
          esac
        done
      done
      if grep -Eiq "$FORBIDDEN_LIB" <<< "$seen"; then fail "$exe links a forbidden library"; fi
      defined=$(nm -gU "$path" 2>/dev/null || true)
      undefined=$(nm -gu "$path" 2>/dev/null || true)
      ;;
    *) fail "unsupported platform $(uname)" ;;
  esac
  # Here-strings avoid SIGPIPE from a producer when grep exits after a match.
  # With pipefail, large symbol tables otherwise invert successful matches.
  if grep -Eq '\b_?(fz_[a-z]|dpx_|pdf_doc_|pdf_dev_)' <<< "$defined"; then
    fail "$exe defines MuPDF/xdvipdfmx symbols"
  fi
  if grep -Eq '\b_?TECkit_' <<< "$defined"; then
    fail "$exe defines TECkit symbols (TECkit must stay a replaceable shared library)"
  fi
  if [ "$exe" = pitex-preview-xetex ] && ! grep -q 'TECkit_ConvertBuffer' <<< "$undefined"; then
    fail "pitex-preview-xetex does not import TECkit_ConvertBuffer from a shared library"
  fi
done

if [ "$FAIL" -ne 0 ]; then
  echo "audit FAILED"
  exit 1
fi
echo "audit OK"
