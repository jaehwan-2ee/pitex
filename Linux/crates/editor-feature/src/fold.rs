//! Pure fold-region discovery — the platform-neutral half of
//! `EditorFolding.swift`. `FoldEngine` (in `pitex-shell`) binds these results
//! to a GtkSourceView; everything here is text-in/regions-out so the scan is
//! verifiable without a display.

use language_core::{DeterministicTeXLexer, LanguageTokenKind, TeXDialect};

/// One foldable region (verbatim `FoldRegion`): `hidden_line_range` is the
/// closed line range collapsed while folded — everything after the header
/// line through the closing line inclusive.
#[derive(Debug, Clone, PartialEq)]
pub struct FoldRegion {
    pub header_line: usize,
    pub hidden_line_range: (usize, usize),
    /// Stable-ish identity carried across edits: env/section name + the
    /// header line's trimmed content.
    pub signature: String,
    pub folded: bool,
}

/// Line index → (byte offset, char offset) of the line start. Token ranges
/// are UTF-8 bytes; text iters take unichar offsets, so both are tracked.
pub fn compute_line_starts(text: &str) -> Vec<(usize, usize)> {
    let mut starts = vec![(0usize, 0usize)];
    let mut chars = 0usize;
    for (byte, c) in text.char_indices() {
        chars += 1;
        if c == '\n' {
            starts.push((byte + 1, chars));
        }
    }
    starts
}

