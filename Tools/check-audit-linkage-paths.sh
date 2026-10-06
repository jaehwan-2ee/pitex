#!/usr/bin/env bash
# Regression check for PreviewEngine/tools/audit-linkage.sh on Linux.
# Fakes a Darwin environment and exercises @loader_path traversal through
# paths containing spaces, including a cycle, a diamond, a self-referential
# library, and a forbidden-library case. Wraps every audit run in timeout(1).
set -euo pipefail

cd "$(dirname "$0")/.."
root=$(mktemp -d "/tmp/pitex audit linkage.XXXXXX")
trap 'rm -rf -- "$root"' EXIT

bin_dir="$root/bin"
mkdir -p "$bin_dir"
export PATH="$bin_dir:$PATH"

cat > "$bin_dir/uname" <<'EOF'
#!/bin/sh
echo Darwin
EOF
chmod +x "$bin_dir/uname"

cat > "$bin_dir/otool" <<'EOF'
#!/bin/sh
path=$2
base=$(basename "$path")
helpers=$(dirname "$path")
contents=$(dirname "$helpers")
app=$(basename "$(dirname "$contents")")

echo "Binary:"
case "$app" in
  "Pitex Straight.app")
    case "$base" in
      pitex-preview)
        echo "    @loader_path/libhelper.dylib (compatibility version 1.0.0)"
        ;;
      libhelper.dylib)
        echo "    @loader_path/libdep.dylib (compatibility version 1.0.0)"
        ;;
      libdep.dylib)
        echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
        ;;
      pitex-preview-xetex)
        echo "    @loader_path/libxetexhelper.dylib (compatibility version 1.0.0)"
        ;;
      libxetexhelper.dylib)
        echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
        ;;
    esac
    ;;
  "Pitex Cycle.app")
    case "$base" in
      pitex-preview)
        echo "    @loader_path/cycle-A.dylib (compatibility version 1.0.0)"
        ;;
      pitex-preview-xetex)
        echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
        ;;
      cycle-A.dylib)
        echo "    @loader_path/cycle-B.dylib (compatibility version 1.0.0)"
        ;;
      cycle-B.dylib)
        echo "    @loader_path/cycle-A.dylib (compatibility version 1.0.0)"
        ;;
    esac
    ;;
  "Pitex Diamond.app")
    case "$base" in
      pitex-preview)
        echo "    @loader_path/diamond-B.dylib (compatibility version 1.0.0)"
        echo "    @loader_path/diamond-C.dylib (compatibility version 1.0.0)"
        ;;
      pitex-preview-xetex)
        echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
        ;;
      diamond-B.dylib)
        echo "    @loader_path/diamond-D.dylib (compatibility version 1.0.0)"
        ;;
      diamond-C.dylib)
        echo "    @loader_path/diamond-D.dylib (compatibility version 1.0.0)"
        ;;
      diamond-D.dylib)
        echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
        ;;
    esac
    ;;
  "Pitex Selfref.app")
    case "$base" in
      pitex-preview)
        echo "    @loader_path/selfref.dylib (compatibility version 1.0.0)"
        ;;
      pitex-preview-xetex)
        echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
        ;;
      selfref.dylib)
        echo "    @loader_path/selfref.dylib (compatibility version 1.0.0)"
        echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
        ;;
    esac
    ;;
  "Forbidden Test.app")
    case "$base" in
      pitex-preview|pitex-preview-xetex)
        echo "    @loader_path/libmupdf.dylib (compatibility version 1.0.0)"
        ;;
      libmupdf.dylib)
        echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
        ;;
    esac
    ;;
esac
EOF
chmod +x "$bin_dir/otool"

cat > "$bin_dir/nm" <<'EOF'
#!/usr/bin/env bash
args=("$@")
path=${args[${#args[@]} - 1]}
base=$(basename "$path")
case "${args[*]}" in
  *-gU*) exit 0 ;;
  *-gu*)
    if [ "$base" = "pitex-preview-xetex" ]; then
      echo "TECkit_ConvertBuffer"
    fi
    ;;
esac
EOF
chmod +x "$bin_dir/nm"

cat > "$bin_dir/cargo" <<'EOF'
#!/bin/sh
# Stand in for the preview-provenance cargo run during this path check.
exit 0
EOF
chmod +x "$bin_dir/cargo"

helpers() {
  local app="$1"
  local dir="$root/$app/Contents/Helpers"
  mkdir -p "$dir"
  for exe in pitex-preview pitex-preview-xetex; do
    touch "$dir/$exe"
    chmod +x "$dir/$exe"
  done
  case "$app" in
    "Pitex Cycle.app") touch "$dir/cycle-A.dylib" "$dir/cycle-B.dylib" ;;
    "Pitex Diamond.app") touch "$dir/diamond-B.dylib" "$dir/diamond-C.dylib" "$dir/diamond-D.dylib" ;;
    "Pitex Selfref.app") touch "$dir/selfref.dylib" ;;
    "Forbidden Test.app") touch "$dir/libmupdf.dylib" ;;
  esac
  echo "$dir"
}

