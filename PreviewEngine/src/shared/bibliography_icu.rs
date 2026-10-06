// Pitex-authored bindings to the existing system ICU library.
// SPDX-License-Identifier: AGPL-3.0-or-later
// ICU supplies Unicode algorithms, not Biber/Perl implementation code. These
// handles are local to one auxiliary pass and never survive engine checkpoints.
#[cfg(pitex_native_icu)]
mod native {
    use std::ffi::{c_char, c_void, CString};

    extern "C" {
        #[link_name = concat!("ucol_open_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn col_open(locale: *const c_char, status: *mut i32) -> *mut c_void;
        #[link_name = concat!("ucol_close_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn col_close(collator: *mut c_void);
        #[link_name = concat!("ucol_setAttribute_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn col_attribute(collator: *mut c_void, attribute: i32, value: i32, status: *mut i32);
        #[link_name = concat!("ucol_getSortKey_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn col_key(
            collator: *const c_void,
            source: *const u16,
            length: i32,
            out: *mut u8,
            capacity: i32,
        ) -> i32;
        #[link_name = concat!("uregex_open_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_open(
            pattern: *const u16,
            length: i32,
            flags: u32,
            parse_error: *mut c_void,
            status: *mut i32,
        ) -> *mut c_void;
        #[link_name = concat!("uregex_close_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_close(expression: *mut c_void);
        #[link_name = concat!("uregex_setText_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_text(expression: *mut c_void, text: *const u16, length: i32, status: *mut i32);
        #[link_name = concat!("uregex_findNext_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_next(expression: *mut c_void, status: *mut i32) -> i8;
        #[link_name = concat!("uregex_start_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_start(expression: *mut c_void, group: i32, status: *mut i32) -> i32;
        #[link_name = concat!("uregex_end_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_end(expression: *mut c_void, group: i32, status: *mut i32) -> i32;
        #[link_name = concat!("uregex_groupCount_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_count(expression: *mut c_void, status: *mut i32) -> i32;
        #[link_name = concat!("uregex_groupNumberFromName_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_named(
            expression: *mut c_void,
            name: *const u16,
            length: i32,
            status: *mut i32,
        ) -> i32;
        #[link_name = concat!("uregex_setTimeLimit_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_time(expression: *mut c_void, limit: i32, status: *mut i32);
        #[link_name = concat!("uregex_setStackLimit_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn re_stack(expression: *mut c_void, limit: i32, status: *mut i32);
        #[link_name = concat!("u_errorName_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn error_name(status: i32) -> *const c_char;
        #[link_name = concat!("unorm2_getNFDInstance_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn nfd(status: *mut i32) -> *const c_void;
        #[link_name = concat!("unorm2_getNFCInstance_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn nfc(status: *mut i32) -> *const c_void;
        #[link_name = concat!("unorm2_normalize_", env!("PITEX_PREVIEW_ICU_MAJOR"))]
        fn normalize_utf16(
            normalizer: *const c_void,
            source: *const u16,
            length: i32,
            destination: *mut u16,
            capacity: i32,
            status: *mut i32,
        ) -> i32;
    }
    fn check(status: i32) -> Result<(), String> {
        if status <= 0 {
            Ok(())
        } else {
            Err(unsafe {
                std::ffi::CStr::from_ptr(error_name(status))
                    .to_string_lossy()
                    .into_owned()
            })
        }
    }
    // The documented bibliography interchange normalizes input to NFD and
    // UTF-8 output to NFC. In particular \p{L} excludes decomposed accent marks.
    fn normalize(text: &str, compose: bool) -> Result<String, String> {
        let source = text.encode_utf16().collect::<Vec<_>>();
        let mut status = 0;
        let normalizer = unsafe {
            if compose {
                nfc(&mut status)
            } else {
                nfd(&mut status)
            }
        };
        check(status)?;
        let length = unsafe {
            normalize_utf16(
                normalizer,
                source.as_ptr(),
                source.len() as i32,
                std::ptr::null_mut(),
                0,
                &mut status,
            )
        };
        if status != 15 {
            check(status)?;
        }
        if length < 0 || length > 16 * 1024 * 1024 {
            return Err("Bibliography normalization output limit exceeded".into());
        }
        let mut result = vec![0; length as usize];
        status = 0;
        unsafe {
            normalize_utf16(
                normalizer,
                source.as_ptr(),
                source.len() as i32,
                result.as_mut_ptr(),
                length,
                &mut status,
            );
        }
        check(status)?;
        String::from_utf16(&result).map_err(|_| "Invalid Unicode normalization result".into())
    }
    pub(crate) struct Collator(*mut c_void);
    impl Drop for Collator {
        fn drop(&mut self) {
            unsafe { col_close(self.0) }
        }
    }
    impl Collator {
        pub(crate) fn new(locale: &str, case: bool, upper: bool) -> Result<Self, String> {
            let locale = CString::new(locale).map_err(|_| "NUL in bibliography sorting locale")?;
            let mut status = 0;
            let collator = unsafe { col_open(locale.as_ptr(), &mut status) };
            check(status)?;
            if collator.is_null() {
                return Err("ICU returned a null collator".into());
            }
            let result = Self(collator);
            // UCOL_NORMALIZATION_MODE=4, UCOL_CASE_FIRST=2, UCOL_STRENGTH=5.
            // Secondary strength preserves accents but ignores case, as sortcase=false.
            unsafe {
                col_attribute(collator, 4, 17, &mut status);
                col_attribute(collator, 2, if upper { 25 } else { 24 }, &mut status);
                col_attribute(collator, 5, if case { 2 } else { 1 }, &mut status);
            }
            check(status)?;
            Ok(result)
        }
        pub(crate) fn key(&self, text: &str) -> Vec<u8> {
            let source = text.encode_utf16().collect::<Vec<_>>();
            let size = unsafe {
                col_key(
                    self.0,
                    source.as_ptr(),
                    source.len() as i32,
                    std::ptr::null_mut(),
                    0,
                )
            };
            let mut key = vec![0; size.max(0) as usize];
            unsafe {
                col_key(
                    self.0,
                    source.as_ptr(),
                    source.len() as i32,
                    key.as_mut_ptr(),
                    size,
                );
            }
            key
        }
    }
    struct Expression(*mut c_void);
    impl Drop for Expression {
        fn drop(&mut self) {
            unsafe { re_close(self.0) }
        }
    }
    // ICU accepts the usual Perl lookarounds, captures, backreferences, atomic
    // groups, possessive quantifiers, inline flags and Unicode properties. Normalize
    // Perl's alternative spelling for named groups and explicit backreferences.
    fn pattern_spelling(pattern: &str) -> Result<String, String> {
        let mut out = String::new();
        let mut chars = pattern.chars().peekable();
        let mut class = false;
        let mut quoted = false;
        while let Some(c) = chars.next() {
            if c == '\\' {
                let Some(next) = chars.next() else {
                    out.push(c);
                    break;
                };
                if !class && next == 'Q' {
                    quoted = true;
                } else if quoted && next == 'E' {
                    quoted = false;
                }
                if !class && !quoted && (next == 'g' || next == 'k') && chars.peek() == Some(&'{') {
                    chars.next();
                    let mut name = String::new();
                    let mut closed = false;
                    while let Some(c) = chars.next() {
                        if c == '}' {
                            closed = true;
                            break;
                        }
                        name.push(c);
                    }
                    if !closed {
                        return Err("Unclosed Perl bibliography backreference".into());
                    }
                    if name.starts_with(['-', '+']) {
                        return Err(
                            "Relative Perl bibliography backreferences are unsupported".into()
                        );
                    }
                    if next == 'g' && !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
                        out.push('\\');
                        out.push_str(&name);
                    } else {
                        out.push_str("\\k<");
                        out.push_str(&name);
                        out.push('>');
                    }
                } else {
                    out.push('\\');
                    out.push(next);
                }
                continue;
            }
            if quoted {
                out.push(c);
                continue;
            }
            if c == '[' {
                class = true;
            } else if c == ']' {
                class = false;
            }
            out.push(c);
            if !class && c == '(' && chars.peek() == Some(&'?') {
                chars.next();
                out.push('?');
                if chars.peek() == Some(&'\'') {
                    chars.next();
                    out.push('<');
                    while let Some(c) = chars.next() {
                        if c == '\'' {
                            break;
                        }
                        out.push(c);
                    }
                    out.push('>');
                }
            }
        }
        Ok(out)
    }
    fn capture(expression: &Expression, source: &[u16], group: i32) -> Result<String, String> {
        let mut status = 0;
        let start = unsafe { re_start(expression.0, group, &mut status) };
        let end = unsafe { re_end(expression.0, group, &mut status) };
        check(status)?;
        if start < 0 {
            return Ok(String::new());
        }
        let value = source
            .get(start as usize..end as usize)
            .ok_or("Invalid ICU capture range")?;
        String::from_utf16(value).map_err(|_| "Invalid Unicode capture".into())
    }
    // Replacement text is data. Interpret Perl's documented capture and case
    // interpolation forms without evaluating Perl, variables, or executable code.
    fn replacement(expression: &Expression, source: &[u16], value: &str) -> Result<String, String> {
        let mut status = 0;
        let count = unsafe { re_count(expression.0, &mut status) };
        check(status)?;
        let mut chars = value.chars().peekable();
        let mut out = String::new();
        let mut case = None;
        let mut once = None;
        while let Some(c) = chars.next() {
            let piece = if c == '\\' {
                match chars.next() {
                    Some('U') => {
                        case = Some(true);
                        continue;
                    }
                    Some('L') => {
                        case = Some(false);
                        continue;
                    }
                    Some('u') => {
                        once = Some(true);
                        continue;
                    }
                    Some('l') => {
                        once = Some(false);
                        continue;
                    }
                    Some('E') => {
                        case = None;
                        continue;
                    }
                    Some('n') => "\n".into(),
                    Some('r') => "\r".into(),
                    Some('t') => "\t".into(),
                    Some(c) => c.to_string(),
                    None => "\\".into(),
                }
            } else if c == '$' {
                if chars.peek().is_some_and(|c| c.is_ascii_digit()) || chars.peek() == Some(&'{') {
                    let brace = chars.peek() == Some(&'{');
                    if brace {
                        chars.next();
                    }
                    let mut number = String::new();
                    while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
                        number.push(chars.next().unwrap());
                    }
                    if brace && chars.next() != Some('}') {
                        return Err("Unsupported bibliography replacement variable".into());
                    }
                    let group = number
                        .parse::<i32>()
                        .map_err(|_| "Unsupported bibliography replacement variable")?;
                    if group > count {
                        String::new()
                    } else {
                        capture(expression, source, group)?
                    }
                } else if chars.peek() == Some(&'+') {
                    chars.next();
                    if chars.next() != Some('{') {
                        return Err("Unsupported bibliography replacement variable".into());
                    }
                    let mut name = String::new();
                    let mut closed = false;
                    while let Some(c) = chars.next() {
                        if c == '}' {
                            closed = true;
                            break;
                        }
                        name.push(c);
                    }
                    if !closed {
                        return Err("Unclosed bibliography replacement capture name".into());
                    }
                    let name = name.encode_utf16().collect::<Vec<_>>();
                    let group = unsafe {
                        re_named(expression.0, name.as_ptr(), name.len() as i32, &mut status)
                    };
                    check(status)?;
                    capture(expression, source, group)?
                } else if chars.peek() == Some(&'&') {
                    chars.next();
                    capture(expression, source, 0)?
                } else if chars.peek() == Some(&'`') || chars.peek() == Some(&'\'') {
                    let pre = chars.next() == Some('`');
                    let index = unsafe {
                        if pre {
                            re_start(expression.0, 0, &mut status)
                        } else {
                            re_end(expression.0, 0, &mut status)
                        }
                    } as usize;
                    check(status)?;
                    String::from_utf16(if pre {
                        &source[..index]
                    } else {
                        &source[index..]
                    })
                    .map_err(|_| "Invalid Unicode replacement")?
                } else if chars.peek().is_some_and(|c| c.is_alphabetic() || *c == '_') {
                    return Err(
                        "Executable or variable Perl bibliography replacements are unsupported"
                            .into(),
                    );
                } else {
                    "$".into()
                }
            } else {
                c.to_string()
            };
            for c in piece.chars() {
                match once.take().or(case) {
                    Some(true) => out.extend(c.to_uppercase()),
                    Some(false) => out.extend(c.to_lowercase()),
                    None => out.push(c),
                }
            }
        }
        Ok(out)
    }
    pub(crate) fn regex(
        pattern: &str,
        text: &str,
        insensitive: bool,
        replace: Option<&str>,
    ) -> Result<(bool, Option<String>), String> {
        if pattern.len() > 65_536 || text.len() > 16 * 1024 * 1024 {
            return Err("Bibliography regex input limit exceeded".into());
        }
        let pattern = normalize(&pattern_spelling(pattern)?, false)?
            .encode_utf16()
            .collect::<Vec<_>>();
        let text = normalize(text, false)?;
        let replacement_text = replace
            .map(|replacement| normalize(replacement, false))
            .transpose()?;
        let replace = replacement_text.as_deref();
        let source = text.encode_utf16().collect::<Vec<_>>();
        let mut status = 0;
        let pointer = unsafe {
            re_open(
                pattern.as_ptr(),
                pattern.len() as i32,
                512 | if insensitive { 2 } else { 0 },
                std::ptr::null_mut(),
                &mut status,
            )
        };
        check(status)?;
        if pointer.is_null() {
            return Err("ICU returned a null regex".into());
        }
        let expression = Expression(pointer);
        unsafe {
            re_time(pointer, 100, &mut status);
            re_stack(pointer, 8 * 1024 * 1024, &mut status);
            re_text(pointer, source.as_ptr(), source.len() as i32, &mut status);
        }
        check(status)?;
        let mut found = false;
        let mut result = String::new();
        let mut last = 0usize;
        loop {
            let matched = unsafe { re_next(pointer, &mut status) } != 0;
            check(status)?;
            if !matched {
                break;
            }
            found = true;
            let Some(replace) = replace else {
                return Ok((true, None));
            };
            let start = unsafe { re_start(pointer, 0, &mut status) } as usize;
            let end = unsafe { re_end(pointer, 0, &mut status) } as usize;
            check(status)?;
            result.push_str(
                &String::from_utf16(&source[last..start]).map_err(|_| "Invalid Unicode match")?,
            );
            result.push_str(&replacement(&expression, &source, replace)?);
            if result.len() > 16 * 1024 * 1024 {
                return Err("Bibliography replacement output limit exceeded".into());
            }
            last = end;
        }
        if replace.is_some() {
            result.push_str(
                &String::from_utf16(&source[last..]).map_err(|_| "Invalid Unicode tail")?,
            );
        }
        Ok((
            found,
            replace.map(|_| normalize(&result, true)).transpose()?,
        ))
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn unicode_empty_matches_and_quoted_pattern() {
            assert_eq!(
                regex("(?=.)", "é😊", false, Some("-")).unwrap(),
                (true, Some("-e-\u{301}-😊".into()))
            );
            assert!(
                regex(r"\Q\g{1}(?'name')\E", r"\g{1}(?'name')", false, None)
                    .unwrap()
                    .0
            );
        }
        #[test]
        fn unsupported_perl_is_reported() {
            for pattern in [r"\g{1", r"\g{-1}", "(?R)", "(?{system('false')})"] {
                assert!(regex(pattern, "text", false, None).is_err(), "{pattern}");
            }
            assert!(regex("(text)", "text", false, Some("$variable")).is_err());
        }
        #[test]
        fn canonical_collation_and_case_settings() {
            let english = Collator::new("en_US", false, true).unwrap();
            assert_eq!(english.key("é"), english.key("e\u{301}"));
            assert_eq!(english.key("Alpha"), english.key("alpha"));
            let swedish = Collator::new("sv_SE", true, true).unwrap();
            assert!(swedish.key("Z") < swedish.key("Å"));
            let upper = Collator::new("en_US", true, true).unwrap();
            let lower = Collator::new("en_US", true, false).unwrap();
            assert!(upper.key("Alpha") < upper.key("alpha"));
            assert!(lower.key("alpha") < lower.key("Alpha"));
        }
    }
}
#[cfg(pitex_native_icu)]
pub(super) use native::{regex, Collator};

#[cfg(not(pitex_native_icu))]
pub(super) struct Collator;
#[cfg(not(pitex_native_icu))]
impl Collator {
    pub(super) fn new(_: &str, _: bool, _: bool) -> Result<Self, String> {
        Err("System ICU is unavailable for bibliography controls".into())
    }
    pub(super) fn key(&self, _: &str) -> Vec<u8> {
        unreachable!("a collator cannot be created without ICU")
    }
}
#[cfg(not(pitex_native_icu))]
pub(super) fn regex(
    _: &str,
    _: &str,
    _: bool,
    _: Option<&str>,
) -> Result<(bool, Option<String>), String> {
    Err("System ICU is unavailable for bibliography sourcemaps".into())
}
