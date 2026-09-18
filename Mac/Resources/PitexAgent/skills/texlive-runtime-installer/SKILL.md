---
name: texlive-runtime-installer
description: Detect existing TeX Live or MacTeX first, then optionally install a Pitex-managed full TeX Live runtime only when no existing TeX Live installation is detected.
---

# TeX Live Runtime Installer

Use this skill when the user asks to install or repair LaTeX support.

The helper scripts live in the `scripts/` directory next to this SKILL.md.

Default behavior is detect-only:

```bash
python3 <this-skill-directory>/scripts/install_texlive.py
```

The script exits without installing when it detects an existing TeX Live or MacTeX installation.

## Full Managed Install

Only run the full install after the user explicitly confirms they want Pitex to download and run the TeX Live installer. The install is large and can take a long time.

```bash
python3 <this-skill-directory>/scripts/install_texlive.py --install-managed-full
```

The managed runtime is installed under:

```text
~/.cache/pitex/texlive/full
```

The installer does not use `sudo`, does not write `/Library/TeX`, does not write `/usr/local/texlive`, and does not modify shell startup files.

## Force Mode

If an existing TeX installation is partial or broken and the user still wants a separate managed runtime:

```bash
python3 <this-skill-directory>/scripts/install_texlive.py --install-managed-full --force-managed
```

Do not use force mode unless the user explicitly asks for it.

## Safety

Running `--install-managed-full` downloads and runs the upstream TeX Live installer. Ask for confirmation immediately before running that command.
