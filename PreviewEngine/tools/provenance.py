#!/usr/bin/env python3
"""Provenance manifest for PreviewEngine/.

  provenance.py generate --upstream <texpresso checkout at UPSTREAM_SHA>
  provenance.py verify

`generate` classifies every file under PreviewEngine/ as imported from
TeXpresso (upstream path, upstream and current SHA-256, modified flag,
license notice found in the file) or Pitex-authored, records the upstream
components that were deliberately NOT imported, and writes provenance.json.
`verify` (no network, no upstream checkout) fails when a file changed or
appeared without the manifest being regenerated, when an imported file lost
its upstream license notice, or when an excluded component (GPL dpx code,
vendored TECkit, MuPDF/SDL frontend) is present again.

This documents the technical dependency boundary; it is not legal advice.
"""
import hashlib
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)  # PreviewEngine/
MANIFEST = os.path.join(ROOT, "provenance.json")
UPSTREAM_URL = "https://github.com/let-def/texpresso"
UPSTREAM_SHA = "e8df7709077b2f86f6e16e6c86ceefb86de06f8d"

# PreviewEngine path prefix -> upstream path prefix
IMPORT_MAP = [
    ("xetex/engine/", "src/engine/engine/"),
    ("xetex/layout/", "src/engine/layout/"),
    ("xetex/main/", "src/engine/main/"),
    ("xetex/include/", "src/engine/include/"),
    ("xetex/common/include/", "src/include/"),
    ("xetex/common/", "src/common/"),
    ("licenses/TeXpresso-LICENSE.txt", "LICENSE"),
]
# Driver files derived from the TeXpresso frontend (renamed where noted).
DRIVER_MAP = {
    "driver/engine_tex.c": "src/frontend/engine_tex.c",
    "driver/engine_tex.h": "src/frontend/engine.h",
    "driver/state.c": "src/frontend/state.c",
    "driver/state.h": "src/frontend/state.h",
    "driver/fs.c": "src/frontend/fs.c",
    "driver/sprotocol.c": "src/frontend/sprotocol.c",
    "driver/sprotocol.h": "src/frontend/sprotocol.h",
    "driver/myabort.c": "src/frontend/myabort.c",
    "driver/myabort.h": "src/frontend/myabort.h",
}
# Pitex-authored files that adapt small parts of TeXpresso logic.
DERIVED_NOTES = {
    "driver/main.c": "update/close handling adapted from src/frontend/main.c "
                     "interpret_open/interpret_close (MIT); the rest is Pitex-authored",
}

EXCLUDED = [
    {"upstream": "src/engine/dpx/", "license": "GPL-2.0-or-later (xdvipdfmx)",
     "reason": "Incompatible copyleft. The engine only used it to size PDF/PNG/JPEG/BMP "
               "pictures; replaced by Pitex-authored shared/pdfread.c and shared/imginfo.c "
               "written from the format specifications. PDF output is produced by the "
               "Pitex-authored driver/xdv2pdf.c instead of xdvipdfmx."},
    {"upstream": "src/engine/engine/teckit-*", "license": "LGPL-2.1-or-later OR CPL-0.5-or-later (SIL TECkit)",
     "reason": "Not vendored. The engine dynamically links the distribution's unmodified, "
               "replaceable libTECkit shared library (Ubuntu libteckit0, Homebrew teckit)."},
    {"upstream": "src/frontend/ (renderer, incdvi, main, editor, SDL/MuPDF UI)",
     "license": "MIT source, but links MuPDF (AGPL-3.0/commercial) and SDL",
     "reason": "The MuPDF display-list renderer and SDL window are replaced by the Pitex "
               "XDV->PDF writer and the native Pitex PDF views."},
    {"upstream": "src/dvi/", "license": "MIT source, built on MuPDF fz_* APIs",
     "reason": "MuPDF-coupled XDV renderer; replaced by driver/xdv.c, driver/xdv2pdf.c, "
               "driver/fonts.c, driver/images.c, driver/pdfw.c."},
    {"upstream": "src/common/tectonic_provider.c",
     "license": "MIT",
     "reason": "External `tectonic` CLI file provider (process spawn / automatic "
               "bundle download) was removed; texlive_provider.c is the only "
               "provider. Covered by TeXpresso's root MIT license (Frédéric Bour)."},
    {"upstream": "mupdf-config.sh, Makefile, emacs/, doc/, test/", "license": "MIT",
     "reason": "Build glue and editor integrations for the upstream application."},
]

