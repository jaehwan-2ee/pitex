//! A variable OpenType CFF2 face in a native preview session: the PDF must
//! render like XeLaTeX's output for the same face's default instance as a
//! static CFF font, copy its text, and embed a subset of the CFF2 program.
//! Stock xdvipdfmx (TeX Live 2023) cannot write CFF2 faces at all, so the
//! static twin from `Fixtures/fonts/cff2/make-font.py` is the oracle.
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
const TEXT: [&str; 2] = [
    "The quick brown fox jumps over the lazy dog.",
    "Pack my box with five dozen liquor jugs.",
];
fn source(font: &str) -> String {
    format!(
        "\\documentclass{{article}}\\pagestyle{{empty}}\n\\usepackage{{fontspec}}\n\\newfontface\\testface{{{font}}}[Path=./]\n\\begin{{document}}\n{{\\testface {}}}\\par\n{{\\testface {}}}\n\\end{{document}}\n",
        TEXT[0], TEXT[1]
    )
}
/// Gray pixels of page 1 at 150 dpi.
fn raster(pdf: &Path) -> Result<(usize, Vec<u8>), Box<dyn Error>> {
    let output = output_timeout(
        Command::new("pdftoppm")
            .args(["-singlefile", "-r", "150", "-gray"])
            .arg(pdf),
        TIMEOUT,
    )?;
    let bytes = output.stdout;
    let header = String::from_utf8_lossy(&bytes[..bytes.len().min(32)]).to_string();
    let fields = header.split_ascii_whitespace().take(4).collect::<Vec<_>>();
    if !output.status.success() || fields.len() < 4 || fields[0] != "P5" || fields[3] != "255" {
        return Err(format!("rasterization failed: {header:?}").into());
    }
    let (width, height) = (fields[1].parse::<usize>()?, fields[2].parse::<usize>()?);
    Ok((width, bytes[bytes.len() - width * height..].to_vec()))
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Fixtures/fonts/cff2");
    let temp = TempDir::new("pitex-variable-cff-")?;
    let root = fs::canonicalize(temp.path())?;
    for font in ["PitexCFF2Test.otf", "PitexCFF2Test-Default.otf"] {
        fs::copy(fixtures.join(font), root.join(font))?;
    }
    let reference = root.join("reference");
    fs::create_dir(&reference)?;
    fs::write(
        reference.join("main.tex"),
        source("PitexCFF2Test-Default.otf"),
    )?;
    fs::copy(
        fixtures.join("PitexCFF2Test-Default.otf"),
        reference.join("PitexCFF2Test-Default.otf"),
    )?;
    let oracle = output_timeout(
        Command::new("xelatex")
            .args([
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-no-shell-escape",
                "main.tex",
            ])
            .current_dir(&reference),
        TIMEOUT,
    )?;
    if !oracle.status.success() {
        return Err(format!("oracle: {}", String::from_utf8_lossy(&oracle.stdout)).into());
    }
    let main = root.join("main.tex");
    let text = source("PitexCFF2Test.otf");
    fs::write(&main, &text)?;
    let out = root.join("preview");
    fs::create_dir(&out)?;
    let mut session = Session::spawn(
        Command::new(bin.join("pitex-preview"))
            .arg("--root")
            .arg(&root)
            .args(["--main", "main.tex", "--out"])
            .arg(&out)
            .arg("--cache")
            .arg(root.join("cache"))
            .stderr(Stdio::null()),
        &out,
    )?;
    session.send(
        &json!({"op":"update","generation":1,"files":[{"path":main,"text":text}],"closed":[]}),
    )?;
    let event = session.wait_published(1, TIMEOUT)?;
    if event["errors"] != 0 {
        return Err(format!("native: {event}").into());
    }
    let pdf = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
    let copied = output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
    let copied = String::from_utf8(copied.stdout)?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if copied != TEXT.join(" ") {
        return Err(format!("copied text differs: {copied:?}").into());
    }
    let (width, pixels) = raster(pdf)?;
    let (reference_width, expected) = raster(&reference.join("main.pdf"))?;
    if (width, pixels.len()) != (reference_width, expected.len()) {
        return Err("page geometry differs from the oracle".into());
    }
    let ink = pixels
        .iter()
        .zip(&expected)
        .filter(|(a, b)| **a < 128 || **b < 128)
        .count();
    let different = pixels
        .iter()
        .zip(&expected)
        .filter(|(a, b)| a.abs_diff(**b) > 64)
        .count();
    if ink < 500 || different * 100 > ink {
        return Err(format!(
            "rendering differs from the oracle on {different} of {ink} ink pixels"
        )
        .into());
    }
    let fonts = output_timeout(Command::new("pdffonts").arg(pdf), TIMEOUT)?;
    let fonts = String::from_utf8(fonts.stdout)?;
    let face = fonts
        .lines()
        .find(|l| l.starts_with("PitexCFF2Test"))
        .ok_or(format!("face missing: {fonts}"))?;
    if !face.contains("CID Type 0C (OT)") || face.split_whitespace().rev().nth(4) != Some("yes") {
        return Err(format!("CFF2 face not embedded as OpenType: {face}").into());
    }
    let show = |args: &[&str]| -> Result<Vec<u8>, Box<dyn Error>> {
        let result = output_timeout(
            Command::new("mutool")
                .arg("show")
                .args(&args[..args.len() - 1])
                .arg(pdf)
                .arg(args[args.len() - 1]),
            TIMEOUT,
        )?;
        if !result.status.success() {
            return Err(format!(
                "mutool show failed: {}",
                String::from_utf8_lossy(&result.stderr)
            )
            .into());
        }
        Ok(result.stdout)
    };
    // `grep` prints one object per line; the font program is the OpenType FontFile3 stream.
    let objects = String::from_utf8(show(&["grep"])?)?;
    let number = regex::Regex::new(r"(?m)^(\d+) 0 obj.*/Subtype\s*/OpenType")?
        .captures(&objects)
        .ok_or(format!("no OpenType font program: {objects}"))?[1]
        .to_string();
    let program = show(&["-b", &number])?;
    let whole = fs::read(fixtures.join("PitexCFF2Test.otf"))?;
    let count = program
        .get(4..6)
        .map_or(0, |b| u16::from_be_bytes([b[0], b[1]]) as usize);
    let tags = (0..count)
        .filter_map(|i| program.get(12 + 16 * i..16 + 16 * i))
        .collect::<Vec<_>>();
    // Unused capitals are emptied and STAT/HVAR dropped, so the program shrinks.
    if !tags.contains(&b"CFF2".as_slice())
        || tags.contains(&b"STAT".as_slice())
        || program.len() >= whole.len()
    {
        return Err(format!(
            "CFF2 program not subset: {} bytes of {}",
            program.len(),
            whole.len()
        )
        .into());
    }
    preview_regression::pass(
        "variable-cff2",
        Some("xetex"),
        &[&text],
        "CFF2 face renders like the static default instance and copies its text",
    )
}
fn main() {
    pitex_native_tools::exit_on_error(run());
}
