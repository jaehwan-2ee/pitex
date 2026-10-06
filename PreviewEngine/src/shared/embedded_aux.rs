// Pitex-authored in-process bibliography and source-highlighting services.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// These implementations use documented BibTeX/BBL/TeX interfaces; they do not
// incorporate Biber, BibTeX, Pygments, or shell-command implementation code.
#[path = "bibliography_control.rs"]
mod bibliography_control;
#[path = "bibtex_style.rs"]
mod bibtex_style;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

#[derive(Default, Debug)]
pub struct Prepared {
    pub files: BTreeMap<String, String>,
    pub warnings: Vec<String>,
    pub bibliography_warning_count: usize,
}

fn normalize(path: &Path) -> String {
    let mut parts = Vec::new();
    for p in path.components() {
        match p {
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            Component::ParentDir => {
                if parts.last().map(|p| p.as_str()) == Some("..") || parts.is_empty() {
                    parts.push("..".into());
                } else {
                    parts.pop();
                }
            }
            Component::RootDir => parts.push(String::new()),
            _ => {}
        }
    }
    parts.join("/")
}
fn whitespace(s: &str, mut i: usize) -> usize {
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}
fn group(s: &str, start: usize, open: u8, close: u8) -> Option<(String, usize)> {
    let mut i = whitespace(s, start);
    if s.as_bytes().get(i).copied() != Some(open) {
        return None;
    }
    i += 1;
    let begin = i;
    let mut depth = 1;
    while i < s.len() {
        if s.as_bytes()[i] == b'\\' {
            i += 1;
            if i < s.len() {
                i += 1;
            }
            continue;
        }
        if s.as_bytes()[i] == open {
            depth += 1;
        }
        if s.as_bytes()[i] == close {
            depth -= 1;
            if depth == 0 {
                return Some((s[begin..i].into(), i + 1));
            }
        }
        i += 1;
    }
    None
}
fn command_args(s: &str, name: &str) -> Vec<String> {
    let needle = format!("\\{name}");
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = s[i..].find(&needle) {
        let p = i + p;
        i = p + needle.len();
        if s.as_bytes()
            .get(i)
            .map(|b| b.is_ascii_alphabetic())
            .unwrap_or(false)
        {
            continue;
        }
        if s.as_bytes().get(i) == Some(&b'*') {
            i += 1;
        }
        for _ in 0..2 {
            if let Some((_, end)) = group(s, i, b'[', b']') {
                i = end;
            } else {
                break;
            }
        }
        if let Some((arg, end)) = group(s, i, b'{', b'}') {
            out.push(arg);
            i = end;
        }
    }
    out
}
fn package_used(source: &str, name: &str) -> bool {
    ["usepackage", "RequirePackage"].iter().any(|command| {
        command_args(source, command).iter().any(|packages| {
            split_top(packages, ",")
                .iter()
                .any(|package| package.trim() == name)
        })
    })
}
fn uncomment(s: &str) -> String {
    s.lines()
        .map(|line| {
            let mut escaped = false;
            for (i, c) in line.char_indices() {
                if c == '%' && !escaped {
                    return &line[..i];
                }
                if c == '\\' {
                    escaped = !escaped;
                } else {
                    escaped = false;
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn collect_sources(
    main: &str,
    load: &mut impl FnMut(&str) -> Option<String>,
) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut pending = vec![main.to_string()];
    while let Some(path) = pending.pop() {
        if files.contains_key(&path) || files.len() >= 256 {
            continue;
        }
        let Some(text) = load(&path) else {
            continue;
        };
        let clean = uncomment(&text);
        for command in ["input", "include", "subfile"] {
            for mut input in command_args(&clean, command) {
                if input.contains('\\') || input.contains('#') {
                    continue;
                }
                if Path::new(&input).extension().is_none() {
                    input.push_str(".tex");
                }
                let direct = normalize(Path::new(&input));
                let relative = normalize(
                    &Path::new(&path)
                        .parent()
                        .unwrap_or(Path::new(""))
                        .join(&input),
                );
                let found = if load(&direct).is_some() {
                    direct
                } else {
                    relative
                };
                pending.push(found);
            }
        }
        // Project document classes can load packages directly or through a
        // local base class. Installed classes remain engine-provided inputs.
        for command in ["documentclass", "LoadClass", "LoadClassWithOptions"] {
            for class in command_args(&clean, command) {
                if class.contains('\\') || class.contains('#') {
                    continue;
                }
                let class_file = format!("{class}.cls");
                if load(&class_file).is_some() {
                    pending.push(class_file);
                }
            }
        }
        // Local style packages can load these features through RequirePackage.
        // Installed biblatex is metadata, not an editor source buffer.
        for command in ["usepackage", "RequirePackage"] {
            for packages in command_args(&clean, command) {
                for package in split_top(&packages, ",") {
                    if package == "biblatex" || package.contains('\\') || package.contains('#') {
                        continue;
                    }
                    let style = format!("{package}.sty");
                    if load(&style).is_some() {
                        pending.push(style);
                    }
                }
            }
        }
        files.insert(path, text);
    }
    files
}

pub fn prepare(main: &str, mut load: impl FnMut(&str) -> Option<String>) -> Prepared {
    let mut result = Prepared::default();
    let sources = collect_sources(main, &mut load);
    let combined = sources
        .values()
        .map(|s| uncomment(s))
        .collect::<Vec<_>>()
        .join("\n");
    bibliography(main, &combined, &mut load, &mut result, None);
    result.bibliography_warning_count = result.warnings.len();
    for command in command_args(&combined, "ShellEscape") {
        result.warnings.push(format!(
            "External shell command is not executed by embedded preview: {command}"
        ));
    }
    let mut shell_offset = 0;
    while let Some(position) = combined[shell_offset..].find("\\write18") {
        shell_offset += position + "\\write18".len();
        if let Some((command, end)) = group(&combined, shell_offset, b'{', b'}') {
            result.warnings.push(format!(
                "External shell command is not executed by embedded preview: {command}"
            ));
            shell_offset = end;
        }
    }
    // A valid frozen Pygments cache is already handled by the installed minted
    // package. Preserve its exact styling and package behavior.
    if !combined.contains("frozencache")
        && (package_used(&combined, "minted")
            || combined.contains("\\begin{minted}")
            || combined.contains("\\inputminted")
            || combined.contains("\\mintinline"))
    {
        for (path, source) in sources {
            let rewritten = highlight_source(&path, &source, &mut load, &mut result);
            if rewritten != source {
                result.files.insert(path, rewritten);
            }
        }
    }
    if package_used(&combined, "microtype") {
        if let Some(original) = result.files.get(main).cloned().or_else(|| load(main)) {
            // Install the driver adapter only when the real engine primitives
            // are present. Native paragraph line breaking applies expansion.
            let hook = r"\ifdefined\PitexFontExpansion\AddToHook{package/microtype/after}{\expandafter\def\csname MT@setup@expansion\endcsname{\csname ifMT@expansion\endcsname\global\PitexFontExpansion=2 \global\PitexFontStretch=20 \global\PitexFontShrink=20 \global\PitexFontStep=1 \ifcsname MT@stretch\endcsname\ifnum\csname MT@stretch\endcsname>-1 \global\PitexFontStretch=\csname MT@stretch\endcsname\relax\fi\fi\ifcsname MT@shrink\endcsname\ifnum\csname MT@shrink\endcsname>-1 \global\PitexFontShrink=\csname MT@shrink\endcsname\relax\fi\fi\ifcsname MT@step\endcsname\ifnum\csname MT@step\endcsname>0 \global\PitexFontStep=\csname MT@step\endcsname\relax\fi\fi\fi}}\fi ";
            result
                .files
                .insert(main.to_string(), format!("{hook}{original}"));
        }
    }
    result
}

// Actual TeX execution supplies expanded citation/resource names and section
// contexts. Source scanning is only the first-pass bootstrap.
pub fn prepare_from_controls(main: &str, mut load: impl FnMut(&str) -> Option<String>) -> Prepared {
    let mut result = Prepared::default();
    let control = match bibliography_control::read(main, &mut load) {
        Ok(Some(control)) => control,
        Ok(None) => return result,
        Err(error) => {
            result.warnings.push(error);
            return result;
        }
    };
    let job = Path::new(main)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let output = format!("{job}.bbl");
    if load(&output).is_some() {
        return result;
    }
    if !control.biblatex {
        let section = &control.sections[0];
        let source = format!(
            "\\bibliographystyle{{{}}}\\bibliography{{{}}}\\nocite{{{}}}",
            control.style,
            section.resources.join(","),
            section.keys.join(",")
        );
        bibliography(main, &source, &mut load, &mut result, None);
        return result;
    }
    let mut assembled = String::new();
    for section in &control.sections {
        let lists = if section.lists.is_empty() {
            vec![bibliography_control::List {
                name: "nty/global//global/global".into(),
                sorting: "nty".into(),
                kind: "entry".into(),
                ..bibliography_control::List::default()
            }]
        } else {
            section.lists.clone()
        };
        let mut blocks = Vec::new();
        for list in lists {
            let source = format!(
                "\\usepackage[sorting={}]{{biblatex}}{}\\nocite{{{}}}",
                list.sorting,
                section
                    .resources
                    .iter()
                    .map(|path| format!("\\addbibresource{{{path}}}"))
                    .collect::<String>(),
                section.keys.join(",")
            );
            let mut prepared = Prepared::default();
            bibliography(
                main,
                &source,
                &mut load,
                &mut prepared,
                control.document.as_ref().map(|doc| (doc, &list)),
            );
            result.warnings.extend(prepared.warnings);
            if let Some(bbl) = prepared.files.get(&output) {
                if assembled.is_empty() {
                    assembled.push_str(bbl.split("\\refsection{").next().unwrap_or(""));
                }
                if let Some(start) = bbl.find("\\datalist[") {
                    if let Some(end) = bbl.rfind("\\enddatalist") {
                        let mut block = bbl[start..end + "\\enddatalist".len()].to_string();
                        block = block.replacen(
                            &format!("{{{}/global//global/global}}", list.sorting),
                            &format!("{{{}}}", list.name),
                            1,
                        );
                        if list.kind == "shorthand" {
                            block = block.replacen("\\datalist[entry]", "\\datalist[shorthand]", 1);
                        }
                        blocks.push(block);
                    }
                }
            }
        }
        if !blocks.is_empty() {
            assembled.push_str(&format!(
                "\\refsection{{{}}}\n{}\n\\endrefsection\n",
                section.number,
                blocks.join("\n")
            ));
        }
    }
    if !assembled.is_empty() {
        assembled.push_str("\\endinput\n");
        result.files.insert(output, assembled);
    }
    result
}

#[derive(Clone, Debug)]
struct Entry {
    kind: String,
    key: String,
    fields: BTreeMap<String, String>,
}
struct BibParser<'a> {
    s: &'a str,
    i: usize,
    strings: BTreeMap<String, String>,
    warnings: Vec<String>,
    preamble: String,
}
impl<'a> BibParser<'a> {
    fn skip(&mut self) {
        loop {
            self.i = whitespace(self.s, self.i);
            if self.s.as_bytes().get(self.i) == Some(&b'%') {
                while self.i < self.s.len() && self.s.as_bytes()[self.i] != b'\n' {
                    self.i += 1;
                }
            } else {
                break;
            }
        }
    }
    fn token(&mut self) -> String {
        self.skip();
        let start = self.i;
        while self.i < self.s.len()
            && !self.s.as_bytes()[self.i].is_ascii_whitespace()
            && !b"=,{}()#\"".contains(&self.s.as_bytes()[self.i])
        {
            self.i += 1;
        }
        self.s[start..self.i].into()
    }
    fn value(&mut self) -> String {
        let mut output = String::new();
        loop {
            self.skip();
            if let Some((v, end)) = group(self.s, self.i, b'{', b'}') {
                output.push_str(&v);
                self.i = end;
            } else if self.s.as_bytes().get(self.i) == Some(&b'"') {
                self.i += 1;
                let start = self.i;
                let mut braces = 0;
                while self.i < self.s.len() {
                    match self.s.as_bytes()[self.i] {
                        b'\\' => {
                            self.i += 1;
                        }
                        b'{' => braces += 1,
                        b'}' => braces -= 1,
                        b'"' if braces == 0 => break,
                        _ => {}
                    }
                    self.i += 1;
                }
                output.push_str(&self.s[start..self.i]);
                if self.i < self.s.len() {
                    self.i += 1;
                }
            } else {
                let token = self.token();
                output.push_str(
                    self.strings
                        .get(&token.to_lowercase())
                        .map(String::as_str)
                        .unwrap_or(&token),
                );
            }
            self.skip();
            if self.s.as_bytes().get(self.i) != Some(&b'#') {
                break;
            }
            self.i += 1;
        }
        output
    }
    fn parse(&mut self) -> Vec<Entry> {
        let mut entries = Vec::new();
        while self.i < self.s.len() {
            let Some(p) = self.s[self.i..].find('@') else {
                break;
            };
            self.i += p + 1;
            let kind = self.token().to_lowercase();
            self.skip();
            let Some(open) = self.s.as_bytes().get(self.i).copied() else {
                break;
            };
            if open != b'{' && open != b'(' {
                self.i += 1;
                continue;
            }
            let close = if open == b'{' { b'}' } else { b')' };
            self.i += 1;
            if kind == "preamble" {
                let value = self.value();
                self.preamble.push_str(&value);
                while self.i < self.s.len() && self.s.as_bytes()[self.i] != close {
                    self.i += 1;
                }
                self.i += usize::from(self.i < self.s.len());
                continue;
            }
            if kind == "comment" {
                if let Some((_, end)) = group(self.s, self.i - 1, open, close) {
                    self.i = end;
                }
                continue;
            }
            if kind == "string" {
                let key = self.token().to_lowercase();
                self.skip();
                if self.s.as_bytes().get(self.i) == Some(&b'=') {
                    self.i += 1;
                    let v = self.value();
                    self.strings.insert(key, v);
                }
                while self.i < self.s.len() && self.s.as_bytes()[self.i] != close {
                    self.i += 1;
                }
                self.i += usize::from(self.i < self.s.len());
                continue;
            }
            let key = self.token();
            self.skip();
            if self.s.as_bytes().get(self.i) == Some(&b',') {
                self.i += 1;
            }
            let mut fields = BTreeMap::new();
            loop {
                self.skip();
                if self.i >= self.s.len() {
                    self.warnings.push(format!("Unclosed BibTeX entry '{key}'"));
                    break;
                }
                if self.s.as_bytes()[self.i] == close {
                    self.i += 1;
                    break;
                }
                if self.s.as_bytes()[self.i] == b',' {
                    self.i += 1;
                    continue;
                }
                let name = self.token().to_lowercase();
                self.skip();
                if name.is_empty() || self.s.as_bytes().get(self.i) != Some(&b'=') {
                    self.warnings
                        .push(format!("Malformed BibTeX field in '{key}'"));
                    self.i += 1;
                    continue;
                }
                self.i += 1;
                let v = self.value();
                fields.insert(name, v);
            }
            entries.push(Entry { kind, key, fields });
        }
        entries
    }
}
#[cfg(test)]
fn parse_bib(s: &str) -> (Vec<Entry>, Vec<String>) {
    let (entries, warnings, _) = parse_bib_with_macros(s, &BTreeMap::new());
    (entries, warnings)
}
fn parse_bib_with_macros(
    s: &str,
    style_macros: &BTreeMap<String, String>,
) -> (Vec<Entry>, Vec<String>, String) {
    let months = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let mut strings = BTreeMap::new();
    for m in months {
        strings.insert(m[..3].to_lowercase(), m.into());
    }
    strings.extend(style_macros.clone());
    let mut parser = BibParser {
        s,
        i: 0,
        strings,
        warnings: Vec::new(),
        preamble: String::new(),
    };
    let entries = parser.parse();
    (entries, parser.warnings, parser.preamble)
}
fn field<'a>(e: &'a Entry, name: &str) -> &'a str {
    e.fields.get(name).map(String::as_str).unwrap_or("")
}
fn split_top(s: &str, delimiter: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let mut depth = 0;
    while i < s.len() {
        let c = s.as_bytes()[i];
        if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            depth -= 1;
        }
        if depth == 0 && s[i..].starts_with(delimiter) {
            out.push(s[start..i].trim().into());
            i += delimiter.len();
            start = i;
        } else {
            i += s[i..].chars().next().unwrap().len_utf8();
        }
    }
    out.push(s[start..].trim().into());
    out
}
#[derive(Debug)]
struct Name {
    family: String,
    given: String,
    prefix: String,
    suffix: String,
    literal: bool,
}
fn name(s: &str) -> Name {
    if s.starts_with('{') && s.ends_with('}') {
        return Name {
            family: s[1..s.len() - 1].into(),
            given: String::new(),
            prefix: String::new(),
            suffix: String::new(),
            literal: true,
        };
    }
    let comma = split_top(s, ",");
    let (family, given, suffix) = if comma.len() > 1 {
        (
            comma[0].clone(),
            comma.last().unwrap().clone(),
            if comma.len() > 2 {
                comma[1].clone()
            } else {
                String::new()
            },
        )
    } else {
        let words = s.split_whitespace().collect::<Vec<_>>();
        if words.len() > 1 {
            let von = words[..words.len() - 1]
                .iter()
                .position(|word| word.chars().next().map(char::is_lowercase).unwrap_or(false))
                .unwrap_or(words.len() - 1);
            (
                words[von..].join(" "),
                words[..von].join(" "),
                String::new(),
            )
        } else {
            (s.into(), String::new(), String::new())
        }
    };
    let words = family.split_whitespace().collect::<Vec<_>>();
    let prefix_count = words
        .iter()
        .take_while(|w| w.chars().next().map(char::is_lowercase).unwrap_or(false))
        .count();
    Name {
        family: words[prefix_count..].join(" "),
        given,
        prefix: words[..prefix_count].join(" "),
        suffix,
        literal: false,
    }
}
fn names(s: &str) -> Vec<Name> {
    if s.is_empty() {
        Vec::new()
    } else {
        split_top(s, " and ").iter().map(|s| name(s)).collect()
    }
}
fn names_text(s: &str) -> String {
    names(s)
        .iter()
        .map(|n| {
            [
                n.given.as_str(),
                n.prefix.as_str(),
                n.family.as_str(),
                n.suffix.as_str(),
            ]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
        })
        .collect::<Vec<_>>()
        .join(", ")
}
fn author_label(e: &Entry) -> String {
    let ns = names(if field(e, "author").is_empty() {
        field(e, "editor")
    } else {
        field(e, "author")
    });
    match ns.len() {
        0 => field(e, "key").to_string(),
        1 => ns[0].family.clone(),
        2 => format!("{} and {}", ns[0].family, ns[1].family),
        _ => format!("{} et~al.", ns[0].family),
    }
}
fn bibliography(
    main: &str,
    source: &str,
    load: &mut impl FnMut(&str) -> Option<String>,
    result: &mut Prepared,
    control: Option<(&bibliography_control::Xml, &bibliography_control::List)>,
) {
    let biblatex = package_used(source, "biblatex");
    let resources = if biblatex {
        command_args(source, "addbibresource")
    } else {
        command_args(source, "bibliography")
    };
    if resources.is_empty() {
        return;
    }
    let job = Path::new(main)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let output = format!("{job}.bbl");
    if load(&output).is_some() {
        return;
    }
    let style = command_args(source, "bibliographystyle")
        .first()
        .cloned()
        .unwrap_or_else(|| "plain".into());
    let style_program = if biblatex {
        None
    } else {
        load(&format!("{style}.bst"))
    };
    let style_macros = style_program
        .as_deref()
        .map(bibtex_style::macros)
        .unwrap_or_default();
    let mut all = BTreeMap::new();
    let mut database_order = Vec::new();
    let mut database = String::new();
    for resource in resources {
        for item in split_top(&resource, ",") {
            let path = if item.ends_with(".bib") {
                item
            } else {
                format!("{item}.bib")
            };
            if let Some(source) = load(&path) {
                database.push_str(&source);
                database.push('\n');
            } else {
                result
                    .warnings
                    .push(format!("Bibliography resource '{path}' is unavailable"));
            }
        }
    }
    let (parsed, warnings, preamble) = parse_bib_with_macros(&database, &style_macros);
    result.warnings.extend(warnings);
    for entry in parsed {
        if !all.contains_key(&entry.key) {
            database_order.push(entry.key.clone());
        }
        all.insert(entry.key.clone(), entry);
    }
    if let Some((document, _)) = control {
        bibliography_control::sourcemap(&mut all, document, &mut result.warnings);
    }
    // Resolve crossref/xdata without mutating the input database.
    for _ in 0..8 {
        let previous = all.clone();
        for e in all.values_mut() {
            for parent in ["xdata", "crossref"] {
                for key in field(e, parent)
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
                {
                    if let Some(p) = previous.get(&key) {
                        for (k, v) in &p.fields {
                            if k != "crossref" && k != "xdata" {
                                e.fields.entry(k.clone()).or_insert(v.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    let mut cited = Vec::new();
    let mut seen = BTreeSet::new();
    let mut all_cited = false;
    let mut occurrences = Vec::new();
    for command in [
        "cite",
        "citep",
        "citet",
        "citealp",
        "citealt",
        "citeauthor",
        "citeyear",
        "autocite",
        "parencite",
        "textcite",
        "footcite",
        "smartcite",
        "supercite",
        "fullcite",
        "nocite",
    ] {
        let needle = format!("\\{command}");
        let mut offset = 0;
        while let Some(position) = source[offset..].find(&needle) {
            let position = position + offset;
            let mut i = position + needle.len();
            offset = i;
            if source
                .as_bytes()
                .get(i)
                .map(|b| b.is_ascii_alphabetic())
                .unwrap_or(false)
            {
                continue;
            }
            if source.as_bytes().get(i) == Some(&b'*') {
                i += 1;
            }
            for _ in 0..2 {
                if let Some((_, end)) = group(source, i, b'[', b']') {
                    i = end;
                } else {
                    break;
                }
            }
            if let Some((keys, end)) = group(source, i, b'{', b'}') {
                occurrences.push((position, keys));
                offset = end;
            }
        }
    }
    occurrences.sort_by_key(|(p, _)| *p);
    for (_, keys) in occurrences {
        for key in keys.split(',').map(str::trim).filter(|key| !key.is_empty()) {
            if key == "*" {
                all_cited = true;
            } else if seen.insert(key.to_string()) {
                cited.push(key.to_string());
            }
        }
    }
    if all_cited {
        for key in &database_order {
            if seen.insert(key.clone()) {
                cited.push(key.clone());
            }
        }
    }
    if !biblatex {
        let mut referenced = BTreeMap::<String, usize>::new();
        for key in &cited {
            if let Some(entry) = all.get(key) {
                let parent = field(entry, "crossref");
                if !parent.is_empty() {
                    *referenced.entry(parent.to_string()).or_default() += 1;
                }
            }
        }
        for (parent, count) in referenced {
            if count >= 2 && seen.insert(parent.clone()) && all.contains_key(&parent) {
                cited.push(parent);
            }
        }
        for key in &cited {
            if let Some(entry) = all.get_mut(key) {
                let parent = field(entry, "crossref").to_string();
                if !parent.is_empty() && !seen.contains(&parent) {
                    entry.fields.remove("crossref");
                }
            }
        }
    }
    let mut entries = Vec::new();
    for key in cited {
        if let Some(e) = all.get(&key) {
            entries.push(e.clone());
        } else {
            result
                .warnings
                .push(format!("Citation '{key}' has no bibliography entry"));
        }
    }
    if let Some((_, list)) = control {
        if list.kind == "shorthand" {
            entries.retain(|entry| !field(entry, "shorthand").is_empty());
        }
    }
    let sorting = option_value(source, "sorting").unwrap_or_else(|| {
        if biblatex {
            let style = option_value(source, "bibstyle")
                .or_else(|| option_value(source, "style"))
                .unwrap_or_else(|| "numeric".into());
            let mut next = vec![style.clone()];
            let mut seen = BTreeSet::new();
            while let Some(style) = next.pop() {
                if !seen.insert(style.clone()) || seen.len() > 16 {
                    continue;
                }
                if let Some(template) = load(&format!("{style}.bbx")) {
                    if let Some(sorting) = option_value(&template, "sorting") {
                        return sorting;
                    }
                    next.extend(command_args(&template, "RequireBibliographyStyle"));
                }
            }
            if style.starts_with("authoryear") {
                "nyt".into()
            } else if style.starts_with("alphabetic") {
                "anyt".into()
            } else {
                "nty".into()
            }
        } else {
            "plain".into()
        }
    });

    let controlled_sort = if let Some((document, list)) = control {
        bibliography_control::sort(&mut entries, list, document, &mut result.warnings)
    } else {
        false
    };
    if biblatex
        && !controlled_sort
        && !["nty", "nyt", "nyvt", "ynt", "ydnt", "none", "debug", "anyt"]
            .contains(&sorting.as_str())
    {
        result.warnings.push(format!("Custom bibliography sorting '{sorting}' uses the embedded canonical ordering; custom Biber sorting templates are not yet supported"));
    }
    let unsorted = if biblatex {
        sorting == "none"
    } else {
        style.starts_with("unsrt")
    };
    if biblatex && !unsorted && !controlled_sort {
        entries.sort_by_key(|e| {
            let n = format!("{} {}", author_label(e), field(e, "title")).to_lowercase();
            if sorting.starts_with('y') {
                format!("{} {n}", field(e, "year"))
            } else if sorting == "nyt" || sorting == "nyvt" {
                format!(
                    "{} {} {}",
                    author_label(e),
                    field(e, "year"),
                    field(e, "title")
                )
                .to_lowercase()
            } else {
                n
            }
        });
        if sorting == "ydnt" {
            entries.reverse();
        }
    }
    let bbl = if biblatex {
        biblatex_bbl(
            &entries,
            &sorting,
            load,
            control.map(|(document, _)| document),
            &mut result.warnings,
        )
    } else {
        match style_program
            .as_deref()
            .map(|program| bibtex_style::render_with_preamble(program, &entries, &preamble))
        {
            Some(Ok((output, warnings))) => {
                result.warnings.extend(warnings);
                output
            }
            error => {
                result.warnings.push(match error {Some(Err(error))=>format!("Embedded BibTeX style '{style}' failed: {error}; canonical formatting is shown"),_=>format!("Bibliography style '{style}.bst' is unavailable; canonical formatting is shown")});
                bibtex_bbl(
                    &entries,
                    &style,
                    source.contains("{natbib}") || style.ends_with("nat"),
                )
            }
        }
    };
    result.files.insert(output, bbl);
}
fn option_value(s: &str, key: &str) -> Option<String> {
    for (position, _) in s.match_indices(key) {
        if position > 0 && s.as_bytes()[position - 1].is_ascii_alphanumeric() {
            continue;
        }
        let mut i = whitespace(s, position + key.len());
        if s.as_bytes().get(i) != Some(&b'=') {
            continue;
        }
        i = whitespace(s, i + 1);
        if let Some((value, _)) = group(s, i, b'{', b'}') {
            return Some(value);
        }
        return Some(
            s[i..]
                .split(|c: char| c == ',' || c == ']' || c == '}' || c.is_whitespace())
                .next()?
                .to_string(),
        );
    }
    None
}

fn bibtex_bbl(entries: &[Entry], style: &str, natbib: bool) -> String {
    let mut s = format!(
        "% Pitex in-process bibliography\n\\begin{{thebibliography}}{{{}}}\n",
        entries.len().max(1)
    );
    for (index, e) in entries.iter().enumerate() {
        let label = if natbib {
            format!("[{}({})]", author_label(e), field(e, "year"))
        } else if style == "alpha" {
            let letters = author_label(e)
                .chars()
                .filter(|c| c.is_alphabetic())
                .take(3)
                .collect::<String>();
            let year = field(e, "year");
            format!(
                "[{letters}{}]",
                if year.len() >= 2 {
                    &year[year.len() - 2..]
                } else {
                    year
                }
            )
        } else {
            String::new()
        };
        s.push_str(&format!("\\bibitem{label}{{{}}}\n", e.key));
        let author = names_text(if field(e, "author").is_empty() {
            field(e, "editor")
        } else {
            field(e, "author")
        });
        if !author.is_empty() {
            s.push_str(&format!("{author}.\n\\newblock "));
        }
        if !field(e, "title").is_empty() {
            s.push_str(&format!(
                "{}{}.\n\\newblock ",
                if e.kind == "book" { "\\emph{" } else { "" },
                if e.kind == "book" {
                    format!("{}}}", field(e, "title"))
                } else {
                    field(e, "title").to_string()
                }
            ));
        }
        let journal = field(e, "journal");
        let booktitle = field(e, "booktitle");
        if !journal.is_empty() {
            s.push_str(&format!("\\emph{{{journal}}}"));
        } else if !booktitle.is_empty() {
            s.push_str(&format!("In \\emph{{{booktitle}}}"));
        }
        for f in [
            "volume",
            "number",
            "pages",
            "publisher",
            "institution",
            "school",
            "address",
            "edition",
            "year",
            "note",
        ] {
            let v = field(e, f);
            if !v.is_empty() {
                s.push_str(&format!(", {v}"));
            }
        }
        s.push_str(".\n");
        let doi = field(e, "doi");
        let url = field(e, "url");
        if !doi.is_empty() {
            s.push_str(&format!(
                "\\newblock DOI: \\texttt{{{}}}.\n",
                tex_escape(doi)
            ));
        } else if !url.is_empty() {
            s.push_str(&format!("\\newblock \\texttt{{{}}}.\n", tex_escape(url)));
        }
        let _ = index;
    }
    s.push_str("\\end{thebibliography}\n");
    s
}
fn stable_hash(s: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}
fn initials(s: &str) -> String {
    s.split(|c: char| c.is_whitespace() || c == '-')
        .filter_map(|w| w.chars().find(|c| c.is_alphabetic()))
        .map(|c| format!("{c}\\bibinitperiod"))
        .collect::<Vec<_>>()
        .join("\\bibinitdelim ")
}
fn bib_year(e: &Entry) -> String {
    if !field(e, "year").is_empty() {
        field(e, "year").to_string()
    } else {
        field(e, "date").split('-').next().unwrap_or("").to_string()
    }
}
fn alpha_label(e: &Entry) -> String {
    let ns = names(if field(e, "author").is_empty() {
        field(e, "editor")
    } else {
        field(e, "author")
    });
    let family = if ns.len() == 1 {
        ns[0]
            .family
            .chars()
            .filter(|c| c.is_alphabetic())
            .take(3)
            .collect::<String>()
    } else if !ns.is_empty() {
        let mut initials = ns
            .iter()
            .take(3)
            .filter_map(|n| n.family.chars().find(|c| c.is_alphabetic()))
            .collect::<String>();
        if ns.len() > 3 {
            initials.push('+');
        }
        initials
    } else {
        field(e, "title")
            .chars()
            .filter(|c| c.is_alphabetic())
            .take(3)
            .collect()
    };
    let year = bib_year(e);
    let suffix = year
        .chars()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{family}{suffix}")
}
fn biblatex_bbl(
    entries: &[Entry],
    sorting: &str,
    load: &mut impl FnMut(&str) -> Option<String>,
    document: Option<&bibliography_control::Xml>,
    warnings: &mut Vec<String>,
) -> String {
    let package = load("biblatex.sty").unwrap_or_default();
    let version = package
        .split("\\def\\blx@bblversion{")
        .nth(1)
        .and_then(|s| s.split('}').next())
        .unwrap_or("3.2");
    let mut s=format!("% $ biblatex auxiliary file $\n% $ biblatex bbl format version {version} $\n% Pitex in-process bibliography data\n\\refsection{{0}}\n\\datalist[entry]{{{sorting}/global//global/global}}\n");
    let mut alpha_labels = BTreeMap::new();
    for entry in entries {
        let alpha = if let Some(root) = document {
            bibliography_control::alpha_label(entry, root, warnings)
        } else {
            alpha_label(entry)
        };
        alpha_labels.insert(entry.key.clone(), alpha);
    }
    let mut date_counts = BTreeMap::<String, usize>::new();
    let mut alpha_counts = BTreeMap::<String, usize>::new();
    for e in entries {
        let date = format!("{}:{}", field(e, "author"), bib_year(e));
        *date_counts.entry(date).or_default() += 1;
        *alpha_counts
            .entry(alpha_labels[&e.key].clone())
            .or_default() += 1;
    }
    let mut date_seen = BTreeMap::<String, usize>::new();
    let mut alpha_seen = BTreeMap::<String, usize>::new();
    for e in entries {
        s.push_str(&format!(
            "\\entry{{{}}}{{{}}}{{}}\n",
            e.key,
            if e.kind == "conference" {
                "inproceedings"
            } else {
                &e.kind
            }
        ));
        for role in ["author", "editor", "translator", "bookauthor"] {
            let ns = names(field(e, role));
            if ns.is_empty() {
                continue;
            }
            s.push_str(&format!("\\name{{{role}}}{{{}}}{{}}{{%\n", ns.len()));
            for n in ns {
                let hash = stable_hash(&format!("{} {}", n.family, n.given));
                s.push_str(&format!("{{{{hash={hash}}}{{family={{{}}}, familyi={{{}}}, given={{{}}}, giveni={{{}}}, prefix={{{}}}, prefixi={{{}}}, suffix={{{}}}, suffixi={{{}}}}}}}%\n",n.family,initials(&n.family),n.given,initials(&n.given),n.prefix,initials(&n.prefix),n.suffix,initials(&n.suffix)));
                let _ = n.literal;
            }
            s.push_str("}\n");
        }
        let hash = stable_hash(if field(e, "author").is_empty() {
            field(e, "editor")
        } else {
            field(e, "author")
        });
        for name in [
            "namehash",
            "fullhash",
            "bibnamehash",
            "authorbibnamehash",
            "authornamehash",
            "authorfullhash",
        ] {
            s.push_str(&format!("\\strng{{{name}}}{{{hash}}}\n"));
        }
        let label = if !field(e, "author").is_empty() {
            "author"
        } else {
            "editor"
        };
        s.push_str(&format!(
            "\\field{{labelnamesource}}{{{label}}}\n\\field{{labeltitlesource}}{{title}}\n"
        ));
        if !field(e, "year").is_empty() || !field(e, "date").is_empty() {
            s.push_str("\\field{extradatescope}{labelyear}\n\\field{labeldatesource}{}\n");
        }
        let date_key = format!("{}:{}", field(e, "author"), bib_year(e));
        if !bib_year(e).is_empty() && date_counts.get(&date_key).copied().unwrap_or(0) > 1 {
            let number = date_seen.entry(date_key).or_default();
            *number += 1;
            s.push_str(&format!("\\field{{extradate}}{{{number}}}\n"));
        }
        let alpha = alpha_labels[&e.key].clone();
        s.push_str(&format!("\\field{{labelalpha}}{{{alpha}}}\n"));
        if alpha_counts.get(&alpha).copied().unwrap_or(0) > 1 {
            let number = alpha_seen.entry(alpha).or_default();
            *number += 1;
            s.push_str(&format!("\\field{{extraalpha}}{{{number}}}\n"));
        }
        let sortinit = author_label(e).chars().next().unwrap_or('A');
        s.push_str(&format!(
            "\\field{{sortinit}}{{{sortinit}}}\n\\field{{sortinithash}}{{{hash}}}\n"
        ));
        for (key, value) in &e.fields {
            if [
                "author",
                "editor",
                "translator",
                "bookauthor",
                "crossref",
                "xdata",
                "entryset",
                "ids",
                "keywords",
                "options",
            ]
            .contains(&key.as_str())
            {
                continue;
            }
            let target = match key.as_str() {
                "journal" => "journaltitle",
                "address" => "location",
                "school" => "institution",
                _ => key,
            };
            if [
                "publisher",
                "location",
                "institution",
                "organization",
                "language",
            ]
            .contains(&target)
            {
                let items = split_top(value, " and ");
                s.push_str(&format!("\\list{{{target}}}{{{}}}{{", items.len()));
                for item in items {
                    s.push_str(&format!("{{{item}}}"));
                }
                s.push_str("}\n");
            } else if ["url", "doi", "eprint", "file"].contains(&target) {
                s.push_str(&format!(
                    "\\verb{{{target}}}\n\\verb {}\n\\endverb\n",
                    value
                ));
            } else if ["date", "urldate", "eventdate", "origdate"].contains(&target) {
                let date = value.split('-').collect::<Vec<_>>();
                let prefix = target.trim_end_matches("date");
                for (k, v) in ["year", "month", "day"].iter().zip(date) {
                    s.push_str(&format!("\\field{{{prefix}{k}}}{{{v}}}\n"));
                }
            } else {
                let numeric_month = if target == "month" {
                    [
                        "January",
                        "February",
                        "March",
                        "April",
                        "May",
                        "June",
                        "July",
                        "August",
                        "September",
                        "October",
                        "November",
                        "December",
                    ]
                    .iter()
                    .position(|month| month.eq_ignore_ascii_case(value))
                    .map(|index| (index + 1).to_string())
                } else {
                    None
                };
                s.push_str(&format!(
                    "\\field{{{target}}}{{{}}}\n",
                    numeric_month.as_deref().unwrap_or(value)
                ));
            }
        }
        if !field(e, "keywords").is_empty() {
            s.push_str(&format!("\\keyw{{{}}}\n", field(e, "keywords")));
        }
        s.push_str("\\endentry\n");
    }
    s.push_str("\\enddatalist\n\\endrefsection\n\\endinput\n");
    s
}
fn tex_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\textbackslash{}"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '#' | '$' | '%' | '&' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '^' => out.push_str("\\textasciicircum{}"),
            '~' => out.push_str("\\textasciitilde{}"),
            _ => out.push(c),
        }
    }
    out
}

fn highlight_source(
    path: &str,
    source: &str,
    load: &mut impl FnMut(&str) -> Option<String>,
    result: &mut Prepared,
) -> String {
    let mut s = source.to_string();
    // Replace only a package request containing minted, preserving the remaining
    // package list. The generated Verbatim inputs use fvextra and xcolor directly.
    let mut replacements = Vec::new();
    for command in ["usepackage", "RequirePackage"] {
        let needle = format!("\\{command}");
        let mut start = 0;
        while let Some(p) = s[start..].find(&needle) {
            let p = start + p;
            let mut i = p + needle.len();
            if let Some((_, end)) = group(&s, i, b'[', b']') {
                i = end;
            }
            let Some((packages, end)) = group(&s, i, b'{', b'}') else {
                start = i;
                continue;
            };
            start = end;
            if packages.split(',').any(|p| p.trim() == "minted") {
                let other = packages
                    .split(',')
                    .filter(|p| p.trim() != "minted")
                    .collect::<Vec<_>>()
                    .join(",");
                let mut replacement = String::new();
                if !other.is_empty() {
                    replacement.push_str(&format!("\\{command}{{{other}}}"));
                }
                replacement.push_str(&format!("\\{command}{{fvextra,xcolor,float}}"));
                replacement.push_str(r"\ifdefined\listing\else\newfloat{listing}{tbp}{lol}\floatname{listing}{Listing}\fi\ifdefined\listoflistings\else\newcommand{\listoflistings}{\listof{listing}{List of Listings}}\fi ");
                // Preserve line numbering of the source file for SyncTeX.
                replacement
                    .push_str(&"\n".repeat(s[p..end].bytes().filter(|b| *b == b'\n').count()));
                replacements.push((p, end, replacement));
            }
        }
    }
    for (p, end, text) in replacements.into_iter().rev() {
        s.replace_range(p..end, &text);
    }
    let mut defaults = String::new();
    for command in ["setminted", "setmintedinline", "usemintedstyle"] {
        let needle = format!("\\{command}");
        let mut offset = 0;
        while let Some(p) = s[offset..].find(&needle) {
            let p = offset + p;
            let mut i = p + needle.len();
            if let Some((_, end)) = group(&s, i, b'[', b']') {
                i = end;
            }
            if let Some((opts, end)) = group(&s, i, b'{', b'}') {
                defaults.push_str(&if command == "usemintedstyle" {
                    format!(",style={opts}")
                } else {
                    format!(",{opts}")
                });
                let new = "\n".repeat(s[p..end].bytes().filter(|b| *b == b'\n').count());
                s.replace_range(p..end, &new);
                offset = p + new.len();
            } else {
                offset = i;
            }
        }
    }
    // Resolve minted's common environment/inline aliases before highlighting.
    // Definitions are source adapters, not executable shell-generator stubs.
    for (declaration, environment) in [("newminted", true), ("newmintinline", false)] {
        let needle = format!("\\{declaration}");
        let mut offset = 0;
        let mut aliases = Vec::new();
        while let Some(p) = s[offset..].find(&needle) {
            let p = p + offset;
            let mut i = p + needle.len();
            let alias = group(&s, i, b'[', b']').map(|(name, end)| {
                i = end;
                name
            });
            let Some((language, end)) = group(&s, i, b'{', b'}') else {
                offset = i;
                continue;
            };
            i = end;
            let Some((options, end)) = group(&s, i, b'{', b'}') else {
                offset = i;
                continue;
            };
            let name = alias.unwrap_or_else(|| {
                format!("{language}{}", if environment { "code" } else { "inline" })
            });
            aliases.push((name, language, options));
            let replacement = "\n".repeat(s[p..end].bytes().filter(|b| *b == b'\n').count());
            s.replace_range(p..end, &replacement);
            offset = p + replacement.len();
        }
        for (alias, language, options) in aliases {
            if environment {
                let needle = format!("\\begin{{{alias}}}");
                let mut offset = 0;
                while let Some(p) = s[offset..].find(&needle) {
                    let p = p + offset;
                    let mut end = p + needle.len();
                    let mut opts = options.clone();
                    if let Some((extra, next)) = group(&s, end, b'[', b']') {
                        end = next;
                        opts.push_str(&format!(",{extra}"));
                    }
                    let replacement = format!("\\begin{{minted}}[{opts}]{{{language}}}");
                    s.replace_range(p..end, &replacement);
                    offset = p + replacement.len();
                }
                s = s.replace(&format!("\\end{{{alias}}}"), "\\end{minted}");
            } else {
                let needle = format!("\\{alias}");
                let mut offset = 0;
                while let Some(p) = s[offset..].find(&needle) {
                    let p = p + offset;
                    let end = p + needle.len();
                    offset = end;
                    if s.as_bytes()
                        .get(end)
                        .map(|c| c.is_ascii_alphabetic())
                        .unwrap_or(false)
                    {
                        continue;
                    }
                    let mut end = end;
                    let mut opts = options.clone();
                    if let Some((extra, next)) = group(&s, end, b'[', b']') {
                        end = next;
                        opts.push_str(&format!(",{extra}"));
                    }
                    let replacement = format!("\\mintinline[{opts}]{{{language}}}");
                    s.replace_range(p..end, &replacement);
                    offset = p + replacement.len();
                }
            }
        }
    }

    let mut offset = 0;
    let mut number = 0;
    while let Some(p) = s[offset..].find("\\begin{minted}") {
        let p = offset + p;
        let mut i = p + "\\begin{minted}".len();
        let mut options = defaults.clone();
        if let Some((opts, end)) = group(&s, i, b'[', b']') {
            options.push_str(&format!(",{opts}"));
            i = end;
        }
        let Some((language, end)) = group(&s, i, b'{', b'}') else {
            offset = i;
            continue;
        };
        i = end;
        let Some(end) = s[i..].find("\\end{minted}").map(|e| i + e) else {
            result
                .warnings
                .push(format!("Unclosed minted environment in '{path}'"));
            break;
        };
        let raw = &s[i..end];
        let code = raw
            .strip_prefix("\r\n")
            .or_else(|| raw.strip_prefix('\n'))
            .unwrap_or(raw);
        number += 1;
        let name = format!("pitex-code-{}-{number}.tex", stable_hash(path));
        let rendered = highlight_block(code, &language, &options, &mut result.warnings);
        result.files.insert(name.clone(), rendered);
        let until = end + "\\end{minted}".len();
        let replacement = format!(
            "\\input{{{name}}}{}",
            "\n".repeat(s[p..until].bytes().filter(|b| *b == b'\n').count())
        );
        s.replace_range(p..until, &replacement);
        offset = p + replacement.len();
    }
    for command in ["inputminted", "mintinline", "mint"] {
        let needle = format!("\\{command}");
        let mut offset = 0;
        while let Some(p) = s[offset..].find(&needle) {
            let p = offset + p;
            let mut i = p + needle.len();
            let mut options = defaults.clone();
            if let Some((opts, end)) = group(&s, i, b'[', b']') {
                options.push_str(&format!(",{opts}"));
                i = end;
            }
            let Some((lang, end)) = group(&s, i, b'{', b'}') else {
                offset = i;
                continue;
            };
            i = whitespace(&s, end);
            let value = if let Some(v) = group(&s, i, b'{', b'}') {
                Some(v)
            } else if command == "mintinline" || command == "mint" {
                let delim = s[i..].chars().next();
                delim.and_then(|d| {
                    s[i + d.len_utf8()..].find(d).map(|p| {
                        (
                            s[i + d.len_utf8()..i + d.len_utf8() + p].into(),
                            i + d.len_utf8() + p + d.len_utf8(),
                        )
                    })
                })
            } else {
                None
            };
            let Some((value, end)) = value else {
                offset = i;
                continue;
            };
            let replacement = if command == "mint" {
                number += 1;
                let name = format!("pitex-code-{}-{number}.tex", stable_hash(path));
                result.files.insert(
                    name.clone(),
                    highlight_block(&value, &lang, &options, &mut result.warnings),
                );
                format!("\\input{{{name}}}")
            } else if command == "inputminted" {
                let direct = load(&value);
                let relative = normalize(
                    &Path::new(path)
                        .parent()
                        .unwrap_or(Path::new(""))
                        .join(&value),
                );
                if let Some(code) = direct.or_else(|| load(&relative)) {
                    number += 1;
                    let name = format!("pitex-code-{}-{number}.tex", stable_hash(path));
                    result.files.insert(
                        name.clone(),
                        highlight_block(&code, &lang, &options, &mut result.warnings),
                    );
                    format!("\\input{{{name}}}")
                } else {
                    result
                        .warnings
                        .push(format!("minted input file '{value}' is unavailable"));
                    format!("\\texttt{{{}}}", tex_escape(&value))
                }
            } else {
                format!(
                    "{{\\ttfamily {}}}",
                    highlight_inline(&value, &lang, &mut result.warnings)
                )
            };
            let replacement = format!(
                "{replacement}{}",
                "\n".repeat(s[p..end].bytes().filter(|b| *b == b'\n').count())
            );
            s.replace_range(p..end, &replacement);
            offset = p + replacement.len();
        }
    }
    if s.contains("\\newmint") {
        result.warnings.push("minted custom command/environment declarations are not yet supported by the embedded highlighter".into());
    }
    s
}
#[derive(Clone, Copy)]
enum Kind {
    Normal,
    Keyword,
    String,
    Comment,
    Number,
}
fn language_family(language: &str) -> Option<&'static str> {
    match language.trim().to_lowercase().as_str() {
        "python" | "py" | "python3" => Some("python"),
        "c" | "cpp" | "c++" | "csharp" | "c#" | "java" | "javascript" | "js" | "typescript"
        | "ts" | "rust" | "rs" | "go" | "swift" | "kotlin" => Some("c"),
        "bash" | "sh" | "shell" | "zsh" => Some("shell"),
        "json" | "yaml" | "yml" | "toml" | "ini" => Some("data"),
        "tex" | "latex" => Some("tex"),
        "sql" => Some("sql"),
        "r" => Some("r"),
        "text" | "txt" => Some("text"),
        _ => None,
    }
}
fn lex(code: &str, language: &str, warnings: &mut Vec<String>) -> Vec<(Kind, String)> {
    let Some(family) = language_family(language) else {
        warnings.push(format!(
            "minted language '{language}' has no embedded lexer; verbatim text is shown"
        ));
        return vec![(Kind::Normal, code.into())];
    };
    let keywords=match family{
        "python"=>"False None True and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield print",
        "c"=>"abstract alignas alignof as asm async await auto bool boolean break case catch char class const constexpr continue crate default defer delete do double dyn else enum export extern false final finally float fn for friend func function goto if impl import in inline int interface let long match mod move mut namespace new noexcept null nullptr override package private protected pub public readonly register return sealed self short signed sizeof static struct super switch synchronized template this throw trait true try type typedef typeof union unsafe unsigned use using var virtual void volatile where while yield",
        "shell"=>"if then else elif fi for do done while until case esac function in local export readonly return exit source",
        "data"=>"true false null True False None",
        "sql"=>"SELECT FROM WHERE INSERT INTO VALUES UPDATE DELETE CREATE TABLE JOIN LEFT RIGHT INNER OUTER ON GROUP BY ORDER ASC DESC LIMIT HAVING AS AND OR NOT NULL IS DISTINCT UNION ALL ALTER DROP INDEX PRIMARY KEY REFERENCES",
        "r"=>"if else repeat while function for in next break TRUE FALSE NULL Inf NaN NA library require",
        _=>""
    };
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < code.len() {
        let start = i;
        let tail = &code[i..];
        let c = tail.chars().next().unwrap();
        let mut kind = Kind::Normal;
        if (family == "python" || family == "shell" || family == "r" || family == "data")
            && c == '#'
            || family == "tex" && c == '%'
            || family == "sql" && tail.starts_with("--")
            || family == "c" && tail.starts_with("//")
        {
            kind = Kind::Comment;
            i += tail.find('\n').unwrap_or(tail.len());
        } else if (family == "c" || family == "sql") && tail.starts_with("/*") {
            kind = Kind::Comment;
            i += tail[2..].find("*/").map(|p| p + 4).unwrap_or(tail.len());
        } else if c == '\'' || c == '"' || family == "c" && c == '`' {
            kind = Kind::String;
            let triple = family == "python" && tail.starts_with(&c.to_string().repeat(3));
            let delimiter = c.to_string().repeat(if triple { 3 } else { 1 });
            i += delimiter.len();
            while i < code.len() {
                if code[i..].starts_with(&delimiter) {
                    i += delimiter.len();
                    break;
                }
                let ch = code[i..].chars().next().unwrap();
                i += ch.len_utf8();
                if ch == '\\' && i < code.len() {
                    i += code[i..].chars().next().unwrap().len_utf8();
                }
            }
        } else if c.is_ascii_digit() {
            kind = Kind::Number;
            i += c.len_utf8();
            while i < code.len()
                && code[i..]
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
                    .unwrap_or(false)
            {
                i += code[i..].chars().next().unwrap().len_utf8();
            }
        } else if c.is_alphabetic() || c == '_' {
            i += c.len_utf8();
            while i < code.len()
                && code[i..]
                    .chars()
                    .next()
                    .map(|c| c.is_alphanumeric() || c == '_')
                    .unwrap_or(false)
            {
                i += code[i..].chars().next().unwrap().len_utf8();
            }
            let word = &code[start..i];
            if keywords.split_whitespace().any(|k| {
                if family == "sql" {
                    k.eq_ignore_ascii_case(word)
                } else {
                    k == word
                }
            }) {
                kind = Kind::Keyword;
            }
        } else if family == "tex" && c == '\\' {
            kind = Kind::Keyword;
            i += 1;
            while i < code.len() && code.as_bytes()[i].is_ascii_alphabetic() {
                i += 1;
            }
        } else {
            i += c.len_utf8();
        }
        tokens.push((kind, code[start..i].into()));
    }
    tokens
}
fn color(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Normal => None,
        Kind::Keyword => Some("0.12,0.20,0.65"),
        Kind::String => Some("0.60,0.12,0.16"),
        Kind::Comment => Some("0.30,0.45,0.30"),
        Kind::Number => Some("0.45,0.16,0.60"),
    }
}
fn highlight_inline(code: &str, language: &str, warnings: &mut Vec<String>) -> String {
    lex(code, language, warnings)
        .iter()
        .map(|(k, s)| match color(*k) {
            Some(c) => format!("\\textcolor[rgb]{{{c}}}{{{}}}", tex_escape(s)),
            None => tex_escape(s),
        })
        .collect()
}
fn highlight_block(
    code: &str,
    language: &str,
    options: &str,
    warnings: &mut Vec<String>,
) -> String {
    let mut fv = vec!["commandchars=\\\\\\{\\}".to_string()];
    let mut gobble = 0usize;
    let mut autogobble = false;
    let mut first = 1usize;
    let mut last = usize::MAX;
    for item in split_top(options, ",") {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let (k, v) = item.split_once('=').unwrap_or((item, "true"));
        match k.trim() {
            "linenos" => {
                if v != "false" {
                    fv.push("numbers=left".into());
                }
            }
            "fontsize" | "numbersep" | "tabsize" | "frame" | "framesep" | "framerule"
            | "rulecolor" | "label" | "labelposition" | "breaklines" | "breakanywhere"
            | "firstnumber" | "stepnumber" | "numberblanklines" | "highlightlines"
            | "highlightcolor" | "mathescape" | "showspaces" | "showtabs" => {
                fv.push(format!("{k}={v}"))
            }
            "gobble" => gobble = v.parse().unwrap_or(0),
            "autogobble" => autogobble = v != "false",
            "firstline" => first = v.parse().unwrap_or(1),
            "lastline" => last = v.parse().unwrap_or(usize::MAX),
            "style" => {
                if v != "default" {
                    warnings.push(format!("minted style '{v}' uses the embedded palette; Pygments theme definitions are not interpreted"));
                }
            }
            "cache" | "cachedir" | "outputdir" | "draft" | "final" => {}
            _ => warnings.push(format!(
                "minted option '{k}' is not yet supported by the embedded highlighter"
            )),
        }
    }
    let lines = code
        .lines()
        .enumerate()
        .filter(|(i, _)| i + 1 >= first && i + 1 <= last)
        .map(|(_, s)| s)
        .collect::<Vec<_>>();
    if autogobble {
        gobble = lines
            .iter()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.chars().take_while(|c| *c == ' ' || *c == '\t').count())
            .min()
            .unwrap_or(0);
    }
    let selected = lines
        .iter()
        .map(|s| s.chars().skip(gobble).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    let mut s = String::from("\\definecolor{PitexCodeKeyword}{rgb}{0.12,0.20,0.65}\n\\definecolor{PitexCodeString}{rgb}{0.60,0.12,0.16}\n\\definecolor{PitexCodeComment}{rgb}{0.30,0.45,0.30}\n\\definecolor{PitexCodeNumber}{rgb}{0.45,0.16,0.60}\n");
    s.push_str(&format!("\\begin{{Verbatim}}[{}]\n", fv.join(",")));
    for (kind, text) in lex(&selected, language, warnings) {
        // Each physical code line must stay a physical FancyVerb line. Close
        // color commands before a newline and reopen them on the next line.
        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                s.push('\n');
            }
            if !line.is_empty() {
                let escaped = tex_escape(line);
                if let Some(c) = color(kind) {
                    let name = match kind {
                        Kind::Keyword => "PitexCodeKeyword",
                        Kind::String => "PitexCodeString",
                        Kind::Comment => "PitexCodeComment",
                        Kind::Number => "PitexCodeNumber",
                        _ => unreachable!(),
                    };
                    let _ = c;
                    s.push_str(&format!("\\textcolor{{{name}}}{{{escaped}}}"));
                } else {
                    s.push_str(&escaped);
                }
            }
        }
    }
    s.push_str("\n\\end{Verbatim}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bib_macros_crossrefs_and_names() {
        let (mut es, ws) = parse_bib(
            r#"@string{j = "Journal" # " Name"} @article{key,author={Doe, Jane and {Research Group}},title={{Nested} title},journal=j,year=2025} @book{parent,title={Book}}"#,
        );
        assert!(ws.is_empty());
        assert_eq!(field(&es.remove(0), "journal"), "Journal Name");
        assert_eq!(
            names("Doe, Jane and {Research Group}")[1].family,
            "Research Group"
        );
    }
    #[test]
    fn bibliography_generates_without_programs() {
        let map = BTreeMap::from([
            (
                "main.tex".to_string(),
                "\\usepackage[backend=biber]{biblatex}\\addbibresource{refs.bib}\\autocite{doe}"
                    .to_string(),
            ),
            (
                "refs.bib".into(),
                "@article{doe,author={Jane Doe},title={Paper},year=2025}".into(),
            ),
        ]);
        let result = prepare("main.tex", |p| map.get(p).cloned());
        let bbl = &result.files["main.bbl"];
        assert!(bbl.contains("\\entry{doe}{article}"));
        assert!(bbl.contains("family={Doe}"));
        assert!(bbl.contains("\\field{year}{2025}"));
    }
    #[test]
    fn prepared_bbl_is_preserved() {
        let map: BTreeMap<String, String> = BTreeMap::from([
            ("main.tex".into(), "\\bibliography{refs}".into()),
            ("main.bbl".into(), "user cache".into()),
        ]);
        assert!(!prepare("main.tex", |p| map.get(p).cloned())
            .files
            .contains_key("main.bbl"));
    }
    #[test]
    fn minted_escapes_and_colors() {
        let map:BTreeMap<String,String>=BTreeMap::from([("main.tex".into(),"\\usepackage{minted}\n\\begin{minted}[linenos]{python}\ndef f(x):\n    return \"a_{%}\\\\\" # comment\n\\end{minted}".into())]);
        let p = prepare("main.tex", |s| map.get(s).cloned());
        assert!(p.files["main.tex"].contains("fvextra,xcolor"));
        let code = p
            .files
            .iter()
            .find(|(k, _)| k.starts_with("pitex-code-"))
            .unwrap()
            .1;
        assert!(code.contains("numbers=left"));
        assert!(code.contains("textcolor"));
        assert!(code.contains("\\_"));
        assert!(!p.files["main.tex"].contains("\\begin{minted}"));
    }
    #[test]
    fn braces_and_utf8_are_retained() {
        let p = highlight_block("let s = \"한국어\"; // {x}\n", "rust", "", &mut vec![]);
        assert!(p.contains("한국어"));
        assert!(p.contains("\\{x\\}"));
    }
    #[test]
    fn source_citation_order_and_crossref() {
        let map:BTreeMap<String,String>=BTreeMap::from([
            ("main.tex".into(),r"\usepackage{natbib}\citet{second}\cite{first}\bibliographystyle{unsrtnat}\bibliography{refs}".into()),
            ("refs.bib".into(),r"@article{first,title={First},year=2024} @article{second,title={Second},crossref={parent}} @book{parent,year=2025,publisher={Press}}".into())]);
        let p = prepare("main.tex", |s| map.get(s).cloned());
        let bbl = &p.files["main.bbl"];
        assert!(bbl.find("{second}").unwrap() < bbl.find("{first}").unwrap());
        assert!(bbl.contains("2025"));
        assert!(bbl.contains("Press"));
    }
    #[test]
    fn minted_aliases_options_and_inline() {
        let source = r"\usepackage{minted}\newminted[pycode]{python}{linenos}\newmintinline[py]{python}{style=default}\begin{pycode}[autogobble]
    def f(): pass
\end{pycode}\py|print('x')|";
        let p = prepare("main.tex", |s| {
            if s == "main.tex" {
                Some(source.into())
            } else {
                None
            }
        });
        assert!(!p.files["main.tex"].contains("newmint"));
        assert!(!p.files["main.tex"].contains("\\py|"));
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        let code = p
            .files
            .iter()
            .find(|(path, _)| path.starts_with("pitex-code-"))
            .unwrap()
            .1;
        assert!(code.contains("numbers=left"));
        assert!(!code.contains("    \\textcolor"));
    }
    #[test]
    fn shell_escape_has_explicit_diagnostic() {
        let source = r"\immediate\write18{printf audit > sentinel.txt}";
        let p = prepare("main.tex", |s| {
            if s == "main.tex" {
                Some(source.into())
            } else {
                None
            }
        });
        assert!(p.files.is_empty());
        assert_eq!(p.warnings.len(), 1);
        assert!(p.warnings[0].contains("not executed"));
    }
    #[test]
    fn bibtex_von_and_suffix_names() {
        let n = name("Ludwig van Beethoven");
        assert_eq!(n.prefix, "van");
        assert_eq!(n.family, "Beethoven");
        assert_eq!(n.given, "Ludwig");
        let n = name("von Last, Jr, First");
        assert_eq!(n.suffix, "Jr");
        assert_eq!(n.prefix, "von");
        assert_eq!(n.family, "Last");
    }
    #[test]
    fn bibliography_year_and_alphabetic_disambiguation() {
        let source = r"\usepackage[style=authoryear]{biblatex}\addbibresource{refs.bib}\nocite{*}";
        let bib = r"@article{a,author={Jane Doe},title={A},year=2025} @article{b,author={Jane Doe},title={B},year=2025}";
        let p = prepare("main.tex", |path| match path {
            "main.tex" => Some(source.into()),
            "refs.bib" => Some(bib.into()),
            _ => None,
        });
        let bbl = &p.files["main.bbl"];
        assert!(bbl.contains("nyt/global"));
        assert!(bbl.contains("\\field{labelalpha}{Doe25}"));
        assert!(bbl.contains("\\field{extradate}{1}"));
        assert!(bbl.contains("\\field{extradate}{2}"));
        assert!(bbl.contains("\\field{extraalpha}{2}"));
    }
    #[test]
    fn executed_aux_resolves_dynamic_source_keys() {
        let data: BTreeMap<&str, &str> = BTreeMap::from([
            (
                "main.aux",
                r"\citation{actual-key}\bibdata{expanded-resource}\bibstyle{tiny}",
            ),
            (
                "expanded-resource.bib",
                r"@article{actual-key,title={Executed Citation}}",
            ),
            (
                "tiny.bst",
                r"ENTRY {title} {} {} FUNCTION {out} { title write$ newline$ } READ ITERATE {out}",
            ),
        ]);
        let prepared =
            prepare_from_controls("main.tex", |path| data.get(path).map(|s| s.to_string()));
        assert_eq!(prepared.files["main.bbl"], "Executed Citation\n");
        assert!(prepared.warnings.is_empty());
    }
    #[test]
    fn executed_bcf_preserves_two_reference_sections() {
        let bcf = r#"<bcf:controlfile><bcf:bibdata section="1"><bcf:datasource type="file" datatype="bibtex">refs.bib</bcf:datasource></bcf:bibdata><bcf:bibdata section="2"><bcf:datasource type="file" datatype="bibtex">refs.bib</bcf:datasource></bcf:bibdata><bcf:section number="1"><bcf:citekey order="1">a</bcf:citekey></bcf:section><bcf:section number="2"><bcf:citekey order="1">b</bcf:citekey></bcf:section><bcf:datalist section="1" type="entry" name="none/global//global/global" sortingtemplatename="none"/><bcf:datalist section="2" type="entry" name="none/global//global/global" sortingtemplatename="none"/></bcf:controlfile>"#;
        let bib = r"@article{a,author={Jane Doe},title={Section One},year=2025}@article{b,author={John Smith},title={Section Two},year=2024}";
        let prepared = prepare_from_controls("main.tex", |path| match path {
            "main.bcf" => Some(bcf.into()),
            "refs.bib" => Some(bib.into()),
            _ => None,
        });
        let bbl = &prepared.files["main.bbl"];
        assert!(bbl.contains("\\refsection{1}"));
        assert!(bbl.contains("\\refsection{2}"));
        assert_eq!(bbl.matches("\\entry{a}").count(), 1);
        assert_eq!(bbl.matches("\\entry{b}").count(), 1);
        assert_eq!(bbl.matches("\\endinput").count(), 1);
    }
    #[test]
    fn mixed_package_lists_and_local_requirepackage() {
        let source = r"\documentclass{article}\usepackage{localstyle}";
        let local = r"\RequirePackage{babel,microtype,xcolor}\RequirePackage{babel,minted,xcolor}";
        let prepared = prepare("main.tex", |path| match path {
            "main.tex" => Some(source.into()),
            "localstyle.sty" => Some(local.into()),
            _ => None,
        });
        assert!(prepared.files["main.tex"].contains("ifdefined\\PitexFontExpansion"));
        assert!(prepared.files["localstyle.sty"].contains("fvextra,xcolor,float"));
    }
    #[test]
    fn local_documentclass_loadclass_package_traversal() {
        let source = r"\documentclass{outer}";
        let outer = r"\ProvidesClass{outer}\LoadClass{inner}";
        let inner = r"\ProvidesClass{inner}\RequirePackage{babel,microtype,xcolor}";
        let prepared = prepare("main.tex", |path| match path {
            "main.tex" => Some(source.into()),
            "outer.cls" => Some(outer.into()),
            "inner.cls" => Some(inner.into()),
            _ => None,
        });
        assert!(prepared.files["main.tex"].contains("ifdefined\\PitexFontExpansion"));
    }
}