FORBIDDEN_PATTERNS = [
    (re.compile(r"^xetex/.*dpx-"), "GPL dpx source"),
    (re.compile(r"(^|/)teckit-[^/]*\.(c|cpp|h)$"), "vendored TECkit source"),
    (re.compile(r"(^|/)(incdvi|renderer)\.[ch]$"), "MuPDF renderer source"),
    (re.compile(r"mupdf", re.I), "MuPDF reference in file name"),
]


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def detect_license(rel, path):
    # Prose docs (PROVENANCE.md, THIRD-PARTY.md, PROTOCOL.md, README) discuss
    # licenses by name; do not let a mention become the file's license.
    # Their license_in_file is the repository root license, like other
    # Pitex-authored files.
    if rel in PROSE_DOCS or rel.endswith(".md"):
        return "n/a (documentation)"
    try:
        with open(path, "rb") as f:
            head = f.read(4096).decode("utf-8", "replace")
    except OSError:
        return "unknown"
    if "GNU General Public License" in head and "Lesser" not in head:
        return "GPL"
    if "Tectonic Project" in head and "MIT" in head:
        return "MIT (Tectonic Project)"
    if "Permission is hereby granted" in head and "SIL" in head:
        return "MIT/X11 (SIL International)"
    if "MIT License" in head or "Permission is hereby granted" in head:
        return "MIT"
    if "public domain" in head.lower():
        return "public domain"
    if "Pitex-authored" in head:
        return "PolyForm-Shield-1.0.0 (Pitex)"
    return "none-in-file"


# Prose documents that name licenses without being license texts.
PROSE_DOCS = {"PROVENANCE.md", "PROTOCOL.md", "README.md", "licenses/THIRD-PARTY.md"}


def tracked_files():
    out = []
    for base, dirs, files in os.walk(ROOT):
        dirs[:] = [d for d in dirs if d not in (".git", "__pycache__")]
        for name in files:
            rel = os.path.relpath(os.path.join(base, name), ROOT).replace(os.sep, "/")
            if rel == "provenance.json":
                continue
            out.append(rel)
    return sorted(out)


def upstream_path(rel):
    if rel in DRIVER_MAP:
        return DRIVER_MAP[rel]
    for local, up in IMPORT_MAP:
        if rel == local or (local.endswith("/") and rel.startswith(local)):
            return up + rel[len(local):] if local.endswith("/") else up
    return None


def generate(upstream):
    head = subprocess.run(["git", "-C", upstream, "rev-parse", "HEAD"], capture_output=True,
                          text=True, check=True).stdout.strip()
    if head != UPSTREAM_SHA:
        sys.exit(f"upstream checkout is at {head}, expected {UPSTREAM_SHA}")
    entries = []
    for rel in tracked_files():
        path = os.path.join(ROOT, rel)
        up = upstream_path(rel)
        entry = {"path": rel, "sha256": sha256(path), "license_in_file": detect_license(rel, path)}
        if up:
            up_file = os.path.join(upstream, up)
            if not os.path.isfile(up_file):
                sys.exit(f"{rel}: upstream file {up} missing")
            entry.update({"origin": "texpresso", "upstream_path": up,
                          "upstream_sha256": sha256(up_file),
                          "upstream_license": detect_license(up, up_file)})
            entry["modified"] = entry["sha256"] != entry["upstream_sha256"]
        else:
            entry["origin"] = "pitex"
            if rel in DERIVED_NOTES:
                entry["adapts"] = DERIVED_NOTES[rel]
        entries.append(entry)
    manifest = {
        "upstream": {"url": UPSTREAM_URL, "commit": UPSTREAM_SHA,
                     "root_license": "MIT (Frédéric Bour); per-file notices retained"},
        "excluded_upstream_components": EXCLUDED,
        "runtime_dependencies": RUNTIME_DEPS,
        "files": entries,
    }
    with open(MANIFEST, "w") as f:
        json.dump(manifest, f, indent=1, sort_keys=True, ensure_ascii=False)
        f.write("\n")
    print(f"wrote {os.path.relpath(MANIFEST)} ({len(entries)} files)")


