#!/usr/bin/env bash
# Builds pitex.exe (release) and bundles the GTK4 runtime into dist/.
#
# Runs under MSYS2 UCRT64:
#   bash Windows/build.sh [output-name]
#
# Requires: mingw-w64-ucrt-x86_64-{gcc,rust,pkgconf,gtk4,libadwaita,
# gtksourceview5,poppler,librsvg,adwaita-icon-theme,hicolor-icon-theme}
#
# Layout (everything resolves relative to the DLLs — no env, no installer):
#   dist/pitex/bin/pitex.exe + *.dll
#   dist/pitex/etc/fonts/fonts.conf      minimal, points at Windows fonts
#   dist/pitex/share/icons/{Adwaita,hicolor}
#   dist/pitex/lib/{gdk-pixbuf-2.0,gtk-4.0}  dynamic modules, if present
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${MINGW_PREFIX:-/ucrt64}"

PITEX_CHANNEL="${PITEX_CHANNEL:-stable}"
if [ "$PITEX_CHANNEL" = "nightly" ]; then
    PITEX_BUNDLE="pitex-nightly"
    PITEX_NAME="Pitex Nightly"
    PITEX_BINARY="pitex-nightly.exe"
    PITEX_ICON="dev.pitex.app.nightly"
else
    PITEX_BUNDLE="pitex"
    PITEX_NAME="Pitex"
    PITEX_BINARY="pitex.exe"
    PITEX_ICON="dev.pitex.app"
fi

DIST="$HERE/dist/$PITEX_BUNDLE"
BIN="$DIST/bin"

echo "==> cargo build --release"
cargo build --release --manifest-path "$HERE/Cargo.toml" -p pitex-windows

echo "==> build editing-preview helper"
CARGO_TARGET_DIR="$HERE/target/preview-engine" cargo build --release --locked \
    --manifest-path "$HERE/../PreviewEngine/Windows/Cargo.toml"

rm -rf "$DIST"
mkdir -p "$BIN" "$DIST/etc/fonts" "$DIST/share/icons" "$DIST/lib"
cp "$HERE/target/release/pitex.exe" "$BIN/$PITEX_BINARY"
cp "$HERE/target/preview-engine/release/pitex-preview.exe" "$BIN/"
# WebView2Loader.dll — the Evergreen runtime stays external; the app loads
# the stub loader dynamically (LoadLibraryW) so its absence only degrades
# the preview, but ship it beside the exe so preview works out of the box.
# pitex-shell's build.rs also stages a copy into target/release for
# `cargo run`/`test`, but don't rely on that here: cargo gives no ordering
# guarantee between unrelated build scripts, so resolve the vendored DLL
# deterministically (newest match wins).
LOADER=""
for candidate in "$HERE"/target/release/build/webview2-com-sys-*/out/x64/WebView2Loader.dll; do
    if [ -f "$candidate" ] && { [ -z "$LOADER" ] || [ "$candidate" -nt "$LOADER" ]; }; then
        LOADER="$candidate"
    fi
done
if [ -z "$LOADER" ] && [ -f "$HERE/target/release/WebView2Loader.dll" ]; then
    LOADER="$HERE/target/release/WebView2Loader.dll"
fi
if [ -z "$LOADER" ]; then
    echo "error: WebView2Loader.dll not found under target/release/build/webview2-com-sys-*/out/x64/" >&2
    echo "       (nor pre-staged in target/release) — is markdown-preview enabled?" >&2
    exit 1
fi
cp "$LOADER" "$BIN/"

echo "==> collecting runtime DLLs"
echo "==> runtime resources"
mkdir -p "$DIST/share/$PITEX_BUNDLE"
cp -r "$HERE/../Mac/Resources/PitexAgent" "$DIST/share/$PITEX_BUNDLE/"
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

