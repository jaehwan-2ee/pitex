//! Differential PostScript/PSTricks regressions. Reference executables are
//! used only by this test; rendering in the application is entirely internal.
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
type Result<T> = std::result::Result<T, Box<dyn Error>>;
const TIMEOUT: Duration = Duration::from_secs(60);
fn raster(pdf: &Path, prefix: &Path) -> Result<Vec<u8>> {
    let output = output_timeout(
        Command::new("pdftoppm")
            .args(["-f", "1", "-singlefile", "-r", "96"])
            .arg(pdf)
            .arg(prefix),
        TIMEOUT,
    )?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(fs::read(prefix.with_extension("ppm"))?)
}
fn pixel_data(ppm: &[u8]) -> Result<&[u8]> {
    let mut i = 0;
    let mut words = Vec::new();
    while words.len() < 4 {
        while i < ppm.len() && ppm[i].is_ascii_whitespace() {
            i += 1;
        }
        if ppm.get(i) == Some(&b'#') {
            while i < ppm.len() && ppm[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        let start = i;
        while i < ppm.len() && !ppm[i].is_ascii_whitespace() {
            i += 1;
        }
        if start == i {
            return Err("invalid PPM".into());
        }
        words.push(std::str::from_utf8(&ppm[start..i])?);
    }
    if words[0] != "P6" || words[3] != "255" {
        return Err("unsupported PPM".into());
    }
    let size = words[1]
        .parse::<usize>()?
        .checked_mul(words[2].parse::<usize>()?)
        .and_then(|n| n.checked_mul(3))
        .ok_or("PPM size overflow")?;
    let offset = ppm.len().checked_sub(size).ok_or("short PPM")?;
    if offset <= i {
        return Err("short PPM header".into());
    }
    Ok(&ppm[offset..])
}
fn run() -> Result<()> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temporary = TempDir::new("pitex-ps-vm-")?;
    let root = fs::canonicalize(temporary.path())?;
    let out = root.join("preview");
    let reference = root.join("reference");
    fs::create_dir(&out)?;
    fs::create_dir(&reference)?;
    let program = r"/a [10 20] def /alias a def /slice a 1 1 getinterval def /d << /n 7 >> def /str (abc) def /s save def a 0 99 put slice 0 88 put d /n 8 put str 0 90 put s restore a 0 get 10 eq alias 0 get 10 eq and slice 0 get 20 eq and d /n get 7 eq and str 0 get 90 eq and
    /ro a readonly def a wcheck and ro wcheck not and ro rcheck and
    /dictalias d def d readonly pop dictalias wcheck not and
    true setglobal /g [1] def false setglobal /s save def g 0 9 put a 0 8 put s restore g 0 get 9 eq and a 0 get 10 eq and g gcheck and a gcheck not and currentglobal not and
    /p {1 2} def /q /p load def /s save def /p load 0 8 put s restore /q load 0 get 1 eq and /p load cvlit /q load eq and
    true setpacking /packed {1} def false setpacking /packed load type /packedarraytype eq and /packed load wcheck not and currentpacking not and
    /VMFile (4142>) /ASCIIHexDecode filter def /VMBlocked VMFile noaccess def VMFile rcheck and VMBlocked rcheck not and VMFile wcheck not and VMFile VMBlocked eq and VMFile read {65 eq}{false} ifelse and VMFile read {66 eq}{false} ifelse and
    {0 .6 0 setrgbcolor}{1 0 0 setrgbcolor} ifelse 0 0 80 35 rectfill";
    let oracle = output_timeout(
        Command::new("gs")
            .args(["-q", "-dNODISPLAY", "-dBATCH", "-c"])
            .arg(program),
        TIMEOUT,
    )?;
    if !oracle.status.success() {
        return Err(format!(
            "invalid VM oracle fixture: {}",
            String::from_utf8_lossy(&oracle.stdout)
        )
        .into());
    }
    let source = format!(
        r"\documentclass{{article}}
\usepackage[paperwidth=12cm,paperheight=12cm,margin=12mm]{{geometry}}
\usepackage{{pstricks}}\pagestyle{{empty}}
\begin{{document}}VMMarker0.\par\begin{{pspicture}}(0,0)(4,2)\pstverb{{{program}}}\end{{pspicture}}\end{{document}}"
    );
    let main = root.join("main.tex");
    fs::write(&main, &source)?;
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
    for generation in [1, 2] {
        let text = source.replace("VMMarker0", &format!("VMMarker{generation}"));
        fs::write(reference.join("main.tex"), &text)?;
        let output = output_timeout(
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
        if !output.status.success() {
            return Err(format!(
                "reference failed {}",
                String::from_utf8_lossy(&output.stdout)
            )
            .into());
        }
        let expected = raster(
            &reference.join("main.pdf"),
            &root.join(format!("reference{generation}")),
        )?;
        let expected = pixel_data(&expected)?;
        if expected
            .chunks_exact(3)
            .filter(|pixel| pixel[1] > 80 && pixel[1] < 200 && pixel[0] < 20 && pixel[2] < 20)
            .count()
            < 1000
        {
            return Err("oracle VM predicates did not paint green".into());
        }
        session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":text}],"closed":[]}))?;
        let event = session.wait_published(generation, TIMEOUT)?;
        if event["errors"] != 0
            || event["warnings"]
                .as_str()
                .unwrap_or("")
                .contains("PostScript")
        {
            return Err(format!("VM preview failed: {event}").into());
        }
        let actual = raster(
            Path::new(event["pdf"].as_str().ok_or("PDF missing")?),
            &root.join(format!("actual{generation}")),
        )?;
        let actual = pixel_data(&actual)?;
        if actual.len() != expected.len() {
            return Err("VM page dimensions differ".into());
        }
        let mean = actual
            .iter()
            .zip(expected)
            .map(|(a, b)| (*a as i16 - *b as i16).unsigned_abs() as u64)
            .sum::<u64>() as f64
            / actual.len() as f64;
        if mean > 0.1 {
            return Err(format!("VM predicates/raster differ: {mean}").into());
        }
        println!("PASS PostScript VM generation{generation}, reference raster mean={mean:.5}");
    }
    if fs::read_to_string(&main)? != source {
        return Err("preview changed saved VM source".into());
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
