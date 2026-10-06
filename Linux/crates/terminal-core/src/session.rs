//! PTY session: spawns the shell on a pseudoterminal (openpty on Unix,
//! ConPTY on Windows 10 1809+) and drives the emulator from alacritty's
//! own event loop thread.
//!
//! The widget-facing surface is deliberately small: `feed`/`write`/`resize`/
//! `scroll`/`drain_events`, plus `set_wakeup` — the callback the PTY thread
//! fires whenever UI work is pending, so the GTK layer can hop threads and
//! repaint.

use std::borrow::Cow;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::event_loop::{EventLoop, EventLoopSender, Msg};
use alacritty_terminal::grid::Scroll;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{ClipboardType, Config, Term};
use alacritty_terminal::tty::{self, Options, Shell};
use alacritty_terminal::vte::ansi::{Processor, Rgb};

/// Formatter the emulator hands us for clipboard-load queries (OSC 52 `?`):
/// feed it the clipboard text and write the result to the PTY.
pub type ClipboardFormatter = Arc<dyn Fn(&str) -> String + Sync + Send>;
/// Same, for OSC 4/10/11/12 color queries.
pub type ColorFormatter = Arc<dyn Fn(Rgb) -> String + Sync + Send>;
/// Same, for CSI 14/16/18 size reports.
pub type SizeFormatter = Arc<dyn Fn(WindowSize) -> String + Sync + Send>;

/// Everything the UI must run on its own thread. Queued by the proxy and
/// drained via [`Session::drain_events`].
pub enum SessionEvent {
    /// Grid or cursor changed — repaint.
    Wakeup,
    /// The shell process exited (`Some(code)`) or the event loop ended.
    Exited(Option<i32>),
    /// OSC 52: the shell wrote to a clipboard.
    SetClipboard(ClipboardType, String),
    /// OSC 52 `?`: the shell reads a clipboard — answer with the formatter.
    ClipboardQuery(ClipboardType, ClipboardFormatter),
    /// OSC 4/10/11/12: report a palette entry as `rgb:rr/gg/bb`.
    ColorQuery(usize, ColorFormatter),
    /// CSI 14/16/18: report the drawable size.
    SizeQuery(SizeFormatter),
    /// Window title change; `None` resets to the default.
    Title(Option<String>),
    Bell,
}

/// The emulator's event sink. Replies the terminal owes the PTY (device
/// attributes, paste replies, …) are routed back into the event loop;
/// everything else is queued for the UI thread.
#[derive(Clone)]
pub struct Proxy {
    events: Sender<SessionEvent>,
    writer: Arc<Mutex<Option<EventLoopSender>>>,
    wakeup: Arc<Mutex<Option<Box<dyn Fn() + Send + Sync>>>>,
}

impl EventListener for Proxy {
    fn send_event(&self, event: Event) {
        let event = match event {
            Event::PtyWrite(text) => {
                if let Some(writer) = self.writer.lock().unwrap().as_ref() {
                    let _ = writer.send(Msg::Input(text.into_bytes().into()));
                }
                return;
            }
            Event::Wakeup | Event::MouseCursorDirty | Event::CursorBlinkingChange => {
                SessionEvent::Wakeup
            }
            Event::ChildExit(code) => SessionEvent::Exited(Some(code)),
            Event::Exit => SessionEvent::Exited(None),
            Event::ClipboardStore(ty, text) => SessionEvent::SetClipboard(ty, text),
            Event::ClipboardLoad(ty, fmt) => SessionEvent::ClipboardQuery(ty, fmt),
            Event::ColorRequest(index, fmt) => SessionEvent::ColorQuery(index, fmt),
            Event::TextAreaSizeRequest(fmt) => SessionEvent::SizeQuery(fmt),
            Event::Title(title) => SessionEvent::Title(Some(title)),
            Event::ResetTitle => SessionEvent::Title(None),
            Event::Bell => SessionEvent::Bell,
        };
        if self.events.send(event).is_ok() {
            if let Some(wakeup) = self.wakeup.lock().unwrap().as_ref() {
                wakeup();
            }
        }
    }
}

/// Shell + environment for [`Session::spawn`].
pub struct SpawnOptions {
    /// `None` → [`default_shell`].
    pub shell: Option<Shell>,
    pub working_directory: Option<PathBuf>,
    /// Environment overlaid on the inherited one (TERM, COLORTERM, …).
    pub env: Vec<(String, String)>,
    pub cols: usize,
    pub rows: usize,
    pub cell_width: u16,
    pub cell_height: u16,
}

/// The platform default: a login `$SHELL` on Unix, `%COMSPEC%` (cmd.exe —
/// or powershell.exe if configured) on Windows.
pub fn default_shell() -> Shell {
    #[cfg(unix)]
    {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        Shell::new(shell, vec!["-l".into()])
    }
    #[cfg(windows)]
    {
        let shell = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into());
        Shell::new(shell, Vec::new())
    }
}

