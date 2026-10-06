---
name: submission-preflight
description: Check a LaTeX manuscript before submission for missing source/figure/bibliography
  files, unresolved references and citations, duplicate labels, compile-log warnings,
  and optional ChkTeX findings.
license: AGPL-3.0-or-later
metadata:
  runtime: Uses Node 22.19 or later or Bun on macOS, Ubuntu, and Windows. ChkTeX is
    optional and comes from the user's TeX distribution.
---

# Submission Preflight

Inspect the selected manuscript and the venue instructions the user provides. This skill checks and reports; it does not submit, upload, install tools, compile, or modify sources unless those actions are part of the user's request.

Run the portable local checker with an absolute script path:

```sh
node <skill-directory>/scripts/preflight.mjs --root <project-folder> --main main.tex --json
# Or: bun <skill-directory>/scripts/preflight.mjs --root <project-folder> --main main.tex
```

Use `--log <project-relative-log>` to inspect a specific existing compiler log. `--chktex` runs an installed ChkTeX process with a bounded timeout; missing ChkTeX is reported as unavailable and does not install anything. Sources and dependencies stay within the selected project. The checker follows static `input`, `include`, `subfile`, bibliography and figure references from the main file; it ignores comments and common verbatim content.

Report missing files, undefined keys, duplicate labels/keys and compiler-log warnings with file/line locations. Suggestions should preserve meaning and existing citation keys. The scanner is intentionally conservative: macros, conditionally selected files, custom bibliography packages and shell-generated content require inspection or a real final build. An unbuilt or dynamic project is not certified by a clean static scan.

For venue rules, inspect page limits, document class/options, anonymization, bibliography style, figure formats, declarations and required supplementary files against the actual instructions. Mark absent instructions as unchecked; do not invent journal-specific requirements. Verify author names and identifying material only when anonymization is required.

Use the existing LaTeX Compile skill for an authorized final build and Citation Management for bibliography repairs. Return a concise report with blockers, warnings, manual checks and the next corrective actions. Do not automatically mark a manuscript accepted or submitted.
