#!/usr/bin/env bash
# Regression check for PreviewEngine/tools/audit-linkage.sh on Linux.
# Fakes a Darwin environment and exercises the @loader_path traversal through
# paths that contain spaces and through a forbidden-library case.
set -euo pipefail

cd "$(dirname "$0")/.."
root=$(mktemp -d "/tmp/pitex audit linkage.XXXXXX")
trap 'rm -rf -- "$root"' EXIT

bin_dir="$root/bin"
helpers_dir="$root/Pitex Nightly.app/Contents/Helpers"
mkdir -p "$bin_dir" "$helpers_dir"
export PATH="$bin_dir:$PATH"

cat > "$bin_dir/uname" <<'EOF'
#!/bin/sh
echo Darwin
EOF
chmod +x "$bin_dir/uname"

cat > "$bin_dir/otool" <<'EOF'
#!/bin/sh
base=$(basename "$2")
echo "Binary:"
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
  libmupdf.dylib)
    echo "    /usr/lib/libSystem.dylib (compatibility version 1.0.0)"
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

for exe in pitex-preview pitex-preview-xetex; do
  touch "$helpers_dir/$exe"
  chmod +x "$helpers_dir/$exe"
done

out=$(PreviewEngine/tools/audit-linkage.sh "$helpers_dir" 2>&1) || { printf '%s\n' "$out"; exit 1; }
printf '%s\n' "$out"

printf '%s\n' "$out" | grep -q "libdep.dylib" || { echo "FAIL: did not traverse @loader_path chain" >&2; exit 1; }
printf '%s\n' "$out" | grep -q "libSystem.dylib" || { echo "FAIL: did not reach system library" >&2; exit 1; }
printf '%s\n' "$out" | grep -q "audit OK" || { echo "FAIL: did not report audit OK" >&2; exit 1; }

forbidden_dir="$root/Forbidden Test.app/Contents/Helpers"
mkdir -p "$forbidden_dir"
touch "$forbidden_dir/pitex-preview" "$forbidden_dir/libmupdf.dylib"
chmod +x "$forbidden_dir/pitex-preview"

if PreviewEngine/tools/audit-linkage.sh "$forbidden_dir" >/dev/null 2>&1; then
  echo "FAIL: forbidden library should have failed the audit" >&2
  exit 1
fi

echo "check-audit-linkage-paths OK"
