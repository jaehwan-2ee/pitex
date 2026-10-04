#!/usr/bin/env python3
"""Real-engine regression for microtype compatibility and interrupted updates.

Usage: PITEX_BIN=<directory containing both helpers> regress-live-updates.py
Requires TeX Live (xelatex/kpsewhich) and pdftotext on PATH. All source and
output files live in an owned temporary directory, never in a user project.
"""
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import time


bin_dir = Path(os.environ["PITEX_BIN"]).resolve()
with tempfile.TemporaryDirectory(prefix="pitex-live-updates-") as temporary:
    work = Path(temporary).resolve()
    root = work / "project"
    root.mkdir()
    main = root / "main.tex"
    # Compare paragraph-token behavior with the distribution's real XeLaTeX.
    probes = r"""
\newcount\PitexPars
\let\PitexOriginalPar\par
\def\PitexPar{\global\advance\PitexPars1\PitexOriginalPar}
\partokenname\PitexPar
\partokencontext=0 \setbox0=\vbox{Probe zero}
\typeout{PITEX-PAR-ZERO=\the\PitexPars}
\partokencontext=1 \setbox0=\vbox{Probe one}
\typeout{PITEX-PAR-ONE=\the\PitexPars}
\partokencontext=2 \setbox0=\vbox{Probe two\vadjust{Adjustment}}
\typeout{PITEX-PAR-TWO=\the\PitexPars}
\partokenname\par
"""
    text = ("\\documentclass[twocolumn]{article}\n"
            "\\usepackage{microtype}\n\\usepackage{hyperref}\n"
            "\\begin{document}\n" + probes + "\nLiveMarker0.\n"
            + ("A paragraph with enough text for protrusion and several pages. " * 12
               + "\\par\n") * 35 + "\\end{document}\n")
    main.write_text(text)
    reference = work / "reference"
    reference.mkdir()
    subprocess.run(["xelatex", "-interaction=nonstopmode", "-halt-on-error",
                    f"-output-directory={reference}", main.name], cwd=root,
                   stdout=subprocess.DEVNULL, check=True, timeout=60)
    expected_probes = [line for line in (reference / "main.log").read_text().splitlines()
                       if line.startswith("PITEX-PAR-")]
    assert expected_probes == ["PITEX-PAR-ZERO=0", "PITEX-PAR-ONE=1", "PITEX-PAR-TWO=3"]

    def exercise(interrupted):
        out = work / ("interrupted" if interrupted else "normal")
        out.mkdir()
        args = [str(bin_dir / "pitex-preview"), "--root", str(root),
                "--main", main.name, "--out", str(out), "--cache", str(work / "cache")]
        marker = work / "worker-ready"
        if interrupted:
            # First worker completes the real handshake but makes no file
            # progress. A new edit must kill it and start the real engine,
            # even though its trace is still empty (no checkpoint to resume).
            shim = work / "engine-shim"
            shim.write_text(f"#!{sys.executable}\n" +
                            "import os, socket, sys, time\nfrom pathlib import Path\n" +
                            f"marker = Path({str(marker)!r})\n" +
                            "if not marker.exists():\n"
                            " s = socket.socket(fileno=int(os.environ['TEXPRESSO_FD']))\n"
                            " data = b''\n"
                            " while len(data) < 12: data += s.recv(12 - len(data))\n"
                            " assert data == b'TEXPRESSOS01'\n"
                            " s.sendall(b'TEXPRESSOC01')\n"
                            " marker.touch()\n"
                            " while True: time.sleep(1)\n" +
                            f"os.execv({str(bin_dir / 'pitex-preview-xetex')!r}, sys.argv)\n")
            shim.chmod(0o755)
            args += ["--engine", str(shim)]
        process = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                   stderr=subprocess.DEVNULL, bufsize=0, start_new_session=True)
        pending = b""
        displayed = None

        def send(request):
            data = (json.dumps(request) + "\n").encode()
            while data:
                data = data[os.write(process.stdin.fileno(), data):]

        def update(generation, source):
            send({"op": "update", "generation": generation,
                  "files": [{"path": str(main), "text": source}], "closed": []})

        def wait(generation, timeout=60):
            nonlocal pending, displayed
            deadline = time.monotonic() + timeout
            while time.monotonic() < deadline:
                ready, _, _ = select.select([process.stdout], [], [],
                                            min(1, deadline - time.monotonic()))
                if not ready:
                    continue
                chunk = os.read(process.stdout.fileno(), 65536)
                assert chunk, (f"helper exited: {process.poll()}: "
                               + (out / "driver.log").read_text(errors="replace")[-3000:])
                pending += chunk
                while b"\n" in pending:
                    line, pending = pending.split(b"\n", 1)
                    event = json.loads(line)
                    assert event["event"] not in ("error", "failed"), event
                    if event["event"] == "published":
                        if displayed is not None:
                            send({"op": "release", "seq": displayed})
                        displayed = event["seq"]
                    if generation == 0 and event["event"] == "ready":
                        return event
                    if (event["event"] == "published" and event.get("complete")
                            and event["generation"] == generation):
                        return event
            raise AssertionError(f"no complete publication for generation {generation}: "
                                 + (out / "driver.log").read_text(errors="replace")[-2000:])

        def check(event, marker_text):
            assert event["errors"] == 0, event
            actual_probes = [line for line in Path(event["log"]).read_text().splitlines()
                             if line.startswith("PITEX-PAR-")]
            assert actual_probes == expected_probes, actual_probes
            pdf_text = subprocess.check_output(["pdftotext", event["pdf"], "-"], text=True)
            assert marker_text in pdf_text, f"preview omitted {marker_text}"

        try:
            wait(0)
            update(1, text)
            if interrupted:
                deadline = time.monotonic() + 15
                while not marker.exists() and time.monotonic() < deadline:
                    time.sleep(0.01)
                assert marker.exists(), "test worker never handshook"
                update(2, text.replace("LiveMarker0", "LiveMarker2"))
                check(wait(2, 30), "LiveMarker2")
                assert "[kill] worker might be stuck" in (out / "driver.log").read_text()
                print("PASS: interrupted worker restarts before its first checkpoint", flush=True)
                return
            check(wait(1), "LiveMarker0")
            for generation in range(2, 12):
                update(generation, text.replace("LiveMarker0", f"LiveMarker{generation}"))
                check(wait(generation, 30), f"LiveMarker{generation}")
            # Keep editing while a pass is running, without waiting for it.
            for generation in range(12, 32):
                update(generation, text.replace("LiveMarker0", f"LiveMarker{generation}"))
                time.sleep(0.1)
            check(wait(31, 30), "LiveMarker31")
            # A genuine undefined command must still surface, then clear
            # once corrected; a valid PDF alone is not proof of no TeX errors.
            update(32, text.replace("LiveMarker0", r"\PitexUndefined LiveMarker32"))
            broken = wait(32, 30)
            assert broken["errors"] > 0 and "Undefined control sequence" in broken["first_error"]
            update(33, text.replace("LiveMarker0", "LiveMarker33"))
            check(wait(33, 30), "LiveMarker33")
            print("PASS: microtype, paragraph tokens, repeated edits, error and recovery", flush=True)
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
            process.wait()

    exercise(False)
    exercise(True)
