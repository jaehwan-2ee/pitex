#!/usr/bin/env python3
"""Built-in TeX engine fidelity benchmark.

Compiles every document in the corpus with the stock engine named by its
`% !TEX program` line (latexmk drives BibTeX/Biber) and with the embedded
preview helper, then compares the two PDFs: build result, log warnings, pages,
per-page text, per-page raster, fonts, links, outlines and wall time.

Usage:
  PITEX_BIN=<dir with pitex-preview and pitex-preview-xetex> \
    python3 Tools/tex-fidelity.py [--runs 3] [--dpi 72] [--only a,b] \
      [--out DIR] [--json results.json] [--markdown table.md]
  python3 Tools/tex-fidelity.py --self-test

Requires latexmk, pdflatex/xelatex, bibtex/biber and poppler-utils
(pdfinfo, pdftotext, pdftoppm, pdffonts, pdftohtml) on PATH. Standard
library only. Documents are copied into a temporary work directory; the
corpus is never written.
"""
import argparse
import difflib
import json
import os
import re
import select
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORPUS = ROOT / "Fixtures" / "tex-fidelity"
TIMEOUT = 300


def run(cmd, cwd=None, timeout=TIMEOUT):
    return subprocess.run(cmd, cwd=cwd, capture_output=True, timeout=timeout)


def program_of(source):
    match = re.search(r"^%\s*!TEX\s+program\s*=\s*(\w+)", source, re.M | re.I)
    return match.group(1).lower() if match else "pdflatex"


# ---------- reference (stock TeX Live) ----------

def latexmk(work, program, outdir):
    shutil.rmtree(outdir, ignore_errors=True)
    flag = "-xelatex" if program == "xelatex" else "-pdf"
    start = time.perf_counter()
    proc = run(["latexmk", flag, "-interaction=nonstopmode", "-f",
                f"-outdir={outdir}", "main.tex"], cwd=work)
    return time.perf_counter() - start, proc.returncode


def single_pass(work, program, outdir):
    start = time.perf_counter()
    run([program, "-interaction=nonstopmode", f"-output-directory={outdir}", "main.tex"], cwd=work)
    return time.perf_counter() - start


# ---------- built-in (embedded preview helper) ----------

