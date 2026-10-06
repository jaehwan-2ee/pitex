//! Unicode MATH constants, kerns, variants and assemblies through real native
//! sessions, compared with the installed XeLaTeX test oracle.
use pitex_native_tools::{
    preview_regression::{self, output_timeout, Session},
    TempDir,
};
use serde_json::json;
use std::{
    error::Error,
    fs,
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
const TIMEOUT: Duration = Duration::from_secs(60);
fn metrics(log: &str) -> Vec<String> {
    log.lines()
        .filter(|line| line.starts_with("OT-MATH-"))
        .map(str::to_string)
        .collect()
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temp = TempDir::new("pitex-opentype-math-")?;
    let work = fs::canonicalize(temp.path())?;
    for (name, setup, body) in [
        (
            "scripts-kerns",
            "",
            r"\setbox0=\hbox{$f^j_{\!x}+\symit{f}^{\symit{j}}_{\symit{f}}+\sum_{k=0}^{n^2}k$}\typeout{OT-MATH-SCRIPTS=\the\wd0,\the\ht0,\the\dp0}\noindent\box0",
        ),
        (
            "variants-assemblies",
            "",
            r"\setbox0=\hbox{$\displaystyle\left(\frac{\sum_{k=0}^{n^2} k^2}{\sqrt[3]{\frac{a+b}{c+d}}}\right)\quad\sqrt[7]{\dfrac{a^j_f+b}{c}}$}\typeout{OT-MATH-VERTICAL=\the\wd0,\the\ht0,\the\dp0}\noindent\box0\par\setbox1=\hbox{$\overbrace{a+b+c+d+e+f+g}^{n}\;\widehat{abcdefghij}\;\overrightarrow{abcdefghij}$}\typeout{OT-MATH-HORIZONTAL=\the\wd1,\the\ht1,\the\dp1}\box1",
        ),
        (
            "scaled-math",
            r"\AtBeginDocument{\fontsize{17.3125pt}{23pt}\selectfont}",
            r"\setbox0=\hbox{$\displaystyle\frac{x^j_f+y}{\sqrt[3]{z}}+\left\langle\dfrac{a}{b}\right\rangle$}\typeout{OT-MATH-SCALED=\the\wd0,\the\ht0,\the\dp0}\noindent\box0",
        ),
    ] {
        let root = work.join(name);
        fs::create_dir(&root)?;
        let main = root.join("main.tex");
        let source=format!("\\documentclass{{article}}\\pagestyle{{empty}}\n\\usepackage{{fontspec,unicode-math}}\\setmainfont{{lmroman10-regular.otf}}\\setmathfont{{latinmodern-math.otf}}\n{setup}\n\\begin{{document}}\nOTMathMarkerOriginal\\par\n{body}\n\\end{{document}}\n");
        fs::write(&main, &source)?;
        let reference = root.join("reference");
        fs::create_dir(&reference)?;
        let result = output_timeout(
            Command::new("xelatex")
                .args([
                    "-interaction=nonstopmode",
                    "-halt-on-error",
                    "-no-shell-escape",
                ])
                .arg(format!("-output-directory={}", reference.display()))
                .arg("main.tex")
                .current_dir(&root),
            TIMEOUT,
        )?;
        if !result.status.success() {
            return Err(
                format!("oracle {name}: {}", String::from_utf8_lossy(&result.stdout)).into(),
            );
        }
        let expected = metrics(&fs::read_to_string(reference.join("main.log"))?);
        if expected.is_empty() {
            return Err("math oracle metrics missing".into());
        }
        let out = root.join("preview");
        fs::create_dir(&out)?;
        let mut session = Session::spawn(
            Command::new(bin.join("pitex-preview"))
                .arg("--root")
                .arg(&root)
                .args(["--main", "main.tex", "--out"])
                .arg(&out)
                .arg("--cache")
                .arg(work.join("cache"))
                .stderr(Stdio::null()),
            &out,
        )?;
        for generation in [1u64, 2] {
            let marker = format!("OTMathMarker{generation}");
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":source.replace("OTMathMarkerOriginal",&marker)}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 {
                return Err(format!("native {name}: {event}").into());
            }
            let log = fs::read_to_string(event["log"].as_str().ok_or("missing log")?)?;
            if metrics(&log) != expected {
                return Err(format!(
                    "{name}: math metrics differ\nexpected {expected:?}\nactual {:?}",
                    metrics(&log)
                )
                .into());
            }
            let pdf = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
            let text = output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
            if !String::from_utf8(text.stdout)?.contains(&marker) {
                return Err("unsaved math marker missing".into());
            }
            if fs::read_to_string(&main)? != source {
                return Err("saved math source changed".into());
            }
        }
        println!("PASS: {name}, exact XeLaTeX math metrics and two native updates");
    }
    Ok(())
}
fn main() {
    pitex_native_tools::exit_on_error(run());
}
