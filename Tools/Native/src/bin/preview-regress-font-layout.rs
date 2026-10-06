//! Font table queries, metric caches, vertical shaping and Graphite feature /
//! line-breaking behavior against the installed XeLaTeX test oracle.
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
        .filter(|line| line.starts_with("LAYOUT-"))
        .map(str::to_string)
        .collect()
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temp = TempDir::new("pitex-font-layout-")?;
    let work = fs::canonicalize(temp.path())?;
    for (name, setup, body) in [
        (
            "opentype-tables",
            r"\newfontfamily\testfont{lmroman10-regular.otf}",
            r#"{\testfont\count0=\XeTeXOTscripttag\font0\relax\typeout{LAYOUT-SCRIPTS=\the\XeTeXOTcountscripts\font,\the\count0}\typeout{LAYOUT-LANGUAGES=\the\XeTeXOTcountlanguages\font\count0,\the\XeTeXOTlanguagetag\font\count0 0}\typeout{LAYOUT-FEATURES=\the\XeTeXOTcountfeatures\font\count0 0,\the\XeTeXOTfeaturetag\font\count0 0 0}\setbox0=\hbox{office affinity AV}\typeout{LAYOUT-WIDTH=\the\wd0,\the\ht0,\the\dp0}\box0}"#,
        ),
        (
            "metric-cache",
            r"\newfontfamily\testfont[FakeStretch=1.25,FakeSlant=.2]{lmroman10-regular.otf}",
            r#"{\testfont\XeTeXuseglyphmetrics=1\count0=\XeTeXcharglyph`A\relax\lpcode\font\count0=300\rpcode\font\count0=400\typeout{LAYOUT-PROTRUSION=\the\lpcode\font\count0,\the\rpcode\font\count0}\dimen0=\XeTeXglyphbounds1 \count0\dimen1=\XeTeXglyphbounds2 \count0\dimen2=\XeTeXglyphbounds3 \count0\dimen3=\XeTeXglyphbounds4 \count0\typeout{LAYOUT-BOUNDS=\the\dimen0,\the\dimen1,\the\dimen2,\the\dimen3}\setbox0=\hbox{AAAA office}\typeout{LAYOUT-CACHED=\the\wd0,\the\ht0,\the\dp0}\box0}"#,
        ),
        (
            "vertical",
            r"\newfontfamily\testfont[RawFeature={vertical}]{lmroman10-regular.otf}",
            r#"{\testfont\setbox0=\hbox{ABC xyz}\typeout{LAYOUT-VERTICAL=\the\wd0,\the\ht0,\the\dp0}\box0}"#,
        ),
        (
            "graphite",
            r"\newfontfamily\testfont[Renderer=Graphite]{Padauk}",
            r#"{\testfont\setbox0=\hbox{မြန်မာစာ abc}\count0=\XeTeXfeaturecode\font0\relax\typeout{LAYOUT-GR-FEATURES=\the\XeTeXcountfeatures\font,\the\count0}\typeout{LAYOUT-GR-SELECTORS=\the\XeTeXcountselectors\font\count0,\the\XeTeXselectorcode\font\count0 0}\typeout{LAYOUT-GR-WIDTH=\the\wd0,\the\ht0,\the\dp0}\box0\par\XeTeXlinebreaklocale="G"\setbox1=\vbox{\hsize=80pt\noindent မြန်မာစာ မြန်မာစာ မြန်မာစာ မြန်မာစာ မြန်မာစာ မြန်မာစာ\par}\typeout{LAYOUT-GR-BREAKS=\the\wd1,\the\ht1,\the\dp1}\box1}"#,
        ),
    ] {
        if let Ok(filter) = std::env::var("PITEX_LAYOUT_CASE") {
            if !name.contains(&filter) {
                continue;
            }
        }
        let root = work.join(name);
        fs::create_dir(&root)?;
        let main = root.join("main.tex");
        let source=format!("\\documentclass{{article}}\\pagestyle{{empty}}\n\\usepackage{{fontspec}}\\setmainfont{{lmroman10-regular.otf}}\n{setup}\n\\begin{{document}}\nLayoutMarkerOriginal\\par\n{body}\n\\end{{document}}\n");
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
            return Err("layout oracle metrics missing".into());
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
            let marker = format!("LayoutMarker{generation}");
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":source.replace("LayoutMarkerOriginal",&marker)}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 {
                return Err(format!("native {name}: {event}").into());
            }
            let log = fs::read_to_string(event["log"].as_str().ok_or("missing log")?)?;
            if metrics(&log) != expected {
                return Err(format!(
                    "{name}: layout metrics differ\nexpected {expected:?}\nactual {:?}",
                    metrics(&log)
                )
                .into());
            }
            let pdf = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
            let text = output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
            if !String::from_utf8(text.stdout)?.contains(&marker) {
                return Err("unsaved layout marker missing".into());
            }
            if fs::read_to_string(&main)? != source {
                return Err("saved layout source changed".into());
            }
        }
        preview_regression::pass(
            name,
            Some("xetex"),
            &[&source],
            "exact XeLaTeX layout metrics and two native updates",
        )?;
    }
    Ok(())
}
fn main() {
    pitex_native_tools::exit_on_error(run());
}
