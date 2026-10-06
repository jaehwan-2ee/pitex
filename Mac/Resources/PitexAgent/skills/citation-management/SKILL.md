---
name: citation-management
description: Verify DOI and citation metadata, create or repair BibTeX entries, find
  duplicate citation keys and identifiers, and organize the bibliography for a LaTeX
  manuscript.
license: MIT
metadata:
  upstream-version: '2.3'
  upstream-commit: 154988403bb5a18e9d3c0ce4e6d5e2e4b184a298
  runtime: Pi file tools work on all platforms. The bundled DOI helper uses Node 22.19+
    or Bun; optional upstream Python scripts require Python 3.9+, and online scripts
    also require requests.
---

# Citation Management

Work on the manuscript's actual bibliography and citation commands. Preserve existing citation keys unless the user requests a rename; update every affected citation when renaming a key. Use DOI, PMID, arXiv identifiers, titles and authors to verify identity, and retain distinct editions or versions when appropriate.

## Local bibliography work

Read the selected `.bib` files and referenced `.tex` documents. Check duplicate keys/DOIs, missing fields, malformed entries and unresolved citation keys. Preserve LaTeX braces, accents, capitalization protection and string macros. Explain changes before applying broad deduplication; never merge entries solely because titles look similar. Do not fabricate metadata for missing identifiers.

Optional dependency-free Python helpers in this skill include `format_bibtex.py` and `_common.py`; run the script with `--help` and an absolute path. They require Python 3.9 or later. If Python is unavailable, use Pi's existing file tools for the same task. See [BibTeX formatting](references/bibtex_formatting.md) and [citation checklist](assets/citation_checklist.md).

## DOI metadata

The portable helper needs no packages or credentials and prints verified BibTeX without modifying project files:

```sh
node <skill-directory>/scripts/doi_metadata.mjs 10.1038/nphys1170
# Or: bun <skill-directory>/scripts/doi_metadata.mjs <DOI>
```

It queries DOI content negotiation, verifies the returned DOI, and falls back to Crossref metadata. Use `--json` for structured metadata. Network errors or missing records are unresolved results, not evidence that the paper does not exist. Compare metadata against the source before inserting or replacing an entry.

## Literature discovery

Use the existing SciSpace skill, public DOI metadata, or user-connected research tools. The optional upstream scripts support OpenAlex, PubMed and Google Scholar; online scripts need `requests`, and Scholar also needs `scholarly`. Prefer maintained APIs; Scholar scraping can be blocked. Use existing user-configured credentials where present, and do not add accounts or install dependencies for a simple bibliography edit.

Consult [API contracts](references/api_contracts.md), [metadata extraction](references/metadata_extraction.md), or [search strategies](references/search_strategies.md) for the selected source. Search metadata locates a paper; it does not prove its findings. Cite a page or section only after reading the source.

Adapted for Pitex from K-Dense Scientific Agent Skills. Upstream helper files retain the MIT notice in [LICENSE.md](LICENSE.md); `doi_metadata.mjs` is Pitex-authored.
