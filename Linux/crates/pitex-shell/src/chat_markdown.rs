//! Assistant-message Markdown for the agent transcript: a small CommonMark
//! subset (GFM tables, fenced code) plus TeX math, parsed into the block
//! model the GTK transcript draws. The Mac host has the same model and
//! cases in `Packages/TexApp/Sources/AppPorts/AgentMarkdown.swift`; keep
//! the two in step.
//!
//! The renderer is host-side, so every backend (pi, Claude Code,
//! Antigravity, ...) gets it from plain `text` deltas.

use std::cell::{Cell, RefCell};
use std::path::{Component, Path, PathBuf};

/// Characters drawn per message; the rest is cut with a note (Paseo's
/// assistant-message render limit).
pub const RENDER_LIMIT: usize = 32_000;

/// Quotes, lists and inline links or emphasis nest at most this deep; deeper
/// markers stay text, so `>>>>…` cannot overflow the parser or the GTK tree.
const MAX_DEPTH: usize = 16;

/// Inline scan steps per parse: unclosed openers (`[[[…`, `*a *a …`) rescan
/// the rest of a paragraph each, so pathological text stays literal past this
/// instead of costing tens of milliseconds per streaming tick.
const SCAN_FUEL: usize = 2_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading(u8, Vec<Inline>),
    List { ordered: bool, start: u64, items: Vec<Vec<Block>> },
    Quote(Vec<Block>),
    Code { language: String, text: String },
    /// Display math (`$$…$$`, `\[…\]`, a ```math fence), TeX without delimiters.
    Math(String),
    /// Rows of cells; the first row is the header.
    Table(Vec<Vec<Vec<Inline>>>),
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Code(String),
    Link { url: String, children: Vec<Inline> },
    /// Inline math (`$…$`, `\(…\)`), TeX without delimiters.
    Math(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
    /// The message was longer than [`RENDER_LIMIT`] and was cut.
    pub truncated: bool,
}

/// Where a link in a message goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    Web(String),
    /// A file inside the project, with an optional 1-based line.
    File { path: PathBuf, line: Option<usize> },
    /// Anything else (other schemes, paths outside the project): not clickable.
    Unsupported,
}

thread_local! {
    /// The last streaming head and its blocks: closed blocks are parsed once.
    static HEAD_CACHE: RefCell<Option<(String, Vec<Block>)>> = const { RefCell::new(None) };
    static FUEL: Cell<usize> = const { Cell::new(SCAN_FUEL) };
    static INLINE_DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// False once this parse's scan budget is spent.
fn has_fuel() -> bool {
    FUEL.with(|f| f.get() > 0)
}

fn spend(steps: usize) {
    FUEL.with(|f| f.set(f.get().saturating_sub(steps)));
}

/// Parses a message. While `streaming`, the closed head is parsed once and
/// reused; only the open tail is parsed again, and an unclosed fence or
/// display-math delimiter in it stays literal text until it closes.
pub fn parse(text: &str, streaming: bool) -> Document {
    let (text, truncated) = cap(text);
    FUEL.with(|f| f.set(SCAN_FUEL));
    if !streaming {
        return Document { blocks: parse_blocks(&lines(text), false, 0), truncated };
    }
    let (head, tail) = text.split_at(stable_prefix_len(text));
    let mut blocks = HEAD_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        match cache.as_ref() {
            Some((cached, blocks)) if cached == head => blocks.clone(),
            _ => {
                let blocks = parse_blocks(&lines(head), false, 0);
                *cache = Some((head.to_string(), blocks.clone()));
                blocks
            }
        }
    });
    blocks.extend(parse_blocks(&lines(tail), true, 0));
    Document { blocks, truncated }
}

/// The first [`RENDER_LIMIT`] characters, and whether anything was cut.
pub fn cap(text: &str) -> (&str, bool) {
    match text.char_indices().nth(RENDER_LIMIT) {
        Some((index, _)) => (&text[..index], true),
        None => (text, false),
    }
}

fn lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.strip_suffix('\n').unwrap_or(text).split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect()
}

/// Byte length of the closed head of a streaming message: everything before
/// the last top-level block that starts after a blank line, outside any
/// fence or display math. Only complete lines count (Paseo
/// `splitStreamingMarkdown`).
pub fn stable_prefix_len(text: &str) -> usize {
    let complete = text.rfind('\n').map_or(0, |i| i + 1);
    let mut boundary = 0;
    let mut offset = 0;
    let mut fence: Option<(u8, usize)> = None;
    let mut math_closer: Option<&str> = None;
    let mut previous_blank = false;
    for line in text[..complete].split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let line = line.trim_end_matches(['\n', '\r']);
        if let Some((ch, len)) = fence {
            if is_fence_close(line, ch, len) {
                fence = None;
            }
            continue;
        }
        let blank = line.trim().is_empty();
        if let Some(closer) = math_closer {
            // Display math cannot span a blank line (TeX rejects it): it was text.
            if !blank {
                if closes_math(line, closer) {
                    math_closer = None;
                }
                continue;
            }
            math_closer = None;
        }
        if previous_blank && !blank && !line.starts_with([' ', '\t']) {
            boundary = start;
        }
        previous_blank = blank;
        let (indent, rest) = indentation(line);
        if indent > 3 {
            continue;
        }
        if let Some((ch, len, _)) = fence_open(rest) {
            fence = Some((ch, len));
        } else if let Some((opener, closer)) = math_delimiters(rest) {
            if math_body(rest).len() < opener.len() + closer.len() || !closes_math(rest, closer) {
                math_closer = Some(closer);
            }
        }
    }
    boundary
}

