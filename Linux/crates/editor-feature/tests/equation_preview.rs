//! Port of `EquationPreviewEngineTests.swift` — drives
//! `EquationPreviewEngine` the way a host does: events with a virtual
//! clock, `poll` at `next_deadline`, renderer completions fed back with
//! artificial delays.

use editor_feature::equation_preview::*;
use language_core::equation_preview::{MathIncludeRequest, MathSourceScanner};
use std::collections::HashMap;

struct Host {
    engine: EquationPreviewEngine,
    text: String,
    now: u64,
    revision: u64,
    log: Vec<EquationPreviewCommand>,
    answers_includes_as_missing: bool,
}

impl Host {
    fn new(settings: EquationPreviewSettings) -> Self {
        Self {
            engine: EquationPreviewEngine::new(settings, EquationPreviewAppearance::default()),
            text: String::new(),
            now: 1_000,
            revision: 0,
            log: Vec::new(),
            answers_includes_as_missing: true,
        }
    }

    /// Logs commands; like a host, answers include requests (unknown
    /// targets are missing files) unless a test answers them itself.
    fn record(&mut self, commands: Vec<EquationPreviewCommand>) {
        let requests: Vec<MathIncludeRequest> = commands
            .iter()
            .filter_map(|c| match c {
                EquationPreviewCommand::ResolveIncludes(r) => Some(r.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        self.log.extend(commands);
        if self.answers_includes_as_missing && !requests.is_empty() {
            let now = self.now;
            let answers = requests.into_iter().map(|r| (r.key, None)).collect();
            let commands = self.engine.includes_resolved(answers, HashMap::new(), now);
            self.record(commands);
        }
    }

    /// Records the commands an engine call returns (the Swift tests'
    /// `host.record(engine.foo(nowMs: host.now))`).
    fn act(&mut self, f: impl FnOnce(&mut EquationPreviewEngine, u64) -> Vec<EquationPreviewCommand>) {
        let now = self.now;
        let commands = f(&mut self.engine, now);
        self.record(commands);
    }

    /// Opens `marked` (‸ = caret) as the focused document.
    fn open(&mut self, marked: &str, file_id: &str, ready: bool) {
        let (source, caret) = split_marked(marked);
        self.text = source;
        self.revision += 1;
        let revision = self.revision;
        self.act(|e, now| e.open_document(Some(file_id.to_string()), revision, now));
        self.act(|e, now| e.focus_changed(true, now));
        if ready {
            self.act(|e, now| e.renderer_ready("test-renderer", now));
        }
        if let Some(caret) = caret {
            self.act(|e, now| e.selection_changed(caret, 0, revision, now));
        }
    }

    fn open_default(&mut self, marked: &str) {
        self.open(marked, "/p/main.tex", true);
    }

    /// An edit: new text and its post-edit caret at the new revision.
    fn edit(&mut self, marked: &str) {
        let (source, caret) = split_marked(marked);
        self.text = source;
        self.revision += 1;
        let revision = self.revision;
        if let Some(caret) = caret {
            self.act(|e, now| e.selection_changed(caret, 0, revision, now));
        }
        self.act(|e, now| e.text_changed(revision, now));
    }

    fn move_caret(&mut self, location: usize, length: usize) {
        let revision = self.revision;
        self.act(|e, now| e.selection_changed(location, length, revision, now));
    }

    /// Advances the clock, polling at every deadline on the way.
    fn advance(&mut self, milliseconds: u64) {
        let end = self.now + milliseconds;
        while let Some(deadline) = self.engine.next_deadline() {
            if deadline > end {
                break;
            }
            self.now = self.now.max(deadline);
            let text = self.text.clone();
            let commands = self.engine.poll(self.now, &|| text.clone());
            self.record(commands);
        }
        self.now = end;
    }

    fn renders(&self) -> Vec<EquationRenderRequest> {
        self.log
            .iter()
            .filter_map(|c| match c {
                EquationPreviewCommand::Render(r) => Some(r.clone()),
                _ => None,
            })
            .collect()
    }

    fn exact_renders(&self) -> Vec<ExactEquationRequest> {
        self.log
            .iter()
            .filter_map(|c| match c {
                EquationPreviewCommand::RenderExact(r) => Some(r.clone()),
                _ => None,
            })
            .collect()
    }

    fn shows(&self) -> Vec<EquationPreviewPresentation> {
        self.log
            .iter()
            .filter_map(|c| match c {
                EquationPreviewCommand::Show(p) => Some(p.clone()),
                _ => None,
            })
            .collect()
    }

    /// Visible state after replaying show/hide.
    fn visible(&self) -> Option<EquationPreviewPresentation> {
        let mut current = None;
        for command in &self.log {
            match command {
                EquationPreviewCommand::Show(p) => current = Some(p.clone()),
                EquationPreviewCommand::Hide => current = None,
                _ => {}
            }
        }
        current
    }

    fn complete(&mut self, request: &EquationRenderRequest, outcome: EquationRenderOutcome) {
        let key = request.key.clone();
        self.act(|e, now| e.fast_render_completed(&key, outcome, now));
    }

    fn complete_latest_svg(&mut self, svg: &str) {
        let key = self.renders().last().unwrap().key.clone();
        self.act(|e, now| e.fast_render_completed(&key, EquationRenderOutcome::Svg(svg.to_string()), now));
    }

    fn fail_latest(&mut self, failure: EquationRenderFailure) {
        let key = self.renders().last().unwrap().key.clone();
        self.act(|e, now| e.fast_render_completed(&key, EquationRenderOutcome::Failed(failure), now));
    }
}

/// Marker → source without marker + char offset of the caret (Swift tests
/// use utf16 offsets; these sources are ASCII so chars == units).
fn split_marked(marked: &str) -> (String, Option<usize>) {
    match marked.find('\u{2038}') {
        Some(byte) => (
            marked.replacen('\u{2038}', "", 1),
            Some(marked[..byte].chars().count()),
        ),
        None => (marked.to_string(), None),
    }
}

fn fast(svg: &str) -> EquationPreviewContent {
    EquationPreviewContent::Fast { svg: svg.to_string() }
}

#[test]
fn caret_entering_math_renders_and_leaving_hides() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("Text $a‸+b$ more.");
    host.advance(100);
    assert_eq!(host.renders().iter().map(|r| r.source.as_str()).collect::<Vec<_>>(), ["a+b"]);
    assert!(!host.renders()[0].display_mode);
    host.complete_latest_svg("<svg id='ab'/>");
    let visible = host.visible().unwrap();
    assert_eq!(visible.content, fast("<svg id='ab'/>"));
    assert_eq!(visible.anchor, 5..10);
    assert_eq!(visible.trigger, EquationPreviewTrigger::Caret);

    host.move_caret(2, 0);
    host.advance(100);
    assert!(host.visible().is_none());
}

#[test]
fn display_mode_distinguishes_delimiters() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("\\[x‸^2\\]");
    host.advance(100);
    let request = host.renders().last().unwrap().clone();
    assert!(request.display_mode);
    assert_eq!(request.source, "x^2");
}

#[test]
fn hover_previews_another_equation_without_moving_caret() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$ and $b$");
    host.advance(100);
    host.complete_latest_svg("A");
    assert_eq!(host.visible().unwrap().content, fast("A"));

