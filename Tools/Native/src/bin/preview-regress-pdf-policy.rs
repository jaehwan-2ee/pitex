//! Native PDF destination, duplicate-warning, and inclusion policy regressions.
//! PITEX_BIN=<helper directory> cargo run --manifest-path Tools/Native/Cargo.toml
//! --bin preview-regress-pdf-policy
//! pdfLaTeX is only the reference/fixture generator; preview uses no external converter.
use pitex_native_tools::{
    preview_regression::{self, output_timeout, Session},
    TempDir,
};
use serde_json::{json, Value};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const TIMEOUT: Duration = Duration::from_secs(60);
fn check(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn command(command: &mut Command) -> Result<Vec<u8>> {
    let output = output_timeout(command, TIMEOUT)?;
    check(
        output.status.success(),
        &String::from_utf8_lossy(&output.stderr),
    )?;
    Ok(output.stdout)
}
fn show(pdf: &Path, key: &str) -> Result<String> {
    Ok(String::from_utf8(command(
        Command::new("mutool").arg("show").arg(pdf).arg(key),
    )?)?)
}
fn compile(root: &Path) -> Result<(bool, String)> {
    let output = output_timeout(
        Command::new("pdflatex")
            .args([
                "-no-shell-escape",
                "-interaction=nonstopmode",
                "-halt-on-error",
                "main.tex",
            ])
            .current_dir(root),
        TIMEOUT,
    )?;
    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}
fn create_assets(root: &Path) -> Result<()> {
    let source = r"\documentclass{article}\pdfminorversion=7\pdfobjcompresslevel=0\usepackage[paperwidth=4cm,paperheight=2cm,margin=3mm]{geometry}\pagestyle{empty}\begin{document}Included figure.\end{document}";
    let build = root.join("asset-build");
    fs::create_dir(&build)?;
    fs::write(build.join("main.tex"), source)?;
    check(compile(&build)?.0, "figure oracle failed")?;
    fs::copy(build.join("main.pdf"), root.join("figure.pdf"))?;
    let grouped=source.replace(r"\pagestyle{empty}",r"\pagestyle{empty}\pdfpageattr{/Group << /Type /Group /S /Transparency /CS /DeviceRGB /I true >>}");
    fs::write(build.join("main.tex"), &grouped)?;
    check(compile(&build)?.0, "grouped figure oracle failed")?;
    fs::copy(build.join("main.pdf"), root.join("group-a.pdf"))?;
    fs::write(
        build.join("main.tex"),
        grouped.replace("Included figure", "Second grouped figure"),
    )?;
    check(compile(&build)?.0, "second grouped oracle failed")?;
    fs::copy(build.join("main.pdf"), root.join("group-b.pdf"))?;
    Ok(())
}
fn event(session: &mut Session, generation: i32) -> Result<Value> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("preview timeout")?;
        if let Some(event) = session.next_event(remaining)? {
            if event["event"] == "error"
                || event["event"] == "failed"
                || (event["event"] == "published"
                    && event["complete"] == true
                    && event["generation"] == generation)
            {
                return Ok(event);
            }
        }
    }
}
fn numbers(value: &str) -> Vec<f64> {
    value
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect()
}
fn destination_rect(pdf: &Path) -> Result<Vec<f64>> {
    let names = show(pdf, "trailer/Root/Names/Dests/Names")?;
    let tokens = names.split_whitespace().collect::<Vec<_>>();
    let at = tokens
        .iter()
        .position(|s| *s == "(rect)")
        .ok_or("FitR destination missing")?;
    // Native stores arrays directly; pdfLaTeX stores a destination dictionary.
    let value = if tokens.get(at + 1) == Some(&"[") {
        names
    } else {
        show(
            pdf,
            &format!("{}/D", tokens.get(at + 1).ok_or("destination ref missing")?),
        )?
    };
    let position = value.find("/FitR").ok_or("FitR view missing")?;
    let tail = &value[position + 5..];
    let tail = &tail[..tail.find(']').ok_or("FitR array malformed")?];
    Ok(numbers(tail))
}
fn run_case(
    bin: &Path,
    work: &Path,
    name: &str,
    preamble: &str,
    body: &str,
    error: bool,
    warning: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let root = work.join(name);
    fs::create_dir(&root)?;
    for file in ["figure.pdf", "group-a.pdf", "group-b.pdf"] {
        fs::copy(work.join(file), root.join(file))?;
    }
    let source=format!("\\documentclass{{article}}\n\\usepackage[paperwidth=12cm,paperheight=12cm,margin=12mm]{{geometry}}\n\\usepackage{{graphicx}}\n\\pagestyle{{empty}}\n{preamble}\n\\begin{{document}}\nPolicyMarker0.\\par\n{body}\n\\end{{document}}\n");
    let main = root.join("main.tex");
    fs::write(&main, &source)?;
    let reference = root.join("reference");
    fs::create_dir(&reference)?;
    for file in ["figure.pdf", "group-a.pdf", "group-b.pdf"] {
        fs::copy(work.join(file), reference.join(file))?;
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
    let mut pdfs = Vec::new();
    for generation in [1, 2] {
        let edited = source.replace("PolicyMarker0", &format!("PolicyMarker{generation}"));
        fs::write(reference.join("main.tex"), &edited)?;
        let (reference_ok, reference_log) = compile(&reference)?;
        check(
            reference_ok != error,
            &format!("{name}: reference status changed: {reference_log}"),
        )?;
        session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":edited}],"closed":[]}))?;
        let event = event(&mut session, generation)?;
        if error {
            check(
                matches!(event["event"].as_str(), Some("error" | "failed")),
                "strict PDF inclusion must reject publication",
            )?;
            continue;
        }
        check(
            event["event"] == "published" && event["errors"] == 0,
            &format!("{name}: {event}"),
        )?;
        let warnings = event["warnings"].as_str().unwrap_or("");
        if name.starts_with("version-") {
            let output_log = fs::read_to_string(event["log"].as_str().ok_or("missing log")?)?;
            let expected = reference_log
                .lines()
                .find(|line| line.starts_with("POLICY-AFTER="))
                .ok_or("reference did not report inclusion parameter state")?;
            check(
                output_log.contains(expected),
                "deprecated inclusion parameter consumption differs from pdfLaTeX",
            )?;
        }

        if let Some(expected) = warning {
            check(
                warnings.contains(expected),
                &format!("{name}: missing warning {expected}: {warnings}"),
            )?;
        } else {
            check(
                ![
                    "duplicate destination",
                    "duplicate font map",
                    "multiple page groups",
                    "newer than requested",
                ]
                .iter()
                .any(|message| warnings.contains(message)),
                &format!("{name}: policy warning not suppressed: {warnings}"),
            )?;
        }
        let pdf = root.join(format!("generation{generation}.pdf"));
        fs::copy(event["pdf"].as_str().ok_or("missing PDF")?, &pdf)?;
        if name.starts_with("destination-") {
            let actual = destination_rect(&pdf)?;
            let expected = destination_rect(&reference.join("main.pdf"))?;
            check(
                actual.len() == 4 && expected.len() == 4,
                "destination rectangle malformed",
            )?;
            check(
                (actual[2] - actual[0] - expected[2] + expected[0]).abs() < 0.05
                    && (actual[3] - actual[1] - expected[3] + expected[1]).abs() < 0.05,
                "destination fit rectangle/margin differs from pdfLaTeX",
            )?;
        }
        if name.starts_with("duplicate-dest") {
            let names = show(&pdf, "trailer/Root/Names/Dests/Names")?;
            check(
                names.matches("(same)").count() == 1,
                "duplicate destination not deduplicated",
            )?;
        }
        if name == "font-copy" || name == "font-local" {
            let all = show(&pdf, "grep")?;
            check(all.contains("/Subtype/Type1"), "included font missing")?;
            check(
                all.contains("/BaseFont/CMR10") == (name == "font-local"),
                "font inclusion copy policy did not change embedded-vs-local program",
            )?;
            command(
                Command::new("pdftoppm")
                    .args(["-f", "1", "-singlefile", "-r", "72"])
                    .arg(&pdf)
                    .arg(root.join(format!("render{generation}"))),
            )?;
        }
        pdfs.push(pdf);
    }
    // Destination rectangles and inclusion-parameter states are compared with
    // pdfLaTeX; the other cases check native warnings and font programs.
    let reference =
        (name.starts_with("destination-") || name.starts_with("version-")).then_some("pdftex");
    preview_regression::pass(
        name,
        reference,
        &[&source],
        "reference semantics and two unsaved generations",
    )?;
    Ok(pdfs)
}
fn run() -> Result<()> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN is required")?)?;
    let temporary = TempDir::new("pitex-pdf-policy-")?;
    let work = fs::canonicalize(temporary.path())?;
    create_assets(&work)?;
    for margin in [0, 2] {
        run_case(
            &bin,
            &work,
            &format!("destination-{margin}"),
            &format!("\\pdfdestmargin={margin}pt"),
            r"\leavevmode\pdfdest name{rect} fitr width40pt height20pt depth5pt Rectangle destination.",
            false,
            None,
        )?;
    }
    for suppress in [0, 1] {
        run_case(
            &bin,
            &work,
            &format!("duplicate-dest-{suppress}"),
            &format!("\\pdfsuppresswarningdupdest={suppress}"),
            r"\leavevmode\pdfdest name{same} xyz First.\pdfdest name{same} xyz Second.",
            false,
            (suppress == 0).then_some("duplicate destination"),
        )?;
        run_case(&bin,&work,&format!("duplicate-map-{suppress}"),&format!("\\pdfsuppresswarningdupmap={suppress}\\pdfmapline{{+cmr10 CMR10 <cmr10.pfb}}\\pdfmapline{{+cmr10 CMR10 <cmr10.pfb}}"),"Font map policy.",false,(suppress==0).then_some("duplicate font map"))?;
        run_case(
            &bin,
            &work,
            &format!("page-group-{suppress}"),
            &format!("\\pdfminorversion=7\\pdfsuppresswarningpagegroup={suppress}"),
            r"\includegraphics[width=3cm]{group-a.pdf}\includegraphics[width=3cm]{group-b.pdf}",
            false,
            (suppress == 0).then_some("multiple page groups"),
        )?;
    }
    for (new, legacy) in [(0, 0), (-1, 0), (1, 0), (1, -1), (-1, 1)] {
        let effective = if legacy != 0 { legacy } else { new };
        let warning = (effective == 0).then_some("newer than requested");
        run_case(&bin,&work,&format!("version-{new}-{legacy}"),&format!("\\pdfminorversion=4\\pdfinclusionerrorlevel={new}\\pdfoptionpdfinclusionerrorlevel={legacy}"),r"\pdfximage width3cm{figure.pdf}\typeout{POLICY-AFTER=\the\pdfinclusionerrorlevel,\the\pdfoptionpdfinclusionerrorlevel}\pdfrefximage\pdflastximage",effective>0,warning)?;
    }
    let copied = run_case(
        &bin,
        &work,
        "font-copy",
        r"\pdfinclusioncopyfonts=1",
        r"\includegraphics[width=4cm]{figure.pdf}",
        false,
        None,
    )?;
    let local = run_case(
        &bin,
        &work,
        "font-local",
        r"\pdfinclusioncopyfonts=0",
        r"\includegraphics[width=4cm]{figure.pdf}",
        false,
        None,
    )?;
    for generation in 1..=2 {
        check(
            fs::read(
                work.join("font-copy")
                    .join(format!("render{generation}.ppm")),
            )? == fs::read(
                work.join("font-local")
                    .join(format!("render{generation}.ppm")),
            )?,
            "local font substitution changed pixels with matching installed font",
        )?;
    }
    check(
        fs::read(&copied[0])? != fs::read(&local[0])?,
        "font copy policy produced identical PDF programs",
    )?;
    println!("PASS: matching local font substitution preserves exact pixels");
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
