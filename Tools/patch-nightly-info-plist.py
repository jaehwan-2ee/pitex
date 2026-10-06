#!/usr/bin/env python3
"""Patch Mac/Config/Info.plist and Base.xcconfig for the nightly build in CI.

Stable config is left untouched on disk; this script is only invoked from
release.yml on the nightly macOS path. It uses plistlib so no per-key shell
outs are needed.
"""
import plistlib
import re
import shutil
import sys
import tempfile
from pathlib import Path


def rewrite_utis(obj):
    """Rewrite app.pitex.<ext> identifiers to app.pitex.nightly.<ext>."""
    if isinstance(obj, str):
        return re.sub(
            r"^app\.pitex\.(?!desktop(?:\.|$)|nightly(?:\.|$))([A-Za-z0-9_.-]+)$",
            r"app.pitex.nightly.\1",
            obj,
        )
    if isinstance(obj, list):
        return [rewrite_utis(item) for item in obj]
    if isinstance(obj, dict):
        return {key: rewrite_utis(value) for key, value in obj.items()}
    return obj


def patch_plist(plist_path: Path, version: str) -> None:
    with open(plist_path, "rb") as f:
        plist = plistlib.load(f)

    # CFBundleVersion is a monotonic numeric UTC stamp derived from the
    # last dotted component of the nightly version.
    bundle_version = version.split(".")[-1]

    plist["CFBundleDisplayName"] = "Pitex Nightly"
    plist["CFBundleName"] = "Pitex Nightly"
    plist["CFBundleShortVersionString"] = version
    plist["CFBundleVersion"] = bundle_version
    plist["PITEXChannel"] = "nightly"

    plist = rewrite_utis(plist)

    with open(plist_path, "wb") as f:
        plistlib.dump(plist, f)


def patch_xcconfig(xcconfig_path: Path, bundle_id: str) -> None:
    """Set PRODUCT_BUNDLE_IDENTIFIER in Base.xcconfig for the nightly build."""
    text = xcconfig_path.read_text(encoding="utf-8")
    text = re.sub(
        r"^(PRODUCT_BUNDLE_IDENTIFIER\s*=\s*).*$",
        lambda m: f"{m.group(1)}{bundle_id}",
        text,
        flags=re.MULTILINE,
    )
    xcconfig_path.write_text(text, encoding="utf-8")


def _all_strings(obj):
    if isinstance(obj, str):
        yield obj
    elif isinstance(obj, list):
        for item in obj:
            yield from _all_strings(item)
    elif isinstance(obj, dict):
        for value in obj.values():
            yield from _all_strings(value)


def _self_check() -> None:
    stable_plist = Path("Mac/Config/Info.plist")
    stable_xcconfig = Path("Mac/Config/Base.xcconfig")
    if not stable_plist.exists() or not stable_xcconfig.exists():
        raise SystemExit("Run the self-check from the repository root.")

    version = "1.0.1-nightly.202610061800"
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        test_plist = tmp / "Info.plist"
        test_xcconfig = tmp / "Base.xcconfig"
        shutil.copy(stable_plist, test_plist)
        shutil.copy(stable_xcconfig, test_xcconfig)
        patch_plist(test_plist, version)
        patch_xcconfig(test_xcconfig, "app.pitex.desktop.nightly")

        with open(test_plist, "rb") as f:
            patched = plistlib.load(f)

        assert patched["CFBundleDisplayName"] == "Pitex Nightly", patched["CFBundleDisplayName"]
        assert patched["CFBundleName"] == "Pitex Nightly", patched["CFBundleName"]
        assert patched["CFBundleShortVersionString"] == version, patched["CFBundleShortVersionString"]
        assert patched["CFBundleVersion"] == "202610061800", patched["CFBundleVersion"]
        assert patched["PITEXChannel"] == "nightly", patched["PITEXChannel"]

        strings = set(_all_strings(patched))
        assert "app.pitex.nightly.tex" in strings, "app.pitex.tex should rewrite to app.pitex.nightly.tex"
        assert "app.pitex.desktop.callback" in strings, "app.pitex.desktop identifiers should remain stable"
        assert "app.pitex.tex" not in strings, "old app.pitex.tex identifier should be gone"

        xcconfig = test_xcconfig.read_text(encoding="utf-8")
        assert "PRODUCT_BUNDLE_IDENTIFIER = app.pitex.desktop.nightly" in xcconfig, xcconfig
        assert "PRODUCT_BUNDLE_IDENTIFIER = app.pitex.desktop\n" not in xcconfig, "stable bundle id should be replaced"

    print("Self-check passed.")


if __name__ == "__main__":
    if len(sys.argv) == 1:
        _self_check()
    else:
        patch_plist(Path("Mac/Config/Info.plist"), sys.argv[1])
        patch_xcconfig(Path("Mac/Config/Base.xcconfig"), "app.pitex.desktop.nightly")
