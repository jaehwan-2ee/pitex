//! Clean-room style VM differential tests. BibTeX is an oracle only; production
//! bibliography processing uses the Rust interpreter and project/installed BST.
#[allow(dead_code)]
#[path = "../../../../PreviewEngine/src/shared/embedded_aux.rs"]
mod embedded_aux;
use pitex_native_tools::{preview_regression, TempDir};
use std::{collections::BTreeMap, error::Error, fs, process::Command, time::Duration};
fn normalized(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let temporary = TempDir::new("pitex-bibtex-style-")?;
    let root = fs::canonicalize(temporary.path())?;
    let bib = r#"@preamble{ "\providecommand{\AuxBibPreamble}{Preview bibliography}" }
@article{doe,author={Jane Mary Doe and Ludwig van Beethoven},title={An {Audit} Paper},journal={Audit Journal},year={2025},volume={3},number={2},pages={10--20},month=jan}
@book{smith,author={Smith, Jr, John},title={The Book Title},year={2024},publisher={Example Press},address={Seoul},edition={Second}}
@inproceedings{third,author={A. B. Researcher and C. D. Scholar and Others},title={On Tests},booktitle={The Proceedings},year={2025},pages={100--110}}
@misc{misc,title={Misc Title},howpublished={Online},note={A note}}
@book{long,author={Charles Louis Xavier Joseph de la Vall{\'e}e Poussin},title={Long Names},year={2023},publisher={Press}}
@book{corp,author={{Research Group}},title={Team Work},year={2024},publisher={Team Press}}
@proceedings{parent,title={Parent Proceedings},booktitle={Parent Proceedings},editor={Editor, Edward},year={2022},publisher={Conference Press}}
@inproceedings{child,author={Child, Clare},title={Child Paper},crossref={parent}}
"#;
    let custom = r#"ENTRY {author title year} {counter} {label}
INTEGERS {n} STRINGS {s}
FUNCTION {sorter} { cite$ 'sort.key$ := }
FUNCTION {begin} { "\begin{thebibliography}{99}" write$ newline$ #0 'n := }
FUNCTION {article} { "\bibitem{" cite$ * "}" * write$ newline$ title "t" change.case$ write$ " / " write$ author #1 "{ll}, {f.}" format.name$ write$ newline$ n #1 + 'n := }
FUNCTION {default.type} { "\bibitem{" cite$ * "}" * write$ newline$ title write$ newline$ }
FUNCTION {finish} { "\end{thebibliography}" write$ newline$ }
READ ITERATE {sorter} SORT EXECUTE {begin} ITERATE {call.type$} EXECUTE {finish}
"#;
    fs::write(root.join("pitex-custom.bst"), custom)?;
    let mut styles = BTreeMap::from([("pitex-custom.bst".to_string(), custom.to_string())]);
    for style in [
        "plain",
        "abbrv",
        "alpha",
        "unsrt",
        "plainnat",
        "abbrvnat",
        "unsrtnat",
        "pitex-custom",
    ] {
        preview_regression::check_interrupt()?;
        if style != "pitex-custom" {
            let output = preview_regression::output(
                Command::new("kpsewhich").arg(format!("{style}.bst")),
                None,
                true,
            )?;
            styles.insert(
                format!("{style}.bst"),
                fs::read_to_string(String::from_utf8(output.stdout)?.trim())?,
            );
        }
        let source=format!("\\documentclass{{article}}\\begin{{document}}\\nocite{{*}}\\bibliographystyle{{{style}}}\\bibliography{{refs}}\\end{{document}}");
        fs::write(root.join("refs.bib"), bib)?;
        fs::write(
            root.join("main.aux"),
            format!("\\citation{{*}}\n\\bibstyle{{{style}}}\n\\bibdata{{refs}}\n"),
        )?;
        let oracle = preview_regression::output_timeout(
            Command::new("bibtex").arg("main").current_dir(&root),
            Duration::from_secs(30),
        )?;
        if !oracle.status.success() {
            return Err(String::from_utf8_lossy(&oracle.stdout).into_owned().into());
        }
        let expected = fs::read_to_string(root.join("main.bbl"))?;
        let prepared = embedded_aux::prepare("main.tex", |path| match path {
            "main.tex" => Some(source.clone()),
            "refs.bib" => Some(bib.into()),
            _ => styles.get(path).cloned(),
        });
        let actual = prepared
            .files
            .get("main.bbl")
            .ok_or("generated BBL missing")?;
        if normalized(actual) != normalized(&expected) {
            return Err(format!("{style}: generated BBL differs\nexpected:\n{expected}\nactual:\n{actual}\nwarnings:{:?}",prepared.warnings).into());
        }
        println!("PASS: {style}, BBL matches BibTeX oracle across article/book/proceedings/misc and name/month formatting");
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
