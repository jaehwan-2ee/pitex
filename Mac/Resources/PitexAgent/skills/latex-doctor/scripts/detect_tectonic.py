#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
from pathlib import Path
from typing import Any


def plugin_root() -> Path:
    return Path(__file__).resolve().parents[1]


def executable_name() -> str:
    return "tectonic.exe" if os.name == "nt" else "tectonic"


def bundled_tectonic_path(root: Path | None = None) -> Path:
    return (root or plugin_root()) / "bin" / executable_name()


def run_tool(args: list[str], *, timeout_sec: int = 5) -> tuple[int, str]:
    try:
        completed = subprocess.run(
            args,
            check=False,
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


def tectonic_version(path: Path) -> str | None:
    _code, output = run_tool([os.fspath(path), "--version"])
    return first_nonempty_line(output)


def detect_tectonic(extra_paths: list[str] | None = None) -> dict[str, Any]:
    candidates: list[tuple[str, Path]] = []
    bundled_path = bundled_tectonic_path()
    candidates.append(("bundled", bundled_path))

    search_path_parts = [*(extra_paths or [])]
    current_path = os.environ.get("PATH", "")
    if current_path:
        search_path_parts.extend(current_path.split(os.pathsep))

    path_match = shutil.which(
        executable_name(),
        path=os.pathsep.join(search_path_parts) if search_path_parts else None,
    )
    if path_match:
        candidates.append(("path", Path(path_match)))

    seen: set[Path] = set()
    checked: list[dict[str, str]] = []
    for source, candidate in candidates:
        resolved = candidate.expanduser().resolve()
        if resolved in seen:
            continue
        seen.add(resolved)
        checked.append({"source": source, "path": os.fspath(resolved)})
        if resolved.is_file():
            return {
                "status": "available",
                "reason": "Tectonic executable is available.",
                "path": os.fspath(resolved),
                "source": source,
                "version": tectonic_version(resolved),
                "checked": checked,
            }

    return {
        "status": "missing",
        "reason": "No bundled or PATH Tectonic executable was found.",
        "path": None,
        "source": None,
        "version": None,
        "checked": checked,
    }


def print_human(result: dict[str, Any]) -> None:
    print(f"Status: {result['status']}")
    print(f"Reason: {result['reason']}")
    if result.get("path"):
        print(f"Tectonic: {result['path']}")
    if result.get("version"):
        print(f"Version: {result['version']}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Detect bundled or PATH Tectonic.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable JSON.")
    args = parser.parse_args()

    result = detect_tectonic()
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print_human(result)

    return 0 if result["status"] == "available" else 3


if __name__ == "__main__":
    raise SystemExit(main())