fn indentation(line: &str) -> (usize, &str) {
    let rest = line.trim_start_matches(' ');
    (line.len() - rest.len(), rest)
}

/// ``` or ~~~ (3+), its length and the info string's first word.
fn fence_open(rest: &str) -> Option<(u8, usize, &str)> {
    let ch = *rest.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = rest.bytes().take_while(|&b| b == ch).count();
    let info = rest[len..].trim();
    // A backtick fence's info string cannot contain a backtick (that is inline code).
    if len < 3 || (ch == b'`' && info.contains('`')) {
        return None;
    }
    Some((ch, len, info.split_whitespace().next().unwrap_or("")))
}

fn is_fence_close(line: &str, ch: u8, len: usize) -> bool {
    let (indent, rest) = indentation(line);
    let run = rest.bytes().take_while(|&b| b == ch).count();
    indent <= 3 && run >= len && rest[run..].trim().is_empty()
}

/// `$$` or `\[` at the start of a line, with the closer to look for.
fn math_delimiters(rest: &str) -> Option<(&'static str, &'static str)> {
    if rest.starts_with("$$") {
        Some(("$$", "$$"))
    } else if rest.starts_with("\\[") {
        Some(("\\[", "\\]"))
    } else {
        None
    }
}

/// A display-math line without trailing space or sentence punctuation
/// (`$$.` closes like `$$`).
fn math_body(line: &str) -> &str {
    line.trim_end().trim_end_matches(['.', ',', ';', ':']).trim_end()
}

fn closes_math(line: &str, closer: &str) -> bool {
    math_body(line).ends_with(closer)
}

fn is_rule(rest: &str) -> bool {
    let mut chars = rest.chars().filter(|c| !c.is_whitespace());
    let Some(first) = chars.next() else { return false };
    matches!(first, '-' | '*' | '_') && rest.chars().filter(|&c| c == first).count() >= 3 && chars.all(|c| c == first)
}

fn heading(rest: &str) -> Option<(u8, &str)> {
    let level = rest.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let body = &rest[level..];
    if !body.is_empty() && !body.starts_with([' ', '\t']) {
        return None;
    }
    let body = body.trim();
    // An optional closing run of #'s after a space.
    let trimmed = body.trim_end_matches('#');
    let body = if trimmed.is_empty() || trimmed.ends_with(' ') { trimmed.trim_end() } else { body };
    Some((level as u8, body))
}

/// A list marker: (indent, ordered, number, content column).
fn list_marker(line: &str) -> Option<(usize, bool, u64, usize)> {
    let (indent, rest) = indentation(line);
    let bytes = rest.as_bytes();
    let (ordered, number, marker_len) = match bytes.first()? {
        b'-' | b'*' | b'+' => (false, 0, 1),
        b'0'..=b'9' => {
            let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
            if digits > 9 || !matches!(bytes.get(digits), Some(b'.') | Some(b')')) {
                return None;
            }
            (true, rest[..digits].parse().ok()?, digits + 1)
        }
        _ => return None,
    };
    let after = &rest[marker_len..];
    if after.is_empty() {
        return Some((indent, ordered, number, indent + marker_len + 1));
    }
    let spaces = after.bytes().take_while(|&b| b == b' ').count();
    if spaces == 0 {
        return None;
    }
    // Five or more spaces: the content starts after one (CommonMark).
    let spaces = if spaces > 4 { 1 } else { spaces };
    Some((indent, ordered, number, indent + marker_len + spaces))
}

fn is_delimiter_row(line: &str) -> bool {
    let row = line.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    let row = row.strip_suffix('|').unwrap_or(row);
    !row.trim().is_empty()
        && row.split('|').all(|cell| {
            let cell = cell.trim();
            let core = cell.strip_prefix(':').unwrap_or(cell);
            let core = core.strip_suffix(':').unwrap_or(core);
            !core.is_empty() && core.bytes().all(|b| b == b'-')
        })
}

/// A table needs a delimiter row with as many cells as its header (GFM).
fn starts_table(line: &str, next: Option<&&str>) -> bool {
    line.contains('|') && next.is_some_and(|next| is_delimiter_row(next) && split_cells(next).len() == split_cells(line).len())
}

fn table_cells(line: &str) -> Vec<Vec<Inline>> {
    split_cells(line).into_iter().map(|cell| inlines(&cell.trim().replace("\\|", "|"))).collect()
}

