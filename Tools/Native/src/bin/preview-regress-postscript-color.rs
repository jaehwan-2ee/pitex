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
    let temporary = TempDir::new("pitex-ps-color-")?;
    let work = fs::canonicalize(temporary.path())?;
    if std::env::var_os("PITEX_COLOR_DEBUG").is_some() {
        eprintln!("artifacts: {}", work.display());
        std::mem::forget(temporary);
    }
    let cases = [
        (
            "mask-separate",
            r#"1 1 0 setrgbcolor 0 0 40 40 rectfill /DeviceRGB setcolorspace gsave 40 40 scale << /ImageType 3 /InterleaveType 3 /DataDict << /ImageType 1 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /DataSource <ff000000ff000000ffffffff> >> /MaskDict << /ImageType 1 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 1 /Decode [0 1] /DataSource <8040> >> >> image grestore"#,
        ),
        (
            "mask-sample",
            r#"1 1 0 setrgbcolor 0 0 40 40 rectfill /DeviceRGB setcolorspace gsave 40 40 scale << /ImageType 3 /InterleaveType 1 /DataDict << /ImageType 1 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /DataSource <FFFF00000000FF00000000FFFFFFFFFF> >> /MaskDict << /ImageType 1 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 8 /Decode [0 1] >> >> image grestore"#,
        ),
        (
            "mask-row",
            r#"1 1 0 setrgbcolor 0 0 40 40 rectfill /DeviceRGB setcolorspace gsave 40 40 scale << /ImageType 3 /InterleaveType 2 /DataDict << /ImageType 1 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /DataSource <80FF000000FF00400000FFFFFFFF> >> /MaskDict << /ImageType 1 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 1 /Decode [0 1] >> >> image grestore"#,
        ),
        (
            "mask-resolution",
            r#"1 1 0 setrgbcolor 0 0 40 40 rectfill /DeviceRGB setcolorspace gsave 40 40 scale << /ImageType 3 /InterleaveType 3 /DataDict << /ImageType 1 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /DataSource <ff000000ff000000ffffffff> >> /MaskDict << /ImageType 1 /Width 4 /Height 4 /BitsPerComponent 1 /ImageMatrix [4 0 0 -4 0 4] /Decode [1 0] /DataSource <A050A050> >> >> image grestore"#,
        ),
        (
            "key-exact-before-decode",
            r#"1 1 0 setrgbcolor 0 0 40 40 rectfill /DeviceRGB setcolorspace gsave 40 40 scale << /ImageType 4 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 8 /MaskColor [255 0 0] /Decode [1 0 1 0 1 0] /DataSource <ff000000ff000000ffffffff> >> image grestore"#,
        ),
        (
            "key-ranges",
            r#"1 1 0 setrgbcolor 0 0 40 40 rectfill /DeviceRGB setcolorspace gsave 40 40 scale << /ImageType 4 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /MaskColor [240 255 0 10 0 10] /DataSource <ff000000ff000000ffffffff> >> image grestore"#,
        ),
        (
            "indexed-string",
            r#"[/Indexed /DeviceRGB 3 <FF000000FF000000FFFFFFFF>] setcolorspace gsave 40 40 scale << /ImageType 1 /Width 2 /Height 2 /BitsPerComponent 8 /ImageMatrix [2 0 0 -2 0 2] /Decode [0 255] /DataSource <00010203> >> image grestore"#,
        ),
        (
            "indexed-procedure",
            r#"[/Indexed /DeviceRGB 3 {dup 1 eq {pop 0 1 0} {dup 2 eq {pop 0 0 1} {0 eq {1 0 0} {1 1 1} ifelse} ifelse} ifelse}] setcolorspace gsave 40 40 scale << /ImageType 1 /Width 2 /Height 2 /BitsPerComponent 2 /ImageMatrix [2 0 0 -2 0 2] /Decode [0 3] /DataSource <10B0> >> image grestore"#,
        ),
        (
            "cie-a-solid",
            r#"[/CIEBasedA << /WhitePoint [.9505 1 1.089] /MatrixA [.9505 1 1.089] /RangeLMN [0 .9505 0 1 0 1.089] /DecodeA {2.2 exp} >>] setcolorspace .5 setcolor 0 0 40 40 rectfill"#,
        ),
        (
            "cie-abc-image",
            r#"[/CIEBasedABC << /WhitePoint [.95047 1 1.08883] /DecodeABC [{2.2 exp} {2.2 exp} {2.2 exp}] /MatrixLMN [.4124564 .2126729 .0193339 .3575761 .7151522 .1191920 .1804375 .0721750 .9503041] >>] setcolorspace gsave 40 40 scale << /ImageType 1 /Width 2 /Height 2 /ImageMatrix [2 0 0 -2 0 2] /BitsPerComponent 8 /Decode [0 1 0 1 0 1] /DataSource <ff000000ff000000ffffffff> >> image grestore"#,
        ),
        (
            "separation-procedure",
            r#"[/Separation /AuditSpot /DeviceRGB {dup 0 exch 1 exch sub}] setcolorspace .75 setcolor 0 0 40 40 rectfill"#,
        ),
        (
            "devicen-procedure",
            r#"[/DeviceN [/AuditA /AuditB] /DeviceRGB {0}] setcolorspace .25 .75 setcolor 0 0 40 40 rectfill"#,
        ),
        (
            "plane-shared-file",
            r#"/DeviceRGB setcolorspace gsave 40 40 scale /f (FF000000FF00>) /ASCIIHexDecode filter def 1 2 8 [1 0 0 -2 0 2] {f 1 string readstring pop} {f 1 string readstring pop} {f 1 string readstring pop} true 3 colorimage grestore"#,
        ),
        (
            "space-resets-color",
            r#"1 0 0 setrgbcolor /DeviceGray setcolorspace 0 0 40 40 rectfill"#,
        ),
        (
            "image-alpha",
            r#".25 .setopacityalpha gsave 40 40 scale 1 1 8 [1 0 0 -1 0 1] <FF0000> false 3 colorimage grestore"#,
        ),
        (
            "image-12bit",
            r#"gsave 40 40 scale 2 2 12 [2 0 0 -2 0 2] <000800FFF400> image grestore"#,
        ),
        (
            "key-12bit",
            r#"/DeviceGray setcolorspace 1 1 0 setrgbcolor 0 0 40 40 rectfill /DeviceGray setcolorspace gsave 40 40 scale << /ImageType 4 /Width 2 /Height 2 /BitsPerComponent 12 /ImageMatrix [2 0 0 -2 0 2] /Decode [0 1] /MaskColor [4095] /DataSource <000800FFF400> >> image grestore"#,
        ),
        (
            "cie-def-table",
            r#"[/CIEBasedDEF << /WhitePoint [.95047 1 1.08883] /MatrixLMN [.4124564 .2126729 .0193339 .3575761 .7151522 .1191920 .1804375 .0721750 .9503041] /Table [2 2 2 [<0000000000FF00FF0000FFFF> <FF0000FF00FFFFFF00FFFFFF>]] >>] setcolorspace .25 .5 .75 setcolor 0 0 40 40 rectfill"#,
        ),
        (
            "cie-defg-table",
            r#"[/CIEBasedDEFG << /WhitePoint [.95047 1 1.08883] /MatrixLMN [.4124564 .2126729 .0193339 .3575761 .7151522 .1191920 .1804375 .0721750 .9503041] /Table [2 2 2 2 [[<0000000000000000FF0000FF> <00FF0000FF0000FFFF00FFFF>] [<FF0000FF0000FF00FFFF00FF> <FFFF00FFFF00FFFFFFFFFFFF>]]] >>] setcolorspace .25 .5 .75 .5 setcolor 0 0 40 40 rectfill"#,
        ),
        (
            "cie-shading-a",
            r#"<< /ShadingType 2 /BBox [0 0 40 40] /ColorSpace [/CIEBasedA << /WhitePoint [.9505 1 1.089] /MatrixA [.9505 1 1.089] /RangeLMN [0 .9505 0 1 0 1.089] /DecodeA {2.2 exp} >>] /Coords [0 0 40 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> /Extend [true true] >> shfill"#,
        ),
        (
            "cie-shading-range",
            r#"<< /ShadingType 2 /BBox [0 0 40 40] /ColorSpace [/CIEBasedA << /WhitePoint [.9505 1 1.089] /MatrixA [.9505 1 1.089] /RangeLMN [0 .9505 0 1 0 1.089] /RangeA [0 100] /DecodeA {100 div 2.2 exp} >>] /Coords [0 0 40 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [100] /N 1 >> /Extend [true true] >> shfill"#,
        ),
        (
            "tint-shading-procedure",
            r#"<< /ShadingType 2 /BBox [0 0 40 40] /ColorSpace [/Separation /AuditTint /DeviceRGB {dup 0 exch 0}] /Coords [0 0 40 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> /Extend [true true] >> shfill"#,
        ),
        (
            "indexed-high-12bit",
            r#"[/Indexed /DeviceRGB 256 {256 div 0 0}] setcolorspace gsave 40 40 scale << /ImageType 1 /Width 2 /Height 1 /BitsPerComponent 12 /ImageMatrix [2 0 0 -1 0 1] /Decode [0 4095] /DataSource <000100> >> image grestore"#,
        ),
        ("cie-shading-cubic-range",r#"<< /ShadingType 2 /BBox [0 0 40 40] /ColorSpace [/CIEBasedA << /WhitePoint [.9505 1 1.089] /MatrixA [.9505 1 1.089] /RangeLMN [0 .9505 0 1 0 1.089] /RangeA [0 100] /DecodeA {100 div 2.2 exp} >>] /Coords [0 0 40 0] /Function << /FunctionType 0 /Domain [0 1] /Range [0 100] /Size [4] /BitsPerSample 8 /Order 3 /Encode [0 3] /Decode [0 100] /DataSource <0055AAFF> >> /Extend [true true] >> shfill"#),
    ];
    for (name, program) in cases {
        if let Ok(selected) = std::env::var("PITEX_COLOR_CASE") {
            if !name.contains(&selected) {
                continue;
            }
        }
        let root = work.join(name);
        fs::create_dir(&root)?;
        let out = root.join("preview");
        let reference = root.join("reference");
        fs::create_dir(&out)?;
        fs::create_dir(&reference)?;
        let eps =
            format!("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 40 40\n{program}\nshowpage\n");
        fs::write(root.join("image.eps"), &eps)?;
        let conversion = output_timeout(
            Command::new("gs")
                .args([
                    "-q",
                    "-dBATCH",
                    "-dNOPAUSE",
                    "-dALLOWPSTRANSPARENCY",
                    "-dEPSCrop",
                    "-sDEVICE=pdfwrite",
                ])
                .arg(format!(
                    "-sOutputFile={}",
                    reference.join("image.pdf").display()
                ))
                .arg(root.join("image.eps")),
            TIMEOUT,
        )?;
        if !conversion.status.success() {
            return Err(format!(
                "{name}: oracle failed {} {}",
                String::from_utf8_lossy(&conversion.stdout),
                String::from_utf8_lossy(&conversion.stderr)
            )
            .into());
        }
        let source = r"\documentclass{article}\usepackage[paperwidth=12cm,paperheight=12cm,margin=12mm]{geometry}\usepackage{graphicx}\pagestyle{empty}\begin{document}ColorMarker0.\par\includegraphics[bb=0 0 40 40,width=4cm]{image.eps}\end{document}";
        // Smaller 12-bit fixtures avoid a renderer-dependent one-pixel crop edge;
        // interior samples and the emitted 16-bit structure are checked as well.
        let source = if name.contains("12bit") || name == "plane-shared-file" {
            source.replace("width=4cm", "width=3cm")
        } else {
            source.to_string()
        };
        let main = root.join("main.tex");
        fs::write(&main, &source)?;
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
        for generation in [1, 2] {
            let text = source.replace("ColorMarker0", &format!("ColorMarker{generation}"));
            fs::write(
                reference.join("main.tex"),
                text.replace("bb=0 0 40 40,", "")
                    .replace("image.eps", "image.pdf"),
            )?;
            let compiled = output_timeout(
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
            if !compiled.status.success() {
                return Err(format!(
                    "{name}: reference failed {}",
                    String::from_utf8_lossy(&compiled.stdout)
                )
                .into());
            }
            let expected = raster(
                &reference.join("main.pdf"),
                &root.join(format!("reference{generation}")),
            )?;
            let expected = pixel_data(&expected)?;
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":text}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0
                || event["warnings"]
                    .as_str()
                    .unwrap_or("")
                    .contains("PostScript")
            {
                return Err(format!("{name}: {event}").into());
            }
            let actual = raster(
                Path::new(event["pdf"].as_str().ok_or("missing PDF")?),
                &root.join(format!("actual{generation}")),
            )?;
            let actual = pixel_data(&actual)?;
            if actual.len() != expected.len() {
                return Err(format!("{name}: page dimensions differ").into());
            }
            let mean = actual
                .iter()
                .zip(expected)
                .map(|(a, b)| (*a as i16 - *b as i16).unsigned_abs() as u64)
                .sum::<u64>() as f64
                / actual.len() as f64;
            let bounded_mean = if name.starts_with("cie-") {
                actual
                    .iter()
                    .zip(expected)
                    .map(|(a, b)| ((*a as i16 - *b as i16).unsigned_abs() as u64).saturating_sub(1))
                    .sum::<u64>() as f64
                    / actual.len() as f64
            } else {
                mean
            };
            if bounded_mean > 0.1 {
                return Err(format!("{name}: raster error {mean:.5} exceeds 0.1").into());
            }
            println!("PASS {name} generation{generation}, raster mean={mean:.5}");
        }
        if fs::read_to_string(&main)? != source {
            return Err("preview altered saved source".into());
        }
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
