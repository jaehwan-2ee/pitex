//! Port of `Packages/TexCore/Tests/TexCoreTests/BuildOrchestratorTests.swift`.
#![cfg(unix)] // POSIX process semantics — the Windows runner is covered by CI builds.

use build_core::*;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};

// ---------------------------------------------------------------------------
// Fixture

struct Fixture {
    root: PathBuf,
    source_url: PathBuf,
    output_url: PathBuf,
    target: BuildTarget,
}

impl Fixture {
    fn new(extra_generated: &[&str]) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "BuildOrchestratorTests-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_url = root.join("main.tex");
        let output_url = root.join("main.pdf");
        std::fs::write(&source_url, "source").unwrap();
        let mut generated: HashSet<String> =
            extra_generated.iter().map(|s| s.to_string()).collect();
        generated.insert("main.pdf".to_string());
        let target = BuildTarget::new(
            &root,
            "main.tex",
            "main.pdf",
            generated,
            if extra_generated.contains(&"build") {
                Some("build".to_string())
            } else {
                None
            },
        )
        .unwrap();
        Self {
            root,
            source_url,
            output_url,
            target,
        }
    }

    /// A nested-source project built into `.pitex-live/`.
    fn live(source_path: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "BuildOrchestratorTests-{}-{nanos}",
            std::process::id()
        ));
        let source_url = root.join(source_path);
        std::fs::create_dir_all(source_url.parent().unwrap()).unwrap();
        std::fs::write(&source_url, "source").unwrap();
        let target = BuildTarget::live(&root, source_path).unwrap();
        let output_url = target.project_root.join(&target.output_pdf_path);
        Self {
            root,
            source_url,
            output_url,
            target,
        }
    }

    /// Paths resolved the way `BuildTarget` sees them (project root has
    /// symlinks resolved at initialization).
    fn resolved(&self, relative_path: &str) -> PathBuf {
        self.target.project_root.join(relative_path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

// ---------------------------------------------------------------------------
// FakeExecutor — a Mutex+Condvar port of the Swift actor fake.

enum Behavior {
    Success {
        chunks: Vec<BuildProcessOutput>,
        pdf: Option<Vec<u8>>,
    },
    Failure {
        exit_code: i32,
        chunks: Vec<BuildProcessOutput>,
        partial_pdf: Option<Vec<u8>>,
    },
    ToolMissing(String),
    WaitForCancellation {
        partial_pdf: Option<Vec<u8>>,
    },
}

struct FakeState {
    behaviors: VecDeque<Behavior>,
    requests: Vec<BuildProcessRequest>,
    pending: Option<(BuildProcessRequest, Option<Vec<u8>>)>,
}

struct FakeExecutor {
    state: Mutex<FakeState>,
    started: Condvar,
    resumed: Condvar,
    pdf_path: String,
}

impl FakeExecutor {
    fn new(behaviors: Vec<Behavior>) -> Self {
        Self::with_pdf_path(behaviors, "main.pdf")
    }

    fn with_pdf_path(behaviors: Vec<Behavior>, pdf_path: &str) -> Self {
        Self {
            state: Mutex::new(FakeState {
                behaviors: behaviors.into(),
                requests: Vec::new(),
                pending: None,
            }),
            started: Condvar::new(),
            resumed: Condvar::new(),
            pdf_path: pdf_path.to_string(),
        }
    }

    fn wait_until_started(&self) {
        let mut state = self.state.lock().unwrap();
        while state.pending.is_none() {
            let (guard, _) = self
                .started
                .wait_timeout(state, std::time::Duration::from_secs(5))
                .unwrap();
            state = guard;
            if state.pending.is_none() {
                panic!("executor never reached the pending state");
            }
        }
    }

    fn request_count(&self) -> usize {
        self.state.lock().unwrap().requests.len()
    }

    fn executables(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .requests
            .iter()
            .filter_map(|r| match &r.command {
                BuildProcessCommand::Direct(plan) => Some(plan.executable.clone()),
                _ => None,
            })
            .collect()
    }

    fn direct_arguments(&self) -> Vec<Vec<String>> {
        self.state
            .lock()
            .unwrap()
            .requests
            .iter()
            .filter_map(|r| match &r.command {
                BuildProcessCommand::Direct(plan) => Some(plan.arguments.clone()),
                _ => None,
            })
            .collect()
    }

    fn shell_commands(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .requests
            .iter()
            .filter_map(|r| match &r.command {
                BuildProcessCommand::LoginShell(plan) => Some(plan.command.clone()),
                _ => None,
            })
            .collect()
    }

    fn write_pdf(&self, data: &Option<Vec<u8>>, request: &BuildProcessRequest) {
        if let Some(data) = data {
            let path = request.project_root.join(&self.pdf_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, data).unwrap();
        }
    }
}

impl BuildProcessExecuting for FakeExecutor {
    fn execute(
        &self,
        request: &BuildProcessRequest,
        output: &mut (dyn FnMut(BuildProcessOutput) + Send),
    ) -> Result<BuildProcessResult, BuildProcessExecutorError> {
        let behavior = {
            let mut state = self.state.lock().unwrap();
            state.requests.push(request.clone());
            state
                .behaviors
                .pop_front()
                .ok_or_else(|| BuildProcessExecutorError::LaunchFailed("unexpected request".into()))?
        };
        match behavior {
            Behavior::Success { chunks, pdf } => {
                for chunk in chunks {
                    output(chunk);
                }
                self.write_pdf(&pdf, request);
                Ok(BuildProcessResult { exit_code: 0 })
            }
            Behavior::Failure {
                exit_code,
                chunks,
                partial_pdf,
            } => {
                for chunk in chunks {
                    output(chunk);
                }
                self.write_pdf(&partial_pdf, request);
                Ok(BuildProcessResult { exit_code })
            }
            Behavior::ToolMissing(tool) => Err(BuildProcessExecutorError::ToolNotFound(tool)),
            Behavior::WaitForCancellation { partial_pdf } => {
                let mut state = self.state.lock().unwrap();
                state.pending = Some((request.clone(), partial_pdf));
                self.started.notify_all();
                while state.pending.is_some() {
                    let (guard, _) = self
                        .resumed
                        .wait_timeout(state, std::time::Duration::from_secs(5))
                        .unwrap();
                    state = guard;
                }
                Ok(BuildProcessResult { exit_code: 0 })
            }
        }
    }

    fn cancel(&self, build_id: &BuildID) {
        let mut state = self.state.lock().unwrap();
        if let Some((pending_request, partial_pdf)) = state.pending.take() {
            if pending_request.build_id == *build_id {
                self.write_pdf(&partial_pdf, &pending_request);
            }
            self.resumed.notify_all();
        }
    }
}

fn chunk(text: &str, channel: BuildLogChannel) -> BuildProcessOutput {
    BuildProcessOutput {
        channel,
        bytes: text.as_bytes().to_vec(),
    }
}

struct EventRecorder {
    events: Mutex<Vec<BuildEvent>>,
}
impl EventRecorder {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
        })
    }
    fn handler(self: &Arc<Self>) -> EventHandler {
        let this = Arc::clone(self);
        Arc::new(move |event| {
            this.events.lock().unwrap().push(event);
        })
    }
    fn log_sequences(&self) -> Vec<u64> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                BuildEvent::Log(entry) => Some(entry.sequence),
                _ => None,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------

