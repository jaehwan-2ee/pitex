//! Keyboard input → xterm byte sequences.
//!
//! `Key`/`Modifiers` are the platform-neutral surface; the GTK widget maps
//! `gdk::Key`/`gdk::ModifierType` onto them. Encoding follows xterm:
//! modifier keys encode as a parameter (`1;{1+shift+2·alt+4·ctrl}`),
//! unmodified navigation keys use SS3 when the application cursor mode is
//! on, and Alt prefixes printable keys with ESC.

/// Keys the terminal encodes itself. Printable characters without
/// modifiers return `None` so the caller can route them through the IM
/// context (IME commit) instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    Backspace,
    Tab,
    Enter,
    Escape,
    F(u8),
    /// A printable key — only relevant when Ctrl/Alt are held (plain text
    /// arrives via the IM context's `commit`).
    Char(char),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl Modifiers {
    fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt
    }

    /// xterm modifier parameter: 1 + shift(1) + alt(2) + ctrl(4).
    fn param(&self) -> u8 {
        1 + self.shift as u8 + self.alt as u8 * 2 + self.ctrl as u8 * 4
    }
}

/// Emulator modes that change encoding (snapshot of `TermMode`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct KeyModes {
    /// DECCKM — arrows/Home/End send SS3 instead of CSI.
    pub app_cursor_keys: bool,
    /// Wrap pasted text in `\x1b[200~` … `\x1b[201~`.
    pub bracketed_paste: bool,
}

fn csi(final_byte: u8, mods: Modifiers) -> Vec<u8> {
    if mods.any() {
        format!("\x1b[1;{}{}", mods.param(), final_byte as char).into_bytes()
    } else {
        vec![0x1b, b'[', final_byte]
    }
}

fn ss3_or_csi(final_byte: u8, mods: Modifiers, app_cursor: bool) -> Vec<u8> {
    if mods.any() {
        csi(final_byte, mods)
    } else if app_cursor {
        vec![0x1b, b'O', final_byte]
    } else {
        vec![0x1b, b'[', final_byte]
    }
}

fn tilde(n: u8, mods: Modifiers) -> Vec<u8> {
    if mods.any() {
        format!("\x1b[{n};{}~", mods.param()).into_bytes()
    } else {
        format!("\x1b[{n}~").into_bytes()
    }
}

/// Ctrl+<key> control bytes, following xterm's table (Ctrl+2 → NUL,
/// Ctrl+6 → ^^, Ctrl+/ → ^? …).
fn control_byte(c: char) -> Option<u8> {
    match c.to_ascii_lowercase() {
        ' ' | '@' | '2' => Some(0x00),
        'a'..='z' => Some(c.to_ascii_lowercase() as u8 & 0x1f),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '7' | '/' => Some(0x1f),
        '8' | '?' => Some(0x7f),
        _ => None,
    }
}

/// Encode one key press. `None` → let the IM context / text path handle it.
pub fn encode_key(key: Key, mods: Modifiers, modes: KeyModes) -> Option<Vec<u8>> {
    let bytes = match key {
        Key::Up => ss3_or_csi(b'A', mods, modes.app_cursor_keys),
        Key::Down => ss3_or_csi(b'B', mods, modes.app_cursor_keys),
        Key::Right => ss3_or_csi(b'C', mods, modes.app_cursor_keys),
        Key::Left => ss3_or_csi(b'D', mods, modes.app_cursor_keys),
        Key::Home => ss3_or_csi(b'H', mods, modes.app_cursor_keys),
        Key::End => ss3_or_csi(b'F', mods, modes.app_cursor_keys),
        Key::Insert => tilde(2, mods),
        Key::Delete => tilde(3, mods),
        Key::PageUp => tilde(5, mods),
        Key::PageDown => tilde(6, mods),
        Key::F(n) => match n {
            1..=4 => {
                let b = b'P' + (n - 1);
                if mods.any() {
                    format!("\x1b[1;{}{}", mods.param(), b as char).into_bytes()
                } else {
                    vec![0x1b, b'O', b]
                }
            }
            // xterm: F5=15~ F6=17~ F7=18~ F8=19~ F9=20~ F10=21~ F11=23~ F12=24~
            5..=12 => tilde([15, 17, 18, 19, 20, 21, 23, 24][(n - 5) as usize], mods),
            _ => return None,
        },
        Key::Backspace => vec![0x7f],
        Key::Tab if mods.shift => b"\x1b[Z".to_vec(),
        Key::Tab => vec![b'\t'],
        Key::Enter if mods.alt => b"\x1b\r".to_vec(),
        Key::Enter => vec![b'\r'],
        Key::Escape if mods.alt => b"\x1b\x1b".to_vec(),
        Key::Escape => vec![0x1b],
        Key::Char(c) => {
            if mods.ctrl {
                let mut out = if mods.alt { vec![0x1b] } else { Vec::new() };
                out.push(control_byte(c)?);
                out
            } else if mods.alt {
                let mut out = vec![0x1b];
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                out
            } else {
                return None;
            }
        }
    };
    Some(bytes)
}

