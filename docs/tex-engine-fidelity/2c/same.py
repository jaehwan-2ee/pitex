# Compares first-generation PDFs of two helper builds: per-page text and 72-dpi raster bytes.
# usage: same.py BIN_A BIN_B [doc ...]
import importlib.util, shutil, subprocess, sys, tempfile, hashlib
from pathlib import Path
S=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("tf", S/"base/Tools/tex-fidelity.py"); tf=importlib.util.module_from_spec(spec); spec.loader.exec_module(tf)
a, b = Path(sys.argv[1]), Path(sys.argv[2])
docs = sys.argv[3:] or sorted(p.name for p in tf.CORPUS.iterdir() if p.is_dir())
root = Path(tempfile.mkdtemp(prefix="same-"))
def fingerprint(pdf):
    text = subprocess.run(["pdftotext", "-layout", str(pdf), "-"], capture_output=True).stdout
    d = Path(tempfile.mkdtemp(dir=root)); subprocess.run(["pdftoppm", "-r", "72", "-gray", str(pdf), str(d/"p")], check=True)
    pages = [hashlib.sha256(p.read_bytes()).hexdigest()[:12] for p in sorted(d.iterdir())]
    return hashlib.sha256(text).hexdigest()[:12], pages
bad = 0
for name in docs:
    out = {}
    for tag, bindir in (("a", a), ("b", b)):
        work = root/f"{name}-{tag}"; shutil.copytree(tf.CORPUS/name, work); shutil.copy(tf.CORPUS/"refs.bib", work/"refs.bib")
        h = tf.Helper(bindir, work, root/f"{name}-{tag}-out", root/f"cache-{tag}", tf.program_of((work/"main.tex").read_text()))
        (root/f"cache-{tag}").mkdir(exist_ok=True)
        try:
            ev, _ = h.update(1, [])
            out[tag] = [fingerprint(ev["pdf"]) if ev.get("event") == "published" else ("FAILED", ev.get("event"), ev.get("code"))]
            # The harness's warm edit: a full body pass in the same session.
            source = (work/"main.tex").read_text()
            for g in (2, 3):
                ev, _ = h.update(g, [{"path": str(work/"main.tex"), "text": source.replace("\\begin{document}", "\\begin{document}" + "\\relax " * (1 + g % 2), 1)}])
                out[tag].append(fingerprint(ev["pdf"]) if ev.get("event") == "published" else out[tag][-1] if ev.get("event") == "idle" else ("FAILED", ev.get("event"), ev.get("code")))
            # An edit that renumbers sections and labels, so the warm pass reruns for its AUX file.
            ev, _ = h.update(4, [{"path": str(work/"main.tex"), "text": source.replace("\\begin{document}", "\\begin{document}\\section{Inserted}\\label{inserted}", 1)}])
            out[tag].append(fingerprint(ev["pdf"]) if ev.get("event") == "published" else out[tag][-1] if ev.get("event") == "idle" else ("FAILED", ev.get("event"), ev.get("code")))
        finally:
            h.close()
    same = out["a"] == out["b"]; bad += not same
    if not same: print("  a:", out["a"], "\n  b:", out["b"])
    print(f"{name:22} {'same' if same else 'DIFF'} " + ' '.join(f[0] for f in out['a']) + f" pages={len(out['a'][0][1])}", flush=True)
shutil.rmtree(root, ignore_errors=True)
sys.exit(1 if bad else 0)
