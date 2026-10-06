// Pitex-authored BibTeX style-language interpreter.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// Interface specification: Oren Patashnik, Designing BibTeX Styles (btxhak).
// https://tug.ctan.org/biblio/bibtex/base/btxhak.pdf
// No BibTeX/Biber implementation code is incorporated here. Installed .bst
// programs are runtime document resources, not compiled or bundled dependencies.
use super::{group, names, Entry};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Word(String),
    Int(i64),
    Str(String),
    Quote(String),
    Block(Vec<Token>),
}
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Int(i64),
    Str(String),
    Missing,
    Function(Vec<Token>),
    Name(String),
}

fn tokens(s: &str, i: &mut usize, nested: bool) -> Result<Vec<Token>, String> {
    let mut out = Vec::new();
    while *i < s.len() {
        let c = s.as_bytes()[*i];
        if c.is_ascii_whitespace() {
            *i += 1;
            continue;
        }
        if c == b'%' {
            while *i < s.len() && s.as_bytes()[*i] != b'\n' {
                *i += 1;
            }
            continue;
        }
        if c == b'}' {
            if nested {
                *i += 1;
                return Ok(out);
            }
            return Err("Unexpected } in bibliography style".into());
        }
        if c == b'{' {
            *i += 1;
            out.push(Token::Block(tokens(s, i, true)?));
            continue;
        }
        if c == b'"' {
            *i += 1;
            let start = *i;
            while *i < s.len() && s.as_bytes()[*i] != b'"' {
                *i += 1;
            }
            if *i == s.len() {
                return Err("Unclosed bibliography style string".into());
            }
            out.push(Token::Str(s[start..*i].into()));
            *i += 1;
            continue;
        }
        let prefix = if c == b'\'' || c == b'#' {
            *i += 1;
            Some(c)
        } else {
            None
        };
        let start = *i;
        while *i < s.len()
            && !s.as_bytes()[*i].is_ascii_whitespace()
            && !b"{}\"%".contains(&s.as_bytes()[*i])
        {
            *i += 1;
        }
        if start == *i {
            return Err(format!("Invalid bibliography style token near byte{start}"));
        }
        let word = &s[start..*i];
        out.push(match prefix {
            Some(b'#') => Token::Int(
                word.parse()
                    .map_err(|_| format!("Invalid style integer {word}"))?,
            ),
            Some(b'\'') => Token::Quote(word.to_lowercase()),
            _ => Token::Word(word.to_lowercase()),
        });
    }
    if nested {
        return Err("Unclosed bibliography style function".into());
    }
    Ok(out)
}
fn block<'a>(ts: &'a [Token], i: &mut usize) -> Result<&'a [Token], String> {
    match ts.get(*i) {
        Some(Token::Block(body)) => {
            *i += 1;
            Ok(body)
        }
        _ => Err("Expected bibliography style argument block".into()),
    }
}
fn symbol(ts: &[Token]) -> Result<String, String> {
    match ts.first() {
        Some(Token::Word(s)) => Ok(s.clone()),
        _ => Err("Expected bibliography style identifier".into()),
    }
}
#[derive(Clone)]
struct Row {
    entry: Entry,
    variables: BTreeMap<String, Value>,
}
struct Vm {
    functions: BTreeMap<String, Vec<Token>>,
    globals: BTreeMap<String, Value>,
    rows: Vec<Row>,
    order: Vec<usize>,
    current: Option<usize>,
    stack: Vec<Value>,
    output: String,
    warnings: Vec<String>,
    steps: usize,
    preamble: String,
}
impl Vm {
    fn pop(&mut self) -> Result<Value, String> {
        self.stack
            .pop()
            .ok_or_else(|| "Bibliography style stack underflow".into())
    }
    fn integer(&mut self) -> Result<i64, String> {
        match self.pop()? {
            Value::Int(i) => Ok(i),
            v => Err(format!("Bibliography style expected integer, got {v:?}")),
        }
    }
    fn string(&mut self) -> Result<String, String> {
        match self.pop()? {
            Value::Str(s) => Ok(s),
            Value::Missing => Ok(String::new()),
            v => Err(format!("Bibliography style expected string, got {v:?}")),
        }
    }
    fn pushs(&mut self, s: String) {
        self.stack.push(Value::Str(s));
    }
    fn pushi(&mut self, i: i64) {
        self.stack.push(Value::Int(i));
    }
    fn run(&mut self, body: &[Token], depth: usize) -> Result<(), String> {
        if depth > 512 {
            return Err("Bibliography style recursion limit exceeded".into());
        }
        for token in body {
            self.steps += 1;
            if self.steps > 5_000_000 {
                return Err("Bibliography style instruction limit exceeded".into());
            }
            match token {
                Token::Int(i) => self.pushi(*i),
                Token::Str(s) => self.pushs(s.clone()),
                Token::Quote(n) => self.stack.push(Value::Name(n.clone())),
                Token::Block(b) => self.stack.push(Value::Function(b.clone())),
                Token::Word(n) => self.call(n, depth + 1)?,
            }
        }
        Ok(())
    }
    fn invoke(&mut self, value: Value, depth: usize) -> Result<(), String> {
        match value {
            Value::Name(n) => self.call(&n, depth + 1),
            Value::Function(b) => self.run(&b, depth + 1),
            v => Err(format!("Bibliography style expected function, got {v:?}")),
        }
    }
    fn call(&mut self, name: &str, depth: usize) -> Result<(), String> {
        if let Some(f) = self.functions.get(name).cloned() {
            return self.run(&f, depth + 1);
        }
        match name {
            "+" | "-" | ">" | "<" => {
                let b = self.integer()?;
                let a = self.integer()?;
                self.pushi(match name {
                    "+" => a.saturating_add(b),
                    "-" => a.saturating_sub(b),
                    ">" => i64::from(a > b),
                    _ => i64::from(a < b),
                });
            }
            "=" => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.pushi(i64::from(a == b));
            }
            "*" => {
                let b = self.string()?;
                let a = self.string()?;
                if a.len().saturating_add(b.len()) > 8 * 1024 * 1024 {
                    return Err("Bibliography style string limit exceeded".into());
                }
                self.pushs(a + &b);
            }
            ":=" => {
                let var = match self.pop()? {
                    Value::Name(s) => s,
                    _ => {
                        return Err("Bibliography style assignment requires quoted variable".into())
                    }
                };
                let v = self.pop()?;
                if let Some(row) = self.current.and_then(|i| self.rows.get_mut(i)) {
                    if row.variables.contains_key(&var) {
                        row.variables.insert(var, v);
                        return Ok(());
                    }
                }
                if self.globals.contains_key(&var) {
                    self.globals.insert(var, v);
                } else {
                    return Err(format!("Undeclared bibliography style variable {var}"));
                }
            }
            "duplicate$" => {
                let v = self
                    .stack
                    .last()
                    .cloned()
                    .ok_or("Bibliography style stack underflow")?;
                self.stack.push(v);
            }
            "swap$" => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(b);
                self.stack.push(a);
            }
            "pop$" => {
                self.pop()?;
            }
            "skip$" => {}
            "if$" => {
                let otherwise = self.pop()?;
                let yes = self.pop()?;
                let c = self.integer()?;
                self.invoke(if c > 0 { yes } else { otherwise }, depth + 1)?;
            }
            "while$" => {
                let body = self.pop()?;
                let condition = self.pop()?;
                loop {
                    self.invoke(condition.clone(), depth + 1)?;
                    if self.integer()? <= 0 {
                        break;
                    }
                    self.invoke(body.clone(), depth + 1)?;
                }
            }
            "write$" => {
                let s = self.string()?;
                if self.output.len().saturating_add(s.len()) > 64 * 1024 * 1024 {
                    return Err("Bibliography output limit exceeded".into());
                }
                self.output.push_str(&s);
            }
            "newline$" => self.output.push('\n'),
            "warning$" => {
                let w = self.string()?;
                self.warnings.push(w);
            }
            "stack$" => {
                self.warnings
                    .push(format!("Bibliography style stack: {:?}", self.stack));
                self.stack.clear();
            }
            "top$" => {
                let v = self.pop()?;
                self.warnings.push(format!("Bibliography style top: {v:?}"));
            }
            "empty$" => {
                let v = self.pop()?;
                self.pushi(i64::from(match v {
                    Value::Missing => true,
                    Value::Str(s) => s.trim().is_empty(),
                    _ => false,
                }));
            }
            "missing$" => {
                let v = self.pop()?;
                self.pushi(i64::from(v == Value::Missing));
            }
            "cite$" => {
                let s = self
                    .current
                    .map(|i| self.rows[i].entry.key.clone())
                    .unwrap_or_default();
                self.pushs(s);
            }
            "type$" => {
                let s = self
                    .current
                    .map(|i| self.rows[i].entry.kind.clone())
                    .unwrap_or_default();
                self.pushs(s);
            }
            "call.type$" => {
                let n = self
                    .current
                    .map(|i| self.rows[i].entry.kind.clone())
                    .unwrap_or_default();
                if self.functions.contains_key(&n) {
                    self.call(&n, depth + 1)?;
                } else if self.functions.contains_key("default.type") {
                    self.call("default.type", depth + 1)?;
                }
            }
            "preamble$" => self.pushs(self.preamble.clone()),
            "quote$" => self.pushs("\"".into()),
            "global.max$" | "entry.max$" => self.pushi(1_000_000),
            "int.to.str$" => {
                let i = self.integer()?;
                self.pushs(i.to_string());
            }
            "int.to.chr$" => {
                let i = self.integer()?;
                self.pushs(char::from_u32(i as u32).unwrap_or('\u{fffd}').to_string());
            }
            "chr.to.int$" => {
                let s = self.string()?;
                self.pushi(s.chars().next().map(|c| c as i64).unwrap_or(0));
            }
            "num.names$" => {
                let s = self.string()?;
                self.pushi(names(&s).len() as i64);
            }
            "format.name$" => {
                let f = self.string()?;
                let n = self.integer()?;
                let s = self.string()?;
                self.pushs(format_name(&s, n, &f)?);
            }
            "add.period$" => {
                let mut s = self.string()?;
                if !s.trim_end_matches('}').ends_with(['.', '?', '!']) && !s.is_empty() {
                    s.push('.');
                }
                self.pushs(s);
            }
            "change.case$" => {
                let mode = self.string()?;
                let s = self.string()?;
                self.pushs(change_case(&s, &mode));
            }
            "purify$" => {
                let s = self.string()?;
                self.pushs(purify(&s));
            }
            "substring$" => {
                let count = self.integer()?;
                let from = self.integer()?;
                let s = self.string()?;
                let chars = s.chars().collect::<Vec<_>>();
                let (start, end) = if from > 0 {
                    (from - 1, (from - 1).saturating_add(count))
                } else if from < 0 {
                    let end = chars.len() as i64 + from + 1;
                    (end.saturating_sub(count), end)
                } else {
                    (0, 0)
                };
                let mut out = String::new();
                if count > 0 {
                    for i in start..end {
                        if i >= 0 && (i as usize) < chars.len() {
                            out.push(chars[i as usize]);
                        }
                    }
                }
                self.pushs(out);
            }
            "text.length$" => {
                let s = self.string()?;
                self.pushi(text_units(&s).len() as i64);
            }
            "text.prefix$" => {
                let count = self.integer()?;
                let s = self.string()?;
                self.pushs(text_prefix(&s, count.max(0) as usize));
            }
            "width$" => {
                let s = self.string()?;
                self.pushi(width(&s));
            }
            _ => {
                if let Some(i) = self.current {
                    if let Some(v) = self.rows[i].variables.get(name).cloned() {
                        self.stack.push(v);
                        return Ok(());
                    }
                    if let Some(v) = self.rows[i].entry.fields.get(name).cloned() {
                        self.pushs(v);
                        return Ok(());
                    }
                }
                if let Some(v) = self.globals.get(name).cloned() {
                    self.stack.push(v);
                    return Ok(());
                }
                if self.globals.get(&format!("@field:{name}")).is_some() {
                    self.stack.push(Value::Missing);
                    return Ok(());
                }
                return Err(format!("Unknown bibliography style function '{name}'"));
            }
        }
        Ok(())
    }
}
fn change_case(s: &str, mode: &str) -> String {
    let mut depth = 0;
    let mut out = String::new();
    let mut first = true;
    let mut after_colon = false;
    for c in s.chars() {
        if c == '{' {
            depth += 1;
            out.push(c);
            continue;
        }
        if c == '}' {
            depth -= 1;
            out.push(c);
            continue;
        }
        if depth > 0 {
            out.push(c);
            continue;
        }
        if mode == "u" {
            out.extend(c.to_uppercase());
        } else if mode == "l" {
            out.extend(c.to_lowercase());
        } else if mode == "t" && first || after_colon {
            out.push(c);
        } else if mode == "t" {
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
        if c.is_alphabetic() {
            first = false;
            after_colon = false;
        }
        if c == ':' {
            after_colon = true;
        }
    }
    out
}
fn purify(s: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < s.len() {
        let c = s[i..].chars().next().unwrap();
        i += c.len_utf8();
        if c == '\\' {
            let start = i;
            while i < s.len() && s.as_bytes()[i].is_ascii_alphabetic() {
                i += 1;
            }
            let cmd = &s[start..i];
            if ["ae", "AE", "oe", "OE", "aa", "AA", "o", "O", "l", "L", "ss"].contains(&cmd) {
                out.push_str(cmd);
            }
            continue;
        }
        if c.is_alphanumeric() || c.is_whitespace() {
            out.push(c);
        } else if c == '~' || c == '-' {
            out.push(' ');
        }
    }
    out
}
fn text_units(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with("{\\") {
            if let Some((_, end)) = group(s, i, b'{', b'}') {
                out.push(s[i..end].into());
                i = end;
                continue;
            }
        }
        let c = s[i..].chars().next().unwrap();
        i += c.len_utf8();
        if c != '{' && c != '}' {
            out.push(c.to_string());
        }
    }
    out
}
fn text_prefix(s: &str, count: usize) -> String {
    let mut out = String::new();
    let mut i = 0;
    let mut used = 0;
    let mut depth = 0;
    while i < s.len() && used < count {
        if s[i..].starts_with("{\\") {
            if let Some((_, end)) = group(s, i, b'{', b'}') {
                out.push_str(&s[i..end]);
                i = end;
                used += 1;
                continue;
            }
        }
        let c = s[i..].chars().next().unwrap();
        i += c.len_utf8();
        out.push(c);
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
        } else {
            used += 1;
        }
    }
    for _ in 0..depth.max(0) {
        out.push('}');
    }
    out
}
fn width(s: &str) -> i64 {
    // Computer Modern Roman 10 advances, rounded to BibTeX's thousand-em
    // interface units; these are font metric facts rather than engine code.
    const CMR: [i64; 128] = [
        625, 833, 778, 694, 667, 750, 722, 778, 722, 778, 722, 583, 556, 556, 833, 833, 278, 306,
        500, 500, 500, 500, 500, 750, 444, 500, 722, 778, 500, 903, 1014, 778, 278, 278, 500, 833,
        500, 833, 778, 278, 389, 389, 500, 778, 278, 333, 278, 500, 500, 500, 500, 500, 500, 500,
        500, 500, 500, 500, 278, 278, 278, 778, 472, 472, 778, 750, 708, 722, 764, 681, 653, 785,
        750, 361, 514, 778, 625, 917, 750, 778, 681, 778, 736, 556, 722, 750, 750, 1028, 750, 750,
        611, 278, 500, 278, 500, 278, 278, 500, 556, 444, 556, 444, 306, 500, 556, 278, 306, 528,
        278, 833, 556, 500, 556, 528, 392, 394, 389, 556, 528, 722, 528, 528, 444, 500, 1000, 500,
        500, 500,
    ];
    let mut total = 0;
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with("{\\") {
            if let Some((_, end)) = group(s, i, b'{', b'}') {
                let purified = purify(&s[i..end]);
                i = end;
                let special = match purified.as_str() {
                    "ss" => Some(25),
                    "ae" => Some(26),
                    "oe" => Some(27),
                    "o" => Some(28),
                    "AE" => Some(29),
                    "OE" => Some(30),
                    "O" => Some(31),
                    "i" => Some(16),
                    "j" => Some(17),
                    _ => None,
                };
                total += if let Some(index) = special {
                    CMR[index]
                } else {
                    purified
                        .chars()
                        .map(|c| if c.is_ascii() { CMR[c as usize] } else { 500 })
                        .sum()
                };
                continue;
            }
        }
        let c = s[i..].chars().next().unwrap();
        i += c.len_utf8();
        total += if c == '~' {
            CMR[b' ' as usize]
        } else if c.is_ascii() {
            CMR[c as usize]
        } else {
            500
        };
    }
    total
}