    let b_offset = host.text.find('b').unwrap();
    host.act(|e, now| e.hover(Some(b_offset), now));
    host.advance(100);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("b"));
    host.complete_latest_svg("B");
    let visible = host.visible().unwrap();
    assert_eq!(visible.content, fast("B"));
    assert_eq!(visible.trigger, EquationPreviewTrigger::Hover);

    // Pointer leaves: a grace period, then the caret equation returns.
    host.act(|e, now| e.hover(None, now));
    host.advance(100);
    assert_eq!(host.visible().unwrap().content, fast("B"));
    host.advance(300);
    let visible = host.visible().unwrap();
    assert_eq!(visible.content, fast("A"));
    assert_eq!(visible.trigger, EquationPreviewTrigger::Caret);
}

#[test]
fn hover_survives_the_move_onto_the_popover() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("Intro text\n$x^2$ tail");
    // Upstream bug: Swift `range(of: "x")` resolves to the x in "text"
    // (index 8) and the test crashes there; "^" selects the math content.
    let x = host.text.find('^').unwrap();
    host.act(|e, now| e.hover(Some(x), now));
    host.advance(100);
    host.complete_latest_svg("X");
    // Crossing plain text, then entering the popover.
    host.act(|e, now| e.hover(Some(2), now));
    host.advance(100);
    host.act(|e, now| e.pointer_in_popover(true, now));
    host.advance(1_000);
    assert_eq!(host.visible().unwrap().content, fast("X"));
    host.act(|e, now| e.pointer_in_popover(false, now));
    host.act(|e, now| e.hover(None, now));
    host.advance(400);
    assert!(host.visible().is_none());
}

