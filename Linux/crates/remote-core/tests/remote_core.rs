//! Port of `Packages/TexCore/Tests/TexCoreTests/RemoteCoreTests.swift`,
//! plus the golden remote-script fixture check shared with Swift. Runs on
//! Windows too; the POSIX-shell pieces (a local /bin/sh, a live sshd) stay
//! `cfg(unix)`, and the Windows-only checks are `windows_tests` in lib.rs.

use remote_core::remote_scripts;
use remote_core::{
    RemoteDirectoryListing, RemoteFileState, RemoteMirror, RemoteProject, RemoteSyncRules,
    SshClient, SshConfigParser, SshConnection, SshHostEntry, SyncPlanner,
};
#[cfg(unix)]
use remote_core::{MirrorWriteGate, RemoteBuildExecutor, RemoteSync};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use std::sync::{Arc, Mutex};

/// A unique directory under the system temp folder, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "pitex-{tag}-{}",
            uuid_like()
        ));
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn join(&self, rest: &str) -> PathBuf {
        self.0.join(rest)
    }
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}-{}", std::process::id())
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
}

#[cfg(unix)]
fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(unix)]
fn sha(text: &str) -> String {
    remote_core::sha256_hex(text.as_bytes())
}

#[cfg(unix)]
#[derive(Default)]
struct RecordingGate {
    events: Mutex<Vec<String>>,
}
#[cfg(unix)]
impl MirrorWriteGate for RecordingGate {
    fn begin_sync_commit(&self) {
        self.events.lock().unwrap().push("begin".into());
    }
    fn end_sync_commit(&self) {
        self.events.lock().unwrap().push("end".into());
    }
}

// ─── ssh config ──

#[test]
fn config_hosts_skip_patterns_match_blocks_and_keep_first_values() {
    let config = "# personal machines\n\
                  Host *\n  \
                      ServerAliveInterval 30\n\
                  Host mini studio\n  \
                      HostName mini.local\n  \
                      User \"jae hwan\"\n  \
                      Port=2222\n  \
                      HostName ignored.local\n\
                  Host !blocked *.corp\n  \
                      User nobody\n\
                  Match host mini\n  \
                      User matched\n\
                  Host=lab\n  \
                      Hostname = 10.0.0.5 # trailing comment\n\
                  Include extra\n";
    let hosts = SshConfigParser::hosts_in(config, |argument| {
        if argument == "extra" {
            vec!["Host included\n  User inc\nHost mini\n  User late".to_string()]
        } else {
            vec![]
        }
    });
    let aliases: Vec<&str> = hosts.iter().map(|h| h.alias.as_str()).collect();
    assert_eq!(aliases, ["mini", "studio", "lab", "included"]);
    assert_eq!(
        hosts[0],
        SshHostEntry::new(
            "mini",
            Some("mini.local".into()),
            Some("jae hwan".into()),
            Some(2222)
        )
    );
    assert_eq!(hosts[1].host_name.as_deref(), Some("mini.local"));
    assert_eq!(hosts[2].host_name.as_deref(), Some("10.0.0.5"));
    assert_eq!(hosts[2].user, None, "values under Match must not leak into later hosts");
    assert_eq!(hosts[3].user.as_deref(), Some("inc"));
    assert_eq!(hosts[0].summary(), "jae hwan@mini.local:2222");
}

#[test]
fn include_glob_and_missing_config() {
    let directory = TempDir::new("ssh");
    std::fs::create_dir_all(directory.path()).unwrap();
    write(&directory.join("one.conf"), "Host a\n");
    write(&directory.join("two.conf"), "Host b\n");
    write(&directory.join("skip.txt"), "Host c\n");
    let main = directory.join("config");
    write(
        &main,
        &format!("Include {}/*.conf\nHost main\n", directory.path().display()),
    );
    let aliases: Vec<String> = SshConfigParser::load_hosts(&main)
        .into_iter()
        .map(|h| h.alias)
        .collect();
    assert_eq!(aliases, ["a", "b", "main"]);
    assert!(SshConfigParser::load_hosts(&directory.join("missing")).is_empty());
    assert!(SshConfigParser::matches("*.conf", "x.conf"));
    assert!(!SshConfigParser::matches("*.conf", "x.txt"));
    assert!(SshConfigParser::matches("h?st", "host"));
}

// ─── connection and argv ──

