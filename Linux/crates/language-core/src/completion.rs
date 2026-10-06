//! Port of `Packages/TexCore/Sources/LanguageCore/CompletionContext.swift` —
//! caret-context detection for command/citation/reference completion plus
//! the project key-set merge feeding both platforms' completion UIs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{CompletionKind, LanguageCompletion, LanguageIndex};

const BACKSLASH: u8 = b'\\' as u8;
const LBRACE: u8 = b'{' as u8;
const LBRACKET: u8 = b'[' as u8;
const RBRACKET: u8 = b']' as u8;
const COMMA: u8 = b',' as u8;
const STAR: u8 = b'*' as u8;

/// What the caret is completing — drives which candidate list the editor
/// shows. Mirrors `CompletionContextKind` in the Swift `LanguageCore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompletionContextKind {
    /// A `\command` prefix — candidates come from the built-in command list.
    Command,
    /// Inside a `\cite{…}`-style group — candidates are project .bib keys.
    Citation,
    /// Inside a `\ref{…}`-style group — candidates are project \label keys.
    Reference,
}

/// A detected completion context at the caret.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompletionContext {
    pub kind: CompletionContextKind,
    /// Text the chosen candidate replaces: for `Command` it includes the
    /// leading `\` (`\alp`), for group contexts it is the key fragment
    /// after `{` or the last `,` (`kn` in `\cite{a,kn`).
    pub prefix: String,
    /// UTF-16 offset where `prefix` starts — the replacement range is
    /// `prefix_utf16_offset .. caret_utf16_offset`.
    #[serde(rename = "prefixUTF16Offset")]
    pub prefix_utf16_offset: usize,
}

/// Shared caret-context detection — the same scan the Swift
/// `CompletionContextDetector` runs so both platforms trigger identical
/// popups.
pub struct CompletionContextDetector;

