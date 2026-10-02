//! Port of `EquationPreviewSyntax.swift` + `EquationPreviewContext.swift` —
//! complete math regions, preview-visible macro definitions and include
//! edges, found in one forward pass over UTF-8, plus the project-level
//! definition context they feed. All offsets are UTF-8 byte offsets;
//! hosts convert their own units (UTF-16 on macOS, chars on GTK).
//!
//! The shared fixture `Fixtures/equation-preview/syntax.json` is the parity
//! contract with the Swift scanner.

use crate::SourceRange;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MathDelimiter {
    Dollar,
    DoubleDollar,
    Parenthesis,
    Bracket,
    Environment,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MathRegion {
    /// The whole construct, delimiters or `\begin{…}`/`\end{…}` included.
    pub range: SourceRange,
    pub content_range: SourceRange,
    pub delimiter: MathDelimiter,
    pub environment_name: Option<String>,
    pub display_mode: bool,
    /// False when TeX would reject the region before any closer: a
    /// paragraph break, `\end{document}`, a mismatched `\end`, or EOF.
    pub is_complete: bool,
    /// Braces balance and no stray math shift appears inside the body.
    pub is_well_formed: bool,
}

impl MathRegion {
    pub fn is_renderable(&self) -> bool {
        self.is_complete && self.is_well_formed
    }

    /// Caret semantics: just after the opener's first byte through just
    /// after the closer — `$x$|` still previews, `|$x$` does not.
    pub fn contains_caret(&self, utf8_offset: usize) -> bool {
        (utf8_offset as i64) > self.range.utf8_offset
            && (utf8_offset as i64) <= self.range.end_utf8_offset()
    }

    /// Pointer semantics: the character starting at `utf8_offset`.
    pub fn contains_character(&self, utf8_offset: usize) -> bool {
        (utf8_offset as i64) >= self.range.utf8_offset
            && (utf8_offset as i64) < self.range.end_utf8_offset()
    }

    /// Environments whose body is handed to MathJax as display/inline math;
    /// every other supported environment is passed whole because MathJax
    /// parses `\begin{align}…\end{align}` itself.
    fn body_only_environment(name: &str) -> bool {
        matches!(name, "math" | "displaymath" | "equation" | "equation*")
    }

    /// The TeX handed to the fast renderer.
    pub fn render_source(&self, source: &str) -> String {
        if self.delimiter == MathDelimiter::Environment
            && self
                .environment_name
                .as_deref()
                .map_or(false, |name| !Self::body_only_environment(name))
        {
            return Self::slice(source, self.range);
        }
        Self::slice(source, self.content_range)
    }

    /// The region exactly as written, for the exact TeX document.
    pub fn source_text(&self, source: &str) -> String {
        Self::slice(source, self.range)
    }

    /// Lossy UTF-8 slice with bounds clamped to the source (Swift
    /// `String(decoding:as: UTF8.self)` semantics).
    fn slice(source: &str, range: SourceRange) -> String {
        let utf8 = source.as_bytes();
        let lower = (range.utf8_offset.max(0) as usize).min(utf8.len());
        let upper = lower.saturating_add(range.utf8_length.max(0) as usize).min(utf8.len());
        String::from_utf8_lossy(&utf8[lower..upper]).into_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MathDefinitionKind {
    NewCommand,
    RenewCommand,
    ProvideCommand,
    DeclareMathOperator,
    NewEnvironment,
    RenewEnvironment,
}

impl MathDefinitionKind {
    /// The Swift rawValue — the string the shared fixture compares.
    pub fn raw_value(self) -> &'static str {
        match self {
            Self::NewCommand => "newCommand",
            Self::RenewCommand => "renewCommand",
            Self::ProvideCommand => "provideCommand",
            Self::DeclareMathOperator => "declareMathOperator",
            Self::NewEnvironment => "newEnvironment",
            Self::RenewEnvironment => "renewEnvironment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MathDefinition {
    pub kind: MathDefinitionKind,
    /// `\R` for commands and operators, the bare name for environments.
    pub name: String,
    /// Normalized, MathJax-parsable form (`\providecommand` is emitted as
    /// `\newcommand`; the context decides whether it applies).
    pub math_jax_source: String,
    /// The definition exactly as written, for the exact TeX document.
    pub original_source: String,
    pub range: SourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MathInclude {
    /// As written; `\usepackage{x}` is recorded as `x.sty` so project-local
    /// packages contribute definitions while system packages never resolve.
    pub target: String,
    pub utf8_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MathSourceScan {
    /// Top-level regions, sorted and non-overlapping.
    pub regions: Vec<MathRegion>,
    pub definitions: Vec<MathDefinition>,
    pub includes: Vec<MathInclude>,
    /// Offset of `\begin{document}` outside comments/verbatim, if any.
    pub document_begin_offset: Option<usize>,
    /// Source before `\begin{document}`.
    pub preamble: Option<String>,
}

impl MathSourceScan {
    /// Last region starting strictly before the caret.
    pub fn region_at_caret(&self, utf8_offset: usize) -> Option<&MathRegion> {
        let split = self
            .regions
            .partition_point(|r| r.range.utf8_offset < utf8_offset as i64);
        let region = self.regions.get(split.checked_sub(1)?)?;
        region.contains_caret(utf8_offset).then_some(region)
    }

    pub fn region_at_character(&self, utf8_offset: usize) -> Option<&MathRegion> {
        let split = self
            .regions
            .partition_point(|r| r.range.utf8_offset <= utf8_offset as i64);
        let region = self.regions.get(split.checked_sub(1)?)?;
        region.contains_character(utf8_offset).then_some(region)
    }
}

pub struct MathSourceScanner;

impl MathSourceScanner {
    /// Display-level environments (`math` is the one inline member).
    fn outer_environment(name: &str) -> Option<bool> {
        match name {
            "math" => Some(false),
            "displaymath" | "equation" | "equation*" | "align" | "align*" | "gather"
            | "gather*" | "multline" | "multline*" | "flalign" | "flalign*" | "alignat"
            | "alignat*" | "eqnarray" | "eqnarray*" => Some(true),
            _ => None,
        }
    }

    /// Inner math environments: part of an enclosing region, or display
    /// math of their own when written standalone.
    fn inner_environment(name: &str) -> bool {
        matches!(
            name,
            "aligned" | "alignedat" | "gathered" | "split" | "cases" | "array" | "matrix"
                | "pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix" | "smallmatrix"
        )
    }

    fn verbatim_environment(name: &str) -> bool {
        matches!(
            name,
            "verbatim" | "verbatim*" | "Verbatim" | "Verbatim*" | "BVerbatim" | "LVerbatim"
                | "lstlisting" | "minted" | "comment" | "filecontents" | "filecontents*"
        )
    }

    pub fn scan(source: &str) -> MathSourceScan {
        let bytes = source.as_bytes();
        let mut scanner = Scanner::new(bytes);
        scanner.run();
        let preamble = scanner
            .document_begin
            .map(|end| String::from_utf8_lossy(&bytes[..end]).into_owned());
        MathSourceScan {
            regions: scanner.regions,
            definitions: scanner.definitions,
            includes: scanner.includes,
            document_begin_offset: scanner.document_begin,
            preamble,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Keyword {
    Begin,
    End,
    Par,
    InlineVerbatim,
    RawGroup,
    MintInline,
    NewCommand,
    RenewCommand,
    ProvideCommand,
    DeclareMathOperator,
    NewEnvironment,
    RenewEnvironment,
    Input,
    ImportFile,
    SubImport,
    Usepackage,
    TextArgument,
}

fn keyword(name: &[u8]) -> Option<Keyword> {
    Some(match name {
        b"begin" => Keyword::Begin,
        b"end" => Keyword::End,
        b"par" => Keyword::Par,
        b"verb" | b"Verb" | b"lstinline" => Keyword::InlineVerbatim,
        b"url" | b"path" | b"href" => Keyword::RawGroup,
        b"mintinline" => Keyword::MintInline,
        b"newcommand" => Keyword::NewCommand,
        b"renewcommand" => Keyword::RenewCommand,
        b"providecommand" => Keyword::ProvideCommand,
        b"DeclareMathOperator" => Keyword::DeclareMathOperator,
        b"newenvironment" => Keyword::NewEnvironment,
        b"renewenvironment" => Keyword::RenewEnvironment,
        b"input" | b"include" | b"subfile" => Keyword::Input,
        b"import" => Keyword::ImportFile,
        b"subimport" => Keyword::SubImport,
        b"usepackage" | b"RequirePackage" => Keyword::Usepackage,
        b"text" | b"textrm" | b"textit" | b"textbf" | b"textsf" | b"texttt" | b"textup"
        | b"textnormal" | b"textmd" | b"textsc" | b"textsl" | b"emph" | b"mbox" | b"hbox"
        | b"fbox" | b"makebox" | b"framebox" | b"parbox" | b"intertext" | b"shortintertext"
        | b"tag" => Keyword::TextArgument,
        _ => return None,
    })
}

enum MathOutcome {
    Closed { content_end: usize, end: usize, well_formed: bool },
    /// `end` is where the region stops; scanning resumes in text at `resume`.
    Incomplete { end: usize, resume: usize },
}

const BACKSLASH: u8 = b'\\';
const DOLLAR: u8 = b'$';
const PERCENT: u8 = b'%';
const LBRACE: u8 = b'{';
const RBRACE: u8 = b'}';
const LBRACKET: u8 = b'[';
const RBRACKET: u8 = b']';
const LPAREN: u8 = b'(';
const RPAREN: u8 = b')';
const NEWLINE: u8 = b'\n';
const CARRIAGE_RETURN: u8 = b'\r';
const SPACE: u8 = b' ';
const TAB: u8 = b'\t';
const STAR: u8 = b'*';
const COMMA: u8 = b',';

struct Scanner<'a> {
    b: &'a [u8],
    i: usize,
    regions: Vec<MathRegion>,
    definitions: Vec<MathDefinition>,
    includes: Vec<MathInclude>,
    document_begin: Option<usize>,
    /// `\text{$…\text{$…` recursion guard — deeper nesting is read as plain math.
    nesting: u8,
}

impl<'a> Scanner<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, i: 0, regions: Vec::new(), definitions: Vec::new(), includes: Vec::new(), document_begin: None, nesting: 0 }
    }

    fn count(&self) -> usize {
        self.b.len()
    }

    fn is_letter(byte: u8) -> bool {
        byte.is_ascii_uppercase() || byte.is_ascii_lowercase()
    }

    fn string(&self, start: usize, end: usize) -> String {
        String::from_utf8_lossy(&self.b[start..end]).into_owned()
    }

    fn keyword(&self, start: usize, end: usize) -> Option<Keyword> {
        keyword(&self.b[start..end])
    }

    // MARK: Text mode

    fn run(&mut self) {
        while self.i < self.count() {
            match self.b[self.i] {
                PERCENT => self.skip_comment(),
                DOLLAR => self.open_dollar(),
                BACKSLASH => self.text_control_sequence(),
                _ => self.i += 1,
            }
        }
    }

    fn skip_comment(&mut self) {
        while self.i < self.count() && self.b[self.i] != NEWLINE && self.b[self.i] != CARRIAGE_RETURN {
            self.i += 1;
        }
    }

    /// After a backslash's next byte: skip the rest of a control symbol
    /// (UTF-8 continuation bytes ride along).
    fn skip_symbol(&mut self) {
        self.i += 1;
        while self.i < self.count() && self.b[self.i] & 0xC0 == 0x80 {
            self.i += 1;
        }
    }

    fn read_letters(&mut self) -> (usize, usize) {
        let start = self.i;
        while self.i < self.count() && Self::is_letter(self.b[self.i]) {
            self.i += 1;
        }
        (start, self.i)
    }

    fn skip_spaces(&mut self) {
        while self.i < self.count() && (self.b[self.i] == SPACE || self.b[self.i] == TAB) {
            self.i += 1;
        }
    }

    /// Spaces plus at most one line break — TeX's tokenization between a
    /// control word and its arguments without crossing a paragraph. A `%`
    /// comment eats the rest of its line (the newline it ends at counts as
    /// the one break); a second line break is still a paragraph boundary.
    fn skip_inter_argument_space(&mut self) {
        loop {
            self.skip_spaces();
            if self.i < self.count() && self.b[self.i] == PERCENT {
                self.skip_comment();
                continue;
            }
            if self.i < self.count()
                && (self.b[self.i] == NEWLINE || self.b[self.i] == CARRIAGE_RETURN)
            {
                if self.blank_line_end(self.i).is_some() {
                    return;
                }
                if self.b[self.i] == CARRIAGE_RETURN
                    && self.i + 1 < self.count()
                    && self.b[self.i + 1] == NEWLINE
                {
                    self.i += 1;
                }
                self.i += 1;
                continue;
            }
            return;
        }
    }

    fn text_control_sequence(&mut self) {
        let start = self.i;
        self.i += 1;
        if self.i >= self.count() {
            return;
        }
        if !Self::is_letter(self.b[self.i]) {
            match self.b[self.i] {
                LPAREN => self.open_region(start, self.i + 1, MathDelimiter::Parenthesis, None, false),
                LBRACKET => self.open_region(start, self.i + 1, MathDelimiter::Bracket, None, true),
                _ => self.skip_symbol(),
            }
            return;
        }
        let (name_start, name_end) = self.read_letters();
        let Some(keyword) = self.keyword(name_start, name_end) else { return };
        match keyword {
            Keyword::Begin => {
                let Some((env_start, env_end, after)) = self.group_name() else { return };
                let name = self.string(env_start, env_end);
                if name == "document" {
                    if self.document_begin.is_none() {
                        self.document_begin = Some(start);
                    }
                    self.i = after;
                } else if MathSourceScanner::verbatim_environment(&name) {
                    self.skip_verbatim_environment(&name, after);
                } else if let Some(display) = MathSourceScanner::outer_environment(&name) {
                    self.open_region(start, after, MathDelimiter::Environment, Some(name), display);
                } else if MathSourceScanner::inner_environment(&name) {
                    self.open_region(start, after, MathDelimiter::Environment, Some(name), true);
                } else {
                    self.i = after;
                }
            }
            Keyword::End => {
                let Some((env_start, env_end, after)) = self.group_name() else { return };
                self.i = after;
                // TeX stops reading at \end{document}.
                if self.string(env_start, env_end) == "document" {
                    self.i = self.count();
                }
            }
            Keyword::InlineVerbatim => self.skip_inline_verbatim(false),
            Keyword::MintInline => self.skip_inline_verbatim(true),
            Keyword::RawGroup => self.skip_raw_group_or_delimited(),
            Keyword::NewCommand => self.parse_command_definition(start, MathDefinitionKind::NewCommand),
            Keyword::RenewCommand => self.parse_command_definition(start, MathDefinitionKind::RenewCommand),
            Keyword::ProvideCommand => self.parse_command_definition(start, MathDefinitionKind::ProvideCommand),
            Keyword::DeclareMathOperator => self.parse_operator_definition(start),
            Keyword::NewEnvironment => self.parse_environment_definition(start, MathDefinitionKind::NewEnvironment),
            Keyword::RenewEnvironment => self.parse_environment_definition(start, MathDefinitionKind::RenewEnvironment),
            Keyword::Input => {
                self.skip_inter_argument_space();
                if let Some((content_start, content_end, end)) = self.balanced_group() {
                    self.append_include(self.identifier(content_start, content_end), start);
                    self.i = end;
                }
            }
            Keyword::ImportFile | Keyword::SubImport => {
                self.skip_inter_argument_space();
                let Some((dir_start, dir_end, dir_after)) = self.balanced_group() else { return };
                self.i = dir_after;
                self.skip_inter_argument_space();
                let Some((file_start, file_end, file_after)) = self.balanced_group() else { return };
                let mut directory = self.identifier(dir_start, dir_end);
                if !directory.is_empty() && !directory.ends_with('/') {
                    directory.push('/');
                }
                self.append_include(directory + &self.identifier(file_start, file_end), start);
                self.i = file_after;
            }
            Keyword::Usepackage => {
                self.skip_inter_argument_space();
                while self.i < self.count() && self.b[self.i] == LBRACKET {
                    if !self.skip_bracket_group() {
                        return;
                    }
                    self.skip_inter_argument_space();
                }
                let Some((content_start, content_end, end)) = self.balanced_group() else { return };
                for name in self.identifier(content_start, content_end).split(COMMA as char) {
                    let package = name.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r'));
                    if !package.is_empty() {
                        self.append_include(format!("{package}.sty"), start);
                    }
                }
                self.i = end;
            }
            Keyword::Par | Keyword::TextArgument => {}
        }
    }

    fn append_include(&mut self, target: String, offset: usize) {
        if target.is_empty() {
            return;
        }
        self.includes.push(MathInclude { target, utf8_offset: offset });
    }

    fn open_dollar(&mut self) {
        let start = self.i;
        if self.i + 1 < self.count() && self.b[self.i + 1] == DOLLAR {
            self.open_region(start, self.i + 2, MathDelimiter::DoubleDollar, None, true);
        } else {
            self.open_region(start, self.i + 1, MathDelimiter::Dollar, None, false);
        }
    }

    fn open_region(
        &mut self,
        start: usize,
        content_start: usize,
        delimiter: MathDelimiter,
        environment: Option<String>,
        display: bool,
    ) {
        self.i = content_start;
        let outcome = self.scan_math(delimiter, environment.as_deref());
        let region = match outcome {
            MathOutcome::Closed { content_end, end, well_formed } => {
                let region = MathRegion {
                    range: SourceRange::lexer_range(start, end - start),
                    content_range: SourceRange::lexer_range(content_start, content_end - content_start),
                    delimiter,
                    environment_name: environment,
                    display_mode: display,
                    is_complete: true,
                    is_well_formed: well_formed,
                };
                self.i = end;
                region
            }
            MathOutcome::Incomplete { end, resume } => {
                let region = MathRegion {
                    range: SourceRange::lexer_range(start, end.max(content_start) - start),
                    content_range: SourceRange::lexer_range(content_start, end.saturating_sub(content_start)),
                    delimiter,
                    environment_name: environment,
                    display_mode: display,
                    is_complete: false,
                    is_well_formed: false,
                };
                self.i = resume.max(content_start);
                region
            }
        };
        self.regions.push(region);
    }

    /// Offset of the second line break when a paragraph break starts at `at`.
    fn blank_line_end(&self, position: usize) -> Option<usize> {
        let mut j = position;
        if self.b[j] == CARRIAGE_RETURN && j + 1 < self.count() && self.b[j + 1] == NEWLINE {
            j += 2;
        } else {
            j += 1;
        }
        while j < self.count() && (self.b[j] == SPACE || self.b[j] == TAB) {
            j += 1;
        }
        if j < self.count() && (self.b[j] == NEWLINE || self.b[j] == CARRIAGE_RETURN) {
            Some(j)
        } else {
            None
        }
    }

    // MARK: Math mode

    fn scan_math(&mut self, closer: MathDelimiter, environment: Option<&str>) -> MathOutcome {
        let mut braces = 0i64;
        let mut malformed = false;
        let mut same_name_depth = 0usize;
        while self.i < self.count() {
            match self.b[self.i] {
                PERCENT => self.skip_comment(),
                NEWLINE | CARRIAGE_RETURN => {
                    if let Some(resume) = self.blank_line_end(self.i) {
                        return MathOutcome::Incomplete { end: self.i, resume };
                    }
                    self.i += 1;
                }
                LBRACE => {
                    braces += 1;
                    self.i += 1;
                }
                RBRACE => {
                    braces -= 1;
                    self.i += 1;
                }
                DOLLAR => match closer {
                    MathDelimiter::Dollar => {
                        let content_end = self.i;
                        self.i += 1;
                        return MathOutcome::Closed {
                            content_end,
                            end: self.i,
                            well_formed: braces == 0 && !malformed,
                        };
                    }
                    MathDelimiter::DoubleDollar => {
                        if self.i + 1 < self.count() && self.b[self.i + 1] == DOLLAR {
                            let content_end = self.i;
                            self.i += 2;
                            return MathOutcome::Closed {
                                content_end,
                                end: self.i,
                                well_formed: braces == 0 && !malformed,
                            };
                        }
                        // A lone $ inside $$…$$ ends display math with an error.
                        return MathOutcome::Incomplete { end: self.i, resume: self.i };
                    }
                    MathDelimiter::Parenthesis | MathDelimiter::Bracket | MathDelimiter::Environment => {
                        malformed = true;
                        self.i += 1;
                    }
                },
                BACKSLASH => {
                    let command_start = self.i;
                    self.i += 1;
                    if self.i >= self.count() {
                        break;
                    }
                    if !Self::is_letter(self.b[self.i]) {
                        let symbol = self.b[self.i];
                        if (symbol == RPAREN && closer == MathDelimiter::Parenthesis)
                            || (symbol == RBRACKET && closer == MathDelimiter::Bracket)
                        {
                            self.i += 1;
                            return MathOutcome::Closed {
                                content_end: command_start,
                                end: self.i,
                                well_formed: braces == 0 && !malformed,
                            };
                        }
                        self.skip_symbol();
                        continue;
                    }
                    let (name_start, name_end) = self.read_letters();
                    match self.keyword(name_start, name_end) {
                        Some(Keyword::Par) => {
                            return MathOutcome::Incomplete { end: command_start, resume: name_end };
                        }
                        Some(Keyword::Begin) => {
                            if let Some((env_start, env_end, after)) = self.group_name() {
                                if let Some(env) = environment {
                                    if self.string(env_start, env_end) == env {
                                        same_name_depth += 1;
                                    }
                                }
                                self.i = after;
                            }
                        }
                        Some(Keyword::End) => {
                            if let Some((env_start, env_end, after)) = self.group_name() {
                                let name = self.string(env_start, env_end);
                                if closer == MathDelimiter::Environment && Some(name.as_str()) == environment {
                                    if same_name_depth == 0 {
                                        self.i = after;
                                        return MathOutcome::Closed {
                                            content_end: command_start,
                                            end: after,
                                            well_formed: braces == 0 && !malformed,
                                        };
                                    }
                                    same_name_depth -= 1;
                                } else if name == "document"
                                    || MathSourceScanner::outer_environment(&name).is_some()
                                {
                                    // Closes something this region never opened.
                                    return MathOutcome::Incomplete { end: command_start, resume: command_start };
                                }
                                self.i = after;
                            }
                        }
                        Some(Keyword::TextArgument) => {
                            if let Some(stop) = self.skip_text_argument() {
                                return stop;
                            }
                        }
                        Some(Keyword::InlineVerbatim) => self.skip_inline_verbatim(false),
                        Some(Keyword::RawGroup) => self.skip_raw_group_or_delimited(),
                        _ => {}
                    }
                }
                _ => self.i += 1,
            }
        }
        MathOutcome::Incomplete { end: self.count(), resume: self.count() }
    }

    /// Text-mode argument inside math: `$…$` there is nested math. Returns
    /// an outcome only when the enclosing region must stop.
    fn skip_text_argument(&mut self) -> Option<MathOutcome> {
        if self.i < self.count() && self.b[self.i] == STAR {
            self.i += 1;
        }
        self.skip_inter_argument_space();
        while self.i < self.count() && self.b[self.i] == LBRACKET {
            if !self.skip_bracket_group() {
                return None;
            }
            self.skip_inter_argument_space();
        }
        if self.i >= self.count() || self.b[self.i] != LBRACE {
            return None;
        }
        self.i += 1;
        let mut depth = 1i64;
        while self.i < self.count() {
            match self.b[self.i] {
                PERCENT => self.skip_comment(),
                NEWLINE | CARRIAGE_RETURN => {
                    if let Some(resume) = self.blank_line_end(self.i) {
                        return Some(MathOutcome::Incomplete { end: self.i, resume });
                    }
                    self.i += 1;
                }
                LBRACE => {
                    depth += 1;
                    self.i += 1;
                }
                RBRACE => {
                    depth -= 1;
                    self.i += 1;
                    if depth == 0 {
                        return None;
                    }
                }
                DOLLAR if self.nesting < 8 => {
                    self.i += 1;
                    self.nesting += 1;
                    let inner = self.scan_math(MathDelimiter::Dollar, None);
                    self.nesting -= 1;
                    if let MathOutcome::Incomplete { end, resume } = inner {
                        return Some(MathOutcome::Incomplete { end, resume });
                    }
                }
                BACKSLASH => {
                    self.i += 1;
                    if self.i >= self.count() {
                        break;
                    }
                    if Self::is_letter(self.b[self.i]) {
                        let (name_start, name_end) = self.read_letters();
                        if self.keyword(name_start, name_end) == Some(Keyword::Par) {
                            return Some(MathOutcome::Incomplete { end: name_start - 1, resume: name_end });
                        }
                    } else if self.b[self.i] == LPAREN && self.nesting < 8 {
                        self.i += 1;
                        self.nesting += 1;
                        let inner = self.scan_math(MathDelimiter::Parenthesis, None);
                        self.nesting -= 1;
                        if let MathOutcome::Incomplete { end, resume } = inner {
                            return Some(MathOutcome::Incomplete { end, resume });
                        }
                    } else {
                        self.skip_symbol();
                    }
                }
                _ => self.i += 1,
            }
        }
        Some(MathOutcome::Incomplete { end: self.count(), resume: self.count() })
    }

    // MARK: Verbatim

    fn skip_verbatim_environment(&mut self, name: &str, start: usize) {
        let marker = format!("\\end{{{name}}}").into_bytes();
        let mut j = start;
        while j + marker.len() <= self.count() {
            if self.b[j] == BACKSLASH && self.b[j + 1..j + marker.len()] == marker[1..] {
                self.i = j + marker.len();
                return;
            }
            j += 1;
        }
        self.i = self.count();
    }

    /// `\verb|…|`, `\lstinline[…]{…}`, `\mintinline{lang}|…|`.
    fn skip_inline_verbatim(&mut self, mint: bool) {
        if self.i < self.count() && self.b[self.i] == STAR {
            self.i += 1;
        }
        while self.i < self.count() && self.b[self.i] == LBRACKET {
            if !self.skip_bracket_group() {
                return;
            }
        }
        if mint {
            let Some((_, _, end)) = self.balanced_group() else { return };
            self.i = end;
        }
        self.skip_raw_group_or_delimited();
    }

    /// A raw `{…}` (braces nest, nothing else is special) or `<c>…<c>`.
    fn skip_raw_group_or_delimited(&mut self) {
        if self.i >= self.count() {
            return;
        }
        let opener = self.b[self.i];
        if opener == LBRACE {
            let mut depth = 0i64;
            while self.i < self.count() {
                if self.b[self.i] == LBRACE {
                    depth += 1;
                }
                if self.b[self.i] == RBRACE {
                    depth -= 1;
                    if depth == 0 {
                        self.i += 1;
                        return;
                    }
                }
                if self.b[self.i] == NEWLINE && self.blank_line_end(self.i).is_some() {
                    return;
                }
                self.i += 1;
            }
            return;
        }
        if opener == SPACE || opener == NEWLINE || opener == CARRIAGE_RETURN || Self::is_letter(opener) {
            return;
        }
        self.i += 1;
        while self.i < self.count()
            && self.b[self.i] != opener
            && self.b[self.i] != NEWLINE
            && self.b[self.i] != CARRIAGE_RETURN
        {
            self.i += 1;
        }
        if self.i < self.count() && self.b[self.i] == opener {
            self.i += 1;
        }
    }

    // MARK: Groups

    /// `{name}` after optional spaces: (nameStart, nameEnd, after).
    fn group_name(&self) -> Option<(usize, usize, usize)> {
        let mut j = self.i;
        while j < self.count() && (self.b[j] == SPACE || self.b[j] == TAB) {
            j += 1;
        }
        if j >= self.count() || self.b[j] != LBRACE {
            return None;
        }
        let start = j + 1;
        let mut end = start;
        while end < self.count()
            && self.b[end] != RBRACE
            && self.b[end] != NEWLINE
            && self.b[end] != CARRIAGE_RETURN
            && self.b[end] != LBRACE
        {
            end += 1;
        }
        if end < self.count() && self.b[end] == RBRACE {
            Some((start, end, end + 1))
        } else {
            None
        }
    }

    /// Balanced `{…}` at `i` (escapes and comments respected):
    /// (contentStart, contentEnd, after).
    fn balanced_group(&self) -> Option<(usize, usize, usize)> {
        if self.i >= self.count() || self.b[self.i] != LBRACE {
            return None;
        }
        let mut j = self.i + 1;
        let mut depth = 1i64;
        while j < self.count() {
            match self.b[j] {
                BACKSLASH => {
                    j += 2;
                    while j < self.count() && self.b[j] & 0xC0 == 0x80 {
                        j += 1;
                    }
                    continue;
                }
                PERCENT => {
                    while j < self.count() && self.b[j] != NEWLINE && self.b[j] != CARRIAGE_RETURN {
                        j += 1;
                    }
                    continue;
                }
                LBRACE => depth += 1,
                RBRACE => {
                    depth -= 1;
                    if depth == 0 {
                        return Some((self.i + 1, j, j + 1));
                    }
                }
                _ => {}
            }
            j += 1;
        }
        None
    }

    /// `[…]` with braces protecting nested brackets. False when unterminated.
    fn skip_bracket_group(&mut self) -> bool {
        let Some((_, _, end)) = self.bracket_group() else { return false };
        self.i = end;
        true
    }

    fn bracket_group(&self) -> Option<(usize, usize, usize)> {
        if self.i >= self.count() || self.b[self.i] != LBRACKET {
            return None;
        }
        let mut j = self.i + 1;
        let mut depth = 0i64;
        while j < self.count() {
            match self.b[j] {
                BACKSLASH => {
                    j += 2;
                    continue;
                }
                LBRACE => depth += 1,
                RBRACE => depth -= 1,
                RBRACKET if depth == 0 => return Some((self.i + 1, j, j + 1)),
                NEWLINE if self.blank_line_end(j).is_some() => return None,
                _ => {}
            }
            j += 1;
        }
        None
    }

    fn trimmed(&self, start: usize, end: usize) -> String {
        let mut lo = start;
        let mut hi = end;
        while lo < hi && matches!(self.b[lo], SPACE | TAB | NEWLINE | CARRIAGE_RETURN) {
            lo += 1;
        }
        while hi > lo && matches!(self.b[hi - 1], SPACE | TAB | NEWLINE | CARRIAGE_RETURN) {
            hi -= 1;
        }
        self.string(lo, hi)
    }

    /// Identifier-style argument text (`\input`, `\usepackage`, dir/file):
    /// `%` comments drop the rest of their line (the break itself stays,
    /// matching TeX), `\%` survives, then edges are trimmed.
    fn identifier(&self, start: usize, end: usize) -> String {
        let mut result: Vec<u8> = Vec::with_capacity(end - start);
        let mut j = start;
        while j < end {
            match self.b[j] {
                BACKSLASH => {
                    result.push(BACKSLASH);
                    j += 1;
                    if j < end {
                        result.push(self.b[j]);
                        j += 1;
                        while j < end && self.b[j] & 0xC0 == 0x80 {
                            result.push(self.b[j]);
                            j += 1;
                        }
                    }
                }
                PERCENT => {
                    while j < end && self.b[j] != NEWLINE && self.b[j] != CARRIAGE_RETURN {
                        j += 1;
                    }
                }
                byte => {
                    result.push(byte);
                    j += 1;
                }
            }
        }
        String::from_utf8_lossy(&result)
            .trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
            .to_string()
    }

    // MARK: Definitions

    /// `\name` (control word or symbol) at `i`, or nil.
    fn control_sequence(&mut self) -> Option<String> {
        if self.i >= self.count() || self.b[self.i] != BACKSLASH || self.i + 1 >= self.count() {
            return None;
        }
        let start = self.i;
        self.i += 1;
        if Self::is_letter(self.b[self.i]) {
            let _ = self.read_letters();
        } else {
            self.skip_symbol();
        }
        Some(self.string(start, self.i))
    }

    /// `{\name}` or `\name`.
    fn defined_name(&mut self) -> Option<String> {
        if self.i < self.count() && self.b[self.i] == LBRACE {
            self.i += 1;
            self.skip_inter_argument_space();
            let name = self.control_sequence()?;
            self.skip_inter_argument_space();
            if self.i < self.count() && self.b[self.i] == RBRACE {
                self.i += 1;
                return Some(name);
            }
            return None;
        }
        self.control_sequence()
    }

    /// `[n]` then, when present, `[default]` — as normalized text.
    fn argument_spec(&mut self) -> Option<String> {
        let mut spec = String::new();
        self.skip_inter_argument_space();
        if self.i >= self.count() || self.b[self.i] != LBRACKET {
            return Some(spec);
        }
        let (count_start, count_end, count_after) = self.bracket_group()?;
        let arity = self.trimmed(count_start, count_end);
        if arity.len() != 1 || !arity.as_bytes()[0].is_ascii_digit() {
            return None;
        }
        spec += "[";
        spec += &arity;
        spec += "]";
        self.i = count_after;
        self.skip_inter_argument_space();
        if self.i < self.count() && self.b[self.i] == LBRACKET {
            let (default_start, default_end, default_after) = self.bracket_group()?;
            spec += "[";
            spec += &self.string(default_start, default_end);
            spec += "]";
            self.i = default_after;
        }
        Some(spec)
    }

    /// A `{body}` group after optional space, as written (braces included).
    fn body_group(&mut self) -> Option<String> {
        self.skip_inter_argument_space();
        let (_, _, end) = self.balanced_group()?;
        let text = self.string(self.i, end);
        self.i = end;
        Some(text)
    }

    fn parse_command_definition(&mut self, start: usize, kind: MathDefinitionKind) {
        let resume = self.i;
        if self.i < self.count() && self.b[self.i] == STAR {
            self.i += 1;
        }
        self.skip_inter_argument_space();
        let parsed = self
            .defined_name()
            .and_then(|name| self.argument_spec().map(|spec| (name, spec)))
            .and_then(|(name, spec)| self.body_group().map(|body| (name, spec, body)));
        let Some((name, spec, body)) = parsed else {
            self.i = resume;
            return;
        };
        let command = if kind == MathDefinitionKind::RenewCommand { "\\renewcommand" } else { "\\newcommand" };
        self.definitions.push(MathDefinition {
            kind,
            math_jax_source: format!("{command}{{{name}}}{spec}{body}"),
            name,
            original_source: self.string(start, self.i),
            range: SourceRange::lexer_range(start, self.i - start),
        });
    }

    fn parse_operator_definition(&mut self, start: usize) {
        let resume = self.i;
        let mut starred = false;
        if self.i < self.count() && self.b[self.i] == STAR {
            starred = true;
            self.i += 1;
        }
        self.skip_inter_argument_space();
        let parsed = self
            .defined_name()
            .and_then(|name| self.body_group().map(|body| (name, body)));
        let Some((name, body)) = parsed else {
            self.i = resume;
            return;
        };
        let star = if starred { "*" } else { "" };
        self.definitions.push(MathDefinition {
            kind: MathDefinitionKind::DeclareMathOperator,
            math_jax_source: format!("\\DeclareMathOperator{star}{{{name}}}{body}"),
            name,
            original_source: self.string(start, self.i),
            range: SourceRange::lexer_range(start, self.i - start),
        });
    }

    fn parse_environment_definition(&mut self, start: usize, kind: MathDefinitionKind) {
        let resume = self.i;
        if self.i < self.count() && self.b[self.i] == STAR {
            self.i += 1;
        }
        self.skip_inter_argument_space();
        let Some((name_start, name_end, name_after)) = self.balanced_group() else {
            self.i = resume;
            return;
        };
        let name = self.trimmed(name_start, name_end);
        self.i = name_after;
        let parsed = if name.is_empty() {
            None
        } else {
            self.argument_spec().and_then(|spec| {
                self.body_group().and_then(|begin| {
                    self.body_group().map(|end| (spec, begin, end))
                })
            })
        };
        let Some((spec, begin, end)) = parsed else {
            self.i = resume;
            return;
        };
        let command = if kind == MathDefinitionKind::RenewEnvironment {
            "\\renewenvironment"
        } else {
            "\\newenvironment"
        };
        self.definitions.push(MathDefinition {
            kind,
            math_jax_source: format!("{command}{{{name}}}{spec}{begin}{end}"),
            name,
            original_source: self.string(start, self.i),
            range: SourceRange::lexer_range(start, self.i - start),
        });
    }
}

// MARK: Context (EquationPreviewContext.swift)

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MathIncludeKey {
    pub from_file_id: String,
    pub target: String,
}

impl MathIncludeKey {
    pub fn new(from_file_id: impl Into<String>, target: impl Into<String>) -> Self {
        Self { from_file_id: from_file_id.into(), target: target.into() }
    }
}

/// An include the context could not place yet: the host tries the
/// candidates in order and reports the first existing file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MathIncludeRequest {
    pub key: MathIncludeKey,
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MathPreviewContext {
    /// MathJax sources, applied in order.
    pub definitions: Vec<String>,
    /// Opaque identity of `definitions` (cumulative FNV-1a).
    pub key: String,
    /// Bounds dropped later definitions.
    pub truncated: bool,
}

impl MathPreviewContext {
    pub fn empty() -> Self {
        Self { definitions: Vec::new(), key: MathPreviewHash::hex(MathPreviewHash::BASIS), truncated: false }
    }
}

/// Cumulative FNV-1a 64 with a 0x1F separator step between entries.
pub struct MathPreviewHash;

impl MathPreviewHash {
    pub const BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub fn combine(hash: u64, text: &str) -> u64 {
        let mut value = hash;
        for byte in text.as_bytes() {
            value ^= *byte as u64;
            value = value.wrapping_mul(Self::PRIME);
        }
        // Unit separator keeps ["ab","c"] and ["a","bc"] apart.
        value ^= 0x1F;
        value.wrapping_mul(Self::PRIME)
    }

    /// 16-digit lowercase hex.
    pub fn hex(value: u64) -> String {
        format!("{value:016x}")
    }
}

#[derive(Debug, Clone)]
pub struct MathProjectContext {
    pub entries: Vec<MathContextEntry>,
    /// `hashes[k]` identifies the first `k` entries.
    hashes: Vec<u64>,
    /// Per visited file: its own definitions/includes as (offset, entry index).
    checkpoints: HashMap<String, Vec<(usize, usize)>>,
    segment_ends: HashMap<String, usize>,
    pub truncated: bool,
    /// Includes with no known resolution — the host should resolve them.
    pub unresolved: Vec<MathIncludeRequest>,
    pub root_file_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MathContextEntry {
    pub file_id: String,
    pub definition: MathDefinition,
}

impl MathProjectContext {
    /// Bounds keep a pathological project from turning every render into a
    /// megabyte of definitions.
    pub const MAXIMUM_DEFINITIONS: usize = 4096;
    pub const MAXIMUM_DEFINITION_BYTES: usize = 512 * 1024;
    pub const MAXIMUM_FILES: usize = 256;

    /// Depth-first in document order from `root_file_id`; `active_file_id`
    /// is appended after the root's tree when the root never reaches it (a
    /// chapter opened without its main file resolved, or a standalone
    /// file). `resolutions[key] == Some(None)` marks a known-missing target.
    pub fn new(
        root_file_id: &str,
        active_file_id: Option<&str>,
        active_scan: Option<&MathSourceScan>,
        scans: &HashMap<String, MathSourceScan>,
        resolutions: &HashMap<MathIncludeKey, Option<String>>,
    ) -> Self {
        let mut context = Self {
            entries: Vec::new(),
            hashes: vec![MathPreviewHash::BASIS],
            checkpoints: HashMap::new(),
            segment_ends: HashMap::new(),
            truncated: false,
            unresolved: Vec::new(),
            root_file_id: root_file_id.to_string(),
        };
        let mut state = VisitState::default();
        context.visit(root_file_id, active_file_id, active_scan, scans, resolutions, &mut state);
        if let Some(active) = active_file_id {
            context.visit(active, active_file_id, active_scan, scans, resolutions, &mut state);
        }
        context
    }

    fn append(&mut self, definition: &MathDefinition, file_id: &str, state: &mut VisitState) {
        if self.truncated {
            return;
        }
        match definition.kind {
            MathDefinitionKind::ProvideCommand => {
                // LaTeX keeps an existing definition; only a first one applies.
                if !state.defined.insert(definition.name.clone()) {
                    return;
                }
            }
            MathDefinitionKind::NewCommand
            | MathDefinitionKind::RenewCommand
            | MathDefinitionKind::DeclareMathOperator => {
                state.defined.insert(definition.name.clone());
            }
            MathDefinitionKind::NewEnvironment | MathDefinitionKind::RenewEnvironment => {
                // environments share no namespace with commands
            }
        }
        state.bytes += definition.math_jax_source.len();
        if self.entries.len() >= Self::MAXIMUM_DEFINITIONS || state.bytes > Self::MAXIMUM_DEFINITION_BYTES {
            self.truncated = true;
            return;
        }
        self.entries.push(MathContextEntry {
            file_id: file_id.to_string(),
            definition: definition.clone(),
        });
        let last = *self.hashes.last().unwrap();
        self.hashes.push(MathPreviewHash::combine(last, &definition.math_jax_source));
    }

    fn visit(
        &mut self,
        file_id: &str,
        active_file_id: Option<&str>,
        active_scan: Option<&MathSourceScan>,
        scans: &HashMap<String, MathSourceScan>,
        resolutions: &HashMap<MathIncludeKey, Option<String>>,
        state: &mut VisitState,
    ) {
        if state.visited.len() >= Self::MAXIMUM_FILES || !state.visited.insert(file_id.to_string()) {
            return;
        }
        // The active file's in-memory scan wins over any on-disk copy.
        let Some(scan) = (if Some(file_id) == active_file_id { active_scan } else { None })
            .or_else(|| scans.get(file_id)) else { return };
        let mut marks: Vec<(usize, usize)> = Vec::new();
        let mut definition_index = 0;
        let mut include_index = 0;
        while definition_index < scan.definitions.len() || include_index < scan.includes.len() {
            let take_definition = include_index >= scan.includes.len()
                || (definition_index < scan.definitions.len()
                    && scan.definitions[definition_index].range.utf8_offset
                        < scan.includes[include_index].utf8_offset as i64);
            if take_definition {
                let definition = scan.definitions[definition_index].clone();
                definition_index += 1;
                marks.push((definition.range.utf8_offset as usize, self.entries.len()));
                self.append(&definition, file_id, state);
            } else {
                let include = scan.includes[include_index].clone();
                include_index += 1;
                marks.push((include.utf8_offset, self.entries.len()));
                let key = MathIncludeKey::new(file_id, &include.target);
                if let Some(resolution) = resolutions.get(&key) {
                    if let Some(child) = resolution {
                        self.visit(child, active_file_id, active_scan, scans, resolutions, state);
                    }
                } else if state.requested.insert(key.clone()) {
                    self.unresolved.push(MathIncludeRequest {
                        candidates: MathIncludePaths::candidates(&include.target, file_id, &self.root_file_id),
                        key,
                    });
                }
            }
        }
        self.checkpoints.insert(file_id.to_string(), marks);
        self.segment_ends.insert(file_id.to_string(), self.entries.len());
    }

    /// Number of entries TeX has read before `utf8_offset` in `file_id`.
    pub fn visible_count(&self, file_id: &str, utf8_offset: usize) -> usize {
        let Some(marks) = self.checkpoints.get(file_id) else { return self.entries.len() };
        let lo = marks.partition_point(|&(offset, _)| offset < utf8_offset);
        if lo < marks.len() {
            marks[lo].1
        } else {
            *self.segment_ends.get(file_id).unwrap_or(&self.entries.len())
        }
    }

    /// The fast-preview context for a region starting at `utf8_offset`.
    pub fn context(&self, file_id: &str, utf8_offset: usize) -> MathPreviewContext {
        let visible = self.visible_count(file_id, utf8_offset);
        MathPreviewContext {
            definitions: self.entries[..visible].iter().map(|e| e.definition.math_jax_source.clone()).collect(),
            key: MathPreviewHash::hex(self.hashes[visible]),
            truncated: self.truncated,
        }
    }

    /// Definitions the exact document must restate after `\begin{document}`:
    /// those TeX reads after the root preamble and before the region. The
    /// root's preamble (with its own `\input`s) runs verbatim, so earlier
    /// entries are already in effect there.
    pub fn body_definitions(
        &self,
        file_id: &str,
        utf8_offset: usize,
        root_document_begin: Option<usize>,
    ) -> Vec<String> {
        let visible = self.visible_count(file_id, utf8_offset);
        let start = root_document_begin
            .map(|offset| self.visible_count(&self.root_file_id, offset))
            .unwrap_or(0);
        if start >= visible {
            return Vec::new();
        }
        self.entries[start..visible].iter().map(|e| e.definition.original_source.clone()).collect()
    }
}

#[derive(Default)]
struct VisitState {
    defined: HashSet<String>,
    bytes: usize,
    visited: HashSet<String>,
    requested: HashSet<MathIncludeKey>,
}

/// Include-target resolution candidates, TeX-style: relative to the main
/// document's directory first (TeX's working directory), then to the
/// including file's; `.tex` is implied for extensionless targets.
pub struct MathIncludePaths;

impl MathIncludePaths {
    pub fn candidates(target: &str, from: &str, root: &str) -> Vec<String> {
        let name = if Self::has_extension(target) { target.to_string() } else { format!("{target}.tex") };
        if name.starts_with('/') {
            return vec![Self::normalize(&name)];
        }
        let mut result: Vec<String> = Vec::new();
        for directory in [Self::directory_name(root), Self::directory_name(from)] {
            let path = Self::normalize(&if directory.is_empty() { name.clone() } else { format!("{directory}/{name}") });
            if !result.contains(&path) {
                result.push(path);
            }
        }
        result
    }

    fn has_extension(path: &str) -> bool {
        let last = path.rsplit('/').next().unwrap_or("");
        match last.rfind('.') {
            Some(dot) => dot != 0,
            None => false,
        }
    }

    fn directory_name(path: &str) -> String {
        match path.rfind('/') {
            None => String::new(),
            Some(0) => "/".to_string(),
            Some(slash) => path[..slash].to_string(),
        }
    }

    fn normalize(path: &str) -> String {
        let absolute = path.starts_with('/');
        let mut components: Vec<&str> = Vec::new();
        for component in path.split('/').filter(|c| !c.is_empty()) {
            if component == "." {
                continue;
            }
            if component == ".." && matches!(components.last(), Some(&last) if last != "..") {
                components.pop();
            } else if component == ".." && absolute {
                continue;
            } else {
                components.push(component);
            }
        }
        let joined = components.join("/");
        if absolute {
            format!("/{joined}")
        } else {
            joined
        }
    }
}

/// The minimal document for the optional exact TeX preview: the project's
/// own preamble (or a plain AMS one when the file has none), definitions TeX
/// would have read before the region, then the region exactly as written.
pub struct ExactEquationDocument;

impl ExactEquationDocument {
    pub const FALLBACK_PREAMBLE: &'static str =
        "\\documentclass{article}\n\\usepackage{amsmath,amssymb}\n";

    pub fn make(
        preamble: Option<&str>,
        preamble_definitions: &[String],
        body_definitions: &[String],
        region: &str,
    ) -> String {
        let mut document = preamble.unwrap_or(Self::FALLBACK_PREAMBLE).to_string();
        if !document.ends_with('\n') {
            document.push('\n');
        }
        for definition in preamble_definitions {
            document.push_str(definition);
            document.push('\n');
        }
        document.push_str("\\begin{document}\n\\thispagestyle{empty}\n");
        for definition in body_definitions {
            document.push_str(definition);
            document.push('\n');
        }
        document.push_str("\\noindent\n");
        document.push_str(region);
        document.push_str("\n\\end{document}\n");
        document
    }
}
