//! Compare the shipping Rust profile with ThinLTO and opt-level 3 on real core code.
use pitex_native_tools::{python_json, repo_root, run_checked, TempDir};
use std::{
    error::Error,
    fs,
    io::{self, Write},
    process::{Command, Stdio},
};

const HARNESS: &str = r####"
use language_core::{DeterministicTeXLexer, TeXDialect};
fn main() {
    let text = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let expected = DeterministicTeXLexer::tokenize(&text, TeXDialect::Latex).len();
    let mut samples = Vec::new();
    for _ in 0..15 {
        let started = std::time::Instant::now();
        let tokens = DeterministicTeXLexer::tokenize(std::hint::black_box(&text), TeXDialect::Latex);
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(std::hint::black_box(tokens).len(), expected);
    }
    samples.sort_by(f64::total_cmp);
    println!("{{\"median_ms\":{},\"tokens\":{expected}}}", samples[7]);
}
"####;

fn main() {
    pitex_native_tools::exit_on_error(check());
}

fn check() -> Result<(), Box<dyn Error>> {
    let repo = repo_root()?;
    let directory = TempDir::new("pitex-profiles-")?;
    let root = directory.path();
    fs::create_dir(root.join("src"))?;
    let manifest = format!(
        "{}{}{}",
        r#"[package]
name = "pitex-profile-check"
version = "0.0.0"
edition = "2021"
[dependencies]
language-core = { path = "#,
        python_json(
            &repo
                .join("Linux/crates/language-core")
                .to_string_lossy()
                .into_owned()
                .into()
        ),
        r#" }
[profile.release]
opt-level = 2
lto = false
[profile.thin]
inherits = "release"
lto = "thin"
[profile.opt3]
inherits = "thin"
opt-level = 3
"#
    );
    fs::write(root.join("Cargo.toml"), manifest)?;
    fs::write(root.join("src/main.rs"), HARNESS)?;
    for profile in ["release", "thin", "opt3"] {
        run_checked(
            Command::new("cargo")
                .args(["build", "--quiet", "--manifest-path"])
                .arg(root.join("Cargo.toml"))
                .args(["--profile", profile]),
            None,
        )?;
        let output = pitex_native_tools::run_output(
            Command::new(
                root.join("target")
                    .join(profile)
                    .join("pitex-profile-check"),
            )
            .arg(repo.join("Fixtures/projects/large/main.tex"))
            .stdin(Stdio::inherit()),
            None,
            true,
        )?;
        if !output.status.success() {
            return Err(format!("Profile check failed with {}", output.status).into());
        }
        let result: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let result = result
            .as_object()
            .ok_or("Profile result must be an object")?;
        let mut fields = serde_json::Map::new();
        fields.insert("profile".into(), profile.into());
        fields.extend(result.clone());
        println!("{}", python_json(&fields.into()));
        io::stdout().flush()?;
    }
    directory.close()?;
    Ok(())
}
