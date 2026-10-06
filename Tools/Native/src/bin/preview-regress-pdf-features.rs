//! Structural and visual-document regressions for the native PDF converter.
//! PITEX_BIN=<helpers directory> cargo run --manifest-path Tools/Native/Cargo.toml
//! --bin preview-regress-pdf-features
//! Requires xelatex, mutool, pdftotext and Noto Serif CJK JP. External XeLaTeX
//! is only the comparison oracle; preview rendering uses the embedded engine.
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
fn show(pdf: &Path, selector: &str) -> Result<String> {
    let result = output_timeout(
        Command::new("mutool").arg("show").arg(pdf).arg(selector),
        TIMEOUT,
    )?;
    if !result.status.success() {
        return Err(String::from_utf8_lossy(&result.stderr).into_owned().into());
    }
    Ok(String::from_utf8(result.stdout)?)
}
fn pdf_string(value: &str) -> String {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('<').and_then(|v| v.strip_suffix('>')) {
        let hex = hex.split_whitespace().collect::<String>();
        let bytes = (0..hex.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
            .collect::<Vec<_>>();
        if bytes.starts_with(&[0xfe, 0xff]) {
            return String::from_utf16_lossy(
                &bytes[2..]
                    .chunks_exact(2)
                    .map(|p| u16::from_be_bytes([p[0], p[1]]))
                    .collect::<Vec<_>>(),
            );
        }
        return String::from_utf8_lossy(&bytes).into_owned();
    }
    value.to_owned()
}
fn check(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn count_refs(value: &str) -> usize {
    value.split_whitespace().filter(|part| *part == "R").count()
}
fn validate(name: &str, pdf: &Path, reference: &Path) -> Result<()> {
    let all = show(pdf, "grep")?;
    match name {
        "links" => {
            check(
                all.contains("/URI(https://example.org)"),
                "external link lost its URI",
            )?;
            check(
                all.contains("/S/GoTo/D(section.1)"),
                "internal link lost its destination",
            )?;
            check(
                count_refs(&show(pdf, "pages/1/Annots")?) >= 2,
                "missing link annotations",
            )?;
            check(
                show(pdf, "trailer/Root/Outlines/Count")?.trim()
                    == show(reference, "trailer/Root/Outlines/Count")?.trim(),
                "nested bookmark count",
            )?;
            check(
                pdf_string(&show(pdf, "trailer/Root/Outlines/First/First/Title")?)
                    .contains("Child"),
                "nested bookmark parent/child relationship",
            )?;
            check(
                show(pdf, "trailer/Root/Names/Dests/Names")?.contains("section.1"),
                "missing named destination tree",
            )?;
        }
        "forms" => {
            check(
                count_refs(&show(pdf, "trailer/Root/AcroForm/Fields")?) == 2,
                "form fields missing from AcroForm",
            )?;
            check(
                count_refs(&show(pdf, "pages/1/Annots")?) == 2,
                "widget annotations missing",
            )?;
            check(
                all.contains("/BaseFont/Helvetica"),
                "form default font object missing",
            )?;
            check(
                all.contains("/FT/Tx") && all.contains("/FT/Btn"),
                "text/check field dictionaries missing",
            )?;
            check(
                !all.contains("/P 0 0 R"),
                "form refers to invalid page object",
            )?;
        }
        "annotation" => {
            check(
                count_refs(&show(pdf, "pages/1/Annots")?) == 1,
                "text annotation missing",
            )?;
            check(
                show(pdf, "pages/1/Annots/1/Contents")?.contains("Audit annotation"),
                "annotation contents missing",
            )?;
            let rectangle = show(pdf, "pages/1/Annots/1/Rect")?;
            let expected = show(reference, "pages/1/Annots/1/Rect")?;
            let numbers = |value: &str| {
                value
                    .split_whitespace()
                    .filter_map(|v| v.parse::<f64>().ok())
                    .collect::<Vec<_>>()
            };
            let actual = numbers(&rectangle);
            let oracle = numbers(&expected);
            check(
                actual.len() == 4 && oracle.len() == 4,
                "annotation rectangle malformed",
            )?;
            check(
                actual
                    .iter()
                    .zip(&oracle)
                    .all(|(a, b)| (a - b).abs() < 0.05),
                "annotation geometry differs from XeLaTeX",
            )?;
        }
        "metadata" => {
            check(
                pdf_string(&show(pdf, "trailer/Info/Title")?).contains("Audit Title"),
                "PDF title missing",
            )?;
            check(
                pdf_string(&show(pdf, "trailer/Info/Author")?).contains("Audit Author"),
                "PDF author missing",
            )?;
            check(
                show(pdf, "trailer/Root/PageMode")?.contains("UseOutlines"),
                "viewer settings missing",
            )?;
            check(
                show(pdf, "trailer/Root/OpenAction")?.contains(" R"),
                "viewer open action page unresolved",
            )?;
        }
        "wrapped-link" => {
            let count = all.matches("/Subtype/Link").count();
            check(
                count >= 3,
                "wrapped link did not create per-line annotations",
            )?;
            check(
                all.matches("/URI(https://example.org)").count() == count,
                "wrapped link segments lost actions",
            )?;
        }
        "page-link" => {
            check(
                count_refs(&show(pdf, "pages/1/Annots")?) == 1,
                "link ending on next page missing first-page segment",
            )?;
            check(
                count_refs(&show(pdf, "pages/2/Annots")?) == 1,
                "link ending on next page missing second-page segment",
            )?;
        }
        "vertical" => {
            check(
                all.contains("/Encoding/Identity-V"),
                "native vertical writing font encoding missing",
            )?;
            check(all.contains("/W2["), "native vertical font metrics missing")?;
            let text = output_timeout(Command::new("pdftotext").arg(pdf).arg("-"), TIMEOUT)?;
            let text = String::from_utf8(text.stdout)?;
            check(
                "日本語縦書き".chars().all(|c| text.contains(c)),
                "vertical glyph Unicode extraction changed",
            )?;
        }
        "unicode" => {
            check(
                pdf_string(&show(pdf, "trailer/Info/Title")?).contains("한글 제목 Ω"),
                "Unicode metadata lost",
            )?;
            check(
                pdf_string(&show(pdf, "trailer/Root/Outlines/First/Title")?)
                    .contains("한글 제목 Ω"),
                "Unicode bookmark lost",
            )?;
            check(
                show(pdf, "trailer/Root/Outlines/Count")?.trim()
                    == show(reference, "trailer/Root/Outlines/Count")?.trim(),
                "Unicode outline tree differs",
            )?;
        }
        "multipage-forms" => {
            check(
                count_refs(&show(pdf, "trailer/Root/AcroForm/Fields")?) == 3,
                "multi-page form fields missing",
            )?;
            check(
                count_refs(&show(pdf, "pages/1/Annots")?) == 1
                    && count_refs(&show(pdf, "pages/2/Annots")?) == 2,
                "multi-page widgets assigned incorrectly",
            )?;
        }
        "attachment" => {
            check(
                show(pdf, "pages/1/Annots/1/Subtype")?.contains("/FileAttachment"),
                "attachment annotation missing",
            )?;
            let bytes = output_timeout(
                Command::new("mutool")
                    .args(["show", "-b"])
                    .arg(pdf)
                    .arg("trailer/Root/Names/EmbeddedFiles/Names/2/EF/F"),
                TIMEOUT,
            )?;
            check(
                bytes.stdout == b"Pitex attachment contents.\n".repeat(80),
                "embedded attachment bytes changed",
            )?;
        }
        "stream-escape" => {
            for (key, expected) in [
                ("Escape", b"AA()\\\nB".as_slice()),
                ("Hex", b"\0A\x7f\xff".as_slice()),
                ("Ascii", b"xxxxxxxx".as_slice()),
            ] {
                let bytes = output_timeout(
                    Command::new("mutool")
                        .args(["show", "-b"])
                        .arg(pdf)
                        .arg(format!("trailer/Root/{key}")),
                    TIMEOUT,
                )?;
                check(
                    bytes.stdout == expected,
                    &format!("{key}: stream bytes/filter changed"),
                )?;
            }
        }
        "name-order" => {
            let names = show(pdf, "trailer/Root/Names/Dests/Names")?;
            let a = names.find("(A)").ok_or("escaped destination A missing")?;
            let z = names.find("(Z)").ok_or("destination Z missing")?;
            check(
                a < z,
                "name tree sorted encoded strings instead of decoded bytes",
            )?;
        }
        "names-preserve" => {
            check(
                show(pdf, "trailer/Root/Names/PitexCustom")?.contains("Retained"),
                "custom names dictionary lost",
            )?;
            check(
                show(pdf, "trailer/Root/Names/Dests/Names")?.contains("section.1"),
                "destinations lost while merging names",
            )?;
        }
        "named-resources" => {
            let form = show(pdf, "trailer/Root/AuditForm")?;
            check(
                form.contains("/Subtype /Form"),
                "form dictionary lost while registering resources",
            )?;
            for index in [1, 16, 32, 40] {
                check(
                    show(
                        pdf,
                        &format!("trailer/Root/AuditCollection/Items/{index}/Index"),
                    )?
                    .trim()
                        == (index - 1).to_string(),
                    "forward resource reference lost its object",
                )?;
            }
        }
        "builtin-references" => {
            check(
                show(pdf, "trailer/Root/AuditInfo/Title")?.contains("Builtin information"),
                "document information alias lost",
            )?;
            check(
                show(pdf, "trailer/Root/AuditNames/PitexCustom")?.contains("Alias retained"),
                "name dictionary alias lost",
            )?;
            check(
                show(pdf, "trailer/Root/AuditPages/Count")?.trim() == "1",
                "page tree alias lost",
            )?;
            check(
                show(pdf, "trailer/Root/Names/PitexCustom")?.contains("Alias retained"),
                "names dictionary fields lost",
            )?;
        }
        _ => return Err("unknown fixture".into()),
    }
    Ok(())
}
fn run() -> Result<()> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN is required")?)?;
    let temporary = TempDir::new("pitex-pdf-features-")?;
    let work = fs::canonicalize(temporary.path())?;
    for &(name, preamble, fixture_body) in CASES {
        preview_regression::check_interrupt()?;
        let root = work.join(name);
        fs::create_dir(&root)?;
        let out = root.join("preview");
        let reference = root.join("reference");
        fs::create_dir(&out)?;
        fs::create_dir(&reference)?;
        let main = root.join("main.tex");
        let body = if name == "named-resources" {
            let references = (0..40)
                .map(|i| format!("@resource{i}"))
                .collect::<Vec<_>>()
                .join(" ");
            let definitions = (0..40)
                .map(|i| format!("\\special{{pdf:obj @resource{i} << /Index {i} >>}}"))
                .collect::<String>();
            format!("\\special{{pdf:bxobj @resourceform width80pt height20pt depth0pt}}Reusable.\\special{{pdf:obj @collection << /Items [{references}] >>}}{definitions}\\special{{pdf:exobj}}\\special{{pdf:uxobj @resourceform}}\\special{{pdf:put @catalog << /AuditForm @resourceform /AuditCollection @collection >>}}")
        } else {
            fixture_body.to_string()
        };
        let source=format!("\\documentclass{{article}}\n\\usepackage[paperwidth=12cm,paperheight=12cm,margin=12mm]{{geometry}}\n\\pagestyle{{empty}}\n{preamble}\n\\begin{{document}}\nAuditMarker0.\\par\n{body}\n\\end{{document}}\n");
        fs::write(&main, &source)?;
        for directory in [&root, &reference] {
            fs::write(
                directory.join("attachment.txt"),
                b"Pitex attachment contents.\n".repeat(80),
            )?;
        }
        for _ in 0..2 {
            let result = output_timeout(
                Command::new("xelatex")
                    .args([
                        "-no-shell-escape",
                        "-interaction=nonstopmode",
                        "-halt-on-error",
                    ])
                    .arg(format!("-output-directory={}", reference.display()))
                    .arg("main.tex")
                    .current_dir(&root),
                TIMEOUT,
            )?;
            check(
                result.status.success(),
                &format!(
                    "{name}: reference compilation failed: {}",
                    String::from_utf8_lossy(&result.stdout)
                ),
            )?;
        }
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
            let edited = source.replace("AuditMarker0", &format!("AuditMarker{generation}"));
            // The reference uses the same unsaved bytes; changing a digit can
            // change a native word's height and thus the first-line baseline.
            fs::write(reference.join("main.tex"), &edited)?;
            let result = output_timeout(
                Command::new("xelatex")
                    .args([
                        "-no-shell-escape",
                        "-interaction=nonstopmode",
                        "-halt-on-error",
                    ])
                    .arg("main.tex")
                    .current_dir(&reference),
                TIMEOUT,
            )?;
            check(
                result.status.success(),
                "edited reference compilation failed",
            )?;
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":edited}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            check(
                event["errors"] == 0,
                &format!("{name}: embedded errors: {event}"),
            )?;
            let pdf = Path::new(event["pdf"].as_str().ok_or("missing published PDF")?);
            validate(name, pdf, &reference.join("main.pdf"))?;
        }
        // Outline counts and annotation rectangles are compared with XeLaTeX.
        preview_regression::pass(
            name,
            matches!(name, "links" | "annotation" | "unicode").then_some("xetex"),
            &[&source],
            "PDF objects and two unsaved generations",
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
const CASES: &[(&str, &str, &str)] = &[
    (
        "builtin-references",
        "",
        r"\special{pdf:put @docinfo << /Title (Builtin information) >>}\special{pdf:put @names << /PitexCustom (Alias retained) >>}\special{pdf:put @catalog << /Names @names /AuditInfo @docinfo /AuditNames @names /AuditPages @pages >>}References.",
    ),
    ("named-resources", "", ""),
    (
        "unicode",
        r"\usepackage{fontspec}\setmainfont{Noto Serif CJK JP}\usepackage[bookmarksopen=true]{hyperref}\hypersetup{pdftitle={한글 제목 Ω},pdfauthor={저자 이름}}",
        r"\section{한글 제목 Ω}\subsection{둘째 β}\subsubsection{Nested latin}\section{Next}Text.",
    ),
    (
        "multipage-forms",
        r"\usepackage{hyperref}",
        r"\begin{Form}\TextField[name=first]{First:}\newpage\TextField[name=second]{Second:}\CheckBox[name=check]{Check}\end{Form}",
    ),
    (
        "attachment",
        "",
        r"\special{pdf:fstream @embedded (attachment.txt) << /Type /EmbeddedFile >>}\special{pdf:obj @filespec << /Type /Filespec /F (attachment.txt) /EF << /F @embedded >> >>}\special{pdf:names /EmbeddedFiles [(attachment.txt) @filespec]}\special{pdf:ann bbox 0 0 20 20 << /Subtype /FileAttachment /FS @filespec /Name /PushPin >>}Attachment.",
    ),
    (
        "stream-escape",
        "",
        r"\special{pdf:stream @escape (A\string\101\string\050\string\051\string\134\string\n B) << /Type /EmbeddedFile >>}\special{pdf:stream @hex <00417FFF> << /Type /EmbeddedFile >>}\special{pdf:stream @ascii (G^+IXG^+IX) << /Filter /ASCII85Decode >>}\special{pdf:put @catalog << /Escape @escape /Hex @hex /Ascii @ascii >>}Streams.",
    ),
    (
        "name-order",
        "",
        r"\special{pdf:dest (Z) [@thispage /Fit]}\special{pdf:dest (\string\101) [@thispage /Fit]}\special{pdf:dest <FEFFAC00> [@thispage /Fit]}Destinations.",
    ),
    (
        "names-preserve",
        r"\usepackage{hyperref}",
        r"\special{pdf:obj @namesdict << /PitexCustom (Retained) >>}\special{pdf:put @catalog << /Names @namesdict >>}\section{Named}Names.",
    ),
    (
        "links",
        r"\usepackage[colorlinks]{hyperref}",
        r"\section{First}\label{first}\subsection{Child}\subsubsection{Grandchild}\href{https://example.org}{External link}. \hyperref[first]{Internal link}.",
    ),
    (
        "forms",
        r"\usepackage{hyperref}",
        r"\begin{Form}\TextField[name=name]{Name:}\CheckBox[name=agree]{Agree}\end{Form}",
    ),
    (
        "annotation",
        "",
        r"\special{pdf:ann width 40pt height 14pt depth 0pt << /Type /Annot /Subtype /Text /Contents (Audit annotation) >>}Annotation here.",
    ),
    (
        "metadata",
        r"\usepackage{hyperref}\hypersetup{pdftitle={Audit Title},pdfauthor={Audit Author}}",
        "Metadata document.",
    ),
    (
        "wrapped-link",
        r"\usepackage{hyperref}",
        r"\noindent\parbox{2.5cm}{\href{https://example.org}{A longer link with several words that wraps onto multiple lines and preserves its action.}}",
    ),
    (
        "page-link",
        "",
        r"\special{pdf:bann << /Subtype /Link /A << /S /URI /URI (https://example.org) >> >>}First page link.\newpage Second page link.\special{pdf:eann}",
    ),
    (
        "vertical",
        r"\usepackage{fontspec}",
        r#"{\font\vertical="Noto Serif CJK JP:vertical" at 20pt\vertical 日本語縦書き}"#,
    ),
];
