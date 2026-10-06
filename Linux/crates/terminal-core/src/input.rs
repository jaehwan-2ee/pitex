//! Keyboard and mouse input → xterm byte sequences.
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
    Begin,
    PageUp,
    PageDown,
    Insert,
    Delete,
    Backspace,
    Tab,
    Enter,
    Escape,
    F(u8),
    /// Keypad PF1..PF4 have fixed sequences, including with modifiers.
    KeypadF(u8),
    /// Keypad Space, Tab, or Separator (','); modifiers do not change them.
    Keypad(char),
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
    /// DECKPAM — keypad Space/Tab/Separator send SS3 sequences.
    pub app_keypad: bool,
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
        Key::Begin => ss3_or_csi(b'E', mods, modes.app_cursor_keys),
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
        Key::KeypadF(n) => match n {
            1..=4 => vec![0x1b, b'O', b'P' + n - 1],
            _ => return None,
        },
        Key::Keypad(c) => {
            let final_byte = match c {
                ' ' => b' ',
                '\t' => b'I',
                ',' => b'l',
                _ => return None,
            };
            if modes.app_keypad {
                vec![0x1b, b'O', final_byte]
            } else {
                vec![c as u8]
            }
        }
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
    // Windows clipboards commonly contain CRLF. Collapse the pair first
    // so one pasted newline sends one Enter instead of two commands.
    let text = text.replace("\r\n", "\n").replace('\n', "\r");
    if modes.bracketed_paste {
        [b"\x1b[200~", text.as_bytes(), b"\x1b[201~"].concat()
    } else {
        text.into_bytes()
    }
}