fn split_cells(line: &str) -> Vec<&str> {
    let row = line.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    let row = if row.ends_with('|') && !row.ends_with("\\|") { &row[..row.len() - 1] } else { row };
    let mut cells = Vec::new();
    let (mut start, mut in_code, mut escaped) = (0, false, false);
    for (i, b) in row.bytes().enumerate() {
        match b {
            b'\\' if !escaped => {
                escaped = true;
                continue;
            }
            b'`' => in_code = !in_code,
            b'|' if !escaped && !in_code => {
                cells.push(&row[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        escaped = false;
    }
    cells.push(&row[start..]);
    cells
}

/// A line that starts a block other than a paragraph.
fn starts_block(line: &str) -> bool {
    let (indent, rest) = indentation(line);
    indent <= 3
        && (fence_open(rest).is_some()
            || heading(rest).is_some()
            || rest.starts_with('>')
            || is_rule(rest)
            || math_delimiters(rest).is_some()
            || list_marker(line).is_some())
}

/// Drops up to `count` leading spaces.
fn dedent(line: &str, count: usize) -> &str {
    let spaces = line.bytes().take(count).take_while(|&b| b == b' ').count();
    &line[spaces..]
}

fn parse_blocks(lines: &[&str], streaming: bool, depth: usize) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if !paragraph.is_empty() {
            let text = paragraph.iter().map(|l| l.trim()).collect::<Vec<_>>().join("\n");
            blocks.push(Block::Paragraph(inlines(&text)));
            paragraph.clear();
        }
    };
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let (indent, rest) = indentation(line);
        if rest.trim().is_empty() {
            flush(&mut paragraph, &mut blocks);
            i += 1;
            continue;
        }
        if indent > 3 {
            paragraph.push(line);
            i += 1;
            continue;
        }
        if let Some((ch, len, info)) = fence_open(rest) {
            let close = (i + 1..lines.len()).find(|&j| is_fence_close(lines[j], ch, len));
            let language = info.to_ascii_lowercase();
            if close.is_none() && streaming && language == "math" {
                // Still streaming: a math fence stays literal until it closes
                // (no pdflatex on half a formula); code runs to the end.
                flush(&mut paragraph, &mut blocks);
                blocks.push(Block::Paragraph(vec![Inline::Text(lines[i..].join("\n"))]));
                break;
            }
            flush(&mut paragraph, &mut blocks);
            let end = close.unwrap_or(lines.len());
            let text = lines[i + 1..end].iter().map(|l| dedent(l, indent)).collect::<Vec<_>>().join("\n");
            blocks.push(if language == "math" {
                Block::Math(text.trim().to_string())
            } else {
                Block::Code { language: info.to_string(), text }
            });
            i = end + 1;
            continue;
        }
        if let Some((opener, closer)) = math_delimiters(rest) {
            let first = math_body(rest);
            let single = first.len() >= opener.len() + closer.len() + 1 && first.ends_with(closer);
            // Display math ends at its closer, and never spans a blank line.
            let block = (i + 1..lines.len()).take_while(|&j| !lines[j].trim().is_empty());
            let close = if single { Some(i) } else { block.clone().find(|&j| closes_math(lines[j], closer)) };
            let open_to_end = block.last().map_or(i, |j| j) + 1 == lines.len();
            // `$$a$$ and $$b$$` on one line is inline math in a paragraph.
            let inline = single && first[opener.len()..first.len() - closer.len()].contains(opener);
            match close {
                Some(end) if !inline => {
                    flush(&mut paragraph, &mut blocks);
                    let body = lines[i..=end].iter().map(|l| l.trim()).collect::<Vec<_>>().join("\n");
                    let body = math_body(&body);
                    let tex = &body[opener.len()..body.len() - closer.len()];
                    blocks.push(Block::Math(tex.trim().to_string()));
                    i = end + 1;
                    continue;
                }
                None if streaming && open_to_end => {
                    flush(&mut paragraph, &mut blocks);
                    blocks.push(Block::Paragraph(vec![Inline::Text(lines[i..].join("\n"))]));
                    break;
                }
                _ => {}
            }
        }
        if let Some((level, body)) = heading(rest) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Heading(level, inlines(body)));
            i += 1;
            continue;
        }
        if is_rule(rest) {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }
        if rest.starts_with('>') && depth < MAX_DEPTH {
            flush(&mut paragraph, &mut blocks);
            let mut inner = Vec::new();
            while i < lines.len() {
                let (indent, rest) = indentation(lines[i]);
                let Some(body) = rest.strip_prefix('>').filter(|_| indent <= 3) else { break };
                inner.push(body.strip_prefix(' ').unwrap_or(body));
                i += 1;
            }
            blocks.push(Block::Quote(parse_blocks(&inner, streaming, depth + 1)));
            continue;
        }
        if let Some((_, ordered, start, _)) = list_marker(line).filter(|_| depth < MAX_DEPTH) {
            flush(&mut paragraph, &mut blocks);
            let mut items = Vec::new();
            while let Some((_, item_ordered, _, column)) = lines.get(i).and_then(|l| list_marker(l)) {
                if item_ordered != ordered {
                    break;
                }
                let mut item = vec![lines[i].get(column..).unwrap_or("")];
                i += 1;
                while i < lines.len() {
                    let line = lines[i];
                    if line.trim().is_empty() {
                        // A blank line stays in the item only when indented content follows.
                        let next = (i + 1..lines.len()).find(|&j| !lines[j].trim().is_empty());
                        match next {
                            Some(j) if indentation(lines[j]).0 >= column => {
                                item.push("");
                                i += 1;
                            }
                            Some(j) if list_marker(lines[j]).is_some_and(|m| m.1 == ordered && m.0 < column) => i = j,
                            _ => break,
                        }
                        if i < lines.len() && list_marker(lines[i]).is_some_and(|m| m.0 < column) {
                            break;
                        }
                        continue;
                    }
                    if indentation(line).0 >= column {
                        item.push(&line[column..]);
                    } else if list_marker(line).is_some() || starts_block(line) || item.last() == Some(&"") {
                        break;
                    } else {
                        // Lazy continuation of the item's paragraph.
                        item.push(line.trim_start());
                    }
                    i += 1;
                }
                items.push(parse_blocks(&item, streaming, depth + 1));
                // The list ends at a blank line not followed by a sibling item.
                if lines.get(i).is_some_and(|l| l.trim().is_empty()) {
                    break;
                }
            }
            blocks.push(Block::List { ordered, start, items });
            continue;
        }
        if starts_table(line, lines.get(i + 1)) {
            flush(&mut paragraph, &mut blocks);
            let mut rows = vec![table_cells(line)];
            i += 2;
            while i < lines.len() && !lines[i].trim().is_empty() && lines[i].contains('|') {
                rows.push(table_cells(lines[i]));
                i += 1;
            }
            blocks.push(Block::Table(rows));
            continue;
        }
        paragraph.push(line);
        i += 1;
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}

