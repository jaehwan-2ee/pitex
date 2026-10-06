//! Public font-primitive metrics compared with stock pdfTeX; real native preview
//! sessions cover PDF font dictionaries, code tables, typography and unsaved edits.
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
fn show(pdf: &Path, query: &str) -> Result<String, Box<dyn Error>> {
    let result = output_timeout(
        Command::new("mutool").arg("show").arg(pdf).arg(query),
        TIMEOUT,
    )?;
    if !result.status.success() {
        return Err(format!("mutool failed: {}", String::from_utf8_lossy(&result.stderr)).into());
    }
    Ok(String::from_utf8(result.stdout)?)
}
fn records(log: &str) -> Vec<String> {
    log.lines()
        .filter(|s| s.starts_with("FONT-AUDIT-"))
        .map(str::to_string)
        .collect()
}
fn raster(pdf: &Path) -> Result<(usize, usize, Vec<u8>), Box<dyn Error>> {
    let output = output_timeout(
        Command::new("pdftoppm")
            .args(["-singlefile", "-r", "72"])
            .arg(pdf),
        TIMEOUT,
    )?;
    if !output.status.success() {
        return Err("rasterization failed".into());
    }
    let bytes = output.stdout;
    let mut cursor = 0;
    let mut tokens = Vec::new();
    while tokens.len() < 4 {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if bytes.get(cursor) == Some(&b'#') {
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            continue;
        }
        let start = cursor;
        while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        tokens.push(std::str::from_utf8(&bytes[start..cursor])?.to_string());
    }
    if tokens[0] != "P6" || tokens[3] != "255" {
        return Err("unexpected PPM format".into());
    }
    cursor += 1;
    let width = tokens[1].parse::<usize>()?;
    let height = tokens[2].parse::<usize>()?;
    let pixels = bytes[cursor..].to_vec();
    if pixels.len() != width * height * 3 {
        return Err("PPM data size mismatch".into());
    }
    Ok((width, height, pixels))
}
fn compare_rendering(pdf: &Path, reference: &Path) -> Result<(), Box<dyn Error>> {
    let (width, height, pixels) = raster(pdf)?;
    let (rw, rh, expected) = raster(reference)?;
    if (width, height) != (rw, rh) {
        return Err("rendered page geometry changed".into());
    }
    let bounds = output_timeout(
        Command::new("pdftotext")
            .args(["-bbox", "-f", "1", "-l", "1"])
            .arg(reference)
            .arg("-"),
        TIMEOUT,
    )?;
    let bounds = String::from_utf8(bounds.stdout)?;
    let line = bounds
        .lines()
        .find(|line| line.contains("TypographyMarkerOriginal"))
        .ok_or("marker bounds missing")?;
    let bottom = regex::Regex::new(r#"yMax="([0-9.]+)""#)?
        .captures(line)
        .ok_or("marker y bound missing")?[1]
        .parse::<f64>()?
        + 2.;
    let mut ink = 0;
    let mut different = 0;
    for (index, (left, right)) in pixels
        .chunks_exact(3)
        .zip(expected.chunks_exact(3))
        .enumerate()
    {
        if (index / width) as f64 <= bottom {
            continue;
        }
        if left.iter().chain(right).any(|channel| *channel < 245) {
            ink += 1;
            if left.iter().zip(right).any(|(a, b)| a.abs_diff(*b) > 32) {
                different += 1;
            }
        }
    }
    if ink == 0 || different as f64 / ink as f64 > 0.20 {
        return Err(format!("body rendering differs on {different}/{ink} ink pixels").into());
    }
    Ok(())
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temp = TempDir::new("pitex-font-typography-")?;
    let work = fs::canonicalize(temp.path())?;
    let cases = [
        (
            "code-tables",
            "",
            r"\font\f=cmr10 \typeout{FONT-AUDIT-DEFAULT=\the\efcode\f`A,\the\knaccode\f`A}\efcode\f`A=4000 {\efcode\f`B=-2\knaccode\f`!=1500\knbccode\f`?=-2000\knbscode\f`.=200\stbscode\f`.=100\shbscode\f`.=-100}\typeout{FONT-AUDIT-CODES=\the\efcode\f`A,\the\efcode\f`B,\the\knaccode\f`!,\the\knbccode\f`?,\the\knbscode\f`.,\the\stbscode\f`.,\the\shbscode\f`.}Code tables.",
            true,
        ),
        (
            "font-copy",
            "",
            r"\font\f=cmr10 at12pt\pdfcopyfont\copy\f\typeout{FONT-AUDIT-SIZE=\pdffontsize\copy}\setbox0=\hbox{\f ABC}\setbox1=\hbox{\copy ABC}\typeout{FONT-AUDIT-WIDTH=\the\wd0,\the\wd1}\fontdimen2\copy=8pt\typeout{FONT-AUDIT-COPYSPACE=\the\fontdimen2\f,\the\fontdimen2\copy}Font copy.",
            true,
        ),
        (
            "letterspace",
            "",
            r"\font\f=cmr10 at12pt\letterspacefont\wide\f100\letterspacefont\tight\f-50\setbox0=\hbox{\f ABC}\setbox1=\hbox{\wide ABC}\setbox2=\hbox{\tight ABC}\typeout{FONT-AUDIT-WIDTH=\the\wd0,\the\wd1,\the\wd2}\noindent\f ABC\par\noindent\wide ABC\par\noindent\tight ABC",
            true,
        ),
        (
            "native-letterspace",
            r"\usepackage{fontspec}\setmainfont{Latin Modern Roman}",
            r"\letterspacefont\wide\font100\setbox0=\hbox{ABC}\setbox1=\hbox{\wide ABC}\typeout{FONT-AUDIT-NATIVE=\the\wd0,\the\wd1}\noindent ABC\par\noindent\wide ABC",
            false,
        ),
        (
            "native-ligatures",
            r"\usepackage{fontspec}\setmainfont{Latin Modern Roman}",
            r"\setbox0=\hbox{office AV}\typeout{FONT-AUDIT-BEFORE=\the\wd0}\pdfnoligatures\font\setbox1=\hbox{office AV}\typeout{FONT-AUDIT-AFTER=\the\wd1}\box0\par\box1",
            false,
        ),
        (
            "ligatures",
            "",
            r"\font\f=cmr10\setbox0=\hbox{\f office AV}\typeout{FONT-AUDIT-BEFORE=\the\wd0}\pdfnoligatures\f\setbox1=\hbox{\f office AV}\typeout{FONT-AUDIT-AFTER=\the\wd1}\noindent\f office AV",
            true,
        ),
        (
            "prepend-append",
            "",
            r"\font\f=cmr10\pdfprependkern=1\pdfappendkern=1\knbccode\f`?=150\knaccode\f`!=200\setbox0=\hbox{\f ?A!}\typeout{FONT-AUDIT-SPACING=\the\wd0}\setbox1=\hbox{\unhcopy0}\typeout{FONT-AUDIT-UNBOX=\the\wd1}\f ?A!",
            true,
        ),
        (
            "interword-glue",
            "",
            r"\font\f=cmr10\pdfadjustinterwordglue=1\knbscode\f`.=200\stbscode\f`.=100\shbscode\f`.=-100\setbox0=\hbox{\f A. A}\typeout{FONT-AUDIT-GLUE=\the\wd0}\f A. A",
            true,
        ),
        (
            "paragraph-dimensions",
            "",
            r"\pdffirstlineheight=20pt\pdflastlinedepth=7pt\pdfeachlineheight=15pt\pdfeachlinedepth=3pt\setbox0=\vbox{\hsize=2cm\noindent Alpha beta gamma delta epsilon zeta eta theta iota kappa.\par\global\setbox1=\lastbox}\typeout{FONT-AUDIT-PARAGRAPH=\the\ht1,\the\dp1}\box0\box1",
            true,
        ),
        (
            "insert-height",
            "",
            r"\insert254{\hbox{\vrule height12pt depth3pt width1pt}}\par\typeout{FONT-AUDIT-INSERT=\pdfinsertht254}Insertion.",
            true,
        ),
        (
            "font-resources",
            "",
            r"\font\f=cmr10\pdffontattr\f{/AuditFont (FontAttribute)}\edef\fontobject{\pdffontobjnum\f}\pdfcatalog{/AuditFontObject \fontobject\space 0 R}\edef\fontresource{\pdffontname\f}\typeout{FONT-RESOURCE=\fontresource}\pdfincludechars\f{AZ}\noindent\f Visible font.",
            false,
        ),
        (
            "unused-font",
            "",
            r"\font\f=cmbx10\edef\fontobject{\pdffontobjnum\f}\pdfcatalog{/AuditUnusedFont \fontobject\space 0 R}\pdfincludechars\f{Z}Visible normal font.",
            false,
        ),
        (
            "disable-unicode",
            "",
            r"\font\f=cmr10\pdfgentounicode=1\pdfnobuiltintounicode\f\edef\object{\pdffontobjnum\f}\pdfcatalog{/AuditFont \object\space 0 R}\noindent\f Unicode disabled font.",
            true,
        ),
        (
            "native-disable-unicode",
            r"\usepackage{fontspec}\setmainfont{Latin Modern Roman}",
            r"\pdfnobuiltintounicode\font\edef\object{\pdffontobjnum\font}\pdfcatalog{/AuditFont \object\space 0 R}Native Unicode disabled.",
            false,
        ),
        (
            "fake-space",
            "",
            r"A\pdffakespace B\par\pdfinterwordspaceon C D\par\pdfinterwordspaceoff E F.",
            true,
        ),
        (
            "space-font",
            "",
            r"\pdfspacefont{texnansi-lmr10}A\pdffakespace B",
            true,
        ),
        (
            "charset-default",
            "",
            r"\font\f=cmr10\edef\object{\pdffontobjnum\f}\pdfcatalog{/AuditFont \object\space 0 R}\f ABZ",
            true,
        ),
        (
            "charset-omit",
            r"\pdfomitcharset=1",
            r"\font\f=cmr10\edef\object{\pdffontobjnum\f}\pdfcatalog{/AuditFont \object\space 0 R}\f ABZ",
            true,
        ),
        (
            "quitvmode",
            "",
            r"\quitvmode X\par\typeout{FONT-AUDIT-PREV=\the\prevgraf}Paragraph entry.",
            true,
        ),
        (
            "glyph-expansion",
            "",
            r"\pdftracingfonts=1\tracingoutput=1\showboxbreadth=100\showboxdepth=10\pdfadjustspacing=1\font\f=cmr10\pdffontexpand\f100 100 1 autoexpand\efcode\f`A=0\efcode\f`B=500\hbox to80pt{\vbox{\hsize=80pt\noindent\f AABBAABBAABBAA\break}}",
            true,
        ),
    ];
    for (name, preamble, body, oracle) in cases {
        if let Ok(filter) = std::env::var("PITEX_FONT_CASE") {
            if !name.contains(&filter) {
                continue;
            }
        }
        let root = work.join(name);
        fs::create_dir(&root)?;
        let main = root.join("main.tex");
        let source=format!("\\documentclass{{article}}\n\\pagestyle{{empty}}\n{preamble}\n\\begin{{document}}\nTypographyMarkerOriginal.\n\n{body}\n\\end{{document}}\n");
        fs::write(&main, &source)?;
        let mut expected = Vec::new();
        let mut reference_pdf = None;
        if oracle {
            let reference = root.join("reference");
            fs::create_dir(&reference)?;
            let result = output_timeout(
                Command::new("pdflatex")
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
            expected = records(&fs::read_to_string(reference.join("main.log"))?);
            reference_pdf = Some(reference.join("main.pdf"));
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
            let marker = format!("TypographyMarker{generation}");
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":source.replace("TypographyMarkerOriginal",&marker)}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 {
                return Err(format!("native {name}: {event}").into());
            }
            let log = fs::read_to_string(event["log"].as_str().ok_or("missing log")?)?;
            if oracle && records(&log) != expected {
                return Err(format!(
                    "{name}: metrics differ\nexpected {expected:?}\nactual {:?}",
                    records(&log)
                )
                .into());
            }
            let pdf = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
            let text = output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
            if !String::from_utf8(text.stdout)?.contains(&marker) {
                return Err(format!("{name}: unsaved text missing").into());
            }
            if matches!(
                name,
                "letterspace" | "ligatures" | "prepend-append" | "interword-glue"
            ) {
                compare_rendering(
                    pdf,
                    reference_pdf.as_deref().ok_or("rendering oracle missing")?,
                )?;
            }
            match name {
                "font-resources" => {
                    let font = show(pdf, "trailer/Root/AuditFontObject")?;
                    if !font.contains("/Type /Font") || !font.contains("FontAttribute") {
                        return Err(format!("font resource/attribute missing: {font}").into());
                    }
                    let resource = log
                        .lines()
                        .find_map(|s| s.strip_prefix("FONT-RESOURCE="))
                        .ok_or("fontresource query missing")?;
                    if !show(pdf, "pages/1/Resources/Font")?.contains(&format!("/F{resource} ")) {
                        return Err("font name did not match PDF resource".into());
                    }
                }
                "unused-font" => {
                    if !show(pdf, "trailer/Root/AuditUnusedFont")?.contains("/LastChar 90") {
                        return Err("unused glyph inclusion did not create font widths".into());
                    }
                }
                "disable-unicode" | "native-disable-unicode" => {
                    if show(pdf, "trailer/Root/AuditFont")?.contains("/ToUnicode") {
                        return Err("per-font Unicode suppression ignored".into());
                    }
                }
                "fake-space" => {
                    if show(pdf, "pages/1/Contents")?.matches("( ) Tj").count() != 2 {
                        return Err("explicit/automatic fake spaces missing".into());
                    }
                }
                "space-font" => {
                    let contents = show(pdf, "pages/1/Contents")?;
                    if !contents.contains("( ) Tj") || contents.contains("/PitexSpace") {
                        return Err("configured space font was ignored".into());
                    }
                }
                "charset-default" => {
                    let charset = show(pdf, "trailer/Root/AuditFont/FontDescriptor/CharSet")?;
                    if !charset.contains("/A/B/Z") {
                        return Err(format!("included Type1 glyph names missing: {charset}").into());
                    }
                }
                "charset-omit" => {
                    if show(pdf, "trailer/Root/AuditFont/FontDescriptor")?.contains("/CharSet") {
                        return Err("pdfomitcharset was ignored".into());
                    }
                }
                "native-ligatures" => {
                    let values = records(&log);
                    let number = regex::Regex::new(r"=([0-9.]+)pt")?;
                    let before = number.captures(&values[0]).ok_or("native width missing")?[1]
                        .parse::<f64>()?;
                    let after = number.captures(&values[1]).ok_or("native width missing")?[1]
                        .parse::<f64>()?;
                    if after <= before {
                        return Err(
                            "native ligature/kerning suppression did not change metrics".into()
                        );
                    }
                }
                "glyph-expansion" => {
                    let content = show(pdf, "pages/1/Contents")?;
                    if !content.contains("0.95 0") || !content.contains("1 0") {
                        return Err(format!("efcode did not alter outlines: {content}").into());
                    }
                    if !log.contains("cmr10-50@10.0pt") {
                        return Err(format!("expanded font tracing details missing: {log}").into());
                    }
                }
                _ => {}
            }
            if fs::read_to_string(&main)? != source {
                return Err("preview modified the saved source".into());
            }
        }
        let rendered = matches!(
            name,
            "letterspace" | "ligatures" | "prepend-append" | "interword-glue"
        );
        let reference = (oracle && (!expected.is_empty() || rendered)).then_some("pdftex");
        preview_regression::pass(
            name,
            reference,
            &[&source],
            "native behavior and two unsaved generations",
        )?;
    }
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