# Seed the dependency closure with executables and dynamically loaded
# modules, then include the dependencies of every newly copied DLL.
# Module dependencies are invisible when inspecting pitex.exe alone.
changed=1
while [ "$changed" = 1 ]; do
    changed=0
    while IFS= read -r -d '' runtime_file; do
        while IFS= read -r dep; do
            case "$dep" in
                "$PREFIX"/*|/ucrt64/*|/mingw64/*|/clang64/*)
                    base="$(basename "$dep")"
                    if [ ! -f "$BIN/$base" ]; then
                        cp "$dep" "$BIN/"
                        changed=1
                    fi ;;
            esac
        done < <(ldd "$runtime_file" 2>/dev/null | awk '/=> \// {print $3}')
    done < <(find "$DIST" -type f \( -name '*.exe' -o -name '*.dll' \) -print0)
done

# GdkPixbuf needs an index as well as the loader DLLs. Windows relocatable
# builds resolve relative cache paths against the application's prefix.
PIXBUF_DIR="$DIST/lib/gdk-pixbuf-2.0/2.10.0"
if [ -d "$PIXBUF_DIR/loaders" ]; then
    "$PREFIX/bin/gdk-pixbuf-query-loaders.exe" "$PIXBUF_DIR/loaders/"*.dll | \
        awk '
            /^".*\.dll"[[:space:]]*$/ {
                module = $0
                gsub(/\\/, "/", module)
                sub(/^.*\//, "", module)
                sub(/"[[:space:]]*$/, "", module)
                print "\"lib/gdk-pixbuf-2.0/2.10.0/loaders/" module "\""
                next
            }
            { print }
        ' > "$PIXBUF_DIR/loaders.cache"
fi

if [ -d "$PREFIX/share/glib-2.0/schemas" ]; then
    mkdir -p "$DIST/share/glib-2.0"
    cp -r "$PREFIX/share/glib-2.0/schemas" "$DIST/share/glib-2.0/"
fi

# Bundle notices for the renderer embedded in pitex.exe and the helper.
NOTICES="$DIST/share/doc/$PITEX_BUNDLE"
mkdir -p "$NOTICES/equation-preview/mathjax" "$NOTICES/equation-preview/mathjax-tex-font"
cp "$HERE/../LICENSE" "$NOTICES/LICENSE"
cp "$HERE/../PreviewEngine/Windows/README.md" "$NOTICES/editing-preview.md"
cp "$HERE/../PreviewEngine/Windows/THIRD-PARTY-NOTICES.txt" "$NOTICES/editing-preview-THIRD-PARTY-NOTICES.txt"
cp "$HERE/../PreviewEngine/licenses/Rust-MIT.txt" \
    "$HERE/../PreviewEngine/licenses/Rust-APACHE.txt" \
    "$HERE/../PreviewEngine/licenses/Rust-standard-library-COPYRIGHT.html" "$NOTICES/"
cp "$HERE/../Assets/equation-preview/vendor/mathjax/LICENSE" "$NOTICES/equation-preview/mathjax/"
cp "$HERE/../Assets/equation-preview/vendor/mathjax/mathjax-tex-font/OFL-1.1.txt" \
    "$NOTICES/equation-preview/mathjax-tex-font/"
cp "$HERE/../Assets/equation-preview/vendor/VERSIONS" \
    "$HERE/../Assets/equation-preview/vendor/manifest.json" "$NOTICES/equation-preview/"

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
cp "$HERE/../Linux/packaging/dev.pitex.app.png" "$DIST/share/icons/hicolor/512x512/apps/$PITEX_ICON.png"

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

cat > "$HERE/dist/channel-vars.sh" <<EOF
PITEX_BUNDLE='$PITEX_BUNDLE'
PITEX_NAME='$PITEX_NAME'
PITEX_REGKEY='$PITEX_NAME'
PITEX_BINARY='$PITEX_BINARY'
PITEX_ICON='$PITEX_ICON'
EOF

echo "==> bundle ready: $DIST"
du -sh "$DIST"
