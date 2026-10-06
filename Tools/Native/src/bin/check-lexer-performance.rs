// Compare native Swift/Rust lexer output and timing with a Git baseline (default v1.4.2).
use pitex_native_tools::{repo_root, run_checked, run_output, TempDir};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const SWIFT_DRIVER: &str = r####"
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
"####;

const RUST_DRIVER: &str = r####"
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
"####;

const HOT_PATHS: &str = r####"
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
mod glib {
    use super::*;
    thread_local! { static QUEUE: RefCell<Vec<Option<Box<dyn FnOnce()>>>> = RefCell::new(Vec::new()); }
    pub struct SourceId(usize);
    impl SourceId {
        pub fn remove(self) { QUEUE.with(|q| { q.borrow_mut()[self.0].take(); }); }
    }
    pub fn timeout_add_local_once(_: Duration, f: impl FnOnce() + 'static) -> SourceId {
        QUEUE.with(|q| {
            let mut queue = q.borrow_mut();
            let id = SourceId(queue.len());
            queue.push(Some(Box::new(f)));
            id
        })
    }
    pub fn drain() -> usize {
        let callbacks = QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()));
        let count = callbacks.iter().filter(|callback| callback.is_some()).count();
        for callback in callbacks.into_iter().flatten() { callback(); }
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
"####;

fn read(path: &Path) -> Result<String, Box<dyn Error>> {
    Ok(normalize_newlines(fs::read_to_string(path)?))
}

fn normalize_newlines(text: String) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn old(repo: &Path, baseline: &str, path: &str) -> Result<String, Box<dyn Error>> {
    // Release baselines that predate this repository's history are fixtures.
    let fixture = repo
        .join("Fixtures/baselines")
        .join(baseline)
        .join(format!("{path}.txt"));
    if fixture.is_file() {
        return read(&fixture);
    }
    let mut command = Command::new("git");
    command
        .args(["show", &format!("{baseline}:{path}")])
        .current_dir(repo);
    let output = run_output(&mut command, None, true)?;
    if !output.status.success() {
        return Err(format!("Command {command:?} failed with {}", output.status).into());
    }
    Ok(normalize_newlines(String::from_utf8(output.stdout)?))
}

fn index(source: &str, marker: &str) -> Result<usize, Box<dyn Error>> {
    source
        .find(marker)
        .ok_or_else(|| format!("substring not found: {marker}").into())
}

fn rust_end(source: &str) -> Result<usize, Box<dyn Error>> {
    source[..index(source, "pub enum OutlineKind")?]
        .rfind("#[derive")
        .ok_or_else(|| "substring not found: #[derive before pub enum OutlineKind".into())
}

fn function(source: &str, signature: &str) -> Result<String, Box<dyn Error>> {
    let start = index(source, signature)?;
    let brace = start + index(&source[start..], "{")?;
    let mut depth = 1;
    let mut end = brace + 1;
    while depth != 0 {
        match source.as_bytes().get(end) {
            Some(b'{') => depth += 1,
            Some(b'}') => depth -= 1,
            Some(_) => (),
            None => return Err(format!("Unterminated function: {signature}").into()),
        }
        end += 1;
    }
    Ok(source[start..end].to_owned())
}

// CPython's integer-seeded MT19937 preserves the existing randomized corpus.
struct PythonRandom {
    state: [u32; 624],
    index: usize,
}

impl PythonRandom {
    fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = 19_650_218;
        for i in 1..624 {
            state[i] = 1_812_433_253u32
                .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        let mut i = 1;
        for _ in 0..624 {
            state[i] = (state[i] ^ (state[i - 1] ^ (state[i - 1] >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(seed);
            i += 1;
            if i >= 624 {
                state[0] = state[623];
                i = 1;
            }
        }
        for _ in 0..623 {
            state[i] = (state[i]
                ^ (state[i - 1] ^ (state[i - 1] >> 30)).wrapping_mul(1_566_083_941))
            .wrapping_sub(i as u32);
            i += 1;
            if i >= 624 {
                state[0] = state[623];
                i = 1;
            }
        }
        state[0] = 0x8000_0000;
        Self { state, index: 624 }
    }

    fn next(&mut self) -> u32 {
        if self.index >= 624 {
            for i in 0..624 {
                let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % 624] & 0x7fff_ffff);
                self.state[i] = self.state[(i + 397) % 624]
                    ^ (y >> 1)
                    ^ if y & 1 == 0 { 0 } else { 0x9908_b0df };
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    fn count(&mut self) -> usize {
        let mut value = self.next() >> 25;
        while value >= 89 {
            value = self.next() >> 25;
        }
        value as usize + 1
    }

    fn choice<'a>(&mut self, atoms: &[&'a str]) -> &'a str {
        let a = self.next() >> 5;
        let b = self.next() >> 6;
        let value = (f64::from(a) * 67_108_864.0 + f64::from(b)) / 9_007_199_254_740_992.0;
        atoms[(value * atoms.len() as f64) as usize]
    }
}

fn check() -> Result<(), Box<dyn Error>> {
    let repo = repo_root()?;
    let baseline = std::env::args().nth(1).unwrap_or_else(|| "v1.4.2".into());
    let swift_path = "Packages/TexCore/Sources/LanguageCore/LanguageCore.swift";
    let rust_path = "Linux/crates/language-core/src/lib.rs";
    let ui_path = "Linux/crates/pitex-shell/src/app_ui.rs";
    let current_ui = read(&repo.join(ui_path))?;
    let old_schedule = function(
        &old(&repo, &baseline, ui_path)?,
        "fn schedule_rehighlight_static()",
    )?;
    let new_schedule = function(&current_ui, "fn schedule_rehighlight(&self)")?;
    // Use the production timer helper and declarations now used by the scheduler.
    let debounce = current_ui
        .lines()
        .find(|line| line.starts_with("pub(crate) const ANALYSIS_DEBOUNCE:"))
        .ok_or("substring not found: ANALYSIS_DEBOUNCE")?;
    let slot = current_ui
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("static HIGHLIGHT_SOURCE:"))
        .ok_or("substring not found: HIGHLIGHT_SOURCE")?;
    let timer_support = format!(
        "{debounce}\nthread_local! {{ {slot} }}\n{}\n",
        function(&current_ui, "fn restart_timer(")?
    );
    let tag_function = function(
        &read(&repo.join("Linux/crates/editor-feature/src/lib.rs"))?,
        "pub fn decoration_tag_name(",
    )?;
    let hot_paths = HOT_PATHS
        .replace("OLD_SCHEDULE", &old_schedule)
        .replace("NEW_SCHEDULE", &new_schedule)
        .replace("TAG_FUNCTION", &tag_function);
    let rust_driver = RUST_DRIVER.replace("fn main() {", "fn main() {\n    check_ui_hot_paths();");