/// Wraps an `Arc<FakeExecutor>` so the test can inspect it after moving the
/// executor into the orchestrator.
struct FakeExecutorRef(Arc<FakeExecutor>);
impl BuildProcessExecuting for FakeExecutorRef {
    fn execute(
        &self,
        request: &BuildProcessRequest,
        output: &mut (dyn FnMut(BuildProcessOutput) + Send),
    ) -> Result<BuildProcessResult, BuildProcessExecutorError> {
        self.0.execute(request, output)
    }
    fn cancel(&self, build_id: &BuildID) {
        self.0.cancel(build_id)
    }
}

#[test]
fn successful_multi_pass_build_runs_in_order_and_publishes_ordered_logs() {
    let fixture = Fixture::new(&[]);
    let fake = Arc::new(FakeExecutor::new(vec![
        Behavior::Success {
            chunks: vec![chunk("first\n", BuildLogChannel::StandardOutput)],
            pdf: None,
        },
        Behavior::Success {
            chunks: vec![chunk("second\n", BuildLogChannel::StandardOutput)],
            pdf: None,
        },
        Behavior::Success {
            chunks: vec![chunk("third\n", BuildLogChannel::StandardOutput)],
            pdf: Some(b"pdf-v1".to_vec()),
        },
    ]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::Stages(vec![
                BuildStagePlan {
                    tool: BuildToolStage::Pdflatex,
                },
                BuildStagePlan {
                    tool: BuildToolStage::Bibtex,
                },
                BuildStagePlan {
                    tool: BuildToolStage::Pdflatex,
                },
            ]),
        )
        .unwrap();
    let events = EventRecorder::new();
    let outcome = orchestrator
        .build(BuildID::new("multi").unwrap(), events.handler())
        .unwrap();

    assert_eq!(
        fake.executables(),
        ["pdflatex", "bibtex", "pdflatex"]
    );
    assert_eq!(events.log_sequences(), [0, 1, 2]);
    match outcome.lifecycle {
        BuildLifecycle::Succeeded { exit_code, .. } => assert_eq!(exit_code, 0),
        other => panic!("unexpected lifecycle {other:?}"),
    }
    assert_eq!(
        outcome.artifact_disposition,
        BuildArtifactDisposition::ReplacedWithSuccessfulPDF
    );
    assert_eq!(orchestrator.successful_pdf(), Some(b"pdf-v1".to_vec()));
}

#[test]
fn log_text_survives_utf8_sequences_split_across_chunks() {
    let text = "한국어 로그 👩🏽‍💻 é\n";
    let bytes = text.as_bytes();
    let stdout_of = |chunks: Vec<BuildProcessOutput>| {
        let fixture = Fixture::new(&[]);
        let fake = Arc::new(FakeExecutor::new(vec![Behavior::Success {
            chunks,
            pdf: Some(b"pdf".to_vec()),
        }]));
        let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
        orchestrator
            .select(
                fixture.target.clone(),
                BuildPipeline::Stages(vec![BuildStagePlan { tool: BuildToolStage::Pdflatex }]),
            )
            .unwrap();
        let events = EventRecorder::new();
        orchestrator.build(BuildID::new("utf8").unwrap(), events.handler()).unwrap();
        let sequences = events.log_sequences();
        assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]), "{sequences:?}");
        let stdout: String = events
            .events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                BuildEvent::Log(entry) if entry.channel == BuildLogChannel::StandardOutput => {
                    Some(entry.text.clone())
                }
                _ => None,
            })
            .collect();
        stdout
    };
    let raw = |channel, bytes: &[u8]| BuildProcessOutput { channel, bytes: bytes.to_vec() };
    for split in 1..bytes.len() {
        // A stderr chunk in between must not disturb stdout's carry.
        let stdout = stdout_of(vec![
            raw(BuildLogChannel::StandardOutput, &bytes[..split]),
            raw(BuildLogChannel::StandardError, b"err\n"),
            raw(BuildLogChannel::StandardOutput, &bytes[split..]),
        ]);
        assert_eq!(stdout, text, "split at byte {split}");
    }
    // A sequence still cut off when the process ends stays in the log,
    // decoded lossily as before.
    let cut = "a한".as_bytes();
    assert_eq!(stdout_of(vec![raw(BuildLogChannel::StandardOutput, &cut[..3])]), "a\u{FFFD}");
}

