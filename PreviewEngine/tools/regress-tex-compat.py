#!/usr/bin/env python3
"""Common TeX package regressions against the installed XeLaTeX.

Usage: PITEX_BIN=<directory containing both helpers> regress-tex-compat.py
Requires TeX Live packages used below, xelatex, bibtex, kpsewhich and
pdftotext on PATH. External bibliography tools run only for the reference;
the preview must read an existing BBL without executing shell commands.
All files live in an owned temporary directory, never in a user project.
"""
from collections import Counter
import json
import os
from pathlib import Path
import select
import signal
import subprocess
import tempfile
import time

cases={
'macros':(r'\usepackage{xparse}',r'\NewDocumentCommand{\hello}{O{world}m}{#1 #2}\def\foo#1{Value #1}\let\bar\foo\hello{Test}\bar{7}\ExplSyntaxOn\fp_eval:n {sqrt(25)}\ExplSyntaxOff'),
'math':(r'\usepackage{amsmath,amssymb,mathtools,amsthm}',r'\begin{align}f(x)&=\int_0^1 x^2\,dx\\g(x)&=\begin{cases}0&x<0\\1&x\ge0\end{cases}\end{align}\begin{pmatrix}1&2\\3&4\end{pmatrix}'),
'fonts':(r'\usepackage{fontspec,unicode-math}\setmainfont{lmroman10-regular.otf}\setmathfont{latinmodern-math.otf}',r'Unicode: café naïve. $\symbf{v}\in\mathbb{R}^n$, $\sum_{k=1}^n k$.'),
'shaped-text':(r'\XeTeXgenerateactualtext=1\usepackage{fontspec}\setmainfont{lmroman10-regular.otf}\newfontfamily\mathtext{latinmodern-math.otf}',r'office affinity café naïve. \textit{office} {\mathtext 𝐀}'),
'korean':(r'\usepackage{kotex}',r'한글 문서와 수식 $x^2+1$.'),
'microtype':(r'\usepackage{microtype}',r'\texttt{https://example.org} This paragraph tests protrusion and paragraph hooks.'),
'lists':(r'\usepackage{enumitem}',r'\begin{enumerate}[label=\alph*)]\item First\item Second\end{enumerate}'),
'code':(r'\usepackage{listings}',r'\begin{lstlisting}[language=Python]'+'\nprint("hello")\n'+r'\end{lstlisting}'),
'algorithms':(r'\usepackage{algorithm,algpseudocode}',r'\begin{algorithm}\caption{Test}\begin{algorithmic}\State $x\gets1$\For{$i=1$ to $n$}\State $x\gets x+i$\EndFor\end{algorithmic}\end{algorithm}'),
'tables':(r'\usepackage{booktabs,tabularx,longtable,multirow}',r'\begin{tabularx}{\linewidth}{lX}\toprule Name&Description\\\midrule A&Cell\\\bottomrule\end{tabularx}\begin{longtable}{ll}One&Two\\Three&Four\end{longtable}'),
'units':(r'\usepackage{siunitx}',r'\qty{3.5}{\metre\per\second}\num{123456}\begin{tabular}{S}1.23\\4.56\end{tabular}'),
'links':(r'\usepackage{xcolor,hyperref,cleveref}',r'\section{Start}\label{sec:first}\Cref{sec:first}. \href{https://example.com}{External link}. \hyperref[sec:first]{Internal link}. \textcolor{red}{Colored text}.'),
'graphics':(r'\usepackage{graphicx,caption,subcaption}',r'\begin{figure}\begin{subfigure}{.4\linewidth}\includegraphics[width=\linewidth]{example-image-a}\caption{First}\end{subfigure}\begin{subfigure}{.4\linewidth}\includegraphics[width=\linewidth]{example-image-b}\caption{Second}\end{subfigure}\caption{Sample}\end{figure}'),
'drawing':(r'\usepackage{tikz}',r'\begin{tikzpicture}\fill[blue!30] (0,0) rectangle (2,1);\draw[red,thick,->] (0,0)--(2,2);\node at (1,1){Node};\end{tikzpicture}'),
'plot':(r'\usepackage{pgfplots}\pgfplotsset{compat=1.18}',r'\begin{tikzpicture}\begin{axis}[width=6cm]\addplot[domain=0:2,samples=12]{x^2};\end{axis}\end{tikzpicture}'),
'bibliography':(r'\usepackage[backend=bibtex]{biblatex}\addbibresource{references.bib}',r'Text \autocite{doe}.\printbibliography'),
'crossfiles':(r'\usepackage{import,subfiles}',r'\input{chapter}\import{parts/}{extra}\subfile{subdocument}'),
'boxes':(r'\usepackage[most]{tcolorbox}\usepackage{fancyhdr}\pagestyle{fancy}',r'\begin{tcolorbox}[enhanced,breakable,colback=blue!5,colframe=blue,title=Box]Box content and $a^2+b^2=c^2$.\end{tcolorbox}'),
'shadings':(r'\usepackage{tikz}',r'\begin{tikzpicture}\shade[left color=red,right color=blue] (0,0) rectangle (3,2);\fill[green,opacity=.3] (1,1) circle (1);\end{tikzpicture}'),
'graphics-options':(r'\usepackage{graphicx}',r'\includegraphics[width=4cm,angle=30,trim=10 20 30 40,clip]{example-image-a}\reflectbox{Reflection}\resizebox{5cm}{!}{Resize}'),
'natbib':(r'\usepackage{natbib}',r'Text \citet{doe}.\bibliographystyle{plainnat}\bibliography{references}'),
'pdfpages':(r'\usepackage{pdfpages}',r'\includepdf[pages=1]{example-image-a}'),
'font-packages':(r'\usepackage{newtxtext,newtxmath}',r'\textbf{Bold text} $\left(\int_0^1 x^2dx\right)$'),
'chemistry':(r'\usepackage{mhchem,chemformula}',r'\ce{H2O + CO2 -> H2CO3} \ch{H2SO4}'),
'circuits':(r'\usepackage{circuitikz,tikz-cd}',r'\begin{circuitikz}\draw (0,0) to[R=$R$] (2,0);\end{circuitikz}\begin{tikzcd}A\arrow[r]&B\end{tikzcd}'),
'engine-detection':(r'\usepackage{ifpdf,iftex}',r'\ifpdf PDF mode.\else DVI mode.\fi \ifXeTeX XeTeX mode.\fi'),
'primitives':('',r'\typeout{PITEX-EXPANDED=\expanded{abc}}\typeout{PITEX-COMPARE=\strcmp{same}{same}}\typeout{PITEX-UCHAR=\Uchar65}\typeout{PITEX-FILESIZE=\filesize{chapter.tex}}\typeout{PITEX-MD5=\mdfivesum{abc}} Primitive text.'),
}