    let temporary = TempDir::new("pitex-lexer-")?;
    let root = temporary.path();
    let mut rng = PythonRandom::new(1500);
    let atoms = [
        "a",
        "é",
        "한글",
        "日本語",
        "👩🏽‍💻",
        "e\u{301}",
        "\\",
        "\\(",
        "\\)",
        "\\[",
        "\\]",
        "$",
        "$$",
        "\\begin",
        "\\end",
        "\\cite",
        "\\α",
        "\\👩",
        "{",
        "}",
        "[",
        "]",
        "(",
        ")",
        "#",
        "\"",
        "@",
        ",",
        "=",
        "%",
        "\r",
        "\n",
        " ",
        "\t",
    ];
    let mut cases: Vec<String> = ["", "\\", "\\é", "\\👩🏽‍💻", "\\begin{é}\n", "$\\👩$"]
        .map(str::to_owned)
        .into();
    for _ in 0..2000 {
        let count = rng.count();
        cases.push((0..count).map(|_| rng.choice(&atoms)).collect());
    }
    fs::write(root.join("corpus"), cases.join("\0"))?;
    fs::copy(
        repo.join("Fixtures/projects/large/main.tex"),
        root.join("large.tex"),
    )?;
    fs::write(root.join("large.bib"), (0..8000).map(|i| format!("@article{{ref{i}, title={{한글 é 👩🏽‍💻 paper {i}}}, author={{Doe, Jane}}, year={{2026}}}}\n")).collect::<String>())?;
    fs::write(
        root.join("unfinished.tex"),
        "\\( unfinished 한글 é text\n".repeat(2000),
    )?;

    let current_swift = read(&repo.join(swift_path))?;
    let prior = old(&repo, &baseline, swift_path)?;
    let old_swift_lexer = prior[index(&prior, "public enum DeterministicTeXLexer")?
        ..index(&prior, "public enum OutlineKind")?]
        .replace("DeterministicTeXLexer", "BaselineTeXLexer");
    let swift = root.join("Check.swift");
    fs::write(
        &swift,
        current_swift + "\n" + &old_swift_lexer + SWIFT_DRIVER,
    )?;
    run_checked(
        Command::new("swiftc")
            .args(["-O", "-parse-as-library"])
            .arg(&swift)
            .arg("-o")
            .arg(root.join("swift-check")),
        None,
    )?;
    run_checked(
        Command::new(root.join("swift-check")).arg(root),
        Some(Duration::from_secs(120)),
    )?;

    let current_rust = read(&repo.join(rust_path))?;
    let start = current_rust[..index(&current_rust, "pub struct SourceRange")?]
        .rfind("#[derive")
        .ok_or("substring not found: #[derive before pub struct SourceRange")?;
    let records = &current_rust[start..rust_end(&current_rust)?];
    let prior = old(&repo, &baseline, rust_path)?;
    let prior = &prior[index(&prior, "pub struct DeterministicTeXLexer;")?..rust_end(&prior)?];
    let code = (records.to_owned() + &prior.replace("DeterministicTeXLexer", "BaselineTeXLexer"))
        .replace(", Serialize, Deserialize", "");
    let code = regex::Regex::new(r"\s*#\[serde\([^\n]*\)\]\n")?.replace_all(&code, "\n");
    let rust = root.join("check.rs");
    fs::write(&rust, "use std::collections::HashSet;\n#[derive(Debug)] pub enum LanguageCoreError { InvalidRange }\n".to_owned() + &code + &timer_support + &hot_paths + &rust_driver)?;
    run_checked(
        Command::new("rustc")
            .args(["-O", "--edition=2021"])
            .arg(&rust)
            .arg("-o")
            .arg(root.join("rust-check")),
        None,
    )?;
    run_checked(
        Command::new(root.join("rust-check")).arg(root),
        Some(Duration::from_secs(120)),
    )?;
    temporary.close()?;
    Ok(())
}

fn main() {
    pitex_native_tools::exit_on_error(check());
}
