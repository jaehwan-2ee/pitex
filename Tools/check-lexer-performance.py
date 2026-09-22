#!/usr/bin/env python3
"""Compare Swift/Rust lexer output and release-mode timing with a git baseline.

Usage: python3 Tools/check-lexer-performance.py [baseline-ref] (default v1.4.2).
Compiles the actual lexer implementations with their native compilers. No GTK,
Xcode, third-party Python packages, user documents or network required.
"""
from pathlib import Path
import random
import re
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
baseline = sys.argv[1] if len(sys.argv) > 1 else 'v1.4.2'
swift_path = 'Packages/TexCore/Sources/LanguageCore/LanguageCore.swift'
rust_path = 'Linux/crates/language-core/src/lib.rs'

def old(path):
    return subprocess.check_output(['git', 'show', f'{baseline}:{path}'], cwd=repo, text=True)

def swift_lexer(source):
    return source[source.index('public enum DeterministicTeXLexer'):source.index('public enum OutlineKind')]

def rust_end(source):
    return source.rfind('#[derive', 0, source.index('pub enum OutlineKind'))

swift_driver = r'''
import Foundation
@main struct Benchmark {
    static func median(_ source: String, _ dialect: TeXDialect, _ lex: (String, TeXDialect) -> [LanguageToken]) -> Double {
        var times: [Double] = []
        for _ in 0..<5 {
            let start = DispatchTime.now().uptimeNanoseconds
            let tokens = lex(source, dialect)
            times.append(Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000)
            precondition(!tokens.isEmpty)
        }
        return times.sorted()[2]
    }
    static func main() throws {
        let directory = URL(fileURLWithPath: CommandLine.arguments[1])
        let corpus = try String(contentsOf: directory.appendingPathComponent("corpus"), encoding: .utf8)
        for source in corpus.split(separator: "\0", omittingEmptySubsequences: false).map(String.init) {
            for dialect in [TeXDialect.latex, .bibtex] {
                precondition(BaselineTeXLexer.tokenize(source, dialect: dialect) == DeterministicTeXLexer.tokenize(source, dialect: dialect), "Lexer changed: \(source.debugDescription)")
            }
        }
        for name in ["large.tex", "large.bib", "unfinished.tex"] {
            let source = try String(contentsOf: directory.appendingPathComponent(name), encoding: .utf8)
            let dialect: TeXDialect = name.hasSuffix(".bib") ? .bibtex : .latex
            precondition(BaselineTeXLexer.tokenize(source, dialect: dialect) == DeterministicTeXLexer.tokenize(source, dialect: dialect))
            let before = median(source, dialect, BaselineTeXLexer.tokenize)
            let after = median(source, dialect, DeterministicTeXLexer.tokenize)
            print("{\"language\":\"Swift\",\"case\":\"\(name)\",\"before_ms\":\(before),\"after_ms\":\(after),\"speedup\":\(before / after)}")
        }
        print("Swift: token kinds, text and UTF-8 ranges match the baseline for all fixtures and generated cases")
    }
}
'''
rust_driver = r'''
fn median(source: &str, dialect: TeXDialect, lex: fn(&str, TeXDialect) -> Vec<LanguageToken>) -> f64 {
    let mut times = Vec::new();
    for _ in 0..5 {
        let start = std::time::Instant::now();
        let tokens = lex(std::hint::black_box(source), dialect);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        assert!(!std::hint::black_box(tokens).is_empty());
    }
    times.sort_by(f64::total_cmp);
    times[2]
}
fn main() {
    let directory = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let corpus = std::fs::read_to_string(directory.join("corpus")).unwrap();
    for source in corpus.split('\0') {
        for dialect in [TeXDialect::Latex, TeXDialect::Bibtex] {
            assert_eq!(BaselineTeXLexer::tokenize(source, dialect), DeterministicTeXLexer::tokenize(source, dialect), "Lexer changed: {source:?}");
        }
    }
    for name in ["large.tex", "large.bib", "unfinished.tex"] {
        let source = std::fs::read_to_string(directory.join(name)).unwrap();
        let dialect = if name.ends_with(".bib") { TeXDialect::Bibtex } else { TeXDialect::Latex };
        assert_eq!(BaselineTeXLexer::tokenize(&source, dialect), DeterministicTeXLexer::tokenize(&source, dialect));
        let before = median(&source, dialect, BaselineTeXLexer::tokenize);
        let after = median(&source, dialect, DeterministicTeXLexer::tokenize);
        println!("{{\"language\":\"Rust\",\"case\":\"{name}\",\"before_ms\":{before},\"after_ms\":{after},\"speedup\":{}}}", before / after);
    }
    println!("Rust: token kinds, text and UTF-8 ranges match the baseline for all fixtures and generated cases");
}
'''

def function(source, signature):
    start = source.index(signature)
    brace = source.index('{', start)
    depth = 1
    end = brace + 1
    while depth:
        depth += (source[end] == '{') - (source[end] == '}')
        end += 1
    return source[start:end]

