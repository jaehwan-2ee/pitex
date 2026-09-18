//! Rust port of `Packages/TexCore/Sources/SyncTeXCore`.
//!
//! Pure value types, the `.synctex` text grammar parser, the normalized
//! `synctex view`/`edit` query-result parser, and the exact-match selectors.
//! No process execution lives here — the feature layer runs `synctex` and
//! feeds decoded bytes in.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncTeXError {
    EmptyPath,
    InvalidCoordinate,
    InvalidLine,
    InvalidUTF8,
    MalformedInput { line: usize },
    StaleResult,
    NoMatch,
    AmbiguousMatch { count: usize },
}
impl fmt::Display for SyncTeXError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SyncTeXError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncTeXQueryError {
    Malformed { line: usize },
    PathOutsideRoot { line: usize },
    StaleResult,
    NoMatch,
    AmbiguousMatch { count: usize },
}
impl fmt::Display for SyncTeXQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SyncTeXQueryError {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncTeXRevision {
    #[serde(rename = "buildID")]
    pub build_id: String,
    pub fingerprint: u64,
}
impl SyncTeXRevision {
    pub fn new(build_id: impl Into<String>, fingerprint: u64) -> Result<Self, SyncTeXError> {
        let build_id = build_id.into();
        if build_id.is_empty() {
            return Err(SyncTeXError::EmptyPath);
        }
        Ok(Self {
            build_id,
            fingerprint,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NormalizedSourcePath {
    pub value: String,
}
impl NormalizedSourcePath {
    pub fn new(path: &str) -> Result<Self, SyncTeXError> {
        if path.is_empty() || path.chars().any(|c| c == '\0') {
            return Err(SyncTeXError::EmptyPath);
        }
        let absolute = path.starts_with('/');
        let mut components: Vec<&str> = Vec::new();
        for component in path.split('/') {
            if component.is_empty() || component == "." {
                continue;
            }
            if component == ".." {
                match components.last() {
                    Some(&last) if last != ".." => {
                        components.pop();
                    }
                    _ => return Err(SyncTeXError::EmptyPath),
                }
            } else {
                components.push(component);
            }
        }
        if components.is_empty() {
            return Err(SyncTeXError::EmptyPath);
        }
        Ok(Self {
            value: format!("{}{}", if absolute { "/" } else { "" }, components.join("/")),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PDFPoint {
    pub x: f64,
    pub y: f64,
}
impl Eq for PDFPoint {}
impl std::hash::Hash for PDFPoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}
impl PDFPoint {
    pub fn new(x: f64, y: f64) -> Result<Self, SyncTeXError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(SyncTeXError::InvalidCoordinate);
        }
        Ok(Self { x, y })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub path: NormalizedSourcePath,
    pub line: i64,
    pub column: i64,
}
impl SourceLocation {
    pub fn new(
        path: NormalizedSourcePath,
        line: i64,
        column: i64,
    ) -> Result<Self, SyncTeXError> {
        if line <= 0 || column < 0 {
            return Err(SyncTeXError::InvalidLine);
        }
        Ok(Self { path, line, column })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PDFLocation {
    #[serde(rename = "pdfPath")]
    pub pdf_path: NormalizedSourcePath,
    pub page: i64,
    pub point: PDFPoint,
}
impl PDFLocation {
    pub fn new(
        pdf_path: NormalizedSourcePath,
        page: i64,
        point: PDFPoint,
    ) -> Result<Self, SyncTeXError> {
        if page <= 0 {
            return Err(SyncTeXError::InvalidLine);
        }
        Ok(Self {
            pdf_path,
            page,
            point,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ForwardSyncQuery {
    pub revision: SyncTeXRevision,
    pub source: SourceLocation,
    #[serde(rename = "expectedPDF")]
    pub expected_pdf: NormalizedSourcePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InverseSyncQuery {
    pub revision: SyncTeXRevision,
    pub pdf: PDFLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncTeXMatch {
    pub revision: SyncTeXRevision,
    pub source: SourceLocation,
    pub pdf: PDFLocation,
    pub exact: bool,
}

pub struct ExactMatchSelector;
impl ExactMatchSelector {
    pub fn forward(
        candidates: &[SyncTeXMatch],
        query: &ForwardSyncQuery,
    ) -> Result<SyncTeXMatch, SyncTeXError> {
        Self::one(
            candidates
                .iter()
                .filter(|c| c.source == query.source && c.pdf.pdf_path == query.expected_pdf)
                .cloned()
                .collect(),
            &query.revision,
        )
    }
    pub fn inverse(
        candidates: &[SyncTeXMatch],
        query: &InverseSyncQuery,
    ) -> Result<SyncTeXMatch, SyncTeXError> {
        Self::one(
            candidates
                .iter()
                .filter(|c| c.pdf == query.pdf)
                .cloned()
                .collect(),
            &query.revision,
        )
    }
    fn one(matches: Vec<SyncTeXMatch>, revision: &SyncTeXRevision) -> Result<SyncTeXMatch, SyncTeXError> {
        if matches.is_empty() {
            return Err(SyncTeXError::NoMatch);
        }
        let current: Vec<_> = matches
            .into_iter()
            .filter(|m| m.revision == *revision)
            .collect();
        if current.is_empty() {
            return Err(SyncTeXError::StaleResult);
        }
        let exact: Vec<_> = current.into_iter().filter(|m| m.exact).collect();
        if exact.is_empty() {
            return Err(SyncTeXError::NoMatch);
        }
        if exact.len() != 1 {
            return Err(SyncTeXError::AmbiguousMatch { count: exact.len() });
        }
        Ok(exact.into_iter().next().unwrap())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncTeXInput {
    pub tag: i64,
    pub path: NormalizedSourcePath,
}
impl SyncTeXInput {
    pub fn new(tag: i64, path: NormalizedSourcePath) -> Result<Self, SyncTeXError> {
        if tag < 0 {
            return Err(SyncTeXError::MalformedInput { line: 0 });
        }
        Ok(Self { tag, path })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncTeXTextDocument {
    pub inputs: Vec<SyncTeXInput>,
    #[serde(rename = "contentLines")]
    pub content_lines: Vec<String>,
}

/// Accepts the textual content emitted by SyncTeX, after any gzip decoding.
pub struct SyncTeXTextParser;
impl SyncTeXTextParser {
    pub fn parse_decoded_utf8(bytes: &[u8]) -> Result<SyncTeXTextDocument, SyncTeXError> {
        let text = std::str::from_utf8(bytes).map_err(|_| SyncTeXError::InvalidUTF8)?;
        Self::parse(text)
    }

    pub fn parse(text: &str) -> Result<SyncTeXTextDocument, SyncTeXError> {
        let mut inputs: Vec<SyncTeXInput> = Vec::new();
        let mut seen_tags = std::collections::HashSet::new();
        let lines: Vec<&str> = text.split('\n').collect();
        for (index, raw_line) in lines.iter().enumerate() {
            if !raw_line.starts_with("Input:") {
                continue;
            }
            let body = &raw_line["Input:".len()..];
            let Some(separator) = body.find(':') else {
                return Err(SyncTeXError::MalformedInput { line: index + 1 });
            };
            let Ok(tag) = body[..separator].parse::<i64>() else {
                return Err(SyncTeXError::MalformedInput { line: index + 1 });
            };
            if !seen_tags.insert(tag) {
                return Err(SyncTeXError::MalformedInput { line: index + 1 });
            }
            let raw_path = body[separator + 1..].trim();
            let path = NormalizedSourcePath::new(raw_path)
                .map_err(|_| SyncTeXError::MalformedInput { line: index + 1 })?;
            inputs.push(
                SyncTeXInput::new(tag, path)
                    .map_err(|_| SyncTeXError::MalformedInput { line: index + 1 })?,
            );
        }
        if inputs.is_empty() {
            return Err(SyncTeXError::MalformedInput { line: 1 });
        }
        inputs.sort_by_key(|i| i.tag);
        Ok(SyncTeXTextDocument {
            inputs,
            content_lines: lines.iter().map(|l| l.to_string()).collect(),
        })
    }
}

// ---------------------------------------------------------------------------
// Query-result parser (`synctex view` / `synctex edit`, normalized output)

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncTeXOutputBinding {
    pub revision: SyncTeXRevision,
    #[serde(rename = "outputHash")]
    pub output_hash: String,
}
impl SyncTeXOutputBinding {
    pub fn new(
        revision: SyncTeXRevision,
        output_hash: impl Into<String>,
    ) -> Result<Self, SyncTeXQueryError> {
        let output_hash = output_hash.into();
        if output_hash.is_empty() {
            return Err(SyncTeXQueryError::Malformed { line: 0 });
        }
        Ok(Self {
            revision,
            output_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SyncTeXQueryMetadata {
    pub version: i64,
    pub fingerprint: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncTeXQueryCandidate {
    pub binding: SyncTeXOutputBinding,
    pub metadata: SyncTeXQueryMetadata,
    pub source: SourceLocation,
    pub pdf: PDFLocation,
    pub h: f64,
    pub v: f64,
    pub width: f64,
    pub height: f64,
}
impl SyncTeXQueryCandidate {
    pub fn to_match(&self) -> SyncTeXMatch {
        SyncTeXMatch {
            revision: self.binding.revision.clone(),
            source: self.source.clone(),
            pdf: self.pdf.clone(),
            exact: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncTeXQueryDocument {
    pub metadata: SyncTeXQueryMetadata,
    pub candidates: Vec<SyncTeXQueryCandidate>,
}

const ALLOWED_FIELDS: &[&str] = &[
    "Input", "Line", "Column", "Output", "Page", "x", "y", "h", "v", "W", "H",
];

/// Parser for the normalized, line-oriented output of `synctex view` and
/// `synctex edit`. Normalization adds the queried Input/Line/Column and
/// Page/x/y values to each result block, but does not change SyncTeX values.
pub struct SyncTeXQueryParser;
impl SyncTeXQueryParser {
    pub fn parse(
        text: &str,
        project_root: &str,
        binding: &SyncTeXOutputBinding,
    ) -> Result<SyncTeXQueryDocument, SyncTeXQueryError> {
        let root = canonical_root(project_root)?;
        let mut version: Option<i64> = None;
        let mut fingerprint: Option<u64> = None;
        let mut blocks: Vec<(usize, HashMap<String, (String, usize)>)> = Vec::new();
        let mut block_start: Option<usize> = None;
        let mut fields: HashMap<String, (String, usize)> = HashMap::new();

        for (offset, raw) in text.split('\n').enumerate() {
            let number = offset + 1;
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if line == "SyncTeX result begin" {
                if block_start.is_some() {
                    return Err(SyncTeXQueryError::Malformed { line: number });
                }
                block_start = Some(number);
                fields = HashMap::new();
                continue;
            }
            if line == "SyncTeX result end" {
                let Some(start) = block_start else {
                    return Err(SyncTeXQueryError::Malformed { line: number });
                };
                blocks.push((start, std::mem::take(&mut fields)));
                block_start = None;
                continue;
            }
            if block_start.is_some() {
                if line.is_empty() {
                    return Err(SyncTeXQueryError::Malformed { line: number });
                }
                let Some(separator) = line.find(':') else {
                    return Err(SyncTeXQueryError::Malformed { line: number });
                };
                let key = &line[..separator];
                let value = &line[separator + 1..];
                if !ALLOWED_FIELDS.contains(&key) || fields.contains_key(key) {
                    return Err(SyncTeXQueryError::Malformed { line: number });
                }
                fields.insert(key.to_string(), (value.to_string(), number));
            } else if let Some(rest) = line.strip_prefix("SyncTeX Version:") {
                if version.is_some() {
                    return Err(SyncTeXQueryError::Malformed { line: number });
                }
                version = Some(
                    strict_positive_int(rest)
                        .ok_or(SyncTeXQueryError::Malformed { line: number })?,
                );
            } else if let Some(rest) = line.strip_prefix("SyncTeX Fingerprint:") {
                if fingerprint.is_some() || !is_ascii_digits(rest) {
                    return Err(SyncTeXQueryError::Malformed { line: number });
                }
                fingerprint = Some(
                    rest.parse::<u64>()
                        .map_err(|_| SyncTeXQueryError::Malformed { line: number })?,
                );
            } else if !line.is_empty() {
                return Err(SyncTeXQueryError::Malformed { line: number });
            }
        }
        if let Some(start) = block_start {
            return Err(SyncTeXQueryError::Malformed { line: start });
        }
        let (Some(version), Some(fingerprint)) = (version, fingerprint) else {
            return Err(SyncTeXQueryError::Malformed { line: 1 });
        };
        if fingerprint != binding.revision.fingerprint {
            return Err(SyncTeXQueryError::StaleResult);
        }
        if blocks.is_empty() {
            return Err(SyncTeXQueryError::NoMatch);
        }

        let metadata = SyncTeXQueryMetadata {
            version,
            fingerprint,
        };
        let mut candidates = Vec::with_capacity(blocks.len());
        for (start, fields) in blocks {
            for required in ALLOWED_FIELDS {
                if !fields.contains_key(*required) {
                    return Err(SyncTeXQueryError::Malformed { line: start });
                }
            }
            let input = &fields["Input"];
            let output = &fields["Output"];
            let source_path = canonical_project_path(&input.0, &root, input.1)?;
            let pdf_path = canonical_project_path(&output.0, &root, output.1)?;
            let malformed = || SyncTeXQueryError::Malformed { line: start };
            let line = strict_positive_int(&fields["Line"].0).ok_or_else(malformed)?;
            let column = strict_nonnegative_int(&fields["Column"].0).ok_or_else(malformed)?;
            let page = strict_positive_int(&fields["Page"].0).ok_or_else(malformed)?;
            let x = strict_double(&fields["x"].0).ok_or_else(malformed)?;
            let y = strict_double(&fields["y"].0).ok_or_else(malformed)?;
            let h = strict_double(&fields["h"].0).ok_or_else(malformed)?;
            let v = strict_double(&fields["v"].0).ok_or_else(malformed)?;
            let width = strict_double(&fields["W"].0).ok_or_else(malformed)?;
            let height = strict_double(&fields["H"].0).ok_or_else(malformed)?;
            if width < 0.0 || height < 0.0 {
                return Err(malformed());
            }
            let source = SourceLocation::new(source_path, line, column).map_err(|_| malformed())?;
            let pdf = PDFLocation::new(pdf_path, page, PDFPoint { x, y })
                .map_err(|_| malformed())?;
            candidates.push(SyncTeXQueryCandidate {
                binding: binding.clone(),
                metadata: metadata.clone(),
                source,
                pdf,
                h,
                v,
                width,
                height,
            });
        }
        Ok(SyncTeXQueryDocument {
            metadata,
            candidates,
        })
    }
}

fn canonical_root(raw: &str) -> Result<Vec<String>, SyncTeXQueryError> {
    if !raw.starts_with('/') || raw.contains('\\') || raw.contains('\0') {
        return Err(SyncTeXQueryError::PathOutsideRoot { line: 0 });
    }
    let components: Vec<String> = raw
        .split('/')
        .filter(|c| !c.is_empty())
        .map(String::from)
        .collect();
    if components.is_empty() || components.iter().any(|c| c == "..") {
        return Err(SyncTeXQueryError::PathOutsideRoot { line: 0 });
    }
    Ok(components
        .into_iter()
        .filter(|c| c != ".")
        .collect())
}

fn canonical_project_path(
    raw: &str,
    root: &[String],
    line: usize,
) -> Result<NormalizedSourcePath, SyncTeXQueryError> {
    let err = || SyncTeXQueryError::PathOutsideRoot { line };
    if raw.is_empty() || raw.contains('\\') || raw.contains('\0') {
        return Err(err());
    }
    let components: Vec<String> = raw
        .split('/')
        .filter(|c| !c.is_empty())
        .map(String::from)
        .collect();
    if components.iter().any(|c| c == "..") {
        return Err(err());
    }
    let clean: Vec<String> = components.into_iter().filter(|c| c != ".").collect();
    let relative: Vec<String> = if raw.starts_with('/') {
        if clean.len() <= root.len() || clean[..root.len()] != root[..] {
            return Err(err());
        }
        clean[root.len()..].to_vec()
    } else {
        if clean.is_empty() {
            return Err(err());
        }
        clean
    };
    NormalizedSourcePath::new(&relative.join("/")).map_err(|_| err())
}

fn is_ascii_digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
}

fn strict_positive_int(value: &str) -> Option<i64> {
    if !is_ascii_digits(value) {
        return None;
    }
    value.parse::<i64>().ok().filter(|v| *v > 0)
}

fn strict_nonnegative_int(value: &str) -> Option<i64> {
    if !is_ascii_digits(value) {
        return None;
    }
    value.parse::<i64>().ok()
}

fn strict_double(value: &str) -> Option<f64> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'E' | b'e'))
    {
        return None;
    }
    value.parse::<f64>().ok().filter(|v| v.is_finite())
}

// ---------------------------------------------------------------------------
// Exact selectors over parsed candidates (revision + output-hash bound)

pub struct ExactSyncTeXQuerySelector;
impl ExactSyncTeXQuerySelector {
    pub fn forward(
        candidates: &[SyncTeXQueryCandidate],
        query: &ForwardSyncQuery,
        output_hash: &str,
    ) -> Result<SyncTeXQueryCandidate, SyncTeXQueryError> {
        let path_matches: Vec<_> = candidates
            .iter()
            .filter(|c| c.source == query.source && c.pdf.pdf_path == query.expected_pdf)
            .cloned()
            .collect();
        Self::select_bound(path_matches, &query.revision, output_hash)
    }

    pub fn inverse(
        candidates: &[SyncTeXQueryCandidate],
        query: &InverseSyncQuery,
        output_hash: &str,
        coordinate_epsilon: f64,
    ) -> Result<SyncTeXQueryCandidate, SyncTeXQueryError> {
        if !coordinate_epsilon.is_finite() || coordinate_epsilon < 0.0 {
            return Err(SyncTeXQueryError::Malformed { line: 0 });
        }
        let page_matches: Vec<_> = candidates
            .iter()
            .filter(|c| {
                c.pdf.pdf_path == query.pdf.pdf_path && c.pdf.page == query.pdf.page
            })
            .cloned()
            .collect();
        if page_matches.is_empty() {
            return Err(SyncTeXQueryError::NoMatch);
        }
        let bound: Vec<_> = page_matches
            .into_iter()
            .filter(|c| {
                c.binding.revision == query.revision && c.binding.output_hash == output_hash
            })
            .collect();
        if bound.is_empty() {
            return Err(SyncTeXQueryError::StaleResult);
        }
        let coordinate_matches: Vec<_> = bound
            .into_iter()
            .filter(|c| {
                (c.pdf.point.x - query.pdf.point.x).abs() <= coordinate_epsilon
                    && (c.pdf.point.y - query.pdf.point.y).abs() <= coordinate_epsilon
            })
            .collect();
        if coordinate_matches.is_empty() {
            return Err(SyncTeXQueryError::NoMatch);
        }
        if coordinate_matches.len() != 1 {
            return Err(SyncTeXQueryError::AmbiguousMatch {
                count: coordinate_matches.len(),
            });
        }
        Ok(coordinate_matches.into_iter().next().unwrap())
    }

    /// Convenience overload matching Swift's `coordinateEpsilon = 0` default.
    pub fn inverse_exact(
        candidates: &[SyncTeXQueryCandidate],
        query: &InverseSyncQuery,
        output_hash: &str,
    ) -> Result<SyncTeXQueryCandidate, SyncTeXQueryError> {
        Self::inverse(candidates, query, output_hash, 0.0)
    }

    fn select_bound(
        candidates: Vec<SyncTeXQueryCandidate>,
        revision: &SyncTeXRevision,
        output_hash: &str,
    ) -> Result<SyncTeXQueryCandidate, SyncTeXQueryError> {
        if candidates.is_empty() {
            return Err(SyncTeXQueryError::NoMatch);
        }
        let bound: Vec<_> = candidates
            .into_iter()
            .filter(|c| c.binding.revision == *revision && c.binding.output_hash == output_hash)
            .collect();
        if bound.is_empty() {
            return Err(SyncTeXQueryError::StaleResult);
        }
        if bound.len() != 1 {
            return Err(SyncTeXQueryError::AmbiguousMatch { count: bound.len() });
        }
        Ok(bound.into_iter().next().unwrap())
    }
}