/// Spec 26.4: a → ab → abc → abcd with a slow renderer; only abcd may ever
/// become visible.
#[test]
fn rapid_edits_publish_only_the_newest_result() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$");
    host.advance(100);
    let first = host.renders()[0].clone();
    for marked in ["$ab‸$", "$abc‸$", "$abcd‸$"] {
        host.edit(marked);
        host.advance(90);
    }
    // Still only the first request is in flight; later ones coalesce.
    assert_eq!(host.renders().iter().map(|r| r.source.as_str()).collect::<Vec<_>>(), ["a"]);
    host.complete(&first, EquationRenderOutcome::Svg("<s/>".to_string()));
    assert!(host.shows().is_empty());
    assert_eq!(
        host.renders().iter().map(|r| r.source.as_str()).collect::<Vec<_>>(),
        ["a", "abcd"]
    );
    let second = host.renders()[1].clone();
    let expected = format!("<svg data-source=\"{}\"/>", second.source);
    host.complete(&second, EquationRenderOutcome::Svg(expected.clone()));
    assert_eq!(
        host.shows().iter().map(|s| s.content.clone()).collect::<Vec<_>>(),
        [fast(&expected)]
    );
}

#[test]
fn late_result_after_an_edit_is_not_shown() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$x‸$");
    host.advance(100);
    let request = host.renders()[0].clone();
    host.edit("$x+‸$");
    host.complete(&request, EquationRenderOutcome::Svg("stale".to_string()));
    assert!(host.visible().is_none());
    // The newer revision renders on its own.
    host.advance(100);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("x+"));
}

#[test]
fn caret_moves_inside_one_equation_reuse_the_cache() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸bc$");
    host.advance(100);
    host.complete_latest_svg("ABC");
    host.move_caret(3, 0);
    host.advance(100);
    host.move_caret(4, 0);
    host.advance(100);
    assert_eq!(host.renders().len(), 1);
    assert_eq!(host.visible().unwrap().content, fast("ABC"));
}

/// Spec 13: an invalid intermediate keeps the last preview ~300 ms, then hides.
#[test]
fn invalid_intermediate_keeps_last_good_briefly_then_hides() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$\\frac{1}{2}‸$");
    host.advance(100);
    host.complete_latest_svg("HALF");
    host.edit("$\\frac{1}{2}{‸$");
    host.advance(100);
    assert_eq!(host.visible().unwrap().content, fast("HALF"));
    host.advance(300);
    assert!(host.visible().is_none());
    assert!(!host
        .log
        .iter()
        .any(|c| matches!(c, EquationPreviewCommand::RenderExact(_))));
}

/// Spec 26.3: every intermediate state is handled without a crash, modal
/// error or stale overwrite.
#[test]
fn incomplete_editing_sequence() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("");
    let mut answered = 0usize;
    for marked in ["$‸", "$\\frac‸", "$\\frac{‸", "$\\frac{1}‸", "$\\frac{1}{‸"] {
        host.edit(marked);
        host.advance(90);
        while answered < host.renders().len() {
            let request = host.renders()[answered].clone();
            answered += 1;
            let outcome = if request.source == "\\frac{1}{2}" {
                EquationRenderOutcome::Svg("HALF".to_string())
            } else {
                EquationRenderOutcome::Failed(EquationRenderFailure::Invalid)
            };
            host.complete(&request, outcome);
        }
        assert!(host.visible().is_none(), "{marked}");
    }
    host.edit("$\\frac{1}{2}‸$");
    host.advance(90);
    while answered < host.renders().len() {
        let request = host.renders()[answered].clone();
        answered += 1;
        let outcome = if request.source == "\\frac{1}{2}" {
            EquationRenderOutcome::Svg("HALF".to_string())
        } else {
            EquationRenderOutcome::Failed(EquationRenderFailure::Invalid)
        };
        host.complete(&request, outcome);
    }
    assert_eq!(host.visible().unwrap().content, fast("HALF"));
    assert_eq!(host.shows().len(), 1);
}