#[test]
fn tool_missing_fails_closed_without_trying_another_stage() {
    let fixture = Fixture::new(&[]);
    let fake = Arc::new(FakeExecutor::new(vec![Behavior::ToolMissing("xelatex".into())]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::Stages(vec![
                BuildStagePlan {
                    tool: BuildToolStage::Xelatex,
                },
                BuildStagePlan {
                    tool: BuildToolStage::Pdflatex,
                },
            ]),
        )
        .unwrap();

    let error = orchestrator
        .build(
            BuildID::new("missing").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap_err();
    assert!(
        matches!(error, BuildOrchestratorError::ProcessError(_)),
        "unexpected error: {error:?}"
    );
    assert_eq!(fake.request_count(), 1);
}

#[test]
fn parser_handles_every_utf8_and_line_boundary_split() {
    let root = Path::new("/tmp/project");
    let bytes = "main.tex:12:4: warning: café 漢字\n".as_bytes();

    for split in 0..=bytes.len() {
        let mut parser = BuildLogParser::new(root);
        let mut issues = parser.consume_bytes(&bytes[..split], BuildLogChannel::StandardError);
        issues.extend(parser.consume_bytes(&bytes[split..], BuildLogChannel::StandardError));
        issues.extend(parser.finish());
        assert_eq!(issues.len(), 1, "split {split}");
        assert_eq!(issues[0].message, "warning: café 漢字");
        assert_eq!(issues[0].file.as_deref(), Some("main.tex"));
        assert_eq!(issues[0].line, Some(12));
        assert_eq!(issues[0].column, Some(4));
        assert!(issues[0].is_clickable());
    }
}

#[test]
fn parser_tracks_nested_files_classic_errors_warnings_and_control_characters() {
    let mut parser = BuildLogParser::new(Path::new("/work/project"));
    let mut issues = parser.consume(
        "(./main.tex\n(./chapters/one.tex\n! Undefined control sequence.\nl.27 \\bad\n)\nOverfull \\hbox (2.0pt too wide) at lines 40--41\nWarning--empty journal\u{1B}\n",
        BuildLogChannel::StandardOutput,
    );
    issues.extend(parser.finish());

    let severities: Vec<BuildIssueSeverity> = issues.iter().map(|i| i.severity).collect();
    assert_eq!(
        severities,
        [
            BuildIssueSeverity::Error,
            BuildIssueSeverity::Warning,
            BuildIssueSeverity::Warning
        ]
    );
    assert_eq!(issues[0].file.as_deref(), Some("chapters/one.tex"));
    assert_eq!(issues[0].line, Some(27));
    assert_eq!(issues[1].file.as_deref(), Some("main.tex"));
    assert_eq!(issues[1].line, Some(40));
    assert!(issues[2].message.contains("\\u{1B}"));
}

#[test]
fn duplicate_issues_are_emitted_once() {
    let fixture = Fixture::new(&[]);
    let line = chunk("main.tex:3: warning: duplicate\n", BuildLogChannel::StandardError);
    let fake = Arc::new(FakeExecutor::new(vec![Behavior::Success {
        chunks: vec![line.clone(), line],
        pdf: Some(b"pdf".to_vec()),
    }]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::single_pass(BuildToolStage::Pdflatex),
        )
        .unwrap();
    let outcome = orchestrator
        .build(
            BuildID::new("dedupe").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap();
    assert_eq!(outcome.issues.len(), 1);
}

#[test]
fn cancellation_wins_race_and_prohibits_remaining_passes() {
    let fixture = Fixture::new(&[]);
    let fake = Arc::new(FakeExecutor::new(vec![
        Behavior::WaitForCancellation {
            partial_pdf: Some(b"partial".to_vec()),
        },
        Behavior::Success {
            chunks: vec![],
            pdf: Some(b"must-not-run".to_vec()),
        },
    ]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::Stages(vec![
                BuildStagePlan {
                    tool: BuildToolStage::Pdflatex,
                },
                BuildStagePlan {
                    tool: BuildToolStage::Pdflatex,
                },
            ]),
        )
        .unwrap();

    let orchestrator = Arc::new(orchestrator);
    let orchestrator2 = Arc::clone(&orchestrator);
    let id = BuildID::new("cancel").unwrap();
    let handle = std::thread::spawn(move || {
        orchestrator2.build(id, EventRecorder::new().handler())
    });
    fake.wait_until_started();
    orchestrator.cancel(0).unwrap();

    let outcome = handle.join().unwrap().unwrap();
    assert!(
        matches!(outcome.lifecycle, BuildLifecycle::Cancelled { .. }),
        "cancellation must win over the child result"
    );
    assert_eq!(fake.request_count(), 1);
    assert!(!fixture.output_url.exists());
}

#[test]
fn partial_pdf_cannot_replace_last_successful_pdf() {
    let fixture = Fixture::new(&[]);
    let fake = Arc::new(FakeExecutor::new(vec![
        Behavior::Success {
            chunks: vec![],
            pdf: Some(b"good".to_vec()),
        },
        Behavior::Failure {
            exit_code: 1,
            chunks: vec![],
            partial_pdf: Some(b"bad".to_vec()),
        },
    ]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::single_pass(BuildToolStage::Pdflatex),
        )
        .unwrap();
    orchestrator
        .build(
            BuildID::new("good").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap();

    let error = orchestrator
        .build(
            BuildID::new("bad").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap_err();
    assert_eq!(
        error,
        BuildOrchestratorError::ProcessFailed {
            stage: 0,
            exit_code: 1
        }
    );
    assert_eq!(std::fs::read(&fixture.output_url).unwrap(), b"good".to_vec());
    assert_eq!(orchestrator.successful_pdf(), Some(b"good".to_vec()));
    assert_eq!(
        orchestrator
            .last_outcome()
            .unwrap()
            .artifact_disposition,
        BuildArtifactDisposition::PreservedLastSuccessfulPDF {
            partial_output_was_discarded: true
        }
    );
}

#[test]
fn custom_login_shell_command_is_authoritative_and_runs_exactly_once() {
    let fixture = Fixture::new(&[]);
    let authority = ShellAuthority::new(
        ShellAuthoritySource::UserConfiguration,
        true,
        "Test-approved custom build",
    )
    .unwrap();
    let command = LoginShellCommandPlan::new(
        "/bin/sh",
        "printf custom",
        WorkingDirectoryPolicy::ProjectRoot,
        EnvironmentPolicy::Inherit {
            overrides: Default::default(),
        },
        authority,
    )
    .unwrap();
    let fake = Arc::new(FakeExecutor::new(vec![Behavior::Success {
        chunks: vec![],
        pdf: Some(b"custom-pdf".to_vec()),
    }]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(fixture.target.clone(), BuildPipeline::Custom(command))
        .unwrap();

    orchestrator
        .build(
            BuildID::new("custom").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap();

    assert_eq!(fake.request_count(), 1);
    assert_eq!(fake.shell_commands(), ["printf custom"]);
    assert!(fake.executables().is_empty());
}

#[test]
fn cleanup_only_removes_declared_generated_children() {
    let fixture = Fixture::new(&["main.aux", "build"]);
    std::fs::write(fixture.root.join("main.aux"), "aux").unwrap();
    std::fs::create_dir(fixture.root.join("build")).unwrap();
    let fake = Arc::new(FakeExecutor::new(vec![]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::single_pass(BuildToolStage::Pdflatex),
        )
        .unwrap();

    let removed = orchestrator
        .cleanup(&CleanupPolicy::RemoveKnownAuxiliaryFiles(
            ["main.aux".to_string()].into_iter().collect(),
        ))
        .unwrap();
    assert_eq!(removed, ["main.aux"]);
    assert_eq!(
        orchestrator
            .cleanup(&CleanupPolicy::RemoveKnownAuxiliaryFiles(
                ["main.tex".to_string()].into_iter().collect()
            ))
            .unwrap_err(),
        BuildOrchestratorError::UndeclaredGeneratedPath("main.tex".into())
    );
    assert_eq!(
        orchestrator
            .cleanup(&CleanupPolicy::RemoveKnownAuxiliaryFiles(
                ["../outside.aux".to_string()].into_iter().collect()
            ))
            .unwrap_err(),
        BuildOrchestratorError::PathTraversal("../outside.aux".into())
    );
    assert!(fixture.source_url.exists());
}

#[test]
fn target_rejects_traversal_absolute_paths_and_source_in_build_directory() {
    let root = Path::new("/tmp/project");
    assert!(BuildTarget::new(
        root,
        "../main.tex",
        "main.pdf",
        ["main.pdf".to_string()].into_iter().collect(),
        None,
    )
    .is_err());
    assert!(BuildTarget::new(
        root,
        "main.tex",
        "/tmp/main.pdf",
        ["/tmp/main.pdf".to_string()].into_iter().collect(),
        None,
    )
    .is_err());
    assert!(BuildTarget::new(
        root,
        "build/main.tex",
        "build/main.pdf",
        ["build".to_string(), "build/main.pdf".to_string()]
            .into_iter()
            .collect(),
        Some("build".to_string()),
    )
    .is_err());
}

#[test]
fn latexmk_stages_pass_engine_flag_and_change_to_source_directory() {
    for (stage, engine) in [
        (BuildToolStage::Latexmk, "-pdf"),
        (BuildToolStage::LatexmkXeLaTeX, "-pdfxe"),
        (BuildToolStage::LatexmkLuaLaTeX, "-pdflua"),
    ] {
        let fixture = Fixture::new(&[]);
        let fake = Arc::new(FakeExecutor::new(vec![Behavior::Success {
            chunks: vec![],
            pdf: Some(b"pdf".to_vec()),
        }]));
        let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
        orchestrator
            .select(
                fixture.target.clone(),
                BuildPipeline::single_pass(stage),
            )
            .unwrap();
        let events = EventRecorder::new();
        orchestrator
            .build(BuildID::new("latexmk").unwrap(), events.handler())
            .unwrap();
        let state = fake.state.lock().unwrap();
        let BuildProcessCommand::Direct(plan) = &state.requests[0].command else {
            panic!("expected a direct latexmk invocation");
        };
        assert_eq!(plan.executable, "latexmk");
        assert_eq!(
            plan.arguments,
            [
                engine,
                "-cd",
                "-synctex=1",
                "-interaction=nonstopmode",
                "-file-line-error",
                "main.tex"
            ]
        );
    }
}

#[test]
fn live_target_passes_relative_output_paths_to_every_tool() {
    let fixture = Fixture::live("manuscript/main.tex");
    let fake = Arc::new(FakeExecutor::with_pdf_path(
        vec![
            Behavior::Success {
                chunks: vec![],
                pdf: None,
            },
            Behavior::Success {
                chunks: vec![],
                pdf: None,
            },
            Behavior::Success {
                chunks: vec![],
                pdf: Some(b"pdf".to_vec()),
            },
        ],
        ".pitex-live/manuscript/main/main.pdf",
    ));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::Stages(vec![
                BuildStagePlan {
                    tool: BuildToolStage::Latexmk,
                },
                BuildStagePlan {
                    tool: BuildToolStage::Pdflatex,
                },
                BuildStagePlan {
                    tool: BuildToolStage::Bibtex,
                },
            ]),
        )
        .unwrap();

    orchestrator
        .build(
            BuildID::new("live-args").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap();

    let arguments = fake.direct_arguments();
    // latexmk -cd enters manuscript/, so -outdir climbs back out with a
    // relative path that also works on a remote host.
    assert_eq!(
        arguments[0],
        [
            "-pdf",
            "-cd",
            "-synctex=1",
            "-interaction=nonstopmode",
            "-file-line-error",
            "-outdir=../.pitex-live/manuscript/main",
            "manuscript/main.tex"
        ]
    );
    assert_eq!(
        arguments[1],
        [
            "-synctex=1",
            "-interaction=nonstopmode",
            "-file-line-error",
            "-output-directory=.pitex-live/manuscript/main",
            "manuscript/main.tex"
        ]
    );
    assert_eq!(arguments[2], [".pitex-live/manuscript/main/main"]);
}

#[test]
fn unset_build_directory_keeps_existing_arguments() {
    let fixture = Fixture::new(&[]);
    let fake = Arc::new(FakeExecutor::new(vec![
        Behavior::Success {
            chunks: vec![],
            pdf: None,
        },
        Behavior::Success {
            chunks: vec![],
            pdf: None,
        },
        Behavior::Success {
            chunks: vec![],
            pdf: Some(b"pdf".to_vec()),
        },
    ]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::Stages(vec![
                BuildStagePlan {
                    tool: BuildToolStage::Pdflatex,
                },
                BuildStagePlan {
                    tool: BuildToolStage::Latexmk,
                },
                BuildStagePlan {
                    tool: BuildToolStage::Bibtex,
                },
            ]),
        )
        .unwrap();

    orchestrator
        .build(
            BuildID::new("plain-args").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap();

    let arguments = fake.direct_arguments();
    assert_eq!(
        arguments[0],
        [
            "-synctex=1",
            "-interaction=nonstopmode",
            "-file-line-error",
            "main.tex"
        ]
    );
    assert_eq!(
        arguments[1],
        [
            "-pdf",
            "-cd",
            "-synctex=1",
            "-interaction=nonstopmode",
            "-file-line-error",
            "main.tex"
        ]
    );
    assert_eq!(arguments[2], ["main"]);
}

#[test]
fn live_build_isolates_outputs_and_mirrors_only_tex_subdirectories() {
    let fixture = Fixture::live("manuscript/main.tex");
    // A chapter that needs an aux directory under the output root.
    std::fs::create_dir_all(fixture.resolved("manuscript/chapters")).unwrap();
    std::fs::write(fixture.resolved("manuscript/chapters/chapter.tex"), "chapter").unwrap();
    // Non-TeX content and a dependency cache must not be mirrored.
    std::fs::create_dir_all(fixture.resolved("manuscript/assets")).unwrap();
    std::fs::write(fixture.resolved("manuscript/assets/logo.png"), "png").unwrap();
    std::fs::create_dir_all(fixture.resolved("node_modules/pkg")).unwrap();
    std::fs::write(fixture.resolved("node_modules/pkg/x.tex"), "vendored").unwrap();
    // A manual PDF beside the source is left untouched.
    let manual_pdf = fixture.resolved("manuscript/main.pdf");
    std::fs::write(&manual_pdf, "manual").unwrap();

    let fake = Arc::new(FakeExecutor::with_pdf_path(
        vec![Behavior::Success {
            chunks: vec![],
            pdf: Some(b"live-pdf".to_vec()),
        }],
        ".pitex-live/manuscript/main/main.pdf",
    ));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::single_pass(BuildToolStage::Latexmk),
        )
        .unwrap();

    let outcome = orchestrator
        .build(
            BuildID::new("live").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap();

    assert_eq!(
        outcome.artifact_disposition,
        BuildArtifactDisposition::ReplacedWithSuccessfulPDF
    );
    let output = fixture.resolved(".pitex-live/manuscript/main");
    assert_eq!(
        std::fs::read(output.join("main.pdf")).unwrap(),
        b"live-pdf".to_vec()
    );
    // Mirrored for latexmk's -cd view and for engines at the root.
    assert!(output.join("chapters").is_dir());
    assert!(output.join("manuscript/chapters").is_dir());
    assert!(!output.join("assets").exists());
    assert!(!output.join("manuscript/assets").exists());
    assert!(!output.join("node_modules").exists());
    assert_eq!(std::fs::read(&manual_pdf).unwrap(), b"manual".to_vec());
}

#[test]
fn output_child_symlink_escape_is_rejected() {
    let fixture = Fixture::live("manuscript/main.tex");
    std::fs::create_dir_all(fixture.resolved("manuscript/chapters")).unwrap();
    std::fs::write(fixture.resolved("manuscript/chapters/chapter.tex"), "chapter").unwrap();
    let output = fixture.resolved(".pitex-live/manuscript/main");
    std::fs::create_dir_all(&output).unwrap();
    let escape = std::env::temp_dir().join(format!("escape-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&escape);
    std::fs::create_dir(&escape).unwrap();
    std::os::unix::fs::symlink(&escape, output.join("chapters")).unwrap();

    let fake = Arc::new(FakeExecutor::new(vec![Behavior::Success {
        chunks: vec![],
        pdf: Some(b"pdf".to_vec()),
    }]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::single_pass(BuildToolStage::Pdflatex),
        )
        .unwrap();

    let error = orchestrator
        .build(
            BuildID::new("escape").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap_err();
    assert_eq!(error, BuildOrchestratorError::PathTraversal("chapters".into()));
    assert_eq!(fake.request_count(), 0);
    assert_eq!(std::fs::read_dir(&escape).unwrap().count(), 0);
    let _ = std::fs::remove_dir_all(&escape);
}

#[test]
fn output_root_symlink_redirect_is_rejected_and_manual_pdf_survives() {
    let fixture = Fixture::live("manuscript/main.tex");
    let manual_pdf = fixture.resolved("manuscript/main.pdf");
    std::fs::write(&manual_pdf, "manual").unwrap();
    std::fs::create_dir_all(fixture.resolved(".pitex-live/manuscript")).unwrap();
    // An in-project redirect: .pitex-live/manuscript/main -> manuscript.
    std::os::unix::fs::symlink(
        fixture.resolved("manuscript"),
        fixture.resolved(".pitex-live/manuscript/main"),
    )
    .unwrap();

    let fake = Arc::new(FakeExecutor::new(vec![Behavior::Success {
        chunks: vec![],
        pdf: Some(b"pdf".to_vec()),
    }]));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::single_pass(BuildToolStage::Pdflatex),
        )
        .unwrap();

    let error = orchestrator
        .build(
            BuildID::new("redirect").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap_err();
    assert_eq!(
        error,
        BuildOrchestratorError::PathTraversal(".pitex-live/manuscript/main".into())
    );
    assert_eq!(fake.request_count(), 0);
    assert_eq!(std::fs::read(&manual_pdf).unwrap(), b"manual".to_vec());
}

#[test]
fn build_directory_setup_failure_clears_active_for_retry() {
    let fixture = Fixture::live("manuscript/main.tex");
    // A regular file where a directory must be created.
    std::fs::write(fixture.resolved(".pitex-live"), "blocker").unwrap();

    let fake = Arc::new(FakeExecutor::with_pdf_path(
        vec![Behavior::Success {
            chunks: vec![],
            pdf: Some(b"pdf".to_vec()),
        }],
        ".pitex-live/manuscript/main/main.pdf",
    ));
    let orchestrator = BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake)));
    orchestrator
        .select(
            fixture.target.clone(),
            BuildPipeline::single_pass(BuildToolStage::Pdflatex),
        )
        .unwrap();

    orchestrator
        .build(
            BuildID::new("blocked").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap_err();
    assert_eq!(
        orchestrator.last_outcome().unwrap().artifact_disposition,
        BuildArtifactDisposition::NoPDF {
            partial_output_was_discarded: false
        }
    );

    std::fs::remove_file(fixture.resolved(".pitex-live")).unwrap();
    let outcome = orchestrator
        .build(
            BuildID::new("retry").unwrap(),
            EventRecorder::new().handler(),
        )
        .unwrap();
    assert_eq!(
        outcome.artifact_disposition,
        BuildArtifactDisposition::ReplacedWithSuccessfulPDF
    );
}

#[test]
fn cancel_racing_build_start_does_not_panic_or_wedge() {
    for _ in 0..16 {
        let fixture = Fixture::new(&[]);
        let fake = Arc::new(FakeExecutor::new(vec![Behavior::Success {
            chunks: vec![],
            pdf: Some(b"pdf".to_vec()),
        }]));
        let orchestrator = Arc::new(BuildOrchestrator::new(FakeExecutorRef(Arc::clone(&fake))));
        orchestrator
            .select(
                fixture.target.clone(),
                BuildPipeline::single_pass(BuildToolStage::Pdflatex),
            )
            .unwrap();
        let worker = Arc::clone(&orchestrator);
        let handle = std::thread::spawn(move || {
            worker.build(BuildID::new("race").unwrap(), EventRecorder::new().handler())
        });
        // May land before or after `active` is installed; either way the
        // build thread must return, never panic on a stale lifecycle read.
        let _ = orchestrator.cancel(0);
        let _ = handle.join().unwrap();
    }
}
