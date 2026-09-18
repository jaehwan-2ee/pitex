#!/usr/bin/env python3
"""SciSpace paper search for Pitex Agent — hits the public SciSpace search
endpoint (the same one scispace.com uses) and prints a readable table or
JSON for downstream synthesis."""
from __future__ import annotations

import argparse
import json
import sys
from urllib.request import Request, urlopen

ENDPOINT = "https://scispace.com/api/search"


def search(query: str, limit: int, offset: int) -> dict:
    payload = json.dumps(
        {"search_term": query, "limit": limit, "offset": offset}
    ).encode("utf-8")
    request = Request(
        ENDPOINT,
        data=payload,
        headers={
            "Content-Type": "application/json",
            "Accept": "application/json",
            "User-Agent": "pitex-agent",
        },
    )
    with urlopen(request, timeout=30) as response:
        return json.loads(response.read().decode("utf-8"))


def paper_url(paper: dict) -> str:
    slug = paper.get("full_slug")
    return f"https://scispace.com/papers/{slug}" if slug else ""


def render(papers: list[dict]) -> str:
    lines = []
    for index, paper in enumerate(papers, 1):
        title = (paper.get("title") or "Untitled").strip()
        year = (paper.get("date") or "")[:4]
        venue = paper.get("journal") or paper.get("publisher") or ""
        doi = paper.get("doi") or ""
        url = paper_url(paper)
        lines.append(f"{index}. {title} ({year or 'n.d.'})")
        if venue:
            lines.append(f"   {venue}")
        if doi:
            lines.append(f"   DOI: {doi}")
        if url:
            lines.append(f"   {url}")
        abstract = (paper.get("abstract") or "").strip()
        if abstract:
            lines.append(f"   {abstract[:300]}{'…' if len(abstract) > 300 else ''}")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Search papers on SciSpace.")
    parser.add_argument("query", help="Free-text query (topic, title, author, DOI).")
    parser.add_argument("--limit", type=int, default=10, help="Max results (default 10).")
    parser.add_argument("--offset", type=int, default=0, help="Pagination offset.")
    parser.add_argument("--json", action="store_true", help="Emit raw JSON.")
    args = parser.parse_args()

    try:
        result = search(args.query, args.limit, args.offset)
    except Exception as error:  # noqa: BLE001 - surface any transport failure
        print(f"SciSpace search failed: {error}", file=sys.stderr)
        return 1

    papers = result.get("data") or []
    if args.json:
        print(json.dumps({"total": result.get("total"), "papers": papers}, indent=2))
        return 0
    print(f"SciSpace: {result.get('total', 0)} results for “{args.query}”\n")
    print(render(papers))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