#[test]
fn connection_validation_rejects_option_injection() {
    assert!(SshConnection::new("mini", "mini", None, None, None)
        .validation_error()
        .is_none());
    for connection in [
        SshConnection::new("x", "-oProxyCommand=evil", None, None, None),
        SshConnection::new("x", "host name", None, None, None),
        SshConnection::new("x", "", None, None, None),
        SshConnection::new("x", "h", Some("-l".into()), None, None),
        SshConnection::new("x", "h", Some("a@b".into()), None, None),
        SshConnection::new("x", "h", None, Some(70000), None),
    ] {
        assert!(connection.validation_error().is_some(), "{connection:?}");
    }
}

#[test]
fn arguments_quote_the_script_and_keep_destination_after_double_dash() {
    let connection = SshConnection::new(
        "lab",
        "lab.example",
        Some("me".into()),
        Some(2200),
        Some("/k/id".into()),
    );
    let client = SshClient {
        connection,
        ssh_executable: PathBuf::from("/usr/bin/ssh"),
        control_directory: None,
        extra_arguments: vec![],
    };
    let argv = client
        .arguments(r#"echo "$1""#, &["it's here".to_string()], false)
        .unwrap();
    assert!(argv.iter().any(|a| a == "BatchMode=yes"));
    assert!(
        !argv.iter().any(|a| a.contains("StrictHostKeyChecking")),
        "host keys stay checked"
    );
    let dashes = argv.iter().position(|a| a == "--").unwrap();
    assert_eq!(argv[dashes + 1], "lab.example");
    assert_eq!(
        argv[dashes + 2],
        r#"'/bin/sh' '-c' 'echo "$1"' 'pitex' 'it'\''s here'"#
    );
    assert_eq!(
        &argv[dashes - 8..dashes],
        ["-l", "me", "-p", "2200", "-i", "/k/id", "-o", "IdentitiesOnly=yes"]
    );
    assert!(SshClient::new(SshConnection::new("x", "-x", None, None, None))
        .arguments("true", &[], false)
        .is_err());
}

#[cfg(unix)]
#[test]
fn control_directory_is_short_private_and_owned() {
    use std::os::unix::fs::PermissionsExt;
    // Room for "/<40 hex>.<16 chars>" under the 104-byte socket limit.
    let default = SshClient::default_control_directory().unwrap();
    assert!(default.as_os_str().len() + 58 < 104, "{default:?}");
    let base = TempDir::new("cd");
    std::fs::create_dir_all(base.path()).unwrap();
    let directory = base.join("dir");
    let link = base.join("link");
    assert_eq!(
        SshClient::usable_control_directory(&directory).as_deref(),
        Some(directory.as_path()),
        "created private on first use"
    );
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        SshClient::usable_control_directory(&directory),
        None,
        "group/other access disables multiplexing"
    );
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::os::unix::fs::symlink(&directory, &link).unwrap();
    assert_eq!(
        SshClient::usable_control_directory(&link),
        None,
        "a symlink is not trusted"
    );
    assert_eq!(
        SshClient::usable_control_directory(Path::new(&format!("/tmp/{}", "x".repeat(60)))),
        None,
        "too long for a socket path"
    );
}

// ─── hashing, parsing, planning ──