#[test]
fn redefinition_changes_the_cache_key() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("\\newcommand{\\R}{\\mathbb{R}}\n$x \\in \\R‸$");
    host.advance(100);
    let first = host.renders()[0].clone();
    assert_eq!(first.definitions, ["\\newcommand{\\R}{\\mathbb{R}}"]);
    host.complete(&first, EquationRenderOutcome::Svg("BB".to_string()));

    host.edit("\\renewcommand{\\R}{\\mathbf{R}}\n$x \\in \\R‸$");
    host.advance(100);
    let second = host.renders()[1].clone();
    assert_eq!(second.definitions, ["\\renewcommand{\\R}{\\mathbf{R}}"]);
    assert_ne!(first.context_key(), second.context_key());
    host.complete(&second, EquationRenderOutcome::Svg("BF".to_string()));
    assert_eq!(host.visible().unwrap().content, fast("BF"));

    // Back to the original definition: served from the cache.
    host.edit("\\newcommand{\\R}{\\mathbb{R}}\n$x \\in \\R‸$");
    host.advance(100);
    assert_eq!(host.renders().len(), 2);
    assert_eq!(host.visible().unwrap().content, fast("BB"));
}

/// State isolation across documents: a definition in document A must not
/// reach document B's request.
#[test]
fn definitions_do_not_leak_across_documents() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open("\\newcommand{\\R}{\\mathbb{R}}\n$\\R‸$", "/a/a.tex", true);
    host.advance(100);
    let from_a = host.renders()[0].clone();
    host.complete(&from_a, EquationRenderOutcome::Svg("A".to_string()));
    host.open("$\\R‸$", "/b/b.tex", true);
    host.advance(100);
    let from_b = host.renders()[1].clone();
    assert_eq!(from_b.definitions, Vec::<String>::new());
    assert_ne!(from_a.context_key(), from_b.context_key());
    assert_ne!(from_a.key, from_b.key);
}

#[test]
fn included_definitions_resolve_through_the_host() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.answers_includes_as_missing = false;
    host.open_default("\\input{commands}\n$x \\in \\R‸$");
    host.advance(100);
    let requests: Vec<MathIncludeRequest> = host
        .log
        .iter()
        .filter_map(|c| match c {
            EquationPreviewCommand::ResolveIncludes(r) => Some(r.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(
        requests.iter().map(|r| r.candidates.clone()).collect::<Vec<_>>(),
        vec![vec!["/p/commands.tex".to_string()]]
    );
    // No render with a known-incomplete context (it would flash "unavailable").
    assert!(host.renders().is_empty());
    let scan = MathSourceScanner::scan("\\newcommand{\\R}{\\mathbb{R}}\n");
    let request = requests.first().expect("include request").clone();
    host.act(|e, now| {
        e.includes_resolved(
            vec![(request.key.clone(), Some("/p/commands.tex".to_string()))],
            HashMap::from([("/p/commands.tex".to_string(), scan)]),
            now,
        )
    });
    host.advance(10);
    assert_eq!(
        host.renders().iter().map(|r| r.definitions.clone()).collect::<Vec<_>>(),
        vec![vec!["\\newcommand{\\R}{\\mathbb{R}}".to_string()]]
    );
}

#[test]
fn escape_hides_until_another_equation() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$ and $b$");
    host.advance(100);
    host.complete_latest_svg("A");
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(consumed);
    assert!(host.visible().is_none());
    host.move_caret(1, 0);
    host.advance(100);
    assert!(host.visible().is_none());
    let b = host.text.find('b').unwrap();
    host.move_caret(b, 0);
    host.advance(100);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("b"));
    host.complete_latest_svg("B");
    assert_eq!(host.visible().unwrap().content, fast("B"));
}

#[test]
fn focus_loss_composition_selection_and_disable_hide() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$");
    host.advance(100);
    host.complete_latest_svg("A");
    host.act(|e, now| e.focus_changed(false, now));
    assert!(host.visible().is_none());
    host.act(|e, now| e.focus_changed(true, now));
    host.advance(100);
    assert_eq!(host.visible().unwrap().content, fast("A"));

    host.act(|e, now| e.composition_changed(true, now));
    assert!(host.visible().is_none());
    host.advance(500);
    assert!(host.visible().is_none());
    host.act(|e, now| e.composition_changed(false, now));
    host.advance(100);
    assert_eq!(host.visible().unwrap().content, fast("A"));

    host.move_caret(1, 1);
    host.advance(100);
    assert!(host.visible().is_none());
    host.move_caret(1, 0);
    host.advance(100);
    assert_eq!(host.visible().unwrap().content, fast("A"));

    host.act(|e, now| {
        e.set_settings(
            EquationPreviewSettings { enabled: false, ..EquationPreviewSettings::default() },
            now,
        )
    });
    assert!(host.visible().is_none());
    host.move_caret(2, 0);
    host.advance(500);
    assert!(host.visible().is_none());
    assert_eq!(host.renders().len(), 1);
}

