//! Native PDF object streams, article threads, and interactive document tests.
//! PITEX_BIN=<helper directory> cargo run --manifest-path Tools/Native/Cargo.toml
//! --bin preview-regress-pdf-structures
//! External pdfLaTeX is used only to generate comparison documents.
use pitex_native_tools::{
    preview_regression::{self, output_timeout, Session},
    TempDir,
};
use serde_json::json;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const TIMEOUT: Duration = Duration::from_secs(60);
fn output(command: &mut Command) -> Result<Vec<u8>> {
    let out = output_timeout(command, TIMEOUT)?;
    if !out.status.success() {
        return Err(format!("{}: {}", out.status, String::from_utf8_lossy(&out.stderr)).into());
    }
    Ok(out.stdout)
}
fn show(pdf: &Path, key: &str) -> Result<String> {
    Ok(String::from_utf8(output(
        Command::new("mutool").arg("show").arg(pdf).arg(key),
    )?)?)
}
fn text(pdf: &Path) -> Result<String> {
    Ok(String::from_utf8(output(
        Command::new("pdftotext").arg(pdf).arg("-"),
    )?)?)
}
fn check(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn references(value: &str) -> usize {
    value.split_whitespace().filter(|s| *s == "R").count()
}
fn numbers(value: &str) -> Vec<f64> {
    let value = if let (Some(start), Some(end)) = (value.find('['), value.rfind(']')) {
        &value[start + 1..end]
    } else {
        value
    };
    value
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect()
}
fn run_case(
    bin: &Path,
    work: &Path,
    name: &str,
    preamble: &str,
    body: &str,
) -> Result<Vec<PathBuf>> {
    let root = work.join(name);
    fs::create_dir(&root)?;
    let out = root.join("preview");
    fs::create_dir(&out)?;
    let main = root.join("main.tex");
    let source=format!("\\documentclass{{article}}\n\\usepackage[paperwidth=12cm,paperheight=12cm,margin=12mm]{{geometry}}\n\\pagestyle{{empty}}\n{preamble}\n\\begin{{document}}\nStructureMarker0.\\par\n{body}\n\\end{{document}}\n");
    fs::write(&main, &source)?;
    let reference = root.join("reference");
    fs::create_dir(&reference)?;
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
        let edited = source.replace("StructureMarker0", &format!("StructureMarker{generation}"));
        fs::write(reference.join("main.tex"), &edited)?;
        for _ in 0..2 {
            let compiled = output_timeout(
                Command::new("pdflatex")
                    .args([
                        "-no-shell-escape",
                        "-interaction=nonstopmode",
                        "-halt-on-error",
                        "main.tex",
                    ])
                    .current_dir(&reference),
                TIMEOUT,
            )?;
            check(
                compiled.status.success(),
                &format!(
                    "{name}: invalid reference: {}",
                    String::from_utf8_lossy(&compiled.stdout)
                ),
            )?;
        }
        session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":edited}],"closed":[]}))?;
        let event = session.wait_published(generation, TIMEOUT)?;
        check(event["errors"] == 0, &format!("{name}: {event}"))?;
        let path = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
        let saved = root.join(format!("generation{generation}.pdf"));
        fs::copy(path, &saved)?;
        validate(name, &saved, &reference.join("main.pdf"))?;
        pdfs.push(saved);
    }
    // Thread bead rectangles are the only values compared with pdfLaTeX.
    let reference = (name == "threads").then_some("pdftex");
    preview_regression::pass(
        name,
        reference,
        &[&source],
        "PDF structures and two unsaved generations",
    )?;
    Ok(pdfs)
}
fn validate(name: &str, pdf: &Path, reference: &Path) -> Result<()> {
    check(
        text(pdf)?.contains("StructureMarker"),
        "text lost while writing PDF structures",
    )?;
    let all = show(pdf, "grep")?;
    match name {
        "level0" | "level1" | "level2" | "level3" => {
            let level = name[5..].parse::<u8>()?;
            check(
                all.contains("/Type/ObjStm") == (level > 0),
                "object compression setting ignored",
            )?;
            check(
                show(pdf, "trailer")?.contains("/Type /XRef") == (level > 0),
                "cross-reference stream missing",
            )?;
            let xref = show(pdf, "xref")?;
            let info = xref
                .lines()
                .find(|line| line.starts_with("00003:"))
                .ok_or("Info object missing from xref")?;
            check(
                info.trim_end()
                    .ends_with(if level == 3 { "o" } else { "n" }),
                "Info compression level policy changed",
            )?;
            check(
                all.matches("/Subtype/Link").count() >= 150,
                "compressed links disappeared",
            )?;
            check(
                text(pdf)?
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .contains("Link 150"),
                "compressed document text incomplete",
            )?;
        }
        "version14" => check(
            !all.contains("/Type/ObjStm"),
            "object streams must be disabled for requested PDF1.4",
        )?,
        "threads" => {
            check(
                references(&show(pdf, "trailer/Root/Threads")?) == 2,
                "article threads missing",
            )?;
            check(
                show(pdf, "trailer/Root/Threads/1/F")?
                    == show(pdf, "trailer/Root/Threads/1/F/N/N")?,
                "two-bead thread cycle broken",
            )?;
            check(
                show(pdf, "trailer/Root/Threads/1/F/V")?
                    == show(pdf, "trailer/Root/Threads/1/F/N")?,
                "backward bead link broken",
            )?;
            check(
                references(&show(pdf, "pages/1/B")?) == 1
                    && references(&show(pdf, "pages/2/B")?) == 2,
                "page bead arrays missing",
            )?;
            let r = numbers(&show(pdf, "trailer/Root/Threads/1/F/R")?);
            let expected = numbers(&show(reference, "trailer/Root/Threads/1/F/R")?);
            check(
                r.len() == 4 && expected.len() == 4,
                "bead rectangle missing",
            )?;
            check(
                ((r[2] - r[0]) - (expected[2] - expected[0])).abs() < 0.05
                    && ((r[3] - r[1]) - (expected[3] - expected[1])).abs() < 0.05,
                "thread dimensions/margins differ from pdfLaTeX",
            )?;
        }
        "active-thread" => {
            check(
                references(&show(pdf, "trailer/Root/Threads")?) == 1,
                "active thread missing",
            )?;
            check(
                references(&show(pdf, "pages/1/B")?) == 1,
                "active thread page bead missing",
            )?;
            check(
                show(pdf, "trailer/Root/Threads/1/F/R")?.contains('['),
                "active thread region missing",
            )?;
        }
        "interactive" => {
            check(
                show(pdf, "trailer/Root/LastLink/Subtype")?.contains("Link"),
                "link identifier did not resolve to its annotation",
            )?;
            check(
                show(pdf, "trailer/Root/LastAnnotation/Contents")?
                    .contains("Identified annotation"),
                "annotation identifier lost",
            )?;
            let r = numbers(&show(pdf, "trailer/Root/LastAnnotation/Rect")?);
            check(
                r.len() == 4 && (r[2] - r[0] - 20. * 72. / 72.27).abs() < 0.05,
                "annotation dimensions changed",
            )?;
        }
        _ => return Err("unknown PDF structure fixture".into()),
    }
    Ok(())
}
fn run() -> Result<()> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN is required")?)?;
    let temporary = TempDir::new("pitex-pdf-structures-")?;
    let work = fs::canonicalize(temporary.path())?;
    let mut plain = None;
    let mut packed = None;
    for level in 0..=3 {
        let name = format!("level{level}");
        let preamble = format!("\\usepackage{{hyperref}}\n\\pdfobjcompresslevel={level}");
        let body = r"\newcount\n\n=0\loop\advance\n by1 \href{https://example.org}{Link \the\n}. \ifnum\n<150\repeat";
        let pdfs = run_case(&bin, &work, &name, &preamble, body)?;
        if level == 0 {
            plain = Some(pdfs[0].clone());
        }
        if level == 3 {
            packed = Some(pdfs[0].clone());
        }
    }
    let plain = plain.unwrap();
    let packed = packed.unwrap();
    check(
        fs::metadata(&packed)?.len() * 100 < fs::metadata(&plain)?.len() * 85,
        "object streams did not materially reduce file size",
    )?;
    for (pdf, name) in [(&plain, "plain"), (&packed, "packed")] {
        output(
            Command::new("pdftoppm")
                .args(["-f", "1", "-singlefile", "-r", "72"])
                .arg(pdf)
                .arg(work.join(name)),
        )?;
    }
    check(
        fs::read(work.join("plain.ppm"))? == fs::read(work.join("packed.ppm"))?,
        "object compression changed rendered pixels",
    )?;
    println!("PASS: object streams reduce size and preserve exact rendered pixels");
    run_case(
        &bin,
        &work,
        "version14",
        r"\pdfminorversion=4\pdfobjcompresslevel=3",
        "Version compatibility.",
    )?;
    run_case(
        &bin,
        &work,
        "threads",
        "",
        r"\pdfthreadmargin=3pt\pdfthread width100pt height20pt depth2pt attr{/I << /Title (Article one) >>} num1 First article.\newpage\pdfthread width100pt height20pt depth2pt num1 Second article.\pdfthread width100pt height20pt depth0pt name{Other}Other article.",
    )?;
    run_case(
        &bin,
        &work,
        "active-thread",
        "",
        r"\vbox{\pdfstartthread name{Active}First paragraph in article.\par More article text.\par\pdfendthread}Outside article.",
    )?;
    run_case(
        &bin,
        &work,
        "interactive",
        "",
        r"\leavevmode\pdflinkmargin=2pt\pdfstartlink user{/Subtype /Link /A << /S /URI /URI (https://example.org) >>}Link text.\pdfendlink\edef\lastlink{\the\pdflastlink}\pdfannot width20pt height10pt depth0pt{/Subtype /Text /Contents (Identified annotation)}\pdfcatalog{/LastLink \lastlink\space0 R /LastAnnotation \the\pdflastannot\space0 R}",
    )?;
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
