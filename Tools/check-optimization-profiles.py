#!/usr/bin/env python3
"""Compare the shipping Rust profile with ThinLTO and opt-level 3 on real core code."""
from pathlib import Path
import json
import subprocess
import tempfile

repo = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='pitex-profiles-') as directory:
    root = Path(directory)
    (root / 'src').mkdir()
    (root / 'Cargo.toml').write_text('''[package]
name = "pitex-profile-check"
version = "0.0.0"
edition = "2021"
[dependencies]
language-core = { path = ''' + json.dumps(str(repo / 'Linux/crates/language-core')) + ''' }
[profile.release]
opt-level = 2
lto = false
[profile.thin]
inherits = "release"
lto = "thin"
[profile.opt3]
inherits = "thin"
opt-level = 3
''')
    (root / 'src/main.rs').write_text(r'''
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
''')
    for profile in ['release', 'thin', 'opt3']:
        subprocess.run(['cargo', 'build', '--quiet', '--manifest-path', str(root / 'Cargo.toml'),
                        '--profile', profile], check=True)
        result = json.loads(subprocess.check_output([str(root / 'target' / profile / 'pitex-profile-check'),
                           str(repo / 'Fixtures/projects/large/main.tex')], text=True))
        print(json.dumps({'profile': profile, **result}), flush=True)