type PtyLoop = EventLoop<tty::Pty, Proxy>;

/// One terminal: the emulator (`term`) plus, after [`Session::spawn`], a
/// PTY with the shell on it. Without a PTY `feed` still renders — status
/// text needs no shell.
pub struct Session {
    /// Emulator state — lock it to read the grid or move the selection.
    pub term: Arc<FairMutex<Term<Proxy>>>,
    // `feed` gets its own parser: it's a second byte stream into the same
    // terminal, so it must not share the event loop's parser state.
    feed_parser: Mutex<Processor>,
    proxy: Proxy,
    writer: Option<EventLoopSender>,
    events: Receiver<SessionEvent>,
    join: Option<JoinHandle<(PtyLoop, alacritty_terminal::event_loop::State)>>,
}

impl Session {
    pub fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let (events, rx) = mpsc::channel();
        let proxy = Proxy {
            events,
            writer: Arc::new(Mutex::new(None)),
            wakeup: Arc::new(Mutex::new(None)),
        };
        let config = Config { scrolling_history: scrollback, ..Default::default() };
        let term = Term::new(config, &TermSize::new(cols, rows), proxy.clone());
        Self {
            term: Arc::new(FairMutex::new(term)),
            feed_parser: Mutex::new(Processor::new()),
            proxy,
            writer: None,
            events: rx,
            join: None,
        }
    }

    /// The terminal's own event listener — the widget clones it to reach
    /// the wakeup hook. (Exposed for `EventLoop` wiring inside `spawn`.)
    fn proxy(&self) -> Proxy {
        self.proxy.clone()
    }

    /// Install the UI-side wakeup: fired from the PTY thread whenever
    /// [`drain_events`](Self::drain_events) has work.
    pub fn set_wakeup(&self, f: impl Fn() + Send + Sync + 'static) {
        *self.proxy.wakeup.lock().unwrap() = Some(Box::new(f));
    }

    /// Pending UI events, oldest first.
    pub fn drain_events(&self) -> impl Iterator<Item = SessionEvent> + '_ {
        self.events.try_iter()
    }

    /// Text injected by the app (status lines) — parsed like child output,
    /// never reaches the shell's stdin.
    pub fn feed(&self, bytes: &[u8]) {
        let mut term = self.term.lock();
        let mut parser = self.feed_parser.lock().unwrap();
        for byte in bytes {
            parser.advance(&mut *term, *byte);
        }
    }

    /// Bytes to the shell's stdin. No-op before `spawn` or after exit —
    /// callers fall back to the external terminal then.
    pub fn write(&self, bytes: impl Into<Cow<'static, [u8]>>) {
        if let Some(writer) = &self.writer {
            let _ = writer.send(Msg::Input(bytes.into()));
        }
    }

    /// Live PTY — the gate for `send` falling back to an external terminal.
    pub fn is_spawned(&self) -> bool {
        self.writer.is_some()
    }

    /// Resize the grid and the PTY (`SIGWINCH`/`ResizePseudoConsole`).
    pub fn resize(&self, cols: usize, rows: usize, cell_width: u16, cell_height: u16) {
        self.term.lock().resize(TermSize::new(cols, rows));
        if let Some(writer) = &self.writer {
            let _ = writer.send(Msg::Resize(WindowSize {
                num_lines: rows as u16,
                num_cols: cols as u16,
                cell_width,
                cell_height,
            }));
        }
    }

    /// Move the viewport within the scrollback (`display_offset`).
    pub fn scroll(&self, scroll: Scroll) {
        self.term.lock().scroll_display(scroll);
    }

    /// Spawn the shell on a new PTY and start the reader/event thread.
    pub fn spawn(&mut self, opts: SpawnOptions) -> io::Result<()> {
        let options = Options {
            shell: Some(opts.shell.unwrap_or_else(default_shell)),
            working_directory: opts.working_directory,
            drain_on_exit: true,
            env: opts.env.into_iter().collect(),
        };
        let size = WindowSize {
            num_lines: opts.rows as u16,
            num_cols: opts.cols as u16,
            cell_width: opts.cell_width,
            cell_height: opts.cell_height,
        };
        let pty = tty::new(&options, size, 0)?;
        let event_loop = EventLoop::new(self.term.clone(), self.proxy(), pty, true, false)?;
        let writer = event_loop.channel();
        *self.proxy.writer.lock().unwrap() = Some(writer.clone());
        self.writer = Some(writer);
        self.join = Some(event_loop.spawn());
        Ok(())
    }
}

impl Drop for Session {
    /// Shutting the event loop down drops the PTY, which hangs up on the
    /// child (SIGHUP + reap on Unix; ConPTY teardown terminates the shell
    /// on Windows) — no orphaned shells when the app quits.
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            let _ = writer.send(Msg::Shutdown);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
