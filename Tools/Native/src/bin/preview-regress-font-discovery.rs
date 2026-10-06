//! Fontconfig or CoreText font discovery compared with XeLaTeX through two
//! unsaved native preview updates.
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
fn records(log: &str) -> Vec<String> {
    log.lines()
        .filter(|line| line.starts_with("DISCOVERY-"))
        .map(str::to_owned)
        .collect()
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temp = TempDir::new("pitex-font-discovery-")?;
    let work = fs::canonicalize(temp.path())?;
    #[cfg(target_os = "macos")]
    let cases = [
        ("coretext-family", "Helvetica", ""),
        ("coretext-full-name", "Helvetica Bold", ""),
        ("coretext-postscript", "Helvetica-BoldOblique", ""),
        ("coretext-serif", "Times-Roman", ""),
        ("type1-afm-file", "[lmr10.pfb]", ""),
    ];
    #[cfg(not(target_os = "macos"))]
    let cases = [
        ("family", "DejaVu Sans", ""),
        ("full-name", "DejaVu Sans Bold Oblique", ""),
        ("preferred-family-and-style", "Lato-Light", ""),
        ("preferred-subfamily", "Lato Light", ""),
        ("family-style-composite", "DejaVu Sans Oblique", ""),
        (
            "postscript-global-cache",
            "DejaVuSans-BoldOblique",
            r#"\font\second="DejaVuSerif-BoldItalic" at12pt\second\setbox0=\hbox{office affinity AV}\typeout{DISCOVERY-SECOND=\fontname\second,\the\wd0,\the\ht0,\the\dp0}\box0"#,
        ),
        ("collection-face-index", "AR PL UMing TW MBE", ""),
        ("non-sfnt-type1", "LMRoman10-Regular", ""),
        ("type1-afm-file", "[lmr10.pfb]", ""),
    ];
    for (case, name, extra) in cases {
        if std::env::var("PITEX_DISCOVERY_CASE").is_ok_and(|filter| !case.contains(&filter)) {
            continue;
        }
        let root = work.join(case);
        fs::create_dir(&root)?;
        let main = root.join("main.tex");
        let source = format!(
            r#"\documentclass{{article}}\pagestyle{{empty}}
\font\probe="{name}" at12pt
\begin{{document}}
DiscoveryMarkerOriginal\par
{{\probe\XeTeXuseglyphmetrics=0\setbox0=\hbox{{office affinity AV}}
\typeout{{DISCOVERY-NAME=\fontname\probe}}
\typeout{{DISCOVERY-GLYPHS=\the\XeTeXcountglyphs\probe}}
\typeout{{DISCOVERY-METRICS=\the\wd0,\the\ht0,\the\dp0}}\box0}}
{extra}
\end{{document}}
"#
        );
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
                format!("oracle {case}: {}", String::from_utf8_lossy(&result.stdout)).into(),
            );
        }
        let expected = records(&fs::read_to_string(reference.join("main.log"))?);
        if expected.len() < 3 {
            return Err("font discovery oracle records missing".into());
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
            let marker = format!("DiscoveryMarker{generation}");
            session.send(&json!({"op":"update", "generation":generation,
                "files":[{"path":main, "text":source.replace("DiscoveryMarkerOriginal", &marker)}], "closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 {
                return Err(format!("native {case}: {event}").into());
            }
            let log = fs::read_to_string(event["log"].as_str().ok_or("missing log")?)?;
            if records(&log) != expected {
                return Err(format!("{case}: font names/glyph counts/metrics differ\nexpected {expected:?}\nactual {:?}", records(&log)).into());
            }
            let pdf = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
            let text = output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
            if !String::from_utf8(text.stdout)?.contains(&marker) {
                return Err("unsaved discovery marker missing".into());
            }
            if fs::read_to_string(&main)? != source {
                return Err("saved font discovery source changed".into());
            }
        }
        println!("PASS: {case}, exact XeLaTeX font names/glyphs/metrics and two native updates");
    }
    Ok(())
}
fn main() {
    pitex_native_tools::exit_on_error(run());
}