/// Pasted text for the PTY. Newlines become CR (what shells treat as
/// Enter) and bracketed-paste mode wraps the payload so readline doesn't
/// execute it line by line.
pub fn encode_paste(text: &str, modes: KeyModes) -> Vec<u8> {
    let text = text.replace('\n', "\r");
    if modes.bracketed_paste {
        [b"\x1b[200~", text.as_bytes(), b"\x1b[201~"].concat()
    } else {
        text.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Modifiers = Modifiers { shift: false, ctrl: false, alt: false };
    const SHIFT: Modifiers = Modifiers { shift: true, ctrl: false, alt: false };
    const CTRL: Modifiers = Modifiers { shift: false, ctrl: true, alt: false };
    const ALT: Modifiers = Modifiers { shift: false, ctrl: false, alt: true };

    #[test]
    fn arrows_normal_and_application_mode() {
        let normal = KeyModes::default();
        let app = KeyModes { app_cursor_keys: true, ..Default::default() };
        assert_eq!(encode_key(Key::Up, NONE, normal).unwrap(), b"\x1b[A");
        assert_eq!(encode_key(Key::Up, NONE, app).unwrap(), b"\x1bOA");
        assert_eq!(encode_key(Key::Left, NONE, app).unwrap(), b"\x1bOD");
        // Modifiers force the CSI form even in application mode.
        assert_eq!(encode_key(Key::Up, CTRL, app).unwrap(), b"\x1b[1;5A");
        assert_eq!(encode_key(Key::Right, SHIFT, normal).unwrap(), b"\x1b[1;2C");
    }

    #[test]
    fn navigation_keys() {
        let modes = KeyModes::default();
        assert_eq!(encode_key(Key::Home, NONE, modes).unwrap(), b"\x1b[H");
        assert_eq!(encode_key(Key::End, NONE, modes).unwrap(), b"\x1b[F");
        assert_eq!(encode_key(Key::PageUp, NONE, modes).unwrap(), b"\x1b[5~");
        assert_eq!(encode_key(Key::PageDown, CTRL, modes).unwrap(), b"\x1b[6;5~");
        assert_eq!(encode_key(Key::Delete, NONE, modes).unwrap(), b"\x1b[3~");
        assert_eq!(encode_key(Key::Insert, NONE, modes).unwrap(), b"\x1b[2~");
    }

    #[test]
    fn function_keys() {
        let modes = KeyModes::default();
        assert_eq!(encode_key(Key::F(1), NONE, modes).unwrap(), b"\x1bOP");
        assert_eq!(encode_key(Key::F(4), NONE, modes).unwrap(), b"\x1bOS");
        assert_eq!(encode_key(Key::F(5), NONE, modes).unwrap(), b"\x1b[15~");
        assert_eq!(encode_key(Key::F(12), NONE, modes).unwrap(), b"\x1b[24~");
        assert_eq!(encode_key(Key::F(3), SHIFT, modes).unwrap(), b"\x1b[1;2R");
        assert!(encode_key(Key::F(13), NONE, modes).is_none());
    }

    #[test]
    fn editing_keys() {
        let modes = KeyModes::default();
        assert_eq!(encode_key(Key::Backspace, NONE, modes).unwrap(), b"\x7f");
        assert_eq!(encode_key(Key::Tab, NONE, modes).unwrap(), b"\t");
        assert_eq!(encode_key(Key::Tab, SHIFT, modes).unwrap(), b"\x1b[Z");
        assert_eq!(encode_key(Key::Enter, NONE, modes).unwrap(), b"\r");
        assert_eq!(encode_key(Key::Enter, ALT, modes).unwrap(), b"\x1b\r");
        assert_eq!(encode_key(Key::Escape, NONE, modes).unwrap(), b"\x1b");
    }

    #[test]
    fn ctrl_and_alt_chars() {
        let modes = KeyModes::default();
        assert_eq!(encode_key(Key::Char('c'), CTRL, modes).unwrap(), b"\x03");
        assert_eq!(encode_key(Key::Char('d'), CTRL, modes).unwrap(), b"\x04");
        assert_eq!(encode_key(Key::Char('a'), ALT, modes).unwrap(), b"\x1ba");
        assert_eq!(
            encode_key(
                Key::Char('c'),
                Modifiers { ctrl: true, alt: true, shift: false },
                modes
            )
            .unwrap(),
            b"\x1b\x03"
        );
        // Plain characters defer to the IM context.
        assert!(encode_key(Key::Char('a'), NONE, modes).is_none());
        assert!(encode_key(Key::Char('A'), SHIFT, modes).is_none());
        // Non-ASCII text isn't a control key.
        assert!(encode_key(Key::Char('한'), CTRL, modes).is_none());
    }

    #[test]
    fn bracketed_and_plain_paste() {
        let off = KeyModes::default();
        let on = KeyModes { bracketed_paste: true, ..Default::default() };
        assert_eq!(encode_paste("a\nb", off), b"a\rb");
        assert_eq!(encode_paste("a\nb", on), b"\x1b[200~a\rb\x1b[201~");
    }
}
