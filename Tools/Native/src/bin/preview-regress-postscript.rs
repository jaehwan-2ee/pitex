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
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN is required")?)?;
    let temporary = TempDir::new("pitex-ps-regression-")?;
    let work = fs::canonicalize(temporary.path())?;
    let cases = [
        (
            "pstricks-frame-circle",
            r"\usepackage{pstricks}",
            r"\begin{pspicture}(0,0)(4,2)\psframe[fillstyle=solid,fillcolor=red](0,0)(4,2)\pscircle(2,1){.5}\end{pspicture}",
        ),
        (
            "pstricks-bezier-polygon",
            r"\usepackage{pstricks}",
            r"\begin{pspicture}(0,0)(4,3)\pspolygon[linecolor=blue,fillstyle=solid,fillcolor=yellow](0,0)(2,2)(4,0)\psbezier[linecolor=red](0,1)(1,3)(3,0)(4,2)\end{pspicture}",
        ),
        (
            "pstricks-dots",
            r"\usepackage{pstricks}",
            r"\begin{pspicture}(0,0)(4,2)\psdots[dotstyle=*,dotsize=6pt](1,1)(2,1)(3,1)\end{pspicture}",
        ),
        (
            "pstricks-arrow-dash",
            r"\usepackage{pstricks}",
            r"\begin{pspicture}(0,0)(4,2)\psline[linestyle=dashed,linecolor=blue]{->}(0,0)(4,2)\end{pspicture}",
        ),
        (
            "pstricks-roundrect",
            r"\usepackage{pstricks}",
            r"\begin{pspicture}(0,0)(4,2)\psframe[framearc=.3,fillstyle=solid,fillcolor=yellow](0,0)(4,2)\end{pspicture}",
        ),
        (
            "pstricks-plot",
            r"\usepackage{pst-plot}",
            r"\begin{pspicture}(0,0)(4,2)\psplot[linecolor=blue]{0}{4}{x 20 mul sin}\end{pspicture}",
        ),
        (
            "pstricks-node",
            r"\usepackage{pst-node}",
            r"\begin{pspicture}(0,0)(4,2)\rnode{A}{First}\hspace{1cm}\rnode{B}{Next}\ncline{->}{A}{B}\end{pspicture}",
        ),
        (
            "pstricks-rotation",
            r"\usepackage{pstricks}",
            r"\begin{pspicture}(0,0)(4,3)\rput{30}(2,1){\psframe[fillstyle=solid,fillcolor=yellow](0,0)(1,1)}\end{pspicture}",
        ),
        (
            "postscript-literal",
            r"\usepackage{pstricks}",
            r"\begin{pspicture}(0,0)(4,2)\pstVerb{/AuditSize 20 def}\pstverb{0 0 1 setrgbcolor 0 0 AuditSize AuditSize rectfill 1 0 0 setrgbcolor 70 20 12 0 360 arc stroke}\end{pspicture}",
        ),
        (
            "postscript-text",
            r"\usepackage{pstricks}",
            r"\begin{pspicture}(0,0)(4,2)\pstverb{/Times-Roman findfont 14 scalefont setfont 5 20 moveto (Preview) show}\end{pspicture}",
        ),
        ("postscript-gray-image", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave 40 40 scale 2 2 8 [2 0 0 -2 0 2] {<004080FF>} image grestore}\end{pspicture}"),
        ("postscript-color-image", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave 40 40 scale 2 2 8 [2 0 0 -2 0 2] {<FF000000FF000000FFFFFFFF>} false 3 colorimage grestore}\end{pspicture}"),
        ("postscript-planar-image", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave 40 40 scale 2 2 8 [2 0 0 -2 0 2] {<FF0000FF>} {<00FF00FF>} {<0000FFFF>} true 3 colorimage grestore}\end{pspicture}"),
        ("postscript-image-mask", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave 0 0 1 setrgbcolor 40 40 scale 8 2 true [8 0 0 -2 0 2] {<AA55>} imagemask grestore}\end{pspicture}"),
        ("postscript-filter-image", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave 40 40 scale 2 2 8 [2 0 0 -2 0 2] (004080FF>) /ASCIIHexDecode filter image grestore}\end{pspicture}"),
        ("postscript-currentfile-image", r"\usepackage{graphicx}", r"\includegraphics[bb=0 0 40 40,width=4cm]{image-currentfile.eps}"),
        ("postscript-dct-image", r"\usepackage{graphicx}", r"\includegraphics[bb=0 0 40 40,width=4cm]{image-dct.eps}"),
        ("postscript-dictionary-image", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave 40 40 scale /DeviceRGB setcolorspace << /ImageType 1 /Width 2 /Height 2 /BitsPerComponent 8 /ImageMatrix [2 0 0 -2 0 2] /Decode [0 1 0 1 0 1] /DataSource <FF000000FF000000FFFFFFFF> >> image grestore}\end{pspicture}"),
        ("postscript-axial-shading", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave 0 0 40 40 rectclip << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 40 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >> /Extend [true true] >> shfill grestore}\end{pspicture}"),
        ("postscript-radial-shading", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave 0 0 40 40 rectclip << /ShadingType 3 /ColorSpace /DeviceRGB /Coords [20 20 0 20 20 25] /Function << /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [0 0 1] /N 1 >> /Extend [true true] >> shfill grestore}\end{pspicture}"),
        ("postscript-function-shading", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave << /ShadingType 1 /ColorSpace /DeviceRGB /Domain [0 1 0 1] /Matrix [40 0 0 40 0 0] /Function << /FunctionType 0 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] /Size [2 2] /BitsPerSample 8 /DataSource <FF000000FF000000FFFFFFFF> >> >> shfill grestore}\end{pspicture}"),
        ("postscript-triangle-shading", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave << /ShadingType 4 /ColorSpace /DeviceRGB /BitsPerCoordinate 8 /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 40 0 40 0 1 0 1 0 1] /DataSource <000000FF000000FF0000FF000000FF0000FF> >> shfill grestore}\end{pspicture}"),
        ("postscript-lattice-shading", r"\usepackage{pstricks}", r"\begin{pspicture}(0,0)(4,2)\pstverb{gsave << /ShadingType 5 /ColorSpace /DeviceRGB /BitsPerCoordinate 8 /BitsPerComponent 8 /VerticesPerRow 2 /Decode [0 40 0 40 0 1 0 1 0 1] /DataSource <0000FF0000FF0000FF0000FF0000FFFFFFFFFFFFFF> >> shfill grestore}\end{pspicture}"),
        ("postscript-coons-shading", r"\usepackage{pstricks}", "GENERATED-MESH-6"),
        ("postscript-tensor-shading", r"\usepackage{pstricks}", "GENERATED-MESH-7"),
    ];
    for (name, preamble, body) in cases {
        preview_regression::check_interrupt()?;
        let root = work.join(name);
        fs::create_dir(&root)?;
        fs::write(root.join("image-currentfile.eps"), b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 40 40\n40 40 scale\n2 2 8 [2 0 0 -2 0 2] {currentfile 4 string readhexstring pop} image\n004080FF\nshowpage\n")?;
        let jpeg = include_bytes!("../../fixtures/postscript-dct.jpg");
        let encoded = jpeg.iter().map(|b| format!("{b:02X}")).collect::<String>();
        fs::write(root.join("image-dct.eps"), format!("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 40 40\n40 40 scale\n/DeviceRGB setcolorspace\n<< /ImageType 1 /Width 2 /Height 2 /BitsPerComponent 8 /ImageMatrix [2 0 0 -2 0 2] /Decode [0 1 0 1 0 1] /DataSource <{encoded}> /DCTDecode filter >> image\nshowpage\n"))?;
        let out = root.join("preview");
        let reference = root.join("reference");
        fs::create_dir(&out)?;
        fs::create_dir(&reference)?;
        fs::copy(root.join("image-currentfile.eps"), reference.join("image-currentfile.eps"))?;
        fs::copy(root.join("image-dct.eps"), reference.join("image-dct.eps"))?;
        let main = root.join("main.tex");
        let mut mesh = String::new();
        let body = if let Some(kind) = body.strip_prefix("GENERATED-MESH-") {
            let mut data=vec![0u8];
            for (x,y) in [(0,0),(85,0),(170,0),(255,0),(255,85),(255,170),(255,255),(170,255),(85,255),(0,255),(0,170),(0,85)] {data.extend([x,y]);}
            if kind=="7" {for (x,y) in [(85,85),(170,85),(170,170),(85,170)] {data.extend([x,y]);}}
            data.extend([255,0,0,0,255,0,0,0,255,255,255,255]);
            let hex=data.iter().map(|b|format!("{b:02X}")).collect::<String>();
            mesh=format!(r"\begin{{pspicture}}(0,0)(4,2)\pstverb{{gsave << /ShadingType {kind} /ColorSpace /DeviceRGB /BitsPerCoordinate 8 /BitsPerComponent 8 /BitsPerFlag 8 /Decode [0 40 0 40 0 1 0 1 0 1] /DataSource <{hex}> >> shfill grestore}}\end{{pspicture}}");
            mesh.as_str()
        } else {body};
        let source=format!("\\documentclass{{article}}\n\\usepackage[paperwidth=12cm,paperheight=12cm,margin=12mm]{{geometry}}\n\\pagestyle{{empty}}\n{preamble}\n\\begin{{document}}\nMarker0.\n\n{body}\n\\end{{document}}\n");
        fs::write(&main, &source)?;
        let mut session = Session::spawn(
            Command::new(bin.join("pitex-preview"))
                .arg("--root")
                .arg(&root)
                .arg("--main")
                .arg("main.tex")
                .arg("--out")
                .arg(&out)
                .arg("--cache")
                .arg(work.join("cache"))
                .stderr(Stdio::null()),
            &out,
        )?;
        for generation in [1, 2] {
            let text = source.replace("Marker0.", &format!("Marker{generation}."));
            fs::write(reference.join("main.tex"), &text)?;
            let compiled = output_timeout(
                Command::new("xelatex")
                    .args([
                        "-interaction=nonstopmode",
                        "-halt-on-error",
                        "-no-shell-escape",
                    ])
                    .arg("main.tex")
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
                || event
                    .get("warnings")
                    .and_then(|w| w.as_str())
                    .is_some_and(|w| w.contains("PostScript"))
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
            let ink = |pixels: &[u8]| {
                pixels
                    .chunks_exact(3)
                    .filter(|p| p.iter().any(|v| *v < 245))
                    .count()
            };
            let expected_ink = ink(expected);
            let actual_ink = ink(actual);
            if name.contains("image") || name.contains("shading") {if expected_ink<500{return Err(format!("{name}: reference drew no useful image ({expected_ink} pixels)").into());}}
            let ink_error = expected_ink.abs_diff(actual_ink);
            if ink_error > 12 + expected_ink / 12 {
                return Err(format!("{name}: drawing coverage differs: expected {expected_ink}, actual {actual_ink} pixels").into());
            }
            // Both engines see identical unsaved bytes. The bounds allow raster
            // antialiasing while rejecting lost paths, glyphs or transforms.
            println!("RASTER: {name} generation{generation} mean={mean:.5}");
            if mean > 0.1 {
                return Err(
                    format!("{name}: mean absolute raster error {mean:.4} exceeds0.1").into(),
                );
            }
        }
        if fs::read_to_string(&main)? != source {
            return Err(format!("{name}: preview modified project source").into());
        }
        println!("PASS: {name}, reference pixels and two unsaved generations");
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1)
    }
}
