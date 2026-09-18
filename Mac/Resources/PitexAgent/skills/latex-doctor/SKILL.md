---
name: latex-doctor
description: Detect Tectonic plus TeX Live or MacTeX availability, report missing LaTeX tools, and run small compile smoke tests when possible.
---

# LaTeX Doctor

Use this skill when the user asks whether LaTeX, Tectonic, TeX Live, MacTeX, `latexmk`, `pdflatex`, `xelatex`, `lualatex`, `biber`, or `kpsewhich` are installed or working.

The helper scripts live in the `scripts/` directory next to this SKILL.md:

```bash
python3 <this-skill-directory>/scripts/latex_doctor.py
```

For machine-readable output:

```bash
python3 <this-skill-directory>/scripts/latex_doctor.py --json
```

## Interpretation

- `ready`: At least one runtime passed a smoke compile. Prefer `latex-compile` in `auto` mode.
- `existing-usable`: Use the existing TeX installation. Do not install managed TeX Live.
- `existing-partial`: Report the gaps. Do not install managed TeX Live unless the user explicitly asks to replace or bypass the partial installation.
- `missing`: Managed full TeX Live can be offered through `texlive-runtime-installer`.

## Output Contract

Summarize:

- detector status
- Tectonic path and smoke-test result when available
- detected TeX bin directory
- `TEXMFROOT` when available
- missing required or recommended tools
- TeX Live smoke-test result when a compile was attempted