fn push_text(out: &mut Vec<Inline>, text: &mut String) {
    if text.is_empty() {
        return;
    }
    if let Some(Inline::Text(last)) = out.last_mut() {
        last.push_str(text);
    } else {
        out.push(Inline::Text(text.clone()));
    }
    text.clear();
}

/// Inline content: emphasis, code spans, links, autolinks and math.
/// Math is recognised outside code spans only; `\$` and currency such as
/// `$5 and $10` stay literal (a `$` before a digit or a space opens nothing).
pub fn parse_inlines(s: &str) -> Vec<Inline> {
    FUEL.with(|f| f.set(SCAN_FUEL));
    inlines(s)
}

/// Inline content of a link label or emphasis; past [`MAX_DEPTH`] it stays text.
fn nested(s: &str) -> Vec<Inline> {
    let depth = INLINE_DEPTH.with(Cell::get);
    if depth >= MAX_DEPTH {
        return vec![Inline::Text(s.to_string())];
    }
    INLINE_DEPTH.with(|d| d.set(depth + 1));
    let out = inlines(s);
    INLINE_DEPTH.with(|d| d.set(depth));
    out
}

fn inlines(s: &str) -> Vec<Inline> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    while i < b.len() {
        let parsed: Option<(Inline, usize)> = match b[i] {
            b'\\' => match b.get(i + 1) {
                Some(b'(') => delimited_math(s, i + 2, "\\)"),
                Some(b'[') => delimited_math(s, i + 2, "\\]"),
                _ => None,
            },
            b'`' => code_span(s, i),
            b'$' => dollar_math(s, i),
            b'*' | b'_' => emphasis(s, i),
            b'[' => link(s, i),
            b'<' => autolink(s, i),
            b'h' if (i == 0 || !b[i - 1].is_ascii_alphanumeric()) && (s[i..].starts_with("https://") || s[i..].starts_with("http://")) => {
                bare_url(s, i)
            }
            _ => None,
        };
        if let Some((node, next)) = parsed {
            push_text(&mut out, &mut text);
            out.push(node);
            i = next;
            continue;
        }
        // An image (`![alt](src)`) is not drawn: it stays its source text.
        if let Some((_, next)) = (b[i] == b'!' && b.get(i + 1) == Some(&b'[')).then(|| link(s, i + 1)).flatten() {
            text.push_str(&s[i..next]);
            i = next;
            continue;
        }
        match b[i] {
            b'\\' if b.get(i + 1).is_some_and(|n| n.is_ascii_punctuation()) => {
                text.push(b[i + 1] as char);
                i += 2;
            }
            c @ (b'`' | b'*' | b'_' | b'$') => {
                // An unmatched run stays literal as a whole.
                let run = b[i..].iter().take_while(|&&x| x == c).count();
                text.push_str(&s[i..i + run]);
                i += run;
            }
            _ => {
                let ch = s[i..].chars().next().unwrap();
                text.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    push_text(&mut out, &mut text);
    out
}

fn delimited_math(s: &str, start: usize, closer: &str) -> Option<(Inline, usize)> {
    let end = start + find(s.get(start..)?, closer)?;
    let tex = s[start..end].trim();
    (!tex.is_empty()).then(|| (Inline::Math(tex.to_string()), end + closer.len()))
}

/// `str::find` that spends fuel on what it scanned.
fn find(s: &str, needle: &str) -> Option<usize> {
    if !has_fuel() {
        return None;
    }
    let found = s.find(needle);
    spend(found.unwrap_or(s.len()));
    found
}

fn code_span(s: &str, i: usize) -> Option<(Inline, usize)> {
    if !has_fuel() {
        return None;
    }
    let b = s.as_bytes();
    let run = b[i..].iter().take_while(|&&x| x == b'`').count();
    let mut j = i + run;
    while j < b.len() {
        if b[j] == b'`' {
            let close = b[j..].iter().take_while(|&&x| x == b'`').count();
            if close == run {
                spend(j - i);
                let mut code = s[i + run..j].replace('\n', " ");
                if code.len() >= 2 && code.starts_with(' ') && code.ends_with(' ') && !code.trim().is_empty() {
                    code = code[1..code.len() - 1].to_string();
                }
                return Some((Inline::Code(code), j + close));
            }
            j += close;
        } else {
            j += 1;
        }
    }
    spend(j - i);
    None
}

fn dollar_math(s: &str, i: usize) -> Option<(Inline, usize)> {
    let b = s.as_bytes();
    if s[i..].starts_with("$$") {
        let end = i + 2 + find(&s[i + 2..], "$$")?;
        let tex = s[i + 2..end].trim();
        return (!tex.is_empty()).then(|| (Inline::Math(tex.to_string()), end + 2));
    }
    let next = *b.get(i + 1)?;
    if next.is_ascii_whitespace() || next.is_ascii_digit() || !has_fuel() {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            // Not before a digit or letter: `$5 and $10`, `$FOO$BAR` stay text (Pandoc).
            b'$' if !b[j - 1].is_ascii_whitespace() && !b.get(j + 1).is_some_and(|n| n.is_ascii_alphanumeric()) => {
                spend(j - i);
                return Some((Inline::Math(s[i + 1..j].to_string()), j + 1));
            }
            _ => j += 1,
        }
    }
    spend(j - i);
    None
}

fn emphasis(s: &str, i: usize) -> Option<(Inline, usize)> {
    let b = s.as_bytes();
    let c = b[i];
    let run = b[i..].iter().take_while(|&&x| x == c).count().min(3);
    let opener_ok = b.get(i + run).is_some_and(|n| !n.is_ascii_whitespace())
        && (c != b'_' || i == 0 || !b[i - 1].is_ascii_alphanumeric());
    if !opener_ok || !has_fuel() {
        return None;
    }
    let mut j = i + run + 1;
    while j < b.len() {
        if b[j] == b'`' {
            // Delimiters inside a code span do not close.
            if let Some((_, next)) = code_span(s, j) {
                j = next;
                continue;
            }
        }
        if b[j] != c {
            j += 1;
            continue;
        }
        let close = b[j..].iter().take_while(|&&x| x == c).count();
        let closes = close == run
            && !b[j - 1].is_ascii_whitespace()
            && (c != b'_' || !b.get(j + close).is_some_and(|n| n.is_ascii_alphanumeric()));
        if closes {
            spend(j - i);
            let inner = nested(&s[i + run..j]);
            let node = match run {
                1 => Inline::Emphasis(inner),
                2 => Inline::Strong(inner),
                _ => Inline::Strong(vec![Inline::Emphasis(inner)]),
            };
            return Some((node, j + close));
        }
        j += close;
    }
    spend(j - i);
    None
}

/// `[text](url)` starting at `i` (also the `[alt](src)` of an image).
fn link(s: &str, i: usize) -> Option<(Inline, usize)> {
    if !has_fuel() {
        return None;
    }
    let b = s.as_bytes();
    let label_end = matching(b, i, b'[', b']', false);
    let opens_dest = label_end.is_some_and(|end| b.get(end + 1) == Some(&b'('));
    let dest_end = label_end.filter(|_| opens_dest).and_then(|end| matching(b, end + 1, b'(', b')', true));
    // A failed scan ran to the end of the text (or of the line, for the destination).
    spend(match (label_end, dest_end) {
        (Some(end), None) if !opens_dest => end,
        (_, Some(end)) => end,
        _ => b.len(),
    } - i);
    let (label_end, dest_end) = (label_end?, dest_end?);
    let dest = s[label_end + 2..dest_end].trim();
    let url = match dest.strip_prefix('<').and_then(|d| d.split_once('>')) {
        Some((url, _)) => url,
        None => dest.split_whitespace().next().unwrap_or(""),
    };
    let children = nested(&s[i + 1..label_end]);
    Some((Inline::Link { url: url.to_string(), children }, dest_end + 1))
}

/// Index of the bracket that closes the one at `start`.
fn matching(b: &[u8], start: usize, open: u8, close: u8, stop_at_newline: bool) -> Option<usize> {
    let (mut depth, mut j) = (0usize, start);
    while j < b.len() {
        match b[j] {
            b'\\' => j += 1,
            x if x == open => depth += 1,
            x if x == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            b'\n' if stop_at_newline => return None,
            _ => {}
        }
        j += 1;
    }
    None
}

fn autolink(s: &str, i: usize) -> Option<(Inline, usize)> {
    let end = i + find(&s[i..], ">")?;
    let url = &s[i + 1..end];
    let web = url.starts_with("https://") || url.starts_with("http://");
    (web && !url.contains(char::is_whitespace))
        .then(|| (Inline::Link { url: url.to_string(), children: vec![Inline::Text(url.to_string())] }, end + 1))
}

fn bare_url(s: &str, i: usize) -> Option<(Inline, usize)> {
    let len = s[i..].find(|c: char| c.is_whitespace() || c == '<').unwrap_or(s.len() - i);
    let mut url = &s[i..i + len];
    // Trailing punctuation ends the sentence, not the URL.
    loop {
        let trimmed = url.trim_end_matches(['.', ',', ';', ':', '!', '?', '"', '\'', '*', '_']);
        let trimmed = if trimmed.ends_with(')') && trimmed.matches('(').count() < trimmed.matches(')').count() {
            &trimmed[..trimmed.len() - 1]
        } else {
            trimmed
        };
        if trimmed.len() == url.len() {
            break;
        }
        url = trimmed;
    }
    (url.len() > "https://".len()).then(|| (Inline::Link { url: url.to_string(), children: vec![Inline::Text(url.to_string())] }, i + url.len()))
}

/// http(s) opens in the browser; a relative or absolute path inside the
/// project (optionally `:line`, `:line:col` or `#Lnn`) opens in the editor.
pub fn classify_link(url: &str, root: Option<&Path>) -> LinkTarget {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return LinkTarget::Web(url.to_string());
    }
    let Some(root) = root else { return LinkTarget::Unsupported };
    let path = url.strip_prefix("file://").unwrap_or(url);
    let (path, line) = split_line_suffix(path);
    if path.is_empty() || path.contains("://") || lower.starts_with("mailto:") || lower.starts_with("javascript:") || lower.starts_with("data:") {
        return LinkTarget::Unsupported;
    }
    let joined = if Path::new(path).is_absolute() { PathBuf::from(path) } else { root.join(path) };
    let normalized = normalize(&joined);
    let canonical = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    if normalized.starts_with(root) || canonical(&normalized).starts_with(canonical(root)) {
        LinkTarget::File { path: normalized, line }
    } else {
        LinkTarget::Unsupported
    }
}