straight=$(helpers "Pitex Straight.app")
cycle=$(helpers "Pitex Cycle.app")
diamond=$(helpers "Pitex Diamond.app")
selfref=$(helpers "Pitex Selfref.app")
forbidden=$(helpers "Forbidden Test.app")

run_audit() {
  local dir="$1"
  shift
  timeout 20s PreviewEngine/tools/audit-linkage.sh "$dir" 2>&1 || local rc=$?
  : "${rc:=0}"
  if [ "$rc" -eq 124 ]; then
    echo "TIMEOUT" >&2
    return 124
  fi
  return "$rc"
}

count_line() {
  local text="$1"
  local needle="$2"
  printf '%s\n' "$text" | grep -cE -- "$needle" || true
}

# Straight chain through a path containing a space.
out=$(run_audit "$straight") || { echo "FAIL: straight chain audit" >&2; printf '%s\n' "$out"; exit 1; }
printf '%s\n' "$out"
[ "$(count_line "$out" "libdep\\.dylib")" -eq 2 ] || { echo "FAIL: straight chain visit count" >&2; exit 1; }
printf '%s\n' "$out" | grep -q "audit OK" || { echo "FAIL: straight chain not OK" >&2; exit 1; }

# Cycle A -> B -> A must terminate (not loop forever).
out=$(run_audit "$cycle") || { echo "FAIL: cycle audit" >&2; printf '%s\n' "$out"; exit 1; }
printf '%s\n' "$out"
[ "$(count_line "$out" "cycle-A\\.dylib.*<- cycle-B\\.dylib")" -eq 1 ] || { echo "FAIL: cycle-A from cycle-B count" >&2; exit 1; }
[ "$(count_line "$out" "cycle-B\\.dylib.*<- cycle-A\\.dylib")" -eq 1 ] || { echo "FAIL: cycle-B from cycle-A count" >&2; exit 1; }
printf '%s\n' "$out" | grep -q "audit OK" || { echo "FAIL: cycle not OK" >&2; exit 1; }

# Diamond A -> B, A -> C, B -> D, C -> D must visit D once.
out=$(run_audit "$diamond") || { echo "FAIL: diamond audit" >&2; printf '%s\n' "$out"; exit 1; }
printf '%s\n' "$out"
[ "$(count_line "$out" "diamond-D\\.dylib.*<- diamond-B\\.dylib")" -eq 1 ] || { echo "FAIL: diamond-D from B count" >&2; exit 1; }
[ "$(count_line "$out" "diamond-D\\.dylib.*<- diamond-C\\.dylib")" -eq 1 ] || { echo "FAIL: diamond-D from C count" >&2; exit 1; }
[ "$(count_line "$out" "libSystem\\.dylib.*<- diamond-D\\.dylib")" -eq 1 ] || { echo "FAIL: diamond-D system-lib count" >&2; exit 1; }
printf '%s\n' "$out" | grep -q "audit OK" || { echo "FAIL: diamond not OK" >&2; exit 1; }

# Self-referential LC_ID_DYLIB line must not recurse.
out=$(run_audit "$selfref") || { echo "FAIL: selfref audit" >&2; printf '%s\n' "$out"; exit 1; }
printf '%s\n' "$out"
[ "$(count_line "$out" "selfref\\.dylib.*<- selfref\\.dylib")" -eq 1 ] || { echo "FAIL: selfref self-line count" >&2; exit 1; }
[ "$(count_line "$out" "libSystem\\.dylib.*<- selfref\\.dylib")" -eq 1 ] || { echo "FAIL: selfref system-lib count" >&2; exit 1; }
printf '%s\n' "$out" | grep -q "audit OK" || { echo "FAIL: selfref not OK" >&2; exit 1; }

# Forbidden library must still fail.
if out=$(run_audit "$forbidden"); then
  echo "FAIL: forbidden library should have failed the audit" >&2
  exit 1
fi
echo "Forbidden case failed as expected"

echo "check-audit-linkage-paths OK"
