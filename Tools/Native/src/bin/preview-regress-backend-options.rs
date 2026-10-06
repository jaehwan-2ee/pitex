//! Verify native output policies through real unsaved preview generations.
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
    time::{Duration, Instant},
};
const TIMEOUT: Duration = Duration::from_secs(60);
fn crc(bytes: &[u8]) -> u32 {
    let mut value = !0u32;
    for byte in bytes {
        value ^= *byte as u32;
        for _ in 0..8 {
            value = (value >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(value & 1));
        }
    }
    !value
}
fn png(bits: u8) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, name: &[u8; 4], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        out.extend(name);
        out.extend(data);
        let mut check = name.to_vec();
        check.extend(data);
        out.extend(crc(&check).to_be_bytes());
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend(5u32.to_be_bytes());
    header.extend(1u32.to_be_bytes());
    header.extend([bits, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    let mut row = vec![0];
    if bits == 16 {
        for value in [0u16, 10000, 32768, 50000, 65535] {
            row.extend(value.to_be_bytes());
        }
    } else {
        row.extend([0, 64, 128, 192, 255]);
    }
    // A zlib stream with one stored DEFLATE block keeps fixtures dependency free.
    let mut stream = vec![0x78, 0x01, 0x01];
    let length = row.len() as u16;
    stream.extend(length.to_le_bytes());
    stream.extend((!length).to_le_bytes());
    stream.extend(&row);
    let (mut a, mut b) = (1u32, 0u32);
    for byte in row {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    stream.extend(((b << 16) | a).to_be_bytes());
    chunk(&mut out, b"IDAT", &stream);
    chunk(&mut out, b"IEND", &[]);
    out
}
fn show(pdf: &Path, selector: &str, raw: bool) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut cmd = Command::new("mutool");
    cmd.arg("show");
    if raw {
        cmd.arg("-b");
    }
    cmd.arg(pdf).arg(selector);
    let output = output_timeout(&mut cmd, TIMEOUT)?;
    if !output.status.success() {
        return Err("mutool failed".into());
    }
    Ok(output.stdout)
}
fn string(pdf: &Path, selector: &str) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(show(pdf, selector, false)?)?)
}
fn image(pdf: &Path) -> Result<(String, Vec<u8>), Box<dyn Error>> {
    let objects = string(pdf, "grep")?;
    let line = objects
        .lines()
        .find(|line| line.contains("/Subtype/Image") && line.contains("/Width 5"))
        .ok_or("5-pixel image missing")?;
    let number = line
        .split_whitespace()
        .next()
        .ok_or("image object number missing")?;
    Ok((line.into(), show(pdf, number, true)?))
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temp = TempDir::new("pitex-backend-options-")?;
    let work = fs::canonicalize(temp.path())?;
    let cases = [
        (
            "highcolor",
            16,
            r"\pdfimagehicolor=1",
            r"\pdfimagehicolor=0",
        ),
        (
            "gamma",
            8,
            r"\pdfimageapplygamma=0",
            r"\pdfimageapplygamma=1\pdfgamma=1000\pdfimagegamma=2200",
        ),
        (
            "precision",
            8,
            r"\pdfdecimaldigits=0",
            r"\pdfdecimaldigits=4",
        ),
        (
            "resolution",
            8,
            r"\pdfimageresolution=72",
            r"\pdfimageresolution=144",
        ),
        (
            "unicode",
            8,
            r"\pdfgentounicode=1\pdfglyphtounicode{a}{03B1}",
            r"\pdfgentounicode=1\pdfglyphtounicode{a}{0061}",
        ),
        (
            "resource-policy",
            8,
            r"\pdfuniqueresname=1\pdfomitprocset=1",
            r"\pdfuniqueresname=0\pdfomitprocset=0",
        ),
        ("origin", 8, r"\pdfhorigin=0pt\pdfvorigin=0pt", r"\pdfhorigin=-10pt\pdfvorigin=20pt"),
    ];
    for (name, bits, first, second) in cases {
        let root = work.join(name);
        fs::create_dir(&root)?;
        fs::write(root.join("sample.png"), png(bits))?;
        let main = root.join("main.tex");
        let source = |settings: &str, generation: u64| {
            format!(
                r"\documentclass{{article}}
\pdfmajorversion=1\pdfminorversion=7 {settings}
\begin{{document}}BackendMarker{generation}.\par
\font\f=cmr10\f abc.\par
\pdfximage{{sample.png}}\setbox0=\hbox{{\pdfrefximage\pdflastximage}}
\typeout{{BACKEND-WIDTH=\the\wd0}}\box0
\end{{document}}"
            )
        };
        let saved = source(first, 0);
        fs::write(&main, &saved)?;
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
        for (generation, settings) in [(1u64, first), (2, second)] {
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":source(settings,generation)}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 {
                return Err(format!("{name}: {event}").into());
            }
            let pdf = Path::new(event["pdf"].as_str().ok_or("PDF missing")?);
            let (dictionary, samples) = image(pdf)?;
            match name {
                "highcolor" => {
                    let (depth, expected) = if generation == 1 {
                        (16, vec![0, 0, 39, 16, 128, 0, 195, 80, 255, 255])
                    } else {
                        (8, vec![0, 39, 128, 195, 255])
                    };
                    if !dictionary.contains(&format!("/BitsPerComponent {depth}"))
                        || samples != expected
                    {
                        return Err(format!(
                            "highcolor changed sample bits: {dictionary} {samples:?}"
                        )
                        .into());
                    }
                }
                "gamma" => {
                    let expected = if generation == 1 {
                        vec![0, 64, 128, 192, 255]
                    } else {
                        vec![0, 12, 56, 137, 255]
                    };
                    if samples != expected {
                        return Err(format!("gamma/cache failed: {samples:?}").into());
                    }
                }
                "precision" => {
                    let content = string(pdf, "pages/1/Contents")?;
                    let coordinates = regex::Regex::new(r"(?m)([-0-9.]+) ([-0-9.]+) Tm")?;
                    let values: Vec<_> = coordinates.captures_iter(&content).collect();
                    if values.is_empty()
                        || (generation == 1
                            && values
                                .iter()
                                .any(|v| v[1].contains('.') || v[2].contains('.')))
                        || (generation == 2
                            && !values
                                .iter()
                                .any(|v| v[1].contains('.') || v[2].contains('.')))
                    {
                        return Err(format!("coordinate precision ignored: {content}").into());
                    }
                }
                "resolution" => {
                    let log = fs::read_to_string(event["log"].as_str().ok_or("log missing")?)?;
                    let width = log
                        .lines()
                        .find_map(|v| v.strip_prefix("BACKEND-WIDTH="))
                        .ok_or("width missing")?
                        .trim_end_matches("pt")
                        .parse::<f64>()?;
                    let expected = if generation == 1 {
                        5. * 72.27 / 72.
                    } else {
                        5. * 72.27 / 144.
                    };
                    if (width - expected).abs() > 0.0001 {
                        return Err(
                            format!("resolution width {width} differs from {expected}").into()
                        );
                    }
                }
                "unicode" => {
                    let text =
                        output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
                    let text = String::from_utf8(text.stdout)?;
                    if !text.contains(if generation == 1 { "αbc." } else { "abc." }) {
                        return Err(format!("glyph Unicode override ignored: {text:?}").into());
                    }
                }
                "resource-policy" => {
                    let resources = string(pdf, "pages/1/Resources")?;
                    if resources.contains("/ProcSet") != (generation == 2) {
                        return Err("ProcSet policy ignored".into());
                    }
                    let names = string(pdf, "pages/1/Resources/XObject")?;
                    if names.contains("Im1P") != (generation == 1) {
                        return Err(format!("image namespace ignored: {names}").into());
                    }
                }
                "origin" => {
                    let reference = root.join(format!("reference{generation}"));
                    fs::create_dir(&reference)?;
                    fs::write(reference.join("oracle.tex"), source(settings, generation))?;
                    let output = output_timeout(Command::new("pdflatex").args(["-interaction=nonstopmode", "-halt-on-error", "-no-shell-escape"]).arg(format!("-output-directory={}",reference.display())).arg(reference.join("oracle.tex")).current_dir(&root), TIMEOUT)?;
                    if !output.status.success() {return Err(format!("origin oracle failed: {}",String::from_utf8_lossy(&output.stdout)).into());}
                    let coordinate = |text: &str| -> Result<(f64,f64),Box<dyn Error>> {
                        let pattern=regex::Regex::new(r"(?m)([-0-9.]+) ([-0-9.]+) T[m|d]")?;
                        let value=pattern.captures(text).ok_or("text origin missing")?;
                        Ok((value[1].parse()?,value[2].parse()?))
                    };
                    let actual=coordinate(&string(pdf,"pages/1/Contents")?)?;
                    let expected=coordinate(&string(&reference.join("oracle.pdf"),"pages/1/Contents")?)?;
                    if (actual.0-expected.0).abs()>0.02 || (actual.1-expected.1).abs()>0.02 {return Err(format!("origin differs: expected {expected:?}, actual {actual:?}").into());}
                }
                _ => {}
            }
            if fs::read_to_string(&main)? != saved {
                return Err("preview changed saved source".into());
            }
        }
        preview_regression::pass(
            name,
            (name == "origin").then_some("pdftex"),
            &[&source(first, 1), &source(second, 2)],
            "two unsaved generations and policy cache changes",
        )?;
    }
    // Draft is a deliberate no-publication state; switching it off resumes output.
    let root = work.join("draft");
    fs::create_dir(&root)?;
    let main = root.join("main.tex");
    let normal = r"\documentclass{article}\begin{document}DraftModeMarker.\end{document}";
    fs::write(&main, normal)?;
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
    session.send(&json!({"op":"update","generation":1,"files":[],"closed":[]}))?;
    let previous = session.wait_published(1, TIMEOUT)?;
    let draft = normal.replace(r"\begin{document}", r"\pdfdraftmode=1\begin{document}");
    session.send(
        &json!({"op":"update","generation":2,"files":[{"path":main,"text":draft}],"closed":[]}),
    )?;
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("draft event missing")?;
        if let Some(event) = session.next_event(remaining)? {
            if event["event"] == "published" && event["generation"] == 2 {
                return Err("draft mode published a PDF".into());
            }
            if event["event"] == "draft" && event["generation"] == 2 {
                break;
            }
        }
    }
    if !Path::new(previous["pdf"].as_str().ok_or("previous PDF missing")?).exists() {
        return Err("draft removed displayed PDF".into());
    }
    session.send(
        &json!({"op":"update","generation":3,"files":[{"path":main,"text":normal}],"closed":[]}),
    )?;
    session.wait_published(3, TIMEOUT)?;
    preview_regression::pass(
        "draft",
        None,
        &[normal, &draft],
        "output suppression and resumption",
    )?;
    Ok(())
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
