//! Routes a file open to the Pitex window whose project contains it. Each
//! window is its own process (`STATE` holds one window), and only the
//! first instance receives OS opens, so every process advertises itself to
//! the others and a window asked to open a file outside its own project
//! offers it around with `OPEN <path>`; the owner shows it and answers
//! `OK`, anyone else `NO`.
//!
//! The transport differs per platform. Unix gives every process a socket
//! at `$XDG_RUNTIME_DIR/pitex/<pid>.sock` (the runtime dir is per-user, so
//! the file itself is the credential). Windows listens on `127.0.0.1:0`
//! and publishes `{port, token}` as `<pid>.json` under
//! `%LOCALAPPDATA%\pitex\windows\`; loopback is reachable by other local
//! users, so each request there is `OPEN <token> <path>` and the listener
//! only honors its own 128-bit token.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(windows)]
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};

/// Bounds a round trip — two windows offering to each other at once would
/// otherwise wait on each other's busy main thread.
const TIMEOUT: Duration = Duration::from_secs(2);

/// The window-ownership check runs on the GTK thread, where `STATE` lives;
/// the reply hop is bounded by TIMEOUT like the socket read.
fn show_in_window(file: &Path) -> bool {
    let (reply, owned) = std::sync::mpsc::channel();
    let file = file.to_path_buf();
    gtk4::glib::MainContext::default().invoke(move || {
        let shown = crate::app_ui::STATE.with(|s| {
            let slot = s.borrow();
            let mut state = slot.as_ref()?.try_borrow_mut().ok()?;
            Some(state.show_if_owned(&file))
        });
        let _ = reply.send(shown == Some(true));
    });
    owned.recv_timeout(TIMEOUT).unwrap_or(false)
}

// ─── Unix ────────────────────────────────────────────────────────────────

#[cfg(unix)]
fn socket_dir() -> PathBuf {
    gtk4::glib::user_runtime_dir().join("pitex")
}

#[cfg(unix)]
fn own_socket() -> PathBuf {
    socket_dir().join(format!("{}.sock", std::process::id()))
}

/// Starts this process's listener (once). Requests are answered on the GTK
/// thread, where the window state lives.
#[cfg(unix)]
pub fn listen() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let _ = std::fs::create_dir_all(socket_dir());
        let path = own_socket();
        let _ = std::fs::remove_file(&path);
        let Ok(listener) = UnixListener::bind(&path) else { return };
        std::thread::spawn(move || serve(listener, show_in_window));
    });
}

/// Accept loop: every connection gets its own thread, so a peer that
/// connects and stays silent only burns its own TIMEOUT — real `OPEN`s
/// keep being answered.
#[cfg(unix)]
fn serve(listener: UnixListener, owns: fn(&Path) -> bool) {
    for stream in listener.incoming().flatten() {
        std::thread::spawn(move || {
            let _ = answer_with(stream, owns);
        });
    }
}

/// Removes this process's socket; called at shutdown.
#[cfg(unix)]
pub fn stop() {
    let _ = std::fs::remove_file(own_socket());
}

#[cfg(unix)]
fn answer_with(
    stream: UnixStream,
    owns: impl Fn(&Path) -> bool,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let Some(file) = line.trim_end_matches('\n').strip_prefix("OPEN ") else {
        return Ok(());
    };
    let owned = owns(&PathBuf::from(file));
    (&stream).write_all(if owned { b"OK\n" } else { b"NO\n" })
}

/// Offers `file` to every other Pitex window; true when one of them owns
/// its project and showed it.
#[cfg(unix)]
pub fn offer_to_other_windows(file: &Path) -> bool {
    offer_in_dir(&socket_dir(), file)
}

#[cfg(unix)]
fn offer_in_dir(dir: &Path, file: &Path) -> bool {
    let own = own_socket();
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    for socket in entries.flatten().map(|e| e.path()).filter(|p| *p != own) {
        match UnixStream::connect(&socket) {
            Ok(stream) => {
                if offer(stream, file).unwrap_or(false) {
                    return true;
                }
            }
            // A crashed window leaves its socket file behind.
            Err(_) => {
                let _ = std::fs::remove_file(&socket);
            }
        }
    }
    false
}

#[cfg(unix)]
fn offer(mut stream: UnixStream, file: &Path) -> std::io::Result<bool> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    writeln!(stream, "OPEN {}", file.display())?;
    let mut reply = String::new();
    BufReader::new(&stream).read_line(&mut reply)?;
    Ok(reply == "OK\n")
}

// ─── Windows ─────────────────────────────────────────────────────────────

/// One entry file per window process, under `%LOCALAPPDATA%` — private to
/// the user, so peers can read the port+token but other accounts cannot.
#[cfg(windows)]
fn window_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("pitex")
        .join("windows")
}

#[cfg(windows)]
fn own_entry() -> PathBuf {
    window_dir().join(format!("{}.json", std::process::id()))
}