/// `lineIndex(forChar:)` — binary search for the line containing a byte
/// offset (Swift searched by utf16 location; byte offsets order identically).
pub fn line_index(line_starts: &[(usize, usize)], byte_offset: usize) -> usize {
    let mut lo = 0usize;
    let mut hi = line_starts.len() - 1;
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        if line_starts[mid].0 <= byte_offset {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

/// `signature(command:headerLine:)` — `"<command>|<trimmed header line>"`.
pub fn header_signature(
    text: &str,
    line_starts: &[(usize, usize)],
    command: &str,
    line: usize,
) -> String {
    let start = line_starts[line].0;
    let end = if line + 1 < line_starts.len() {
        line_starts[line + 1].0
    } else {
        text.len()
    };
    let content = text[start..end.min(text.len())].trim();
    format!("{command}|{content}")
}

/// `findRegions` — the token-level scan: `\begin{X}`/`\end{X}` pairs by
/// name, section commands folded through the line before the next
/// same-or-higher-level section.
pub fn find_regions(
    text: &str,
    line_starts: &[(usize, usize)],
    dialect: TeXDialect,
) -> Vec<FoldRegion> {
    let tokens = DeterministicTeXLexer::tokenize(text, dialect);
    let mut regions: Vec<FoldRegion> = Vec::new();
    let mut env_stack: Vec<(String, usize, String)> = Vec::new();
    let mut open_sections: Vec<(i32, usize, String)> = Vec::new();

    let section_level = |name: &str| -> Option<i32> {
        match name {
            "part" => Some(0),
            "chapter" => Some(1),
            "section" => Some(2),
            "subsection" => Some(3),
            "subsubsection" => Some(4),
            "paragraph" => Some(5),
            _ => None,
        }
    };

    // `closeSections(atOrAbove:endLine:)` — pops every open section at or
    // above `level`, emitting a region when the body spans >1 hidden line.
    fn close_sections(
        open_sections: &mut Vec<(i32, usize, String)>,
        regions: &mut Vec<FoldRegion>,
        level: i32,
        end_line: usize,
    ) {
        while let Some(last) = open_sections.last() {
            if last.0 < level {
                break;
            }
            let last = open_sections.pop().unwrap();
            if last.1 + 1 < end_line {
                regions.push(FoldRegion {
                    header_line: last.1,
                    hidden_line_range: (last.1 + 1, end_line - 1),
                    signature: last.2,
                    folded: false,
                });
            }
        }
    }

    let mut i = 0usize;
    while i < tokens.len() {
        let token = &tokens[i];
        let name = match &token.kind {
            LanguageTokenKind::ControlSequence(name) => name.as_str(),
            _ => {
                i += 1;
                continue;
            }
        };
        if name == "begin" || name == "end" {
            // The environment name follows as the next environmentName token.
            let mut name_value: Option<String> = None;
            let mut j = i + 1;
            while j < tokens.len() && j < i + 4 {
                match &tokens[j].kind {
                    LanguageTokenKind::EnvironmentName(env) => {
                        name_value = Some(env.clone());
                        break;
                    }
                    LanguageTokenKind::Whitespace(_) | LanguageTokenKind::LeftBrace => j += 1,
                    _ => break,
                }
            }
            let start = token.range.utf8_offset.max(0) as usize;
            if start >= text.len() {
                i += 1;
                continue;
            }
            let line = line_index(line_starts, start);
            if name == "begin" {
                if let Some(env_name) = name_value {
                    env_stack.push((
                        env_name.clone(),
                        line,
                        header_signature(
                            text,
                            line_starts,
                            &format!("begin:{env_name}"),
                            line,
                        ),
                    ));
                }
            } else if let Some(env_name) = name_value {
                if let Some(match_index) = env_stack.iter().rposition(|e| e.0 == env_name) {
                    let entry = env_stack.remove(match_index);
                    if entry.1 + 1 < line {
                        regions.push(FoldRegion {
                            header_line: entry.1,
                            hidden_line_range: (entry.1 + 1, line),
                            signature: entry.2,
                            folded: false,
                        });
                    }
                }
            }
        } else if let Some(level) = section_level(name) {
            let start = token.range.utf8_offset.max(0) as usize;
            if start < text.len() {
                let line = line_index(line_starts, start);
                close_sections(&mut open_sections, &mut regions, level, line);
                open_sections.push((
                    level,
                    line,
                    header_signature(text, line_starts, &format!("section:{name}"), line),
                ));
            }
        }
        i += 1;
    }
    close_sections(&mut open_sections, &mut regions, -1, line_starts.len());
    regions.sort_by_key(|r| r.header_line);
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn regions_of(text: &str) -> Vec<FoldRegion> {
        let starts = compute_line_starts(text);
        find_regions(text, &starts, TeXDialect::Latex)
    }

    #[test]
    fn environment_region_spans_begin_to_end() {
        let text = "\\begin{document}\nbody\nmore\n\\end{document}\n";
        let regions = regions_of(text);
        assert_eq!(regions.len(), 1);
        let r = &regions[0];
        assert_eq!(r.header_line, 0);
        assert_eq!(r.hidden_line_range, (1, 3));
        assert!(r.signature.starts_with("begin:document|"));
        assert!(!r.folded);
    }

    #[test]
    fn single_line_environment_is_not_foldable() {
        let text = "\\begin{x}\n\\end{x}\n";
        assert!(regions_of(text).is_empty());
    }

    #[test]
    fn nested_environments_match_by_name() {
        let text = "\\begin{outer}\n\\begin{inner}\nx\n\\end{inner}\ny\n\\end{outer}\n";
        let regions = regions_of(text);
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].header_line, 0);
        assert_eq!(regions[0].hidden_line_range, (1, 5));
        assert_eq!(regions[1].header_line, 1);
        assert_eq!(regions[1].hidden_line_range, (2, 3));
    }

    #[test]
    fn mismatched_end_uses_innermost_same_name() {
        // `\end{other}` does not close `outer`; `\end{outer}` still pairs.
        let text = "\\begin{outer}\n\\begin{other}\nx\n\\end{outer}\n";
        let regions = regions_of(text);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].header_line, 0);
        assert_eq!(regions[0].hidden_line_range, (1, 3));
    }

    #[test]
    fn sections_close_at_same_or_higher_level() {
        let text = "\\section{A}\na\n\\subsection{B}\nb\n\\section{C}\nc\n";
        let regions = regions_of(text);
        // A spans until C; B spans until C; C runs to EOF (hidden through
        // the line before EOF — text ends with a final newline so the last
        // body line is line 5).
        let a = regions.iter().find(|r| r.header_line == 0).unwrap();
        assert_eq!(a.hidden_line_range, (1, 3));
        let b = regions.iter().find(|r| r.header_line == 2).unwrap();
        assert_eq!(b.hidden_line_range, (3, 3));
        let c = regions.iter().find(|r| r.header_line == 4).unwrap();
        assert_eq!(c.hidden_line_range, (5, 6));
    }

    #[test]
    fn section_signature_carries_header_content() {
        let text = "\\section{Intro}\nx\n\\section{Next}\ny\nz\n";
        let regions = regions_of(text);
        let intro = &regions[0];
        assert_eq!(intro.signature, "section:section|\\section{Intro}");
    }

    #[test]
    fn no_regions_in_plain_text() {
        assert!(regions_of("hello\nworld\n").is_empty());
    }

    #[test]
    fn line_index_binary_search() {
        let text = "a\nbb\nccc\n";
        let starts = compute_line_starts(text);
        // A trailing newline still appends a line start, like the Swift
        // `computeLineStarts` (the empty final line is a real line).
        assert_eq!(starts, vec![(0, 0), (2, 2), (5, 5), (9, 9)]);
        assert_eq!(line_index(&starts, 0), 0);
        assert_eq!(line_index(&starts, 2), 1);
        assert_eq!(line_index(&starts, 4), 1);
        assert_eq!(line_index(&starts, 5), 2);
    }

    #[test]
    fn env_and_section_regions_coexist() {
        let text = "\\section{S}\n\\begin{e}\nx\n\\end{e}\ny\n";
        let regions = regions_of(text);
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].header_line, 0);
        assert_eq!(regions[1].header_line, 1);
        assert_eq!(regions[1].hidden_line_range, (2, 3));
    }
}