#[test]
fn sha256_matches_known_vectors() {
    assert_eq!(
        remote_core::sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        remote_core::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        remote_core::sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    assert_eq!(
        remote_core::sha256_hex(&vec![0x61u8; 1_000_000]),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn hash_and_listing_parsers_drop_unsafe_entries() {
    let hash = "a".repeat(64);
    let output = format!(
        "{hash}  ./main.tex\n{hash} *./fig/plot.pdf\n\\{hash}  ./weird\\nname\n{hash}  -\n{hash}  ./../escape.tex\n"
    );
    let parsed = remote_scripts::parse_hashes(&output);
    let expected: HashMap<String, String> = [
        ("main.tex".to_string(), hash.clone()),
        ("fig/plot.pdf".to_string(), hash),
    ]
    .into_iter()
    .collect();
    assert_eq!(parsed, expected);
    let listing = RemoteDirectoryListing::parse("/home/me\nD/papers\nF/notes.md\nD/Archive\n");
    assert_eq!(
        listing,
        Some(RemoteDirectoryListing {
            path: "/home/me".into(),
            folders: vec!["Archive".into(), "papers".into()],
            files: vec!["notes.md".into()],
        })
    );
    assert_eq!(RemoteDirectoryListing::parse("not a path\n"), None);
    assert!(RemoteSyncRules::is_safe_relative_path("a/b.tex"));
    for path in ["", "/etc/passwd", "../x", "a/../b", "a//b", "a/./b", "a/\u{1}b"] {
        assert!(!RemoteSyncRules::is_safe_relative_path(path), "{path}");
    }
    assert!(RemoteSyncRules::is_excluded(".main.tex.texspark-1234.tmp"));
    assert!(RemoteSyncRules::is_excluded(".git"));
    assert!(!RemoteSyncRules::is_excluded("main.tex"));
}

#[test]
fn pull_plan_never_overwrites_local_edits() {
    let remote: HashMap<String, String> = [
        ("same", "1"),
        ("remoteEdit", "2b"),
        ("bothEdit", "3b"),
        ("converged", "4b"),
        ("new", "5"),
        ("newClash", "6r"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let manifest: HashMap<String, String> = [
        ("same", "1"),
        ("remoteEdit", "2"),
        ("bothEdit", "3"),
        ("converged", "4"),
        ("gone", "7"),
        ("goneEdited", "8"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let local: HashMap<String, String> = [
        ("same", "1"),
        ("remoteEdit", "2"),
        ("bothEdit", "3l"),
        ("converged", "4b"),
        ("newClash", "6l"),
        ("gone", "7"),
        ("goneEdited", "8l"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let plan = SyncPlanner::pull(&remote, &manifest, &local);
    assert_eq!(plan.download, ["new", "remoteEdit"]);
    let adopt: HashMap<String, String> = [("converged".to_string(), "4b".to_string())]
        .into_iter()
        .collect();
    assert_eq!(plan.adopt, adopt);
    assert_eq!(plan.delete, ["gone"]);
    assert_eq!(plan.conflicts, ["bothEdit", "goneEdited", "newClash"]);
}

/// `/bin/sh -n` syntax-checks each script — POSIX only.
#[cfg(unix)]
#[test]
fn remote_scripts_are_valid_single_line_sh() {
    for script in [
        remote_scripts::hash_tree(),
        remote_scripts::probe(),
        remote_scripts::commit_upload(),
        remote_scripts::tar_out(),
        remote_scripts::list_directory(),
        remote_scripts::login_exec(),
    ] {
        assert!(!script.contains('\n'), "csh login shells need single-line scripts");
        let status = Command::new("/bin/sh")
            .args(["-n", "-c", &script])
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(0), "{script}");
    }
}

/// The scripts Rust emits must be byte-identical to the ones Swift emits —
/// the fixtures under Fixtures/expected/remote-scripts/ are written from
/// `RemoteScripts` itself and checked on both platforms.
#[test]
fn remote_scripts_match_golden_fixtures() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../Fixtures/expected/remote-scripts");
    for (name, script) in [
        ("hashTree.sh", remote_scripts::hash_tree()),
        ("probe.sh", remote_scripts::probe()),
        ("commitUpload.sh", remote_scripts::commit_upload()),
        ("tarOut.sh", remote_scripts::tar_out()),
        ("listDirectory.sh", remote_scripts::list_directory()),
        ("loginExec.sh", remote_scripts::login_exec()),
    ] {
        let expected = std::fs::read(dir.join(name))
            .unwrap_or_else(|e| panic!("fixture {name}: {e}"));
        assert_eq!(
            script.as_bytes(),
            expected.as_slice(),
            "{name} must match the Swift-emitted fixture byte for byte"
        );
    }
}

/// Runs a remote script with the local /bin/sh, as the device would.
/// PITEX_TEST_SH tries another shell.
#[cfg(unix)]
fn run_script(script: &str, root: &Path, input: &[u8]) -> String {
    let shell = std::env::var("PITEX_TEST_SH").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut child = Command::new(shell)
        .args(["-c", script, "pitex"])
        .arg(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input)
        .unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[cfg(unix)]
#[test]
fn commit_upload_and_probe_scripts_decide_per_file() {
    let base = TempDir::new("script");
    let root = base.join("project");
    let outside = base.join("outside");
    let package = base.join("package");
    for directory in [
        root.clone(),
        outside.clone(),
        package.join("data/fresh"),
        package.join("data/link"),
        root.join("dir.tex"),
    ] {
        std::fs::create_dir_all(&directory).unwrap();
    }
    std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
    write(&root.join("keep.tex"), "old");
    write(&root.join("moved.tex"), "theirs");
    write(&root.join("same.tex"), "new");
    write(&root.join("taken.tex"), "exists");
    // A lock left by a process that no longer exists is taken over.
    let mut finished = Command::new("/bin/sh")
        .args(["-c", "exit 0"])
        .spawn()
        .unwrap();
    let finished_pid = finished.id();
    finished.wait().unwrap();
    std::fs::create_dir_all(root.join(".pitex-upload.lock")).unwrap();
    write(&root.join(".pitex-upload.lock/pid"), &format!("{finished_pid}\n"));

    let uploads: [(&str, String); 7] = [
        ("keep.tex", sha("old")),
        ("moved.tex", sha("old")),
        ("same.tex", sha("old")),
        ("fresh/one.tex", "-".into()),
        ("taken.tex", "-".into()),
        ("link/in.tex", "-".into()),
        ("dir.tex", "-".into()),
    ];
    let mut expect = String::new();
    for (path, base_hash) in &uploads {
        write(&package.join(format!("data/{path}")), "new");
        expect += &format!("{base_hash} {} {path}\n", sha("new"));
    }
    write(&package.join("expect"), &expect);
    let archive = base.join("upload.tar");
    let status = Command::new("/usr/bin/tar")
        .arg("-cf")
        .arg(&archive)
        .arg("-C")
        .arg(&package)
        .args(["expect", "data"])
        .status()
        .unwrap();
    assert!(status.success());

    let archive_bytes = std::fs::read(&archive).unwrap();
    let output = run_script(&remote_scripts::commit_upload(), &root, &archive_bytes);
    let lines: std::collections::HashSet<&str> = output.split('\n').filter(|l| !l.is_empty()).collect();
    let expected: std::collections::HashSet<&str> = [
        "U/keep.tex",
        "C/moved.tex",
        "S/same.tex",
        "U/fresh/one.tex",
        "C/taken.tex",
        "E/link/in.tex",
        "C/dir.tex",
    ]
    .into_iter()
    .collect();
    assert_eq!(lines, expected);
    assert_eq!(read(&root.join("keep.tex")).as_deref(), Some("new"));
    assert_eq!(
        read(&root.join("moved.tex")).as_deref(),
        Some("theirs"),
        "a changed remote file is never replaced"
    );
    assert_eq!(read(&root.join("fresh/one.tex")).as_deref(), Some("new"));
    assert_eq!(read(&root.join("taken.tex")).as_deref(), Some("exists"));
    assert_eq!(
        std::fs::read_dir(&outside).unwrap().count(),
        0,
        "nothing written through the symlink"
    );
    let leftovers: Vec<String> = walk(&root)
        .into_iter()
        .filter(|p| p.rsplit('/').next().unwrap_or("").starts_with(".pitex-upload"))
        .collect();
    assert_eq!(leftovers, Vec::<String>::new(), "staging, backups and the lock are cleaned up");

    let probed = remote_scripts::parse_probe(&run_script(
        &remote_scripts::probe(),
        &root,
        b"keep.tex\ngone.tex\nnodir/x.tex\nlink/in.tex\ndir.tex\n",
    ));
    let expected_probe: HashMap<String, RemoteFileState> = [
        ("keep.tex".into(), RemoteFileState::File(sha("new"))),
        ("gone.tex".into(), RemoteFileState::Absent),
        ("nodir/x.tex".into(), RemoteFileState::Absent),
        ("link/in.tex".into(), RemoteFileState::Unavailable),
        ("dir.tex".into(), RemoteFileState::Unavailable),
    ]
    .into_iter()
    .collect();
    assert_eq!(probed, expected_probe);
}

/// All files under `dir` as root-relative paths (the leftovers check).
#[cfg(unix)]
fn walk(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            out.push(path.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
        }
    }
    out
}

#[test]
fn probe_parser_separates_present_absent_and_unavailable() {
    let hash = "b".repeat(64);
    let states = remote_scripts::parse_probe(&format!("H{hash}/a b.tex\nA/gone.tex\nE/locked/x.tex\nHzz/bad\n"));
    let expected: HashMap<String, RemoteFileState> = [
        ("a b.tex".into(), RemoteFileState::File(hash)),
        ("gone.tex".into(), RemoteFileState::Absent),
        ("locked/x.tex".into(), RemoteFileState::Unavailable),
    ]
    .into_iter()
    .collect();
    assert_eq!(states, expected);
}

#[test]
fn mirror_is_found_again_from_any_path_inside() {
    let store = TempDir::new("store");
    let project = RemoteProject::new(
        SshConnection::new("Mac mini", "mini", None, None, None),
        "/Users/me/thesis",
    );
    let mirror = RemoteMirror::prepare(project.clone(), store.path()).unwrap();
    assert_eq!(mirror.root().file_name().unwrap(), "thesis");
    assert_eq!(
        RemoteMirror::containing(&mirror.root().join("ch/one.tex"), store.path())
            .map(|m| m.project),
        Some(project.clone())
    );
    assert_eq!(
        RemoteMirror::prepare(project, store.path())
            .unwrap()
            .directory,
        mirror.directory,
        "same folder, same mirror"
    );
    assert_eq!(
        RemoteMirror::containing(&mirror.directory.join("manifest.json"), store.path()),
        None
    );
    assert_eq!(
        RemoteMirror::containing(Path::new("/tmp/elsewhere.tex"), store.path()),
        None
    );
}

// ─── Live, against a real sshd (skipped unless configured) ──

/// Configure with PITEX_TEST_SSH_DESTINATION (+ _PORT, _KEY, _KNOWN_HOSTS).
/// The "remote" is expected to share this filesystem (localhost), so the
/// test edits remote files directly. Unix-only: the remote side assumes a
/// POSIX filesystem (symlinks, modes), which a Windows CI sshd would not
/// give it.
#[cfg(unix)]
fn live_client() -> Option<SshClient> {
    let destination = std::env::var("PITEX_TEST_SSH_DESTINATION").ok()?;
    let mut extra: Vec<String> = Vec::new();
    if let Ok(key) = std::env::var("PITEX_TEST_SSH_KEY") {
        extra.extend(["-i".to_string(), key, "-o".to_string(), "IdentitiesOnly=yes".to_string()]);
    }
    if let Ok(known) = std::env::var("PITEX_TEST_SSH_KNOWN_HOSTS") {
        extra.extend(["-o".to_string(), format!("UserKnownHostsFile={known}")]);
    }
    let port = std::env::var("PITEX_TEST_SSH_PORT").ok().and_then(|p| p.parse().ok());
    // The default control directory exercises multiplexing like the app.
    let mut client = SshClient::new(SshConnection::new("test", destination, None, port, None));
    client.extra_arguments = extra;
    Some(client)
}

#[cfg(unix)]
#[test]
fn live_sync_round_trip() {
    let Some(client) = live_client() else {
        eprintln!("skipped: PITEX_TEST_SSH_DESTINATION not set");
        return;
    };
    let remote = TempDir::new("remote");
    let store = TempDir::new("store");
    std::fs::create_dir_all(remote.join("chapters/sub dir")).unwrap();
    std::fs::create_dir_all(remote.join(".git")).unwrap();
    write(&remote.join("main.tex"), "\\documentclass{article}\n\\input{chapters/one}\n");
    write(&remote.join("chapters/one.tex"), "한글 $x$\n");
    write(&remote.join("chapters/sub dir/it's.md"), "it's \"quoted\"\n");
    write(&remote.join(".git/config"), "secret");

    let listing = client.list_directory(remote.path().to_str().unwrap()).unwrap();
    assert_eq!(listing.folders, ["chapters"]);
    assert_eq!(listing.files, ["main.tex"]);
    let chapters = client
        .list_directory(remote.join("chapters").to_str().unwrap())
        .unwrap();
    assert_eq!(chapters.folders, ["sub dir"]);

    let mirror = RemoteMirror::prepare(
        RemoteProject::new(client.connection.clone(), remote.path().to_str().unwrap()),
        store.path(),
    )
    .unwrap();
    let sync = RemoteSync::new(mirror.clone(), client.clone(), None);
    let mut report = sync.pull().unwrap();
    let mut downloaded: Vec<&str> = report.downloaded.iter().map(String::as_str).collect();
    downloaded.sort_unstable();
    assert_eq!(downloaded, ["chapters/one.tex", "chapters/sub dir/it's.md", "main.tex"]);
    assert!(!mirror.root().join(".git/config").exists());
    assert_eq!(
        read(&mirror.root().join("chapters/one.tex")).as_deref(),
        Some("한글 $x$\n")
    );

    // Local edit + new local file → uploaded.
    write(&mirror.root().join("chapters/one.tex"), "한글 $y$\n");
    write(&mirror.root().join("chapters/two.tex"), "new\n");
    report = sync.push().unwrap();
    assert_eq!(report.uploaded, ["chapters/one.tex", "chapters/two.tex"]);
    assert_eq!(
        read(&remote.join("chapters/one.tex")).as_deref(),
        Some("한글 $y$\n")
    );
    let idle = sync.push().unwrap();
    assert!(idle.uploaded.is_empty(), "nothing left to push");
    assert!(
        !std::fs::read_dir(remote.path())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with(".pitex-upload")),
        "the device-side staging directory is cleaned up"
    );

    // Remote edit → pulled; remote delete → deleted locally.
    write(&remote.join("main.tex"), "remote edit\n");
    std::fs::remove_file(remote.join("chapters/two.tex")).unwrap();
    report = sync.pull().unwrap();
    assert_eq!(report.downloaded, ["main.tex"]);
    assert_eq!(report.deleted, ["chapters/two.tex"]);
    assert!(!mirror.root().join("chapters/two.tex").exists());

    // Edited on both sides → conflict both ways, nothing overwritten.
    write(&remote.join("main.tex"), "remote side\n");
    write(&mirror.root().join("main.tex"), "local side\n");
    let pulled = sync.pull().unwrap();
    let pushed = sync.push().unwrap();
    assert_eq!(pulled.conflicts, ["main.tex"]);
    assert_eq!(pushed.conflicts, ["main.tex"]);
    assert_eq!(read(&mirror.root().join("main.tex")).as_deref(), Some("local side\n"));
    assert_eq!(read(&remote.join("main.tex")).as_deref(), Some("remote side\n"));

    // Resolving: keep mine uploads it; take remote replaces the local copy.
    sync.resolve("main.tex", true).unwrap();
    assert_eq!(read(&remote.join("main.tex")).as_deref(), Some("local side\n"));
    write(&remote.join("main.tex"), "remote again\n");
    write(&mirror.root().join("main.tex"), "local again\n");
    let clash = sync.pull().unwrap();
    assert_eq!(clash.conflicts, ["main.tex"]);
    sync.resolve("main.tex", false).unwrap();
    assert_eq!(read(&mirror.root().join("main.tex")).as_deref(), Some("remote again\n"));

    // A fresh RemoteSync reloads the manifest from disk.
    let reopened = RemoteSync::new(mirror.clone(), client.clone(), None);
    let again = reopened.pull().unwrap();
    assert!(again.downloaded.is_empty());

    // Both sides arriving at the same text settles the conflict.
    write(&remote.join("main.tex"), "remote 3\n");
    write(&mirror.root().join("main.tex"), "local 3\n");
    let third = sync.pull().unwrap();
    assert_eq!(third.conflicts, ["main.tex"]);
    write(&mirror.root().join("main.tex"), "remote 3\n");
    let settled = sync.pull().unwrap();
    assert!(settled.conflicts.is_empty());

    // A remote folder swapped for a symlink leading through an
    // unsearchable directory is not proof that its files are gone.
    let vault = TempDir::new("vault");
    std::fs::create_dir_all(vault.path()).unwrap();
    std::fs::rename(remote.join("chapters/sub dir"), vault.join("inner")).unwrap();
    std::os::unix::fs::symlink(vault.join("inner"), remote.join("chapters/sub dir")).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(vault.path(), std::fs::Permissions::from_mode(0o000)).unwrap();
    let through_link = sync.pull().unwrap();
    std::fs::set_permissions(vault.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(through_link.deleted.is_empty());
    assert!(mirror.root().join("chapters/sub dir/it's.md").exists());
    std::fs::remove_file(remote.join("chapters/sub dir")).unwrap();
    std::fs::rename(vault.join("inner"), remote.join("chapters/sub dir")).unwrap();

    // An upload never writes through a symlinked remote folder.
    let elsewhere = TempDir::new("elsewhere");
    std::fs::create_dir_all(elsewhere.path()).unwrap();
    std::os::unix::fs::symlink(elsewhere.path(), remote.join("out")).unwrap();
    std::fs::create_dir_all(mirror.root().join("out")).unwrap();
    write(&mirror.root().join("out/new.tex"), "escape\n");
    assert!(sync.push().is_err(), "placing through a remote symlink must fail");
    assert!(!elsewhere.join("new.tex").exists());
    std::fs::remove_dir_all(mirror.root().join("out")).unwrap();
    std::fs::remove_file(remote.join("out")).unwrap();

    // A file the listing cannot see (unsearchable directory) is not taken
    // for deleted, and "take remote" refuses to guess.
    let locked = remote.join("chapters/sub dir");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    let hidden = sync.pull().unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(hidden.deleted.is_empty());
    assert!(mirror.root().join("chapters/sub dir/it's.md").exists());

    // A symlinked directory in the mirror never redirects a download.
    let outside2 = TempDir::new("outside");
    std::fs::create_dir_all(outside2.path()).unwrap();
    std::os::unix::fs::symlink(outside2.path(), mirror.root().join("figs")).unwrap();
    std::fs::create_dir_all(remote.join("figs")).unwrap();
    write(&remote.join("figs/plot.tex"), "plot");
    assert!(sync.pull().is_err(), "placing through a symlinked directory must fail");
    assert!(!outside2.join("plot.tex").exists());

    // Pushes wait at the app's write gate only around local commits.
    std::fs::remove_file(mirror.root().join("figs")).unwrap();
    let gate = Arc::new(RecordingGate::default());
    let gated = RemoteSync::new(
        mirror.clone(),
        client.clone(),
        Some(gate.clone() as Arc<dyn MirrorWriteGate>),
    );
    let gated_pull = gated.pull().unwrap();
    assert_eq!(gated_pull.downloaded, ["figs/plot.tex"]);
    assert_eq!(*gate.events.lock().unwrap(), vec!["begin", "end"]);
}

#[cfg(unix)]
#[test]
fn live_remote_build_streams_output_and_fetches_artifacts() {
    let Some(client) = live_client() else {
        eprintln!("skipped: PITEX_TEST_SSH_DESTINATION not set");
        return;
    };
    let remote = TempDir::new("remote");
    let store = TempDir::new("store");
    std::fs::create_dir_all(remote.join("src")).unwrap();
    write(&remote.join("src/main.tex"), "doc");
    let mirror = RemoteMirror::prepare(
        RemoteProject::new(client.connection.clone(), remote.path().to_str().unwrap()),
        store.path(),
    )
    .unwrap();
    let sync = Arc::new(RemoteSync::new(mirror.clone(), client.clone(), None));
    sync.pull().unwrap();
    let executor = RemoteBuildExecutor::new(sync.clone());
    executor.set_outputs(
        vec![
            "src/main.pdf".to_string(),
            "src/main.log".to_string(),
            "src/absent.aux".to_string(),
        ],
        None,
    );
    // A stand-in "engine": prints, writes the outputs, exits 3.
    let script =
        r#"printf 'pass %s\n' "$PWD"; echo warn >&2; printf '%%PDF-1.4' > main.pdf; echo log > main.log; exit 3"#;
    let plan = build_core::DirectCommandPlan::new(
        "/bin/sh",
        vec!["-c".to_string(), script.to_string()],
        build_core::WorkingDirectoryPolicy::SourceDirectory,
        build_core::EnvironmentPolicy::Inherit {
            overrides: HashMap::new(),
        },
    )
    .unwrap();
    let request = build_core::BuildProcessRequest {
        build_id: build_core::BuildID::new("b1").unwrap(),
        stage_index: 0,
        command: build_core::BuildProcessCommand::Direct(plan),
        project_root: mirror.root(),
        source_directory: mirror.root().join("src"),
    };
    let collected = Arc::new(Mutex::new(String::new()));
    let sink = collected.clone();
    let result = build_core::BuildProcessExecuting::execute(
        &executor,
        &request,
        &mut |output: build_core::BuildProcessOutput| {
            sink.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&output.bytes));
        },
    )
    .unwrap();
    assert_eq!(result.exit_code, 3);
    let text = collected.lock().unwrap().clone();
    let expected_dir = std::fs::canonicalize(remote.join("src"))
        .unwrap_or_else(|_| remote.join("src"));
    assert!(
        text.contains(&format!("pass {}", expected_dir.display()))
            || text.contains(&format!("pass {}", remote.join("src").display())),
        "{text}"
    );
    assert!(text.contains("warn"), "{text}");
    assert_eq!(
        read(&mirror.root().join("src/main.pdf")).as_deref(),
        Some("%PDF-1.4")
    );
    let after_build = sync.push().unwrap();
    assert!(
        after_build.uploaded.is_empty(),
        "fetched outputs are in the manifest, not pending uploads"
    );
}