fn name_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    let mut start = 0;
    let mut depth = 0;
    while i < s.len() {
        let c = s[i..].chars().next().unwrap();
        if c == '{' {
            depth += 1;
        }
        if c == '}' {
            depth -= 1;
        }
        if depth == 0 && (c.is_whitespace() || c == '~') {
            if start < i {
                out.push(s[start..i].into());
            }
            i += c.len_utf8();
            start = i;
        } else {
            i += c.len_utf8();
        }
    }
    if start < i {
        out.push(s[start..].into());
    }
    out
}
fn format_name(s: &str, n: i64, format: &str) -> Result<String, String> {
    let ns = names(s);
    if n <= 0 || n as usize > ns.len() {
        return Err(format!(
            "Bibliography style name index {n} outside{} names",
            ns.len()
        ));
    }
    let name = &ns[n as usize - 1];
    let mut out = String::new();
    let mut i = 0;
    while i < format.len() {
        if format.as_bytes()[i] == b'{' {
            let (piece, end) = group(format, i, b'{', b'}').ok_or("Unclosed name format group")?;
            i = end;
            let mut depth = 0;
            let mut spec = None;
            for (p, c) in piece.char_indices() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                } else if depth == 0 && "fvlj".contains(c) {
                    spec = Some((p, c));
                    break;
                }
            }
            let Some((p, c)) = spec else {
                out.push_str(&piece);
                continue;
            };
            let corporate = format!("{{{}}}", name.family);
            let part = match c {
                'f' => &name.given,
                'v' => &name.prefix,
                'l' => {
                    if name.literal {
                        &corporate
                    } else {
                        &name.family
                    }
                }
                _ => &name.suffix,
            };
            if part.is_empty() {
                continue;
            }
            let repeated = piece[p..].starts_with(&c.to_string().repeat(2));
            let after = p + if repeated { 2 } else { 1 };
            let prefix = &piece[..p];
            let mut suffix = piece[after..].to_string();
            let mut separator = None;
            if let Some((sep, end)) = group(&suffix, 0, b'{', b'}') {
                separator = Some(sep);
                suffix = suffix[end..].into();
            }
            let ts = name_tokens(part);
            let abbreviation = !repeated;
            let rendered = ts
                .iter()
                .map(|t| {
                    if abbreviation {
                        if t.starts_with("{\\") {
                            t.clone()
                        } else {
                            t.chars()
                                .find(|c| c.is_alphabetic())
                                .map(|c| c.to_string())
                                .unwrap_or_default()
                        }
                    } else {
                        t.clone()
                    }
                })
                .collect::<Vec<_>>();
            out.push_str(prefix);
            let mut joined = String::new();
            for (index, token) in rendered.iter().enumerate() {
                if index > 0 {
                    if let Some(separator) = &separator {
                        joined.push_str(separator);
                    } else {
                        if abbreviation {
                            joined.push('.');
                        }
                        let force_spaces = suffix.contains(' ') && !suffix.contains('~');
                        let tied = !force_spaces
                            && (index + 1 == rendered.len()
                                || (index == 1
                                    && suffix.contains('~')
                                    && text_units(&rendered[0]).len() < 3));
                        joined.push(if tied { '~' } else { ' ' });
                    }
                }
                joined.push_str(token);
            }
            out.push_str(&joined);
            // A single ~ at the end is BibTeX's discretionary tie: names
            // containing at least three text characters may break there.
            if suffix.ends_with('~')
                && !suffix.ends_with("~~")
                && text_units(&format!("{joined}{}", &suffix[..suffix.len() - 1])).len() >= 3
            {
                let end = suffix.len();
                suffix.replace_range(end - 1..end, " ");
            }
            out.push_str(&suffix);
        } else {
            let c = format[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    Ok(out)
}
#[cfg(test)]
pub(super) fn render(style: &str, entries: &[Entry]) -> Result<(String, Vec<String>), String> {
    render_with_preamble(style, entries, "")
}
pub(super) fn render_with_preamble(
    style: &str,
    entries: &[Entry],
    preamble: &str,
) -> Result<(String, Vec<String>), String> {
    let ts = tokens(style, &mut 0, false)?;
    let mut vm = Vm {
        functions: BTreeMap::new(),
        globals: BTreeMap::new(),
        rows: entries
            .iter()
            .cloned()
            .map(|entry| Row {
                entry,
                variables: BTreeMap::from([("sort.key$".into(), Value::Str(String::new()))]),
            })
            .collect(),
        order: (0..entries.len()).collect(),
        current: None,
        stack: Vec::new(),
        output: String::new(),
        warnings: Vec::new(),
        steps: 0,
        preamble: preamble.to_string(),
    };
    vm.globals.insert("@field:crossref".into(), Value::Missing);
    let mut i = 0;
    while i < ts.len() {
        let directive = match &ts[i] {
            Token::Word(s) => s.as_str(),
            _ => return Err("Expected bibliography style directive".into()),
        };
        i += 1;
        match directive {
            "entry" => {
                let fields = block(&ts, &mut i)?;
                let integers = block(&ts, &mut i)?;
                let strings = block(&ts, &mut i)?;
                for token in fields {
                    if let Token::Word(n) = token {
                        vm.globals.insert(format!("@field:{n}"), Value::Missing);
                    }
                }
                for row in &mut vm.rows {
                    for t in integers {
                        if let Token::Word(n) = t {
                            row.variables.insert(n.clone(), Value::Int(0));
                        }
                    }
                    for t in strings {
                        if let Token::Word(n) = t {
                            row.variables.insert(n.clone(), Value::Str(String::new()));
                        }
                    }
                }
            }
            "integers" | "strings" => {
                let body = block(&ts, &mut i)?;
                for t in body {
                    if let Token::Word(n) = t {
                        vm.globals.insert(
                            n.clone(),
                            if directive == "integers" {
                                Value::Int(0)
                            } else {
                                Value::Str(String::new())
                            },
                        );
                    }
                }
            }
            "function" => {
                let n = symbol(block(&ts, &mut i)?)?;
                let body = block(&ts, &mut i)?.to_vec();
                vm.functions.insert(n, body);
            }
            "macro" => {
                let n = symbol(block(&ts, &mut i)?)?;
                let body = block(&ts, &mut i)?.to_vec();
                vm.functions.insert(n, body);
            }
            "read" => {}
            "execute" => {
                let body = block(&ts, &mut i)?;
                vm.current = None;
                vm.run(body, 0)?;
            }
            "iterate" | "reverse" => {
                let body = block(&ts, &mut i)?;
                let mut order = vm.order.clone();
                if directive == "reverse" {
                    order.reverse();
                }
                for index in order {
                    vm.current = Some(index);
                    vm.run(body, 0)?;
                }
                vm.current = None;
            }
            "sort" => {
                let rows = &vm.rows;
                vm.order
                    .sort_by_key(|i| match rows[*i].variables.get("sort.key$") {
                        Some(Value::Str(s)) => s.to_lowercase(),
                        _ => String::new(),
                    });
            }
            _ => {
                return Err(format!(
                    "Unknown bibliography style directive '{directive}'"
                ))
            }
        }
    }
    if !vm.stack.is_empty() {
        vm.warnings
            .push("Bibliography style left values on its stack".into());
    }
    Ok((vm.output, vm.warnings))
}
pub(super) fn macros(style: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Ok(ts) = tokens(style, &mut 0, false) {
        let mut i = 0;
        while i < ts.len() {
            if let Token::Word(ref n) = ts[i] {
                if n == "macro" {
                    i += 1;
                    if let Ok(name) = block(&ts, &mut i).and_then(symbol) {
                        if let Ok(body) = block(&ts, &mut i) {
                            if let Some(Token::Str(s)) = body.first() {
                                out.insert(name, s.clone());
                            }
                        }
                    }
                    continue;
                }
            }
            i += 1;
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_program_and_builtins() {
        let entry = Entry {
            kind: "article".into(),
            key: "doe".into(),
            fields: BTreeMap::from([
                ("author".into(), "Jane Doe".into()),
                ("title".into(), "My TITLE".into()),
            ]),
        };
        let program = r#"ENTRY {author title year} {count} {sortlabel} INTEGERS {i} STRINGS {s} FUNCTION {one} { title "t" change.case$ write$ newline$ author #1 "{ll}, {f.}" format.name$ write$ newline$ year missing$ { "MISSING" write$ } { "wrong" write$ } if$ #2 'count := count int.to.str$ write$ } READ ITERATE {one}"#;
        let (out, w) = render(program, &[entry]).unwrap();
        assert_eq!(out, "My title\nDoe, J.\nMISSING2");
        assert!(w.is_empty());
    }
    #[test]
    fn loop_sort_reverse() {
        let entries = vec![
            Entry {
                kind: "misc".into(),
                key: "b".into(),
                fields: BTreeMap::new(),
            },
            Entry {
                kind: "misc".into(),
                key: "a".into(),
                fields: BTreeMap::new(),
            },
        ];
        let program = r#"ENTRY {} {} {} INTEGERS {n} FUNCTION {key} {cite$ 'sort.key$ :=} FUNCTION {out} { cite$ write$ } READ ITERATE {key} SORT ITERATE {out} REVERSE {out} EXECUTE { #0 'n := { n #3 < } { n #1 + 'n := } while$ n int.to.str$ write$ }"#;
        assert_eq!(render(program, &entries).unwrap().0, "abba3");
    }
}