impl CompletionContextDetector {
    /// Commands whose `{…}` argument holds citation keys. Broader than the
    /// outline parser's cite set so natbib/biblatex styles complete too.
    pub const CITATION_COMMANDS: &'static [&'static str] = &[
        "cite",
        "citep",
        "citet",
        "citealt",
        "citealp",
        "citeauthor",
        "citeyear",
        "citeyearpar",
        "citepos",
        "nocite",
        "Citep",
        "Citet",
        "Citealt",
        "Citealp",
        "Citeauthor",
        "parencite",
        "Parencite",
        "textcite",
        "Textcite",
        "footcite",
        "autocite",
        "smartcite",
        "supercite",
    ];

    /// Commands whose `{…}` argument holds a \label key.
    pub const REFERENCE_COMMANDS: &'static [&'static str] = &[
        "ref",
        "pageref",
        "autoref",
        "eqref",
        "nameref",
        "cref",
        "Cref",
        "vref",
        "Vref",
        "cpageref",
        "labelcref",
    ];

    /// Detects the completion context at `caret_utf16_offset` in `source`.
    /// Returns None in plain text, inside unrecognised `\cmd{…}` groups, and
    /// after an escaped `\\`. Only looks backwards from the caret, so an
    /// unclosed `\cite{` group still completes.
    pub fn context(source: &str, caret_utf16_offset: usize) -> Option<CompletionContext> {
        let units = source.as_bytes();
        let (mut caret, mut caret_units) = (source.len(), 0);
        for (byte, ch) in source.char_indices() {
            if caret_units == caret_utf16_offset { caret = byte; break; }
            if caret_units > caret_utf16_offset { return None; }
            caret_units += ch.len_utf16();
        }
        if caret_units > caret_utf16_offset { return None; }

        // ── group contexts: `\cite{a,pre|` / `\ref{pre|` ──
        // The current key fragment is a run of key characters ending at the
        // caret; the group opener is the nearest `{` reachable over earlier
        // key characters, commas and whitespace.
        let mut index = caret;
        while index > 0 && is_key_char(units[index - 1]) {
            index -= 1;
        }
        let prefix_start = index;
        let mut scan = index;
        while scan > 0
            && (is_key_char(units[scan - 1])
                || units[scan - 1] == COMMA
                || is_whitespace(units[scan - 1]))
        {
            scan -= 1;
        }
        if scan > 0
            && units[scan - 1] == LBRACE
            && (scan < 2 || units[scan - 2] != BACKSLASH)
        {
            if let Some(command) = Self::command_name(&units, scan - 1) {
                let kind = if Self::CITATION_COMMANDS.contains(&command.as_str()) {
                    Some(CompletionContextKind::Citation)
                } else if Self::REFERENCE_COMMANDS.contains(&command.as_str()) {
                    Some(CompletionContextKind::Reference)
                } else {
                    None
                };
                if let Some(kind) = kind {
                    return Some(CompletionContext {
                        kind,
                        prefix: String::from_utf8_lossy(&units[prefix_start..caret]).into_owned(),
                        prefix_utf16_offset: caret_units - (caret - prefix_start),
                    });
                }
            }
        }

        // ── command context: `\pre|` ──
        index = caret;
        while index > 0 && is_letter(units[index - 1]) {
            index -= 1;
        }
        if index > 0
            && units[index - 1] == BACKSLASH
            && !(index > 1 && units[index - 2] == BACKSLASH)
        {
            return Some(CompletionContext {
                kind: CompletionContextKind::Command,
                prefix: String::from_utf8_lossy(&units[index - 1..caret]).into_owned(),
                prefix_utf16_offset: caret_units - (caret - index + 1),
            });
        }
        None
    }

    /// Command name ending just before `brace` (the `{` index), skipping
    /// trailing whitespace, `*`, and `[…]` optional args. Returns None when
    /// the text before the brace is not a `\name` sequence.
    fn command_name(units: &[u8], brace: usize) -> Option<String> {
        let mut index = brace;
        while index > 0 && is_whitespace(units[index - 1]) {
            index -= 1;
        }
        while index > 0 {
            if units[index - 1] == RBRACKET {
                // `]` — walk back over the balanced `[…]` optional arg.
                let mut depth = 1u32;
                let mut cursor = index - 1;
                while cursor > 0 {
                    cursor -= 1;
                    if units[cursor] == RBRACKET {
                        depth += 1;
                    } else if units[cursor] == LBRACKET {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                if depth != 0 {
                    break;
                }
                index = cursor;
                while index > 0 && is_whitespace(units[index - 1]) {
                    index -= 1;
                }
                continue;
            }
            if units[index - 1] == STAR {
                index -= 1;
                while index > 0 && is_whitespace(units[index - 1]) {
                    index -= 1;
                }
                continue;
            }
            break;
        }
        let name_end = index;
        while index > 0 && is_letter(units[index - 1]) {
            index -= 1;
        }
        if index == name_end
            || index == 0
            || units[index - 1] != BACKSLASH
            || (index > 1 && units[index - 2] == BACKSLASH)
        {
            return None;
        }
        Some(String::from_utf8_lossy(&units[index..name_end]).into_owned())
    }
}

/// Letters, digits and the punctuation a BibTeX/label key may contain.
fn is_key_char(unit: u8) -> bool {
    is_letter(unit)
        || (48..=57).contains(&unit)
        || unit == b':' as u8
        || unit == b'_' as u8
        || unit == b'-' as u8
        || unit == b'.' as u8
        || unit == b'/' as u8
}

fn is_letter(unit: u8) -> bool {
    (65..=90).contains(&unit) || (97..=122).contains(&unit)
}

fn is_whitespace(unit: u8) -> bool {
    unit == b' ' as u8 || unit == b'\t' as u8 || unit == b'\n' as u8 || unit == b'\r' as u8
}

impl LanguageIndex {
    /// Project-wide candidates for a detected context: command contexts
    /// filter the built-in command list; citation/reference contexts filter
    /// the supplied project key sets (every project .bib key / \label).
    /// Deterministically ordered like `ProjectLanguageIndex::completions`.
    pub fn completions(
        context: &CompletionContext,
        labels: &BTreeSet<String>,
        citation_keys: &BTreeSet<String>,
    ) -> Vec<LanguageCompletion> {
        match context.kind {
            CompletionContextKind::Command => Self::command_completions(&context.prefix),
            CompletionContextKind::Citation => citation_keys
                .iter()
                .filter(|key| key.starts_with(&context.prefix))
                .map(|key| LanguageCompletion {
                    text: key.clone(),
                    kind: CompletionKind::Citation,
                })
                .collect(),
            CompletionContextKind::Reference => labels
                .iter()
                .filter(|key| key.starts_with(&context.prefix))
                .map(|key| LanguageCompletion {
                    text: key.clone(),
                    kind: CompletionKind::Label,
                })
                .collect(),
        }
    }
}
