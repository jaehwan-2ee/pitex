//! Port of `Packages/TexCore/Tests/TexCoreTests/ProcessRunnerTests.swift`.

use build_core::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn temp_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "ProcessRunnerTests-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_path_buf();
    root.join("Fixtures/process").join(name)
}

fn plan(executable: &str, arguments: &[&str]) -> DirectCommandPlan {
    DirectCommandPlan::new(
        executable,
        arguments.iter().map(|s| s.to_string()).collect(),
        WorkingDirectoryPolicy::ProjectRoot,
        EnvironmentPolicy::Inherit {
            overrides: HashMap::new(),
        },
    )
    .unwrap()
}

#[test]
fn executable_search_uses_child_path_and_working_directory() {
    let directory = temp_dir();
    let bin = directory.join("tools with spaces");
    std::fs::create_dir(&bin).unwrap();
    let tool = bin.join("pitex-test-tool");
    std::fs::write(&tool, "#!/bin/sh\nprintf '%s' \"$PATH\"\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755)).unwrap();
    for path in [bin.to_string_lossy().into_owned(), "tools with spaces".to_string()] {
        for environment in [
            EnvironmentPolicy::Replace(
                [("PATH".to_string(), path.clone())].into_iter().collect(),
            ),
            EnvironmentPolicy::Inherit {
                overrides: [("PATH".to_string(), path.clone())].into_iter().collect(),
            },
        ] {
            let mut p = plan("pitex-test-tool", &[]);
            p.environment = environment;
            let result = ProcessRunner::default()
                .run(&p, &directory, None, None, None, None)
                .unwrap();
            assert_eq!(result.termination, ProcessTermination::Exited { code: 0 });
            assert_eq!(String::from_utf8_lossy(&result.standard_output), path);
        }
    }
    // A bare name absent from the child PATH must not fall back to the
    // parent's search path.
    let mut excluded = plan("env", &[]);
    excluded.environment = EnvironmentPolicy::Replace(
        [("PATH".to_string(), bin.to_string_lossy().into_owned())]
            .into_iter()
            .collect(),
    );
    match ProcessRunner::default().run(&excluded, &directory, None, None, None, None) {
        Err(ProcessRunnerError::SpawnFailed { code }) => {
            assert_eq!(code, 2 /* ENOENT */)
        }
        other => panic!("must not fall back to the parent PATH: {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn direct_arguments_are_not_interpreted_by_a_shell() {
    let directory = temp_dir();
    let metacharacters = "literal ; $(touch NEVER) * ' quote";
    let mut p = plan("/usr/bin/printf", &["%s", metacharacters]);
    p.environment = EnvironmentPolicy::Replace(HashMap::new());
    let result = ProcessRunner::default()
        .run(&p, &directory, None, None, None, None)
        .unwrap();

    assert_eq!(result.termination, ProcessTermination::Exited { code: 0 });
    assert_eq!(
        String::from_utf8_lossy(&result.standard_output),
        metacharacters
    );
    assert!(!directory.join("NEVER").exists());
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn working_directory_and_environment_policies() {
    let directory = temp_dir();
    let child = directory.join("child");
    std::fs::create_dir(&child).unwrap();
    let mut pwd = plan("/bin/pwd", &[]);
    pwd.working_directory = WorkingDirectoryPolicy::Explicit("child".into());
    pwd.environment = EnvironmentPolicy::Replace(HashMap::new());
    let pwd_result = ProcessRunner::default()
        .run(&pwd, &directory, None, None, None, None)
        .unwrap();
    let reported = String::from_utf8_lossy(&pwd_result.standard_output)
        .trim()
        .to_string();
    assert_eq!(
        Path::new(&reported).canonicalize().unwrap(),
        child.canonicalize().unwrap()
    );

    let mut env = plan("/usr/bin/env", &[]);
    env.environment = EnvironmentPolicy::Replace(
        [("PROCESS_RUNNER_SENTINEL".to_string(), "exact value".to_string())]
            .into_iter()
            .collect(),
    );
    let env_result = ProcessRunner::default()
        .run(&env, &directory, None, None, None, None)
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&env_result.standard_output),
        "PROCESS_RUNNER_SENTINEL=exact value\n"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn streams_separate_byte_safe_incremental_channels() {
    let directory = temp_dir();
    let chunks: Arc<Mutex<Vec<ProcessOutputChunk>>> = Arc::new(Mutex::new(Vec::new()));
    let chunks2 = Arc::clone(&chunks);
    let handler = move |chunk: ProcessOutputChunk| {
        chunks2.lock().unwrap().push(chunk);
    };
    let p = plan(fixture("emit-log.sh").to_str().unwrap(), &[]);
    let result = ProcessRunner::default()
        .run(&p, &directory, None, None, None, Some(&handler))
        .unwrap();
    let chunks = chunks.lock().unwrap();

    assert_eq!(result.termination, ProcessTermination::Exited { code: 0 });
    assert_eq!(
        String::from_utf8_lossy(&result.standard_output),
        "stdout-one\n€ stdout-unicode\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&result.standard_error),
        "stderr-one\n漢 stderr-unicode\n"
    );
    assert!(chunks
        .iter()
        .filter(|c| c.channel == ProcessOutputChannel::StandardOutput)
        .count()
        >= 3);
    assert!(chunks
        .iter()
        .filter(|c| c.channel == ProcessOutputChannel::StandardError)
        .count()
        >= 3);
    let sequences: Vec<u64> = chunks.iter().map(|c| c.sequence).collect();
    assert_eq!(sequences, (0..chunks.len() as u64).collect::<Vec<u64>>());
    let joined = |channel| -> Vec<u8> {
        chunks
            .iter()
            .filter(|c| c.channel == channel)
            .flat_map(|c| c.bytes.clone())
            .collect()
    };
    assert_eq!(
        joined(ProcessOutputChannel::StandardOutput),
        result.standard_output
    );
    assert_eq!(
        joined(ProcessOutputChannel::StandardError),
        result.standard_error
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn timeout_terminates_within_bound_and_reports_actual_signal() {
    let directory = temp_dir();
    let runner = ProcessRunner::new(Duration::from_millis(100));
    let p = plan("/bin/sleep", &["300"]);
    let started = Instant::now();
    let result = runner
        .run(&p, &directory, None, Some(Duration::from_millis(100)), None, None)
        .unwrap();

    assert_eq!(result.stop_reason, ProcessStopReason::TimedOut);
    assert!(matches!(
        result.termination,
        ProcessTermination::Signaled { signal }
            if signal == libc::SIGTERM || signal == libc::SIGKILL
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn caller_cancellation_is_bounded_and_idempotent() {
    let directory = temp_dir();
    let token = CancellationToken::new();
    let p = plan("/bin/sleep", &["300"]);
    let runner = ProcessRunner::new(Duration::from_millis(100));
    let token2 = token.clone();
    let directory2 = directory.clone();
    let handle = std::thread::spawn(move || {
        runner.run(&p, &directory2, None, None, Some(&token2), None)
    });
    std::thread::sleep(Duration::from_millis(75));
    let started = Instant::now();
    token.cancel();
    token.cancel();
    let result = handle.join().unwrap().unwrap();

    assert_eq!(result.stop_reason, ProcessStopReason::Cancelled);
    assert!(matches!(
        result.termination,
        ProcessTermination::Signaled { signal }
            if signal == libc::SIGTERM || signal == libc::SIGKILL
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn cancellation_terminates_entire_descendant_process_group() {
    let directory = temp_dir();
    let token = CancellationToken::new();
    let p = plan(
        fixture("child-tree.sh").to_str().unwrap(),
        &[directory.to_str().unwrap()],
    );
    let runner = ProcessRunner::new(Duration::from_millis(100));
    let token2 = token.clone();
    let directory2 = directory.clone();
    let handle = std::thread::spawn(move || {
        runner.run(&p, &directory2, None, None, Some(&token2), None)
    });

    let pid_files = ["parent.pid", "child.pid", "grandchild.pid"];
    let deadline = Instant::now() + Duration::from_secs(2);
    while !pid_files
        .iter()
        .all(|name| directory.join(name).exists())
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    let pids: Vec<i32> = pid_files
        .iter()
        .map(|name| {
            std::fs::read_to_string(directory.join(name))
                .unwrap()
                .trim()
                .parse()
                .unwrap()
        })
        .collect();

    token.cancel();
    let result = handle.join().unwrap().unwrap();
    assert_eq!(result.stop_reason, ProcessStopReason::Cancelled);
    let deadline = Instant::now() + Duration::from_secs(2);
    let all_dead = |pids: &[i32]| {
        pids.iter().all(|pid| !process_exists(*pid))
    };
    while !all_dead(&pids) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(all_dead(&pids));

    let _ = std::fs::remove_dir_all(&directory);
    assert!(!directory.exists());
}

fn process_exists(pid: i32) -> bool {
    unsafe {
        libc::kill(pid, 0) == 0
            || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }
}

#[test]
fn login_shell_requires_and_returns_explicit_authority() {
    let directory = temp_dir();
    let authority = ShellAuthority::new(
        ShellAuthoritySource::UserConfiguration,
        true,
        "Test explicitly permits its isolated login-shell command",
    )
    .unwrap();
    let mut env = HashMap::new();
    env.insert("HOME".to_string(), directory.to_string_lossy().into_owned());
    let plan = LoginShellCommandPlan::new(
        "/bin/sh",
        "printf shell-authorized",
        WorkingDirectoryPolicy::ProjectRoot,
        EnvironmentPolicy::Replace(env),
        authority.clone(),
    )
    .unwrap();
    let result = ProcessRunner::default()
        .run_login_shell(&plan, &directory, None, None, None, None)
        .unwrap();

    assert_eq!(result.termination, ProcessTermination::Exited { code: 0 });
    assert_eq!(
        String::from_utf8_lossy(&result.standard_output),
        "shell-authorized"
    );
    assert_eq!(result.shell_authority, Some(authority));
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn missing_executable_returns_spawn_error() {
    let directory = temp_dir();
    let p = plan("definitely-not-a-real-g003-executable", &[]);
    let error = ProcessRunner::default()
        .run(&p, &directory, None, None, None, None)
        .unwrap_err();
    assert_eq!(
        error,
        ProcessRunnerError::SpawnFailed {
            code: libc::ENOENT
        }
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn signal_termination_is_reported_exactly() {
    let directory = temp_dir();
    let p = plan("/bin/sh", &["-c", "kill -TERM $$"]);
    let result = ProcessRunner::default()
        .run(&p, &directory, None, None, None, None)
        .unwrap();
    assert_eq!(result.stop_reason, ProcessStopReason::Completed);
    assert_eq!(
        result.termination,
        ProcessTermination::Signaled {
            signal: libc::SIGTERM
        }
    );
    let _ = std::fs::remove_dir_all(&directory);
}
