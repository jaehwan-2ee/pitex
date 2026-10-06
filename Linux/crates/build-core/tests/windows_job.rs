//! Native ownership proof: compiler descendants must stop when the runner
//! is cancelled, when its leader finishes, and when the whole app exits.
#![cfg(windows)]

use build_core::{
    CancellationToken, DirectCommandPlan, EnvironmentPolicy, ProcessRunner, ProcessStopReason,
    ProcessTermination, WorkingDirectoryPolicy,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant, SystemTime};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
    System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    },
};

const ROLE: &str = "PITEX_RUNNER_JOB_TEST_ROLE";
const DIRECTORY: &str = "PITEX_RUNNER_JOB_TEST_DIRECTORY";
const FINISH_LEADER: &str = "PITEX_RUNNER_JOB_TEST_FINISH_LEADER";
const PID_FILES: [&str; 3] = ["leader.pid", "child.pid", "grandchild.pid"];

struct TestDirectory(PathBuf);
impl TestDirectory {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("pitex runner 한글 {} {nonce}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        Self(directory)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.is_file() {
        assert!(
            Instant::now() < deadline,
            "fixture never wrote {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn read_pid(path: &Path) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(pid) = std::fs::read_to_string(path)
            .unwrap_or_default()
            .trim()
            .parse::<u32>()
        {
            if pid > 0 {
                return pid;
            }
        }
        assert!(
            Instant::now() < deadline,
            "fixture never completed {}",
            path.display()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn fixture_command(role: &str, directory: &Path) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "windows_runner_fixture", "--nocapture"])
        .env(ROLE, role)
        .env(DIRECTORY, directory);
    command
}

fn runner_plan(directory: &Path, finish_leader: bool) -> DirectCommandPlan {
    let mut overrides = HashMap::from([
        (ROLE.to_owned(), "leader".to_owned()),
        (
            DIRECTORY.to_owned(),
            directory.to_string_lossy().into_owned(),
        ),
    ]);
    if finish_leader {
        overrides.insert(FINISH_LEADER.to_owned(), "1".to_owned());
    }
    DirectCommandPlan::new(
        std::env::current_exe()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
        vec![
            "--exact".to_owned(),
            "windows_runner_fixture".to_owned(),
            "--nocapture".to_owned(),
        ],
        WorkingDirectoryPolicy::ProjectRoot,
        EnvironmentPolicy::Inherit { overrides },
    )
    .unwrap()
}

// Keep references to the actual processes so the proof never confuses a
// recycled PID with a dead fixture. Drop also cleans a broken implementation.
struct ProcessHandle(HANDLE);
impl ProcessHandle {
    fn stopped(&self) -> bool {
        unsafe { WaitForSingleObject(self.0, 0) == WAIT_OBJECT_0 }
    }
}
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            if !self.stopped() {
                TerminateProcess(self.0, 1);
                WaitForSingleObject(self.0, 5000);
            }
            CloseHandle(self.0);
        }
    }
}

fn live_tree(directory: &Path) -> Vec<ProcessHandle> {
    wait_for_file(&directory.join("tree.ready"));
    let mut pids = std::collections::HashSet::new();
    PID_FILES
        .iter()
        .map(|name| {
            let pid = read_pid(&directory.join(name));
            assert!(pids.insert(pid), "three different processes must execute");
            let handle = unsafe {
                OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
                    0,
                    pid,
                )
            };
            assert!(
                !handle.is_null(),
                "cannot open fixture {pid}: {}",
                std::io::Error::last_os_error()
            );
            let process = ProcessHandle(handle);
            assert!(
                !process.stopped(),
                "fixture {pid} must still be alive before teardown"
            );
            process
        })
        .collect()
}

fn assert_tree_stopped(processes: &[ProcessHandle]) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while processes.iter().any(|process| !process.stopped()) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        processes.iter().all(ProcessHandle::stopped),
        "all runner descendants must have stopped"
    );
}

struct Owner(Child);
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn whole_owner_exit_stops_the_compiler_tree() {
    let directory = TestDirectory::new();
    let mut owner = Owner(fixture_command("owner", &directory.0).spawn().unwrap());
    let processes = live_tree(&directory.0);
    // The fixture deliberately uses std::process::exit: no Rust destructors
    // or cancellation threads run. Windows must close the owned job itself.
    std::fs::write(directory.0.join("exit-owner"), b"exit").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = owner.0.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "runner owner did not exit");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(status.success(), "fixture owner failed: {status}");
    assert_tree_stopped(&processes);
}

#[test]
fn normal_completion_stops_descendants_before_output_readers_join() {
    let directory = TestDirectory::new();
    let plan = runner_plan(&directory.0, true);
    let working = directory.0.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let runner = std::thread::spawn(move || {
        let result = ProcessRunner::default().run(&plan, &working, None, None, None, None);
        let _ = sender.send(result);
    });
    let processes = live_tree(&directory.0);
    std::fs::write(directory.0.join("finish-leader"), b"finish").unwrap();
    let result = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("normal completion must not hang on inherited output pipes")
        .unwrap();
    runner.join().unwrap();
    assert_eq!(result.stop_reason, ProcessStopReason::Completed);
    assert_eq!(result.termination, ProcessTermination::Exited { code: 0 });
    assert_tree_stopped(&processes);
}

#[test]
fn cancellation_stops_the_entire_owned_job() {
    let directory = TestDirectory::new();
    let plan = runner_plan(&directory.0, false);
    let working = directory.0.clone();
    let cancel = CancellationToken::new();
    let worker_cancel = cancel.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let runner = std::thread::spawn(move || {
        let result =
            ProcessRunner::default().run(&plan, &working, None, None, Some(&worker_cancel), None);
        let _ = sender.send(result);
    });
    let processes = live_tree(&directory.0);
    cancel.cancel();
    let result = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("cancellation must stop every process and finish output readers")
        .unwrap();
    runner.join().unwrap();
    assert_eq!(result.stop_reason, ProcessStopReason::Cancelled);
    assert_tree_stopped(&processes);
}

#[test]
fn windows_runner_fixture() {
    let Ok(role) = std::env::var(ROLE) else {
        return;
    };
    let directory = PathBuf::from(std::env::var_os(DIRECTORY).unwrap());
    if role == "owner" {
        let plan = runner_plan(&directory, false);
        let working = directory.clone();
        std::thread::spawn(move || {
            ProcessRunner::default()
                .run(&plan, &working, None, None, None, None)
                .unwrap()
        });
        wait_for_file(&directory.join("tree.ready"));
        wait_for_file(&directory.join("exit-owner"));
        std::process::exit(0);
    }
    let name = match role.as_str() {
        "leader" => "leader.pid",
        "child" => "child.pid",
        "grandchild" => "grandchild.pid",
        _ => panic!("unexpected fixture role"),
    };
    std::fs::write(directory.join(name), std::process::id().to_string()).unwrap();
    let _child = match role.as_str() {
        "leader" => Some(fixture_command("child", &directory).spawn().unwrap()),
        "child" => Some(fixture_command("grandchild", &directory).spawn().unwrap()),
        _ => None,
    };
    if role == "leader" {
        for name in PID_FILES {
            read_pid(&directory.join(name));
        }
        std::fs::write(directory.join("tree.ready"), b"ready").unwrap();
        if std::env::var_os(FINISH_LEADER).is_some() {
            wait_for_file(&directory.join("finish-leader"));
            return;
        }
    }
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
