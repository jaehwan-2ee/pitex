import importlib.util, shutil, sys, time, tempfile
from pathlib import Path
S=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location("tf", S/"base/Tools/tex-fidelity.py"); tf=importlib.util.module_from_spec(spec); spec.loader.exec_module(tf)
bin_dir, name, out = Path(sys.argv[1]), sys.argv[2], Path(sys.argv[3])
shutil.rmtree(out, ignore_errors=True); out.mkdir(parents=True)
work=out/"work"; shutil.copytree(tf.CORPUS/name, work); shutil.copy(tf.CORPUS/"refs.bib", work/"refs.bib")
cache=S/"cache1"; cache.mkdir(exist_ok=True)
prog=tf.program_of((work/"main.tex").read_text())
for i in range(2):
    h=tf.Helper(bin_dir, work, out/f"o{i}", cache, prog)
    t=time.perf_counter(); orig=h.event
    def ev(d, orig=orig):
        e=orig(d)
        if e: print(f"{time.perf_counter()-t:.3f}", e.get("event"), e.get("complete"), e.get("pages"))
        return e
    h.event=ev
    h.update(1,[]); h.close()