/// Encode an xterm mouse report with zero-based cell coordinates. `code`
/// includes the button, modifiers (Shift=4, Alt=8, Ctrl=16), and motion/wheel
/// bits. SGR (1006) takes precedence over UTF-8 (1005). Return `None` when a
/// coordinate or button cannot be represented in the selected encoding.
pub fn encode_mouse(
    code: u8,
    column: usize,
    row: usize,
    released: bool,
    sgr: bool,
    utf8: bool,
) -> Option<Vec<u8>> {
    if sgr {
        let column = column.checked_add(1)?;
        let row = row.checked_add(1)?;
        let final_byte = if released { 'm' } else { 'M' };
        return Some(format!("\x1b[<{code};{column};{row}{final_byte}").into_bytes());
    }

    let limit = if utf8 { 2015 } else { 223 };
    if column >= limit || row >= limit {
        return None;
    }
    let code = if released { (code & 28) | 3 } else { code };
    let values = [u32::from(code) + 32, column as u32 + 33, row as u32 + 33];
    let mut bytes = b"\x1b[M".to_vec();
    for value in values {
        if utf8 {
            let mut buffer = [0; 4];
            bytes.extend_from_slice(char::from_u32(value)?.encode_utf8(&mut buffer).as_bytes());
        } else {
            bytes.push(u8::try_from(value).ok()?);
        }
    }
    Some(bytes)
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
    fn keypad_modes_and_fixed_modifier_behavior() {
        for app_cursor_keys in [false, true] {
            for app_keypad in [false, true] {
                let modes = KeyModes { app_cursor_keys, app_keypad, ..Default::default() };
                for bits in 0..8 {
                    let mods = Modifiers {
                        shift: bits & 1 != 0,
                        alt: bits & 2 != 0,
                        ctrl: bits & 4 != 0,
                    };
                    for (key, numeric, application) in [
                        (' ', &b" "[..], &b"\x1bO "[..]),
                        ('\t', &b"\t"[..], &b"\x1bOI"[..]),
                        (',', &b","[..], &b"\x1bOl"[..]),
                    ] {
                        let expected = if app_keypad { application } else { numeric };
                        assert_eq!(encode_key(Key::Keypad(key), mods, modes).unwrap(), expected);
                    }
                    for (key, expected) in [(1, b"\x1bOP"), (2, b"\x1bOQ"), (3, b"\x1bOR"), (4, b"\x1bOS")] {
                        assert_eq!(encode_key(Key::KeypadF(key), mods, modes).unwrap(), expected);
                    }
                }
            }
        }
    }

    #[test]
    fn keypad_begin_uses_cursor_mode_and_modifiers() {
        for app_cursor_keys in [false, true] {
            for app_keypad in [false, true] {
                let modes = KeyModes { app_cursor_keys, app_keypad, ..Default::default() };
                let expected = if app_cursor_keys { b"\x1bOE" } else { b"\x1b[E" };
                assert_eq!(encode_key(Key::Begin, NONE, modes).unwrap(), expected);
                for bits in 1..8 {
                    let mods = Modifiers {
                        shift: bits & 1 != 0,
                        alt: bits & 2 != 0,
                        ctrl: bits & 4 != 0,
                    };
                    let expected = format!("\x1b[1;{}E", bits + 1);
                    assert_eq!(encode_key(Key::Begin, mods, modes).unwrap(), expected.as_bytes());
                }
            }
        }
    }

    #[test]
    fn unsupported_keypad_keys_do_not_encode() {
        for app_keypad in [false, true] {
            let modes = KeyModes { app_keypad, ..Default::default() };
            for key in ['0', '9', '+', '/', '.', '=', '\r', 'P', '한'] {
                assert!(encode_key(Key::Keypad(key), NONE, modes).is_none());
            }
            for key in [0, 5, u8::MAX] {
                assert!(encode_key(Key::KeypadF(key), NONE, modes).is_none());
            }
        }
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
        assert_eq!(encode_paste("a\r\nb\rc\nd", off), b"a\rb\rc\rd");
        assert_eq!(encode_paste("한글\r\ntext", on), "\x1b[200~한글\rtext\x1b[201~".as_bytes());
    }

    #[test]
    fn mouse_buttons_motion_wheel_and_modifiers() {
        for (code, legacy, sgr) in [
            (0, b' ', "\x1b[<0;5;10M"),
            (1, b'!', "\x1b[<1;5;10M"),
            (2, b'\"', "\x1b[<2;5;10M"),
            (32, b'@', "\x1b[<32;5;10M"),
            (33, b'A', "\x1b[<33;5;10M"),
            (34, b'B', "\x1b[<34;5;10M"),
            (35, b'C', "\x1b[<35;5;10M"),
            (64, b'`', "\x1b[<64;5;10M"),
            (65, b'a', "\x1b[<65;5;10M"),
            (66, b'b', "\x1b[<66;5;10M"),
            (67, b'c', "\x1b[<67;5;10M"),
            (30, b'>', "\x1b[<30;5;10M"),
        ] {
            assert_eq!(
                encode_mouse(code, 4, 9, false, false, false).unwrap(),
                [0x1b, b'[', b'M', legacy, b'%', b'*']
            );
            assert_eq!(encode_mouse(code, 4, 9, false, true, false).unwrap(), sgr.as_bytes());
        }
    }

    #[test]
    fn mouse_release_keeps_modifiers_and_sgr_button_identity() {
        for (code, sgr) in [
            (28, "\x1b[<28;1;1m"),
            (29, "\x1b[<29;1;1m"),
            (30, "\x1b[<30;1;1m"),
        ] {
            assert_eq!(encode_mouse(code, 0, 0, true, false, false).unwrap(), b"\x1b[M?!!");
            assert_eq!(encode_mouse(code, 0, 0, true, false, true).unwrap(), b"\x1b[M?!!");
            assert_eq!(encode_mouse(code, 0, 0, true, true, false).unwrap(), sgr.as_bytes());
        }
        assert_eq!(encode_mouse(2, 0, 0, true, false, false).unwrap(), b"\x1b[M#!!");
    }

    #[test]
    fn mouse_legacy_coordinate_limits_do_not_wrap() {
        assert_eq!(encode_mouse(0, 222, 222, false, false, false).unwrap(), b"\x1b[M \xff\xff");
        assert!(encode_mouse(0, 223, 0, false, false, false).is_none());
        assert!(encode_mouse(0, 0, 223, false, false, false).is_none());
        assert!(encode_mouse(224, 0, 0, false, false, false).is_none());
    }

    #[test]
    fn mouse_utf8_encodes_coordinates_and_button_value() {
        assert_eq!(encode_mouse(0, 94, 94, false, false, true).unwrap(), b"\x1b[M \x7f\x7f");
        assert_eq!(encode_mouse(0, 95, 95, false, false, true).unwrap(), b"\x1b[M \xc2\x80\xc2\x80");
        assert_eq!(encode_mouse(128, 0, 0, false, false, true).unwrap(), b"\x1b[M\xc2\xa0!!");
        assert_eq!(encode_mouse(0, 2014, 2014, false, false, true).unwrap(), b"\x1b[M \xdf\xbf\xdf\xbf");
        assert!(encode_mouse(0, 2015, 0, false, false, true).is_none());
        assert!(encode_mouse(0, 0, 2015, false, false, true).is_none());
    }

    #[test]
    fn mouse_sgr_supports_large_coordinates_and_takes_precedence() {
        assert_eq!(encode_mouse(2, 8191, 65535, false, true, true).unwrap(), b"\x1b[<2;8192;65536M");
        for (column, row) in [(usize::MAX, 0), (0, usize::MAX)] {
            for (sgr, utf8) in [(false, false), (false, true), (true, false)] {
                assert!(encode_mouse(0, column, row, false, sgr, utf8).is_none());
            }
        }
    }
}
