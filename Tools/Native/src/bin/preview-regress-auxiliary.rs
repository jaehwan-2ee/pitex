//! Exercises built-in bibliography/highlighting through the actual long-lived
//! preview protocol. External generator names are trapped on PATH. Original
//! project bytes must remain unchanged across unsaved edits and feature removal.
use pitex_native_tools::{
    preview_regression::{self, Session},
    TempDir,
};
use serde_json::{json, Value};
use std::{
    error::Error,
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};
const TIMEOUT: Duration = Duration::from_secs(60);
fn text(event: &Value) -> Result<String, Box<dyn Error>> {
    if event["errors"] != 0 {
        return Err(format!("preview errors: {event}").into());
    }
    let pdf = event["pdf"].as_str().ok_or("PDF missing")?;
    let result =
        preview_regression::output(Command::new("pdftotext").args([pdf, "-"]), None, true)?;
    if !result.status.success() {
        return Err("PDF text extraction failed".into());
    }
    Ok(String::from_utf8(result.stdout)?)
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let bin = fs::canonicalize(PathBuf::from(
        std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required")?,
    ))?;
    let temp = TempDir::new("pitex-auxiliary-regression-")?;
    let work = fs::canonicalize(temp.path())?;
    let traps = work.join("generator-traps");
    fs::create_dir(&traps)?;
    let sentinel = work.join("external-command-was-executed");
    for command in [
        "bibtex",
        "biber",
        "pygmentize",
        "latexminted",
        "xelatex",
        "pdflatex",
        "gs",
    ] {
        let script = traps.join(command);
        fs::write(
            &script,
            format!("#!/bin/sh\n: > '{}'\nexit 99\n", sentinel.display()),
        )?;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
    }
    for (name,preamble,body) in [
        ("biblatex",r"\usepackage[backend=biber,style=authoryear]{biblatex}\addbibresource{refs.bib}",r"\textcite{doe}.\printbibliography"),
        ("biblatex-alphabetic",r"\usepackage[backend=biber,style=alphabetic]{biblatex}\addbibresource{refs.bib}",r"\autocite{doe}.\printbibliography"),
        ("biblatex-disambiguation",r"\usepackage[backend=biber,style=authoryear]{biblatex}\addbibresource{refs.bib}",r"\textcite{doe} and \textcite{second}.\printbibliography"),
        ("biblatex-dynamic",r"\usepackage[backend=biber,style=authoryear]{biblatex}\newcommand{\BibFile}{refs.bib}\newcommand{\CiteKey}{doe}\addbibresource{\BibFile}\newcommand{\MyCite}[1]{\textcite{#1}}",r"\MyCite{\CiteKey}.\printbibliography"),
        ("natbib-dynamic",r"\usepackage[authoryear]{natbib}\newcommand{\BibFile}{refs}\newcommand{\CiteKey}{doe}\newcommand{\MyCite}[1]{\citet{#1}}",r"\MyCite{\CiteKey}.\bibliographystyle{plainnat}\bibliography{\BibFile}"),
        ("biblatex-refsections",r"\usepackage[backend=biber,style=authoryear]{biblatex}\addbibresource{refs.bib}",r"\begin{refsection}\textcite{doe}.\printbibliography\end{refsection}\begin{refsection}\textcite{second}.\printbibliography\end{refsection}"),
        ("biblatex-custom-sorting",r"\usepackage[backend=biber,style=authoryear]{biblatex}\DeclareSortingTemplate{latest}{\sort[direction=descending]{\field{year}}\sort{\field{title}}}\ExecuteBibliographyOptions{sorting=latest}\addbibresource{refs.bib}",r"\nocite{*}\printbibliography"),
        ("biblatex-sourcemap",r"\usepackage[backend=biber,style=authoryear]{biblatex}\DeclareSourcemap{\maps[datatype=bibtex,overwrite]{\map{\step[fieldsource=title,match=\regexp{^Unsaved},replace={Mapped}]}\map{\step[fieldset=keywords,fieldvalue={keep}]}}}\addbibresource{refs.bib}",r"\textcite{doe}.\printbibliography[keyword=keep]"),
        ("biblatex-custom-label",r"\usepackage[backend=biber,style=alphabetic]{biblatex}\DeclareLabelalphaTemplate{\labelelement{\field[strwidth=2]{labelname}}\labelelement{\literal{-}}\labelelement{\field[strwidth=4,strside=right]{year}}}\addbibresource{refs.bib}",r"\autocite{doe}.\printbibliography"),
        ("biblatex-mixed",r"\PassOptionsToPackage{backend=biber,style=authoryear}{biblatex}\usepackage{babel,biblatex,xcolor}\addbibresource{refs.bib}",r"\textcite{doe}.\printbibliography"),
        ("microtype-local-class","",r"\typeout{MixedExpansion=\the\PitexFontExpansion}\parbox{4cm}{Several words check font expansion loaded through a project document class.}"),
        ("microtype-mixed",r"\PassOptionsToPackage{expansion=true}{microtype}\usepackage{babel,microtype,xcolor}",r"\typeout{MixedExpansion=\the\PitexFontExpansion}\parbox{4cm}{Several words exercise the native expansion pass while compiling a mixed package list.}"),
        ("natbib",r"\usepackage[authoryear]{natbib}",r"\citet{doe}.\bibliographystyle{plainnat}\bibliography{refs}"),
        ("minted",r"\usepackage{minted}\newminted[pycode]{python}{linenos}\newmintinline[py]{python}{style=default}","\\begin{pycode}[autogobble]\n    def f():\n        return 41 # CodeMarkerOne\n\\end{pycode}\n\\py|print(\"InlineMarker\")|\n\\inputminted{rust}{code.rs}"),
    ] {
        preview_regression::check_interrupt()?;
        let root=work.join(name);fs::create_dir(&root)?;let out=root.join("preview");fs::create_dir(&out)?;
        let main=root.join("main.tex");let bib=root.join("refs.bib");let code=root.join("code.rs");
        let source=format!("\\documentclass{{{}}}\n{preamble}\n\\begin{{document}}AuxMarkerOriginal.\n{body}\n\\end{{document}}\n",if name=="microtype-local-class"{"pitexlocal"}else{"article"});
        if name=="microtype-local-class" {fs::write(root.join("pitexlocal.cls"),r"\NeedsTeXFormat{LaTeX2e}\ProvidesClass{pitexlocal}\LoadClass{article}\PassOptionsToPackage{expansion=true}{microtype}\RequirePackage{babel,microtype,xcolor}")?;}

        let bibliography=if name=="biblatex-refsections"||name=="biblatex-custom-sorting" {"@article{doe,author={Jane Doe},title={Original Title},journal={Journal},year={2025}}\n@article{second,author={John Smith},title={Earlier Section Title},journal={Journal},year={2024}}"}else if name=="biblatex-disambiguation" {"@article{doe,author={Jane Doe},title={Original Title},journal={Journal},year={2025}}\n@article{second,author={Jane Doe},title={Zeta Title},journal={Journal},year={2025}}"}else{"@article{doe,author={Jane Doe},title={Original Title},journal={Journal},year={2025}}"};
        let rust="fn main() { println!(\"InputMarkerOriginal\"); }";
        fs::write(&main,&source)?;fs::write(&bib,bibliography)?;fs::write(&code,rust)?;
        let mut session=Session::spawn(Command::new(bin.join("pitex-preview"))
            .arg("--root").arg(&root).arg("--main").arg("main.tex").arg("--out").arg(&out)
            .arg("--cache").arg(work.join("cache")).env("PATH",format!("{}:{}",traps.display(),std::env::var("PATH").unwrap_or_default())).stderr(Stdio::null()),&out)?;
        for generation in [1u64,2] {
            let marker=format!("AuxMarker{generation}");
            let edited=source.replace("AuxMarkerOriginal",&marker).replace("CodeMarkerOne",if generation==1{"CodeMarkerOne"}else{"CodeMarkerTwo"});
            let edited_bib=bibliography.replace("Original Title",if generation==1{"Unsaved Title One"}else{"Unsaved Title Two"}).replace("2025",if generation==1{"2025"}else{"2026"});
            let edited_code=rust.replace("InputMarkerOriginal",if generation==1{"InputMarkerOne"}else{"InputMarkerTwo"});
            session.send(&json!({"op":"update","generation":generation,"files":[{"path":main,"text":edited},{"path":bib,"text":edited_bib},{"path":code,"text":edited_code}],"closed":[]}))?;
            let event=session.wait_published(generation,TIMEOUT)?;let rendered=text(&event)?;
            if !rendered.contains(&marker){return Err(format!("{name}: unsaved source missing: {rendered}").into());}
            if name=="minted" {
                for expected in ["InlineMarker",if generation==1{"CodeMarkerOne"}else{"CodeMarkerTwo"},if generation==1{"InputMarkerOne"}else{"InputMarkerTwo"}] {
                    if !rendered.to_lowercase().contains(&expected.to_lowercase()){return Err(format!("{name}: {expected} missing: {rendered}").into());}
                }
            } else if name=="microtype-mixed"||name=="microtype-local-class" {
                let log=fs::read_to_string(event["log"].as_str().ok_or("log missing")?)?;
                if !log.lines().filter_map(|line|line.strip_prefix("MixedExpansion=")).filter_map(|value|value.parse::<i32>().ok()).any(|value|value>0)||log.contains("Font expansion does not work"){return Err(format!("mixed package expansion failed: {log}").into());}
            } else {
                let expected_title=if name=="biblatex-sourcemap" {if generation==1{"Mapped Title One"}else{"Mapped Title Two"}}else if generation==1{"Unsaved Title One"}else{"Unsaved Title Two"};
                for expected in ["Doe",expected_title,if generation==1{"2025"}else{"2026"}] {
                    if !rendered.to_lowercase().contains(&expected.to_lowercase()){return Err(format!("{name}: {expected} missing: {rendered}").into());}
                }
                if name=="biblatex-disambiguation" {for suffix in ["a","b"]{let expected=format!("{}{suffix}",if generation==1{"2025"}else{"2026"});if !rendered.contains(&expected){return Err(format!("date disambiguation label {expected} missing: {rendered}").into());}}}
                if name=="biblatex-alphabetic" {let expected=format!("Doe{}",if generation==1{"25"}else{"26"});if !rendered.contains(&expected){return Err(format!("alphabetic label {expected} missing: {rendered}").into());}}
                if name=="biblatex-refsections" && (!rendered.contains("Smith")||rendered.matches("References").count()!=2){return Err(format!("reference section contexts lost: {rendered}").into());}
                if name=="biblatex-custom-sorting" && rendered.find("Doe").unwrap_or(usize::MAX)>rendered.find("Smith").unwrap_or(0){return Err(format!("custom descending sorting failed: {rendered}").into());}
                if name=="biblatex-custom-label" {let expected=format!("Do-{}",if generation==1{"2025"}else{"2026"});if !rendered.contains(&expected){return Err(format!("custom label {expected} missing: {rendered}").into());}}
                let log=fs::read_to_string(event["log"].as_str().ok_or("log missing")?)?;
                if log.to_lowercase().contains("undefined"){return Err(format!("{name}: unresolved bibliography: {log}").into());}
            }
            if sentinel.exists(){return Err("embedded service executed an external generator".into());}
            if fs::read_to_string(&main)?!=source||fs::read_to_string(&bib)?!=bibliography||fs::read_to_string(&code)?!=rust {return Err("preview modified original project files".into());}
            if root.join("main.bbl").exists()||fs::read_dir(&root)?.filter_map(Result::ok).any(|entry|entry.file_name().to_string_lossy().starts_with("pitex-code-")){return Err("generated preview files leaked into project".into());}
        }
        // Removing a service must clear its old virtual files and diagnostics.
        let removed="\\documentclass{article}\\begin{document}AuxRemoved.\\end{document}";
        session.send(&json!({"op":"update","generation":3,"files":[{"path":main,"text":removed}],"closed":[]}))?;
        let event=session.wait_published(3,TIMEOUT)?;
        if !text(&event)?.contains("AuxRemoved"){return Err("feature removal did not publish new source".into());}
        // A blocked shell request must be visible to users in both the protocol
        // warning field and the published log, while retaining a usable PDF.
        let unsupported="\\documentclass{article}\\begin{document}AuxDiagnostic.\\immediate\\write18{printf audit > ignored.txt}\\end{document}";
        session.send(&json!({"op":"update","generation":4,"files":[{"path":main,"text":unsupported}],"closed":[]}))?;
        let event=session.wait_published(4,TIMEOUT)?;
        if !text(&event)?.contains("AuxDiagnostic")||!event["warnings"].as_str().unwrap_or("").contains("not executed")||!fs::read_to_string(event["log"].as_str().ok_or("log missing")?)?.contains("not executed")||root.join("ignored.txt").exists(){return Err(format!("unsupported generator diagnostic absent: {event}").into());}
        if sentinel.exists(){return Err("external generator executed".into());}
        println!("PASS: {name}: two unsaved generations, removal, diagnostics, no project writes or external generators");
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