/// Starts this process's listener (once): an ephemeral loopback port whose
/// `{port, token}` entry lets peers reach it. Requests are answered on the
/// GTK thread, where the window state lives.
#[cfg(windows)]
pub fn listen() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let _ = std::fs::create_dir_all(window_dir());
        let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else { return };
        let Ok(port) = listener.local_addr().map(|a| a.port()) else { return };
        let token = new_token();
        write_entry(&own_entry(), port, &token);
        std::thread::spawn(move || serve(listener, token, show_in_window));
    });
}

/// Accept loop: every connection gets its own thread, so a peer that
/// connects and stays silent only burns its own TIMEOUT — real `OPEN`s
/// keep being answered.
#[cfg(windows)]
fn serve(listener: TcpListener, token: String, owns: fn(&Path) -> bool) {
    for stream in listener.incoming().flatten() {
        let token = token.clone();
        std::thread::spawn(move || {
            let _ = answer_with(stream, &token, owns);
        });
    }
}

/// Removes this process's entry file; called at shutdown.
#[cfg(windows)]
pub fn stop() {
    let _ = std::fs::remove_file(own_entry());
}

/// GLib's UUID helper draws straight from the OS RNG (CryptGenRandom on
/// Windows) — 122 bits, unguessable to a local process that can reach the
/// loopback port but not the per-user entry directory.
#[cfg(windows)]
fn new_token() -> String {
    gtk4::glib::uuid_string_random().to_string()
}