#[test]
fn preview_while_typing_off_waits_for_navigation() {
    let mut host = Host::new(EquationPreviewSettings {
        while_typing: false,
        ..EquationPreviewSettings::default()
    });
    host.open_default("$a‸$");
    host.advance(100);
    host.complete_latest_svg("A");
    host.edit("$ab‸$");
    assert!(host.visible().is_none());
    host.advance(500);
    assert!(host.visible().is_none());
    assert_eq!(host.renders().len(), 1);
    host.move_caret(2, 0);
    host.advance(100);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("ab"));
}

#[test]
fn delay_setting_controls_the_typing_debounce() {
    let mut host = Host::new(EquationPreviewSettings {
        delay_milliseconds: 150,
        ..EquationPreviewSettings::default()
    });
    host.open_default("");
    host.advance(40);
    host.edit("$ab‸$");
    host.advance(149);
    assert!(host.renders().is_empty());
    host.advance(1);
    assert_eq!(host.renders().iter().map(|r| r.source.as_str()).collect::<Vec<_>>(), ["ab"]);

    let mut instant = Host::new(EquationPreviewSettings {
        delay_milliseconds: 0,
        ..EquationPreviewSettings::default()
    });
    instant.open_default("$a‸$");
    instant.advance(0);
    assert_eq!(
        instant.renders().iter().map(|r| r.source.as_str()).collect::<Vec<_>>(),
        ["a"]
    );
}

#[test]
fn unknown_command_is_unavailable_in_fast_mode() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$\\mySpecialOperator{x}‸$");
    host.advance(100);
    host.fail_latest(EquationRenderFailure::UndefinedCommand);
    assert_eq!(
        host.visible().unwrap().content,
        EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::Unsupported)
    );
    host.advance(5_000);
    assert!(host.exact_renders().is_empty());
}

/// The exact TeX fallback waits for the equation to settle — never once per
/// keystroke — and follows the latest equation only.
#[test]
fn exact_fallback_runs_only_after_settling() {
    let mut host = Host::new(EquationPreviewSettings {
        renderer: EquationPreviewRendererMode::FastWithTexFallback,
        ..EquationPreviewSettings::default()
    });
    host.act(|e, now| e.set_exact_profile(Some("pdflatex|projectDefault".to_string()), now));
    host.open_default(
        "\\documentclass{article}\n\\usepackage{mine}\n\\begin{document}\n$\\mine{a}‸$\n\\end{document}",
    );
    for marked in ["$\\mine{ab}‸$", "$\\mine{abc}‸$"] {
        host.advance(100);
        if !host.renders().is_empty() {
            host.fail_latest(EquationRenderFailure::UndefinedCommand);
        }
        host.edit(&format!(
            "\\documentclass{{article}}\n\\usepackage{{mine}}\n\\begin{{document}}\n{marked}\n\\end{{document}}"
        ));
    }
    host.advance(100);
    host.fail_latest(EquationRenderFailure::UndefinedCommand);
    assert!(host.exact_renders().is_empty());
    host.advance(700);
    let exact = host.exact_renders()[0].clone();
    assert_eq!(host.exact_renders().len(), 1);
    assert!(exact.document.starts_with("\\documentclass{article}\n\\usepackage{mine}\n"));
    assert!(exact.document.contains("$\\mine{abc}$"));
    host.act(|e, now| {
        e.exact_render_completed(&exact.key, ExactEquationOutcome::Pdf(vec![1, 2, 3]), now)
    });
    assert_eq!(
        host.visible().unwrap().content,
        EquationPreviewContent::Exact { pdf: vec![1, 2, 3] }
    );
}

