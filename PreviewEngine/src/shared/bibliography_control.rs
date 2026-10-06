// Pitex-authored bibliography control-file reader. SPDX-License-Identifier: AGPL-3.0-or-later
// Reads data emitted by TeX, not a source approximation. XML has no external
// entity resolution, network access, executable extensions, or subprocesses.
use super::{command_args, Entry};
#[path = "bibliography_icu.rs"]
mod bibliography_icu;
use std::collections::BTreeMap;
#[derive(Clone, Default, Debug)]
pub(super) struct Xml {
    pub name: String,
    pub attrs: BTreeMap<String, String>,
    pub text: String,
    pub children: Vec<Xml>,
}
impl Xml {
    pub fn children<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Xml> {
        self.children.iter().filter(move |n| n.name == name)
    }
    pub fn attr(&self, name: &str) -> &str {
        self.attrs.get(name).map(String::as_str).unwrap_or("")
    }
    pub fn child(&self, name: &str) -> Option<&Xml> {
        self.children.iter().find(|n| n.name == name)
    }
}
fn unescape(s: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    while i < s.len() {
        if s.as_bytes()[i] == b'&' {
            if let Some(end) = s[i..].find(';') {
                let entity = &s[i + 1..i + end];
                let decoded = match entity {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    _ if entity.starts_with("#x") => u32::from_str_radix(&entity[2..], 16)
                        .ok()
                        .and_then(char::from_u32),
                    _ if entity.starts_with('#') => {
                        entity[1..].parse().ok().and_then(char::from_u32)
                    }
                    _ => None,
                };
                if let Some(c) = decoded {
                    result.push(c);
                    i += end + 1;
                    continue;
                }
            }
        }
        let c = s[i..].chars().next().unwrap();
        result.push(c);
        i += c.len_utf8();
    }
    result
}
pub(super) fn xml(s: &str) -> Result<Xml, String> {
    let mut stack = vec![Xml {
        name: "root".into(),
        ..Xml::default()
    }];
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with("<!--") {
            i += s[i..].find("-->").ok_or("Unclosed XML comment")? + 3;
            continue;
        }
        if s[i..].starts_with("<?") {
            i += s[i..].find("?>").ok_or("Unclosed XML declaration")? + 2;
            continue;
        }
        if s[i..].starts_with("<!") {
            return Err("Unsupported XML declaration in bibliography control file".into());
        }
        if s.as_bytes()[i] != b'<' {
            let end = s[i..].find('<').map(|p| i + p).unwrap_or(s.len());
            stack
                .last_mut()
                .unwrap()
                .text
                .push_str(&unescape(&s[i..end]));
            i = end;
            continue;
        }
        let mut end = i + 1;
        let mut quote = None;
        while end < s.len() {
            let c = s.as_bytes()[end];
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
            } else if c == b'\'' || c == b'"' {
                quote = Some(c);
            } else if c == b'>' {
                break;
            }
            end += 1;
        }
        if end == s.len() {
            return Err("Unclosed XML tag in bibliography control file".into());
        }
        let tag = s[i + 1..end].trim();
        i = end + 1;
        if let Some(close) = tag.strip_prefix('/') {
            if stack.len() < 2 {
                return Err("Unexpected XML closing tag".into());
            }
            let node = stack.pop().unwrap();
            if node.name != close.trim().split(':').last().unwrap_or("") {
                return Err("Mismatched XML closing tag".into());
            }
            stack.last_mut().unwrap().children.push(node);
            continue;
        }
        let empty = tag.ends_with('/');
        let tag = tag.trim_end_matches('/').trim();
        let split = tag.find(char::is_whitespace).unwrap_or(tag.len());
        let name = tag[..split].split(':').last().unwrap_or("").to_string();
        let mut attrs = BTreeMap::new();
        let mut p = split;
        while p < tag.len() {
            while p < tag.len() && tag.as_bytes()[p].is_ascii_whitespace() {
                p += 1;
            }
            if p == tag.len() {
                break;
            }
            let start = p;
            while p < tag.len() && !b"= \t\r\n".contains(&tag.as_bytes()[p]) {
                p += 1;
            }
            let key = &tag[start..p];
            while p < tag.len() && tag.as_bytes()[p].is_ascii_whitespace() {
                p += 1;
            }
            if tag.as_bytes().get(p) != Some(&b'=') {
                return Err("XML attribute is missing =".into());
            }
            p += 1;
            while p < tag.len() && tag.as_bytes()[p].is_ascii_whitespace() {
                p += 1;
            }
            let q = *tag.as_bytes().get(p).ok_or("Missing XML attribute value")?;
            if q != b'\'' && q != b'"' {
                return Err("Unquoted XML attribute".into());
            }
            p += 1;
            let start = p;
            while p < tag.len() && tag.as_bytes()[p] != q {
                p += 1;
            }
            if p == tag.len() {
                return Err("Unclosed XML attribute".into());
            }
            attrs.insert(key.into(), unescape(&tag[start..p]));
            p += 1;
        }
        let node = Xml {
            name,
            attrs,
            ..Xml::default()
        };
        if empty {
            stack.last_mut().unwrap().children.push(node);
        } else {
            stack.push(node);
            if stack.len() > 256 {
                return Err("Bibliography XML nesting limit exceeded".into());
            }
        }
    }
    if stack.len() != 1 {
        return Err("Incomplete bibliography XML".into());
    }
    let root = stack.pop().unwrap();
    root.children
        .into_iter()
        .find(|n| n.name == "controlfile")
        .ok_or_else(|| "No bibliography controlfile XML root".into())
}
#[derive(Clone, Default, Debug)]
pub(super) struct Section {
    pub number: String,
    pub resources: Vec<String>,
    pub keys: Vec<String>,
    pub lists: Vec<List>,
}
#[derive(Clone, Default, Debug)]
pub(super) struct List {
    pub name: String,
    pub sorting: String,
    pub kind: String,
    pub locale: String,
    pub name_template: String,
    pub section: String,
}
#[derive(Clone, Default, Debug)]
pub(super) struct Control {
    pub biblatex: bool,
    pub sections: Vec<Section>,
    pub style: String,
    pub document: Option<Xml>,
}
pub(super) fn read(
    main: &str,
    load: &mut impl FnMut(&str) -> Option<String>,
) -> Result<Option<Control>, String> {
    let job = std::path::Path::new(main)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let current_aux = load(&format!("{job}.aux"));
    let use_bcf = current_aux
        .as_ref()
        .map(|aux| aux.contains("\\abx@aux@"))
        .unwrap_or(true);
    if let Some(xml_text) = if use_bcf {
        load(&format!("{job}.bcf"))
    } else {
        None
    } {
        if xml_text.contains("<bcf:controlfile") && xml_text.contains("</bcf:controlfile>") {
            let root = xml(&xml_text)?;
            let mut sections = BTreeMap::<String, Section>::new();
            for node in root.children("bibdata") {
                let number = node.attr("section").to_string();
                let section = sections.entry(number.clone()).or_insert_with(|| Section {
                    number,
                    ..Section::default()
                });
                for source in node.children("datasource") {
                    if source.attr("type") == "file"
                        && (source.attr("datatype") == "bibtex"
                            || source.attr("datatype").is_empty())
                    {
                        section.resources.push(source.text.trim().into());
                    }
                }
            }
            for node in root.children("section") {
                let number = node.attr("number").to_string();
                let section = sections.entry(number.clone()).or_insert_with(|| Section {
                    number,
                    ..Section::default()
                });
                let mut keys = node.children("citekey").collect::<Vec<_>>();
                keys.sort_by_key(|n| {
                    (
                        n.attr("order").parse::<usize>().unwrap_or(0),
                        n.attr("intorder").parse::<usize>().unwrap_or(0),
                    )
                });
                for key in keys {
                    section.keys.push(key.text.trim().into());
                }
            }
            for node in root.children("datalist") {
                let number = node.attr("section").to_string();
                let section = sections.entry(number.clone()).or_insert_with(|| Section {
                    number,
                    ..Section::default()
                });
                section.lists.push(List {
                    name: node.attr("name").into(),
                    sorting: node.attr("sortingtemplatename").into(),
                    kind: node.attr("type").into(),
                    locale: node.attr("sortlocale").into(),
                    name_template: node.attr("sortingnamekeytemplatename").into(),
                    section: node.attr("section").into(),
                });
            }
            let global = sections
                .get("0")
                .map(|s| s.resources.clone())
                .unwrap_or_default();
            for section in sections.values_mut() {
                if section.resources.is_empty() {
                    section.resources = global.clone();
                }
            }
            let mut sections = sections.into_values().collect::<Vec<_>>();
            sections.sort_by_key(|s| s.number.parse::<usize>().unwrap_or(0));
            return Ok(Some(Control {
                biblatex: true,
                sections,
                document: Some(root),
                ..Control::default()
            }));
        }
    }
    let mut pending = vec![format!("{job}.aux")];
    let mut seen = std::collections::BTreeSet::new();
    let mut text = String::new();
    while let Some(path) = pending.pop() {
        if !seen.insert(path.clone()) || seen.len() > 256 {
            continue;
        }
        if let Some(source) = load(&path) {
            pending.extend(command_args(&source, "@input"));
            text.push_str(&source);
            text.push('\n');
        }
    }
    let resources = command_args(&text, "bibdata");
    if resources.is_empty() {
        return Ok(None);
    }
    let keys = command_args(&text, "citation")
        .into_iter()
        .flat_map(|s| {
            s.split(',')
                .map(str::trim)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect();
    let style = command_args(&text, "bibstyle")
        .first()
        .cloned()
        .unwrap_or_else(|| "plain".into());
    Ok(Some(Control {
        biblatex: false,
        style,
        sections: vec![Section {
            number: "0".into(),
            resources,
            keys,
            ..Section::default()
        }],
        ..Control::default()
    }))
}
// Read actual emitted option scopes; per-entry data options take precedence.
fn option(root: &Xml, component: &str, name: &str, entry: Option<&Entry>) -> Option<String> {
    let mut value = None;
    for scope in ["global", entry.map(|e| e.kind.as_str()).unwrap_or("global")] {
        for options in root
            .children("options")
            .filter(|n| n.attr("component") == component && n.attr("type") == scope)
        {
            for option in options.children("option") {
                if option
                    .child("key")
                    .is_some_and(|key| key.text.trim() == name)
                {
                    value = option.child("value").map(|v| v.text.trim().to_string());
                }
            }
        }
    }
    if let Some(entry) = entry {
        if let Some(setting) = super::option_value(super::field(entry, "options"), name) {
            value = Some(setting);
        }
    }
    value
}
fn boolean(value: Option<String>, default: bool) -> bool {
    value
        .as_deref()
        .map(|v| ["1", "true"].contains(&v))
        .unwrap_or(default)
}
fn locale_name(locale: &str) -> &str {
    match locale {
        "" | "auto" | "english" | "american" | "USenglish" => "en_US",
        "british" | "UKenglish" => "en_GB",
        "german" | "ngerman" => "de_DE",
        "austrian" | "naustrian" => "de_AT",
        "french" => "fr_FR",
        "canadian" => "fr_CA",
        "swedish" => "sv_SE",
        "danish" => "da_DK",
        "norsk" | "norwegian" => "nb_NO",
        "finnish" => "fi_FI",
        "dutch" => "nl_NL",
        "spanish" => "es_ES",
        "italian" => "it_IT",
        "portuguese" => "pt_PT",
        "brazilian" => "pt_BR",
        "russian" => "ru_RU",
        "polish" => "pl_PL",
        "czech" => "cs_CZ",
        "slovak" => "sk_SK",
        "hungarian" => "hu_HU",
        "turkish" => "tr_TR",
        "greek" => "el_GR",
        "korean" => "ko_KR",
        "japanese" => "ja_JP",
        "chinese" => "zh_CN",
        "arabic" => "ar",
        "hebrew" => "he",
        "icelandic" => "is_IS",
        "estonian" => "et_EE",
        "latvian" => "lv_LV",
        "lithuanian" => "lt_LT",
        "ukrainian" => "uk_UA",
        _ => locale,
    }
}
fn sort_item(value: &str, item: &Xml) -> String {
    let mut result = value.to_string();
    if !item.attr("substring_width").is_empty() || !item.attr("substring_side").is_empty() {
        let mut part = item.clone();
        if part.attr("substring_width").is_empty() {
            part.attrs.insert("substring_width".into(), "4".into());
        }
        result = substring(&result, &part);
    }
    if ["pad_width", "pad_side", "pad_char"]
        .iter()
        .any(|a| !item.attr(a).is_empty())
    {
        let width = item
            .attr("pad_width")
            .parse::<usize>()
            .unwrap_or(4)
            .min(65_536);
        let padding = item
            .attr("pad_char")
            .chars()
            .next()
            .unwrap_or('0')
            .to_string()
            .repeat(width.saturating_sub(result.chars().count()));
        result = if item.attr("pad_side") == "right" {
            format!("{result}{padding}")
        } else {
            format!("{padding}{result}")
        };
    }
    result
}
fn sort_names(entry: &Entry, field: &str, root: &Xml, list: &List) -> String {
    let field = if field == "labelname" {
        ["author", "editor", "translator"]
            .into_iter()
            .find(|f| !super::field(entry, f).is_empty())
            .unwrap_or("author")
    } else {
        field
    };
    let mut names = super::names(super::field(entry, field));
    let max = option(root, "biblatex", "maxsortnames", Some(entry))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    if names.len() > max {
        let min = option(root, "biblatex", "minsortnames", Some(entry))
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1);
        names.truncate(min);
    }
    let prefix = boolean(option(root, "biblatex", "useprefix", Some(entry)), false);
    let initials = boolean(
        option(root, "biblatex", "sortgiveninits", Some(entry)),
        false,
    );
    let template_name = super::option_value(super::field(entry, "options"), "sortnamekeytemplate")
        .unwrap_or_else(|| {
            if list.name_template.is_empty() {
                "global".into()
            } else {
                list.name_template.clone()
            }
        });
    let template = root
        .children("sortingnamekeytemplate")
        .find(|n| n.attr("name") == template_name);
    names
        .iter()
        .map(|name| {
            let part_value = |part: &str| match part {
                "family" => name.family.clone(),
                "given" => {
                    if initials {
                        super::initials(&name.given)
                    } else {
                        name.given.clone()
                    }
                }
                "prefix" => name.prefix.clone(),
                "suffix" => name.suffix.clone(),
                _ => String::new(),
            };
            if let Some(template) = template {
                let mut keys = template.children("keypart").collect::<Vec<_>>();
                keys.sort_by_key(|n| n.attr("order").parse::<usize>().unwrap_or(0));
                keys.iter()
                    .map(|key| {
                        let mut parts = key.children("part").collect::<Vec<_>>();
                        parts.sort_by_key(|n| n.attr("order").parse::<usize>().unwrap_or(0));
                        parts
                            .iter()
                            .filter(|part| {
                                part.attr("use").is_empty() || (part.attr("use") == "1") == prefix
                            })
                            .map(|part| {
                                if part.attr("type") == "literal" {
                                    part.text.trim().to_string()
                                } else {
                                    sort_item(&part_value(part.text.trim()), part)
                                }
                            })
                            .filter(|s| !s.is_empty())
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            } else if prefix {
                format!(
                    "{} {} {} {}",
                    name.prefix,
                    name.family,
                    part_value("given"),
                    name.suffix
                )
            } else {
                format!(
                    "{} {} {} {}",
                    name.family,
                    part_value("given"),
                    name.suffix,
                    name.prefix
                )
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
// Alternatives, per-sort options, final keys and refcontext locales come from
// the BCF generated by TeX. ICU sort keys are computed once per entry/group.
pub(super) fn sort(
    entries: &mut Vec<Entry>,
    list: &List,
    root: &Xml,
    warnings: &mut Vec<String>,
) -> bool {
    if list.sorting == "none" {
        return true;
    }
    let Some(template) = root
        .children("sortingtemplate")
        .find(|n| n.attr("name") == list.sorting)
    else {
        return false;
    };
    let citation_order = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.key.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let mut emitted_order = BTreeMap::new();
    for section in root.children("section").filter(|section| {
        section.attr("number")
            == if list.section.is_empty() {
                "0"
            } else {
                &list.section
            }
    }) {
        for cite in section.children("citekey") {
            let order = cite.attr("order").parse::<usize>().unwrap_or(0);
            let internal = cite.attr("intorder").parse::<usize>().unwrap_or(0);
            emitted_order
                .entry(cite.text.trim().to_string())
                .or_insert((order, internal));
        }
    }
    let mut groups = template.children("sort").collect::<Vec<_>>();
    groups.sort_by_key(|n| n.attr("order").parse::<usize>().unwrap_or(0));
    let global_locale =
        option(root, "biblatex", "sortlocale", None).unwrap_or_else(|| "en_US".into());
    let list_locale = if list.locale.is_empty() {
        global_locale.as_str()
    } else {
        list.locale.as_str()
    };
    let template_locale = if template.attr("locale").is_empty() {
        list_locale
    } else {
        template.attr("locale")
    };
    let global_case = boolean(option(root, "biber", "sortcase", None), true);
    let global_upper = boolean(option(root, "biber", "sortupper", None), true);
    let mut collators = Vec::new();
    for group in &groups {
        let locale = if group.attr("locale").is_empty() {
            template_locale
        } else {
            group.attr("locale")
        };
        let case_setting = if group.attr("sortcase").is_empty() {
            group.attr("sort_case")
        } else {
            group.attr("sortcase")
        };
        let upper_setting = if group.attr("sortupper").is_empty() {
            group.attr("sort_upper")
        } else {
            group.attr("sortupper")
        };
        let case = boolean(
            (!case_setting.is_empty()).then(|| case_setting.into()),
            global_case,
        );
        let upper = boolean(
            (!upper_setting.is_empty()).then(|| upper_setting.into()),
            global_upper,
        );
        match bibliography_icu::Collator::new(locale_name(locale), case, upper) {
            Ok(collator) => collators.push(collator),
            Err(error) => {
                let warning =
                    format!("Bibliography locale collation failed for '{locale}': {error}");
                if !warnings.contains(&warning) {
                    warnings.push(warning);
                }
                return false;
            }
        }
    }
    let keys = entries
        .iter()
        .map(|entry| {
            let mut result = Vec::new();
            for (group, collator) in groups.iter().zip(&collators) {
                let mut items = group.children("sortitem").collect::<Vec<_>>();
                items.sort_by_key(|n| n.attr("order").parse::<usize>().unwrap_or(0));
                let mut value = String::new();
                for item in items {
                    let field = item.text.trim();
                    let candidate = if item.attr("literal") == "1" {
                        field.into()
                    } else if [
                        "author",
                        "editor",
                        "translator",
                        "sortname",
                        "labelname",
                        "bookauthor",
                    ]
                    .contains(&field)
                    {
                        sort_names(entry, field, root, list)
                    } else if field == "citeorder" || field == "intciteorder" {
                        let order = emitted_order
                            .get(&entry.key)
                            .map(|(order, internal)| {
                                if field == "citeorder" {
                                    *order
                                } else {
                                    *internal
                                }
                            })
                            .unwrap_or_else(|| {
                                citation_order.get(&entry.key).copied().unwrap_or(0)
                            });
                        format!("{:020}", order)
                    } else if field == "presort" && super::field(entry, field).is_empty() {
                        root.child("presort")
                            .map(|n| n.text.clone())
                            .unwrap_or_default()
                    } else if field == "year" {
                        super::bib_year(entry)
                    } else {
                        super::field(entry, field).to_string()
                    };
                    if !candidate.is_empty() {
                        value = sort_item(&candidate, item);
                        break;
                    }
                }
                let present = !value.is_empty();
                // Braces protect TeX casing but are not letters in a collation key.
                let value = value.replace(['{', '}'], "");
                let desc = group.attr("direction") == "descending"
                    || group.attr("sort_direction") == "descending";
                result.push((desc, collator.key(&value)));
                if present && ["1", "true"].contains(&group.attr("final")) {
                    break;
                }
            }
            (entry.key.clone(), result)
        })
        .collect::<BTreeMap<_, _>>();
    entries.sort_by(|a, b| {
        for ((desc, a), (_, b)) in keys[&a.key].iter().zip(&keys[&b.key]) {
            let cmp = a.cmp(b);
            if !cmp.is_eq() {
                return if *desc { cmp.reverse() } else { cmp };
            }
        }
        std::cmp::Ordering::Equal
    });
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_entities_sections_and_lists() {
        let source = r#"<?xml version="1.0"?><bcf:controlfile><bcf:bibdata section="1"><bcf:datasource type="file" datatype="bibtex">a&amp;b.bib</bcf:datasource></bcf:bibdata><bcf:section number="1"><bcf:citekey order="1">key&amp;1</bcf:citekey></bcf:section><bcf:datalist section="1" name="nyt/global//global/global" type="entry" sortingtemplatename="nyt"/></bcf:controlfile>"#;
        let c = read("main.tex", &mut |p| {
            if p == "main.bcf" {
                Some(source.into())
            } else {
                None
            }
        })
        .unwrap()
        .unwrap();
        assert_eq!(c.sections[0].resources[0], "a&b.bib");
        assert_eq!(c.sections[0].keys[0], "key&1");
        assert_eq!(c.sections[0].lists[0].sorting, "nyt");
    }
    #[test]
    fn aux_reads_nested_actual_keys() {
        let files = BTreeMap::from([
            (
                "main.aux",
                r"\bibstyle{plain}\bibdata{dynamic}\@input{chapter.aux}",
            ),
            ("chapter.aux", r"\citation{expanded-key}"),
        ]);
        let c = read("main.tex", &mut |p| files.get(p).map(|s| s.to_string()))
            .unwrap()
            .unwrap();
        assert_eq!(c.style, "plain");
        assert_eq!(c.sections[0].keys, ["expanded-key"]);
    }
}

// Apply the maps actually emitted by biblatex. This includes installed driver
// maps (legacy entry/field names) and document-declared field/type/regex maps.
pub(super) fn sourcemap(
    entries: &mut BTreeMap<String, Entry>,
    root: &Xml,
    warnings: &mut Vec<String>,
) {
    let Some(sourcemap) = root.child("sourcemap") else {
        return;
    };
    let mut removed = Vec::new();
    for (key, entry) in entries.iter_mut() {
        for maps in sourcemap.children("maps") {
            if maps.attr("datatype") != "bibtex" && !maps.attr("datatype").is_empty() {
                continue;
            }
            for map in maps.children("map") {
                for child in &map.children {
                    if !["map_step", "per_type", "per_nottype"].contains(&child.name.as_str()) {
                        let warning = format!(
                            "Bibliography sourcemap element '{}' is not yet implemented",
                            child.name
                        );
                        if !warnings.contains(&warning) {
                            warnings.push(warning);
                        }
                    }
                }
                let allowed = map
                    .children("per_type")
                    .map(|n| n.text.trim())
                    .collect::<Vec<_>>();
                let forbidden = map
                    .children("per_nottype")
                    .map(|n| n.text.trim())
                    .collect::<Vec<_>>();
                if !allowed.is_empty() && !allowed.contains(&entry.kind.as_str())
                    || forbidden.contains(&entry.kind.as_str())
                {
                    continue;
                }
                let overwrite =
                    map.attr("map_overwrite") == "1" || maps.attr("map_overwrite") == "1";
                let original_type = entry.kind.clone();
                let mut last_field = String::new();
                let mut last_value = String::new();
                for step in map.children("map_step") {
                    if step.attr("map_entry_null") == "1" {
                        removed.push(key.clone());
                        break;
                    }
                    let type_source = step.attr("map_type_source");
                    if !type_source.is_empty() {
                        if entry.kind != type_source {
                            if step.attr("map_final") == "1" {
                                break;
                            } else {
                                continue;
                            }
                        }
                        let target = step.attr("map_type_target");
                        if !target.is_empty() {
                            entry.kind = target.into();
                        }
                    }
                    let source = step.attr("map_field_source");
                    if !source.is_empty() {
                        let Some(value) = entry.fields.get(source).cloned() else {
                            if step.attr("map_final") == "1" {
                                break;
                            } else {
                                continue;
                            }
                        };
                        last_field = source.into();
                        last_value = value.clone();
                        let mut transformed = value.clone();
                        let pattern = if !step.attr("map_matchi").is_empty() {
                            step.attr("map_matchi")
                        } else {
                            step.attr("map_match")
                        };
                        let negated = if step.attr("map_notmatchi").is_empty() {
                            step.attr("map_notmatch")
                        } else {
                            step.attr("map_notmatchi")
                        };
                        if !pattern.is_empty() || !negated.is_empty() {
                            let pattern = if !pattern.is_empty() {
                                pattern
                            } else {
                                negated
                            };
                            let replacement = step
                                .attrs
                                .contains_key("map_replace")
                                .then(|| step.attr("map_replace"));
                            match bibliography_icu::regex(
                                pattern,
                                &value,
                                !step.attr("map_matchi").is_empty()
                                    || !step.attr("map_notmatchi").is_empty(),
                                replacement,
                            ) {
                                Ok((matches, replaced)) => {
                                    if if negated.is_empty() {
                                        !matches
                                    } else {
                                        matches
                                    } {
                                        if step.attr("map_final") == "1" {
                                            break;
                                        } else {
                                            continue;
                                        }
                                    }
                                    if let Some(replaced) = replaced {
                                        transformed = replaced;
                                    }
                                }
                                Err(error) => {
                                    let message=format!("Unsupported bibliography sourcemap regex '{pattern}': {error}");
                                    if !warnings.contains(&message) {
                                        warnings.push(message);
                                    }
                                    break;
                                }
                            }
                        }
                        if step.attr("map_lowercase") == "1" {
                            transformed = transformed.to_lowercase();
                        }
                        if step.attr("map_uppercase") == "1" {
                            transformed = transformed.to_uppercase();
                        }
                        let target = step.attr("map_field_target");
                        if !target.is_empty() {
                            if overwrite || !entry.fields.contains_key(target) {
                                entry.fields.insert(target.into(), transformed);
                                entry.fields.remove(source);
                            }
                        } else if transformed != value {
                            entry.fields.insert(source.into(), transformed);
                        }
                    }
                    let target = step.attr("map_field_set");
                    if !target.is_empty() {
                        if step.attr("map_null") == "1" {
                            entry.fields.remove(target);
                            continue;
                        }
                        let value = if step.attr("map_origfieldval") == "1" {
                            last_value.clone()
                        } else if step.attr("map_origfield") == "1" {
                            last_field.clone()
                        } else if step.attr("map_origentrytype") == "1" {
                            original_type.clone()
                        } else {
                            step.attr("map_field_value").to_string()
                        };
                        if step.attr("map_append") == "1" {
                            entry
                                .fields
                                .entry(target.into())
                                .or_default()
                                .push_str(&value);
                        } else if overwrite || !entry.fields.contains_key(target) {
                            entry.fields.insert(target.into(), value);
                        }
                    }
                    for attribute in step.attrs.keys() {
                        if ![
                            "map_entry_null",
                            "map_type_source",
                            "map_type_target",
                            "map_field_source",
                            "map_field_target",
                            "map_match",
                            "map_matchi",
                            "map_notmatch",
                            "map_notmatchi",
                            "map_replace",
                            "map_final",
                            "map_lowercase",
                            "map_uppercase",
                            "map_field_set",
                            "map_null",
                            "map_origfieldval",
                            "map_origfield",
                            "map_origentrytype",
                            "map_field_value",
                            "map_append",
                        ]
                        .contains(&attribute.as_str())
                        {
                            let message=format!("Bibliography sourcemap attribute '{attribute}' is not yet implemented");
                            if !warnings.contains(&message) {
                                warnings.push(message);
                            }
                        }
                    }
                }
            }
        }
    }
    for key in removed {
        entries.remove(&key);
    }
}

fn substring(s: &str, part: &Xml) -> String {
    let width = part.attr("substring_width").parse::<usize>().ok();
    let chars = s.chars().collect::<Vec<_>>();
    match width {
        Some(width) if part.attr("substring_side") == "right" => chars
            .into_iter()
            .rev()
            .take(width)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
        Some(width) => chars.into_iter().take(width).collect(),
        None => s.into(),
    }
}
pub(super) fn alpha_label(entry: &Entry, root: &Xml, warnings: &mut Vec<String>) -> String {
    let Some(template) = root
        .children("labelalphatemplate")
        .find(|n| n.attr("type") == "global")
    else {
        return super::alpha_label(entry);
    };
    let authors = super::names(if super::field(entry, "author").is_empty() {
        super::field(entry, "editor")
    } else {
        super::field(entry, "author")
    });
    let mut elements = template.children("labelelement").collect::<Vec<_>>();
    elements.sort_by_key(|n| n.attr("order").parse::<usize>().unwrap_or(0));
    let mut result = String::new();
    for element in elements {
        for part in element.children("labelpart") {
            if !part.attr("ifnames").is_empty()
                && part.attr("ifnames").parse::<usize>().ok() != Some(authors.len())
            {
                continue;
            }
            if !part.attr("substring_width").is_empty()
                && part.attr("substring_width").parse::<usize>().is_err()
            {
                let warning = format!(
                    "Bibliography variable-width labelalpha '{}' is not yet implemented",
                    part.attr("substring_width")
                );
                if !warnings.contains(&warning) {
                    warnings.push(warning);
                }
            }
            let field = part.text.trim();
            let known_field = root
                .child("datamodel")
                .and_then(|model| model.child("fields"))
                .map(|fields| {
                    fields
                        .children("field")
                        .any(|node| node.text.trim() == field)
                })
                .unwrap_or(false);
            let value = if part.attr("literal") == "1"
                || (!known_field
                    && !["labelname", "labelyear", "year"].contains(&field)
                    && !entry.fields.contains_key(field))
            {
                field.into()
            } else if field == "labelname" {
                authors
                    .iter()
                    .take(3)
                    .map(|name| {
                        let prefix = name
                            .prefix
                            .split_whitespace()
                            .filter_map(|token| token.chars().find(|c| c.is_alphabetic()))
                            .collect::<String>();
                        let family = name
                            .family
                            .chars()
                            .filter(|c| c.is_alphabetic())
                            .collect::<String>();
                        format!("{prefix}{}", substring(&family, part))
                    })
                    .collect::<String>()
            } else if field == "year" || field == "labelyear" {
                substring(&super::bib_year(entry), part)
            } else {
                substring(super::field(entry, field), part)
            };
            if value.is_empty() {
                continue;
            }
            result.push_str(&value);
            if part.attr("final") == "1" {
                return result;
            }
            for attribute in part.attrs.keys() {
                if ![
                    "literal",
                    "ifnames",
                    "final",
                    "substring_width",
                    "substring_side",
                ]
                .contains(&attribute.as_str())
                {
                    let warning = format!(
                        "Bibliography labelalpha attribute '{attribute}' is not yet implemented"
                    );
                    if !warnings.contains(&warning) {
                        warnings.push(warning);
                    }
                }
            }
            break;
        }
    }
    if result.is_empty() {
        super::alpha_label(entry)
    } else {
        result
    }
}
