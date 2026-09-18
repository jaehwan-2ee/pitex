#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Any

from detect_tectonic import detect_tectonic
from detect_texlive import detect_texlive

SMOKE_TEX = r"""\documentclass{article}
\usepackage{amsmath}
\begin{document}
Pitex LaTeX smoke test. \(E = mc^2\).
\end{document}
"""


def run_texlive_smoke_test(detection: dict[str, Any]) -> dict[str, Any]:
    commands = detection["commands"]
    latexmk = commands["latexmk"]["path"]
    pdflatex = commands["pdflatex"]["path"]
    search_path = detection["searchPath"]
    if latexmk is None and pdflatex is None:
        return {
            "attempted": False,
            "passed": False,
            "reason": "Neither latexmk nor pdflatex is available.",
        }

    with tempfile.TemporaryDirectory(prefix="pitex-latex-doctor-") as temp_dir:
        temp_path = Path(temp_dir)
        tex_path = temp_path / "smoke.tex"
        tex_path.write_text(SMOKE_TEX, encoding="utf-8")
        if latexmk:
            command = [
                latexmk,
                "-norc",
                "-pdf",
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-synctex=1",
                os.fspath(tex_path),
            ]
        else:
            command = [
                pdflatex,
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-synctex=1",
                os.fspath(tex_path),
            ]

        completed = subprocess.run(
            command,
            check=False,
            cwd=temp_dir,
            env={**os.environ, "PATH": search_path},
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        pdf_path = temp_path / "smoke.pdf"
        return {
            "attempted": True,
            "passed": completed.returncode == 0 and pdf_path.is_file(),
            "command": command,
            "exitCode": completed.returncode,
            "log": completed.stdout[-4000:],
        }


def run_tectonic_smoke_test(detection: dict[str, Any]) -> dict[str, Any]:
    tectonic_path = detection["path"]
    if tectonic_path is None:
        return {
            "attempted": False,
            "passed": False,
            "reason": "No bundled or PATH Tectonic executable is available.",
        }

    with tempfile.TemporaryDirectory(prefix="pitex-tectonic-doctor-") as temp_dir:
        temp_path = Path(temp_dir)
        tex_path = temp_path / "smoke.tex"
        tex_path.write_text(SMOKE_TEX, encoding="utf-8")
        command = [
            tectonic_path,
            "-X",
            "compile",
            "--outdir",
            temp_dir,
            "--outfmt",
            "pdf",
            "--print",
            "--untrusted",
            "smoke.tex",
        ]
        completed = subprocess.run(
            command,
            check=False,
            cwd=temp_dir,
            env={**os.environ, "TECTONIC_UNTRUSTED_MODE": "1"},
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        pdf_path = temp_path / "smoke.pdf"
        return {
            "attempted": True,
            "passed": completed.returncode == 0 and pdf_path.is_file(),
            "command": command,
            "exitCode": completed.returncode,
            "log": completed.stdout[-4000:],
        }


def print_texlive_summary(detection: dict[str, Any], smoke: dict[str, Any]) -> None:
    print(f"TeX Live status: {detection['status']}")
    print(f"TeX Live reason: {detection['reason']}")
    if detection.get("activeBinDir"):
        print(f"TeX bin: {detection['activeBinDir']}")
    if detection.get("texmfroot"):
        print(f"TEXMFROOT: {detection['texmfroot']}")

    missing_required = detection["missingRequired"]
    missing_recommended = detection["missingRecommended"]
    if missing_required:
        print("Missing required TeX Live tools: " + ", ".join(missing_required))
    if missing_recommended:
        print("Missing recommended TeX Live tools: " + ", ".join(missing_recommended))

    if smoke["attempted"]:
        print(f"TeX Live smoke test: {'passed' if smoke['passed'] else 'failed'}")
        if not smoke["passed"]:
            print(smoke.get("log", ""))
    else:
        print(f"TeX Live smoke test: skipped ({smoke['reason']})")


def print_tectonic_summary(detection: dict[str, Any], smoke: dict[str, Any]) -> None:
    print(f"Tectonic status: {detection['status']}")
    print(f"Tectonic reason: {detection['reason']}")
    if detection.get("path"):
        print(f"Tectonic: {detection['path']}")
    if detection.get("version"):
        print(f"Tectonic version: {detection['version']}")

    if smoke["attempted"]:
        print(f"Tectonic smoke test: {'passed' if smoke['passed'] else 'failed'}")
        if not smoke["passed"]:
            print(smoke.get("log", ""))
    else:
        print(f"Tectonic smoke test: skipped ({smoke['reason']})")


def print_human(result: dict[str, Any]) -> None:
    print(f"Status: {'ready' if result['ready'] else 'not ready'}")
    print()
    print_tectonic_summary(result["tectonic"], result["tectonicSmokeTest"])
    print()
    print_texlive_summary(result["detection"], result["smokeTest"])


def main() -> int:
    parser = argparse.ArgumentParser(description="Check LaTeX readiness for Pitex.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable JSON.")
    args = parser.parse_args()

    tectonic = detect_tectonic()
    tectonic_smoke = run_tectonic_smoke_test(tectonic)
    texlive_detection = detect_texlive()
    texlive_smoke = run_texlive_smoke_test(texlive_detection)
    ready = tectonic_smoke["passed"] or (
        texlive_detection["status"] == "existing-usable" and texlive_smoke["passed"]
    )
    result = {
        "ready": ready,
        "tectonic": tectonic,
        "tectonicSmokeTest": tectonic_smoke,
        "detection": texlive_detection,
        "smokeTest": texlive_smoke,
    }

    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print_human(result)

    return 0 if ready else 1


if __name__ == "__main__":
    raise SystemExit(main())