#[test]
fn exact_job_is_cancelled_when_the_equation_changes() {
    let mut host = Host::new(EquationPreviewSettings {
        renderer: EquationPreviewRendererMode::FastWithTexFallback,
        ..EquationPreviewSettings::default()
    });
    host.act(|e, now| e.set_exact_profile(Some("xelatex|disabled".to_string()), now));
    host.open_default("$\\foo‸$ and $b$");
    host.advance(100);
    host.fail_latest(EquationRenderFailure::UndefinedCommand);
    host.advance(700);
    let exact_key = host.exact_renders()[0].key.clone();
    let b = host.text.find('b').unwrap();
    host.move_caret(b, 0);
    host.advance(100);
    assert!(host
        .log
        .iter()
        .any(|c| matches!(c, EquationPreviewCommand::CancelExact { key } if *key == exact_key)));
    host.act(|e, now| {
        e.exact_render_completed(&exact_key, ExactEquationOutcome::Pdf(vec![9]), now)
    });
    assert_ne!(host.visible().map(|v| v.content), Some(EquationPreviewContent::Exact { pdf: vec![9] }));
}

#[test]
fn fallback_without_supported_engine_says_so() {
    let mut host = Host::new(EquationPreviewSettings {
        renderer: EquationPreviewRendererMode::FastWithTexFallback,
        ..EquationPreviewSettings::default()
    });
    host.open_default("$\\foo‸$");
    host.advance(100);
    host.fail_latest(EquationRenderFailure::UndefinedCommand);
    assert_eq!(
        host.visible().unwrap().content,
        EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::ExactUnavailable)
    );
}

#[test]
fn explicit_exact_request_works_in_fast_mode() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.act(|e, now| e.set_exact_profile(Some("lualatex|projectDefault".to_string()), now));
    host.open_default("$x^2‸$");
    host.advance(100);
    host.complete_latest_svg("X2");
    host.act(|e, now| e.request_exact(now));
    host.advance(1);
    let exact = host.exact_renders()[0].clone();
    assert!(exact.document.contains("\\documentclass{article}"));
    host.act(|e, now| {
        e.exact_render_completed(&exact.key, ExactEquationOutcome::Pdf(vec![7]), now)
    });
    assert_eq!(
        host.visible().unwrap().content,
        EquationPreviewContent::Exact { pdf: vec![7] }
    );
}

#[test]
fn theme_change_represents_without_rerender() {
    // C10: scheme is applied at show-time — a theme change re-presents the
    // cached SVG instead of paying a second typeset.
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$");
    host.advance(100);
    host.complete_latest_svg("light");
    host.act(|e, now| {
        e.set_appearance(
            EquationPreviewAppearance { scheme: EquationPreviewScheme::Dark, font_size: 13.0 },
            now,
        )
    });
    host.advance(1);
    assert_eq!(host.renders().len(), 1, "scheme change must not re-render");
    assert_eq!(
        host.visible().unwrap().content,
        EquationPreviewContent::Fast { svg: "light".into() }
    );
}

#[test]
fn font_size_change_rerenders() {
    // font_size is in the cache key: it changes em/ex metrics.
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$");
    host.advance(100);
    host.complete_latest_svg("light");
    host.act(|e, now| {
        e.set_appearance(
            EquationPreviewAppearance { scheme: EquationPreviewScheme::Light, font_size: 14.0 },
            now,
        )
    });
    host.advance(1);
    assert_eq!(host.renders().len(), 2);
    assert_eq!(host.renders()[1].key.font_size, 14.0);
}

#[test]
fn renderer_version_change_drops_the_cache() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$");
    host.advance(100);
    host.complete_latest_svg("v1");
    host.act(|e, now| e.renderer_ready("other", now));
    host.advance(1);
    assert_eq!(host.renders().len(), 2);
}

