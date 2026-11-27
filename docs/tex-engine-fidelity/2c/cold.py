# Built-in cold/warm times per doc: median of N fresh sessions, same protocol as Tools/tex-fidelity.py.
# usage: cold.py BIN_DIR OUT.json RUNS [doc ...]
import importlib.util, json, shutil, statistics, sys, tempfile, time, hashlib
from pathlib import Path

repo = Path(__file__).resolve().parent / "base"
spec = importlib.util.spec_from_file_location("tf", repo / "Tools/tex-fidelity.py")
tf = importlib.util.module_from_spec(spec); spec.loader.exec_module(tf)

bin_dir, out_json, runs = Path(sys.argv[1]), sys.argv[2], int(sys.argv[3])
docs = sys.argv[4:] or sorted(p.name for p in tf.CORPUS.iterdir() if p.is_dir())
root = Path(tempfile.mkdtemp(prefix="cold-"))
cache = root / "cache"; cache.mkdir()
results = {}
for name in docs:
    work = root / name
    shutil.copytree(tf.CORPUS / name, work)
    shutil.copy(tf.CORPUS / "refs.bib", work / "refs.bib")
    main = work / "main.tex"; source = main.read_text(); program = tf.program_of(source)
    # warm the shared format cache once so cold measures session start, as the harness does
    h = tf.Helper(bin_dir, work, root / f"{name}-prime", cache, program); h.update(1, []); h.close()
    cold, warm, pdfhash, reruns = [], [], None, []
    for attempt in range(runs):
        out = root / f"{name}-out{attempt}"
        h = tf.Helper(bin_dir, work, out, cache, program)
        try:
            t = time.perf_counter(); ev, _ = h.update(1, []); cold.append(time.perf_counter() - t)
            if attempt == 0 and ev.get("event") == "published":
                pdfhash = hashlib.sha256(Path(ev["pdf"]).read_bytes()).hexdigest()[:16]
                for g in range(2, runs + 2):
                    edited = source.replace("\\begin{document}", "\\begin{document}" + "\\relax " * (1 + g % 2), 1)
                    _, s = h.update(g, [{"path": str(main), "text": edited}]); warm.append(s)
        finally:
            h.close()
        log = (out / "driver.log").read_text(errors="replace")
        reruns.append(log.count("rerun"))
    results[name] = {"cold": statistics.median(cold), "warm": statistics.median(warm) if warm else None,
                     "cold_all": cold, "pdf": pdfhash, "rerun_lines": reruns}
    print(name, f"{results[name]['cold']:.3f}", f"{results[name]['warm'] or 0:.3f}", pdfhash, reruns, flush=True)
Path(out_json).write_text(json.dumps(results, indent=1))
shutil.rmtree(root, ignore_errors=True)
