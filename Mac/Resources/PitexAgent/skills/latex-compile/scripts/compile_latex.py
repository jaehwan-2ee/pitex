#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any

from detect_tectonic import detect_tectonic
from detect_texlive import detect_texlive

ROOT_DIRECTIVE_RE = re.compile(r"^%\s*!TEX\s+root\s*=\s*(?P<root>.+?)\s*$")
PROGRAM_DIRECTIVE_RE = re.compile(
    r"^%\s*!TEX\s+program\s*=\s*(?P<program>.+?)\s*$",
    re.IGNORECASE,
)
INPUT_RE = re.compile(r"\\(?:input|include)\{(?P<path>[^}]+)\}")
PACKAGE_RE = re.compile(r"\\usepackage(?:\[[^\]]*\])?\{(?P<packages>[^}]+)\}")
COMPLEX_TECTONIC_PACKAGES = {
    "asymptote",
    "biblatex",
    "glossaries",
    "imakeidx",
    "luacode",
    "makeidx",
    "minted",
    "pythontex",
    "shellesc",
}
COMPLEX_TECTONIC_COMMANDS: tuple[tuple[re.Pattern[str], str], ...] = (
    (re.compile(r"\\(?:addbibresource|bibliography)\b"), "bibliography tooling"),
    (
        re.compile(r"\\(?:makeglossaries|makeindex|printglossary|printindex)\b"),
        "index or glossary tooling",
    ),
    (
        re.compile(r"\\(?:inputminted|tikzexternalize|write18)\b"),
        "shell escape or externalization",
    ),
)
MAX_PROJECT_FILES = 50


