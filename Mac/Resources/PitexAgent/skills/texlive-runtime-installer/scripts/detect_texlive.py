#!/usr/bin/env python3
from __future__ import annotations

import argparse
import glob
import json
import os
import shutil
import subprocess
from pathlib import Path
from typing import Any

REQUIRED_TOOLS = ("latexmk", "pdflatex", "kpsewhich")
RECOMMENDED_TOOLS = ("xelatex", "lualatex", "biber")
ALL_TOOLS = (*REQUIRED_TOOLS, *RECOMMENDED_TOOLS)
MANAGED_ROOT = Path.home() / ".cache" / "pitex" / "texlive"


def discover_known_bin_dirs() -> list[str]:
    candidates: list[str] = []
    candidates.append("/Library/TeX/texbin")
    candidates.extend(sorted(glob.glob("/usr/local/texlive/*/bin/*"), reverse=True))
    candidates.extend(sorted(glob.glob(os.fspath(MANAGED_ROOT / "*" / "bin" / "*")), reverse=True))

    seen: set[str] = set()
    existing: list[str] = []
    for candidate in candidates:
        normalized = os.path.normpath(os.path.expanduser(candidate))
        if normalized in seen or not os.path.isdir(normalized):
            continue
        seen.add(normalized)
        existing.append(normalized)
    return existing


def path_with_known_tex_dirs(extra_paths: list[str] | None = None) -> str:
    paths = [*(extra_paths or []), *discover_known_bin_dirs()]
    current_path = os.environ.get("PATH", "")
    if current_path:
        paths.extend(current_path.split(os.pathsep))

    seen: set[str] = set()
    deduped: list[str] = []
    for path in paths:
        normalized = os.path.normpath(os.path.expanduser(path))
        if not normalized or normalized in seen:
            continue
        seen.add(normalized)
        deduped.append(normalized)
    return os.pathsep.join(deduped)


def find_tool(tool: str, *, search_path: str) -> str | None:
    found = shutil.which(tool, path=search_path)
    # Keep the symlink path so TeX engines see the intended argv[0] format
    # name, e.g. pdflatex instead of the underlying pdftex binary.
    return os.path.abspath(found) if found else None


def run_tool(args: list[str], *, search_path: str, timeout_sec: int = 10) -> tuple[int, str]:
    env = {**os.environ, "PATH": search_path}
    try:
        completed = subprocess.run(
            args,
            check=False,
            env=env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=timeout_sec,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return 127, str(error)
    return completed.returncode, completed.stdout.strip()


def first_nonempty_line(text: str) -> str | None:
    for line in text.splitlines():
        stripped = line.strip()
        if stripped:
            return stripped
    return None


def tool_version(tool: str, path: str, *, search_path: str) -> str | None:
    if tool == "biber":
        # Universal macOS biber binaries can perform slow first-run extraction.
        # Path presence is enough for readiness detection; bibliography builds
        # will surface execution failures with the real project log.
        return "present"
    version_args = [path, "-norc", "-v"] if tool == "latexmk" else [path, "--version"]
    _code, output = run_tool(version_args, search_path=search_path, timeout_sec=5)
    return first_nonempty_line(output)


def detect_texlive(extra_paths: list[str] | None = None) -> dict[str, Any]:
    search_path = path_with_known_tex_dirs(extra_paths)
    commands: dict[str, dict[str, str | None]] = {}
    for tool in ALL_TOOLS:
        path = find_tool(tool, search_path=search_path)
        commands[tool] = {
            "path": path,
            "version": tool_version(tool, path, search_path=search_path) if path else None,
        }

    kpsewhich = commands["kpsewhich"]["path"]
    texmfroot: str | None = None
    if kpsewhich:
        code, output = run_tool(
            [kpsewhich, "-var-value=TEXMFROOT"],
            search_path=search_path,
        )
        if code == 0 and output:
            texmfroot = output.splitlines()[-1].strip() or None

    missing_required = [tool for tool in REQUIRED_TOOLS if commands[tool]["path"] is None]
    missing_recommended = [tool for tool in RECOMMENDED_TOOLS if commands[tool]["path"] is None]
    detected_paths = [
        command["path"] for command in commands.values() if command["path"] is not None
    ]
    active_bin_dir = None
    for preferred_tool in ("pdflatex", "kpsewhich", "xelatex", "lualatex", "biber", "latexmk"):
        preferred_path = commands[preferred_tool]["path"]
        if preferred_path is not None:
            active_bin_dir = os.path.dirname(preferred_path)
            break

    if not detected_paths and texmfroot is None:
        status = "missing"
        reason = "No TeX Live or MacTeX tools were found on PATH or known TeX locations."
    elif missing_required:
        status = "existing-partial"
        reason = "A TeX installation was detected, but required tools are missing."
    else:
        status = "existing-usable"
        reason = "Required TeX tools are available."

    return {
        "status": status,
        "reason": reason,
        "activeBinDir": active_bin_dir,
        "texmfroot": texmfroot,
        "commands": commands,
        "missingRequired": missing_required,
        "missingRecommended": missing_recommended,
        "knownTexBinDirs": discover_known_bin_dirs(),
        "searchPath": search_path,
    }


def print_human(result: dict[str, Any]) -> None:
    print(f"Status: {result['status']}")
    print(f"Reason: {result['reason']}")
    if result.get("activeBinDir"):
        print(f"TeX bin: {result['activeBinDir']}")
    if result.get("texmfroot"):
        print(f"TEXMFROOT: {result['texmfroot']}")

    print("\nTools:")
    for tool, metadata in result["commands"].items():
        path = metadata.get("path") or "missing"
        print(f"- {tool}: {path}")
        if metadata.get("version"):
            print(f"  {metadata['version']}")

    if result["missingRequired"]:
        print("\nMissing required tools: " + ", ".join(result["missingRequired"]))
    if result["missingRecommended"]:
        print("Missing recommended tools: " + ", ".join(result["missingRecommended"]))


def main() -> int:
    parser = argparse.ArgumentParser(description="Detect TeX Live or MacTeX tooling.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable JSON.")
    args = parser.parse_args()

    result = detect_texlive()
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print_human(result)

    if result["status"] == "existing-usable":
        return 0
    if result["status"] == "existing-partial":
        return 2
    return 3


if __name__ == "__main__":
    raise SystemExit(main())
