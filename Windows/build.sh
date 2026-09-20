#!/usr/bin/env bash
# Builds pitex.exe (release) and bundles the GTK4 runtime into dist/.
#
# Runs under MSYS2 UCRT64:
#   bash Windows/build.sh [output-name]
#
# Requires: mingw-w64-ucrt-x86_64-{gcc,rust,pkgconf,gtk4,libadwaita,
# gtksourceview5,poppler,adwaita-icon-theme,hicolor-icon-theme}
#
# Layout (everything resolves relative to the DLLs — no env, no installer):
#   dist/pitex/bin/pitex.exe + *.dll
#   dist/pitex/etc/fonts/fonts.conf      minimal, points at Windows fonts
#   dist/pitex/share/icons/{Adwaita,hicolor}
#   dist/pitex/lib/{gdk-pixbuf-2.0,gtk-4.0}  dynamic modules, if present
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${MINGW_PREFIX:-/ucrt64}"
DIST="$HERE/dist/pitex"
BIN="$DIST/bin"

echo "==> cargo build --release"
cargo build --release --manifest-path "$HERE/Cargo.toml" -p pitex-windows

rm -rf "$DIST"
mkdir -p "$BIN" "$DIST/etc/fonts" "$DIST/share/icons" "$DIST/lib"
cp "$HERE/target/release/pitex.exe" "$BIN/"

echo "==> collecting runtime DLLs"
ldd "$BIN/pitex.exe" | awk '/=> \// {print $3}' | while read -r dll; do
    case "$dll" in
        "$PREFIX"/*|/ucrt64/*|/mingw64/*|/clang64/*) cp "$dll" "$BIN/" ;;
    esac
done
# Second pass: DLLs the first set itself needs (e.g. libadwaita -> libgtk).
changed=1
while [ "$changed" = 1 ]; do
    changed=0
    for f in "$BIN"/*.dll; do
        for dep in $(ldd "$f" 2>/dev/null | awk '/=> \// {print $3}'); do
            case "$dep" in
                "$PREFIX"/*|/ucrt64/*|/mingw64/*|/clang64/*)
                    base="$(basename "$dep")"
                    if [ ! -f "$BIN/$base" ]; then cp "$dep" "$BIN/"; changed=1; fi ;;
            esac
        done
    done
done

echo "==> runtime resources"
# Dynamically loaded modules that ldd cannot see.
for module_dir in \
    "lib/gdk-pixbuf-2.0/2.10.0/loaders" \
    "lib/gtk-4.0/4.0.0/immodules" \
    "lib/gtk-4.0/4.0.0/media" \
    "lib/gtk-4.0/4.0.0/printbackends"; do
    if [ -d "$PREFIX/$module_dir" ]; then
        mkdir -p "$DIST/$module_dir"
        cp "$PREFIX/$module_dir"/*.dll "$DIST/$module_dir/" 2>/dev/null || true
    fi
done

# Icon themes — the UI is built from -symbolic icons (Adwaita set).
for theme in Adwaita hicolor; do
    if [ -d "$PREFIX/share/icons/$theme" ]; then
        cp -r "$PREFIX/share/icons/$theme" "$DIST/share/icons/"
    fi
done
# Drop the heaviest legacy PNG dirs — the app uses symbolic/scalable SVGs.
rm -rf "$DIST/share/icons/Adwaita/cursors" "$DIST/share/icons/Adwaita/96x96" \
       "$DIST/share/icons/Adwaita/256x256" "$DIST/share/icons/Adwaita/512x512" 2>/dev/null || true

mkdir -p "$DIST/share/icons/hicolor/512x512/apps"
cp "$HERE/../Linux/packaging/dev.pitex.app.png" "$DIST/share/icons/hicolor/512x512/apps/"

# Minimal fontconfig — stock MSYS2 fonts.conf names /ucrt64 paths that do not
# exist outside MSYS2; main() sets FONTCONFIG_FILE to this file.
cat > "$DIST/etc/fonts/fonts.conf" <<'EOF'
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
  <dir>C:/Windows/Fonts</dir>
  <dir>~/AppData/Local/Microsoft/Windows/Fonts</dir>
  <cachedir>~/.cache/fontconfig</cachedir>
</fontconfig>
EOF

echo "==> bundle ready: $DIST"
du -sh "$DIST"
