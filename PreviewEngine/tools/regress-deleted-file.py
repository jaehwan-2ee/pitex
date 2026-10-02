#!/usr/bin/env python3
"""Regression: a file removed from disk after being overridden by the editor
must surface as a real file-not-found — never typeset stale cached bytes.

Usage: PITEX_BIN=<dir with pitex-preview> [PITEX_PREVIEW_CACHE=<dir>]
       regress-deleted-file.py
Requires a TeX Live installation on PATH (kpsewhich)."""
import json, os, select, shutil, subprocess, sys, tempfile, time

BIN = os.environ.get("PITEX_BIN", os.path.join(os.path.dirname(__file__), "..", "out", "bin"))
work = tempfile.mkdtemp(prefix="pitex-b1low-")
try:
    root = os.path.join(work, "proj")
    out = os.path.join(work, "out")
    os.makedirs(os.path.join(root, "part"))
    os.makedirs(out)
    open(os.path.join(root, "main.tex"), "w").write(
        "\\documentclass{article}\\begin{document}\nTop.\\input{part/chap}\nEnd.\n\\end{document}\n")
    open(os.path.join(root, "part", "chap.tex"), "w").write("Chapter text.\n")

    env = dict(os.environ)
    env.setdefault("PITEX_PREVIEW_CACHE", os.path.join(work, "cache"))
    p = subprocess.Popen([os.path.join(BIN, "pitex-preview"), "--root", root,
                          "--main", "main.tex", "--out", out],
                         stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env, bufsize=0)

    buf = b""
    def wait(pred, timeout=90):
        global buf
        end = time.time() + timeout
        while time.time() < end:
            r, _, _ = select.select([p.stdout], [], [], max(0, end - time.time()))
            if not r:
                break
            buf += os.read(p.stdout.fileno(), 65536)
            while b"\n" in buf:
                line, buf = buf.split(b"\n", 1)
                ev = json.loads(line)
                if pred(ev):
                    return ev
        return None

    def send(obj):
        p.stdin.write((json.dumps(obj) + "\n").encode())
        p.stdin.flush()

    assert wait(lambda e: e["event"] == "ready", 15), "no ready"
    main = os.path.join(root, "main.tex")
    chap = os.path.join(root, "part", "chap.tex")
    send({"op": "update", "generation": 1,
          "files": [{"path": main, "text": open(main).read()},
                    {"path": chap, "text": open(chap).read()}], "closed": []})
    ev = wait(lambda e: e["event"] == "published" and e.get("complete"))
    assert ev, "baseline publication failed"

    # Editor override lands, then the file is deleted on disk, then closed.
    send({"op": "update", "generation": 2,
          "files": [{"path": chap, "text": "Chapter text.\n"}], "closed": []})
    wait(lambda e: e["event"] == "idle", 30)
    os.unlink(chap)
    send({"op": "update", "generation": 3, "files": [], "closed": [chap]})

    ev = wait(lambda e: e["event"] in ("published", "failed"))
    assert ev, "no event after delete+close"
    stale = ev["event"] == "published" and ev.get("complete") and ev["errors"] == 0
    assert not stale, "deleted file still typeset clean from stale cached bytes"
    assert "not found" in json.dumps(ev).lower(), f"no missing-file signal: {ev}"
    print("PASS: deletion surfaces file-not-found:", ev.get("message") or ev.get("code"))
    p.stdin.close(); p.terminate()
finally:
    shutil.rmtree(work, ignore_errors=True)
