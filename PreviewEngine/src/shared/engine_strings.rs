// Pitex-authored pdfTeX-compatible string/status services.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// POSIX ERE matching uses the platform libc already dynamically linked by this
// engine. It preserves POSIX leftmost-longest semantics rather than translating
// them into a different regular-expression dialect. No pdfTeX code is copied.
use std::ffi::CString;

#[derive(Clone, Debug, Default)]
pub struct MatchTable {
    entries: Vec<Option<(usize, String)>>,
}
impl MatchTable {
    pub fn last(&self, index: i32) -> Result<String, &'static str> {
        if index < 0 {
            return Err("Negative submatch number");
        }
        Ok(
            match self.entries.get(index as usize).and_then(Option::as_ref) {
                Some((start, text)) => format!("{start}->{text}"),
                None => "-1->".into(),
            },
        )
    }
    pub fn find(&mut self, pattern: &str, text: &str, icase: bool, subcount: i32) -> i32 {
        if subcount < 0 || subcount > 1_000_000 {
            return -1;
        }
        let pattern = pattern.split('\0').next().unwrap_or("");
        let text = text.split('\0').next().unwrap_or("");
        let pattern = CString::new(pattern).unwrap();
        let input = CString::new(text).unwrap();
        unsafe {
            let mut expression = std::mem::MaybeUninit::<libc::regex_t>::zeroed();
            let flags = libc::REG_EXTENDED | if icase { libc::REG_ICASE } else { 0 };
            if libc::regcomp(expression.as_mut_ptr(), pattern.as_ptr(), flags) != 0 {
                return -1;
            }
            let mut expression = expression.assume_init();
            // Stock pdfTeX retains the previous capture table when regcomp
            // rejects a new pattern, but clears it for a valid no-match call.
            self.entries.clear();
            let mut matches = vec![
                libc::regmatch_t {
                    rm_so: -1,
                    rm_eo: -1
                };
                subcount as usize
            ];
            let code = libc::regexec(
                &expression,
                input.as_ptr(),
                matches.len(),
                if matches.is_empty() {
                    std::ptr::null_mut()
                } else {
                    matches.as_mut_ptr()
                },
                0,
            );
            libc::regfree(&mut expression);
            if code == libc::REG_NOMATCH {
                return 0;
            }
            if code != 0 {
                return -1;
            }
            self.entries = matches
                .into_iter()
                .map(|matched| {
                    if matched.rm_so < 0 || matched.rm_eo < matched.rm_so {
                        return None;
                    }
                    let begin = matched.rm_so as usize;
                    let end = matched.rm_eo as usize;
                    if end > text.len() {
                        return None;
                    }
                    Some((
                        begin,
                        String::from_utf8_lossy(&text.as_bytes()[begin..end]).into_owned(),
                    ))
                })
                .collect();
            1
        }
    }
}
// The TeX engine is single-threaded. Owned strings survive its fork checkpoints;
// regex_t itself is deliberately never retained in checkpointed global state.
static mut MATCHES: MatchTable = MatchTable {
    entries: Vec::new(),
};
static mut PDF_RETURN_STATUS: i32 = 0;
pub unsafe fn pdf_match(pattern: &str, text: &str, icase: bool, subcount: i32) -> i32 {
    MATCHES.find(pattern, text, icase, subcount)
}
pub unsafe fn pdf_last_match(index: i32) -> Result<String, &'static str> {
    MATCHES.last(index)
}
pub unsafe fn pdf_return_status() -> i32 {
    PDF_RETURN_STATUS
}
// pdfTeX leaves this error status sticky even after a later successful pdfobj.
pub unsafe fn invalid_pdf_object() {
    PDF_RETURN_STATUS = -1;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn posix_longest_and_subcount() {
        let mut state = MatchTable::default();
        assert_eq!(state.find("a|aa", "aa", false, 10), 1);
        assert_eq!(state.last(0).unwrap(), "0->aa");
        assert_eq!(state.find("ab(cd)*ef(gh)(ij)", "abefghij", false, 3), 1);
        assert_eq!(state.last(0).unwrap(), "0->abefghij");
        assert_eq!(state.last(1).unwrap(), "-1->");
        assert_eq!(state.last(2).unwrap(), "4->gh");
        assert_eq!(state.last(3).unwrap(), "-1->");
    }
    #[test]
    fn case_invalid_and_clearing() {
        let mut state = MatchTable::default();
        assert_eq!(state.find("ABC", "xxabc", true, 10), 1);
        assert_eq!(state.last(0).unwrap(), "2->abc");
        assert_eq!(state.find("z+", "abc", false, 10), 0);
        assert_eq!(state.last(0).unwrap(), "-1->");
        assert_eq!(state.find("(", "abc", false, 10), -1);
        assert_eq!(state.last(0).unwrap(), "-1->");
    }
    #[test]
    fn zero_table_and_negative_query() {
        let mut state = MatchTable::default();
        assert_eq!(state.find("a", "a", false, 0), 1);
        assert_eq!(state.last(0).unwrap(), "-1->");
        assert!(state.last(-1).is_err());
    }
    #[test]
    fn invalid_pattern_preserves_previous_matches() {
        let mut state = MatchTable::default();
        assert_eq!(state.find("(ab)", "xxab", false, 10), 1);
        assert_eq!(state.find("(", "yyy", false, 10), -1);
        assert_eq!(state.last(1).unwrap(), "2->ab");
    }
}
