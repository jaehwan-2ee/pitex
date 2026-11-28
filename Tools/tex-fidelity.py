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
  python3 Tools/tex-fidelity.py --check BASELINE.json [--results RUN.json] \
      [--speed ratio|absolute] [--speed-tolerance 0.25]
  python3 Tools/tex-fidelity.py --ab BIN_A BIN_B [--samples 15] [--only a,b] \
      [--json ab.json] [--markdown ab.md]
  python3 Tools/tex-fidelity.py --self-test

--check runs the corpus (or reads --results) and exits 1 when a gated
document scores lower than in BASELINE, loses a definition-of-done gate it
held there, or the built-in engine got slower beyond the speed tolerance.

--ab times two helper builds against each other on one machine (the per-document
speed gate): interleaved new sessions, at least 15 per arm, and a document is
slower only when the 95% bootstrap interval of its median ratio lies above 1.05.
The corpus is slower when the interval of its median-of-medians ratio lies above 1.0.

Requires latexmk, pdflatex/xelatex, bibtex/biber and poppler-utils
(pdfinfo, pdftotext, pdftoppm, pdffonts, pdftohtml) on PATH. Standard
library only. Documents are copied into a temporary work directory; the
corpus is never written.
"""
import argparse
import difflib
import html
import json
import os
import random
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
    def __init__(self, bin_dir, work, out, cache, program):
        out.mkdir(mode=0o700, parents=True, exist_ok=True)
        self.out = out
        # The apps pass the engine of the project's build command the same way.
        self.proc = subprocess.Popen(
            [str(bin_dir / "pitex-preview"), "--root", str(work), "--main", "main.tex",
             "--out", str(out), "--cache", str(cache)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            env={**os.environ, "PITEX_PREVIEW_PROGRAM": program}, start_new_session=True)
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


# pdfTeX writes no ToUnicode for CMEX10 unless \pdfgentounicode is on, so its big operators copy
# as the ASCII letter at their code point (a display sum copies as "X"). These are the glyphs
# that glyphtounicode.tex (and xdvipdfmx, and the built-in engine) map; the reference side is
# corrected to them so text similarity counts real differences. Text and display sizes share
# a base glyph name: summationtext is "P", summationdisplay is "X".
CMEX_BIG_OPERATORS = {
    "P": "∑", "X": "∑",  # summation
    "Q": "∏", "Y": "∏",  # product
    "R": "∫", "Z": "∫",  # integral
    "S": "∪", "[": "∪",  # union
    "T": "∩", "\\": "∩",  # intersection
    "U": "⊎", "]": "⊎",  # unionmulti
    "V": "∧", "^": "∧",  # logicaland
    "W": "∨", "_": "∨",  # logicalor
    "`": "⨿", "a": "⨿",  # coproduct
    "J": "⊙", "K": "⊙",  # circledot
    "L": "⊕", "M": "⊕",  # circleplus
    "N": "⊗", "O": "⊗",  # circlemultiply
}

# Sized delimiters (big, Big, bigg, Bigg) by their cmex10 code, from the built-in encoding of
# cmex10.pfb (lmex10 has the same layout). glyphtounicode.tex has no entry for these names, so
# pdfTeX's ToUnicode CMap leaves them out: pdftotext drops the codes below 0x20 and shows the
# others as the letter at their code point (bracketleftBig is "h"). The built-in engine copies
# them as their base glyph, so the reference copy gets the base glyph's glyphtounicode.tex value.
# Extensible pieces (parenlefttp, braceex, vextendsingle, ...) copy the same way in both PDFs
# and are not listed.
CMEX_DELIMITERS = {code: char for char, codes in {
    "(": (0x00, 0x10, 0x12, 0x20), ")": (0x01, 0x11, 0x13, 0x21),
    "[": (0x02, 0x68, 0x14, 0x22), "]": (0x03, 0x69, 0x15, 0x23),
    "⌊": (0x04, 0x6A, 0x16, 0x24), "⌋": (0x05, 0x6B, 0x17, 0x25),
    "⌈": (0x06, 0x6C, 0x18, 0x26), "⌉": (0x07, 0x6D, 0x19, 0x27),
    "{": (0x08, 0x6E, 0x1A, 0x28), "}": (0x09, 0x6F, 0x1B, 0x29),
    "⟨": (0x0A, 0x44, 0x1C, 0x2A), "⟩": (0x0B, 0x45, 0x1D, 0x2B),
    "/": (0x0E, 0x2E, 0x1E, 0x2C), "\\": (0x0F, 0x2F, 0x1F, 0x2D),
}.items() for code in codes}

# Adds CMEX_DELIMITERS to the ToUnicode CMap of every CMEX10/LMMathExtension font (scriptArgs:
# input, output, {"hex code": "hex UTF-16"}). MuPDF buffers expose bytes by index only.
CMEX_DELIMITERS_JS = """var doc = new PDFDocument(scriptArgs[0]); var table = JSON.parse(scriptArgs[2]);
var block = "", count = 0;
for (var code in table) { block += "<" + code + "><" + table[code] + ">\\n"; count++; }
block = count + " beginbfchar\\n" + block + "endbfchar\\n";
for (var i = 1; i < doc.countObjects(); i++) {
  var font = doc.newIndirect(i, 0).resolve();
  var base = font && font.isDictionary() ? font.get("BaseFont") : null;
  if (!base || !base.isName() || !/^([A-Z]{6}\\+)?(CMEX\\d+|LMMathExtension\\d+(-Regular)?)$/.test(base.asName())) continue;
  var unicode = font.get("ToUnicode");
  if (unicode && unicode.isStream()) {
    var buffer = unicode.readStream(), text = "";
    for (var k = 0; k < buffer.length; k++) text += String.fromCharCode(buffer[k]);
    unicode.writeStream(text.replace("endcmap", block + "endcmap"));
  } else {
    font.put("ToUnicode", doc.addStream("/CIDInit/ProcSet findresource begin 12 dict begin begincmap " +
      "/CMapName/PitexCMEX def /CMapType 2 def 1 begincodespacerange <00><FF> endcodespacerange\\n" +
      block + "endcmap CMapName currentdict/CMap defineresource pop end end\\n", {}));
  }
}
doc.save(scriptArgs[1], "compress");
"""


def with_cmex_delimiters(pdf, scratch):
    """A copy of the reference whose CMEX fonts copy sized delimiters; the reference itself without mutool."""
    if not shutil.which("mutool"):
        return pdf
    scratch.mkdir(parents=True, exist_ok=True)
    script, out = scratch / "cmex-delimiters.js", scratch / "reference-cmex.pdf"
    script.write_text(CMEX_DELIMITERS_JS)
    table = {f"{code:02X}": f"{ord(char):04X}" for code, char in CMEX_DELIMITERS.items()}
    proc = run(["mutool", "run", str(script), str(pdf), str(out), json.dumps(table)])
    return out if proc.returncode == 0 and out.is_file() else pdf


def cmex_corrections(pdf):
    """Per page, one entry per CMEX big operator: ({text as pdftotext may join it}, corrected).

    pdftohtml names the font of each text run; pdftotext -bbox gives each word's box. A word is
    corrected only where a run of one table character in a CMEX font (or Latin Modern's copy,
    LMMathExtension) starts inside it. Plain pdftotext may join the operator to the next words
    on its line ("P" "n" -> "Pn"), so up to two following words are accepted.
    """
    xml = run(["pdftohtml", "-xml", "-i", "-stdout", "-zoom", "1", str(pdf)]).stdout.decode("utf-8", "replace")
    fonts = set(re.findall(
        r'<fontspec id="(\d+)"[^>]*family="(?:[A-Z]{6}\+)?(?:CMEX\d+|LMMathExtension\d+)(?:-Regular)?"', xml))
    runs = {}
    for number, body in re.findall(r'<page number="(\d+)"(.*?)</page>', xml, re.S):
        for top, left, font, char in re.findall(
                r'<text top="(-?\d+)" left="(-?\d+)"[^>]*font="(\d+)"[^>]*>(.*?)</text>', body):
            char = html.unescape(char)
            if font in fonts and char in CMEX_BIG_OPERATORS:
                runs.setdefault(int(number), []).append((float(left), float(top), char))
    if not runs:
        return {}
    bbox = run(["pdftotext", "-bbox", str(pdf), "-"]).stdout.decode("utf-8", "replace")
    corrections = {}
    for number, body in enumerate(re.findall(r"<page [^>]*>(.*?)</page>", bbox, re.S), 1):
        words = [(float(x0), float(y0), float(x1), float(y1), html.unescape(w)) for x0, y0, x1, y1, w in re.findall(
            r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">(.*?)</word>', body)]
        for left, top, char in runs.get(number, []):
            for index, (x0, y0, x1, y1, word) in enumerate(words):
                if x0 - 1 <= left <= x1 and y0 - 1 <= top <= y1 and char in word:
                    joined = [word + "".join(w[4] for w in words[index + 1:index + 1 + k]) for k in range(3)]
                    corrections.setdefault(number, []).append(
                        ({j: j.replace(char, CMEX_BIG_OPERATORS[char], 1) for j in joined}))
                    break
    return corrections


def correct_words(words, hits):
    """Each hit corrects the first remaining word it names; other words stay as extracted."""
    pending, out = list(hits), []
    for word in words:
        hit = next((h for h in pending if word in h), None)
        if hit is not None:
            pending.remove(hit)
            word = hit[word]
        out.append(word)
    return out


def page_texts(pdf, pages, reference=False):
    texts = []
    corrections = cmex_corrections(pdf) if reference else {}
    for page in range(1, pages + 1):
        proc = run(["pdftotext", "-f", str(page), "-l", str(page), str(pdf), "-"])
        words = correct_words(proc.stdout.decode("utf-8", "replace").split(), corrections.get(page, []))
        # NFKC + casefold: ligature code points and small-caps ActualText are not content changes.
        text = unicodedata.normalize("NFKC", " ".join(words)).casefold()
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
    ref_text = page_texts(with_cmex_delimiters(reference, scratch), ref_pages, reference=True)
    out_text = page_texts(builtin, out_pages)
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


# ---------- gate (--check) ----------

# Definition of done in docs/tex-engine-fidelity.md. Korean and CJK documents are reported, never gated.
INFORMATIONAL = {"korean-kotex", "korean-oblivoir", "beamer-korean"}
# Same helpers on the same machine give identical scores (CI runs agree to 4 decimals); this only absorbs float noise.
SCORE_TOLERANCE = 0.001


def doc_gates(r):
    """Per-document definition-of-done gates: name -> held."""
    pages, links_pair, outline = r.get("pages"), r.get("links"), r.get("outline_items")
    return {
        "build": not r["ref_ok"] or (r["builtin_ok"] and r.get("builtin_errors") == r.get("ref_errors")),
        "pages": bool(pages) and pages[0] == pages[1],
        "raster median >= 0.97": r.get("raster_median", 0.0) >= 0.97,
        "raster every page >= 0.90": r.get("raster_min", 0.0) >= 0.90,
        "text median >= 0.98": r.get("text_median", 0.0) >= 0.98,
        "text every page >= 0.95": r.get("text_min", 0.0) >= 0.95,
        "links equal": bool(links_pair) and links_pair[0] == links_pair[1],
        "outline equal": bool(outline) and outline[0] == outline[1],
        "fonts embedded": (r.get("unembedded_fonts") or [0, 0])[1] == 0,
    }


def doc_scores(r):
    """Higher is better; a drop beyond SCORE_TOLERANCE is a regression."""
    def gap(pair):
        return -abs(pair[0] - pair[1]) if pair and None not in pair else 0
    return {
        "text median": r.get("text_median", 0.0), "text min": r.get("text_min", 0.0),
        "raster median": r.get("raster_median", 0.0), "raster min": r.get("raster_min", 0.0),
        "page gap": gap(r.get("pages")), "link gap": gap(r.get("links")),
        "outline gap": gap(r.get("outline_items")),
        "error gap": gap([r.get("ref_errors"), r.get("builtin_errors")]),
    }


def corpus_raster(results):
    return statistics.median(r.get("raster_median", 0.0) for r in results) if results else 0.0


def speeds(results, mode):
    """Corpus speed figures: built-in/reference ratio medians, or built-in medians in seconds."""
    out = {}
    for kind in ("cold", "warm"):
        values = [r[f"builtin_{kind}_s"] / r[f"ref_{kind}_s"] if mode == "ratio" else r[f"builtin_{kind}_s"]
                  for r in results if r.get(f"builtin_{kind}_s") and r.get(f"ref_{kind}_s")]
        out[kind] = statistics.median(values) if values else None
    return out


def check(baseline, results, speed="ratio", tolerance=0.25):
    """Returns (failures, report lines) for gated documents present in both runs."""
    base = {r["name"]: r for r in baseline}
    now = [r for r in results if r["name"] in base and r["name"] not in INFORMATIONAL]
    was = [base[r["name"]] for r in now]
    failures = []
    for new, old in zip(now, was):
        name = new["name"]
        new_scores, old_scores = doc_scores(new), doc_scores(old)
        for metric, value in old_scores.items():
            if new_scores[metric] < value - SCORE_TOLERANCE:
                failures.append(f"{name}: {metric} {new_scores[metric]:.4f} < baseline {value:.4f}")
        new_gates = doc_gates(new)
        for gate, held in doc_gates(old).items():
            if held and not new_gates[gate]:
                failures.append(f"{name}: lost gate '{gate}'")
        if speed == "absolute":
            for kind in ("cold", "warm"):
                key = f"builtin_{kind}_s"
                if new.get(key) and old.get(key) and new[key] > old[key] * (1 + tolerance):
                    failures.append(f"{name}: built-in {kind} {new[key]:.3f} s > baseline {old[key]:.3f} s +{tolerance:.0%}")
    if corpus_raster(was) >= 0.99 > corpus_raster(now):
        failures.append(f"corpus raster median {corpus_raster(now):.4f} fell below 0.99")
    new_speed, old_speed = speeds(now, speed), speeds(was, speed)
    # Absolute: the corpus median may not get slower at all; ratio (shared CI runners): loose guard.
    corpus_tolerance = 0.0 if speed == "absolute" else tolerance
    for kind in ("cold", "warm"):
        if new_speed[kind] and old_speed[kind] and new_speed[kind] > old_speed[kind] * (1 + corpus_tolerance):
            failures.append(f"corpus built-in {kind} median ({speed}) {new_speed[kind]:.3f} > baseline "
                            f"{old_speed[kind]:.3f} +{corpus_tolerance:.0%}")
    lines = [f"Fidelity gate: {len(now)} gated documents ({', '.join(sorted(INFORMATIONAL))} informational)", "",
             "| Gate | Now | Baseline |", "|---|---|---|"]
    for gate in doc_gates(now[0]) if now else []:
        lines.append(f"| {gate} | {sum(doc_gates(r)[gate] for r in now)} / {len(now)} | "
                     f"{sum(doc_gates(r)[gate] for r in was)} / {len(was)} |")
    lines.append(f"| corpus raster median >= 0.99 | {corpus_raster(now):.4f} | {corpus_raster(was):.4f} |")
    for kind in ("cold", "warm"):
        f = lambda v: "—" if v is None else f"{v:.3f}"
        lines.append(f"| built-in {kind} median ({speed}) | {f(new_speed[kind])} | {f(old_speed[kind])} |")
    lines += ["", f"{len(failures)} regression(s)" + (":" if failures else "")] + [f"- {f}" for f in failures]
    return failures, lines


# ---------- A/B speed ----------

AB_SLOWER = 1.05


def median_ratio(a, b, rounds=2000, seed=0):
    """median(b)/median(a) with a 95% bootstrap interval (each arm resampled with replacement)."""
    rng = random.Random(seed)
    ratios = sorted(statistics.median(rng.choices(b, k=len(b))) / statistics.median(rng.choices(a, k=len(a)))
                    for _ in range(rounds))
    return statistics.median(b) / statistics.median(a), ratios[int(0.025 * rounds)], ratios[int(0.975 * rounds) - 1]


def corpus_ratio(gated, kind, rounds=2000, seed=0):
    """Median of B's per-document medians over A's, with a 95% bootstrap interval (every arm resampled)."""
    rng = random.Random(seed)
    corpus = lambda arm, draw: statistics.median(draw(arms[arm][kind]) for arms in gated)
    point = corpus("B", statistics.median) / corpus("A", statistics.median)
    resample = lambda xs: statistics.median(rng.choices(xs, k=len(xs)))
    ratios = sorted(corpus("B", resample) / corpus("A", resample) for _ in range(rounds))
    return point, ratios[int(0.025 * rounds)], ratios[int(0.975 * rounds) - 1]


def ab_verdict(docs):
    """Failures and table lines for {name: {"A": {"cold": [...], "warm": [...]}, "B": {...}}}.

    A gated document is slower only when the lower bound of its 95% interval for median(B)/median(A)
    is above AB_SLOWER; warm times are bimodal (an edit ends idle or republishes), so one median of a
    few samples is not evidence. Over the gated documents, the corpus is slower only when the 95%
    interval for (median of B's per-document medians)/(the same for A) lies above 1.0.
    """
    failures, lines = [], ["| Document | Cold B/A [95% CI] | Warm B/A [95% CI] |", "|---|---|---|"]
    for name, arms in docs.items():
        cells = []
        for kind in ("cold", "warm"):
            ratio, low, high = median_ratio(arms["A"][kind], arms["B"][kind])
            cells.append(f"{ratio:.3f} [{low:.3f}–{high:.3f}]")
            if name in INFORMATIONAL:
                continue
            if low > AB_SLOWER:
                failures.append(f"{name}: {kind} median B/A {ratio:.3f}, 95% CI lower bound {low:.3f} > {AB_SLOWER}")
        lines.append(f"| {name}{' (informational)' if name in INFORMATIONAL else ''} | {cells[0]} | {cells[1]} |")
    gated = [arms for name, arms in docs.items() if name not in INFORMATIONAL]
    for kind in ("cold", "warm") if gated else ():
        a, b = (statistics.median(statistics.median(arms[arm][kind]) for arms in gated) for arm in "AB")
        ratio, low, high = corpus_ratio(gated, kind)
        cell = f"{ratio:.3f} [{low:.3f}–{high:.3f}] (A {a:.3f} s, B {b:.3f} s)"
        lines.append(f"| corpus {kind} median of medians | {cell if kind == 'cold' else ''} | {cell if kind == 'warm' else ''} |")
        if low > 1.0:
            failures.append(f"corpus {kind} median of medians B/A {ratio:.3f}, 95% CI lower bound {low:.3f} > 1.0")
    lines += ["", f"{len(failures)} slower" + (":" if failures else "")] + [f"- {f}" for f in failures]
    return failures, lines


def ab_speed(bin_a, bin_b, samples, names, workroot):
    """Interleaved A/B timing: each sample is a new helper session per arm, arm order alternating.

    Cold is generation 1 to its complete publication; warm is the second edit right after
    \\begin{document} in that session (the first edit settles the session).
    """
    bins = {"A": bin_a, "B": bin_b}
    caches = {}
    for arm, bin_dir in bins.items():
        caches[arm] = workroot / f"cache-{arm}"
        caches[arm].mkdir(mode=0o700)
        for program in ("xelatex", "pdflatex"):
            prime = workroot / f"_format-{arm}-{program}"
            prime.mkdir()
            (prime / "main.tex").write_text("\\documentclass{article}\\begin{document}x\\end{document}\n")
            helper = Helper(bin_dir, prime, workroot / f"_format-{arm}-{program}-out", caches[arm], program)
            helper.update(1, [])
            helper.close()
    docs = {}
    for name in names:
        source = (CORPUS / name / "main.tex").read_text()
        program = program_of(source)
        times = {arm: {"cold": [], "warm": []} for arm in bins}
        for i in range(samples):
            for arm in ("A", "B") if i % 2 == 0 else ("B", "A"):
                work = workroot / f"{name}-{arm}-{i}"
                shutil.copytree(CORPUS / name, work)
                shutil.copy(CORPUS / "refs.bib", work / "refs.bib")
                helper = Helper(bins[arm], work, workroot / f"{name}-{arm}-{i}-out", caches[arm], program)
                try:
                    start = time.perf_counter()
                    event, _ = helper.update(1, [])
                    if event.get("event") != "published":
                        sys.exit(f"{name} ({arm}): {event.get('event')} {event.get('message', '')}")
                    times[arm]["cold"].append(time.perf_counter() - start)
                    for generation in (2, 3):
                        marker = "\\begin{document}" + "\\relax " * (1 + generation % 2)
                        edited = source.replace("\\begin{document}", marker, 1)
                        _, seconds = helper.update(generation, [{"path": str(work / "main.tex"), "text": edited}])
                    times[arm]["warm"].append(seconds)
                finally:
                    helper.close()
                shutil.rmtree(work, ignore_errors=True)
        docs[name] = times
        print(name, json.dumps({arm: {k: round(statistics.median(v), 4) for k, v in t.items()}
                                for arm, t in times.items()}), flush=True)
    return docs


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
        helper = Helper(bin_dir, work, workroot / f"{name}-out{attempt}", cache, program)
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
    assert len(CMEX_DELIMITERS) == 56 and CMEX_DELIMITERS[0x12] == "(" and CMEX_DELIMITERS[0x68] == "["
    assert correct_words(["Proof", "X", "Pn", "X"], [{"X": "∑"}, {"P": "∑", "Pn": "∑n"}]) == ["Proof", "∑", "∑n", "X"]
    doc = {"name": "d", "program": "pdflatex", "ref_ok": True, "builtin_ok": True, "ref_errors": 0,
           "builtin_errors": 0, "pages": [2, 2], "text_median": 0.99, "text_min": 0.96, "raster_median": 0.98,
           "raster_min": 0.95, "links": [3, 3], "outline_items": [1, 1], "unembedded_fonts": [0, 0],
           "ref_cold_s": 1.0, "ref_warm_s": 0.5, "builtin_cold_s": 1.0, "builtin_warm_s": 0.25}
    assert check([doc], [doc])[0] == []
    assert check([doc], [dict(doc, raster_median=0.9795)])[0] == [], "float noise is tolerated"
    assert check([doc], [dict(doc, raster_min=0.94)])[0][0].startswith("d: raster min"), "score drop"
    assert check([doc], [dict(doc, links=[3, 2])])[0] == ["d: link gap -1.0000 < baseline 0.0000",
                                                          "d: lost gate 'links equal'"]
    assert check([doc], [dict(doc, builtin_ok=False, builtin_errors=None, text_median=0.0)])[0], "build failure"
    korean = dict(doc, name="korean-kotex")
    assert check([korean], [dict(korean, text_median=0.5)])[0] == [], "Korean is informational"
    assert check([doc], [dict(doc, builtin_cold_s=1.2)])[0] == [], "ratio guard allows +25% on CI"
    assert check([doc], [dict(doc, builtin_cold_s=1.3)])[0][0].startswith("corpus built-in cold median (ratio)")
    slower = check([doc], [dict(doc, builtin_warm_s=0.27)], speed="absolute", tolerance=0.05)[0]
    assert slower[0].startswith("d: built-in warm 0.270 s") and slower[1].startswith("corpus built-in warm"), slower
    assert check([doc], [dict(doc, name="new")])[0] == [], "documents without a baseline are not gated"
    rng = random.Random(1)
    # Bimodal warm times: one median of 15 samples can move 40% with the mix, the interval does not exclude 1.
    fast_slow = lambda share: [rng.uniform(0.09, 0.11) if rng.random() > share else rng.uniform(0.17, 0.2)
                               for _ in range(15)]
    warm = fast_slow(0.5)
    same = {"A": {"cold": [1.0] * 15, "warm": warm}, "B": {"cold": [1.0] * 15, "warm": warm[::-1]}}
    assert ab_verdict({"d": same})[0] == [], ab_verdict({"d": same})[1]
    mixes = {"A": {"cold": [1.0] * 15, "warm": fast_slow(0.3)}, "B": {"cold": [1.0] * 15, "warm": fast_slow(0.7)}}
    assert not any("d:" in f for f in ab_verdict({"d": mixes})[0]), "a different fast/slow mix is not a slower document"
    slower = {"A": {"cold": [rng.uniform(0.98, 1.02) for _ in range(15)], "warm": [0.1] * 15},
              "B": {"cold": [rng.uniform(1.10, 1.14) for _ in range(15)], "warm": [0.1] * 15}}
    failures = ab_verdict({"d": slower})[0]
    assert failures[0].startswith("d: cold median B/A") and failures[1].startswith("corpus cold"), failures
    assert ab_verdict({"korean-kotex": slower})[0] == [], "Korean is informational"
    # Corpus rule: identical helpers (A/A, both arms drawn from one distribution) pass even when B's
    # point median lands above A's; a broad 3% slowdown that no single document proves fails.
    draw = lambda scale: {"cold": [scale * rng.uniform(0.95, 1.05) for _ in range(15)],
                          "warm": [scale * rng.uniform(0.095, 0.105) for _ in range(15)]}
    aa = {f"d{i}": {"A": draw(1.0), "B": draw(1.0)} for i in range(25)}
    assert ab_verdict(aa)[0] == [], ab_verdict(aa)[1]
    broad = {f"d{i}": {"A": draw(1.0), "B": draw(1.03)} for i in range(25)}
    failures = ab_verdict(broad)[0]
    assert not any(f.startswith("d") for f in failures), failures
    assert any(f.startswith("corpus cold median of medians") for f in failures), failures
    print("self-test ok")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--dpi", type=int, default=72)
    parser.add_argument("--only", default="")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--json", type=Path)
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--check", type=Path, metavar="BASELINE")
    parser.add_argument("--results", type=Path, help="with --check: gate this JSON instead of running")
    parser.add_argument("--speed", choices=("ratio", "absolute"), default="ratio",
                        help="ratio: built-in/reference corpus medians (shared runners); "
                             "absolute: built-in seconds per document, same machine as the baseline")
    parser.add_argument("--speed-tolerance", type=float, default=0.25)
    parser.add_argument("--ab", nargs=2, type=Path, metavar=("BIN_A", "BIN_B"),
                        help="interleaved A/B speed of two helper directories (no reference build)")
    parser.add_argument("--samples", type=int, default=15, help="with --ab: sessions per arm and document")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.ab:
        return ab_main(args)
    if args.check and args.results:
        return gate(args, json.loads(args.results.read_text())["results"])
    bin_dir = Path(os.environ.get("PITEX_BIN", "")).resolve()
    if not (bin_dir / "pitex-preview").is_file():
        sys.exit("PITEX_BIN must name the directory holding pitex-preview and pitex-preview-xetex")
    names = [n for n in args.only.split(",") if n] or sorted(
        p.name for p in CORPUS.iterdir() if (p / "main.tex").is_file())
    workroot = Path(tempfile.mkdtemp(prefix="pitex-fidelity-", dir=args.out))
    cache = workroot / "cache"
    cache.mkdir(mode=0o700)
    # Format generation is a one-time per-install cost for each engine personality; time it separately.
    format_s = {}
    for program in ("xelatex", "pdflatex"):
        warm = workroot / f"_format-{program}"
        warm.mkdir()
        (warm / "main.tex").write_text("\\documentclass{article}\\begin{document}x\\end{document}\n")
        helper = Helper(bin_dir, warm, workroot / f"_format-{program}-out", cache, program)
        start = time.perf_counter()
        event, _ = helper.update(1, [])
        helper.close()
        format_s[program] = round(time.perf_counter() - start, 3)
    header = {"format_build_s": format_s["xelatex"], "format_build_pdflatex_s": format_s["pdflatex"],
              "format_event": event.get("event"),
              "tex_version": run(["pdftex", "--version"]).stdout.decode().splitlines()[0],
              "helpers": str(bin_dir), "runs": args.runs, "dpi": args.dpi,
              "commit": os.environ.get("GITHUB_SHA") or run(["git", "-C", str(ROOT), "rev-parse", "HEAD"]).stdout.decode().strip()}
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
    if args.check:
        gate(args, results)


def ab_main(args):
    if args.samples < 15:
        sys.exit("--ab needs at least 15 samples per arm")
    names = [n for n in args.only.split(",") if n] or sorted(
        p.name for p in CORPUS.iterdir() if (p / "main.tex").is_file())
    workroot = Path(tempfile.mkdtemp(prefix="pitex-ab-", dir=args.out))
    docs = ab_speed(args.ab[0].resolve(), args.ab[1].resolve(), args.samples, names, workroot)
    failures, lines = ab_verdict(docs)
    if args.json:
        args.json.write_text(json.dumps({"samples": args.samples, "A": str(args.ab[0]), "B": str(args.ab[1]),
                                         "docs": docs}, indent=1))
    print("\n".join(lines))
    if args.markdown:
        args.markdown.write_text("\n".join(lines) + "\n")
    shutil.rmtree(workroot, ignore_errors=True)
    if failures:
        sys.exit(1)


def gate(args, results):
    failures, lines = check(json.loads(args.check.read_text())["results"], results, args.speed, args.speed_tolerance)
    print("\n".join(lines))
    if args.markdown:
        with args.markdown.open("a") as table:
            table.write("\n" + "\n".join(lines) + "\n")
    if failures:
        sys.exit(1)


if __name__ == "__main__":
    main()
