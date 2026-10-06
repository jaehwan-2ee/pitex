//! Emulator + PTY integration: feed bytes into the grid, spawn a real
//! shell and read its output back.

use std::time::{Duration, Instant};

use terminal_core::alacritty_terminal::grid::Dimensions;
use terminal_core::alacritty_terminal::index::{Column, Line};
use terminal_core::alacritty_terminal::term::cell::Flags;
use terminal_core::alacritty_terminal::vte::ansi::{Color, NamedColor};
use terminal_core::{Session, SessionEvent, Shell, SpawnOptions};

fn cell_text(session: &Session, line: i32, cols: usize) -> String {
    let term = session.term.lock();
    row_text(&term, line, cols)
}

fn row_text(
    term: &terminal_core::alacritty_terminal::Term<terminal_core::Proxy>,
    line: i32,
    cols: usize,
) -> String {
    let mut text = String::new();
    for col in 0..cols {
        text.push(term.grid()[Line(line)][Column(col)].c);
    }
    text.trim_end().to_string()
}

#[test]
fn feed_paints_text_colors_and_attrs() {
    let session = Session::new(80, 24, 1000);
    session.feed(b"plain \x1b[1;31mred\x1b[0m");
    let term = session.term.lock();
    let grid = term.grid();
    assert_eq!(grid[Line(0)][Column(0)].c, 'p');
    let red = &grid[Line(0)][Column(6)];
    assert_eq!(red.c, 'r');
    assert_eq!(red.fg, Color::Named(NamedColor::Red));
    assert!(red.flags.contains(Flags::BOLD));
    let plain = &grid[Line(0)][Column(0)];
    assert_eq!(plain.fg, Color::Named(NamedColor::Foreground));
}

#[test]
fn cursor_motion_and_line_wrap() {
    let session = Session::new(10, 5, 100);
    // Line wrap: 12 chars on a 10-col grid wrap onto the next line.
    session.feed(b"abcdefghijkl");
    {
        let term = session.term.lock();
        assert_eq!(row_text(&term, 0, 10), "abcdefghij");
        assert_eq!(row_text(&term, 1, 10), "kl");
        assert!(term.grid()[Line(0)][Column(9)].flags.contains(Flags::WRAPLINE));
    }
    // Cursor addressing: CSI row;col H.
    session.feed(b"\x1b[1;1HXX");
    assert_eq!(cell_text(&session, 0, 10), "XXcdefghij");
}

#[test]
fn scrollback_grows_and_alt_screen_is_separate() {
    let session = Session::new(20, 4, 1000);
    for i in 0..10 {
        session.feed(format!("line{i}\r\n").as_bytes());
    }
    {
        let term = session.term.lock();
        assert!(term.grid().history_size() >= 6, "scrollback should hold scrolled-off lines");
        // Rows 0..2 hold line7..line9; the trailing \r\n put the
        // cursor on a fresh last row.
        assert_eq!(row_text(&term, 2, 20), "line9");
    }
    // Alternate screen starts empty and doesn't touch history.
    session.feed(b"\x1b[?1049h\x1b[2Jfull");
    {
        let term = session.term.lock();
        assert!(term.mode().contains(terminal_core::alacritty_terminal::term::TermMode::ALT_SCREEN));
        let rows: Vec<String> = (0..4).map(|l| row_text(&term, l, 20)).collect();
        assert!(rows.iter().any(|r| r == "full"), "alt screen rows: {rows:?}");
    }
    session.feed(b"\x1b[?1049l");
    assert_eq!(cell_text(&session, 2, 20), "line9");
}

#[test]
fn pty_round_trip() {
    let mut session = Session::new(80, 24, 1000);
    #[cfg(unix)]
    let shell = Shell::new("sh".into(), vec!["-c".into(), "printf 'hello-pty\\n'; exit 0".into()]);
    #[cfg(windows)]
    let shell = Shell::new("cmd.exe".into(), vec!["/c".into(), "echo hello-pty".into()]);
    session
        .spawn(SpawnOptions {
            shell: Some(shell),
            working_directory: None,
            env: vec![("TERM".into(), "xterm-256color".into())],
            cols: 80,
            rows: 24,
            cell_width: 8,
            cell_height: 16,
        })
        .unwrap();

    // The child prints and exits; drain events until Exited or timeout.
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut exited = false;
    while Instant::now() < deadline && !exited {
        for event in session.drain_events() {
            if let SessionEvent::Exited(_) = event {
                exited = true;
            }
        }
        if !exited {
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    assert!(exited, "shell never exited");

    let deadline = Instant::now() + Duration::from_secs(5);
    let text = loop {
        let term = session.term.lock();
        let text = term.bounds_to_string(
            terminal_core::alacritty_terminal::index::Point::new(Line(0), Column(0)),
            terminal_core::alacritty_terminal::index::Point::new(Line(23), Column(79)),
        );
        drop(term);
        if text.contains("hello-pty") || Instant::now() > deadline {
            break text;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(text.contains("hello-pty"), "PTY output missing from grid: {text:?}");
}

#[test]
fn input_reaches_the_shell() {
    let mut session = Session::new(80, 24, 1000);
    #[cfg(unix)]
    let shell = Shell::new("sh".into(), Vec::new());
    #[cfg(windows)]
    let shell = Shell::new("cmd.exe".into(), Vec::new());
    session
        .spawn(SpawnOptions {
            shell: Some(shell),
            working_directory: None,
            env: vec![("TERM".into(), "xterm-256color".into())],
            cols: 80,
            rows: 24,
            cell_width: 8,
            cell_height: 16,
        })
        .unwrap();
    // `echo` a marker through the interactive shell's stdin.
    #[cfg(unix)]
    session.write(b"echo typed-$((40+2))\n".as_slice());
    #[cfg(windows)]
    session.write(b"echo typed-42\r\n".as_slice());

    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        {
            let term = session.term.lock();
            let text = term.bounds_to_string(
                terminal_core::alacritty_terminal::index::Point::new(Line(0), Column(0)),
                terminal_core::alacritty_terminal::index::Point::new(Line(23), Column(79)),
            );
            if text.contains("typed-42") {
                return;
            }
        }
        for event in session.drain_events() {
            if let SessionEvent::Exited(_) = event {
                panic!("shell exited before echoing input");
            }
        }
        assert!(Instant::now() < deadline, "typed input never echoed");
        std::thread::sleep(Duration::from_millis(20));
    }
}
