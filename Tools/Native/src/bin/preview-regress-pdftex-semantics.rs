//! Small pdfLaTeX differential fixtures, first for commands whose native
//! `\meaning` differs from stock pdfTeX. Each case compares SEM- log records
//! and PDF probes from one pdfLaTeX run with two unsaved native generations.
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
/// `(case, preamble, body, probes)`. A probe is a `mutool show` selector, or
/// `header`/`text` for the PDF header line and extracted text. With a needle,
/// the probe compares whether that substring is present.
type Case = (
    &'static str,
    &'static str,
    &'static str,
    &'static [(&'static str, Option<&'static str>)],
);
const CASES: &[Case] = &[
    (
        "color-stack",
        "",
        r"\edef\stack{\pdfcolorstackinit page direct{0 g 0 G}}\edef\other{\pdfcolorstackinit{1 g}}\typeout{SEM-STACKS=\stack,\other}\pdfcolorstack\stack push{1 0 0 rg 1 0 0 RG}\rule{20pt}{10pt}\pdfcolorstack\stack set{0 0 1 rg}\rule{20pt}{10pt}\pdfcolorstack\stack pop\rule{20pt}{10pt}\pdfcolorstack\stack current",
        &[
            ("pages/1/Contents", Some("1 0 0 rg 1 0 0 RG")),
            ("pages/1/Contents", Some("0 0 1 rg")),
            ("pages/1/Contents", Some("0 g 0 G")),
        ],
    ),
    (
        "glyph-unicode",
        r"\pdfgentounicode=1 \pdfglyphtounicode{a}{03B1}",
        r"\font\f=cmr10 \f abc",
        &[("text", Some("αbc")), ("text", Some("abc"))],
    ),
    (
        "output-dictionaries",
        r"\pdfmajorversion=2 \pdfminorversion=0 \pdfcompresslevel=0 \pdfobjcompresslevel=0",
        r"\pdftrailer{/AuditTrailer (Works)}\pdftrailerid{AuditIdentifier}\pdfnames{/AuditNames (Works)}\pdfinfo{/Title (Audit Title)}\pdfcatalog{/AuditCatalog (Works)}\typeout{SEM-OUTPUT=\the\pdfmajorversion.\the\pdfminorversion,\the\pdfcompresslevel}",
        &[
            ("header", None),
            ("trailer/Info/Title", None),
            ("trailer/Root/AuditCatalog", None),
            ("trailer/AuditTrailer", None),
            ("trailer/ID", None),
            ("trailer/Root/Names/AuditNames", None),
            ("grep", Some("/FlateDecode")),
        ],
    ),
    (
        "outline",
        "",
        r"\pdfoutline goto name{parent} count 1 {Parent}\pdfoutline goto name{child}{Child}\pdfoutline goto name{closed} count -1 {Closed}\pdfoutline goto name{hidden}{Hidden}\leavevmode\pdfdest name{parent} xyz Parent.\pdfdest name{child} fit Child.\pdfdest name{closed} xyz Closed.\pdfdest name{hidden} xyz Hidden.",
        &[
            ("trailer/Root/Outlines/Count", None),
            ("trailer/Root/Outlines/First/Title", None),
            ("trailer/Root/Outlines/First/Count", None),
            ("trailer/Root/Outlines/First/First/Title", None),
            ("trailer/Root/Outlines/Last/Title", None),
            ("trailer/Root/Outlines/Last/Count", None),
            ("trailer/Root/Outlines/Last/First/Title", None),
        ],
    ),
    (
        "forms",
        "",
        r"\setbox0=\hbox{\vrule width30pt height12pt depth4pt}\immediate\pdfxform attr{/AuditForm (Works)} resources{/Properties << /AuditTag (Kept) >>} 0\typeout{SEM-VOID=\ifvoid0 YES\else NO\fi}\typeout{SEM-FORM=\the\pdflastxform}\setbox2=\hbox{\pdfrefxform\pdflastxform}\typeout{SEM-BOX=\the\wd2,\the\ht2,\the\dp2}\box2",
        &[
            ("pages/1/Resources/XObject/Fm1/AuditForm", None),
            // BBox origins differ by convention (depth vs baseline) with
            // identical placement; the TeX box dimensions are compared instead.
            ("pages/1/Resources/XObject/Fm1/Resources/Properties/AuditTag", None),
        ],
    ),
    (
        "image-pages",
        "",
        r"\pdfximage page 2 {pages.pdf}\typeout{SEM-PAGES=\the\pdflastximagepages,\the\pdflastximage}\setbox2=\hbox{\pdfrefximage\pdflastximage}\typeout{SEM-SIZE=\the\wd2,\the\ht2,\the\dp2}\pdfximage width 2cm page 3 {pages.pdf}\setbox3=\hbox{\pdfrefximage\pdflastximage}\typeout{SEM-SCALED=\the\wd3,\the\ht3,\the\pdflastximage}\box2\box3",
        &[
            ("pages/1/Resources/XObject/Im1/BBox", None),
            ("pages/1/Resources/XObject/Im2/BBox", None),
        ],
    ),
    (
        "escaping",
        "",
        r"\typeout{SEM-STRING=\pdfescapestring{a(b) c\string\\d}}\typeout{SEM-NAME=\pdfescapename{a b/c\string#d(e)[f]}}\typeout{SEM-HEX=\pdfescapehex{A B/}}\typeout{SEM-UNHEX=\pdfunescapehex{4142},\pdfunescapehex{41424},\pdfunescapehex{4a6B}}",
        &[],
    ),
    (
        "primitive-access",
        "",
        r"\typeout{SEM-BEFORE=\ifpdfprimitive\pdfstrcmp YES\else NO\fi,\ifpdfprimitive\relax YES\else NO\fi,\ifpdfprimitive\UndefinedProbe YES\else NO\fi}\def\pdfstrcmp#1#2{redefined}\typeout{SEM-AFTER=\ifpdfprimitive\pdfstrcmp YES\else NO\fi,\pdfprimitive\pdfstrcmp{a}{b},\pdfstrcmp{a}{b}}",
        &[],
    ),
    (
        "absolute-conditionals",
        "",
        r"\typeout{SEM-ABS=\ifpdfabsnum-4>3 Y\else N\fi,\ifpdfabsnum 3=-3 Y\else N\fi,\ifpdfabsnum-5<2 Y\else N\fi,\ifpdfabsdim-2pt=2pt Y\else N\fi,\ifpdfabsdim-1pt<0.5pt Y\else N\fi,\unless\ifpdfabsnum-1=1 Y\else N\fi}",
        &[],
    ),
    (
        "page-dictionaries",
        "",
        r"\def\attributevalue{Before}\pdfpageattr{/Rotate 90 /AuditKey (\attributevalue)}\def\attributevalue{AtShipout}\pdfpageresources{/Properties << /AuditProperty << /Tag (Resources) >> >>}\pdfpagesattr{/AuditPages (Pages)}\pdfpagewidth=10cm \pdfpageheight=8cm \typeout{SEM-ERRHELP=[\the\errhelp]}\typeout{SEM-ATTR=\the\pdfpageattr}\typeout{SEM-SIZE=\the\pdfpagewidth,\the\pdfpageheight}",
        &[
            ("pages/1/Rotate", None),
            ("pages/1/AuditKey", None),
            ("pages/1/MediaBox", None),
            ("pages/1/Resources/Properties/AuditProperty/Tag", None),
            ("trailer/Root/Pages/AuditPages", None),
        ],
    ),
    (
        "protrusion",
        "",
        r"\font\f=cmr10 \rpcode\f`.=500 \lpcode\f`A=300 \pdfprotrudechars=2 \setbox0=\vbox{\hsize=6cm\f\noindent A paragraph ends.\par\global\setbox1=\lastbox}\typeout{SEM-MARGIN=\leftmarginkern1,\rightmarginkern1,\the\pdfprotrudechars}\box1",
        &[],
    ),
    (
        "alias-queries",
        "",
        r"\pdfsetrandomseed 4321 \typeout{SEM-NORMAL=\pdfnormaldeviate,\pdfnormaldeviate,\the\pdfrandomseed}\pdfresettimer\typeout{SEM-TIMER=\ifnum\pdfelapsedtime<65536 SUBSECOND\else SLOW\fi,\ifnum\pdfelapsedtime<0 NEGATIVE\else NONNEGATIVE\fi}\typeout{SEM-SHELL=\the\pdfshellescape}\typeout{SEM-MODDATE=\pdffilemoddate{probe.dat}}\typeout{SEM-FILE=\pdffilesize{probe.dat},\pdffilesize{missing.dat},\pdffiledump offset 1 length 3 {probe.dat}}\typeout{SEM-MD5=\pdfmdfivesum{abc},\pdfmdfivesum file {probe.dat}}\typeout{SEM-STRCMP=\pdfstrcmp{a}{b},\pdfstrcmp{b}{a},\pdfstrcmp{same}{same}}",
        &[],
    ),
    (
        "expansion-and-input",
        "",
        r"\def\TestValue{value}\typeout{SEM-EXPANDED=\detokenize\expandafter{\expanded{\noexpand\TestValue\space\TestValue}}}\expandafter\def\csname Probe\ifincsname Inside\else Outside\fi\endcsname{}\typeout{SEM-INCSNAME=\ifincsname Y\else N\fi,\ifdefined\ProbeInside Y\else N\fi}\input{sub}\input sub.tex ",
        &[],
    ),
    (
        // Distribution pdftexconfig files choose their own default page size.
        "positions",
        r"\pdfpagewidth=\paperwidth \pdfpageheight=\paperheight",
        r"\noindent\pdfsavepos\write16{SEM-POS=\the\pdflastxpos,\the\pdflastypos}Position.",
        &[],
    ),
    (
        "annotations",
        r"\pdfpagewidth=\paperwidth \pdfpageheight=\paperheight",
        r"\noindent\pdflinkmargin=2pt\pdfstartlink attr{/Border [0 0 1]} user{/Subtype /Link /A << /S /URI /URI (https://example.org) >>}\vrule width50pt height8pt depth2pt\pdfendlink\edef\lastlink{\the\pdflastlink}\pdfannot width20pt height10pt depth2pt {/Subtype /Text /Contents (Audit note)}\pdfcatalog{/AuditLink \lastlink\space 0 R /AuditAnnot \the\pdflastannot\space 0 R}",
        &[
            ("trailer/Root/AuditLink/Rect", None),
            ("trailer/Root/AuditLink/A/URI", None),
            ("trailer/Root/AuditLink/Border", None),
            ("trailer/Root/AuditAnnot/Rect", None),
            ("trailer/Root/AuditAnnot/Contents", None),
        ],
    ),
    (
        "content-operators",
        "",
        r"\pdfliteral{0 0 1 RG}\pdfliteral direct{1 0 0 rg 0 0 80 30 re f}\pdfliteral page{0.5 g}\leavevmode\pdfsave\pdfsetmatrix{1.5 0 0 1}\rlap{\vrule width10pt height10pt}\pdfrestore Restored.",
        &[
            ("pages/1/Contents", Some("0 0 1 RG")),
            ("pages/1/Contents", Some("1 0 0 rg 0 0 80 30 re f")),
            ("pages/1/Contents", Some("0.5 g")),
            // pdfTeX translates around a zero-offset matrix; native output folds
            // the offset into one equivalent cm, so only the linear part matches.
            ("pages/1/Contents", Some("1.5 0 0 1 ")),
        ],
    ),
    (
        "paragraph-tokens",
        "",
        "\\def\\MyPar{\\typeout{SEM-MYPAR=\\the\\partokencontext}\\endgraf}\\partokenname\\MyPar Text\n\n\\partokenname\\par\\setbox0=\\vbox{\\hsize=5cm\\noindent A\\vadjust pre{\\hbox to10pt{}}\\vadjust{\\hbox to20pt{}}\\par}\\setbox2=\\vbox{\\unvcopy0 \\global\\setbox4=\\lastbox\\unskip\\global\\setbox5=\\lastbox\\unskip\\global\\setbox6=\\lastbox}\\typeout{SEM-VADJUST=\\the\\wd4,\\the\\wd5,\\the\\wd6}",
        &[],
    ),
    (
        // \pdfoutput stays 0 natively so XeTeX-aware packages keep their
        // xdvipdfmx paths; COMPATIBILITY.md records that difference.
        "output-mode",
        "",
        r"\typeout{SEM-MODE=\the\pdfignoreddimen,\the\pdfdraftmode}",
        &[],
    ),
    (
        "object-references",
        "",
        r"\pdfobj{(ReferencedObject)}\pdfrefobj\pdflastobj\pdfobj{(UnreferencedObject)}\typeout{SEM-OBJECTS=\the\pdflastobj}",
        // Native output also keeps unreferenced non-immediate objects, which
        // pdfTeX omits; COMPATIBILITY.md records that difference.
        &[("grep", Some("ReferencedObject"))],
    ),
];
fn records(log: &str) -> BTreeMap<String, String> {
    log.lines()
        .filter(|line| line.starts_with("SEM-"))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.into(), value.into()))
        .collect()
}
fn probes(pdf: &Path, probes: &[(&str, Option<&str>)]) -> Result<Vec<String>, Box<dyn Error>> {
    let indirect = regex::Regex::new(r"(?s)^\d+ \d+ obj\n(.*)\nendobj$")?;
    let mut values = Vec::new();
    for (selector, needle) in probes {
        let value = match *selector {
            "header" => String::from_utf8_lossy(&fs::read(pdf)?)
                .lines()
                .next()
                .unwrap_or_default()
                .to_string(),
            "text" => String::from_utf8(
                output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?.stdout,
            )?,
            _ => {
                let shown = output_timeout(
                    Command::new("mutool").arg("show").arg(pdf).arg(selector),
                    TIMEOUT,
                )?;
                if shown.status.success() {
                    String::from_utf8(shown.stdout)?
                } else {
                    "<missing>".into()
                }
            }
        };
        values.push(match needle {
            Some(needle) => format!("{selector} contains {needle}: {}", value.contains(needle)),
            // Direct and indirect objects are equivalent PDF values.
            None => format!("{selector}: {}", indirect.replace(value.trim(), "$1")),
        });
    }
    Ok(values)
}
/// PDF numbers may differ below the 0.01 bp precision pdfTeX writes by default.
fn same_values(actual: &[String], expected: &[String]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(actual, expected)| {
            let (actual, expected) = (
                actual.split_whitespace().collect::<Vec<_>>(),
                expected.split_whitespace().collect::<Vec<_>>(),
            );
            actual.len() == expected.len()
                && actual.iter().zip(&expected).all(|(a, e)| {
                    a == e
                        || matches!((a.parse::<f64>(), e.parse::<f64>()),
                            (Ok(a), Ok(e)) if (a - e).abs() <= 0.01)
                })
        })
}
/// Three pages with distinct boxes for page selection and page counts.
fn pages_fixture() -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>".to_string(),
    ];
    for (width, height) in [(100, 50), (200, 80), (300, 150)] {
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] /Resources << >> /Contents 6 0 R >>"));
    }
    objects.push("<< /Length 0 >>\nstream\n\nendstream".into());
    let mut data = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(data.len());
        data.extend(format!("{} 0 obj\n{object}\nendobj\n", index + 1).bytes());
    }
    let xref = data.len();
    data.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for offset in offsets {
        data.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    data.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .bytes(),
    );
    data
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?)?;
    let temp = TempDir::new("pitex-pdftex-semantics-")?;
    let work = fs::canonicalize(temp.path())?;
    for &(name, preamble, body, selectors) in CASES {
        if let Ok(filter) = std::env::var("PITEX_SEMANTIC_CASE") {
            if !name.contains(&filter) {
                continue;
            }
        }
        preview_regression::check_interrupt()?;
        let root = work.join(name);
        fs::create_dir(&root)?;
        fs::write(root.join("probe.dat"), b"abcdef\n")?;
        fs::write(root.join("pages.pdf"), pages_fixture())?;
        fs::write(
            root.join("sub.tex"),
            "\\typeout{SEM-INPUT-\\the\\inputlineno=\\jobname}\n",
        )?;
        let main = root.join("main.tex");
        let source = format!("\\documentclass{{article}}\n\\pagestyle{{empty}}\n{preamble}\n\\begin{{document}}\nSemanticMarker0.\\par\n{body}\n\\end{{document}}\n");
        fs::write(&main, &source)?;
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
            return Err(format!(
                "reference {name}: {}",
                String::from_utf8_lossy(&result.stdout)
            )
            .into());
        }
        let expected = records(&fs::read_to_string(reference.join("main.log"))?);
        let expected_probes = probes(&reference.join("main.pdf"), selectors)?;
        if expected.is_empty() && expected_probes.is_empty() {
            return Err(format!("{name}: the fixture compares nothing").into());
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
            let edited = source.replace("SemanticMarker0", &format!("SemanticMarker{generation}"));
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":edited}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != 0 {
                return Err(format!("native {name}: {event}").into());
            }
            let log = fs::read_to_string(event["log"].as_str().ok_or("missing log")?)?;
            let actual = records(&log);
            let pdf = Path::new(event["pdf"].as_str().ok_or("missing PDF")?);
            let actual_probes = probes(pdf, selectors)?;
            if actual != expected || !same_values(&actual_probes, &expected_probes) {
                return Err(format!(
                    "{name}: differs from pdfLaTeX\nexpected {expected:?} {expected_probes:?}\nactual   {actual:?} {actual_probes:?}"
                )
                .into());
            }
            if fs::read_to_string(&main)? != source {
                return Err("preview changed the saved source".into());
            }
        }
        preview_regression::pass(
            name,
            Some("pdftex"),
            &[&source],
            "pdfLaTeX records/PDF probes and two unsaved generations",
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
