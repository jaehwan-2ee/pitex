//! Reproducible public-name inventory against bare pdfTeX and the actual native
//! helper. Registration and availability are observations, not semantic parity.
//! Usage: PITEX_BIN=/path/to/pair preview-audit-pdftex-vocabulary [--output FILE]
//! --stock-only skips native probing. --require-native fails on missing non-ID
//! catalog commands, including newer manual names, after saving the report.
//! --evidence FILE reads the PITEX_EVIDENCE records written by passing
//! regression cases and derives each command's evidence level from the
//! catalog's links; a link without a matching pass for these helpers fails.
//! --check REPORT fails when REPORT's evidence levels differ from this run.
use pitex_native_tools::{
    preview_regression::{self, output_timeout, Session},
    TempDir,
};
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
const TIMEOUT: Duration = Duration::from_secs(60);
const CATALOG: &str =
    include_str!("../../../../PreviewEngine/compatibility/pdftex-vocabulary.json");
fn probe_source(names: &[String], native: bool) -> String {
    let mut source: String = if native {
        "\\documentclass{article}\n\\errorcontextlines=3\n".into()
    } else {
        "\\scrollmode\n\\catcode123=1 \\catcode125=2\n".into()
    };
    for name in names {
        source.push_str(&format!("\\ifcsname {name}\\endcsname\\immediate\\write16{{PITEXVOCAB|{name}|present|\\expandafter\\meaning\\csname {name}\\endcsname}}\\else\\immediate\\write16{{PITEXVOCAB|{name}|absent}}\\fi\n"));
    }
    if native {
        source.push_str("\\typeout{PITEXENGINE|XeTeX \\the\\XeTeXversion\\XeTeXrevision}\n\\begin{document}Vocabulary audit.\\end{document}\n");
    } else {
        source.push_str("\\immediate\\write16{PITEXENGINE|\\pdftexbanner}\n\\end\n");
    }
    source
}
fn observations(log: &str) -> BTreeMap<String, Value> {
    let mut result = BTreeMap::new();
    for line in log.lines() {
        if let Some(line) = line.strip_prefix("PITEXVOCAB|") {
            let parts = line.splitn(3, '|').collect::<Vec<_>>();
            if parts.len() >= 2 {
                result.insert(
                    parts[0].into(),
                    json!({"available":parts[1]=="present","meaning_sample":parts.get(2).copied()}),
                );
            }
        }
    }
    result
}
fn identity(log: &str) -> Option<String> {
    log.lines()
        .find_map(|line| line.strip_prefix("PITEXENGINE|").map(str::to_string))
}
fn hash(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}
/// A link is a reviewed claim. With pass records, it must match a case that
/// passed against these helpers and names the command; a `tested` claim also
/// needs that case to have compared its result with stock pdfTeX in the same
/// run. Returns the verified links and the problems found.
fn verify_links(
    name: &str,
    identity: bool,
    links: &[Value],
    records: Option<&[Value]>,
) -> (Vec<Value>, Vec<String>) {
    let mut evidence = Vec::new();
    let mut problems = Vec::new();
    for link in links {
        let tool = link["tool"].as_str().unwrap_or_default();
        let case = link["case"].as_str().unwrap_or_default();
        let claim = link["claim"].as_str().unwrap_or_default();
        if identity || !matches!(claim, "tested" | "partial") {
            problems.push(format!("{name}: invalid evidence link {link}"));
            continue;
        }
        let Some(records) = records else { continue };
        let Some(record) = records
            .iter()
            .find(|record| record["tool"] == tool && record["case"] == case)
        else {
            problems.push(format!(
                "{name}: {tool}/{case} has no passing record for these helpers"
            ));
            continue;
        };
        let named = record["control_sequences"]
            .as_array()
            .is_some_and(|names| names.iter().any(|value| value == name));
        if !named {
            problems.push(format!("{name}: \\{name} does not occur in {tool}/{case}"));
        } else if claim == "tested" && record["reference"] != "pdftex" {
            problems.push(format!(
                "{name}: {tool}/{case} is claimed tested without a same-run pdfTeX comparison"
            ));
        } else {
            evidence.push(
                json!({"tool":tool,"case":case,"claim":claim,"reference":record["reference"]}),
            );
        }
    }
    (evidence, problems)
}
fn run() -> Result<(), Box<dyn Error>> {
    preview_regression::watch_interrupt();
    let mut output = None;
    let mut evidence_path = None;
    let mut check_path = None;
    let mut stock_only = false;
    let mut strict = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                output = Some(PathBuf::from(
                    args.next().ok_or("--output requires a path")?,
                ))
            }
            "--evidence" => {
                evidence_path = Some(PathBuf::from(
                    args.next().ok_or("--evidence requires a path")?,
                ))
            }
            "--check" => {
                check_path = Some(PathBuf::from(
                    args.next().ok_or("--check requires a report path")?,
                ))
            }
            "--stock-only" => stock_only = true,
            "--require-native" => strict = true,
            _ => return Err(format!("unknown option {arg}").into()),
        }
    }
    let catalog: Value = serde_json::from_str(CATALOG)?;
    let commands = catalog["commands"]
        .as_array()
        .ok_or("catalog commands missing")?;
    let names = commands
        .iter()
        .map(|entry| entry["name"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    let temporary = TempDir::new("pitex-vocabulary-audit-")?;
    let root = fs::canonicalize(temporary.path())?;
    fs::write(root.join("stock.tex"), probe_source(&names, false))?;
    let version = output_timeout(Command::new("pdftex").arg("--version"), TIMEOUT)?;
    let stock = output_timeout(
        Command::new("pdftex")
            .args([
                "--ini",
                "-etex",
                "-interaction=nonstopmode",
                "-no-shell-escape",
                "stock.tex",
            ])
            .current_dir(&root),
        TIMEOUT,
    )?;
    if !stock.status.success() {
        return Err(format!(
            "reference probe failed: {}",
            String::from_utf8_lossy(&stock.stdout)
        )
        .into());
    }
    let stock_log = fs::read_to_string(root.join("stock.log"))?;
    let reference = observations(&stock_log);
    if reference.len() != names.len() {
        return Err(format!(
            "reference emitted{} observations, expected{}",
            reference.len(),
            names.len()
        )
        .into());
    }
    let registry_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../PreviewEngine/xetex/engine/xetex_format.h");
    let header = fs::read_to_string(&registry_path)?;
    let registry_block = header
        .split("#define XETEX_FORMAT_PRIMITIVE_INITIALIZERS")
        .nth(1)
        .ok_or("canonical primitive registry missing")?;
    let pattern =
        Regex::new(r#"\{\s*"([^"]+)"\s*,\s*([^,\n]+)\s*,\s*([^,\n]+)\s*,\s*([^}\n]+)\}"#)?;
    let mut registry = pattern
        .captures_iter(registry_block)
        .map(|m| {
            (
                m[1].to_string(),
                json!({"command":m[2].trim(),"operand":m[3].trim(),"initializer":m[4].trim()}),
            )
        })
        .collect::<BTreeMap<_, _>>();
    // The original generated header and Pitex-authored extension tables
    // both supply initializer data. Record where each declaration lives;
    // runtime probing below remains independent from this source inventory.
    let shared = registry_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("src/shared");
    let engine_initializer = fs::read_to_string(
        shared
            .parent()
            .unwrap()
            .join("linux/xetex/engine/xetex_ini.rs"),
    )?;
    let parameter =
        Regex::new(r#"name:\s*"([^"]+)"\s*,\s*storage:\s*([^,]+),\s*initial:\s*([^,}\n]+)"#)?;
    let tuples = Regex::new(r#"\(\s*"([^"]+)"\s*,\s*"([^"]+)"(?:\s*,\s*(-?\d+))?\s*\)"#)?;
    let mut definition_sources = Vec::new();
    for entry in fs::read_dir(&shared)? {
        let path = entry?.path();
        let filename = path.file_name().unwrap().to_string_lossy();
        if !filename.ends_with("_definitions.rs") {
            continue;
        }
        let source = fs::read_to_string(&path)?;
        let module = path.file_stem().unwrap().to_string_lossy();
        let wiring = engine_initializer.contains(&format!("crate::{module}::"));
        definition_sources.push(json!({"path":format!("src/shared/{filename}"),"sha256":hash(&path)?,"initializer_reference_present":wiring}));
        for fields in parameter.captures_iter(&source) {
            let name = fields[1].to_string();
            registry.entry(name).or_insert(json!({"source":filename,"command":fields[2].trim(),"initializer":fields[3].trim(),"initializer_reference_present":wiring}));
        }
        for fields in tuples.captures_iter(&source) {
            let name = fields[1].to_string();
            registry.entry(name).or_insert(json!({"source":filename,"command_or_alias":fields[2].trim(),"operand":fields.get(3).map(|m|m.as_str()),"initializer_reference_present":wiring}));
        }
    }
    definition_sources.sort_by_key(|source| source["path"].as_str().unwrap_or("").to_string());
    let mut native = BTreeMap::new();
    let mut native_identity = None;
    let mut native_binary = None;
    if !stock_only {
        let bin = fs::canonicalize(
            std::env::var_os("PITEX_BIN").ok_or("PITEX_BIN required unless --stock-only")?,
        )?;
        let out = root.join("preview");
        fs::create_dir(&out)?;
        let main = root.join("main.tex");
        let source = probe_source(&names, true);
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
                .arg(root.join("cache"))
                .stderr(Stdio::null()),
            &out,
        )?;
        session.send(&json!({"op":"update","generation":1,"files":[{"path":main,"text":source}],"closed":[]}))?;
        let event = session.wait_published(1, TIMEOUT)?;
        if event["errors"] != 0 {
            return Err(format!("native probe errors: {event}").into());
        }
        let log = fs::read_to_string(event["log"].as_str().ok_or("native log missing")?)?;
        native = observations(&log);
        native_identity = identity(&log);
        native_binary = Some(
            json!({"helper_sha256":hash(&bin.join("pitex-preview"))?,"engine_sha256":hash(&bin.join("pitex-preview-xetex"))?}),
        );
        if native.len() != names.len() {
            return Err(format!(
                "native emitted{} observations, expected{}",
                native.len(),
                names.len()
            )
            .into());
        }
    }
    // Passing regression cases append one record each (preview_regression::pass).
    // Only records for these helper binaries count as evidence.
    let records = match &evidence_path {
        None => None,
        Some(path) => {
            let binaries = native_binary
                .as_ref()
                .ok_or("--evidence requires native probing")?;
            let mut records = Vec::new();
            for line in fs::read_to_string(path)?.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let record: Value = serde_json::from_str(line)?;
                if record["helper_sha256"] == binaries["helper_sha256"]
                    && record["engine_sha256"] == binaries["engine_sha256"]
                {
                    records.push(record);
                }
            }
            Some(records)
        }
    };
    let mut problems = Vec::new();
    let mut meaning_gaps = Vec::new();
    let mut rows = Vec::new();
    let mut remaining = Vec::new();
    let mut remaining_catalog = Vec::new();
    let mut intentional = Vec::new();
    let mut new_reference_absent = Vec::new();
    for command in commands {
        let name = command["name"].as_str().unwrap();
        let stock_available = reference[name]["available"] == true;
        let native_available = native.get(name).map(|row| row["available"] == true);
        let identity = command["identity"] == true;
        if native_available == Some(false) {
            if identity {
                intentional.push(name);
            } else {
                remaining_catalog.push(name);
                if stock_available {
                    remaining.push(name);
                }
            }
        }
        if !stock_available {
            new_reference_absent.push(name);
        }
        let (evidence, mut link_problems) = verify_links(
            name,
            identity,
            command["evidence"]
                .as_array()
                .map_or(&[][..], Vec::as_slice),
            records.as_deref(),
        );
        problems.append(&mut link_problems);
        let level = if evidence.iter().any(|link| link["claim"] == "tested") {
            "tested"
        } else if evidence.is_empty() {
            "not_evaluated"
        } else {
            "partial"
        };
        let meaning = match (
            reference[name]["meaning_sample"].as_str(),
            native
                .get(name)
                .and_then(|row| row["meaning_sample"].as_str()),
        ) {
            (Some(stock), Some(native)) => Some(stock == native),
            _ => None,
        };
        if meaning == Some(false) && level == "not_evaluated" && !identity {
            meaning_gaps.push(name);
        }
        rows.push(json!({"name":name,"stock":reference[name],"native":native.get(name),"declared_in_registration_sources":registry.contains_key(name),"registry":registry.get(name),"identity_command":identity,"meaning_matches_reference":meaning,"semantic_parity":level,"semantic_evidence":evidence}));
    }
    let levels = |level: &str| {
        rows.iter()
            .filter(|row| row["semantic_parity"] == level)
            .count()
    };
    let semantic = json!({"verified":records.is_some(),"tested":levels("tested"),"partial":levels("partial"),"not_evaluated":levels("not_evaluated"),"rule":"tested: a linked regression case passed against these helpers and compared this command's result with stock pdfTeX output from the same run. partial: a linked case passed but checked fixed expectations, another reference engine or only part of the interface. not_evaluated: no linked passing case. Links are reviewed claims; neither level establishes full pdfTeX semantic parity."});
    let report = json!({"schema_version":2,"scope":"Command registration, runtime availability and linked differential regression evidence. Full pdfTeX semantic parity is not established.","manual":catalog["manual"],"reference":{"mode":"pdftex --ini -etex","version_output":String::from_utf8_lossy(&version.stdout).lines().next(),"identity":identity(&stock_log),"available":reference.values().filter(|row|row["available"]==true).count()},"native":{"identity":native_identity,"binaries":native_binary,"available":if stock_only{None}else{Some(native.values().filter(|row|row["available"]==true).count())}},"source_registry":{"header_sha256":hash(&registry_path)?,"extension_sources":definition_sources,"unique_declared_names":registry.len()},"remaining_stock_names":remaining,"remaining_valid_catalog_names":remaining_catalog,"intentionally_unspoofed_identity_names":intentional,"manual_names_unavailable_in_this_stock_version":new_reference_absent,"semantic_evidence":semantic,"meaning_differs_without_evidence":meaning_gaps,"commands":rows});
    let text = serde_json::to_string_pretty(&report)? + "\n";
    if let Some(path) = output {
        fs::write(&path, text)?;
        println!("Vocabulary report: {}", path.display());
        println!(
            "Reference available={}, native available={}, remaining={}",
            report["reference"]["available"],
            report["native"]["available"],
            report["remaining_stock_names"].as_array().unwrap().len()
        );
        println!(
            "Evidence verified={}, tested={}, partial={}, not_evaluated={}",
            semantic["verified"],
            semantic["tested"],
            semantic["partial"],
            semantic["not_evaluated"]
        );
    } else {
        print!("{text}");
    }
    if let Some(path) = check_path {
        if records.is_none() {
            return Err("--check requires --evidence".into());
        }
        // The committed report must not claim more (or less) than this run proves.
        let claims = |report: &Value| {
            report["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|row| {
                    let links = row["semantic_evidence"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|link| json!([link["tool"], link["case"], link["claim"]]))
                        .collect::<Vec<_>>();
                    (
                        row["name"].to_string(),
                        json!([row["semantic_parity"], links]),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        };
        let committed: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        let (committed, derived) = (claims(&committed), claims(&report));
        for name in committed
            .keys()
            .chain(derived.keys())
            .collect::<std::collections::BTreeSet<_>>()
        {
            if committed.get(name) != derived.get(name) {
                problems.push(format!(
                    "{}: {name} records {}, this run derives {}",
                    path.display(),
                    committed.get(name).unwrap_or(&Value::Null),
                    derived.get(name).unwrap_or(&Value::Null)
                ));
            }
        }
    }
    if !problems.is_empty() {
        for problem in &problems {
            eprintln!("{problem}");
        }
        return Err(format!("{} semantic evidence problems", problems.len()).into());
    }
    if strict
        && (!report["remaining_valid_catalog_names"]
            .as_array()
            .unwrap()
            .is_empty()
            || stock_only)
    {
        return Err("native command availability audit did not meet --require-native".into());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tested_claims_need_a_passing_pdftex_comparison() {
        let records = [
            json!({"tool":"t","case":"pdftex","reference":"pdftex","control_sequences":["pdfnames"]}),
            json!({"tool":"t","case":"fixed","reference":null,"control_sequences":["pdfnames"]}),
        ];
        let link = |case: &str, claim: &str| json!({"tool":"t","case":case,"claim":claim});
        let check = |links: &[Value], identity| {
            let (evidence, problems) = verify_links("pdfnames", identity, links, Some(&records));
            (evidence.len(), problems.len())
        };
        assert_eq!(check(&[link("pdftex", "tested")], false), (1, 0));
        assert_eq!(check(&[link("fixed", "partial")], false), (1, 0));
        // Fixed expectations, a missing pass and an identity name all fail.
        assert_eq!(check(&[link("fixed", "tested")], false), (0, 1));
        assert_eq!(check(&[link("missing", "partial")], false), (0, 1));
        assert_eq!(check(&[link("pdftex", "tested")], true), (0, 1));
        assert_eq!(check(&[link("pdftex", "proven")], false), (0, 1));
        let (_, problems) = verify_links(
            "pdfinfo",
            false,
            &[link("pdftex", "partial")],
            Some(&records),
        );
        assert_eq!(
            problems.len(),
            1,
            "a case must name the command it is linked to"
        );
        // Without pass records nothing is verified, and nothing fails.
        assert_eq!(
            verify_links("pdfnames", false, &[link("pdftex", "tested")], None)
                .0
                .len(),
            0
        );
    }
}
