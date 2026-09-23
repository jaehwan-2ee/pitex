//! Routes a file open to the Pitex window whose project contains it. Each
//! Linux window is its own process (`STATE` holds one window), and only the
//! first instance receives OS opens, so every process listens on
//! `$XDG_RUNTIME_DIR/pitex/<pid>.sock`. A window asked to open a file
//! outside its own project offers it to the others with `OPEN <path>`; the
//! owner shows it and answers `OK`, anyone else `NO`.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Bounds a round trip — two windows offering to each other at once would
/// otherwise wait on each other's busy main thread.
const TIMEOUT: Duration = Duration::from_secs(2);

fn socket_dir() -> PathBuf {
    gtk4::glib::user_runtime_dir().join("pitex")
}

fn own_socket() -> PathBuf {
    socket_dir().join(format!("{}.sock", std::process::id()))
}

/// Starts this process's listener (once). Requests are answered on the GTK
/// thread, where the window state lives.
pub fn listen() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let _ = std::fs::create_dir_all(socket_dir());
        let path = own_socket();
        let _ = std::fs::remove_file(&path);
        let Ok(listener) = UnixListener::bind(&path) else { return };
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = answer(stream);
            }
        });
    });
}

/// Removes this process's socket; called at shutdown.
pub fn stop() {
    let _ = std::fs::remove_file(own_socket());
}

fn answer(stream: UnixStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let Some(file) = line.trim_end_matches('\n').strip_prefix("OPEN ") else {
        return Ok(());
    };
    let file = PathBuf::from(file);
    let (reply, owned) = std::sync::mpsc::channel();
    gtk4::glib::MainContext::default().invoke(move || {
        let shown = crate::app_ui::STATE.with(|s| {
            let slot = s.borrow();
            let mut state = slot.as_ref()?.try_borrow_mut().ok()?;
            Some(state.show_if_owned(&file))
        });
        let _ = reply.send(shown == Some(true));
    });
    let owned = owned.recv_timeout(TIMEOUT).unwrap_or(false);
    (&stream).write_all(if owned { b"OK\n" } else { b"NO\n" })
}

/// Offers `file` to every other Pitex window; true when one of them owns
/// its project and showed it.
pub fn offer_to_other_windows(file: &Path) -> bool {
    let own = own_socket();
    let Ok(entries) = std::fs::read_dir(socket_dir()) else { return false };
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

fn offer(mut stream: UnixStream, file: &Path) -> std::io::Result<bool> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    writeln!(stream, "OPEN {}", file.display())?;
    let mut reply = String::new();
    BufReader::new(&stream).read_line(&mut reply)?;
    Ok(reply == "OK\n")
}