/// `main.tex:12`, `main.tex:12:3` and `main.tex#L12` → (`main.tex`, 12);
/// any other fragment (`README.md#usage`) is dropped.
fn split_line_suffix(path: &str) -> (&str, Option<usize>) {
    if let Some((file, anchor)) = path.rsplit_once('#') {
        let digits: String = anchor.strip_prefix('L').unwrap_or("").chars().take_while(|c| c.is_ascii_digit()).collect();
        return (file, digits.parse().ok().filter(|&n| n > 0));
    }
    let mut file = path;
    let mut numbers = Vec::new();
    while let Some((head, tail)) = file.rsplit_once(':') {
        if tail.is_empty() || !tail.bytes().all(|b| b.is_ascii_digit()) || numbers.len() == 2 {
            break;
        }
        numbers.push(tail);
        file = head;
    }
    let line = numbers.last().and_then(|n| n.parse().ok()).filter(|&n| n > 0);
    (file, line)
}

/// `a/./b/../c` → `a/c`, without touching the disk.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use Inline::*;

    fn text(s: &str) -> Inline {
        Text(s.into())
    }
    fn para(inlines: Vec<Inline>) -> Block {
        Block::Paragraph(inlines)
    }
    fn blocks(s: &str) -> Vec<Block> {
        parse(s, false).blocks
    }

    #[test]
    fn headings_paragraphs_and_rules() {
        assert_eq!(
            blocks("# Title\n\nSome *em* and **strong**.\nNext line\n\n---\n### Sub ###"),
            vec![
                Block::Heading(1, vec![text("Title")]),
                para(vec![text("Some "), Emphasis(vec![text("em")]), text(" and "), Strong(vec![text("strong")]), text(".\nNext line")]),
                Block::Rule,
                Block::Heading(3, vec![text("Sub")]),
            ]
        );
        assert_eq!(blocks("#hashtag"), vec![para(vec![text("#hashtag")])]);
        assert_eq!(blocks("snake_case_name and 2*3"), vec![para(vec![text("snake_case_name and 2*3")])]);
    }

    #[test]
    fn lists_nest_and_number() {
        let parsed = blocks("- one\n- two\n  - nested\n\n3. three\n4. four\n\n   more four\n");
        assert_eq!(
            parsed,
            vec![
                Block::List {
                    ordered: false,
                    start: 0,
                    items: vec![
                        vec![para(vec![text("one")])],
                        vec![
                            para(vec![text("two")]),
                            Block::List { ordered: false, start: 0, items: vec![vec![para(vec![text("nested")])]] },
                        ],
                    ],
                },
                Block::List {
                    ordered: true,
                    start: 3,
                    items: vec![vec![para(vec![text("three")])], vec![para(vec![text("four")]), para(vec![text("more four")])]],
                },
            ]
        );
        // Blank lines between items keep one list.
        let loose = blocks("1. a\n\n2. b\n");
        assert!(matches!(&loose[..], [Block::List { items, .. }] if items.len() == 2), "{loose:?}");
    }

    #[test]
    fn quotes_and_code_fences() {
        assert_eq!(
            blocks("> quoted **text**\n> more\n\n```latex\n\\section{A} % *not* emphasis\n  $x$\n```\n~~~\nplain\n~~~"),
            vec![
                Block::Quote(vec![para(vec![text("quoted "), Strong(vec![text("text")]), text("\nmore")])]),
                Block::Code { language: "latex".into(), text: "\\section{A} % *not* emphasis\n  $x$".into() },
                Block::Code { language: String::new(), text: "plain".into() },
            ]
        );
        // An unclosed fence runs to the end (CommonMark), streaming or not.
        assert_eq!(blocks("```py\nx = 1"), vec![Block::Code { language: "py".into(), text: "x = 1".into() }]);
        assert_eq!(parse("```py\nx = 1", true).blocks, vec![Block::Code { language: "py".into(), text: "x = 1".into() }]);
        assert_eq!(blocks("```math\nE=mc^2\n```"), vec![Block::Math("E=mc^2".into())]);
    }

    #[test]
    fn tables() {
        assert_eq!(
            blocks("| Cmd | Use |\n|---|:-:|\n| `\\cite` | refs \\| more |\n| $x$ | **b** |\nafter"),
            vec![
                Block::Table(vec![
                    vec![vec![text("Cmd")], vec![text("Use")]],
                    vec![vec![Code("\\cite".into())], vec![text("refs | more")]],
                    vec![vec![Math("x".into())], vec![Strong(vec![text("b")])]],
                ]),
                para(vec![text("after")]),
            ]
        );
        assert_eq!(blocks("a | b\nnot a table"), vec![para(vec![text("a | b\nnot a table")])]);
        // The delimiter row must have as many cells as the header (GFM).
        assert_eq!(blocks("a | b\n---"), vec![para(vec![text("a | b")]), Block::Rule]);
    }

    #[test]
    fn math_delimiters_escapes_and_currency() {
        assert_eq!(
            parse_inlines("Inline $E=mc^2$, \\(a+b\\) and $$x^2$$ here"),
            vec![text("Inline "), Math("E=mc^2".into()), text(", "), Math("a+b".into()), text(" and "), Math("x^2".into()), text(" here")]
        );
        assert_eq!(parse_inlines("costs $5 and $10 today"), vec![text("costs $5 and $10 today")]);
        assert_eq!(parse_inlines("pay \\$x\\$ now"), vec![text("pay $x$ now")]);
        assert_eq!(parse_inlines("a $ b $ c"), vec![text("a $ b $ c")]);
        assert_eq!(parse_inlines("`$x$` is code"), vec![Code("$x$".into()), text(" is code")]);
        assert_eq!(parse_inlines("$a_1 * b_2$"), vec![Math("a_1 * b_2".into())]);
        assert_eq!(parse_inlines("echo $FOO$BAR"), vec![text("echo $FOO$BAR")]);
        assert_eq!(
            blocks("$$\n\\int_0^1 x\\,dx\n$$\n\\[ a = b \\]\n$$y$$"),
            vec![Block::Math("\\int_0^1 x\\,dx".into()), Block::Math("a = b".into()), Block::Math("y".into())]
        );
        assert_eq!(blocks("$$a$$ and $$b$$"), vec![para(vec![Math("a".into()), text(" and "), Math("b".into())])]);
        // A finished message with an unclosed $$ keeps it as text.
        assert_eq!(blocks("$$\nx"), vec![para(vec![text("$$\nx")])]);
        // Sentence punctuation after the closer still closes.
        assert_eq!(blocks("$$\na=b\n$$."), vec![Block::Math("a=b".into())]);
        // Display math never spans a blank line: streaming text after one renders.
        assert_eq!(
            parse("$$5 off\n\nnext **bold**", true).blocks,
            vec![para(vec![text("$$5 off")]), para(vec![text("next "), Strong(vec![text("bold")])])]
        );
    }

    #[test]
    fn links_and_autolinks() {
        assert_eq!(
            parse_inlines("See [the *docs*](https://x.org/a_(b)) or <https://y.org> or https://z.org/p."),
            vec![
                text("See "),
                Link { url: "https://x.org/a_(b)".into(), children: vec![text("the "), Emphasis(vec![text("docs")])] },
                text(" or "),
                Link { url: "https://y.org".into(), children: vec![text("https://y.org")] },
                text(" or "),
                Link { url: "https://z.org/p".into(), children: vec![text("https://z.org/p")] },
                text("."),
            ]
        );
        assert_eq!(parse_inlines("[main.tex:12](main.tex:12)"), vec![Link { url: "main.tex:12".into(), children: vec![text("main.tex:12")] }]);
        assert_eq!(parse_inlines("[x] todo"), vec![text("[x] todo")]);
        // Images are not drawn: they stay their source.
        assert_eq!(parse_inlines("see ![img](x.png)"), vec![text("see ![img](x.png)")]);
    }

    #[test]
    fn link_classification() {
        let root = Path::new("/proj");
        assert_eq!(classify_link("https://example.org/a", Some(root)), LinkTarget::Web("https://example.org/a".into()));
        assert_eq!(classify_link("HTTP://example.org", None), LinkTarget::Web("HTTP://example.org".into()));
        assert_eq!(classify_link("main.tex:12", Some(root)), LinkTarget::File { path: "/proj/main.tex".into(), line: Some(12) });
        assert_eq!(classify_link("sec/intro.tex:4:2", Some(root)), LinkTarget::File { path: "/proj/sec/intro.tex".into(), line: Some(4) });
        assert_eq!(classify_link("/proj/main.tex#L7", Some(root)), LinkTarget::File { path: "/proj/main.tex".into(), line: Some(7) });
        assert_eq!(classify_link("./a/../main.tex", Some(root)), LinkTarget::File { path: "/proj/main.tex".into(), line: None });
        assert_eq!(classify_link("../outside.tex", Some(root)), LinkTarget::Unsupported);
        assert_eq!(classify_link("/etc/passwd", Some(root)), LinkTarget::Unsupported);
        assert_eq!(classify_link("javascript:alert(1)", Some(root)), LinkTarget::Unsupported);
        assert_eq!(classify_link("main.tex", None), LinkTarget::Unsupported);
        assert_eq!(classify_link("README.md#usage", Some(root)), LinkTarget::File { path: "/proj/README.md".into(), line: None });
    }

    #[test]
    fn streaming_runs_an_open_code_fence_to_the_end_and_settles() {
        let text = "# Plan\n\nIntro *text*.\n\n```latex\n\\section{A}\n";
        let split = stable_prefix_len(text);
        assert_eq!(&text[..split], "# Plan\n\nIntro *text*.\n\n");
        let doc = parse(text, true);
        assert_eq!(
            doc.blocks,
            vec![
                Block::Heading(1, vec![text_("Plan")]),
                para(vec![text_("Intro "), Emphasis(vec![text_("text")]), text_(".")]),
                Block::Code { language: "latex".into(), text: "\\section{A}".into() },
            ]
        );
        // An open math fence stays literal: no pdflatex on half a formula.
        assert_eq!(parse("```math\nE=", true).blocks, vec![para(vec![text_("```math\nE=")])]);
        // Once closed, the same text parses as a code block.
        let closed = parse(&format!("{text}```\n"), true);
        assert_eq!(closed.blocks.last(), Some(&Block::Code { language: "latex".into(), text: "\\section{A}".into() }));
        // A blank line inside a fence is not a boundary.
        assert_eq!(stable_prefix_len("```\na\n\nb\n"), 0);
        // Unclosed display math stays literal while streaming.
        assert_eq!(parse("$$\n\\int x", true).blocks, vec![para(vec![text_("$$\n\\int x")])]);
        // Closed input streams into exactly the blocks of the final parse.
        let done = "# Plan\n\n- a\n- b\n\n$$\nx\n$$\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n```latex\n\\section{A}\n```\n\nEnd $y$.\n";
        assert_eq!(parse(done, true), parse(done, false));
    }

    #[test]
    fn deep_nesting_and_unclosed_openers_stay_bounded() {
        fn depth(blocks: &[Block]) -> usize {
            blocks.iter().map(|b| match b {
                Block::Quote(inner) => 1 + depth(inner),
                Block::List { items, .. } => 1 + items.iter().map(|i| depth(i)).max().unwrap_or(0),
                _ => 0,
            }).max().unwrap_or(0)
        }
        fn inline_depth(inlines: &[Inline]) -> usize {
            inlines.iter().map(|i| match i {
                Strong(c) | Emphasis(c) | Link { children: c, .. } => 1 + inline_depth(c),
                _ => 0,
            }).max().unwrap_or(0)
        }
        // Each of these overflowed the stack (or GTK's widget tree) before the cap.
        assert_eq!(depth(&blocks(&">".repeat(20_000))), MAX_DEPTH);
        assert_eq!(depth(&blocks(&"+ ".repeat(10_000))), MAX_DEPTH);
        assert_eq!(depth(&blocks(&"1. ".repeat(10_000))), MAX_DEPTH);
        assert_eq!(depth(&parse(&"> - ".repeat(5_000), true).blocks), MAX_DEPTH);
        let links = format!("{}a{}", "[".repeat(5_000), "](b)".repeat(5_000));
        let Block::Paragraph(inlines) = &blocks(&links)[0] else { panic!() };
        assert!(inline_depth(inlines) <= MAX_DEPTH + 1);
        // Unclosed openers rescan the paragraph each; the budget keeps this fast.
        for text in ["[".repeat(31_000), "*a ".repeat(10_000), "\\(".repeat(15_000), "[a](".repeat(7_000)] {
            let started = std::time::Instant::now();
            let _ = parse(&text, true);
            assert!(started.elapsed() < std::time::Duration::from_secs(1), "{}", &text[..8]);
        }
    }

    fn text_(s: &str) -> Inline {
        Text(s.into())
    }

    #[test]
    fn render_limit_caps_at_32k_characters() {
        let long = "é".repeat(RENDER_LIMIT + 5);
        let doc = parse(&long, false);
        assert!(doc.truncated);
        let Block::Paragraph(inlines) = &doc.blocks[0] else { panic!() };
        assert_eq!(inlines, &vec![Text("é".repeat(RENDER_LIMIT))]);
        let exact = "a".repeat(RENDER_LIMIT);
        assert!(!parse(&exact, false).truncated);
    }
}
