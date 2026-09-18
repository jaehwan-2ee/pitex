//! Port of `Packages/TexCore/Sources/LanguageCore` — deterministic TeX lexer,
//! cross-file language index, outline/reference parsing, and the
//! UTF-8/UTF-16/line-column coordinate map.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageCoreError {
    EmptySourceIdentifier,
    InvalidRange,
    StaleResult {
        expected: SnapshotRevision,
        actual: SnapshotRevision,
    },
}

impl fmt::Display for LanguageCoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LanguageCoreError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceRange {
    #[serde(rename = "utf8Offset")]
    pub utf8_offset: i64,
    #[serde(rename = "utf8Length")]
    pub utf8_length: i64,
}

impl SourceRange {
    pub fn new(utf8_offset: i64, utf8_length: i64) -> Result<Self, LanguageCoreError> {
        if utf8_offset < 0 || utf8_length < 0 {
            return Err(LanguageCoreError::InvalidRange);
        }
        Ok(Self {
            utf8_offset,
            utf8_length,
        })
    }

    fn lexer_range(offset: usize, length: usize) -> Self {
        Self {
            utf8_offset: offset as i64,
            utf8_length: length as i64,
        }
    }

    pub fn end_utf8_offset(&self) -> i64 {
        self.utf8_offset + self.utf8_length
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguageTokenKind {
    ControlSequence(String),
    Comment(String),
    LeftBrace,
    RightBrace,
    Whitespace(String),
    Text(String),
    BibEntryMarker,
    Punctuation(String),
    /// The environment name inside \begin{…} / \end{…} (e.g. `document`).
    EnvironmentName(String),
    /// A complete math region: $…$, $$…$$, \(…\), or \[…\], delimiters included.
    Math(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LanguageToken {
    pub kind: LanguageTokenKind,
    pub range: SourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeXDialect {
    Latex,
    Bibtex,
}

pub struct DeterministicTeXLexer;

impl DeterministicTeXLexer {
    pub fn tokenize(source: &str, dialect: TeXDialect) -> Vec<LanguageToken> {
        let scalars: Vec<char> = source.chars().collect();
        let mut scalar_offsets: Vec<usize> = Vec::with_capacity(scalars.len() + 1);
        let mut offset = 0usize;
        for scalar in &scalars {
            scalar_offsets.push(offset);
            offset += scalar.len_utf8();
        }
        scalar_offsets.push(offset);

        let token = |kind: LanguageTokenKind, start: usize, end: usize| -> LanguageToken {
            LanguageToken {
                kind,
                range: SourceRange::lexer_range(
                    scalar_offsets[start],
                    scalar_offsets[end] - scalar_offsets[start],
                ),
            }
        };
        let string = |start: usize, end: usize| -> String {
            scalars[start..end].iter().collect()
        };
        let is_letter = |scalar: char| scalar.is_ascii_uppercase() || scalar.is_ascii_lowercase();
        let is_whitespace = |scalar: char| scalar == ' ' || scalar == '\t' || scalar == '\r' || scalar == '\n';

        let mut result: Vec<LanguageToken> = Vec::new();
        let mut index = 0usize;
        while index < scalars.len() {
            let start = index;
            let scalar = scalars[index];
            if scalar == '%' {
                index += 1;
                while index < scalars.len() && scalars[index] != '\n' && scalars[index] != '\r' {
                    index += 1;
                }
                result.push(token(LanguageTokenKind::Comment(string(start, index)), start, index));
            } else if scalar == '\\' {
                index += 1;
                if index < scalars.len() {
                    if is_letter(scalars[index]) {
                        while index < scalars.len() && is_letter(scalars[index]) {
                            index += 1;
                        }
                    } else {
                        index += 1;
                    }
                }
                let name = string(start + 1, index);
                if name == "(" || name == "[" {
                    // Display math \( … \) / \[ … \]: scan for the matching
                    // closer, skipping escaped characters.
                    let closer: char = if name == "(" { ')' } else { ']' };
                    let mut scan = index;
                    let mut closed = false;
                    while scan < scalars.len() {
                        if scalars[scan] == '\\' && scan + 1 < scalars.len() {
                            if scalars[scan + 1] == closer {
                                closed = true;
                                scan += 2;
                                break;
                            }
                            scan += 2;
                            continue;
                        }
                        scan += 1;
                    }
                    if closed {
                        result.push(token(LanguageTokenKind::Math(string(start, scan)), start, scan));
                        index = scan;
                    } else {
                        result.push(token(LanguageTokenKind::ControlSequence(name), start, index));
                    }
                } else {
                    result.push(token(LanguageTokenKind::ControlSequence(name.clone()), start, index));
                    if dialect == TeXDialect::Latex && (name == "begin" || name == "end") {
                        // \begin{env} / \end{env}: the name inside the braces
                        // gets its own token so the highlighter can paint it
                        // with the environment color like the reference editor.
                        let mut probe = index;
                        while probe < scalars.len() && (scalars[probe] == ' ' || scalars[probe] == '\t') {
                            probe += 1;
                        }
                        if probe < scalars.len() && scalars[probe] == '{' {
                            if probe > index {
                                result.push(token(
                                    LanguageTokenKind::Whitespace(string(index, probe)),
                                    index,
                                    probe,
                                ));
                            }
                            result.push(token(LanguageTokenKind::LeftBrace, probe, probe + 1));
                            let mut cursor = probe + 1;
                            while cursor < scalars.len()
                                && scalars[cursor] != '}'
                                && scalars[cursor] != '\n'
                                && scalars[cursor] != '\r'
                            {
                                cursor += 1;
                            }
                            if cursor > probe + 1 {
                                result.push(token(
                                    LanguageTokenKind::EnvironmentName(string(probe + 1, cursor)),
                                    probe + 1,
                                    cursor,
                                ));
                            }
                            // The closing brace is emitted by the main loop.
                            index = cursor;
                        }
                    }
                }
            } else if scalar == '$' {
                // Inline/display math $…$ / $$…$$. An unmatched opening
                // delimiter emits only the delimiter itself as math so the
                // rest of the file keeps its normal coloring.
                let is_double = index + 1 < scalars.len() && scalars[index + 1] == '$';
                index += if is_double { 2 } else { 1 };
                let mut scan = index;
                let mut end: Option<usize> = None;
                while scan < scalars.len() {
                    if scalars[scan] == '\\' {
                        scan += 2;
                        continue;
                    }
                    if scalars[scan] == '$' {
                        if is_double {
                            if scan + 1 < scalars.len() && scalars[scan + 1] == '$' {
                                end = Some(scan + 2);
                                break;
                            }
                            scan += 1;
                            continue;
                        }
                        end = Some(scan + 1);
                        break;
                    }
                    scan += 1;
                }
                if let Some(end) = end {
                    result.push(token(LanguageTokenKind::Math(string(start, end)), start, end));
                    index = end;
                } else {
                    result.push(token(LanguageTokenKind::Math(string(start, index)), start, index));
                }
            } else if scalar == '{' || scalar == '[' || (dialect == TeXDialect::Latex && scalar == '(') {
                index += 1;
                result.push(token(LanguageTokenKind::LeftBrace, start, index));
            } else if scalar == '}' || scalar == ']' || (dialect == TeXDialect::Latex && scalar == ')') {
                index += 1;
                result.push(token(LanguageTokenKind::RightBrace, start, index));
            } else if is_whitespace(scalar) {
                index += 1;
                while index < scalars.len() && is_whitespace(scalars[index]) {
                    index += 1;
                }
                result.push(token(LanguageTokenKind::Whitespace(string(start, index)), start, index));
            } else if dialect == TeXDialect::Bibtex && scalar == '@' {
                index += 1;
                result.push(token(LanguageTokenKind::BibEntryMarker, start, index));
            } else if dialect == TeXDialect::Bibtex
                && [',', '=', '(', ')', '#', '"'].contains(&scalar)
            {
                index += 1;
                result.push(token(LanguageTokenKind::Punctuation(scalar.to_string()), start, index));
            } else {
                index += 1;
                while index < scalars.len() {
                    let next = scalars[index];
                    if next == '%'
                        || next == '\\'
                        || next == '{'
                        || next == '}'
                        || next == '['
                        || next == ']'
                        || next == '$'
                        || is_whitespace(next)
                    {
                        break;
                    }
                    if dialect == TeXDialect::Bibtex
                        && (next == '@' || [',', '=', '(', ')', '#', '"'].contains(&next))
                    {
                        break;
                    }
                    if dialect == TeXDialect::Latex && (next == '(' || next == ')') {
                        break;
                    }
                    index += 1;
                }
                result.push(token(LanguageTokenKind::Text(string(start, index)), start, index));
            }
        }
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutlineKind {
    Part,
    Chapter,
    Section,
    Subsection,
    Subsubsection,
    Bibliography,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutlineRecord {
    pub kind: OutlineKind,
    pub title: String,
    pub range: SourceRange,
    pub depth: i64,
}

impl OutlineRecord {
    pub fn new(
        kind: OutlineKind,
        title: String,
        range: SourceRange,
        depth: i64,
    ) -> Result<Self, LanguageCoreError> {
        if title.is_empty() || depth < 0 {
            return Err(LanguageCoreError::InvalidRange);
        }
        Ok(Self {
            kind,
            title,
            range,
            depth,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReferenceKind {
    Label,
    Reference,
    Citation,
    BibliographyEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReferenceRecord {
    pub kind: ReferenceKind,
    pub key: String,
    pub range: SourceRange,
}

impl ReferenceRecord {
    pub fn new(kind: ReferenceKind, key: String, range: SourceRange) -> Result<Self, LanguageCoreError> {
        if key.is_empty() {
            return Err(LanguageCoreError::EmptySourceIdentifier);
        }
        Ok(Self { kind, key, range })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotRevision {
    #[serde(rename = "sourceID")]
    pub source_id: String,
    pub revision: u64,
    #[serde(rename = "contentFingerprint")]
    pub content_fingerprint: u64,
}

impl SnapshotRevision {
    pub fn new(source_id: String, revision: u64, source: &str) -> Result<Self, LanguageCoreError> {
        if source_id.is_empty() {
            return Err(LanguageCoreError::EmptySourceIdentifier);
        }
        Ok(Self {
            source_id,
            revision,
            content_fingerprint: tex_domain::fnv1a64(source.as_bytes()),
        })
    }
}

impl PartialOrd for SnapshotRevision {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SnapshotRevision {
    fn cmp(&self, other: &Self) -> Ordering {
        (
            self.source_id.as_str(),
            self.revision,
            self.content_fingerprint,
        )
            .cmp(&(
                other.source_id.as_str(),
                other.revision,
                other.content_fingerprint,
            ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RevisionBound<T> {
    pub revision: SnapshotRevision,
    pub value: T,
}

impl<T: Clone> RevisionBound<T> {
    pub fn new(revision: SnapshotRevision, value: T) -> Self {
        Self { revision, value }
    }

    pub fn value_for(&self, current: &SnapshotRevision) -> Result<T, LanguageCoreError> {
        if &self.revision != current {
            return Err(LanguageCoreError::StaleResult {
                expected: current.clone(),
                actual: self.revision.clone(),
            });
        }
        Ok(self.value.clone())
    }
}

// ---------------------------------------------------------------------------
// Coordinate map

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinateMapError {
    InvalidUTF8Offset(i64),
    InvalidUTF16Offset(i64),
    InvalidPosition(TextPosition),
}

impl fmt::Display for CoordinateMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for CoordinateMapError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TextPosition {
    pub line: i64,
    pub column: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Boundary {
    utf8: i64,
    utf16: i64,
    position: TextPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnicodeCoordinateMap {
    pub utf8_count: i64,
    pub utf16_count: i64,
    boundaries: Vec<Boundary>,
}

impl UnicodeCoordinateMap {
    pub fn new(source: &str) -> Self {
        let mut result: Vec<Boundary> = Vec::new();
        let mut utf8 = 0i64;
        let mut utf16 = 0i64;
        let mut line = 0i64;
        let mut column = 0i64;
        result.push(Boundary {
            utf8: 0,
            utf16: 0,
            position: TextPosition { line: 0, column: 0 },
        });

        for grapheme in source.graphemes(true) {
            utf8 += grapheme.len() as i64;
            utf16 += grapheme.chars().map(|c| c.len_utf16() as i64).sum::<i64>();
            if grapheme == "\r\n" {
                line += 1;
                column = 0;
            } else {
                for scalar in grapheme.chars() {
                    if scalar == '\n' || scalar == '\r' {
                        line += 1;
                        column = 0;
                    } else {
                        column += scalar.len_utf16() as i64;
                    }
                }
            }
            result.push(Boundary {
                utf8,
                utf16,
                position: TextPosition { line, column },
            });
        }
        Self {
            utf8_count: utf8,
            utf16_count: utf16,
            boundaries: result,
        }
    }

    pub fn utf16_offset_for_utf8(&self, offset: i64) -> Result<i64, CoordinateMapError> {
        self.boundaries
            .iter()
            .find(|b| b.utf8 == offset)
            .map(|b| b.utf16)
            .ok_or(CoordinateMapError::InvalidUTF8Offset(offset))
    }

    pub fn utf8_offset_for_utf16(&self, offset: i64) -> Result<i64, CoordinateMapError> {
        self.boundaries
            .iter()
            .find(|b| b.utf16 == offset)
            .map(|b| b.utf8)
            .ok_or(CoordinateMapError::InvalidUTF16Offset(offset))
    }

    pub fn position_for_utf8(&self, offset: i64) -> Result<TextPosition, CoordinateMapError> {
        self.boundaries
            .iter()
            .find(|b| b.utf8 == offset)
            .map(|b| b.position)
            .ok_or(CoordinateMapError::InvalidUTF8Offset(offset))
    }

    pub fn position_for_utf16(&self, offset: i64) -> Result<TextPosition, CoordinateMapError> {
        self.boundaries
            .iter()
            .find(|b| b.utf16 == offset)
            .map(|b| b.position)
            .ok_or(CoordinateMapError::InvalidUTF16Offset(offset))
    }

    pub fn utf8_offset_for_position(&self, position: TextPosition) -> Result<i64, CoordinateMapError> {
        if position.line < 0 || position.column < 0 {
            return Err(CoordinateMapError::InvalidPosition(position));
        }
        self.boundaries
            .iter()
            .find(|b| b.position == position)
            .map(|b| b.utf8)
            .ok_or(CoordinateMapError::InvalidPosition(position))
    }

    pub fn utf16_offset_for_position(&self, position: TextPosition) -> Result<i64, CoordinateMapError> {
        if position.line < 0 || position.column < 0 {
            return Err(CoordinateMapError::InvalidPosition(position));
        }
        self.boundaries
            .iter()
            .find(|b| b.position == position)
            .map(|b| b.utf16)
            .ok_or(CoordinateMapError::InvalidPosition(position))
    }
}

// ---------------------------------------------------------------------------
// Language index records

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    #[serde(rename = "sourceID")]
    pub source_id: String,
    pub range: SourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IncludeRecord {
    pub target: String,
    pub range: SourceRange,
}

impl IncludeRecord {
    pub fn new(target: String, range: SourceRange) -> Result<Self, LanguageCoreError> {
        if target.is_empty() {
            return Err(LanguageCoreError::EmptySourceIdentifier);
        }
        Ok(Self { target, range })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BibTeXEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub key: String,
    pub range: SourceRange,
}

impl BibTeXEntry {
    pub fn new(entry_type: String, key: String, range: SourceRange) -> Result<Self, LanguageCoreError> {
        if entry_type.is_empty() || key.is_empty() {
            return Err(LanguageCoreError::EmptySourceIdentifier);
        }
        Ok(Self {
            entry_type,
            key,
            range,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguageDiagnostic {
    MalformedGroup {
        #[serde(rename = "sourceID")]
        source_id: String,
        command: String,
        range: SourceRange,
    },
    DuplicateLabel {
        key: String,
        definitions: Vec<SourceLocation>,
    },
    DuplicateCitation {
        key: String,
        definitions: Vec<SourceLocation>,
    },
    MissingLabel {
        key: String,
        #[serde(rename = "use")]
        use_location: SourceLocation,
    },
    MissingCitation {
        key: String,
        #[serde(rename = "use")]
        use_location: SourceLocation,
    },
    IncludeCycle(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MinimapRangeKind {
    Outline,
    Include,
    Definition,
    Use,
    Bibliography,
}

impl MinimapRangeKind {
    fn raw_value(self) -> &'static str {
        match self {
            Self::Outline => "outline",
            Self::Include => "include",
            Self::Definition => "definition",
            Self::Use => "use",
            Self::Bibliography => "bibliography",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MinimapRange {
    pub kind: MinimapRangeKind,
    pub range: SourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompletionKind {
    Command,
    Label,
    Citation,
}

impl CompletionKind {
    fn raw_value(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Label => "label",
            Self::Citation => "citation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LanguageCompletion {
    pub text: String,
    pub kind: CompletionKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageFileSnapshot {
    pub revision: SnapshotRevision,
    pub source: String,
    pub dialect: TeXDialect,
    pub tokens: Vec<LanguageToken>,
    pub outline: Vec<OutlineRecord>,
    pub includes: Vec<IncludeRecord>,
    pub references: Vec<ReferenceRecord>,
    pub bibliography: Vec<BibTeXEntry>,
    #[serde(rename = "minimapRanges")]
    pub minimap_ranges: Vec<MinimapRange>,
    pub diagnostics: Vec<LanguageDiagnostic>,
}

impl LanguageFileSnapshot {
    pub fn new(
        source_id: &str,
        revision: u64,
        source: &str,
        dialect: TeXDialect,
    ) -> Result<Self, LanguageCoreError> {
        let revision = SnapshotRevision::new(source_id.to_string(), revision, source)?;
        let tokens = DeterministicTeXLexer::tokenize(source, dialect);
        let parsed = language_parser::parse(source_id, source, dialect)?;
        let mut minimap_ranges = parsed.minimap;
        minimap_ranges.sort_by(|a, b| {
            (a.range.utf8_offset, a.range.utf8_length, a.kind.raw_value()).cmp(&(
                b.range.utf8_offset,
                b.range.utf8_length,
                b.kind.raw_value(),
            ))
        });
        Ok(Self {
            revision,
            source: source.to_string(),
            dialect,
            tokens,
            outline: parsed.outline,
            includes: parsed.includes,
            references: parsed.references,
            bibliography: parsed.bibliography,
            minimap_ranges,
            diagnostics: parsed.diagnostics,
        })
    }

    pub fn source_id(&self) -> &str {
        &self.revision.source_id
    }

    pub fn coordinates(&self) -> UnicodeCoordinateMap {
        UnicodeCoordinateMap::new(&self.source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectOutlineRecord {
    #[serde(rename = "sourceID")]
    pub source_id: String,
    pub record: OutlineRecord,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectLanguageIndex {
    pub snapshots: BTreeMap<String, LanguageFileSnapshot>,
}

impl ProjectLanguageIndex {
    pub fn new(snapshots: Vec<LanguageFileSnapshot>) -> Result<Self, LanguageCoreError> {
        let mut result: BTreeMap<String, LanguageFileSnapshot> = BTreeMap::new();
        for snapshot in snapshots {
            if let Some(current) = result.get(snapshot.source_id()) {
                if current.revision != snapshot.revision {
                    return Err(LanguageCoreError::StaleResult {
                        expected: current.revision.clone(),
                        actual: snapshot.revision.clone(),
                    });
                }
            }
            result.insert(snapshot.source_id().to_string(), snapshot);
        }
        Ok(Self { snapshots: result })
    }

    fn validated(snapshots: BTreeMap<String, LanguageFileSnapshot>) -> Self {
        Self { snapshots }
    }

    pub fn replacing(&self, snapshot: LanguageFileSnapshot) -> Result<Self, LanguageCoreError> {
        if let Some(current) = self.snapshots.get(snapshot.source_id()) {
            if current.revision == snapshot.revision {
                return Ok(self.clone());
            }
            if snapshot.revision.revision <= current.revision.revision {
                return Err(LanguageCoreError::StaleResult {
                    expected: current.revision.clone(),
                    actual: snapshot.revision.clone(),
                });
            }
        }
        let mut result = self.snapshots.clone();
        result.insert(snapshot.source_id().to_string(), snapshot);
        Ok(Self::validated(result))
    }

    pub fn snapshot(
        &self,
        source_id: &str,
        revision: &SnapshotRevision,
    ) -> Result<&LanguageFileSnapshot, LanguageCoreError> {
        let Some(snapshot) = self.snapshots.get(source_id) else {
            return Err(LanguageCoreError::StaleResult {
                expected: revision.clone(),
                actual: revision.clone(),
            });
        };
        if &snapshot.revision != revision {
            return Err(LanguageCoreError::StaleResult {
                expected: snapshot.revision.clone(),
                actual: revision.clone(),
            });
        }
        Ok(snapshot)
    }

    pub fn references(&self) -> Vec<(String, ReferenceRecord)> {
        let mut out = Vec::new();
        for (source_id, snapshot) in &self.snapshots {
            for record in &snapshot.references {
                out.push((source_id.clone(), record.clone()));
            }
        }
        out
    }

    pub fn bibliography(&self) -> Vec<(String, BibTeXEntry)> {
        let mut out = Vec::new();
        for (source_id, snapshot) in &self.snapshots {
            for entry in &snapshot.bibliography {
                out.push((source_id.clone(), entry.clone()));
            }
        }
        out
    }

    pub fn diagnostics(&self) -> Vec<LanguageDiagnostic> {
        let mut result: Vec<LanguageDiagnostic> = self
            .snapshots
            .values()
            .flat_map(|s| s.diagnostics.clone())
            .collect();
        let mut labels: BTreeMap<String, Vec<SourceLocation>> = BTreeMap::new();
        let mut citations: BTreeMap<String, Vec<SourceLocation>> = BTreeMap::new();
        let mut label_uses: Vec<(String, SourceLocation)> = Vec::new();
        let mut citation_uses: Vec<(String, SourceLocation)> = Vec::new();

        for (source_id, snapshot) in &self.snapshots {
            for record in &snapshot.references {
                let location = SourceLocation {
                    source_id: source_id.clone(),
                    range: record.range,
                };
                match record.kind {
                    ReferenceKind::Label => labels
                        .entry(record.key.clone())
                        .or_default()
                        .push(location),
                    ReferenceKind::Reference => label_uses.push((record.key.clone(), location)),
                    ReferenceKind::Citation => {
                        citation_uses.push((record.key.clone(), location))
                    }
                    ReferenceKind::BibliographyEntry => citations
                        .entry(record.key.clone())
                        .or_default()
                        .push(location),
                }
            }
            for entry in &snapshot.bibliography {
                citations.entry(entry.key.clone()).or_default().push(SourceLocation {
                    source_id: source_id.clone(),
                    range: entry.range,
                });
            }
        }
        for (key, definitions) in &labels {
            if definitions.len() > 1 {
                result.push(LanguageDiagnostic::DuplicateLabel {
                    key: key.clone(),
                    definitions: definitions.clone(),
                });
            }
        }
        for (key, definitions) in &citations {
            if definitions.len() > 1 {
                result.push(LanguageDiagnostic::DuplicateCitation {
                    key: key.clone(),
                    definitions: definitions.clone(),
                });
            }
        }
        let mut sorted_label_uses = label_uses;
        sorted_label_uses.sort_by(|a, b| language_parser::key_location_order(a, b));
        for (key, location) in sorted_label_uses {
            if !labels.contains_key(&key) {
                result.push(LanguageDiagnostic::MissingLabel {
                    key,
                    use_location: location,
                });
            }
        }
        let mut sorted_citation_uses = citation_uses;
        sorted_citation_uses.sort_by(|a, b| language_parser::key_location_order(a, b));
        for (key, location) in sorted_citation_uses {
            if !citations.contains_key(&key) {
                result.push(LanguageDiagnostic::MissingCitation {
                    key,
                    use_location: location,
                });
            }
        }
        for cycle in self.include_cycles() {
            result.push(LanguageDiagnostic::IncludeCycle(cycle));
        }
        result
    }

    pub fn completions(&self, prefix: &str) -> Vec<LanguageCompletion> {
        if prefix.starts_with('\\') {
            return LanguageIndex::command_completions(prefix);
        }
        let mut values: Vec<LanguageCompletion> = Vec::new();
        let normalized = prefix;
        let labels: HashSet<String> = self
            .references()
            .into_iter()
            .filter(|(_, r)| r.kind == ReferenceKind::Label)
            .map(|(_, r)| r.key)
            .collect();
        let citations: HashSet<String> = self
            .bibliography()
            .into_iter()
            .map(|(_, e)| e.key)
            .collect();
        for label in labels {
            if label.starts_with(normalized) {
                values.push(LanguageCompletion {
                    text: label,
                    kind: CompletionKind::Label,
                });
            }
        }
        for citation in citations {
            if citation.starts_with(normalized) {
                values.push(LanguageCompletion {
                    text: citation,
                    kind: CompletionKind::Citation,
                });
            }
        }
        values.sort_by(|a, b| (a.text.as_str(), a.kind.raw_value()).cmp(&(b.text.as_str(), b.kind.raw_value())));
        values
    }

    pub fn recursive_outline(&self, source_id: &str) -> Vec<ProjectOutlineRecord> {
        enum Event {
            Outline(OutlineRecord),
            Include(IncludeRecord),
        }
        fn event_offset(e: &Event) -> i64 {
            match e {
                Event::Outline(v) => v.range.utf8_offset,
                Event::Include(v) => v.range.utf8_offset,
            }
        }

        let mut result: Vec<ProjectOutlineRecord> = Vec::new();
        let mut active: HashSet<String> = HashSet::new();
        fn visit(
            id: &str,
            index: &ProjectLanguageIndex,
            active: &mut HashSet<String>,
            result: &mut Vec<ProjectOutlineRecord>,
        ) {
            let Some(snapshot) = index.snapshots.get(id) else {
                return;
            };
            if !active.insert(id.to_string()) {
                return;
            }
            let mut events: Vec<Event> = snapshot
                .outline
                .iter()
                .cloned()
                .map(Event::Outline)
                .chain(snapshot.includes.iter().cloned().map(Event::Include))
                .collect();
            events.sort_by_key(event_offset);
            for event in events {
                match event {
                    Event::Outline(record) => result.push(ProjectOutlineRecord {
                        source_id: id.to_string(),
                        record,
                    }),
                    Event::Include(include) => {
                        visit(
                            &index.resolve(&include.target, id),
                            index,
                            active,
                            result,
                        );
                    }
                }
            }
            active.remove(id);
        }
        visit(source_id, self, &mut active, &mut result);
        result
    }

    fn include_cycles(&self) -> Vec<Vec<String>> {
        fn canonical(cycle: &[String]) -> Vec<String> {
            let body = &cycle[..cycle.len() - 1];
            if body.is_empty() {
                return cycle.to_vec();
            }
            let mut best: Option<Vec<String>> = None;
            for index in 0..body.len() {
                let mut rotation: Vec<String> = body[index..].to_vec();
                rotation.extend_from_slice(&body[..index]);
                if best.as_ref().map_or(true, |b| rotation < *b) {
                    best = Some(rotation);
                }
            }
            let mut best = best.unwrap();
            best.push(best[0].clone());
            best
        }

        let mut cycles: BTreeSet<Vec<String>> = BTreeSet::new();
        let mut path: Vec<String> = Vec::new();
        let mut active: HashSet<String> = HashSet::new();

        fn visit(
            id: &str,
            index: &ProjectLanguageIndex,
            path: &mut Vec<String>,
            active: &mut HashSet<String>,
            cycles: &mut BTreeSet<Vec<String>>,
            canonical: &dyn Fn(&[String]) -> Vec<String>,
        ) {
            let Some(snapshot) = index.snapshots.get(id) else {
                return;
            };
            if let Some(pos) = path.iter().position(|p| p == id) {
                let mut cycle: Vec<String> = path[pos..].to_vec();
                cycle.push(id.to_string());
                cycles.insert(canonical(&cycle));
                return;
            }
            if !active.insert(id.to_string()) {
                return;
            }
            path.push(id.to_string());
            let mut includes = snapshot.includes.clone();
            includes.sort_by(|a, b| {
                (a.range.utf8_offset, a.target.as_str()).cmp(&(b.range.utf8_offset, b.target.as_str()))
            });
            for include in includes {
                visit(
                    &index.resolve(&include.target, id),
                    index,
                    path,
                    active,
                    cycles,
                    canonical,
                );
            }
            path.pop();
            active.remove(id);
        }

        let ids: Vec<String> = self.snapshots.keys().cloned().collect();
        for id in ids {
            visit(&id, self, &mut path, &mut active, &mut cycles, &canonical);
        }
        cycles.into_iter().collect()
    }

    fn resolve(&self, target: &str, source_id: &str) -> String {
        let mut resolved_target = target.to_string();
        if !resolved_target.ends_with(".tex") {
            resolved_target.push_str(".tex");
        }
        if resolved_target.starts_with('/') {
            return resolved_target;
        }
        let directory: String = match source_id.rfind('/') {
            Some(slash) => source_id[..=slash].to_string(),
            None => String::new(),
        };
        let joined = format!("{directory}{resolved_target}");
        let mut components: Vec<&str> = Vec::new();
        for component in joined.split('/') {
            if component.is_empty() || component == "." {
                continue;
            }
            if component == ".." {
                components.pop();
            } else {
                components.push(component);
            }
        }
        components.join("/")
    }
}

pub struct LanguageIndex;

impl LanguageIndex {
    const COMMANDS: &'static [&'static str] = &[
        "begin",
        "bibliography",
        "cite",
        "documentclass",
        "emph",
        "end",
        "include",
        "input",
        "item",
        "label",
        "paragraph",
        "ref",
        "section",
        "subsection",
        "subsubsection",
        "textbf",
        "textit",
    ];

    pub fn command_completions(prefix: &str) -> Vec<LanguageCompletion> {
        let normalized = prefix.strip_prefix('\\').unwrap_or(prefix);
        let mut matches: Vec<&str> = Self::COMMANDS
            .iter()
            .copied()
            .filter(|c| c.starts_with(normalized))
            .collect();
        matches.sort();
        matches
            .into_iter()
            .map(|c| LanguageCompletion {
                text: format!("\\{c}"),
                kind: CompletionKind::Command,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Parser (LaTeX + BibTeX), byte-level identical to the Swift implementation.

mod language_parser {
    use super::*;

    pub struct Parsed {
        pub outline: Vec<OutlineRecord>,
        pub includes: Vec<IncludeRecord>,
        pub references: Vec<ReferenceRecord>,
        pub bibliography: Vec<BibTeXEntry>,
        pub minimap: Vec<MinimapRange>,
        pub diagnostics: Vec<LanguageDiagnostic>,
    }

    pub fn key_location_order(
        lhs: &(String, SourceLocation),
        rhs: &(String, SourceLocation),
    ) -> Ordering {
        (
            lhs.0.as_str(),
            lhs.1.source_id.as_str(),
            lhs.1.range.utf8_offset,
        )
            .cmp(&(
                rhs.0.as_str(),
                rhs.1.source_id.as_str(),
                rhs.1.range.utf8_offset,
            ))
    }

    pub fn parse(source_id: &str, source: &str, dialect: TeXDialect) -> Result<Parsed, LanguageCoreError> {
        match dialect {
            TeXDialect::Bibtex => parse_bibtex(source_id, source),
            TeXDialect::Latex => parse_latex(source_id, source),
        }
    }

    fn parse_latex(source_id: &str, source: &str) -> Result<Parsed, LanguageCoreError> {
        let bytes = source.as_bytes();
        let mut result = Parsed {
            outline: Vec::new(),
            includes: Vec::new(),
            references: Vec::new(),
            bibliography: Vec::new(),
            minimap: Vec::new(),
            diagnostics: Vec::new(),
        };
        let mut index = 0usize;
        let verbatim_names: HashSet<&str> = ["verbatim", "verbatim*", "Verbatim", "lstlisting"]
            .into_iter()
            .collect();
        while index < bytes.len() {
            if bytes[index] == b'%' {
                while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
                    index += 1;
                }
                continue;
            }
            if bytes[index] != b'\\' {
                index += 1;
                continue;
            }
            let command_start = index;
            index += 1;
            if index >= bytes.len() {
                continue;
            }
            let name_start = index;
            if is_letter(bytes[index]) {
                while index < bytes.len() && is_letter(bytes[index]) {
                    index += 1;
                }
            } else {
                index += 1;
            }
            let command = text(bytes, name_start, index);
            if command == "verb" {
                if index < bytes.len() {
                    let delimiter = bytes[index];
                    index += 1;
                    while index < bytes.len() && bytes[index] != delimiter && bytes[index] != b'\n' && bytes[index] != b'\r' {
                        index += 1;
                    }
                    if index < bytes.len() && bytes[index] == delimiter {
                        index += 1;
                    }
                }
                continue;
            }
            const COMMANDS: &[&str] = &[
                "section", "subsection", "subsubsection", "include", "input", "label", "ref",
                "pageref", "cite", "citep", "citet", "begin",
            ];
            if !COMMANDS.contains(&command.as_str()) {
                continue;
            }
            if index < bytes.len() && bytes[index] == b'*' {
                index += 1;
            }
            let Some(group) = group_after(index, bytes) else {
                let range = SourceRange::new(command_start as i64, (index - command_start) as i64)?;
                result.diagnostics.push(LanguageDiagnostic::MalformedGroup {
                    source_id: source_id.to_string(),
                    command,
                    range,
                });
                continue;
            };
            let command_range =
                SourceRange::new(command_start as i64, (group.2 - command_start) as i64)?;
            let value = trimmed(&group_text(bytes, group.0, group.1)).to_string();
            index = group.2;
            if value.is_empty() {
                result.diagnostics.push(LanguageDiagnostic::MalformedGroup {
                    source_id: source_id.to_string(),
                    command,
                    range: command_range,
                });
                continue;
            }
            match command.as_str() {
                "section" | "subsection" | "subsubsection" => {
                    let (kind, depth) = match command.as_str() {
                        "section" => (OutlineKind::Section, 0),
                        "subsection" => (OutlineKind::Subsection, 1),
                        _ => (OutlineKind::Subsubsection, 2),
                    };
                    result.outline.push(OutlineRecord::new(
                        kind,
                        value,
                        command_range,
                        depth,
                    )?);
                    result.minimap.push(MinimapRange {
                        kind: MinimapRangeKind::Outline,
                        range: command_range,
                    });
                }
                "include" | "input" => {
                    result.includes.push(IncludeRecord::new(value, command_range)?);
                    result.minimap.push(MinimapRange {
                        kind: MinimapRangeKind::Include,
                        range: command_range,
                    });
                }
                "label" => {
                    result.references.push(ReferenceRecord::new(
                        ReferenceKind::Label,
                        value,
                        command_range,
                    )?);
                    result.minimap.push(MinimapRange {
                        kind: MinimapRangeKind::Definition,
                        range: command_range,
                    });
                }
                "ref" | "pageref" => {
                    result.references.push(ReferenceRecord::new(
                        ReferenceKind::Reference,
                        value,
                        command_range,
                    )?);
                    result.minimap.push(MinimapRange {
                        kind: MinimapRangeKind::Use,
                        range: command_range,
                    });
                }
                "cite" | "citep" | "citet" => {
                    for key in value
                        .split(',')
                        .map(trimmed)
                        .filter(|k| !k.is_empty())
                    {
                        result.references.push(ReferenceRecord::new(
                            ReferenceKind::Citation,
                            key.to_string(),
                            command_range,
                        )?);
                    }
                    result.minimap.push(MinimapRange {
                        kind: MinimapRangeKind::Use,
                        range: command_range,
                    });
                }
                "begin" if verbatim_names.contains(value.as_str()) => {
                    let marker = format!("\\end{{{value}}}").into_bytes();
                    if let Some(end) = find(&marker, bytes, index) {
                        index = end + marker.len();
                    }
                }
                _ => {}
            }
        }
        Ok(result)
    }

    fn parse_bibtex(source_id: &str, source: &str) -> Result<Parsed, LanguageCoreError> {
        let bytes = source.as_bytes();
        let mut result = Parsed {
            outline: Vec::new(),
            includes: Vec::new(),
            references: Vec::new(),
            bibliography: Vec::new(),
            minimap: Vec::new(),
            diagnostics: Vec::new(),
        };
        let mut index = 0usize;
        while index < bytes.len() {
            if bytes[index] == b'%' {
                while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
                    index += 1;
                }
                continue;
            }
            if bytes[index] != b'@' {
                index += 1;
                continue;
            }
            let start = index;
            index += 1;
            let type_start = index;
            while index < bytes.len() && is_letter(bytes[index]) {
                index += 1;
            }
            let entry_type = text(bytes, type_start, index).to_lowercase();
            while index < bytes.len() && is_whitespace(bytes[index]) {
                index += 1;
            }
            if entry_type.is_empty()
                || index >= bytes.len()
                || (bytes[index] != b'{' && bytes[index] != b'(')
            {
                let range = SourceRange::new(start as i64, (index - start) as i64)?;
                result.diagnostics.push(LanguageDiagnostic::MalformedGroup {
                    source_id: source_id.to_string(),
                    command: format!("@{entry_type}"),
                    range,
                });
                continue;
            }
            let opening = bytes[index];
            let closing: u8 = if opening == b'{' { b'}' } else { b')' };
            index += 1;
            if ["comment", "preamble", "string"].contains(&entry_type.as_str()) {
                index = balanced_end(bytes, index, opening, closing).unwrap_or(bytes.len());
                continue;
            }
            let key_start = index;
            while index < bytes.len() && bytes[index] != b',' && bytes[index] != closing {
                index += 1;
            }
            let key = trimmed(&text(bytes, key_start, index)).to_string();
            if key.is_empty() || index >= bytes.len() || bytes[index] != b',' {
                let range = SourceRange::new(start as i64, (index - start) as i64)?;
                result.diagnostics.push(LanguageDiagnostic::MalformedGroup {
                    source_id: source_id.to_string(),
                    command: format!("@{entry_type}"),
                    range,
                });
                continue;
            }
            let Some(end) = balanced_end(bytes, index + 1, opening, closing) else {
                let range = SourceRange::new(start as i64, (bytes.len() - start) as i64)?;
                result.diagnostics.push(LanguageDiagnostic::MalformedGroup {
                    source_id: source_id.to_string(),
                    command: format!("@{entry_type}"),
                    range,
                });
                break;
            };
            let range = SourceRange::new(start as i64, (end - start) as i64)?;
            result.bibliography.push(BibTeXEntry::new(
                entry_type.clone(),
                key,
                range,
            )?);
            result.minimap.push(MinimapRange {
                kind: MinimapRangeKind::Bibliography,
                range,
            });
            index = end;
        }
        Ok(result)
    }

    /// (contentStart, contentEnd, end) of a `{…}` group after `start`.
    fn group_after(start: usize, bytes: &[u8]) -> Option<(usize, usize, usize)> {
        let mut index = start;
        while index < bytes.len() && is_whitespace(bytes[index]) {
            index += 1;
        }
        if index >= bytes.len() || bytes[index] != b'{' {
            return None;
        }
        let content_start = index + 1;
        let mut depth = 1usize;
        index += 1;
        while index < bytes.len() {
            if bytes[index] == b'\\' {
                index += std::cmp::min(2, bytes.len() - index);
                continue;
            }
            if bytes[index] == b'%' {
                while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
                    index += 1;
                }
                continue;
            }
            if bytes[index] == b'{' {
                depth += 1;
            }
            if bytes[index] == b'}' {
                depth -= 1;
                if depth == 0 {
                    return Some((content_start, index, index + 1));
                }
            }
            index += 1;
        }
        None
    }

    fn balanced_end(bytes: &[u8], start: usize, opening: u8, closing: u8) -> Option<usize> {
        let mut depth = 1usize;
        let mut quoted = false;
        let mut index = start;
        while index < bytes.len() {
            if bytes[index] == b'\\' {
                index += std::cmp::min(2, bytes.len() - index);
                continue;
            }
            if bytes[index] == b'"' {
                quoted = !quoted;
                index += 1;
                continue;
            }
            if !quoted && bytes[index] == b'%' {
                while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
                    index += 1;
                }
                continue;
            }
            if !quoted && bytes[index] == opening {
                depth += 1;
            }
            if !quoted && bytes[index] == closing {
                depth -= 1;
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            index += 1;
        }
        None
    }

    fn find(needle: &[u8], bytes: &[u8], start: usize) -> Option<usize> {
        if needle.is_empty() || needle.len() > bytes.len() {
            return None;
        }
        if start > bytes.len() - needle.len() {
            return None;
        }
        (start..=bytes.len() - needle.len()).find(|&index| &bytes[index..index + needle.len()] == needle)
    }

    fn text(bytes: &[u8], start: usize, end: usize) -> String {
        String::from_utf8_lossy(&bytes[start..end]).into_owned()
    }

    fn group_text(bytes: &[u8], start: usize, end: usize) -> String {
        let mut content: Vec<u8> = Vec::new();
        let mut index = start;
        while index < end {
            if bytes[index] == b'\\' && index + 1 < end {
                content.push(bytes[index]);
                content.push(bytes[index + 1]);
                index += 2;
            } else if bytes[index] == b'%' {
                while index < end && bytes[index] != b'\n' && bytes[index] != b'\r' {
                    index += 1;
                }
            } else {
                content.push(bytes[index]);
                index += 1;
            }
        }
        String::from_utf8_lossy(&content).into_owned()
    }

    fn trimmed(value: &str) -> &str {
        value.trim_matches(|c: char| c.is_whitespace())
    }

    fn is_letter(byte: u8) -> bool {
        byte.is_ascii_uppercase() || byte.is_ascii_lowercase()
    }

    fn is_whitespace(byte: u8) -> bool {
        byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r'
    }
}
