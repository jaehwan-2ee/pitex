//! Common TeX package regressions against the installed XeLaTeX.
//!
//! Usage: PITEX_BIN=<directory containing both helpers> preview-regress-tex-compat
//! Requires TeX Live packages used below, xelatex, bibtex, kpsewhich and
//! pdftotext on PATH. External bibliography tools run only for the reference;
//! the preview must read an existing BBL without executing shell commands.
//! All files live in an owned temporary directory, never in a user project.

use pitex_native_tools::preview_regression;

use pitex_native_tools::TempDir;
use preview_regression::{output_timeout, Session};
use serde_json::json;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_secs(60);

fn tail(text: &str, count: usize) -> String {
    text.chars()
        .skip(text.chars().count().saturating_sub(count))
        .collect()
}

fn compile_reference(root: &Path, reference: &Path) -> Result<(), Box<dyn Error>> {
    let output = output_timeout(
        Command::new("xelatex")
            .args(["-interaction=nonstopmode", "-halt-on-error"])
            .arg(format!("-output-directory={}", reference.display()))
            .arg("main.tex")
            .current_dir(root),
        TIMEOUT,
    )?;
    if !output.status.success() {
        return Err(tail(&String::from_utf8_lossy(&output.stdout), 3000).into());
    }
    Ok(())
}

fn pdf_text(path: &Path) -> Result<String, Box<dyn Error>> {
    let output =
        preview_regression::output(Command::new("pdftotext").arg(path).arg("-"), None, true)?;
    if !output.status.success() {
        return Err(format!("pdftotext exited {}", output.status).into());
    }
    // Python's text=True performs universal newline conversion.
    Ok(String::from_utf8(output.stdout)?
        .replace("\r\n", "\n")
        .replace('\r', "\n"))
}

fn character_counts(text: &str) -> BTreeMap<char, usize> {
    let mut counts = BTreeMap::new();
    for character in text.chars().filter(|character| {
        // str.isspace() also includes these four Unicode information separators.
        !character.is_whitespace() && !('\u{001c}'..='\u{001f}').contains(character)
    }) {
        *counts.entry(character).or_default() += 1;
    }
    counts
}

fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin_dir =
        std::path::PathBuf::from(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN is required")?);
    let bin_dir = if bin_dir.is_absolute() {
        bin_dir
    } else {
        std::env::current_dir()?.join(bin_dir)
    };
    let bin_dir = fs::canonicalize(&bin_dir).unwrap_or(bin_dir);
    let temporary = TempDir::new("pitex-tex-compat-")?;
    let work = fs::canonicalize(temporary.path())?;
    for &(name, preamble, original_body) in CASES {
        preview_regression::check_interrupt()?;
        let root = work.join(name);
        fs::create_dir(&root)?;
        let out = root.join("preview");
        fs::create_dir(&out)?;
        let reference = root.join("reference");
        fs::create_dir(&reference)?;
        fs::create_dir(root.join("parts"))?;
        fs::write(root.join("chapter.tex"), "Chapter text.\n")?;
        fs::write(root.join("parts/extra.tex"), "Imported text.\n")?;
        fs::write(
            root.join("subdocument.tex"),
            r"\documentclass[main.tex]{subfiles}\begin{document}Subfile text.\end{document}",
        )?;
        fs::write(
            root.join("references.bib"),
            "@article{doe,author={Jane Doe},title={Example},journal={Journal},year={2025}}\n",
        )?;
        let body = if name == "math" {
            original_body
                .replace(r"\begin{pmatrix}", r"$\begin{pmatrix}")
                .replace(r"\end{pmatrix}", r"\end{pmatrix}$")
        } else {
            original_body.to_owned()
        };
        let main = root.join("main.tex");
        let source = format!("\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\nCompatibilityMarker0.\n{body}\n\\end{{document}}\n");
        fs::write(&main, &source)?;
        compile_reference(&root, &reference)?;
        if matches!(name, "bibliography" | "natbib") {
            let output = output_timeout(
                Command::new("bibtex")
                    .arg("main")
                    .current_dir(&reference)
                    .env("BIBINPUTS", format!("{}:", root.display()))
                    .env("TEXINPUTS", format!("{}:", root.display())),
                TIMEOUT,
            )?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stdout).into_owned().into());
            }
            fs::copy(reference.join("main.bbl"), root.join("main.bbl"))?;
        }
        compile_reference(&root, &reference)?;
        compile_reference(&root, &reference)?;
        let reference_text = pdf_text(&reference.join("main.pdf"))?;
        let mut session = Session::spawn(
            Command::new(bin_dir.join("pitex-preview"))
                .arg("--root")
                .arg(&root)
                .arg("--main")
                .arg("main.tex")
                .arg("--out")
                .arg(&out)
                .arg("--cache")
                .arg(work.join("cache"))
                .stderr(Stdio::null()),
            &out,
        )?;
        for generation in [1, 2] {
            let marker = format!("CompatibilityMarker{generation}");
            let edited = source.replace("CompatibilityMarker0", &marker);
            session.send(&json!({"op": "update", "generation": generation,
                "files": [{"path": main, "text": edited}], "closed": []}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 {
                return Err(format!("{name}: {event}").into());
            }
            let pdf = event["pdf"]
                .as_str()
                .ok_or("published event has no PDF path")?;
            let text = pdf_text(Path::new(pdf))?;
            if !text.contains(&marker) {
                return Err(format!("{name}: missing unsaved text").into());
            }
            if matches!(name, "fonts" | "shaped-text") {
                // Script glyphs have no cmap entry. They must copy out as the
                // same Unicode characters as real XeLaTeX.
                let actual = text.replace(&marker, "CompatibilityMarker0");
                if character_counts(&actual) != character_counts(&reference_text) {
                    return Err(format!("{actual:?}, {reference_text:?}").into());
                }
            }
            if matches!(name, "bibliography" | "natbib") {
                if !text.contains("Doe") || !text.contains("2025") {
                    return Err(text.into());
                }
                let log = event["log"]
                    .as_str()
                    .ok_or("published event has no log path")?;
                if fs::read_to_string(log)?
                    .to_lowercase()
                    .contains("undefined")
                {
                    return Err(format!("{name}: undefined bibliography entry").into());
                }
            }
            if name == "crossfiles"
                && ["Chapter text", "Imported text", "Subfile text"]
                    .iter()
                    .any(|part| !text.contains(part))
            {
                return Err(format!("{name}: missing included text").into());
            }
        }
        // Only extracted text of the font cases is compared with XeLaTeX.
        preview_regression::pass(
            name,
            matches!(name, "fonts" | "shaped-text").then_some("xetex"),
            &[&source],
            "reference and two unsaved generations",
        )?;
    }
    preview_regression::check_interrupt()
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(if preview_regression::interrupted(error.as_ref()) {
            130
        } else {
            1
        });
    }
}
// The fixtures retain the exact source bytes from the Python tool.
const CASES: &[(&str, &str, &str)] = &[
    (
        r##"macros"##,
        r##"\usepackage{xparse}"##,
        r##"\NewDocumentCommand{\hello}{O{world}m}{#1 #2}\def\foo#1{Value #1}\let\bar\foo\hello{Test}\bar{7}\ExplSyntaxOn\fp_eval:n {sqrt(25)}\ExplSyntaxOff"##,
    ),
    (
        r##"math"##,
        r##"\usepackage{amsmath,amssymb,mathtools,amsthm}"##,
        r##"\begin{align}f(x)&=\int_0^1 x^2\,dx\\g(x)&=\begin{cases}0&x<0\\1&x\ge0\end{cases}\end{align}\begin{pmatrix}1&2\\3&4\end{pmatrix}"##,
    ),
    (
        r##"fonts"##,
        r##"\usepackage{fontspec,unicode-math}\setmainfont{lmroman10-regular.otf}\setmathfont{latinmodern-math.otf}"##,
        r##"Unicode: café naïve. $\symbf{v}\in\mathbb{R}^n$, $\sum_{k=1}^n k$."##,
    ),
    (
        r##"shaped-text"##,
        r##"\XeTeXgenerateactualtext=1\usepackage{fontspec}\setmainfont{lmroman10-regular.otf}\newfontfamily\mathtext{latinmodern-math.otf}"##,
        r##"office affinity café naïve. \textit{office} {\mathtext 𝐀}"##,
    ),
    (
        r##"korean"##,
        r##"\usepackage{kotex}"##,
        r##"한글 문서와 수식 $x^2+1$."##,
    ),
    (
        r##"microtype"##,
        r##"\usepackage{microtype}"##,
        r##"\texttt{https://example.org} This paragraph tests protrusion and paragraph hooks."##,
    ),
    (
        r##"lists"##,
        r##"\usepackage{enumitem}"##,
        r##"\begin{enumerate}[label=\alph*)]\item First\item Second\end{enumerate}"##,
    ),
    (
        r##"code"##,
        r##"\usepackage{listings}"##,
        r##"\begin{lstlisting}[language=Python]
print("hello")
\end{lstlisting}"##,
    ),
    (
        r##"algorithms"##,
        r##"\usepackage{algorithm,algpseudocode}"##,
        r##"\begin{algorithm}\caption{Test}\begin{algorithmic}\State $x\gets1$\For{$i=1$ to $n$}\State $x\gets x+i$\EndFor\end{algorithmic}\end{algorithm}"##,
    ),
    (
        r##"tables"##,
        r##"\usepackage{booktabs,tabularx,longtable,multirow}"##,
        r##"\begin{tabularx}{\linewidth}{lX}\toprule Name&Description\\\midrule A&Cell\\\bottomrule\end{tabularx}\begin{longtable}{ll}One&Two\\Three&Four\end{longtable}"##,
    ),
    (
        r##"units"##,
        r##"\usepackage{siunitx}"##,
        r##"\qty{3.5}{\metre\per\second}\num{123456}\begin{tabular}{S}1.23\\4.56\end{tabular}"##,
    ),
    (
        r##"links"##,
        r##"\usepackage{xcolor,hyperref,cleveref}"##,
        r##"\section{Start}\label{sec:first}\Cref{sec:first}. \href{https://example.com}{External link}. \hyperref[sec:first]{Internal link}. \textcolor{red}{Colored text}."##,
    ),
    (
        r##"graphics"##,
        r##"\usepackage{graphicx,caption,subcaption}"##,
        r##"\begin{figure}\begin{subfigure}{.4\linewidth}\includegraphics[width=\linewidth]{example-image-a}\caption{First}\end{subfigure}\begin{subfigure}{.4\linewidth}\includegraphics[width=\linewidth]{example-image-b}\caption{Second}\end{subfigure}\caption{Sample}\end{figure}"##,
    ),
    (
        r##"drawing"##,
        r##"\usepackage{tikz}"##,
        r##"\begin{tikzpicture}\fill[blue!30] (0,0) rectangle (2,1);\draw[red,thick,->] (0,0)--(2,2);\node at (1,1){Node};\end{tikzpicture}"##,
    ),
    (
        r##"plot"##,
        r##"\usepackage{pgfplots}\pgfplotsset{compat=1.18}"##,
        r##"\begin{tikzpicture}\begin{axis}[width=6cm]\addplot[domain=0:2,samples=12]{x^2};\end{axis}\end{tikzpicture}"##,
    ),
    (
        r##"bibliography"##,
        r##"\usepackage[backend=bibtex]{biblatex}\addbibresource{references.bib}"##,
        r##"Text \autocite{doe}.\printbibliography"##,
    ),
    (
        r##"crossfiles"##,
        r##"\usepackage{import,subfiles}"##,
        r##"\input{chapter}\import{parts/}{extra}\subfile{subdocument}"##,
    ),
    (
        r##"boxes"##,
        r##"\usepackage[most]{tcolorbox}\usepackage{fancyhdr}\pagestyle{fancy}"##,
        r##"\begin{tcolorbox}[enhanced,breakable,colback=blue!5,colframe=blue,title=Box]Box content and $a^2+b^2=c^2$.\end{tcolorbox}"##,
    ),
    (
        r##"shadings"##,
        r##"\usepackage{tikz}"##,
        r##"\begin{tikzpicture}\shade[left color=red,right color=blue] (0,0) rectangle (3,2);\fill[green,opacity=.3] (1,1) circle (1);\end{tikzpicture}"##,
    ),
    (
        r##"graphics-options"##,
        r##"\usepackage{graphicx}"##,
        r##"\includegraphics[width=4cm,angle=30,trim=10 20 30 40,clip]{example-image-a}\reflectbox{Reflection}\resizebox{5cm}{!}{Resize}"##,
    ),
    (
        r##"natbib"##,
        r##"\usepackage{natbib}"##,
        r##"Text \citet{doe}.\bibliographystyle{plainnat}\bibliography{references}"##,
    ),
    (
        r##"pdfpages"##,
        r##"\usepackage{pdfpages}"##,
        r##"\includepdf[pages=1]{example-image-a}"##,
    ),
    (
        r##"font-packages"##,
        r##"\usepackage{newtxtext,newtxmath}"##,
        r##"\textbf{Bold text} $\left(\int_0^1 x^2dx\right)$"##,
    ),
    (
        r##"chemistry"##,
        r##"\usepackage{mhchem,chemformula}"##,
        r##"\ce{H2O + CO2 -> H2CO3} \ch{H2SO4}"##,
    ),
    (
        r##"circuits"##,
        r##"\usepackage{circuitikz,tikz-cd}"##,
        r##"\begin{circuitikz}\draw (0,0) to[R=$R$] (2,0);\end{circuitikz}\begin{tikzcd}A\arrow[r]&B\end{tikzcd}"##,
    ),
    (
        r##"engine-detection"##,
        r##"\usepackage{ifpdf,iftex}"##,
        r##"\ifpdf PDF mode.\else DVI mode.\fi \ifXeTeX XeTeX mode.\fi"##,
    ),
    (
        r##"primitives"##,
        r##""##,
        r##"\typeout{PITEX-EXPANDED=\expanded{abc}}\typeout{PITEX-COMPARE=\strcmp{same}{same}}\typeout{PITEX-UCHAR=\Uchar65}\typeout{PITEX-FILESIZE=\filesize{chapter.tex}}\typeout{PITEX-MD5=\mdfivesum{abc}} Primitive text."##,
    ),
];