class Helper:
    def __init__(self, bin_dir, work, out, cache):
        out.mkdir(mode=0o700, parents=True, exist_ok=True)
        self.out = out
        self.proc = subprocess.Popen(
            [str(bin_dir / "pitex-preview"), "--root", str(work), "--main", "main.tex",
             "--out", str(out), "--cache", str(cache)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            start_new_session=True)
        self.buffer = b""
        self.displayed = None

    def send(self, request):
        self.proc.stdin.write((json.dumps(request) + "\n").encode())
        self.proc.stdin.flush()

    def event(self, deadline):
        while b"\n" not in self.buffer:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return None
            ready, _, _ = select.select([self.proc.stdout], [], [], remaining)
            if ready:
                chunk = os.read(self.proc.stdout.fileno(), 65536)
                if not chunk:
                    raise RuntimeError(f"helper exited: {self.log_tail()}")
                self.buffer += chunk
        line, self.buffer = self.buffer.split(b"\n", 1)
        return json.loads(line)

    def update(self, generation, files):
        """Sends one generation; returns (complete publication or idle event, seconds)."""
        start = time.perf_counter()
        self.send({"op": "update", "generation": generation, "files": files, "closed": []})
        deadline = time.monotonic() + TIMEOUT
        while (event := self.event(deadline)) is not None:
            kind = event.get("event")
            if kind in ("failed", "error") and event.get("generation", generation) == generation:
                return event, time.perf_counter() - start
            if kind == "published":
                if self.displayed is not None:
                    self.send({"op": "release", "seq": self.displayed})
                self.displayed = event["seq"]
                if event.get("complete") and event.get("generation") == generation:
                    return event, time.perf_counter() - start
            if kind == "idle" and event.get("generation") == generation:
                return event, time.perf_counter() - start
        return {"event": "failed", "code": "timeout", "message": self.log_tail()}, TIMEOUT

    def log_tail(self):
        try:
            return (self.out / "driver.log").read_text(errors="replace")[-1500:]
        except OSError:
            return ""

    def close(self):
        try:
            self.send({"op": "quit"})
            self.proc.wait(timeout=5)
        except Exception:
            pass
        try:
            os.killpg(self.proc.pid, 9)
        except OSError:
            pass
        self.proc.wait()


# ---------- PDF metrics ----------

def page_count(pdf):
    proc = run(["pdfinfo", str(pdf)])
    match = re.search(rb"^Pages:\s+(\d+)", proc.stdout, re.M)
    return int(match.group(1)) if match else 0


def page_texts(pdf, pages):
    texts = []
    for page in range(1, pages + 1):
        proc = run(["pdftotext", "-f", str(page), "-l", str(page), str(pdf), "-"])
        # NFKC + casefold: ligature code points and small-caps ActualText are not content changes.
        text = unicodedata.normalize("NFKC", proc.stdout.decode("utf-8", "replace")).casefold()
        texts.append(text.split())
    return texts


def read_pgm(path):
    data = path.read_bytes()
    fields = re.match(rb"P5\s+(\d+)\s+(\d+)\s+(\d+)\s", data)
    width, height = int(fields.group(1)), int(fields.group(2))
    pixels = data[fields.end():]
    return width, height, pixels


def ink_rows(width, height, pixels, threshold=128):
    """Each row as an int bitmask of dark pixels."""
    table = bytes(ord("1") if value < threshold else ord("0") for value in range(256))
    # Bit x is pixel x of the row.
    return [int(pixels[y * width:(y + 1) * width].translate(table)[::-1], 2) for y in range(height)]


def dilate(rows):
    spread = [row | row << 1 | row >> 1 for row in rows]
    return [spread[y] | (spread[y - 1] if y else 0) | (spread[y + 1] if y + 1 < len(spread) else 0)
            for y in range(len(spread))]


def ink_f1(a_rows, b_rows):
    """Ink F1 with 1-pixel tolerance: 1.0 identical, 0.0 nothing in common."""
    height = max(len(a_rows), len(b_rows))
    a_rows = a_rows + [0] * (height - len(a_rows))
    b_rows = b_rows + [0] * (height - len(b_rows))
    a_ink = sum(row.bit_count() for row in a_rows)
    b_ink = sum(row.bit_count() for row in b_rows)
    if a_ink == 0 and b_ink == 0:
        return 1.0
    if a_ink == 0 or b_ink == 0:
        return 0.0
    a_wide, b_wide = dilate(a_rows), dilate(b_rows)
    recall = sum((a & bw).bit_count() for a, bw in zip(a_rows, b_wide)) / a_ink
    precision = sum((b & aw).bit_count() for b, aw in zip(b_rows, a_wide)) / b_ink
    return 0.0 if recall + precision == 0 else 2 * recall * precision / (recall + precision)


def rasters(pdf, dpi, directory):
    directory.mkdir(parents=True, exist_ok=True)
    run(["pdftoppm", "-r", str(dpi), "-gray", str(pdf), str(directory / "p")])
    pages = sorted(directory.glob("p-*.pgm"), key=lambda p: int(p.stem.split("-")[-1]))
    return [ink_rows(*read_pgm(page)) for page in pages]


def fonts(pdf):
    proc = run(["pdffonts", str(pdf)])
    names, unembedded = set(), 0
    for line in proc.stdout.decode("utf-8", "replace").splitlines()[2:]:
        parts = line.split()
        if not parts:
            continue
        names.add(re.sub(r"^[A-Z]{6}\+", "", parts[0]))
        # Columns: name type [encoding words] emb sub uni object ID
        if len(parts) >= 6 and parts[-5] == "no":
            unembedded += 1
    return sorted(names), unembedded


LINKS_JS = """var doc = Document.openDocument(scriptArgs[0]); var n = 0;
for (var i = 0; i < doc.countPages(); i++) n += doc.loadPage(i).getLinks().length;
print(n);
"""


def links(pdf, scratch):
    """Link annotations with an action, via MuPDF; None without mutool (pdftohtml splits links per text run)."""
    if not shutil.which("mutool"):
        return None
    script = scratch / "links.js"
    script.write_text(LINKS_JS)
    return int(run(["mutool", "run", str(script), str(pdf)]).stdout or 0)


def outline_items(pdf):
    proc = run(["pdftohtml", "-xml", "-i", "-q", "-stdout", str(pdf)])
    return len(re.findall(r"<item ", proc.stdout.decode("utf-8", "replace")))


def log_counts(log):
    try:
        text = log.read_text(errors="replace")
    except OSError:
        return None, None
    return len(re.findall(r"^! ", text, re.M)), len(re.findall(r"Warning", text))


def compare(reference, builtin, dpi, scratch):
    ref_pages, out_pages = page_count(reference), page_count(builtin)
    common = min(ref_pages, out_pages)
    ref_text, out_text = page_texts(reference, ref_pages), page_texts(builtin, out_pages)
    text = [difflib.SequenceMatcher(None, ref_text[i], out_text[i], autojunk=False).ratio()
            for i in range(common)] + [0.0] * abs(ref_pages - out_pages)
    ref_raster, out_raster = rasters(reference, dpi, scratch / "ref"), rasters(builtin, dpi, scratch / "out")
    raster = [ink_f1(ref_raster[i], out_raster[i]) for i in range(common)] + [0.0] * abs(ref_pages - out_pages)
    ref_fonts, ref_unembedded = fonts(reference)
    out_fonts, out_unembedded = fonts(builtin)
    return {
        "pages": [ref_pages, out_pages],
        "text_median": statistics.median(text) if text else 0.0,
        "text_min": min(text) if text else 0.0,
        "raster_median": statistics.median(raster) if raster else 0.0,
        "raster_min": min(raster) if raster else 0.0,
        "raster_pages": [round(score, 4) for score in raster],
        "fonts": [ref_fonts, out_fonts],
        "unembedded_fonts": [ref_unembedded, out_unembedded],
        "links": [links(reference, scratch), links(builtin, scratch)],
        "outline_items": [outline_items(reference), outline_items(builtin)],
        "pdf_bytes": [reference.stat().st_size, builtin.stat().st_size],
    }


# ---------- driver ----------

def bench(name, bin_dir, runs, dpi, workroot, cache):
    source_dir = CORPUS / name
    work = workroot / name
    shutil.copytree(source_dir, work)
    shutil.copy(CORPUS / "refs.bib", work / "refs.bib")
    main = work / "main.tex"
    source = main.read_text()
    program = program_of(source)
    result = {"name": name, "program": program}

    ref_dir = work / "ref"
    cold = []
    for _ in range(runs):
        seconds, code = latexmk(work, program, ref_dir)
        cold.append(seconds)
    reference = ref_dir / "main.pdf"
    result["ref_ok"] = code == 0 and reference.exists()
    result["ref_errors"], result["ref_warnings"] = log_counts(ref_dir / "main.log")
    result["ref_cold_s"] = statistics.median(cold)
    result["ref_warm_s"] = statistics.median(single_pass(work, program, ref_dir) for _ in range(runs))

    # Cold: a new helper session (shared format cache) until generation 1 completes.
    # Warm: an edit right after \begin{document} in the same session.
    builtin_cold, builtin_warm, first = [], [], None
    for attempt in range(runs):
        helper = Helper(bin_dir, work, workroot / f"{name}-out{attempt}", cache)
        try:
            start = time.perf_counter()
            event, _ = helper.update(1, [])
            builtin_cold.append(time.perf_counter() - start)
            if first is None:
                first = event
                if event.get("event") == "published":
                    kept = workroot / f"{name}-builtin.pdf"
                    shutil.copy(event["pdf"], kept)
                    shutil.copy(event["log"], workroot / f"{name}-builtin.log")
                    first = dict(event, pdf=str(kept), log=str(workroot / f"{name}-builtin.log"))
            if attempt == 0 and event.get("event") == "published":
                for generation in range(2, runs + 2):
                    # Each generation differs from disk and from the last one.
                    marker = "\\begin{document}" + "\\relax " * (1 + generation % 2)
                    edited = source.replace("\\begin{document}", marker, 1)
                    _, seconds = helper.update(generation, [{"path": str(main), "text": edited}])
                    builtin_warm.append(seconds)
        finally:
            helper.close()
    result["builtin_event"] = first.get("event")
    result["builtin_ok"] = first.get("event") == "published"
    result["builtin_errors"] = first.get("errors")
    result["builtin_first_error"] = first.get("first_error") or first.get("message")
    result["builtin_compat_warnings"] = (first.get("warnings") or "")[:400]
    result["builtin_cold_s"] = statistics.median(builtin_cold)
    result["builtin_warm_s"] = statistics.median(builtin_warm) if builtin_warm else None
    if result["builtin_ok"]:
        _, result["builtin_log_warnings"] = log_counts(Path(first["log"]))
    if result["ref_ok"] and result["builtin_ok"]:
        result.update(compare(reference, Path(first["pdf"]), dpi, workroot / f"{name}-raster"))
    return result


def markdown(results):
    def f(value, digits=3):
        return "—" if value is None else f"{value:.{digits}f}"
    lines = ["| Document | Engine | Ref ok | Built-in ok (errors) | Pages ref/built-in | Text median/min | Raster median/min | Fonts ref/built-in | Links ref/built-in | Outline ref/built-in | PDF KB ref/built-in | Ref cold/warm s | Built-in cold/warm s |",
             "|---|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for r in results:
        pages = "/".join(map(str, r.get("pages", ["—", "—"])))
        fonts_pair = r.get("fonts")
        font_cell = f"{len(fonts_pair[0])}/{len(fonts_pair[1])}" if fonts_pair else "—"
        lines.append(
            f"| {r['name']} | {r['program']} | {'yes' if r['ref_ok'] else 'NO'} | "
            f"{'yes' if r['builtin_ok'] else 'NO'} ({r.get('builtin_errors')}) | {pages} | "
            f"{f(r.get('text_median'))}/{f(r.get('text_min'))} | {f(r.get('raster_median'))}/{f(r.get('raster_min'))} | "
            f"{font_cell} | {'/'.join('—' if v is None else str(v) for v in r.get('links', [None, None]))} | "
            f"{'/'.join(map(str, r.get('outline_items', ['—', '—'])))} | "
            f"{'/'.join(str(b // 1024) for b in r.get('pdf_bytes', [])) or '—'} | "
            f"{f(r['ref_cold_s'], 2)}/{f(r['ref_warm_s'], 2)} | {f(r['builtin_cold_s'], 2)}/{f(r.get('builtin_warm_s'), 2)} |")
    return "\n".join(lines) + "\n"


def self_test():
    blank = [0] * 4
    box = [0, 0b0110, 0b0110, 0]
    shifted = [0, 0b1100, 0b1100, 0]
    far = [0, 0, 0, 1 << 10]
    assert ink_f1(blank, blank) == 1.0
    assert ink_f1(box, box) == 1.0
    assert ink_f1(box, shifted) == 1.0, "one-pixel shift is tolerated"
    assert ink_f1(box, far) == 0.0
    assert ink_f1(box, blank) == 0.0
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "p.pgm"
        path.write_bytes(b"P5\n3 2\n255\n" + bytes([0, 255, 0, 255, 255, 10]))
        assert ink_rows(*read_pgm(path)) == [0b101, 0b100]
    assert program_of("% !TEX program = xelatex\n") == "xelatex"
    assert program_of("\\documentclass{article}") == "pdflatex"
    print("self-test ok")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--dpi", type=int, default=72)
    parser.add_argument("--only", default="")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--json", type=Path)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    bin_dir = Path(os.environ.get("PITEX_BIN", "")).resolve()
    if not (bin_dir / "pitex-preview").is_file():
        sys.exit("PITEX_BIN must name the directory holding pitex-preview and pitex-preview-xetex")
    names = [n for n in args.only.split(",") if n] or sorted(
        p.name for p in CORPUS.iterdir() if (p / "main.tex").is_file())
    workroot = Path(tempfile.mkdtemp(prefix="pitex-fidelity-", dir=args.out))
    cache = workroot / "cache"
    cache.mkdir(mode=0o700)
    # Format generation is a one-time per-install cost; time it separately.
    warm = workroot / "_format"
    warm.mkdir()
    (warm / "main.tex").write_text("\\documentclass{article}\\begin{document}x\\end{document}\n")
    helper = Helper(bin_dir, warm, workroot / "_format-out", cache)
    start = time.perf_counter()
    event, _ = helper.update(1, [])
    helper.close()
    header = {"format_build_s": round(time.perf_counter() - start, 3), "format_event": event.get("event"),
              "tex_version": run(["pdftex", "--version"]).stdout.decode().splitlines()[0],
              "helpers": str(bin_dir), "runs": args.runs, "dpi": args.dpi}
    print(json.dumps(header), flush=True)
    results = []
    for name in names:
        result = bench(name, bin_dir, args.runs, args.dpi, workroot, cache)
        results.append(result)
        print(json.dumps({k: v for k, v in result.items() if k not in ("raster_pages", "fonts")}), flush=True)
    if args.json:
        args.json.write_text(json.dumps({"header": header, "results": results}, indent=1, ensure_ascii=False))
    table = markdown(results)
    if args.markdown:
        args.markdown.write_text(table)
    print(table)
    print(f"work directory: {workroot}")


if __name__ == "__main__":
    main()
