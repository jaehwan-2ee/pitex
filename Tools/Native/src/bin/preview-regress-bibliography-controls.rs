//! Original auxiliary implementation vs Biber as a test-only oracle. Every
//! control file is emitted by TeX; comparisons inspect BBL fields and ordering.
#[allow(dead_code)]
#[path = "../../../../PreviewEngine/src/shared/embedded_aux.rs"]
mod embedded_aux;
use pitex_native_tools::{
    preview_regression::{self, Session},
    TempDir,
};
use serde_json::json;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

fn entries(bbl: &str) -> Vec<(String, String)> {
    let entry = regex::Regex::new(r"\\entry\{([^}]+)\}").unwrap();
    let title = regex::Regex::new(r"(?s)\\field\{title\}\{(.*?)\}\s*\n").unwrap();
    let positions = entry
        .captures_iter(bbl)
        .map(|capture| (capture.get(0).unwrap().start(), capture[1].to_string()))
        .collect::<Vec<_>>();
    positions
        .iter()
        .enumerate()
        .map(|(index, (start, key))| {
            let end = positions
                .get(index + 1)
                .map(|(start, _)| *start)
                .unwrap_or(bbl.len());
            let value = title
                .captures(&bbl[*start..end])
                .map(|capture| capture[1].split_whitespace().collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            (key.clone(), value)
        })
        .collect()
}
fn requested_entries(oracle: &str, actual: &str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let header = regex::Regex::new(r"\\datalist\[entry\]\{([^}]+)\}")?;
    let mut entries_in_lists = Vec::new();
    for name in header
        .captures_iter(actual)
        .map(|capture| capture[1].to_string())
    {
        let Some(list) = header
            .captures_iter(oracle)
            .find(|capture| capture[1] == name)
        else {
            return Err(format!("Oracle is missing requested datalist {name}").into());
        };
        let start = list.get(0).unwrap().end();
        let end = oracle[start..]
            .find(r"\enddatalist")
            .ok_or("Unclosed oracle datalist")?
            + start;
        entries_in_lists.extend(entries(&oracle[start..end]));
    }
    Ok(entries_in_lists)
}
struct Case {
    name: &'static str,
    options: &'static str,
    preamble: &'static str,
    bib: &'static str,
    body: &'static str,
}
fn cases() -> Vec<Case> {
    let names = r"@book{z,title={EntryZ},author={Zebra, Zoe},year={2025},publisher={Press}}
@book{a,title={EntryA},author={Åberg, Anna},year={2025},publisher={Press}}
@book{ae,title={EntryAe},author={Äberg, Bea},year={2025},publisher={Press}}
@book{o,title={EntryO},author={Öberg, Cai},year={2025},publisher={Press}}
@book{e,title={EntryE},author={Éberg, Dan},year={2025},publisher={Press}}";
    let case_names = r"@book{lower,title={alpha},author={Doe, Jane},year={2025},publisher={Press}}
@book{upper,title={Alpha},author={Doe, Jane},year={2025},publisher={Press}}
@book{accent,title={Álpha},author={Doe, Jane},year={2025},publisher={Press}}";
    let regex_bib = r"@book{a,title={Prefix Alpha Alpha suffix},author={Doe, Jane},year={2025},publisher={Press}}";
    vec![
        Case {
            name: "lookarounds",
            options: "sorting=none",
            preamble: r"\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{(?&lt;=Prefix\s)\p{L}+(?=\s\p{L}+\ssuffix)},replace={Mapped}]}}}",
            bib: regex_bib,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "backreference",
            options: "sorting=none",
            preamble: r"\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{(\p{L}+)\s+\1},replace=\regexp{$1}]}}}",
            bib: regex_bib,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "named-capture",
            options: "sorting=none",
            preamble: r"\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{(?&lt;word&gt;\p{L}+)\s+\k&lt;word&gt;},replace=\regexp{$+{word}-Mapped}]}}}",
            bib: regex_bib,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "negative-lookaround",
            options: "sorting=none",
            preamble: r"\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{(?&lt;!Bad\s)\p{L}+(?!\sBad)},replace={Mapped}]}}}",
            bib: regex_bib,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "case-interpolation",
            options: "sorting=none",
            preamble: r"\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{(\p{L}+)},replace=\regexp{\U$1\E}]}}}",
            bib: regex_bib,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "swedish-locale",
            options: "sorting=nty,sortlocale=sv_SE",
            preamble: "",
            bib: names,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "english-locale",
            options: "sorting=nty,sortlocale=en_US",
            preamble: "",
            bib: names,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "sortupper-true",
            options: "sortupper=true,sortcase=true",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: case_names,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "sortupper-false",
            options: "sortupper=false,sortcase=true",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: case_names,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "sortcase-false",
            options: "sortcase=false",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: case_names,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "sort-group-locale",
            options: "sortlocale=en_US",
            preamble: r"\DeclareSortingTemplate{controls}{\sort[locale=sv_SE,direction=descending]{\field{author}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: names,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "sort-template-locale",
            options: "sortlocale=en_US",
            preamble: r"\DeclareSortingTemplate[locale=sv_SE]{controls}{\sort{\field{author}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: names,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "substring-padding",
            options: "",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\field[strwidth=2,strside=right,padwidth=4,padchar=0]{volume}}\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: r"@book{a,title={EntryA},volume={109},year={2025},publisher={Press}}@book{b,title={EntryB},volume={12},year={2025},publisher={Press}}@book{c,title={EntryC},volume={5},year={2025},publisher={Press}}",
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "sort-final",
            options: "",
            preamble: r"\DeclareSortingTemplate{controls}{\sort[final]{\field{sortkey}}\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: r"@book{a,title={EntryZ},sortkey={same},year={2025},publisher={Press}}@book{b,title={EntryA},sortkey={same},year={2025},publisher={Press}}",
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "given-first-name-template",
            options: "",
            preamble: r"\DeclareSortingNamekeyTemplate[givenfirst]{\keypart{\namepart{given}}\keypart{\namepart{family}}}",
            bib: r"@book{a,title={EntryA},author={Alpha, Zoe},year={2025},publisher={Press}}@book{b,title={EntryB},author={Zulu, Anna},year={2025},publisher={Press}}",
            body: r"\newrefcontext[sortingnamekeytemplatename=givenfirst]\nocite{*}\printbibliography",
        },
        Case {
            name: "explicit-backreference",
            options: "sorting=none",
            preamble: r"\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{(\p{L}+)\s+\g{1}},replace=\regexp{${1}-Mapped}]}}}",
            bib: regex_bib,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "sort-group-case",
            options: "sortcase=true,sortupper=true",
            preamble: r"\DeclareSortingTemplate{controls}{\sort[sortupper=false]{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: case_names,
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "useprefix",
            options: "useprefix=true",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\field{author}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: r"@book{a,title={EntryA},author={van Gogh, Vincent},year={2025},publisher={Press}}@book{b,title={EntryB},author={Zulu, Anna},year={2025},publisher={Press}}",
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "semantic-citeorder",
            options: "",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\citeorder}\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: r"@book{a,title={EntryA},year={2025},publisher={Press}}@book{b,title={EntryB},year={2025},publisher={Press}}",
            body: r"\nocite{b,a}\printbibliography",
        },
        Case {
            name: "internal-citeorder",
            options: "",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\citeorder}\sort{\intciteorder}\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: r"@book{a,title={EntryA},year={2025},publisher={Press}}@book{b,title={EntryB},year={2025},publisher={Press}}",
            body: r"\nocite{b,a}\printbibliography",
        },
        Case {
            name: "unicode-captures",
            options: "sorting=none",
            preamble: r"\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{(\p{L}+)\s+\1},replace=\regexp{\l$1\E-Mapped}]}}}",
            bib: r"@book{a,title={Prefix Ångström Ångström suffix},author={Doe, Jane},year={2025},publisher={Press}}",
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "unicode-mark-captures",
            options: "sorting=none",
            preamble: r"\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{([\p{L}\p{M}]+)\s+\1},replace=\regexp{\l$1\E-Mapped}]}}}",
            bib: r"@book{a,title={Prefix Ångström Ångström suffix},author={Doe, Jane},year={2025},publisher={Press}}",
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "sortgiveninits",
            options: "sortgiveninits=true",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\field{author}}\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: r"@book{a,title={EntryA},author={Doe, John},year={2025},publisher={Press}}@book{b,title={EntryB},author={Doe, Jane},year={2025},publisher={Press}}",
            body: r"\nocite{*}\printbibliography",
        },
        Case {
            name: "maxsortnames",
            options: "maxsortnames=1,minsortnames=1,uniquelist=false",
            preamble: r"\DeclareSortingTemplate{controls}{\sort{\field{author}}\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=controls}",
            bib: r"@book{a,title={EntryA},author={Doe, Jane and Zebra, Zed},year={2025},publisher={Press}}@book{b,title={EntryB},author={Doe, Jane and Alpha, Ann},year={2025},publisher={Press}}",
            body: r"\nocite{*}\printbibliography",
        },
    ]
}
fn oracle(
    root: &Path,
    source: &str,
    bib: &str,
) -> Result<(String, String, String), Box<dyn Error>> {
    fs::create_dir_all(root)?;
    fs::write(root.join("main.tex"), source)?;
    fs::write(root.join("refs.bib"), bib)?;
    let latex = preview_regression::output_timeout(
        Command::new("xelatex")
            .args(["-interaction=nonstopmode", "-halt-on-error", "main.tex"])
            .current_dir(root),
        Duration::from_secs(30),
    )?;
    if !latex.status.success() {
        return Err(format!(
            "TeX oracle failed: {}",
            String::from_utf8_lossy(&latex.stdout)
        )
        .into());
    }
    let bcf = fs::read_to_string(root.join("main.bcf"))?;
    let aux = fs::read_to_string(root.join("main.aux"))?;
    let biber = preview_regression::output_timeout(
        Command::new("biber").arg("main").current_dir(root),
        Duration::from_secs(30),
    )?;
    if !biber.status.success() {
        return Err(format!(
            "Biber oracle failed: {}",
            format!(
                "{}\n{}",
                String::from_utf8_lossy(&biber.stdout),
                bcf.lines()
                    .filter(|line| line.contains("map_"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        )
        .into());
    }
    Ok((bcf, aux, fs::read_to_string(root.join("main.bbl"))?))
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let temp = TempDir::new("pitex-bibliography-controls-")?;
    let root = fs::canonicalize(temp.path())?;
    let package =
        preview_regression::output(Command::new("kpsewhich").arg("biblatex.sty"), None, true)?;
    let package = fs::read_to_string(String::from_utf8(package.stdout)?.trim())?;
    let bin = std::env::var_os("PITEX_BIN").map(PathBuf::from);
    let traps = root.join("traps");
    fs::create_dir(&traps)?;
    let sentinel = root.join("external-generator-executed");
    #[cfg(unix)]
    for command in [
        "biber",
        "bibtex",
        "pygmentize",
        "latexminted",
        "xelatex",
        "pdflatex",
        "gs",
    ] {
        let path = traps.join(command);
        fs::write(
            &path,
            format!("#!/bin/sh\n: > '{}'\nexit 99\n", sentinel.display()),
        )?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    for case in cases() {
        if std::env::var("PITEX_BIB_CONTROL_CASE")
            .ok()
            .is_some_and(|filter| !case.name.contains(&filter))
        {
            continue;
        }
        let source=format!("\\documentclass{{article}}\n\\usepackage[backend=biber,style=authoryear,{}]{{biblatex}}\n{}\n\\addbibresource{{refs.bib}}\n\\begin{{document}}ControlMarkerOriginal.\n{}\n\\end{{document}}",case.options,case.preamble,case.body);
        let project = root.join(case.name);
        fs::create_dir(&project)?;
        let main = project.join("main.tex");
        let bibfile = project.join("refs.bib");
        fs::write(&main, &source)?;
        fs::write(&bibfile, case.bib)?;
        let out = project.join("preview");
        fs::create_dir(&out)?;
        let mut session = if let Some(bin) = &bin {
            Some(Session::spawn(
                Command::new(bin.join("pitex-preview"))
                    .args(["--root"])
                    .arg(&project)
                    .args(["--main", "main.tex", "--out"])
                    .arg(&out)
                    .arg("--cache")
                    .arg(root.join("cache"))
                    .env(
                        "PATH",
                        format!(
                            "{}:{}",
                            traps.display(),
                            std::env::var("PATH").unwrap_or_default()
                        ),
                    )
                    .stderr(Stdio::null()),
                &out,
            )?)
        } else {
            None
        };
        for generation in [1, 2] {
            let marker = format!("ControlMarker{generation}");
            let edited = source.replace("ControlMarkerOriginal", &marker);
            let bib = if generation == 1 {
                case.bib.to_string()
            } else {
                case.bib
                    .replace("Alpha", "Omega")
                    .replace("Åberg", "Aardvark")
                    .replace("Ångström", "Öström")
                    .replace("2025", "2026")
            };
            let (bcf, aux, expected) = oracle(
                &root
                    .join("oracle")
                    .join(case.name)
                    .join(generation.to_string()),
                &edited,
                &bib,
            )?;
            let files = BTreeMap::from([
                ("main.tex", edited.clone()),
                ("refs.bib", bib.clone()),
                ("main.bcf", bcf.clone()),
                ("main.aux", aux),
                ("biblatex.sty", package.clone()),
            ]);
            let prepared =
                embedded_aux::prepare_from_controls("main.tex", |path| files.get(path).cloned());
            let bbl = prepared
                .files
                .get("main.bbl")
                .ok_or("embedded BBL missing")?;
            let actual_entries = entries(bbl);
            let expected_entries = requested_entries(&expected, bbl)?;
            if actual_entries != expected_entries {
                return Err(format!("{} generation {generation}: BBL mismatch\nexpected={expected_entries:?}\nactual={actual_entries:?}\nwarnings={:?}\nBCF tail={}\nactual BBL={bbl}",case.name,prepared.warnings,bcf.chars().rev().take(3500).collect::<String>().chars().rev().collect::<String>()).into());
            }
            if !prepared.warnings.is_empty() {
                return Err(format!("{} warnings: {:?}", case.name, prepared.warnings).into());
            }
            if let Some(session) = &mut session {
                session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":edited},{"path":bibfile,"text":bib}],"closed":[]}))?;
                let event = session.wait_published(generation, Duration::from_secs(60))?;
                if event["errors"] != 0 {
                    return Err(format!("{} preview failed: {event}", case.name).into());
                }
                let pdf = event["pdf"].as_str().ok_or("PDF missing")?;
                let text = preview_regression::output(
                    Command::new("pdftotext").args([pdf, "-"]),
                    None,
                    true,
                )?;
                let text = String::from_utf8(text.stdout)?;
                if !text.contains(&marker) {
                    return Err(format!("{} unsaved marker missing: {text}", case.name).into());
                }
                let mut offset = 0;
                for (_, title) in &expected_entries {
                    let remaining = &text[offset..];
                    let Some(index) = remaining.find(title) else {
                        return Err(format!("{} embedded rendered bibliography order/title mismatch: title={title:?}, text={text}",case.name).into());
                    };
                    offset += index + title.len();
                }
                if event["warnings"]
                    .as_str()
                    .unwrap_or("")
                    .contains("Unsupported bibliography")
                {
                    return Err(format!("unexpected protocol warning: {event}").into());
                }
            }
            if sentinel.exists() {
                return Err("production preview executed a trapped external generator".into());
            }
            if fs::read_to_string(&main)? != source
                || fs::read_to_string(&bibfile)? != case.bib
                || project.join("main.bbl").exists()
                || project.join("main.bcf").exists()
            {
                return Err("preview modified project inputs or leaked auxiliary files".into());
            }
        }
        println!(
            "PASS: {}, emitted BCF and BBL fields/order match Biber for two unsaved generations{}",
            case.name,
            if session.is_some() {
                ", protocol/PDF verified, generators trapped"
            } else {
                ""
            }
        );
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
