#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from urllib.request import Request, urlopen

from detect_texlive import detect_texlive

DEFAULT_REPOSITORY = "https://mirror.ctan.org/systems/texlive/tlnet"
INSTALLER_URL = "https://mirror.ctan.org/systems/texlive/tlnet/install-tl-unx.tar.gz"
MANAGED_ROOT = Path.home() / ".cache" / "pitex" / "texlive"
DEFAULT_TARGET_ROOT = MANAGED_ROOT / "full"


def log(message: str) -> None:
    print(f"[pitex-texlive] {message}", flush=True)


def ensure_within_directory(directory: Path, target: Path) -> None:
    resolved_directory = directory.resolve()
    resolved_target = target.resolve()
    prefix = os.fspath(resolved_directory) + os.sep
    if resolved_target == resolved_directory:
        return
    if not os.fspath(resolved_target).startswith(prefix):
        raise RuntimeError(f"Archive entry would extract outside {directory}: {target}")


def extract_archive(archive_path: Path, extract_dir: Path) -> Path:
    with tarfile.open(archive_path, "r:gz") as archive:
        members = archive.getmembers()
        for member in members:
            ensure_within_directory(extract_dir, extract_dir / member.name)
        archive.extractall(extract_dir)

    installer_dirs = sorted(extract_dir.glob("install-tl-*"))
    if not installer_dirs:
        raise RuntimeError("TeX Live installer archive did not contain install-tl-*.")
    return installer_dirs[0]


def download_installer(destination: Path) -> None:
    request = Request(INSTALLER_URL, headers={"User-Agent": "pitex-texlive-runtime"})
    with urlopen(request) as response, destination.open("wb") as output:
        shutil.copyfileobj(response, output)


def write_profile(profile_path: Path, target_root: Path) -> None:
    profile = f"""selected_scheme scheme-full
TEXDIR {target_root}
TEXMFLOCAL {target_root / "texmf-local"}
TEXMFSYSVAR {target_root / "texmf-var"}
TEXMFSYSCONFIG {target_root / "texmf-config"}
TEXMFCONFIG {MANAGED_ROOT / "texmf-config"}
TEXMFVAR {MANAGED_ROOT / "texmf-var"}
TEXMFHOME ~/texmf
option_doc 1
option_src 1
option_autobackup 0
option_backupdir tlpkg/backups
option_desktop_integration 0
option_file_assocs 0
option_path 0
option_post_code 1
"""
    profile_path.write_text(profile, encoding="utf-8")


def run_install(*, repository: str, target_root: Path) -> None:
    if sys.platform.startswith("win"):
        raise RuntimeError(
            "Managed TeX Live install is currently supported on macOS and Linux only."
        )
    if shutil.which("perl") is None:
        raise RuntimeError("TeX Live installer requires perl, but perl was not found on PATH.")

    with tempfile.TemporaryDirectory(prefix="pitex-texlive-install-") as temp_dir:
        temp_path = Path(temp_dir)
        archive_path = temp_path / "install-tl-unx.tar.gz"
        profile_path = temp_path / "texlive.profile"

        log(f"Downloading TeX Live installer from {INSTALLER_URL}")
        download_installer(archive_path)
        installer_dir = extract_archive(archive_path, temp_path)
        write_profile(profile_path, target_root)

        command = [
            "perl",
            os.fspath(installer_dir / "install-tl"),
            "-profile",
            os.fspath(profile_path),
            "-repository",
            repository,
        ]
        log(f"Running TeX Live installer into {target_root}")
        subprocess.run(command, check=True)


def print_detect_summary(detection: dict) -> None:
    print(f"Status: {detection['status']}")
    print(f"Reason: {detection['reason']}")
    if detection.get("activeBinDir"):
        print(f"TeX bin: {detection['activeBinDir']}")
    if detection.get("texmfroot"):
        print(f"TEXMFROOT: {detection['texmfroot']}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Detect-first managed full TeX Live installer.")
    parser.add_argument("--json", action="store_true", help="Print machine-readable JSON.")
    parser.add_argument(
        "--install-managed-full",
        action="store_true",
        help="Download and run the upstream TeX Live installer when no existing TeX Live is detected.",
    )
    parser.add_argument(
        "--force-managed",
        action="store_true",
        help="Install managed TeX Live even when an existing TeX installation is detected.",
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="Show what would happen without installing."
    )
    parser.add_argument("--repository", default=DEFAULT_REPOSITORY)
    parser.add_argument("--target-root", type=Path, default=DEFAULT_TARGET_ROOT)
    args = parser.parse_args()

    target_root = args.target_root.expanduser().resolve()
    detection = detect_texlive()
    should_install = args.force_managed or detection["status"] == "missing"
    result = {
        "detection": detection,
        "targetRoot": os.fspath(target_root),
        "repository": args.repository,
        "willInstall": bool(args.install_managed_full and should_install),
        "reason": None,
    }

    if not should_install:
        result["reason"] = (
            "Existing TeX installation detected; managed full TeX Live install skipped."
        )
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print_detect_summary(detection)
            print(
                "\nExisting TeX installation detected. No managed TeX Live runtime was installed."
            )
        return 0

    if not args.install_managed_full:
        result["reason"] = "Managed full install requires --install-managed-full."
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print_detect_summary(detection)
            print(
                "\nNo usable TeX installation was detected. Run with --install-managed-full "
                "after user confirmation to install a Pitex-managed full TeX Live runtime."
            )
        return 0

    if args.dry_run:
        result["reason"] = "Dry run requested."
        if args.json:
            print(json.dumps(result, indent=2, sort_keys=True))
        else:
            print_detect_summary(detection)
            print(f"\nWould install managed full TeX Live to: {target_root}")
            print(f"Repository: {args.repository}")
        return 0

    run_install(repository=args.repository, target_root=target_root)
    post_detection = detect_texlive([os.fspath(target_root / "bin" / "*")])
    result["postDetection"] = post_detection
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print("\nManaged full TeX Live install completed.")
        print_detect_summary(post_detection)
    return 0 if post_detection["status"] == "existing-usable" else 1


if __name__ == "__main__":
    raise SystemExit(main())