def read_tex(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def resolve_tex_root(tex_file: Path) -> Path:
    text = read_tex(tex_file)
    for line in text.splitlines()[:20]:
        match = ROOT_DIRECTIVE_RE.match(line)
        if match:
            root = match.group("root").strip()
            candidate = (tex_file.parent / root).resolve()
            if candidate.is_file():
                return candidate
    if "\\documentclass" in text:
        return tex_file.resolve()

    for candidate in sorted(tex_file.parent.glob("*.tex")):
        if candidate == tex_file:
            continue
        try:
            candidate_text = read_tex(candidate)
        except OSError:
            continue
        if "\\documentclass" in candidate_text:
            return candidate.resolve()
    return tex_file.resolve()


def referenced_tex_file(current_file: Path, raw_path: str) -> Path | None:
    raw_path = raw_path.strip()
    if not raw_path or raw_path.startswith("|"):
        return None

    candidate = (current_file.parent / raw_path).expanduser()
    if candidate.suffix == "":
        candidate = candidate.with_suffix(".tex")
    candidate = candidate.resolve()
    return candidate if candidate.is_file() else None


def project_tex_files(root_file: Path) -> list[Path]:
    pending = [root_file.resolve()]
    seen: set[Path] = set()
    ordered: list[Path] = []

    while pending and len(ordered) < MAX_PROJECT_FILES:
        current = pending.pop()
        if current in seen:
            continue
        seen.add(current)
        ordered.append(current)

        try:
            text = read_tex(current)
        except OSError:
            continue
        for match in INPUT_RE.finditer(text):
            included = referenced_tex_file(current, match.group("path"))
            if included is not None and included not in seen:
                pending.append(included)

    return ordered


def package_names(text: str) -> set[str]:
    names: set[str] = set()
    for match in PACKAGE_RE.finditer(text):
        for name in match.group("packages").split(","):
            stripped = name.strip()
            if stripped:
                names.add(stripped)
    return names


def tectonic_suitability(root_file: Path) -> dict[str, Any]:
    reasons: list[str] = []
    scanned_files = project_tex_files(root_file)

    for path in scanned_files:
        try:
            text = read_tex(path)
        except OSError as error:
            reasons.append(f"{path}: could not read file ({error})")
            continue

        for line in text.splitlines()[:20]:
            match = PROGRAM_DIRECTIVE_RE.match(line)
            if match:
                program = match.group("program").strip()
                if program and program.lower() != "tectonic":
                    reasons.append(f"{path}: TeX program directive asks for {program}")

        complex_packages = sorted(package_names(text) & COMPLEX_TECTONIC_PACKAGES)
        if complex_packages:
            reasons.append(f"{path}: package likely needs TeX Live ({', '.join(complex_packages)})")

        for pattern, reason in COMPLEX_TECTONIC_COMMANDS:
            if pattern.search(text):
                reasons.append(f"{path}: {reason}")

    return {
        "suitable": len(reasons) == 0,
        "reasons": reasons,
        "scannedFiles": [os.fspath(path) for path in scanned_files],
    }


def build_texlive_command(
    *,
    detection: dict[str, Any],
    engine: str,
    output_directory: Path | None,
    root_file: Path,
) -> list[str]:
    commands = detection["commands"]
    latexmk = commands["latexmk"]["path"]
    engine_path = commands[engine]["path"]
    if latexmk:
        engine_flag = {
            "pdflatex": "-pdf",
            "xelatex": "-xelatex",
            "lualatex": "-lualatex",
        }[engine]
        command = [
            latexmk,
            # Do not load latexmkrc files from the project, home directory, or
            # system install. Artifact previews are opened for files in an
            # arbitrary workspace; latexmk rc files are Perl and can execute
            # commands while the preview is being generated. latexmk checks
            # for -norc before it evaluates any rc file, so keep this on every
            # latexmk invocation rather than trying to filter particular
            # filenames in the project tree.
            "-norc",
            engine_flag,
            "-interaction=nonstopmode",
            "-halt-on-error",
            "-synctex=1",
        ]
        if output_directory is not None:
            command.append(f"-outdir={output_directory}")
        command.append(os.fspath(root_file))
        return command

    if engine_path is None:
        raise RuntimeError(f"Neither latexmk nor {engine} is available.")

    command = [
        engine_path,
        "-interaction=nonstopmode",
        "-halt-on-error",
        "-synctex=1",
    ]
    if output_directory is not None:
        command.append(f"-output-directory={output_directory}")
    command.append(os.fspath(root_file))
    return command


def build_tectonic_command(
    *,
    output_directory: Path | None,
    root_file: Path,
    tectonic_path: str,
) -> list[str]:
    return [
        tectonic_path,
        "-X",
        "compile",
        "--outdir",
        os.fspath(output_directory or root_file.parent),
        "--outfmt",
        "pdf",
        "--print",
        "--untrusted",
        root_file.name,
    ]


def expected_pdf_path(root_file: Path, output_directory: Path | None) -> Path:
    directory = output_directory if output_directory is not None else root_file.parent
    return directory / f"{root_file.stem}.pdf"


def run_compile_attempt(
    *,
    command: list[str],
    compiler: str,
    cwd: Path,
    env: dict[str, str],
    pdf_path: Path,
) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        check=False,
        cwd=os.fspath(cwd),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return {
        "compiler": compiler,
        "command": command,
        "exitCode": completed.returncode,
        "pdfPath": os.fspath(pdf_path),
        "pdfExists": pdf_path.is_file(),
        "log": completed.stdout[-8000:],
    }


def skipped_attempt(*, compiler: str, reason: str) -> dict[str, Any]:
    return {
        "compiler": compiler,
        "skipped": True,
        "reason": reason,
    }


def attempt_succeeded(attempt: dict[str, Any]) -> bool:
    return attempt.get("exitCode") == 0 and attempt.get("pdfExists") is True


def texlive_is_usable(detection: dict[str, Any] | None) -> bool:
    return detection is not None and detection["status"] == "existing-usable"


def add_texlive_attempt(
    *,
    attempts: list[dict[str, Any]],
    detection: dict[str, Any],
    engine: str,
    output_directory: Path | None,
    pdf_path: Path,
    root_file: Path,
) -> dict[str, Any] | None:
    if detection["status"] == "missing":
        attempts.append(
            skipped_attempt(
                compiler="texlive",
                reason="No TeX Live or MacTeX installation detected. Run latex-doctor first.",
            )
        )
        return None

    try:
        command = build_texlive_command(
            detection=detection,
            engine=engine,
            output_directory=output_directory,
            root_file=root_file,
        )
    except RuntimeError as error:
        attempts.append(skipped_attempt(compiler="texlive", reason=str(error)))
        return None

    attempt = run_compile_attempt(
        command=command,
        compiler="texlive",
        cwd=root_file.parent,
        env={**os.environ, "PATH": detection["searchPath"]},
        pdf_path=pdf_path,
    )
    attempts.append(attempt)
    return attempt if attempt_succeeded(attempt) else None


def add_tectonic_attempt(
    *,
    attempts: list[dict[str, Any]],
    detection: dict[str, Any],
    output_directory: Path | None,
    pdf_path: Path,
    root_file: Path,
    suitability: dict[str, Any],
    force: bool,
) -> dict[str, Any] | None:
    tectonic_path = detection["path"]
    if tectonic_path is None:
        attempts.append(
            skipped_attempt(
                compiler="tectonic",
                reason="No bundled or PATH Tectonic executable was found.",
            )
        )
        return None

    if not force and not suitability["suitable"]:
        attempts.append(
            skipped_attempt(
                compiler="tectonic",
                reason="Project likely needs TeX Live: " + "; ".join(suitability["reasons"]),
            )
        )
        return None

    attempt = run_compile_attempt(
        command=build_tectonic_command(
            output_directory=output_directory,
            root_file=root_file,
            tectonic_path=tectonic_path,
        ),
        compiler="tectonic",
        cwd=root_file.parent,
        env={**os.environ, "TECTONIC_UNTRUSTED_MODE": "1"},
        pdf_path=pdf_path,
    )
    attempts.append(attempt)
    return attempt if attempt_succeeded(attempt) else None


def print_human(result: dict[str, Any]) -> None:
    print(f"Root: {result['rootFile']}")
    compiler = result["compiler"] or "not compiled"
    print(f"Compiler: {compiler}")
    if result.get("command"):
        print(f"Command: {' '.join(result['command'])}")
    print(f"Exit code: {result['exitCode']}")
    print(f"PDF: {result['pdfPath'] if result['pdfExists'] else 'not created'}")

    print("\nAttempts:")
    for attempt in result["attempts"]:
        if attempt.get("skipped"):
            print(f"- {attempt['compiler']}: skipped ({attempt['reason']})")
        else:
            outcome = "passed" if attempt_succeeded(attempt) else "failed"
            print(f"- {attempt['compiler']}: {outcome} (exit {attempt['exitCode']})")

    if result["exitCode"] != 0 and result.get("log"):
        print(result["log"])


def main() -> int:
    parser = argparse.ArgumentParser(description="Compile a TeX file with LaTeX tooling.")
    parser.add_argument("tex_file", type=Path)
    parser.add_argument(
        "--compiler",
        choices=["auto", "tectonic", "texlive"],
        default="auto",
        help="Choose auto to prefer TeX Live when installed, otherwise use Tectonic for simple builds with TeX Live fallback.",
    )
    parser.add_argument(
        "--engine",
        choices=["pdflatex", "xelatex", "lualatex"],
        default="pdflatex",
        help="TeX Live engine to use when compiling through latexmk or a direct engine.",
    )
    parser.add_argument("--output-directory", type=Path)
    parser.add_argument("--json", action="store_true", help="Print machine-readable JSON.")
    args = parser.parse_args()

    tex_file = args.tex_file.expanduser().resolve()
    if not tex_file.is_file():
        raise SystemExit(f"TeX file not found: {tex_file}")

    root_file = resolve_tex_root(tex_file)
    output_directory = (
        args.output_directory.expanduser().resolve() if args.output_directory else None
    )
    if output_directory is not None:
        output_directory.mkdir(parents=True, exist_ok=True)
    pdf_path = expected_pdf_path(root_file, output_directory)

    attempts: list[dict[str, Any]] = []
    selected_attempt: dict[str, Any] | None = None
    suitability = tectonic_suitability(root_file)
    tectonic_detection = detect_tectonic()
    texlive_detection = detect_texlive() if args.compiler in {"auto", "texlive"} else None
    prefer_texlive = args.compiler == "texlive" or (
        args.compiler == "auto" and texlive_is_usable(texlive_detection)
    )

    if prefer_texlive and texlive_detection is not None:
        selected_attempt = add_texlive_attempt(
            attempts=attempts,
            detection=texlive_detection,
            engine=args.engine,
            output_directory=output_directory,
            pdf_path=pdf_path,
            root_file=root_file,
        )

    if selected_attempt is None and args.compiler in {"auto", "tectonic"}:
        if args.compiler == "auto" and prefer_texlive:
            attempts.append(
                skipped_attempt(
                    compiler="tectonic",
                    reason="TeX Live is available, so auto mode prefers TeX Live.",
                )
            )
        else:
            selected_attempt = add_tectonic_attempt(
                attempts=attempts,
                detection=tectonic_detection,
                output_directory=output_directory,
                pdf_path=pdf_path,
                root_file=root_file,
                suitability=suitability,
                force=args.compiler == "tectonic",
            )

    if (
        selected_attempt is None
        and not prefer_texlive
        and args.compiler in {"auto", "texlive"}
        and texlive_detection is not None
    ):
        selected_attempt = add_texlive_attempt(
            attempts=attempts,
            detection=texlive_detection,
            engine=args.engine,
            output_directory=output_directory,
            pdf_path=pdf_path,
            root_file=root_file,
        )

    last_attempt = attempts[-1] if attempts else {}
    result = {
        "rootFile": os.fspath(root_file),
        "compiler": selected_attempt["compiler"] if selected_attempt else None,
        "command": selected_attempt.get("command") if selected_attempt else None,
        "exitCode": selected_attempt["exitCode"] if selected_attempt else 1,
        "pdfPath": os.fspath(pdf_path),
        "pdfExists": pdf_path.is_file(),
        "log": selected_attempt.get("log", "") if selected_attempt else last_attempt.get("log", ""),
        "attempts": attempts,
        "tectonic": tectonic_detection,
        "tectonicSuitability": suitability,
        "texliveDetection": texlive_detection,
    }

    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print_human(result)

    return 0 if selected_attempt is not None else 1


if __name__ == "__main__":
    raise SystemExit(main())
