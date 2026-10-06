---
name: scientific-writing
description: Draft and revise scientific manuscript sections while preserving supplied
  results, uncertainty, citations, and the author's intended claims; check section
  structure and manuscript consistency.
license: MIT
metadata:
  upstream-version: '2.3'
  upstream-commit: 154988403bb5a18e9d3c0ce4e6d5e2e4b184a298
  runtime: Core guidance works on all Pitex platforms. Optional local audit scripts
    require Python 3.11 or later and only the standard library.
---

# Scientific Writing

Use the user's manuscript, notes, tables, and verified sources to draft or revise the requested sections. Preserve the selected language, audience, structure, and journal format. Use the project editor and existing Pi tools; no separate model service or API key is needed.

Respect the task already authorized by the user. Routine rewriting does not require a new permission request or a formal evidence manifest. Ask only for missing facts that materially affect the requested text, and continue work that does not depend on them.

- Preserve scientific meaning, numbers, units, uncertainty, and distinctions between association and causation. Do not invent results, methods, citations, authorship, declarations, or approvals.
- For new factual claims, use sources the agent can inspect and cite a locator. Mark unresolved evidence instead of filling gaps with plausible text. Existing supplied claims can be revised without expanding their scope.
- Keep Methods and Results consistent; explain limitations and separate observation from interpretation. Follow the user's venue instructions when supplied.
- Use confidentiality constraints the user has provided. Keep local audits local; obtaining public citation metadata does not require uploading the manuscript.

Read the relevant references when useful:

| Work | Reference |
|---|---|
| Introduction, Methods, Results, Discussion | [IMRaD structure](references/imrad_structure.md) |
| Clear prose and revision | [Writing principles](references/writing_principles.md) |
| Tables, figures, units and legends | [Figures and tables](references/figures_tables.md) |
| Formal evidence audit | [Source ledger](references/source_ledger.md), [evidence workflow](references/evidence_workflow.md) |
| Journal-specific submission requirements | [Journal policies](references/journal_policies.md), [reporting guidance](references/reporting_guidelines.md) |

For a requested formal audit, optional scripts in this skill's `scripts/` directory check references, consistency, authorship, manifests, and claim evidence. Use absolute script paths, run `--help` first, and consult [CLI reference](references/cli_reference.md). Check the installed Python version before using them. If Python 3.11 is unavailable, perform the requested review with Pi's existing file tools; do not silently install dependencies.

Report the changes and unresolved scientific questions proportionally to the task. For submission checks, use the bundled `submission-preflight` skill. For DOI/BibTeX work, use `citation-management`.

Adapted for Pitex from K-Dense Scientific Agent Skills. The bundled references, scripts, and templates retain their upstream content and MIT notice in [LICENSE.md](LICENSE.md).
