//! Actual engine tests for POSIX string captures, PDF error status, reproducible
//! metadata and omission controls. Stock pdfTeX is an oracle only.
use pitex_native_tools::{
    preview_regression::{self, output_timeout, Session},
    TempDir,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
const TIMEOUT: Duration = Duration::from_secs(60);
fn records(log: &str) -> BTreeMap<String, String> {
    log.lines()
        .filter(|line| line.starts_with("AUDIT-"))
        .filter_map(|line| {
            line.split_once('=')
                .map(|(key, value)| (key.to_string(), value.to_string()))
        })
        .collect()
}
fn show(pdf: &Path, selector: &str) -> Result<String, Box<dyn Error>> {
    let result = output_timeout(
        Command::new("mutool").args(["show"]).arg(pdf).arg(selector),
        TIMEOUT,
    )?;
    if !result.status.success() {
        return Err("mutool failed".into());
    }
    Ok(String::from_utf8(result.stdout)?)
}
fn source(settings: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{settings}\n\\begin{{document}}\n{body}\nPrimitiveMarkerOriginal.\n\\end{{document}}\n")
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(PathBuf::from(
        std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?,
    ))?;
    let temp = TempDir::new("pitex-engine-strings-")?;
    let root = fs::canonicalize(temp.path())?;
    fs::write(
        root.join("included.tex"),
        r"\documentclass{article}\pdfinfo{/Title (Imported Source)}\begin{document}Imported page.\end{document}",
    )?;
    let included = output_timeout(
        Command::new("pdflatex")
            .args([
                "-interaction=nonstopmode",
                "-halt-on-error",
                "-no-shell-escape",
                "included.tex",
            ])
            .current_dir(&root)
            .env("SOURCE_DATE_EPOCH", "32"),
        TIMEOUT,
    )?;
    if !included.status.success() {
        return Err("included PDF oracle setup failed".into());
    }
    let matcher = r"\def\TestPattern{a|aa}\def\TestText{aa}
\typeout{AUDIT-LONGEST=\pdfmatch{\TestPattern}{\TestText}}
\typeout{AUDIT-LONGEST-0=\pdflastmatch0}
\typeout{AUDIT-SUBCOUNT=\pdfmatch subcount 3 {ab(cd)*ef(gh)(ij)}{abefghij}}
\typeout{AUDIT-SUBCOUNT-0=\pdflastmatch0}
\typeout{AUDIT-SUBCOUNT-1=\pdflastmatch1}
\typeout{AUDIT-SUBCOUNT-2=\pdflastmatch2}
\typeout{AUDIT-SUBCOUNT-3=\pdflastmatch3}
\typeout{AUDIT-VALID=\pdfmatch{(ab)}{xxab}}
\typeout{AUDIT-INVALID=\pdfmatch{(}{yyy}}
\typeout{AUDIT-PRESERVE-1=\pdflastmatch1}
\typeout{AUDIT-NOMATCH=\pdfmatch{z+}{abc}}
\typeout{AUDIT-CLEAR-0=\pdflastmatch0}
\typeout{AUDIT-ICASE=\pdfmatch icase {ABC}{xxabc}}
\typeout{AUDIT-ICASE-0=\pdflastmatch0}
\typeout{AUDIT-ZERO=\pdfmatch subcount 0 {a}{a}}
\typeout{AUDIT-ZERO-0=\pdflastmatch0}
\typeout{AUDIT-LIMITED=\pdfmatch subcount 1 {(a)(b)}{ab}}
\typeout{AUDIT-LIMITED-1=\pdflastmatch1}
\typeout{AUDIT-RESET=\pdfmatch{(a)(b)}{ab}}
\typeout{AUDIT-RESET-2=\pdflastmatch2}
\typeout{AUDIT-STATUS-START=\the\pdfretval}
\immediate\pdfobj useobjnum 999 {<< /Invalid (No reservation) >>}
\typeout{AUDIT-STATUS-BAD=\the\pdfretval}
\immediate\pdfobj {<< /Valid (Later success) >>}
\typeout{AUDIT-STATUS-STICKY=\the\pdfretval}
";
    let cases = [
        ("strings", "", matcher, true),
        (
            "negative-query",
            "",
            r"\typeout{AUDIT-NEGATIVE-SET=\pdfmatch{ab}{ab}}\typeout{AUDIT-NEGATIVE=\pdflastmatch-1}\typeout{AUDIT-AFTER-NEGATIVE=\pdflastmatch0}",
            true,
        ),
        ("dates", "", r"\typeout{AUDIT-DATE=\pdfcreationdate}", true),
        (
            "omit-date",
            r"\pdfinfoomitdate=1",
            r"\typeout{AUDIT-DATE=\pdfcreationdate}",
            true,
        ),
        ("omit-info", r"\pdfomitinfodict=1", "", true),
        (
            "explicit-date",
            r"\pdfinfoomitdate=1\pdfinfo{/CreationDate (D:20000101000000Z) /ModDate (D:20010101000000Z)}",
            "",
            true,
        ),
        ("ptex-mask", r"\pdfsuppressptexinfo=1", "", true),
        (
            "ptex-import-0",
            r"\usepackage{graphicx}\pdfsuppressptexinfo=0",
            r"\includegraphics[width=4cm]{included.pdf}",
            true,
        ),
        (
            "ptex-import-2",
            r"\usepackage{graphicx}\pdfsuppressptexinfo=2",
            r"\includegraphics[width=4cm]{included.pdf}",
            true,
        ),
        (
            "ptex-import-4",
            r"\usepackage{graphicx}\pdfsuppressptexinfo=4",
            r"\includegraphics[width=4cm]{included.pdf}",
            true,
        ),
        (
            "ptex-import-8",
            r"\usepackage{graphicx}\pdfsuppressptexinfo=8",
            r"\includegraphics[width=4cm]{included.pdf}",
            true,
        ),
        (
            "ptex-import-15",
            r"\usepackage{graphicx}\pdfsuppressptexinfo=15",
            r"\includegraphics[width=4cm]{included.pdf}",
            true,
        ),
        // Host reference is1.40.25; underscore control was introduced1.40.27.
        ("ptex-underscore", r"\pdfuseptexunderscore=1", "", false),
    ];
    for (name, settings, body, has_oracle) in cases {
        let work = root.join(name);
        fs::create_dir(&work)?;
        fs::copy(root.join("included.pdf"), work.join("included.pdf"))?;
        let main = work.join("main.tex");
        let original = source(settings, body);
        fs::write(&main, &original)?;
        let mut expected = BTreeMap::new();
        if has_oracle {
            let reference = work.join("reference");
            fs::create_dir(&reference)?;
            let output = output_timeout(
                Command::new("pdflatex")
                    .args(["-interaction=nonstopmode", "-no-shell-escape"])
                    .arg(format!("-output-directory={}", reference.display()))
                    .arg("main.tex")
                    .current_dir(&work)
                    .env("SOURCE_DATE_EPOCH", "32"),
                TIMEOUT,
            )?;
            if !output.status.success()
                && !(name == "negative-query" && output.status.code() == Some(1))
            {
                return Err(format!(
                    "reference {name} failed: {}",
                    String::from_utf8_lossy(&output.stdout)
                )
                .into());
            }
            expected = records(&fs::read_to_string(reference.join("main.log"))?);
        }
        let out = work.join("preview");
        fs::create_dir(&out)?;
        let mut session = Session::spawn(
            Command::new(bin.join("pitex-preview"))
                .arg("--root")
                .arg(&work)
                .arg("--main")
                .arg("main.tex")
                .arg("--out")
                .arg(&out)
                .arg("--cache")
                .arg(root.join("cache"))
                .env("SOURCE_DATE_EPOCH", "32")
                .stderr(Stdio::null()),
            &out,
        )?;
        for generation in [1u64, 2] {
            let edited = original.replace(
                "PrimitiveMarkerOriginal",
                &format!("PrimitiveMarker{generation}"),
            );
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":edited}],"closed":[]}))?;
            let event = session.wait_published(generation, TIMEOUT)?;
            if event["errors"] != if name == "negative-query" { 1 } else { 0 } {
                return Err(format!("native {name} errors: {event}").into());
            }
            let log = fs::read_to_string(event["log"].as_str().ok_or("native log missing")?)?;
            if has_oracle && records(&log) != expected {
                return Err(format!(
                    "{name}: records differ\nexpected:{expected:?}\nactual:{:?}",
                    records(&log)
                )
                .into());
            }
            let pdf = Path::new(event["pdf"].as_str().ok_or("PDF missing")?);
            let info = show(pdf, "trailer/Info")?;
            if let Some(mask) = name
                .strip_prefix("ptex-import-")
                .and_then(|value| value.parse::<i32>().ok())
            {
                let resources = show(pdf, "pages/1/Resources/XObject")?;
                let references = regex::Regex::new(r"(\d+)\s+\d+\s+R")?;
                let mut forms = String::new();
                for row in references.captures_iter(&resources) {
                    forms.push_str(&show(pdf, &row[1])?);
                }
                for (field, bit) in [("FileName", 2), ("PageNumber", 4), ("InfoDict", 8)] {
                    let present = forms.contains(&format!("/PTEX.{field}"))
                        || forms.contains(&format!("/PTEX_{field}"));
                    if present != (mask & bit == 0) {
                        return Err(
                            format!("PTEX mask{mask} field{field} mismatch: {forms}").into()
                        );
                    }
                }
            }
            match name {
                "dates" => {
                    if !info.contains("/CreationDate (D:19700101000032Z)")
                        || !info.contains("/ModDate (D:19700101000032Z)")
                    {
                        return Err(format!("SOURCE_DATE_EPOCH metadata mismatch: {info}").into());
                    }
                }
                "omit-date" => {
                    if info.contains("/CreationDate") || info.contains("/ModDate") {
                        return Err(format!("automatic dates not omitted: {info}").into());
                    }
                }
                "omit-info" => {
                    if info.trim() != "null" {
                        return Err(format!("Info dictionary not omitted: {info}").into());
                    }
                }
                "explicit-date" => {
                    if !info.contains("D:20000101000000Z") || !info.contains("D:20010101000000Z") {
                        return Err(format!("explicit user dates lost: {info}").into());
                    }
                }
                "ptex-mask" => {
                    if info.contains("PTEX.Fullbanner") || info.contains("PTEX_Fullbanner") {
                        return Err(format!("PTEX.Fullbanner not suppressed: {info}").into());
                    }
                }
                "ptex-underscore" => {
                    if !info.contains("/PTEX_Fullbanner") || info.contains("/PTEX.Fullbanner") {
                        return Err(format!("PTEX underscore naming failed: {info}").into());
                    }
                }
                _ => {}
            }
            if fs::read_to_string(&main)? != original {
                return Err("primitive test modified original source".into());
            }
        }
        // Cases without logged records compare only fixed PDF expectations.
        let reference = (has_oracle && !expected.is_empty()).then_some("pdftex");
        preview_regression::pass(
            name,
            reference,
            &[&original],
            "actual native behavior and two unsaved generations",
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
