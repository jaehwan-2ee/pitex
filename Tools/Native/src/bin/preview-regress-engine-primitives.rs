//! Structural and rendering checks for the embedded PDF primitives and expansion.
//! PITEX_BIN points to the current helper pair. Reference compilers and mutool
//! are test tools only; the embedded preview never launches them.
use pitex_native_tools::{
    preview_regression::{self, output_timeout, Session},
    TempDir,
};
use regex::Regex;
use serde_json::json;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
const TIMEOUT: Duration = Duration::from_secs(60);

fn show(pdf: &Path, selector: &str) -> Result<String, Box<dyn Error>> {
    let output = output_timeout(
        Command::new("mutool").args(["show"]).arg(pdf).arg(selector),
        TIMEOUT,
    )?;
    if !output.status.success() {
        return Err(format!("mutool failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}
fn reference(root: &Path, compiler: &str) -> Result<PathBuf, Box<dyn Error>> {
    let directory = root.join("reference");
    fs::create_dir(&directory)?;
    for _ in 0..2 {
        let output = output_timeout(
            Command::new(compiler)
                .args([
                    "-interaction=nonstopmode",
                    "-halt-on-error",
                    "-no-shell-escape",
                ])
                .arg(format!("-output-directory={}", directory.display()))
                .arg("main.tex")
                .current_dir(root),
            TIMEOUT,
        )?;
        if !output.status.success() {
            return Err(format!(
                "reference failed: {}",
                String::from_utf8_lossy(&output.stdout)
            )
            .into());
        }
    }
    Ok(directory.join("main.pdf"))
}
fn glyph_scales(contents: &str) -> Vec<f64> {
    let matrix =
        Regex::new(r"([-+0-9.]+)\s+0\s+[-+0-9.]+\s+1\s+[-+0-9.]+\s+[-+0-9.]+\s+Tm").unwrap();
    matrix
        .captures_iter(contents)
        .filter_map(|m| m[1].parse().ok())
        .collect()
}
fn text(pdf: &Path) -> Result<String, Box<dyn Error>> {
    let output = output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
    if !output.status.success() {
        return Err("pdftotext failed".into());
    }
    Ok(String::from_utf8(output.stdout)?)
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let binary = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN is required")?)?;
    let temporary = TempDir::new("pitex-engine-primitives-")?;
    let work = fs::canonicalize(temporary.path())?;
    let cache = work.join("cache");
    // An old-format file is retained during automatic format36 bootstrapping.
    fs::create_dir_all(cache.join("format"))?;
    let old_format = cache.join("format/texlive-pitex-34-xelatex.ini.fmt");
    fs::write(&old_format, b"retained format34 cache")?;
    let cases = [
        (
            "aliases",
            "",
            r"\typeout{AUDIT-STRCMP=\pdfstrcmp{same}{same}}\typeout{AUDIT-MD5=\pdfmdfivesum{abc}}\typeout{AUDIT-SIZE=\pdffilesize{probe.dat}}\typeout{AUDIT-DUMP=\pdffiledump offset1 length3{probe.dat}}\pdfsetrandomseed12345\relax\typeout{AUDIT-SEED=\the\pdfrandomseed}\ifpdfabsnum-4>3\typeout{AUDIT-ABSNUM=YES}\fi\ifpdfabsdim-2pt=2pt\typeout{AUDIT-ABSDIM=YES}\fi\edef\saved{\pdfuniformdeviate1000}\typeout{AUDIT-RANDOM=\saved}\def\pdfstrcmp{redefined}\edef\original{\pdfprimitive\pdfstrcmp{same}{same}}\typeout{AUDIT-ORIGINAL=\original}Aliases.",
            Some("pdflatex"),
        ),
        (
            "color-stacks",
            r"\newcount\stackcount\stackcount=0 \loop\edef\colorid{\pdfcolorstackinit page direct{0 g 0 G}}\advance\stackcount by1 \ifnum\stackcount<20 \repeat",
            r"\pdfcolorstack\colorid push{1 0 0 rg 1 0 0 RG}\rule{20pt}{10pt}\pdfcolorstack\colorid pop\typeout{AUDIT-COLORID=\colorid}Color stacks.",
            Some("pdflatex"),
        ),
        (
            "font-map-line",
            r#"\pdfmapline{=cmr10 CMR10 "0.3 SlantFont" <cmr10.pfb}"#,
            r"\font\mapped=cmr10 \mapped MapProbe.",
            Some("pdflatex"),
        ),
        (
            "font-map-file",
            r"\pdfmapfile{=custom.map}",
            r"\font\mapped=cmr10 \mapped MapProbe.",
            Some("pdflatex"),
        ),
        (
            "page-parameters",
            "",
            r"{\pdfhorigin=5pt\pdfminorversion=4\pdfpageattr{/Rotate 90}\typeout{AUDIT-IN-GROUP=\the\pdfhorigin,\the\pdfminorversion}}\typeout{AUDIT-AFTER-GROUP=\the\pdfhorigin,\the\pdfminorversion}\typeout{AUDIT-EMPTY-ATTR=\the\pdfpageattr}\def\attributevalue{ExpandedValue}\pdfpageattr{/AuditKey (\attributevalue)}\pdfpageresources{/Properties << /AuditProperty << /Tag (PageResources) >> >>}Parameters.",
            Some("pdflatex"),
        ),
        (
            "output-options",
            r"\pdfmajorversion=2 \pdfminorversion=0 \pdfcompresslevel=0 ",
            r"\pdftrailer{/AuditTrailer (Works)}\pdftrailerid{[<01020304><01020304>]}\pdfnames{/AuditNames (Works)}Output options.",
            Some("pdflatex"),
        ),
        (
            "version-14",
            r"\pdfminorversion=4 \font\auditfont=cmr10 \AtBeginDocument{\auditfont}",
            "PDF version control.",
            Some("pdflatex"),
        ),
        (
            "version-native",
            r"\pdfminorversion=4 \usepackage{fontspec}\setmainfont{Latin Modern Roman}",
            "Native PDF version requirement.",
            None,
        ),
        (
            "literal",
            "",
            r"\pdfliteral direct {1 0 0 rg 0 0 80 30 re f}Direct literal.",
            Some("pdflatex"),
        ),
        (
            "objects",
            "",
            r"\setbox0=\hbox{\immediate\pdfobj{<< /AuditKey (DiscardedBoxObject) >>}}\typeout{AUDIT-FIRST=\the\pdflastobj}\pdfobj reserveobjnum\typeout{AUDIT-RESERVE=\the\pdflastobj}\immediate\pdfobj useobjnum 2 {<< /AuditKey (ReservedObject) >>}\pdfcatalog{/PitexAudit 2 0 R}\pdfinfo{/Title (Primitive Title) /Author (Primitive Author)}Object body.",
            Some("pdflatex"),
        ),
        (
            "links",
            "",
            r"\leavevmode\pdfdest name {target} xyz \pdfstartlink attr{/Border[0 0 0]} user{/A << /S /URI /URI (https://example.org) >>}External link\pdfendlink.\pdfannot width20pt height10pt depth0pt {/Subtype /Text /Contents (Primitive annotation)}",
            Some("pdflatex"),
        ),
        (
            "matrix",
            "",
            r"\leavevmode\pdfsave\pdfsetmatrix{1.5 0 0 1}\hbox{MatrixScaled}\pdfrestore Restored.",
            Some("pdflatex"),
        ),
        (
            "forms",
            "",
            r"\setbox0=\hbox{\textbf{ReusableForm}\kern5pt\vrule width10pt height10pt}\immediate\pdfxform attr{/AuditForm (Works)} 0\relax\ifvoid0\typeout{AUDIT-BOX=EMPTY}\fi\typeout{AUDIT-FORM=\the\pdflastxform}\leavevmode\pdfrefxform\pdflastxform\quad\pdfrefxform\pdflastxform",
            Some("pdflatex"),
        ),
        (
            "images",
            "",
            r"\pdfximage width40pt height30pt depth5pt page1 {example-image-a.pdf}\typeout{AUDIT-IMAGE=\the\pdflastximage}\typeout{AUDIT-IMAGE-PAGES=\the\pdflastximagepages}\leavevmode\pdfrefximage\pdflastximage\quad\pdfrefximage\pdflastximage",
            Some("pdflatex"),
        ),
        (
            "outline",
            "",
            r"\pdfoutline goto name {parent} count1 {Parent}\pdfoutline goto name {child} {Child}\leavevmode\pdfdest name {parent} xyz Parent.\pdfdest name {child} fit Child.",
            Some("pdflatex"),
        ),
        (
            "escaping",
            "",
            r"\edef\escaped{\pdfescapestring{a(b)}}\edef\named{\pdfescapename{a b/c}}\edef\hexed{\pdfescapehex{A B}}\edef\unhexed{\pdfunescapehex{4142}}\typeout{AUDIT-STRING=\meaning\escaped}\typeout{AUDIT-NAME=\meaning\named}\typeout{AUDIT-HEX=\hexed}\typeout{AUDIT-UNHEX=\unhexed}\unhexed.",
            Some("pdflatex"),
        ),
        (
            "ignore",
            "",
            r"\ignoreprimitiveerror=0 {\ignoreprimitiveerror=1\setbox0=\vbox{\hrule\vskip10pt minus1fil\hrule}\setbox1=\vsplit0 to5pt}\typeout{AUDIT-IGNORE-GROUP=\the\ignoreprimitiveerror}Ignored shrink diagnostic.",
            None,
        ),
        (
            "expansion",
            r"\usepackage[expansion=true,protrusion=false]{microtype}",
            r"\noindent\parbox{4cm}{A longer paragraph with several words to test font expansion and optical protrusion in the output.}",
            Some("pdflatex"),
        ),
        (
            "expansion-eligibility",
            r"\PitexFontExpansion=1 \PitexFontStretch=20 \PitexFontShrink=20 \PitexFontStep=1",
            r"\noindent\parbox{4cm}{\hbox{FixedBox}\quad$x+y=z$\quad\hbox{OtherBox}\quad\hbox{LastBox}}",
            None,
        ),
        (
            "native-expansion",
            r"\usepackage{fontspec}\setmainfont{Latin Modern Roman}\PitexFontExpansion=1 \PitexFontStretch=20 \PitexFontShrink=20 \PitexFontStep=1",
            r"\noindent\parbox{4cm}{A longer paragraph with several words to test font expansion and optical protrusion in the output.}",
            None,
        ),
    ];
    for (name, preamble, body, compiler) in cases {
        preview_regression::check_interrupt()?;
        let root = work.join(name);
        fs::create_dir(&root)?;
        let out = root.join("preview");
        fs::create_dir(&out)?;
        let main = root.join("main.tex");
        let source = format!("\\documentclass{{article}}\n\\usepackage[paperwidth=12cm,paperheight=12cm,margin=12mm]{{geometry}}\n\\pagestyle{{empty}}\n{preamble}\n\\begin{{document}}\nPrimitiveMarker0.\n\n{body}\n\\end{{document}}\n");
        fs::write(root.join("probe.dat"), b"abcdef\n")?;
        fs::write(
            root.join("custom.map"),
            b"cmr10 CMR10 \"0.3 SlantFont\" <cmr10.pfb\n",
        )?;
        fs::write(&main, &source)?;
        let reference_pdf = compiler
            .map(|compiler| reference(&root, compiler))
            .transpose()?;
        let mut session = Session::spawn(
            Command::new(binary.join("pitex-preview"))
                .arg("--root")
                .arg(&root)
                .args(["--main", "main.tex", "--out"])
                .arg(&out)
                .arg("--cache")
                .arg(&cache)
                .stderr(Stdio::null()),
            &out,
        )?;
        for generation in [1, 2] {
            let marker = format!("PrimitiveMarker{generation}");
            session.send(&json!({"op":"update", "generation":generation,
                "files":[{"path":main, "text":source.replace("PrimitiveMarker0", &marker)}], "closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 || event["pages"] != 1 {
                return Err(format!("{name}: {event}").into());
            }
            let pdf = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
            if !text(pdf)?.contains(&marker) {
                return Err(format!("{name}: lost unsaved text").into());
            }
            let log = fs::read_to_string(event["log"].as_str().ok_or("missing log")?)?;
            match name {
                "aliases" => {
                    for expected in [
                        "AUDIT-STRCMP=0",
                        "AUDIT-MD5=900150983CD24FB0D6963F7D28E17F72",
                        "AUDIT-SIZE=7",
                        "AUDIT-DUMP=626364",
                        "AUDIT-SEED=12345",
                        "AUDIT-ABSNUM=YES",
                        "AUDIT-ABSDIM=YES",
                        "AUDIT-ORIGINAL=0",
                    ] {
                        if !log.contains(expected) {
                            return Err(format!("alias failed {expected}: {log}").into());
                        }
                    }
                    if let Some(reference_pdf) = &reference_pdf {
                        let reference_log =
                            fs::read_to_string(reference_pdf.with_extension("log"))?;
                        let pattern = Regex::new(r"AUDIT-RANDOM=([0-9]+)").unwrap();
                        if pattern.captures(&log).map(|m| m[1].to_owned())
                            != pattern.captures(&reference_log).map(|m| m[1].to_owned())
                        {
                            return Err(
                                "random generator differs from the seeded pdfTeX oracle".into()
                            );
                        }
                    }
                }
                "color-stacks" => {
                    if !log.contains("AUDIT-COLORID=20") {
                        return Err("expandable color stack allocation failed".into());
                    }
                    if !show(pdf, "pages/1/Contents")?.contains("1 0 0 rg") {
                        return Err("color stack push lost its PDF state".into());
                    }
                }
                "font-map-line" | "font-map-file" => {
                    let contents = show(pdf, "pages/1/Contents")?;
                    let matrices =
                        Regex::new(r"1\s+0\s+([-+0-9.]+)\s+1\s+[-+0-9.]+\s+[-+0-9.]+\s+Tm")
                            .unwrap();
                    if !matrices.captures_iter(&contents).any(|matrix| {
                        matrix[1]
                            .parse::<f64>()
                            .is_ok_and(|value| (value - 0.3).abs() < 0.0001)
                    }) {
                        return Err(format!(
                            "{name}: map slant did not reach the actual font matrix"
                        )
                        .into());
                    }
                }
                "page-parameters" => {
                    let after = Regex::new(r"AUDIT-AFTER-GROUP=([0-9.]+)pt,7")?
                        .captures(&log)
                        .and_then(|value| value[1].parse::<f64>().ok());
                    if !log.contains("AUDIT-IN-GROUP=5.0pt,4")
                        || !after.is_some_and(|value| (value - 72.27).abs() <= 1.0 / 65536.0)
                    {
                        return Err(format!("grouped scalar parameters changed: {log}").into());
                    }
                    // pdfTeX writes the token register as text without expansion.
                    let key = show(pdf, "pages/1/AuditKey")?;
                    if !key.contains("attributevalue") || key.contains("ExpandedValue") {
                        return Err(format!("page attributes were expanded: {key}").into());
                    }
                    if !show(pdf, "pages/1/Resources/Properties/AuditProperty/Tag")?
                        .contains("PageResources")
                    {
                        return Err("page resource token register did not reach the PDF".into());
                    }
                }
                "output-options" => {
                    let bytes = fs::read(pdf)?;
                    if !bytes.starts_with(b"%PDF-2.0") {
                        return Err("PDF major/minor version was not applied".into());
                    }
                    if show(pdf, "grep")?.contains("/FlateDecode") {
                        return Err(
                            "compression level zero still emitted deflated PDF streams".into()
                        );
                    }
                    if !show(pdf, "trailer/AuditTrailer")?.contains("Works")
                        // pdfTeX's ID is the MD5 of the given text, written twice.
                        || !show(pdf, "trailer/ID")?
                            .contains("<CE838FA73A9BC27160A96A689F76AD4F> <CE838FA73A9BC27160A96A689F76AD4F>")
                    {
                        return Err("PDF trailer configuration disappeared".into());
                    }
                    if !show(pdf, "trailer/Root/Names/AuditNames")?.contains("Works") {
                        return Err("PDF names dictionary disappeared".into());
                    }
                }
                "version-14" => {
                    if !fs::read(pdf)?.starts_with(b"%PDF-1.4") {
                        return Err("ordinary TFM output did not honor PDF 1.4".into());
                    }
                }
                "version-native" => {
                    let bytes = fs::read(pdf)?;
                    if bytes.starts_with(b"%PDF-1.4") {
                        return Err("native font features were falsely declared as PDF 1.4".into());
                    }
                    if !event["warnings"]
                        .as_str()
                        .unwrap_or("")
                        .contains("PDF features require version")
                    {
                        return Err("required PDF version increase was silent".into());
                    }
                }
                "literal" => {
                    if !show(pdf, "pages/1/Contents")?.contains("80 30 re f") {
                        return Err("direct PDF literal disappeared".into());
                    }
                }
                "objects" => {
                    if !log.contains("AUDIT-FIRST=1") || !log.contains("AUDIT-RESERVE=2") {
                        return Err("PDF object counter or reservation changed".into());
                    }
                    if !show(pdf, "trailer/Root/PitexAudit/AuditKey")?.contains("ReservedObject") {
                        return Err("numeric PDF object reference was not remapped".into());
                    }
                    if !show(pdf, "trailer/Info/Title")?.contains("Primitive Title") {
                        return Err("pdfinfo was not retained".into());
                    }
                    if !show(pdf, "grep")?.contains("DiscardedBoxObject") {
                        return Err("immediate PDF object was discarded with its TeX box".into());
                    }
                }
                "links" => {
                    let annotations = show(pdf, "pages/1/Annots")?;
                    if annotations.matches(" R").count() != 2 {
                        return Err(format!("wrong annotations: {annotations}").into());
                    }
                    if !show(pdf, "trailer/Root/Names/Dests")?.contains("target") {
                        return Err("named destination disappeared".into());
                    }
                }
                "matrix" => {
                    let contents = show(pdf, "pages/1/Contents")?;
                    if !contents.contains("1.5 0 0 1") || !contents.contains(" cm") {
                        return Err("PDF matrix disappeared".into());
                    }
                    if !text(pdf)?.contains("MatrixScaled") {
                        return Err("matrix transform lost text".into());
                    }
                }
                "forms" => {
                    if !log.contains("AUDIT-BOX=EMPTY") || !log.contains("AUDIT-FORM=1") {
                        return Err("form box consumption or counter changed".into());
                    }
                    let objects = show(pdf, "grep")?;
                    if !objects.contains("/Subtype/Form") || !objects.contains("/AuditForm(Works)")
                    {
                        return Err("form XObject was not embedded".into());
                    }
                    if text(pdf)?.matches("ReusableForm").count() != 2 {
                        return Err("form references lost their text or reuse".into());
                    }
                }
                "images" => {
                    if !log.contains("AUDIT-IMAGE=1") || !log.contains("AUDIT-IMAGE-PAGES=1") {
                        return Err("image counter or pages changed".into());
                    }
                    if !show(pdf, "grep")?.contains("/Subtype/Form") {
                        return Err("PDF image page was not imported".into());
                    }
                }
                "outline" => {
                    if !show(pdf, "trailer/Root/Outlines/Count")?.contains('2') {
                        return Err("primitive bookmark hierarchy disappeared".into());
                    }
                }
                "escaping" => {
                    for expected in [
                        r"AUDIT-STRING=macro:->a\(b\)",
                        "AUDIT-NAME=macro:->a#20b#2Fc",
                        "AUDIT-HEX=412042",
                        "AUDIT-UNHEX=AB",
                    ] {
                        if !log.contains(expected) {
                            return Err(format!("escaping lost {expected}: {log}").into());
                        }
                    }
                }
                "ignore" => {
                    if !log.contains("AUDIT-IGNORE-GROUP=0")
                        || !log.contains("ignored: Infinite glue shrinkage")
                    {
                        return Err("ignoreprimitiveerror grouping or bit1 behavior changed".into());
                    }
                }
                "expansion-eligibility" => {
                    let scales = glyph_scales(&show(pdf, "pages/1/Contents")?);
                    if scales.iter().any(|scale| (*scale - 1.0).abs() > 0.0001) {
                        return Err(
                            "math or an independently packed box inherited paragraph expansion"
                                .into(),
                        );
                    }
                }
                "expansion" | "native-expansion" => {
                    let contents = show(pdf, "pages/1/Contents")?;
                    let scales = glyph_scales(&contents);
                    if !scales.iter().any(|scale| (*scale - 1.0).abs() >= 0.0005) {
                        return Err(format!(
                            "{name}: no actual expanded glyph outlines: {scales:?}"
                        )
                        .into());
                    }
                    if scales
                        .iter()
                        .any(|scale| *scale < 0.9799 || *scale > 1.0201)
                    {
                        return Err("font expansion exceeded requested limits".into());
                    }
                    let words = text(pdf)?;
                    for expected in ["longer", "paragraph", "several", "expansion", "output"] {
                        if !words.contains(expected) {
                            return Err(format!("{name}: lost {expected}").into());
                        }
                    }
                    if let Some(reference_pdf) = &reference_pdf {
                        if !text(reference_pdf)?.contains("expansion") {
                            return Err("invalid reference fixture".into());
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        // Only the seeded random value is compared with the reference log;
        // other cases check fixed pdfTeX expectations.
        let reference = (name == "aliases").then_some("pdftex");
        preview_regression::pass(
            name,
            reference,
            &[&source],
            "PDF structure and two unsaved generations",
        )?;
    }
    if fs::read(&old_format)? != b"retained format34 cache" {
        return Err("old user cache was modified".into());
    }
    if !fs::read_dir(cache.join("format"))?
        .filter_map(Result::ok)
        .any(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.starts_with("texlive-pitex-36-") && name.ends_with("xelatex.ini.fmt")
        })
    {
        return Err("new engine format was not generated".into());
    }
    println!("PASS: format migration preserves format34 cache");
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