bin_dir = Path(os.environ["PITEX_BIN"]).resolve()
with tempfile.TemporaryDirectory(prefix="pitex-tex-compat-") as temporary:
    work = Path(temporary).resolve()
    for name, (preamble, body) in cases.items():
        root = work / name
        root.mkdir()
        out = root / "preview"
        out.mkdir()
        reference = root / "reference"
        reference.mkdir()
        (root / "parts").mkdir()
        (root / "chapter.tex").write_text("Chapter text.\n")
        (root / "parts/extra.tex").write_text("Imported text.\n")
        (root / "subdocument.tex").write_text(
            r"\documentclass[main.tex]{subfiles}\begin{document}Subfile text.\end{document}")
        (root / "references.bib").write_text(
            "@article{doe,author={Jane Doe},title={Example},journal={Journal},year={2025}}\n")
        if name == "math":
            body = body.replace(r"\begin{pmatrix}", r"$\begin{pmatrix}").replace(
                r"\end{pmatrix}", r"\end{pmatrix}$")
        main = root / "main.tex"
        source = ("\\documentclass{article}\n" + preamble
                  + "\n\\begin{document}\nCompatibilityMarker0.\n" + body
                  + "\n\\end{document}\n")
        main.write_text(source)
        command = ["xelatex", "-interaction=nonstopmode", "-halt-on-error",
                   f"-output-directory={reference}", main.name]
        def compile_reference():
            result = subprocess.run(command, cwd=root, capture_output=True, timeout=60)
            assert result.returncode == 0, result.stdout.decode(errors="replace")[-3000:]
        compile_reference()
        if name in ("bibliography", "natbib"):
            result = subprocess.run(["bibtex", "main"], cwd=reference,
                                    env={**os.environ, "BIBINPUTS": str(root) + ":",
                                         "TEXINPUTS": str(root) + ":"},
                                    capture_output=True, timeout=60)
            assert result.returncode == 0, result.stdout.decode(errors="replace")
            (root / "main.bbl").write_bytes((reference / "main.bbl").read_bytes())
        compile_reference()
        compile_reference()
        ref_text = subprocess.check_output(["pdftotext", str(reference / "main.pdf"), "-"], text=True)

        helper = subprocess.Popen(
            [str(bin_dir / "pitex-preview"), "--root", str(root), "--main", main.name,
             "--out", str(out), "--cache", str(work / "cache")],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            bufsize=0, start_new_session=True)
        pending = b""
        displayed = None
        def send(request):
            data = (json.dumps(request) + "\n").encode()
            while data:
                data = data[os.write(helper.stdin.fileno(), data):]
        def wait(generation):
            global pending, displayed
            deadline = time.monotonic() + 60
            while time.monotonic() < deadline:
                ready, _, _ = select.select([helper.stdout], [], [], 1)
                if not ready:
                    continue
                chunk = os.read(helper.stdout.fileno(), 65536)
                assert chunk, f"{name}: helper exited {helper.poll()}"
                pending += chunk
                while b"\n" in pending:
                    line, pending = pending.split(b"\n", 1)
                    event = json.loads(line)
                    assert event["event"] not in ("failed", "error"), event
                    if event["event"] == "published":
                        if displayed is not None:
                            send({"op": "release", "seq": displayed})
                        displayed = event["seq"]
                        if event.get("complete") and event["generation"] == generation:
                            return event
            raise AssertionError(f"{name}: no complete generation {generation}: "
                                 + (out / "driver.log").read_text(errors="replace")[-2000:])
        try:
            for generation in (1, 2):
                marker = f"CompatibilityMarker{generation}"
                edited = source.replace("CompatibilityMarker0", marker)
                send({"op": "update", "generation": generation,
                      "files": [{"path": str(main), "text": edited}], "closed": []})
                event = wait(generation)
                assert event["errors"] == 0, (name, event)
                pdf_text = subprocess.check_output(["pdftotext", event["pdf"], "-"], text=True)
                assert marker in pdf_text, (name, "missing unsaved text")
                if name in ("fonts", "shaped-text"):
                    # Script glyphs have no cmap entry. They must copy out
                    # as the same Unicode characters as real XeLaTeX.
                    actual = pdf_text.replace(marker, "CompatibilityMarker0")
                    assert Counter(c for c in actual if not c.isspace()) == Counter(
                        c for c in ref_text if not c.isspace()), (actual, ref_text)
                if name in ("bibliography", "natbib"):
                    assert "Doe" in pdf_text and "2025" in pdf_text, pdf_text
                    assert "undefined" not in Path(event["log"]).read_text().lower()
                if name == "crossfiles":
                    assert all(s in pdf_text for s in ("Chapter text", "Imported text", "Subfile text"))
            print(f"PASS: {name}, reference and two unsaved generations", flush=True)
        finally:
            if helper.poll() is None:
                os.killpg(helper.pid, signal.SIGKILL)
            helper.wait()