# `license` values are valid SPDX license expressions (LicenseRef where no
# SPDX id exists); `license_notes` carry human-readable scope and the chosen
# term for multi-licensed libraries. `source_availability` pins the exact
# upstream source for LGPL/MPL components — concrete archive or named
# distro source package, per LGPL-2.1 §6c / MPL-2.0 §3.2. This table is
# technical evidence for the shipped binaries, not a legal opinion.
RUNTIME_DEPS = [
    {"library": "libTECkit", "license": "LGPL-2.1-or-later OR LicenseRef-TECkit-CPL-0.5",
     "license_notes": "SIL dual choice: LGPL-2.1+ or CPL-0.5+; LGPL-2.1 selected",
     "linkage": "shared, unmodified, replaceable",
     "source_availability": "https://github.com/silnrsi/teckit/releases/download/v2.5.13/teckit-2.5.13.tar.xz (sha256 3f55cd3670f1ff1a439d5a40071870b9e4ca2be8877a0eb80e24783cb532b380; configure --with-system-zlib); Debian/Ubuntu: apt source libteckit0",
     "used_by": "pitex-preview-xetex"},
    {"library": "HarfBuzz", "license": "MIT",
     "license_notes": "upstream 'Old MIT' text; SPDX MIT",
     "linkage": "shared",
     "source_availability": "https://github.com/harfbuzz/harfbuzz/releases/download/14.5.0/harfbuzz-14.5.0.tar.xz (sha256 b7132e148358a45185c9feafd049dbaf243649d3c44414b3534d9c95d18592b9); Debian/Ubuntu: apt source libharfbuzz0b",
     "used_by": "pitex-preview-xetex"},
    {"library": "graphite2", "license": "LGPL-2.1-or-later OR MPL-2.0 OR GPL-2.0-or-later",
     "license_notes": "shipped under LGPL-2.1-or-later; dynamically linked",
     "linkage": "shared",
     "source_availability": "Debian/Ubuntu: apt source libgraphite2-3; macOS bundle: formula + pinned source URL/SHA256 in BUNDLED-VERSIONS.txt",
     "used_by": "pitex-preview-xetex"},
    {"library": "FreeType", "license": "FTL OR GPL-2.0-or-later",
     "license_notes": "FTL selected; copyright © The FreeType Project (www.freetype.org). All rights reserved.",
     "linkage": "shared",
     "source_availability": "Debian/Ubuntu: apt source libfreetype6; macOS bundle: formula + pinned source URL/SHA256 in BUNDLED-VERSIONS.txt",
     "used_by": "pitex-preview-xetex"},
    {"library": "ICU", "license": "Unicode-3.0",
     "license_notes": "Unicode License v3; Debian records also mark some ICU data files Expat",
     "linkage": "shared", "used_by": "pitex-preview-xetex"},
    {"library": "fontconfig", "license": "HPND-sell-variant",
     "license_notes": "MIT-style license including permission to sell",
     "linkage": "shared (Linux)", "used_by": "pitex-preview-xetex"},
    {"library": "libpng", "license": "Libpng",
     "license_notes": "PNG Reference Library License v2",
     "linkage": "shared", "used_by": "pitex-preview-xetex"},
    {"library": "zlib", "license": "Zlib", "linkage": "shared", "used_by": "both helpers"},
    {"library": "TeX distribution (TeX Live / MacTeX)",
     "license_notes": "aggregate data read at run time; per-file licenses (LPPL, GPL variants, OFL, ...) ship inside the distribution itself — not a linked component, no single expression applies",
     "linkage": "not linked: files read at run time via kpsewhich/ls-R, format generated locally",
     "used_by": "both helpers"},
]


def verify():
    with open(MANIFEST) as f:
        manifest = json.load(f)
    problems = []
    listed = {e["path"]: e for e in manifest["files"]}
    current = tracked_files()
    for rel in current:
        for pattern, what in FORBIDDEN_PATTERNS:
            if pattern.search(rel):
                problems.append(f"{rel}: forbidden component ({what})")
        if rel not in listed:
            problems.append(f"{rel}: not in provenance.json (regenerate)")
            continue
        e = listed[rel]
        path = os.path.join(ROOT, rel)
        if sha256(path) != e["sha256"]:
            problems.append(f"{rel}: content differs from provenance.json (regenerate)")
        lic = detect_license(rel, path)
        if lic == "GPL":
            problems.append(f"{rel}: GPL notice found")
        if e["origin"] == "texpresso" and e.get("upstream_license") not in (None, "none-in-file") \
                and lic != e["upstream_license"]:
            problems.append(f"{rel}: upstream notice '{e['upstream_license']}' not retained (found '{lic}')")
    for rel in listed:
        if rel not in current:
            problems.append(f"{rel}: listed but missing")
    if problems:
        print("\n".join(problems))
        sys.exit(1)
    imported = sum(1 for e in manifest["files"] if e["origin"] == "texpresso")
    modified = sum(1 for e in manifest["files"] if e.get("modified"))
    print(f"provenance OK: {len(current)} files, {imported} from TeXpresso {manifest['upstream']['commit'][:12]} "
          f"({modified} modified), {len(current) - imported} Pitex-authored")


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "verify":
        verify()
    elif len(sys.argv) == 4 and sys.argv[1] == "generate" and sys.argv[2] == "--upstream":
        generate(sys.argv[3])
    else:
        sys.exit(__doc__)
