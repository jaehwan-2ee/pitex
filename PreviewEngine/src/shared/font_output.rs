// Original Pitex font-resource and text-extraction services (ISO 32000).
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default)]
pub struct State {
    pub bitmaps: bitmap_fonts::State,
    pub table: *mut font_table,
    attributes: BTreeMap<i32, String>,
    included: BTreeMap<i32, BTreeSet<u32>>,
    objects: BTreeMap<i32, i32>,
    no_unicode: BTreeSet<i32>,
    pub letterspace: BTreeMap<i32, f64>,
    interword: bool,
    space_object: i32,
    last_glyph: Option<(f64, f64, f64)>,
    fallback: String,
    namespace: String,
}
unsafe fn append(out: *mut pbuf, text: &str) {
    pbuf_append(out, text.as_ptr().cast(), text.len());
}
pub unsafe fn resource(c: *mut conv_ctx, font: i32) -> String {
    let source = (*(*c).pfonts.offset(font as isize)).font_id;
    let prefix = if source < 0 {
        format!("V{}", -source)
    } else {
        format!("F{}", source + 1)
    };
    if (*(*c).document).backend.integer("pdfuniqueresname") > 0 {
        format!("{prefix}{}", namespace(c))
    } else {
        prefix
    }
}
pub unsafe fn namespace(c: *mut conv_ctx) -> String {
    let state = &mut *(*c).font_output;
    if state.namespace.is_empty() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        state.namespace = format!("P{:X}{:X}", std::process::id(), stamp);
    }
    state.namespace.clone()
}
pub unsafe fn allow_unicode(c: *mut conv_ctx, p: *mut pdf_font) -> bool {
    (*(*c).document).backend.integer("pdfgentounicode") > 0
        && !(*(*c).font_output).no_unicode.contains(&(*p).font_id)
}
pub unsafe fn object(c: *mut conv_ctx, id: i32) -> i32 {
    (*(*c).font_output)
        .objects
        .get(&id)
        .copied()
        .unwrap_or_else(|| pdfw_alloc((*c).pw))
}
pub unsafe fn attributes(c: *mut conv_ctx, p: *mut pdf_font, out: *mut pbuf) {
    let Some(text) = (*(*c).font_output).attributes.get(&(*p).font_id).cloned() else {
        return;
    };
    let text = pdf_document::pdftex_references(&text);
    let rendered = subst_names(c, text.as_ptr().cast(), text.len(), 0., 0.);
    let text = std::ffi::CStr::from_ptr(rendered).to_string_lossy();
    let text = text.trim().strip_prefix("<<").unwrap_or(&text).trim();
    let text = text.strip_suffix(">>").unwrap_or(text).trim();
    append(out, text);
    free(rendered.cast());
}
pub unsafe fn charset(c: *mut conv_ctx, font: *mut pdf_font, out: *mut pbuf) {
    if (*(*c).document).backend.integer("pdfomitcharset") != 0 {
        return;
    }
    let mut names = BTreeMap::new();
    if !(*font).enc.is_null() {
        for code in 0..256 {
            let name = (*(*font).enc).glyph[code];
            if !name.is_null() {
                names.insert(
                    code,
                    std::ffi::CStr::from_ptr(name)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    } else {
        let text = String::from_utf8_lossy(std::slice::from_raw_parts(
            (*(*font).t1).data,
            (*(*font).t1).len1,
        ));
        let words = text.split_ascii_whitespace().collect::<Vec<_>>();
        for row in words.windows(4) {
            if row[0] == "dup" && row[3] == "put" {
                if let (Ok(code), Some(name)) = (row[1].parse::<usize>(), row[2].strip_prefix('/'))
                {
                    if code < 256 {
                        names.insert(code, name.to_string());
                    }
                }
            }
        }
    }
    let mut set = String::new();
    for (code, name) in names {
        if *(*font).used.offset(code as isize) != 0 {
            set.push('/');
            set.push_str(&name);
        }
    }
    let set = set
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
    append(out, &format!("/CharSet ({set})"));
}
pub unsafe fn mark_included(c: *mut conv_ctx, s: *mut font_slot, p: *mut pdf_font) {
    let Some(included) = (*(*c).font_output).included.get(&(*s).k) else {
        return;
    };
    for character in included {
        if (*p).native {
            for glyph in 0..(*p).nused {
                if *(*(*p).nf).to_unicode.offset(glyph as isize) == *character {
                    *(*p).used.offset(glyph as isize) = 1;
                }
            }
        } else if *character < 256 {
            *(*p).used.offset(*character as isize) = 1;
        }
    }
}
pub unsafe fn special(c: *mut conv_ctx, text: &str, x: f64, y: f64) -> bool {
    let Some(tail) = text.strip_prefix("pitex:") else {
        return false;
    };
    let (command, tail) = tail.split_once(' ').unwrap_or((tail, ""));
    match command {
        "font-letterspace" => {
            let mut fields = tail.split_whitespace();
            if let (Some(id), Some(delta), Some(size)) = (
                fields.next().and_then(|s| s.parse::<i32>().ok()),
                fields.next().and_then(|s| s.parse::<f64>().ok()),
                fields.next().and_then(|s| s.parse::<f64>().ok()),
            ) {
                (*(*c).font_output).letterspace.insert(id, delta);
                let slot = table_find((*(*c).font_output).table, id);
                if !slot.is_null() {
                    (*slot).letterspace = delta;
                }
            }
        }
        "font-attribute" => {
            if let Some((id, body)) = tail.split_once(' ') {
                if let Ok(id) = id.parse() {
                    (*(*c).font_output).attributes.insert(id, body.to_string());
                }
            }
        }
        "font-include" => {
            let mut fields = tail.split_whitespace();
            if let Some(id) = fields.next().and_then(|s| s.parse::<i32>().ok()) {
                let characters = (*(*c).font_output).included.entry(id).or_default();
                characters.extend(fields.filter_map(|s| s.parse::<u32>().ok()));
            }
        }
        "font-resource" => {
            if let Some((id, name)) = tail.split_once(' ') {
                if let Ok(id) = id.parse::<i32>() {
                    let name = name.trim();
                    let named = named_get(c, name.as_ptr().cast(), name.len(), true);
                    let object = (*named).obj;
                    (*named).written = true;
                    (*(*c).font_output).objects.insert(id, object);
                }
            }
        }
        "font-no-unicode" => {
            if let Ok(id) = tail.trim().parse() {
                (*(*c).font_output).no_unicode.insert(id);
            }
        }
        "interword-space" => {
            (*(*c).font_output).interword = tail.trim() == "on";
        }
        "fake-space" => {
            fake_space(c, x, y);
        }
        "space-font" => {
            (*(*c).font_output).fallback = tail.trim().to_owned();
        }
        _ => return false,
    }
    // Font-include/font-resource may be the only use of a font; force its PDF
    // dictionary and widths/CMap entries without adding anything to the page.
    if matches!(command, "font-include" | "font-resource") {
        if let Some(id) = tail
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<i32>().ok())
        {
            let slot = table_find((*(*c).font_output).table, id);
            if !slot.is_null() && matches!((*slot).kind, FS_TYPE1 | FS_NATIVE | FS_BITMAP) {
                let index = pdf_font_for(c, slot);
                let font = (*c).pfonts.offset(index as isize);
                mark_included(c, slot, font);
            }
        }
    }
    true
}
pub unsafe fn before_glyph(c: *mut conv_ctx, x: f64, y: f64, size: f64, width: f64) {
    let previous = (*(*c).font_output).last_glyph;
    if (*(*c).font_output).interword {
        if let Some((end, baseline, _)) = previous {
            if (y - baseline).abs() < size * 0.1 && x - end > size * 0.1 && x - end < size * 3. {
                fake_space(c, end, y);
            }
        }
    }
    (*(*c).font_output).last_glyph = Some((x + width, y, size));
}
pub unsafe fn new_page(c: *mut conv_ctx) {
    (*(*c).font_output).last_glyph = None;
}
unsafe fn ensure_space_font(c: *mut conv_ctx) -> i32 {
    if (*(*c).font_output).space_object > 0 {
        return (*(*c).font_output).space_object;
    }
    let object = pdfw_alloc((*c).pw);
    let glyph = pdfw_alloc((*c).pw);
    let unicode = pdfw_alloc((*c).pw);
    pdfw_stream(
        (*c).pw,
        glyph,
        std::ptr::null(),
        b"333 0 d0\n".as_ptr().cast(),
        9,
        false,
    );
    let cmap=b"/CIDInit/ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo<</Registry(Pitex)/Ordering(UCS)/Supplement 0>>def /CMapName/PitexSpace def /CMapType 2 def 1 begincodespacerange <00><FF> endcodespacerange 1 beginbfchar <20><0020> endbfchar endcmap CMapName currentdict/CMap defineresource pop end end\n";
    pdfw_stream(
        (*c).pw,
        unicode,
        std::ptr::null(),
        cmap.as_ptr().cast(),
        cmap.len(),
        true,
    );
    pdfw_begin((*c).pw, object);
    let dictionary=format!("<</Type/Font/Subtype/Type3/Name/PitexSpace/FontBBox[0 0 0 0]/FontMatrix[.001 0 0 .001 0 0]/CharProcs<</space {glyph} 0 R>>/Encoding<</Type/Encoding/Differences[32/space]>>/FirstChar 32/LastChar 32/Widths[333]/Resources<<>>/ToUnicode {unicode} 0 R>>");
    append(pdfw_out((*c).pw), &dictionary);
    pdfw_end((*c).pw);
    (*(*c).font_output).space_object = object;
    object
}
pub unsafe fn fake_space(c: *mut conv_ctx, x: f64, y: f64) {
    if !(*c).render {
        return;
    }
    let fallback = (*(*c).font_output).fallback.clone();
    let mut resource = "PitexSpace".to_string();
    if !fallback.is_empty() {
        let mut slot = table_find((*(*c).font_output).table, -1_000_000);
        if slot.is_null() {
            slot = new_slot(-1_000_000);
            (*slot).is_virtual = true;
            let name = std::ffi::CString::new(fallback).unwrap();
            (*slot).name = strdup(name.as_ptr());
            (*slot).size = 65536.;
            setup_tfm_slot(c, slot);
            table_add((*(*c).font_output).table, slot);
        }
        if matches!((*slot).kind, FS_TYPE1 | FS_BITMAP) && (*(*slot).tfm).exists[32] {
            let index = pdf_font_for(c, slot);
            let font = (*c).pfonts.offset(index as isize);
            *(*font).used.offset(32) = 1;
            resource = self::resource(c, index);
        }
    }
    if resource == "PitexSpace" {
        ensure_space_font(c);
    }
    text_end(c);
    let out = &raw mut (*cur(c)).content;
    append(
        out,
        &format!("BT /{resource} 1 Tf 3 Tr 1 0 0 1 {x:.6} {y:.6} Tm ( ) Tj 0 Tr ET\n"),
    );
}
pub unsafe fn write_extra_resources(c: *mut conv_ctx, out: *mut pbuf) {
    let object = (*(*c).font_output).space_object;
    if object > 0 {
        append(out, &format!("/PitexSpace {object} 0 R"));
    }
}
