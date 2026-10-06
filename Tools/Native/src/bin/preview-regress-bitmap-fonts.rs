//! Original PK fixtures and an installed prebuilt font exercise native Type 3
//! output against pdfTeX, with font generation disabled in the test oracle.
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
const TIMEOUT: Duration = Duration::from_secs(60);
fn show(pdf: &Path, key: &str) -> Result<String, Box<dyn Error>> {
    let result = output_timeout(
        Command::new("mutool").arg("show").arg(pdf).arg(key),
        TIMEOUT,
    )?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).into_owned().into());
    }
    Ok(String::from_utf8(result.stdout)?)
}
fn installed(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let result = output_timeout(Command::new("kpsewhich").arg(name), TIMEOUT)?;
    let path = PathBuf::from(String::from_utf8(result.stdout)?.trim());
    if !path.is_file() {
        return Err(format!("installed fixture missing: {name}").into());
    }
    Ok(path)
}
fn fixture(dpi: u32) -> Vec<u8> {
    let mut data = vec![247, 89, 0];
    let ppp = (dpi as f64 * 65536. / 72.27).round() as u32;
    for n in [10 << 20, 0, ppp, ppp] {
        data.extend_from_slice(&n.to_be_bytes());
    }
    // This raster is authored for the regression. Rows are densely packed in
    // PK, including the cross-byte transitions; the PDF mask pads every row.
    for (code, pattern) in [
        (65u8, [14u8, 17, 17, 31, 17, 17, 17]),
        (90, [31, 1, 2, 4, 8, 16, 31]),
    ] {
        data.extend_from_slice(&[224, 13, code, 0x0c, 0, 0, 8, 5, 7, 0, 6]);
        let mut raw = [0u8; 5];
        for (row, bits) in pattern.iter().enumerate() {
            for x in 0..5 {
                if bits & (16 >> x) != 0 {
                    let bit = row * 5 + x;
                    raw[bit / 8] |= 128 >> (bit % 8);
                }
            }
        }
        data.extend_from_slice(&raw);
    }
    data.push(245);
    while data.len() % 4 != 0 {
        data.push(246);
    }
    data
}
fn raster(pdf: &Path) -> Result<(usize, usize, Vec<u8>), Box<dyn Error>> {
    let output = output_timeout(
        Command::new("pdftoppm")
            .args(["-singlefile", "-r", "144"])
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
        while bytes.get(cursor).is_some_and(|b| b.is_ascii_whitespace()) {
            cursor += 1;
        }
        if bytes.get(cursor) == Some(&b'#') {
            while bytes.get(cursor).is_some_and(|b| *b != b'\n') {
                cursor += 1;
            }
            continue;
        }
        let start = cursor;
        while bytes.get(cursor).is_some_and(|b| !b.is_ascii_whitespace()) {
            cursor += 1;
        }
        tokens.push(std::str::from_utf8(&bytes[start..cursor])?.to_owned());
    }
    if tokens[0] != "P6" || tokens[3] != "255" {
        return Err("unexpected PPM format".into());
    }
    cursor += 1;
    let width = tokens[1].parse::<usize>()?;
    let height = tokens[2].parse::<usize>()?;
    Ok((width, height, bytes[cursor..].to_vec()))
}
fn compare(pdf: &Path, reference: &Path) -> Result<(), Box<dyn Error>> {
    let (w, h, a) = raster(pdf)?;
    let (rw, rh, b) = raster(reference)?;
    if (w, h) != (rw, rh) || a.len() != b.len() {
        return Err("page geometry differs".into());
    }
    // The marker changes between the two unsaved updates; the bitmap line is
    // placed below the marker's ordinary Type 1 text.
    let (mut ink, mut different) = (0, 0);
    for (index, (a, b)) in a.chunks_exact(3).zip(b.chunks_exact(3)).enumerate() {
        if index / w < 280 || index / w > 1000 {
            continue;
        }
        if a.iter().chain(b).any(|c| *c < 245) {
            ink += 1;
            if a.iter().zip(b).any(|(a, b)| a.abs_diff(*b) > 40) {
                different += 1;
            }
        }
    }
    if ink < 10 || different as f64 / ink as f64 > 0.20 {
        return Err(format!("PK rendering differs on {different}/{ink} ink pixels").into());
    }
    Ok(())
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temp = TempDir::new("pitex-bitmap-fonts-")?;
    let work = fs::canonicalize(temp.path())?;
    let tfm = installed("cmr10.tfm")?;
    let prebuilt = installed("cmr10.pk")?;
    for (name, dpi, mode, prebuilt_font, omit, no_unicode) in [
        ("raw-default", 0, "", false, false, false),
        ("raw-dpi300-mode", 300, "auditmode", false, false, false),
        ("prebuilt-dpi600", 600, "ljfour", true, false, false),
        ("prebuilt-scaled20", 300, "ljfour", true, false, false),
        ("missing-type1-fallback", 72, "", false, false, false),
        ("bitmap-omit-charset", 72, "", false, true, false),
        ("bitmap-no-unicode", 72, "", false, false, true),
    ] {
        let root = work.join(name);
        fs::create_dir(&root)?;
        fs::copy(&tfm, root.join("pitexbitmap.tfm"))?;
        let actual = if name == "prebuilt-scaled20" {
            600
        } else if dpi == 0 {
            72
        } else {
            dpi
        };
        let filename = if mode == "auditmode" {
            let dir = root.join("auditmode/dpi300");
            fs::create_dir_all(&dir)?;
            dir.join("pitexbitmap.pk")
        } else {
            root.join(format!("pitexbitmap.{actual}pk"))
        };
        if prebuilt_font {
            fs::copy(&prebuilt, &filename)?;
        } else {
            fs::write(&filename, fixture(actual))?;
        }
        let mut settings=format!("\\pdfpkresolution={dpi}\\pdfpkmode={{{mode}}}\\pdfgentounicode=1\\pdfglyphtounicode{{a65}}{{0041}}\\pdfglyphtounicode{{a90}}{{005A}}");
        if omit {
            settings += "\\pdfomitcharset=1";
        }
        if name == "missing-type1-fallback" {
            settings+="\\ifdefined\\XeTeXversion\\pdfmapline{pitexbitmap PitexMissingType1 <missing-pitex-bitmap.pfb}\\fi";
        }
        let suppression = if no_unicode {
            "\\pdfnobuiltintounicode\\f"
        } else {
            ""
        };
        let size = if name == "prebuilt-scaled20" {
            " at20pt"
        } else {
            ""
        };
        let source=format!("\\documentclass{{article}}\n\\pagestyle{{empty}}\n{settings}\n\\font\\f=pitexbitmap{size}\\pdffontattr\\f{{/AuditBitmap (PKAttribute)}}{suppression}\\edef\\object{{\\pdffontobjnum\\f}}\\pdfcatalog{{/AuditBitmap \\object\\space 0 R}}\\pdfincludechars\\f{{Z}}\\edef\\resource{{\\pdffontname\\f}}\n\\begin{{document}}\n\\noindent BitmapMarkerOriginal\\typeout{{BITMAP-RESOURCE=\\resource}}\\par\n\\vspace{{150bp}}\\noindent\\f AAAAA\n\\end{{document}}\n");
        let main = root.join("main.tex");
        fs::write(&main, &source)?;
        let reference = root.join("reference");
        fs::create_dir(&reference)?;
        let result = output_timeout(
            Command::new("pdflatex")
                .args([
                    "-interaction=nonstopmode",
                    "-halt-on-error",
                    "-no-shell-escape",
                    "-no-mktex=pk",
                ])
                .env("MKTEXPK", "0")
                .env(
                    "TEXPKS",
                    if mode == "auditmode" {
                        format!("{}//", root.join("auditmode").display())
                    } else {
                        format!("{}//", root.display())
                    },
                )
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
            let marker = format!("BitmapMarker{generation}");
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":source.replace("BitmapMarkerOriginal",&marker)}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 {
                return Err(format!("native {name}: {event}").into());
            }
            let pdf = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
            if let Some(evidence) = std::env::var_os("PITEX_BITMAP_EVIDENCE") {
                let evidence = PathBuf::from(evidence).join(format!("{name}-{generation}"));
                fs::create_dir_all(&evidence)?;
                fs::copy(pdf, evidence.join("native.pdf"))?;
                fs::copy(reference.join("main.pdf"), evidence.join("reference.pdf"))?;
                fs::copy(&main, evidence.join("main.tex"))?;
            }
            let font = show(pdf, "trailer/Root/AuditBitmap")?;
            if !font.contains("/Subtype /Type3")
                || !font.contains("PKAttribute")
                || !font.contains("/a90")
                || !font.contains("/PitexPKResolution")
            {
                return Err(format!("bitmap dictionary incomplete: {font}").into());
            }
            let dpi_value = regex::Regex::new(r"/PitexPKResolution\s*\[\s*([0-9.]+)")?
                .captures(&font)
                .ok_or("PK resolution metadata missing")?[1]
                .parse::<f64>()?;
            if (dpi_value - actual as f64).abs() > 1. {
                return Err(format!("wrong PK resolution {dpi_value}, expected {actual}").into());
            }
            let charset = show(pdf, "trailer/Root/AuditBitmap/FontDescriptor")?;
            if charset.contains("/CharSet") == omit {
                return Err("bitmap charset policy ignored".into());
            }
            if font.contains("/ToUnicode") == no_unicode {
                return Err("bitmap Unicode policy ignored".into());
            }
            let log = fs::read_to_string(event["log"].as_str().ok_or("missing log")?)?;
            let resource = log
                .lines()
                .find_map(|s| s.strip_prefix("BITMAP-RESOURCE="))
                .ok_or("font resource query missing")?;
            if !show(pdf, "pages/1/Resources/Font")?.contains(&format!("/F{resource} ")) {
                return Err("bitmap resource id mismatched".into());
            }
            let text = output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
            if !String::from_utf8(text.stdout)?.contains(&marker) {
                return Err("unsaved marker missing".into());
            }
            compare(pdf, &reference.join("main.pdf"))?;
            if fs::read_to_string(&main)? != source {
                return Err("saved source changed".into());
            }
        }
        preview_regression::pass(
            name,
            Some("pdftex"),
            &[&source],
            "native Type 3 metadata/raster and two unsaved generations",
        )?;
    }
    Ok(())
}
fn main() {
    pitex_native_tools::exit_on_error(run());
}
