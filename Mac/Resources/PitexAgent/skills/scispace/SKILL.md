---
name: scispace
description: Search and organize academic papers through SciSpace — discovery, triage metadata (abstract, venue, DOI), and literature-review synthesis.
---

# SciSpace

Use this skill when the user asks to find, search, review, or organize research papers, or asks for literature-discovery help.

This is the pi-port of the ChatGPT SciSpace connector: it queries SciSpace's public search endpoint, so no API key is required.

The helper script lives in the `scripts/` directory next to this SKILL.md:

```bash
python3 <this-skill-directory>/scripts/scispace_search.py "attention mechanisms"
python3 <this-skill-directory>/scripts/scispace_search.py "CRISPR gene therapy" --limit 20
python3 <this-skill-directory>/scripts/scispace_search.py "10.1038/nature14539" --json
```

## Workflow

1. **Discover** — run the search with the user's topic; refine the query terms if results look off-target.
2. **Triage** — each result carries title, year, venue, DOI, a scispace.com link, and the abstract. Present them as a compact list or markdown table so the user can scan relevance.
3. **Organize** — when the user asks to keep results, write them into the project as a markdown table or a `.bib` file (authors may be absent from the API response — prefer DOI + SciSpace link in that case).
4. **Synthesize** — for literature-review requests, group the found papers by theme and summarize each cluster; cite with the DOI/URL the search returned.

Follow up with narrower searches when the user drills into a subtopic.