/// Spec 26.5: dozens of aligned rows go to the renderer whole; the size
/// bound turns only pathological sources into "unavailable".
#[test]
fn large_aligned_expression_and_bounds() {
    let rows: String = (1..=60)
        .map(|i| format!("a_{{{i}}} &= b_{{{i}}} + c_{{{i}}} \\\\"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default(&format!("\\begin{{aligned}}\n{rows}‸\n\\end{{aligned}}"));
    host.advance(100);
    let request = host.renders().last().unwrap().clone();
    assert_eq!(request.source, format!("\\begin{{aligned}}\n{rows}\n\\end{{aligned}}"));
    assert!(request.display_mode);

    let mut huge = Host::new(EquationPreviewSettings::default());
    let big_source = format!("${}‸$", "x+".repeat(EquationPreviewEngine::MAXIMUM_SOURCE_BYTES));
    huge.open_default(&big_source);
    huge.advance(100);
    assert!(huge.renders().is_empty());
    assert_eq!(
        huge.visible().unwrap().content,
        EquationPreviewContent::Unavailable(EquationPreviewUnavailableReason::TooLarge)
    );
}

#[test]
fn cache_is_bounded_and_evicts_least_recently_used() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$x0‸$");
    for index in 0..=EquationPreviewEngine::CACHE_CAPACITY {
        host.edit(&format!("$x{index}‸$"));
        host.advance(100);
        host.complete_latest_svg(&format!("s{index}"));
    }
    let before = host.renders().len();
    host.edit("$x0‸$");
    host.advance(100);
    assert_eq!(host.renders().len(), before + 1, "x0 was evicted");
    let last = EquationPreviewEngine::CACHE_CAPACITY;
    host.edit(&format!("$x{last}‸$"));
    host.advance(100);
    assert_eq!(host.renders().len(), before + 1, "recent entry still cached");
}

#[test]
fn renderer_not_ready_defers_requests() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open("$a‸$", "/p/main.tex", false);
    host.advance(100);
    assert!(host.renders().is_empty());
    host.act(|e, now| e.renderer_ready("late", now));
    host.advance(1);
    assert_eq!(host.renders().iter().map(|r| r.source.as_str()).collect::<Vec<_>>(), ["a"]);
}

/// B3: pointer parked over B while typing edits A — the stale hover anchor
/// must be dropped, and the caret's equation takes over after the delay.
#[test]
fn typing_supersedes_a_stale_hover_target() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$ and $b$");
    host.advance(100);
    host.complete_latest_svg("A");
    let b_offset = host.text.find('b').unwrap();
    host.act(|e, now| e.hover(Some(b_offset), now));
    host.advance(100);
    host.complete_latest_svg("B");
    assert_eq!(host.visible().unwrap().trigger, EquationPreviewTrigger::Hover);

    host.edit("$ax‸$ and $b$");
    host.advance(100);
    let source = host.renders().last().map(|r| r.source.clone());
    assert_eq!(source.as_deref(), Some("ax"), "caret equation renders, not the stale hover target");
    host.complete_latest_svg("AX");
    let visible = host.visible().unwrap();
    assert_eq!(visible.content, fast("AX"));
    assert_eq!(visible.trigger, EquationPreviewTrigger::Caret);
}

/// C6 r2: grace expires while the pointer already rests on equation C —
/// expiry must clear the hover TARGET, not the location, so this refire
/// resolves C instead of falling back to the caret.
#[test]
fn grace_expiry_with_pointer_on_new_equation_shows_it() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$ x $b$ x $c$");
    host.advance(100);
    host.complete_latest_svg("A");
    let b = host.text.find('b').unwrap();
    host.act(|e, now| e.hover(Some(b), now));
    host.advance(100);
    host.complete_latest_svg("B");
    assert_eq!(host.visible().unwrap().trigger, EquationPreviewTrigger::Hover);

    // Pointer leaves the text (grace starts), then lands on C before the
    // deadline; the expiry tick beats the pending hover fire.
    host.act(|e, now| e.hover(None, now));
    host.advance(200);
    let c = host.text.find('c').unwrap();
    host.act(|e, now| e.hover(Some(c), now));
    host.advance(300);
    let source = host.renders().last().map(|r| r.source.clone());
    assert_eq!(source.as_deref(), Some("c"), "grace expiry must not discard the resting location");
    host.complete_latest_svg("C");
    let visible = host.visible().unwrap();
    assert_eq!(visible.content, fast("C"));
    assert_eq!(visible.trigger, EquationPreviewTrigger::Hover);
}

/// C13: persisted delay extremes — a stored negative is invalid (conservative
/// 80), while stored 0/80/150 keep their exact meaning.
#[test]
fn persisted_delay_normalizes_invalid_values() {
    for (stored, expected) in [(-1i64, 80u64), (i64::MIN, 80), (i64::MAX, 80), (0, 0), (80, 80), (150, 150), (999, 80)] {
        let s = EquationPreviewSettings::from_persisted(true, true, "above", "fast", stored);
        assert_eq!(s.delay_milliseconds, expected, "stored {stored}");
    }
}


