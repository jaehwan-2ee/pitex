// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// Original implementation of PDF article threads and cyclic bead lists.
use super::*;
#[derive(Default)]
pub struct Threads {
    entries: BTreeMap<String, Article>,
    order: Vec<String>,
    active: Vec<(String, BTreeMap<i32, [f64; 4]>)>,
    active_levels: Vec<Option<i32>>,
    active_rules: Vec<RuleDimensions>,
    boxes: Vec<LayoutBox>,
    box_counter: usize,
    pages: BTreeMap<i32, Vec<i32>>,
    page_arrays: BTreeMap<i32, i32>,
    pub margin: f64,
    pub link_margin: f64,
    pub annotation_margin: f64,
    pub running_links: bool,
}
#[derive(Clone, Copy)]
struct LayoutBox {
    id: usize,
    level: i32,
    kind: char,
    rect: [f64; 4],
    local: [f64; 4],
    origin: [f64; 2],
    matrix: [f64; 6],
}
#[derive(Default, Clone, Copy)]
pub(super) struct RuleDimensions {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub depth: Option<f64>,
}
impl LayoutBox {
    fn bounds(&self, rule: RuleDimensions) -> [f64; 4] {
        let mut r = self.local;
        if let Some(w) = rule.width {
            r[2] = r[0] + w;
        }
        if let Some(h) = rule.height {
            r[3] = self.origin[1] + h;
        }
        if let Some(d) = rule.depth {
            r[1] = self.origin[1] - d;
        }
        let m = self.matrix;
        let points = [(r[0], r[1]), (r[0], r[3]), (r[2], r[1]), (r[2], r[3])]
            .map(|(x, y)| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]));
        [
            points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
            points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
            points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
            points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
        ]
    }
}
struct Article {
    object: i32,
    dict: BTreeMap<String, String>,
    beads: Vec<Bead>,
}
struct Bead {
    object: i32,
    page: i32,
    rect: [f64; 4],
}
impl Threads {
    pub unsafe fn reserve(c: *mut conv_ctx, pages: &[i32]) {
        for page in pages {
            let array = pdfw_alloc((*c).pw);
            (*(*c).document).threads.page_arrays.insert(*page, array);
        }
    }
    pub unsafe fn page(c: *mut conv_ctx) {
        let state = &mut (*(*c).document).threads;
        state.running_links = true;
        state.boxes.clear();
    }
    pub unsafe fn touch(c: *mut conv_ctx, x: f64, y: f64, w: f64, h: f64, d: f64) {
        if !(*c).render {
            return;
        }
        let r = Document::rectangle(c, [x, y - d, x + w, y + h]);
        Self::touch_rectangle(c, r);
    }
    pub unsafe fn touch_rectangle(c: *mut conv_ctx, r: [f64; 4]) {
        if !(*c).render {
            return;
        }
        let state = &mut (*(*c).document).threads;
        for (index, (_, pages)) in state.active.iter_mut().enumerate() {
            if state.active_levels.get(index).copied().flatten().is_some() {
                continue;
            }
            let box_ = pages.entry((*c).this_page).or_insert(r);
            box_[0] = box_[0].min(r[0]);
            box_[1] = box_[1].min(r[1]);
            box_[2] = box_[2].max(r[2]);
            box_[3] = box_[3].max(r[3]);
        }
    }
    unsafe fn bead(c: *mut conv_ctx, key: &str, page: i32, mut r: [f64; 4]) {
        let margin = (*(*c).document).threads.margin;
        r = [r[0] - margin, r[1] - margin, r[2] + margin, r[3] + margin];
        let object = pdfw_alloc((*c).pw);
        let state = &mut (*(*c).document).threads;
        if let Some(article) = state.entries.get_mut(key) {
            article.beads.push(Bead {
                object,
                page,
                rect: r,
            });
            state.pages.entry(page).or_default().push(object);
        }
    }
    pub(super) unsafe fn dimensions(c: *mut conv_ctx) -> Option<(f64, f64, f64)> {
        (*(*c).document).threads.boxes.last().map(|b| {
            (
                b.local[2] - b.local[0],
                b.local[3] - b.origin[1],
                b.origin[1] - b.local[1],
            )
        })
    }
    pub unsafe fn link_box(c: *mut conv_ctx) -> (Option<usize>, Option<i32>) {
        (*(*c).document)
            .threads
            .boxes
            .last()
            .map(|b| (Some(b.id), Some(b.level)))
            .unwrap_or((None, None))
    }
    pub unsafe fn link_enabled(c: *mut conv_ctx, start: Option<usize>, level: Option<i32>) -> bool {
        let state = &(*(*c).document).threads;
        if let Some(start) = start {
            if state.boxes.iter().any(|b| b.id == start) {
                return true;
            }
        }
        state.running_links
            && (level.is_none() || state.boxes.iter().any(|b| Some(b.level) == level))
    }
    pub unsafe fn special(c: *mut conv_ctx, command: &str, raw: &str, x: f64, y: f64) -> bool {
        match command {
            "boxbegin" => {
                let words = raw.splitn(4, ' ').collect::<Vec<_>>();
                if words.len() == 4 {
                    if let Ok(level) = words[0].parse::<i32>() {
                        let mut tail = words[3];
                        let mut bounds = [0.; 4];
                        let mut valid = true;
                        for v in &mut bounds {
                            if let Some((n, used)) = read_dimension(tail) {
                                *v = n;
                                tail = tail[used..].trim_start();
                            } else {
                                valid = false;
                                break;
                            }
                        }
                        if valid {
                            let rect = Document::rectangle(
                                c,
                                [x + bounds[0], y + bounds[1], x + bounds[2], y + bounds[3]],
                            );
                            let kind = words[1].chars().next().unwrap_or('h');
                            let state = &mut (*(*c).document).threads;
                            state.box_counter += 1;
                            state.boxes.push(LayoutBox {
                                id: state.box_counter,
                                level,
                                kind,
                                rect,
                                local: [x + bounds[0], y + bounds[1], x + bounds[2], y + bounds[3]],
                                origin: [x, y],
                                matrix: (*(*c).document).ctm,
                            });
                            if (*c).render && kind == 'v' {
                                let current = *state.boxes.last().unwrap();
                                let keys = state
                                    .active
                                    .iter()
                                    .enumerate()
                                    .filter(|(i, _)| {
                                        state.active_levels.get(*i).copied().flatten()
                                            == Some(level)
                                    })
                                    .map(|(i, a)| (a.0.clone(), state.active_rules[i]))
                                    .collect::<Vec<_>>();
                                for (key, rule) in keys {
                                    Self::bead(c, &key, (*c).this_page, current.bounds(rule));
                                }
                            }
                        }
                    }
                }
                true
            }
            "boxend" => {
                if let Ok(level) = raw.parse::<i32>() {
                    let boxes = &mut (*(*c).document).threads.boxes;
                    while boxes.last().is_some_and(|b| b.level >= level) {
                        boxes.pop();
                    }
                }
                true
            }

            "objcompresslevel" => {
                if let Ok(level) = raw.parse::<i32>() {
                    pdfw_object_compression((*c).pw, level);
                } else {
                    diagnostic(
                        c,
                        "object-streams",
                        "object compression level must be an integer",
                    );
                }
                true
            }
            "threadmargin" | "linkmargin" | "annotmargin" => {
                if let Some((value, _)) = read_dimension(raw) {
                    let state = &mut (*(*c).document).threads;
                    match command {
                        "threadmargin" => state.margin = value,
                        "linkmargin" => state.link_margin = value,
                        _ => state.annotation_margin = value,
                    }
                } else {
                    diagnostic(c, "margin", "margin must be a dimension");
                }
                true
            }
            "runninglink" => {
                (*(*c).document).threads.running_links = raw != "off" && raw != "0";
                true
            }
            "ethread" => {
                if !(*c).render {
                    return true;
                }
                if let Some((key, pages)) = (*(*c).document).threads.active.pop() {
                    (*(*c).document).threads.active_levels.pop();
                    (*(*c).document).threads.active_rules.pop();
                    for (page, r) in pages {
                        Self::bead(c, &key, page, r);
                    }
                } else {
                    diagnostic(c, "thread", "thread end has no matching start");
                }
                true
            }
            "thread" | "bthread" => {
                if !(*c).render {
                    return true;
                }
                let mut value = raw;
                let key;
                let mut title = None;
                if let Some(tail) = value.strip_prefix("num ") {
                    let n = token_end(tail);
                    key = format!("num:{}", &tail[..n]);
                    value = tail[n..].trim_start();
                } else if let Some(tail) = value.strip_prefix("name") {
                    let tail = tail.trim_start();
                    let ptr = tail.as_ptr().cast::<::core::ffi::c_char>();
                    let end = ptr.add(tail.len());
                    let n = skip_value(ptr, end).offset_from(ptr) as usize;
                    let Some(bytes) = pdf_string_bytes(&tail[..n]) else {
                        diagnostic(c, "thread", "thread name must be a PDF string");
                        return true;
                    };
                    key = format!("name:{bytes:?}");
                    title = Some(tail[..n].to_owned());
                    value = tail[n..].trim_start();
                } else {
                    diagnostic(c, "thread", "thread requires num or name identifier");
                    return true;
                }
                let (mut width, mut height, mut depth) = (0., 0., 0.);
                let mut supplied = false;
                let mut rule = RuleDimensions::default();
                while !value.is_empty() && !value.starts_with("<<") {
                    let n = ["width", "height", "depth"]
                        .iter()
                        .find(|key| value.starts_with(**key))
                        .map(|key| key.len())
                        .unwrap_or_else(|| token_end(value));
                    if n == 0 {
                        diagnostic(c, "thread", "invalid thread dimensions");
                        return true;
                    }
                    let dimension = &value[..n];
                    value = value[n..].trim_start();
                    let Some((number, used)) = read_dimension(value) else {
                        diagnostic(c, "thread", "invalid thread dimension");
                        return true;
                    };
                    value = value[used..].trim_start();
                    match dimension {
                        "width" => {
                            width = number;
                            rule.width = Some(number);
                        }
                        "height" => {
                            height = number;
                            rule.height = Some(number);
                        }
                        "depth" => {
                            depth = number;
                            rule.depth = Some(number);
                        }
                        _ => {
                            diagnostic(c, "thread", "unknown thread dimension");
                            return true;
                        }
                    }
                    supplied = true;
                }
                let attrs = substitute(c, &pdftex_references(value), x, y);
                let mut dict = dictionary(&attrs);
                if !dict.contains_key("I") {
                    if let Some(title) = title {
                        dict.insert("I".into(), format!("<</Title {title}>>"));
                    }
                }
                let present = (*(*c).document).threads.entries.contains_key(&key);
                if !present {
                    let object = pdfw_alloc((*c).pw);
                    let state = &mut (*(*c).document).threads;
                    state.order.push(key.clone());
                    state.entries.insert(
                        key.clone(),
                        Article {
                            object,
                            dict,
                            beads: Vec::new(),
                        },
                    );
                } else {
                    (*(*c).document)
                        .threads
                        .entries
                        .get_mut(&key)
                        .unwrap()
                        .dict
                        .extend(dict);
                }
                let rectangle = Document::rectangle(c, [x, y - depth, x + width, y + height]);
                if command == "bthread" {
                    let current = (*(*c).document)
                        .threads
                        .boxes
                        .last()
                        .filter(|b| b.kind == 'v')
                        .copied();
                    let mut pages = BTreeMap::new();
                    if let Some(box_) = current {
                        Self::bead(c, &key, (*c).this_page, box_.bounds(rule));
                        (*(*c).document)
                            .threads
                            .active_levels
                            .push(Some(box_.level));
                    } else {
                        if supplied {
                            pages.insert((*c).this_page, rectangle);
                        }
                        (*(*c).document).threads.active_levels.push(None);
                    }
                    (*(*c).document).threads.active.push((key.clone(), pages));
                    (*(*c).document).threads.active_rules.push(rule);
                } else {
                    Self::bead(c, &key, (*c).this_page, rectangle);
                }
                true
            }
            _ => false,
        }
    }
    pub unsafe fn page_body(c: *mut conv_ctx, out: *mut pbuf, page: i32) {
        let state = &(*(*c).document).threads;
        if !state.order.is_empty() {
            if let Some(array) = state.page_arrays.get(&page) {
                append(out, &format!("/B {array} 0 R"));
            }
        }
    }
    pub unsafe fn finish(c: *mut conv_ctx) {
        while let Some((key, pages)) = (*(*c).document).threads.active.pop() {
            (*(*c).document).threads.active_levels.pop();
            (*(*c).document).threads.active_rules.pop();
            for (page, r) in pages {
                Self::bead(c, &key, page, r);
            }
        }
        let state = &mut (*(*c).document).threads;
        if state.order.is_empty() {
            return;
        }
        for (page, array) in &state.page_arrays {
            pdfw_begin((*c).pw, *array);
            let out = pdfw_out((*c).pw);
            append(out, "[");
            if let Some(beads) = state.pages.get(page) {
                for object in beads {
                    append(out, &format!("{object} 0 R "));
                }
            }
            append(out, "]");
            pdfw_end((*c).pw);
        }
        let mut threads = String::from("[");
        for key in &state.order {
            let article = &state.entries[key];
            if article.beads.is_empty() {
                continue;
            }
            threads.push_str(&format!("{} 0 R ", article.object));
            let n = article.beads.len();
            for (index, bead) in article.beads.iter().enumerate() {
                let mut dict = BTreeMap::new();
                dict.insert("Type".into(), "/Bead".into());
                dict.insert("T".into(), format!("{} 0 R", article.object));
                dict.insert("P".into(), format!("{} 0 R", bead.page));
                dict.insert(
                    "N".into(),
                    format!("{} 0 R", article.beads[(index + 1) % n].object),
                );
                dict.insert(
                    "V".into(),
                    format!("{} 0 R", article.beads[(index + n - 1) % n].object),
                );
                dict.insert(
                    "R".into(),
                    format!(
                        "[{} {} {} {}]",
                        bead.rect[0], bead.rect[1], bead.rect[2], bead.rect[3]
                    ),
                );
                write_object(c, bead.object, &dict);
            }
            let mut dict = article.dict.clone();
            dict.insert("Type".into(), "/Thread".into());
            dict.insert("F".into(), format!("{} 0 R", article.beads[0].object));
            write_object(c, article.object, &dict);
        }
        threads.push(']');
        (*(*c).document).catalog.insert("Threads".into(), threads);
    }
}
