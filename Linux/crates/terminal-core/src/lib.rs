//! Portable embedded-terminal engine behind `pitex-shell`'s
//! `compat::ShellTerminal` — everything the GTK terminal widget needs that
//! doesn't depend on GTK:
//!
//! - [`Session`]: emulator + PTY (openpty on Unix, ConPTY on Windows 10
//!   1809+), shell spawn, resize, scrollback, and the UI event queue.
//! - [`input`]: key press → xterm byte sequences.
//!
//! The VT layer is `alacritty_terminal`, re-exported so the widget can read
//! the grid (`term.renderable_content()`) and drive selections without a
//! second copy of the types.

mod input;
mod session;

pub use input::{encode_key, encode_paste, Key, KeyModes, Modifiers};
pub use session::{
    default_shell, ClipboardFormatter, ColorFormatter, Proxy, Session, SessionEvent, SizeFormatter,
    SpawnOptions,
};

pub use alacritty_terminal;
pub use alacritty_terminal::event::WindowSize;
pub use alacritty_terminal::grid::Scroll;
pub use alacritty_terminal::term::ClipboardType;
pub use alacritty_terminal::tty::Shell;
pub use alacritty_terminal::vte::ansi::Rgb;