/// `<pid>.json` = `{"port": N, "token": "…"}`, written via a sibling temp
/// file so a peer never parses a half-written entry.
#[cfg(windows)]
fn write_entry(path: &Path, port: u16, token: &str) {
    let tmp = path.with_extension("tmp");
    let body = serde_json::json!({ "port": port, "token": token }).to_string();
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

#[cfg(windows)]
fn read_entry(path: &Path) -> Option<(u16, String)> {
    let text = std::fs::read_to_string(path).ok()?;
    let entry: serde_json::Value = serde_json::from_str(&text).ok()?;
    let port = u16::try_from(entry.get("port")?.as_u64()?).ok()?;
    let token = entry.get("token")?.as_str()?;
    Some((port, token.to_string()))
}

/// `OPEN <token> <path>` — the token leads so spaces in paths survive the
/// split. A wrong or missing token still gets `NO`: a same-box process
/// that guessed the port never gets to drive the window.
#[cfg(windows)]
fn answer_with(
    stream: TcpStream,
    token: &str,
    owns: impl Fn(&Path) -> bool,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let reply = match parse_request(&line) {
        Some((presented, file)) if presented == token => {
            if owns(&file) { "OK\n" } else { "NO\n" }
        }
        _ => "NO\n",
    };
    (&stream).write_all(reply.as_bytes())
}

#[cfg(windows)]
fn parse_request(line: &str) -> Option<(&str, PathBuf)> {
    let rest = line
        .trim_end_matches(&['\r', '\n'][..])
        .strip_prefix("OPEN ")?;
    let (token, path) = rest.split_once(' ')?;
    Some((token, PathBuf::from(path)))
}

/// Offers `file` to every other Pitex window; true when one of them owns
/// its project and showed it.
#[cfg(windows)]
pub fn offer_to_other_windows(file: &Path) -> bool {
    offer_in_dir(&window_dir(), file)
}

#[cfg(windows)]
fn offer_in_dir(dir: &Path, file: &Path) -> bool {
    let own = own_entry();
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    for entry in entries.flatten().map(|e| e.path()) {
        if entry == own
            || entry.extension().and_then(|e| e.to_str()) != Some("json")
        {
            continue;
        }
        let Some((port, token)) = read_entry(&entry) else { continue };
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        match TcpStream::connect_timeout(&addr, TIMEOUT) {
            Ok(stream) => {
                if offer(stream, &token, file).unwrap_or(false) {
                    return true;
                }
            }
            // A crashed window leaves its entry file behind.
            Err(_) => {
                let _ = std::fs::remove_file(&entry);
            }
        }
    }
    false
}

#[cfg(windows)]
fn offer(mut stream: TcpStream, token: &str, file: &Path) -> std::io::Result<bool> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    writeln!(stream, "OPEN {token} {}", file.display())?;
    let mut reply = String::new();
    BufReader::new(&stream).read_line(&mut reply)?;
    Ok(reply.trim_end() == "OK")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "pitex-ipc-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A live peer that owns the file answers OK.
    #[cfg(unix)]
    #[test]
    fn offer_to_live_peer_gets_ok() {
        let dir = test_dir("ok");
        let listener = UnixListener::bind(dir.join("peer.sock")).unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            answer_with(stream, |_| true).unwrap();
        });
        assert!(offer_in_dir(&dir, Path::new("/tmp/doc.tex")));
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A peer that doesn't own the file answers NO.
    #[cfg(unix)]
    #[test]
    fn peer_without_the_file_says_no() {
        let dir = test_dir("no");
        let listener = UnixListener::bind(dir.join("peer.sock")).unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            answer_with(stream, |_| false).unwrap();
        });
        assert!(!offer_in_dir(&dir, Path::new("/tmp/doc.tex")));
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Re-offer until `path` is removed, or ~2 s pass: a sibling test's
    /// forked child can briefly inherit the dropped listener's fd, so one
    /// cleanup pass occasionally loses the connect race. The file's stay
    /// is still bounded — the process exits the fd on exec.
    fn poll_removed(dir: &Path, file: &Path, path: &Path) {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while path.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            let _ = offer_in_dir(dir, file);
        }
    }

    /// A socket file nobody listens on gets cleaned up.
    #[cfg(unix)]
    #[test]
    fn stale_socket_is_removed() {
        let dir = test_dir("stale");
        let socket = dir.join("dead.sock");
        drop(UnixListener::bind(&socket).unwrap());
        assert!(!offer_in_dir(&dir, Path::new("/tmp/doc.tex")));
        poll_removed(&dir, Path::new("/tmp/doc.tex"), &socket);
        assert!(!socket.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A request that isn't `OPEN` never gets an OK.
    #[cfg(unix)]
    #[test]
    fn malformed_request_gets_no_reply() {
        let dir = test_dir("garbage");
        let listener = UnixListener::bind(dir.join("peer.sock")).unwrap();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            answer_with(stream, |_| true).unwrap();
        });
        let mut stream = UnixStream::connect(dir.join("peer.sock")).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        writeln!(stream, "GARBAGE").unwrap();
        let mut reply = String::new();
        let n = BufReader::new(&stream).read_line(&mut reply).unwrap();
        assert_eq!(n, 0, "malformed request must not be answered");
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A peer that connects but sends nothing must not stall real offers —
    /// serial handling would hold the accept loop for the read timeout.
    #[cfg(unix)]
    #[test]
    fn silent_connection_does_not_block_peers() {
        let dir = test_dir("silent");
        let listener = UnixListener::bind(dir.join("peer.sock")).unwrap();
        std::thread::spawn(move || serve(listener, |_| true));
        let _silent = UnixStream::connect(dir.join("peer.sock")).unwrap();
        let start = std::time::Instant::now();
        assert!(offer_in_dir(&dir, Path::new("/tmp/doc.tex")));
        assert!(
            start.elapsed() < TIMEOUT,
            "offer waited on the silent connection"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A live peer that owns the file answers OK.
    #[cfg(windows)]
    #[test]
    fn offer_to_live_peer_gets_ok() {
        let dir = test_dir("ok");
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let token = "0123456789abcdeffedcba9876543210";
        write_entry(&dir.join("peer.json"), port, token);
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            answer_with(stream, token, |_| true).unwrap();
        });
        assert!(offer_in_dir(&dir, Path::new("C:\\doc.tex")));
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A request whose token isn't the listener's gets NO, not OK.
    #[cfg(windows)]
    #[test]
    fn wrong_token_is_rejected() {
        let dir = test_dir("token");
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        // The entry advertises a token the listener doesn't accept.
        write_entry(&dir.join("peer.json"), port, "00000000000000000000000000000000");
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            answer_with(stream, "ffffffffffffffffffffffffffffffff", |_| true).unwrap();
        });
        assert!(!offer_in_dir(&dir, Path::new("C:\\doc.tex")));
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An entry whose port answers nothing gets cleaned up.
    #[cfg(windows)]
    #[test]
    fn stale_entry_is_removed() {
        let dir = test_dir("stale");
        // Bind-then-drop leaves a port nothing listens on.
        let port = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let entry = dir.join("9999.json");
        write_entry(&entry, port, "deadbeef");
        assert!(!offer_in_dir(&dir, Path::new("C:\\doc.tex")));
        poll_removed(&dir, Path::new("C:\\doc.tex"), &entry);
        assert!(!entry.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A peer that connects but sends nothing must not stall real offers —
    /// serial handling would hold the accept loop for the read timeout.
    #[cfg(windows)]
    #[test]
    fn silent_connection_does_not_block_peers() {
        let dir = test_dir("silent");
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let token = "0123456789abcdeffedcba9876543210";
        write_entry(&dir.join("peer.json"), port, token);
        std::thread::spawn({
            let token = token.to_string();
            move || serve(listener, token, |_| true)
        });
        let _silent = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        let start = std::time::Instant::now();
        assert!(offer_in_dir(&dir, Path::new("C:\\doc.tex")));
        assert!(
            start.elapsed() < TIMEOUT,
            "offer waited on the silent connection"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `OPEN <token> <path>` keeps spaces in the path intact.
    #[cfg(windows)]
    #[test]
    fn request_parses_token_and_spaced_path() {
        let (token, path) =
            parse_request("OPEN deadbeef C:\\My Docs\\a b.tex\r\n").unwrap();
        assert_eq!(token, "deadbeef");
        assert_eq!(path, PathBuf::from("C:\\My Docs\\a b.tex"));
        assert!(parse_request("NOPE x").is_none());
        assert!(parse_request("OPEN onlytoken\n").is_none());
    }
}