/// N5: Escape with a pending (not yet visible) math preview must consume
/// the key and suppress that preview until the target changes.
#[test]
fn escape_consumes_a_pending_math_preview() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$ and $b$");
    host.advance(100);
    host.complete_latest_svg("A");
    assert!(host.visible().is_some());
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(consumed);
    // Caret on $b$, fire still counting down: Escape dismisses the
    // pending preview before it ever renders.
    let b = host.text.find('b').unwrap();
    host.move_caret(b, 0);
    let renders_before = host.renders().len();
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(consumed, "pending math preview must consume Escape");
    host.advance(200);
    assert_eq!(host.renders().len(), renders_before, "pending b never rendered");
    assert!(host.visible().is_none());
    // A different equation is a new target: suppression ends there.
    host.move_caret(1, 0);
    host.advance(100);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("a"));
}

/// N5: a dormant target (stored without a live render, e.g. renderer not
/// ready yet) must not be poisoned by Escape.
#[test]
fn escape_ignores_a_dormant_target() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open("$a‸$", "/p/main.tex", false); // renderer never ready
    host.advance(200); // fire stored the target, nothing rendered
    assert!(host.visible().is_none());
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(!consumed, "dormant target is not dismissible work");
    host.act(|e, now| e.renderer_ready("test-renderer", now));
    host.advance(100);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("a"));
}

/// N5: a pending fire on plain text is not an eligible preview — Escape
/// passes through and nothing is suppressed.
#[test]
fn escape_ignores_a_pending_plain_text_fire() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("plain ‸text and $a$");
    host.advance(200); // first fire ran (hide), snapshot current
    // Re-schedule while still on plain text: fire deadline is pending.
    host.move_caret(host.text.find('x').unwrap(), 0);
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(!consumed, "pending plain-text fire must not consume Escape");
    host.move_caret(host.text.rfind('a').unwrap(), 0);
    host.advance(100);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("a"));
}

/// N5: an in-flight render is consumable work — Escape cancels it and the
/// late reply must not publish.
#[test]
fn escape_cancels_in_flight_render() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$");
    host.advance(100);
    assert_eq!(host.renders().len(), 1); // issued, never completed
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(consumed);
    host.complete_latest_svg("STALE");
    assert!(host.visible().is_none());
    host.advance(500);
    assert!(host.visible().is_none());
}

/// N5 r2: a second Escape inside the already-dismissed equation falls
/// through — the pending fire was filtered as resolved-nothing, so the
/// key is not consumed for a no-op.
#[test]
fn escape_falls_through_when_pending_is_already_dismissed() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$ and $b$");
    host.advance(100);
    host.complete_latest_svg("A");
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(consumed);
    assert!(host.visible().is_none());
    // Caret on $b$: first Escape dismisses the pending fire for b.
    let b = host.text.find('b').unwrap();
    host.move_caret(b, 0);
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(consumed);
    // Move inside $b$ again (another pending fire on the SAME dismissed
    // region): Escape must now fall through.
    host.move_caret(b + 1, 0);
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(!consumed, "second Escape on the dismissed region is a no-op");
    host.advance(200);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("a"));
    assert!(host.visible().is_none());
}

/// N5 r2: after an edit the cached snapshot is stale, so a pending fire
/// is unverifiable — Escape declines without touching engine state, and
/// the fire then renders the updated text.
#[test]
fn escape_declines_when_the_snapshot_is_stale() {
    let mut host = Host::new(EquationPreviewSettings::default());
    host.open_default("$a‸$ and tail");
    host.advance(100);
    host.complete_latest_svg("A");
    // Retire the visible branch so this is purely the pending-fire path.
    let (consumed, commands) = host.engine.escape(host.now);
    host.log.extend(commands);
    assert!(consumed);
    host.move_caret(host.text.find("tail").unwrap(), 0);
    host.advance(200);
    // Edit: caret lands inside a DIFFERENT-offset equation ($ab$ at 2 —
    // not the dismissed region at 0); snapshot revision no longer matches.
    host.edit("x $ab‸$ and tail");
    let deadline_before = host.engine.next_deadline();
    assert!(deadline_before.is_some(), "edit schedules a fire");
    let (consumed, commands) = host.engine.escape(host.now);
    assert!(!consumed, "stale snapshot: eligibility unknown, decline");
    assert!(commands.is_empty(), "declined Escape mutates nothing");
    host.log.extend(commands);
    assert_eq!(host.engine.next_deadline(), deadline_before, "fire untouched");
    host.advance(200);
    assert_eq!(host.renders().last().map(|r| r.source.as_str()), Some("ab"));
}
