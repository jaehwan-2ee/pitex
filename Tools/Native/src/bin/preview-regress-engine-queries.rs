//! Compare real expandable PDF queries with pdfTeX and inspect their references.
//! Reference binaries are used only by this regression tool.
use pitex_native_tools::{
    preview_regression::{self, output_timeout, Session},
    TempDir,
};
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

fn pdf_fixture() -> Vec<u8> {
    let objects: [&[u8]; 4] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 150] /CropBox [10 20 180 120] /BleedBox [11 21 170 110] /TrimBox [12 22 160 100] /ArtBox [13 23 150 90] /Rotate 90 /Resources << >> /Contents 4 0 R >>",
        b"<< /Length 0 >>\nstream\n\nendstream",
    ];
    let mut data = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(data.len());
        data.extend(format!("{} 0 obj\n", index + 1).bytes());
        data.extend(*object);
        data.extend(b"\nendobj\n");
    }
    let xref = data.len();
    data.extend(b"xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        data.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    data.extend(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").bytes());
    data
}
fn png_fixture(bits: u8) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for byte in data {
            crc ^= *byte as u32;
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
            }
        }
        !crc
    }
    fn chunk(out: &mut Vec<u8>, name: &[u8; 4], body: &[u8]) {
        out.extend((body.len() as u32).to_be_bytes());
        let mut contents = name.to_vec();
        contents.extend(body);
        out.extend(&contents);
        out.extend(crc(&contents).to_be_bytes());
    }
    let mut data = b"\x89PNG\r\n\x1a\n".to_vec();
    chunk(
        &mut data,
        b"IHDR",
        &[0, 0, 0, 1, 0, 0, 0, 1, bits, 0, 0, 0, 0],
    );
    // A one-pixel grayscale row in a zlib stored block.
    let row = if bits == 16 {
        vec![0, 127, 0]
    } else {
        vec![0, 127]
    };
    let length = row.len() as u16;
    let mut stream = vec![0x78, 1, 1];
    stream.extend(length.to_le_bytes());
    stream.extend((!length).to_le_bytes());
    stream.extend(&row);
    let (mut a, mut b) = (1u32, 0u32);
    for byte in row {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    stream.extend(((b << 16) | a).to_be_bytes());
    chunk(&mut data, b"IDAT", &stream);
    chunk(&mut data, b"IEND", &[]);
    data
}
fn audit(log: &str) -> BTreeMap<String, String> {
    log.lines()
        .filter_map(|line| line.strip_prefix("AUDIT-"))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.into(), value.trim().into()))
        .collect()
}
fn show(pdf: &Path, selector: &str) -> Result<String, Box<dyn Error>> {
    let output = output_timeout(
        Command::new("mutool").arg("show").arg(pdf).arg(selector),
        TIMEOUT,
    )?;
    if !output.status.success() {
        return Err("mutool failed".into());
    }
    Ok(String::from_utf8(output.stdout)?)
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let binary = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temporary = TempDir::new("pitex-engine-queries-")?;
    let root = fs::canonicalize(temporary.path())?;
    fs::write(root.join("boxes.pdf"), pdf_fixture())?;
    fs::write(root.join("scaled.pdf"), pdf_fixture())?;
    fs::write(root.join("pixel.png"), png_fixture(8))?;
    fs::write(root.join("pixel16.png"), png_fixture(16))?;
    let source = r"\documentclass{article}
\pagestyle{empty}\pdfuniqueresname=UNIQUE
\def\bbox#1{\typeout{AUDIT-BOX#1=\pdfximagebbox\pdflastximage1,\pdfximagebbox\pdflastximage2,\pdfximagebbox\pdflastximage3,\pdfximagebbox\pdflastximage4}}
\def\probe#1{\pdfpagebox=#1\pdfximage width10pt {boxes.pdf}\bbox{#1}}
\begin{document}QueryMarker0.
\typeout{AUDIT-MEANING-PAGE=\meaning\pdfpageref}
\typeout{AUDIT-MEANING-FORM=\meaning\pdfxformname}
\typeout{AUDIT-MEANING-BOX=\meaning\pdfximagebbox}
\typeout{AUDIT-MEANING-DEPTH=\meaning\pdflastximagecolordepth}
\pdfminorversion=4\pdfoptionpdfminorversion=6\typeout{AUDIT-MINOR-ALIAS=\the\pdfminorversion,\the\pdfoptionpdfminorversion}\pdfminorversion=7
\pdfinclusionerrorlevel=3\pdfoptionpdfinclusionerrorlevel=5\typeout{AUDIT-INCLUSION-LEGACY=\the\pdfinclusionerrorlevel,\the\pdfoptionpdfinclusionerrorlevel}
\pdfinclusionerrorlevel=0\pdfoptionpdfinclusionerrorlevel=0
\immediate\pdfobj{<< /Audit (Object) >>}\edef\lastobject{\the\pdflastobj}
\edef\secondpage{\pdfpageref2}\edef\firstpage{\pdfpageref1}
\ifnum\secondpage=\pdfpageref2\typeout{AUDIT-PAGE-REPEAT=YES}\fi
\ifnum\lastobject=\pdflastobj\typeout{AUDIT-LASTOBJ-STABLE=YES}\fi
\pdfcatalog{/AuditFirst \firstpage\space0 R /AuditSecond \secondpage\space0 R}
\typeout{AUDIT-INITIAL-DEPTH=\the\pdflastximagecolordepth}
\pdfximage{pixel.png}\typeout{AUDIT-PNG-DEPTH=\the\pdflastximagecolordepth}\bbox{PNG}
\pdfimagehicolor=0\pdfximage{pixel16.png}\typeout{AUDIT-PNG16-DEPTH=\the\pdflastximagecolordepth}
\probe0\probe1\probe2\probe3\probe4\probe5
\typeout{AUDIT-PDF-DEPTH=\the\pdflastximagecolordepth}
\pdfforcepagebox=1\pdfximage cropbox {boxes.pdf}\bbox{FORCED}
\pdfoptionalwaysusepdfpagebox=2\typeout{AUDIT-ALIAS=\the\pdfforcepagebox}
\pdfforcepagebox=0\pdfoptionalwaysusepdfpagebox=3\pdfximage cropbox {boxes.pdf}\bbox{LEGACY}
\typeout{AUDIT-LEGACY-CONSUMED=\the\pdfforcepagebox,\the\pdfoptionalwaysusepdfpagebox}
\pdfforcepagebox=0\pdfoptionalwaysusepdfpagebox=0\pdfpagebox=2
\pdfximage width17pt height19pt depth3pt {scaled.pdf}\bbox{SCALED}
\setbox4=\hbox{\pdfrefximage\pdflastximage}
\typeout{AUDIT-DISPLAY=\the\wd4,\the\ht4,\the\dp4}\box4
\pdfpxdimen=2pt\dimen0=3px\typeout{AUDIT-PIXEL=\the\dimen0}
\setbox0=\hbox{FirstForm}\immediate\pdfxform0\edef\formone{\the\pdflastxform}
\typeout{AUDIT-FORM1=\pdfxformname\formone}
\immediate\pdfobj{<< /Audit (BetweenForms) >>}
\setbox0=\hbox{SecondForm}\immediate\pdfxform0\edef\formtwo{\the\pdflastxform}
\typeout{AUDIT-FORM2=\pdfxformname\formtwo}
\par\pdfrefxform\formone\quad\pdfrefxform\formtwo
\newpage SecondPage.
\ifnum\firstpage=\pdfpageref1\typeout{AUDIT-PAGE-BACKWARD=YES}\fi
\end{document}
";
    let main = root.join("main.tex");
    fs::write(&main, source.replace("UNIQUE", "0"))?;
    let reference = root.join("reference");
    fs::create_dir(&reference)?;
    let output = output_timeout(
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
    if !output.status.success() {
        return Err(format!(
            "reference failed: {}",
            String::from_utf8_lossy(&output.stdout)
        )
        .into());
    }
    let expected = audit(&fs::read_to_string(reference.join("main.log"))?);
    if expected.len() != 29 {
        return Err(format!("reference query coverage changed: {expected:?}").into());
    }
    let out = root.join("preview");
    fs::create_dir(&out)?;
    let mut session = Session::spawn(
        Command::new(binary.join("pitex-preview"))
            .arg("--root")
            .arg(&root)
            .args(["--main", "main.tex", "--out"])
            .arg(&out)
            .arg("--cache")
            .arg(root.join("cache"))
            .stderr(Stdio::null()),
        &out,
    )?;
    for generation in [1, 2] {
        let marker = format!("QueryMarker{generation}");
        let text = source
            .replace("UNIQUE", if generation == 1 { "0" } else { "1" })
            .replace("QueryMarker0", &marker);
        session.send(&json!({"op":"update", "generation":generation, "files":[{"path":main,"text":text}], "closed":[]}))?;
        let event = session.wait_published(generation, TIMEOUT)?;
        if event["errors"] != 0 || event["pages"] != 2 {
            return Err(format!("query compilation failed: {event}").into());
        }
        let actual = audit(&fs::read_to_string(
            event["log"].as_str().ok_or("log missing")?,
        )?);
        let equal = actual.len() == expected.len()
            && expected.iter().all(|(key, value)| {
                let Some(observed) = actual.get(key) else {
                    return false;
                };
                if key.starts_with("BOX") {
                    let dimensions = |text: &str| {
                        text.split(',')
                            .map(|dimension| dimension.strip_suffix("pt")?.parse::<f64>().ok())
                            .collect::<Option<Vec<_>>>()
                    };
                    match (dimensions(value), dimensions(observed)) {
                        (Some(expected), Some(actual)) if expected.len() == actual.len() => {
                            expected.iter().zip(actual).all(|(expected, actual)| {
                                (expected - actual).abs() <= 1.0 / 65536.0
                            })
                        }
                        _ => false,
                    }
                } else {
                    observed == value
                }
            });
        if !equal {
            return Err(format!(
                "query expansion differs: expected {expected:?}, actual {actual:?}"
            )
            .into());
        }
        let pdf = Path::new(event["pdf"].as_str().ok_or("PDF missing")?);
        for (name, page) in [("AuditFirst", 1), ("AuditSecond", 2)] {
            let alias = show(pdf, &format!("trailer/Root/{name}"))?;
            let target = show(pdf, &format!("pages/{page}"))?;
            if alias != target || !alias.contains("/Type /Page") {
                return Err(format!("page query {name} did not resolve to its real page").into());
            }
        }
        let resources = show(pdf, "pages/1/Resources/XObject")?;
        let contents = show(pdf, "pages/1/Contents")?;
        for ordinal in [1, 2] {
            let expression = if generation == 1 {
                format!(r"/Fm{ordinal}\s")
            } else {
                format!(r"/Fm{ordinal}P[0-9A-F]+\s")
            };
            if !regex::Regex::new(&expression)?.is_match(&resources)
                || !regex::Regex::new(&expression)?.is_match(&contents)
            {
                return Err(format!(
                    "form ordinal {ordinal} did not name the actual resource: {resources}"
                )
                .into());
            }
        }
    }
    let missing = r"\documentclass{article}\begin{document}MissingPage.\edef\missingpage{\pdfpageref20}\pdfcatalog{/Missing \missingpage\space0 R}\end{document}";
    session.send(&json!({"op":"update", "generation":3, "files":[{"path":main,"text":missing}], "closed":[]}))?;
    let event = session.wait_published(3, TIMEOUT)?;
    if event["errors"] != 0
        || event["pages"] != 1
        || !event["warnings"]
            .as_str()
            .unwrap_or("")
            .contains("Page 20 has been referenced but does not exist")
    {
        return Err(format!("missing-page query diagnostic failed: {event}").into());
    }
    let pdf = Path::new(event["pdf"].as_str().ok_or("PDF missing")?);
    if !show(pdf, "trailer/Root/Missing")?.contains("null") {
        return Err("missing-page reference is not null".into());
    }
    drop(session);
    preview_regression::pass("expandable-queries", Some("pdftex"), &[source, missing], "29 oracle values, forward/backward page identity, form resources, unsaved generations and missing-page warning")?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