# Exercise the actual scheduling methods while AppState is mutably borrowed,
# as it is on document submission. The tiny GLib stub records deferred work.
ui_path = 'Linux/crates/pitex-shell/src/app_ui.rs'
current_ui = (repo / ui_path).read_text()
old_schedule = function(old(ui_path), 'fn schedule_rehighlight_static()')
new_schedule = function(current_ui, 'fn schedule_rehighlight(&self)')
tag_path = 'Linux/crates/editor-feature/src/lib.rs'
tag_function = function((repo / tag_path).read_text(), 'pub fn decoration_tag_name(')
hot_paths = r"""
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
mod glib {
    use super::*;
    thread_local! { static QUEUE: RefCell<Vec<Box<dyn FnOnce()>>> = RefCell::new(Vec::new()); }
    pub fn timeout_add_local_once(_: Duration, f: impl FnOnce() + 'static) { QUEUE.with(|q| q.borrow_mut().push(Box::new(f))); }
    pub fn drain() -> usize {
        let callbacks = QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));
        let count = callbacks.len();
        for callback in callbacks { callback(); }
        count
    }
}
thread_local! { static STATE: RefCell<Option<Rc<RefCell<AppState>>>> = RefCell::new(None); }
struct AppState { highlight_pending: Cell<bool>, calls: Cell<usize> }
impl AppState {
    fn rehighlight(&self) { self.calls.set(self.calls.get() + 1); }
OLD_SCHEDULE
NEW_SCHEDULE
}
TAG_FUNCTION
fn check_ui_hot_paths() {
    for legacy in [true, false] {
        let state = Rc::new(RefCell::new(AppState { highlight_pending: Cell::new(false), calls: Cell::new(0) }));
        STATE.with(|s| *s.borrow_mut() = Some(state.clone()));
        for _ in 0..2 {
            let borrowed = state.borrow_mut();
            for _ in 0..25 {
                if legacy { AppState::schedule_rehighlight_static(); }
                else { borrowed.schedule_rehighlight(); }
            }
            drop(borrowed);
            assert_eq!(glib::drain(), if legacy { 25 } else { 1 });
            assert!(!state.borrow().highlight_pending.get());
        }
        assert_eq!(state.borrow().calls.get(), if legacy { 50 } else { 2 });
    }
    assert_eq!(decoration_tag_name(&LanguageTokenKind::Text("a".into())), "pitex.decoration.text");
    assert_eq!(decoration_tag_name(&LanguageTokenKind::ControlSequence("한".into())), "pitex.decoration.controlsequence");
    println!("PASS GTK hot paths: 25 submitted edits coalesce to one pass; rescheduling and stable tags verified");
}
""".replace('OLD_SCHEDULE', old_schedule).replace('NEW_SCHEDULE', new_schedule).replace('TAG_FUNCTION', tag_function)
rust_driver = rust_driver.replace('fn main() {', 'fn main() {\n    check_ui_hot_paths();')

with tempfile.TemporaryDirectory(prefix='pitex-lexer-') as temporary:
    root = Path(temporary)
    rng = random.Random(1500)
    atoms = ['a', 'é', '한글', '日本語', '👩🏽‍💻', 'e\u0301', '\\', '\\(', '\\)', '\\[', '\\]', '$', '$$',
             '\\begin', '\\end', '\\cite', '\\α', '\\👩', '{', '}', '[', ']', '(', ')', '#', '"', '@', ',', '=', '%', '\r', '\n', ' ', '\t']
    cases = ['', '\\', '\\é', '\\👩🏽‍💻', '\\begin{é}\n', '$\\👩$']
    cases += [''.join(rng.choices(atoms, k=rng.randrange(1, 90))) for _ in range(2000)]
    (root / 'corpus').write_text('\0'.join(cases))
    shutil.copyfile(repo / 'Fixtures/projects/large/main.tex', root / 'large.tex')
    (root / 'large.bib').write_text(''.join(f'@article{{ref{i}, title={{한글 é 👩🏽‍💻 paper {i}}}, author={{Doe, Jane}}, year={{2026}}}}\n' for i in range(8000)))
    (root / 'unfinished.tex').write_text('\\( unfinished 한글 é text\n' * 2000)

    current = (repo / swift_path).read_text()
    swift = root / 'Check.swift'
    swift.write_text(current + '\n' + swift_lexer(old(swift_path)).replace('DeterministicTeXLexer', 'BaselineTeXLexer') + swift_driver)
    subprocess.run(['swiftc', '-O', '-parse-as-library', str(swift), '-o', str(root / 'swift-check')], check=True)
    subprocess.run([str(root / 'swift-check'), str(root)], check=True, timeout=120)

    current = (repo / rust_path).read_text()
    start = current.rfind('#[derive', 0, current.index('pub struct SourceRange'))
    records = current[start:rust_end(current)]
    prior = old(rust_path)
    prior = prior[prior.index('pub struct DeterministicTeXLexer;'):rust_end(prior)]
    code = (records + prior.replace('DeterministicTeXLexer', 'BaselineTeXLexer')).replace(', Serialize, Deserialize', '')
    code = re.sub(r'\s*#\[serde\([^\n]*\)\]\n', '\n', code)
    rust = root / 'check.rs'
    rust.write_text('use std::collections::HashSet;\n#[derive(Debug)] pub enum LanguageCoreError { InvalidRange }\n' + code + hot_paths + rust_driver)
    subprocess.run(['rustc', '-O', '--edition=2021', str(rust), '-o', str(root / 'rust-check')], check=True)
    subprocess.run([str(root / 'rust-check'), str(root)], check=True, timeout=120)
