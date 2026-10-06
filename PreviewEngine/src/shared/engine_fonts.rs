// Original Pitex typography services. No pdfTeX implementation code is copied.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use crate::xetex_engine_xetex_ini as state;
use crate::xetex_engine_xetex_xetex0 as tex;
use std::collections::BTreeMap;
extern "C" {
    fn pitex_disable_native_ligatures(engine: *mut std::ffi::c_void);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Expansion {
    pub stretch: i32,
    pub shrink: i32,
    pub step: i32,
}
#[derive(Clone, Debug, Default)]
struct FontSettings {
    codes: BTreeMap<(i32, i32), i32>,
    expansion: Option<Expansion>,
    no_ligatures: bool,
    letterspace: i32,
    object: Option<i32>,
}
static mut FONTS: Vec<FontSettings> = Vec::new();
static mut DEFINITIONS: Vec<i32> = Vec::new();
pub unsafe fn pending_definitions() -> Vec<i32> {
    std::mem::take(&mut DEFINITIONS)
}
unsafe fn request_definition(font: i32) {
    if font > 0 && !DEFINITIONS.contains(&font) {
        DEFINITIONS.push(font);
    }
}
pub static mut PARAGRAPH_PACKING: bool = false;
unsafe fn settings(font: i32) -> &'static mut FontSettings {
    while FONTS.len() <= font.max(0) as usize {
        FONTS.push(FontSettings::default());
    }
    &mut FONTS[font.max(0) as usize]
}
unsafe fn tag_location(font:i32,character:i32)->Option<isize> {
    if font<0||font>state::font_ptr as i32||character<0||character>255{return None;}
    let area=*state::font_area.offset(font as isize) as u32;
    if area==tex::OTGR_FONT_FLAG||area==tex::AAT_FONT_FLAG{return None;}
    if character<*state::font_bc.offset(font as isize) as i32||character>*state::font_ec.offset(font as isize) as i32{return None;}
    Some((*state::char_base.offset(font as isize)+character) as isize)
}
pub unsafe fn code(kind: i32, font: i32, character: i32) -> i32 {
    if kind==106 {
        let Some(location)=tag_location(font,character) else{return 0;};
        return match (*state::font_info.offset(location)).b16.s1 & 3 {1=>1,2=>2,3=>4,_=>0};
    }
    settings(font)
        .codes
        .get(&(kind, character))
        .copied()
        .unwrap_or(if kind == 100 { 1000 } else { 0 })
}
pub unsafe fn set_code(kind: i32, font: i32, character: i32, value: i32) {
    if kind==106 {
        if value>=0{return;}
        let mask=if value<= -7 {7}else{-value};
        if code(kind,font,character)&mask !=0 {
            if let Some(location)=tag_location(font,character) {(*state::font_info.offset(location)).b16.s1 &= !3;}
        }
        return;
    }
    settings(font).codes.insert(
        (kind, character),
        if kind == 100 {
            value.clamp(0, 1000)
        } else {
            value.clamp(-1000, 1000)
        },
    );
}
pub unsafe fn set_expansion(font: i32, stretch: i32, shrink: i32, step: i32) {
    let step = step.clamp(1, 1000);
    settings(font).expansion = Some(Expansion {
        stretch: stretch.clamp(0, 1000) / step * step,
        shrink: shrink.clamp(0, 1000) / step * step,
        step,
    });
}
pub unsafe fn expansion(font: i32, fallback: Expansion) -> Expansion {
    settings(font).expansion.unwrap_or(fallback)
}
pub unsafe fn disable_ligatures(font: i32) {
    settings(font).no_ligatures = true;
    let area = *state::font_area.offset(font as isize) as u32;
    if area == tex::OTGR_FONT_FLAG {
        pitex_disable_native_ligatures(*state::font_layout_engine.offset(font as isize));
    }
    #[cfg(target_os = "macos")]
    if area == tex::AAT_FONT_FLAG {
        disable_aat_ligatures(font);
    }
}
#[cfg(target_os = "macos")]
unsafe fn disable_aat_ligatures(font: i32) {
    extern "C" {
        fn CFDictionaryCreateMutableCopy(
            allocator: *const std::ffi::c_void,
            capacity: isize,
            dictionary: *const std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
        fn CFDictionarySetValue(
            dictionary: *mut std::ffi::c_void,
            key: *const std::ffi::c_void,
            value: *const std::ffi::c_void,
        );
        fn CFNumberCreate(
            allocator: *const std::ffi::c_void,
            kind: isize,
            value: *const std::ffi::c_void,
        ) -> *const std::ffi::c_void;
        fn CFRelease(value: *const std::ffi::c_void);
        static kCTLigatureAttributeName: *const std::ffi::c_void;
        static kCTKernAttributeName: *const std::ffi::c_void;
    }
    let old = *state::font_layout_engine.offset(font as isize);
    let attributes = CFDictionaryCreateMutableCopy(std::ptr::null(), 0, old);
    let zero = 0i32;
    let value = CFNumberCreate(std::ptr::null(), 9, (&zero as *const i32).cast());
    CFDictionarySetValue(attributes, kCTLigatureAttributeName, value);
    let kern = 0f64;
    let kern_value = CFNumberCreate(std::ptr::null(), 13, (&kern as *const f64).cast());
    CFDictionarySetValue(attributes, kCTKernAttributeName, kern_value);
    CFRelease(kern_value);
    CFRelease(value);
    CFRelease(old);
    *state::font_layout_engine.offset(font as isize) = attributes;
}
pub unsafe fn ligatures_disabled(font: i32) -> bool {
    settings(font).no_ligatures
}
pub unsafe fn spacing_configured(font: i32) -> bool {
    settings(font).codes.keys().any(|(k, _)| *k != 100)
}
pub unsafe fn clone_settings(source: i32, target: i32, letterspace: i32) {
    let mut cloned = settings(source).clone();
    cloned.letterspace = letterspace;
    cloned.object = None;
    cloned.no_ligatures = letterspace != 0 || cloned.no_ligatures;
    *settings(target) = cloned;
}
pub unsafe fn letterspace(font: i32) -> i32 {
    settings(font).letterspace
}
unsafe fn is_char_node(node: i32) -> bool {
    node >= state::hi_mem_min
}
pub unsafe fn parameter(name: &[u8]) -> i32 {
    tex::pitex_output_parameter(name.as_ptr().cast())
}
pub unsafe fn apply_spacing(head: i32) -> i32 {
    spacing_list(
        head,
        parameter(b"pdfprependkern\0") > 0,
        parameter(b"pdfappendkern\0") > 0,
        parameter(b"pdfadjustinterwordglue\0") > 0,
    )
}
pub unsafe fn em(font: i32) -> i32 {
    (*state::font_info.offset((*state::param_base.offset(font as isize) + tex::QUAD_CODE) as isize))
        .b32
        .s1
}
pub fn thousandths(em: i32, code: i32) -> i32 {
    ((em as i64 * code as i64) / 1000).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

// A private kern subtype marks typography that has already been applied. It has
// ordinary automatic-kern semantics, so copies/unboxed lists keep their spacing.
// It occupies no new TeX node layout or external ABI, and is ignored by shipout.
const APPLIED_KERN: u16 = 127;
unsafe fn glyph(node: i32) -> Option<(i32, i32)> {
    let node = if is_char_node(node) {
        node
    } else if (*state::mem.offset(node as isize)).b16.s1 as i32 == tex::LIGATURE_NODE {
        node + 1
    } else {
        return None;
    };
    Some((
        (*state::mem.offset(node as isize)).b16.s1 as i32,
        (*state::mem.offset(node as isize)).b16.s0 as i32,
    ))
}
unsafe fn applied(node: i32) -> bool {
    node != tex::TEX_NULL
        && !is_char_node(node)
        && (*state::mem.offset(node as isize)).b16.s1 as i32 == tex::KERN_NODE
        && (*state::mem.offset(node as isize)).b16.s0 == APPLIED_KERN
}
pub unsafe fn spacing_list(mut head: i32, prepend: bool, append: bool, interword: bool) -> i32 {
    if PARAGRAPH_PACKING || (!prepend && !append && !interword) {
        return head;
    }
    let mut previous = tex::TEX_NULL;
    let mut node = head;
    while node != tex::TEX_NULL {
        let next = (*state::mem.offset(node as isize)).b32.s1;
        if let Some((font, character)) = glyph(node) {
            if spacing_configured(font) && !applied(next) {
                let quad = em(font);
                let before = if prepend {
                    thousandths(quad, code(102, font, character))
                } else {
                    0
                };
                let after = if append {
                    thousandths(quad, code(101, font, character))
                } else {
                    0
                };
                if before != 0 {
                    let kern = tex::new_kern(before);
                    (*state::mem.offset(kern as isize)).b16.s0 = APPLIED_KERN;
                    (*state::mem.offset(kern as isize)).b32.s1 = node;
                    if previous == tex::TEX_NULL {
                        head = kern;
                    } else {
                        (*state::mem.offset(previous as isize)).b32.s1 = kern;
                    }
                }
                if interword
                    && next != tex::TEX_NULL
                    && !is_char_node(next)
                    && (*state::mem.offset(next as isize)).b16.s1 as i32 == tex::GLUE_NODE
                {
                    let old = (*state::mem.offset((next + 1) as isize)).b32.s0;
                    let copy = tex::new_spec(old);
                    for (component, kind) in [(1, 103), (2, 104), (3, 105)] {
                        (*state::mem.offset((copy + component) as isize)).b32.s1 = (*state::mem
                            .offset((copy + component) as isize))
                        .b32
                        .s1
                        .saturating_add(thousandths(quad, code(kind, font, character)));
                    }
                    tex::delete_glue_ref(old);
                    (*state::mem.offset((next + 1) as isize)).b32.s0 = copy;
                }
                let marker = tex::new_kern(after);
                (*state::mem.offset(marker as isize)).b16.s0 = APPLIED_KERN;
                (*state::mem.offset(marker as isize)).b32.s1 = next;
                (*state::mem.offset(node as isize)).b32.s1 = marker;
                previous = marker;
                node = next;
                continue;
            }
        }
        previous = node;
        node = next;
    }
    head
}

pub unsafe fn new_letterspaced_font(source: i32, amount: i32) -> i32 {
    let name = *state::font_name.offset(source as isize);
    let area = *state::font_area.offset(source as isize);
    let native = area as u32 == tex::AAT_FONT_FLAG || area as u32 == tex::OTGR_FONT_FLAG;
    // Bypass normal font-instance caching: each instance owns its native layout
    // engine and TECkit mapping, so shutdown never releases a shared pointer twice.
    let target = tex::read_font_info(
        0,
        name,
        if native {
            tex::EMPTY_STRING as i32
        } else {
            area
        },
        *state::font_size.offset(source as isize),
    ) as i32;
    if target == 0 {
        return 0;
    }
    let delta = thousandths(em(source), amount);
    let total = letterspace(source).saturating_add(delta);
    clone_settings(source, target, total);
    request_definition(target);
    let count = *state::font_params.offset(source as isize);
    let end = *state::param_base.offset(target as isize) + count + 1;
    if end > state::font_mem_size {
        let capacity = end.saturating_add(1024);
        let allocation = libc::realloc(
            state::font_info.cast(),
            capacity as usize * std::mem::size_of::<state::memory_word>(),
        )
        .cast::<state::memory_word>();
        if allocation.is_null() {
            libc::abort();
        }
        state::font_info = allocation;
        state::font_mem_size = capacity;
    }
    for parameter in 1..=count {
        let value = (*state::font_info
            .offset((*state::param_base.offset(source as isize) + parameter) as isize))
        .b32
        .s1;
        (*state::font_info
            .offset((*state::param_base.offset(target as isize) + parameter) as isize))
        .b32
        .s1 = value;
    }
    *state::font_params.offset(target as isize) = count;
    state::fmem_ptr = state::fmem_ptr.max(end);
    *state::hyphen_char.offset(target as isize) = *state::hyphen_char.offset(source as isize);
    *state::skew_char.offset(target as isize) = *state::skew_char.offset(source as isize);
    if native {
        *state::font_letter_space.offset(target as isize) =
            (*state::font_letter_space.offset(source as isize)).saturating_add(delta);
        if ligatures_disabled(target) {
            disable_ligatures(target);
        }
    } else {
        let mut maximum = 0;
        for character in
            *state::font_bc.offset(target as isize)..=*state::font_ec.offset(target as isize)
        {
            let info = (*state::font_info
                .offset((*state::char_base.offset(target as isize) + character as i32) as isize))
            .b16;
            maximum = maximum.max(info.s3 as i32);
        }
        for index in 1..=maximum {
            let metric = state::font_info
                .offset((*state::width_base.offset(target as isize) + index) as isize);
            (*metric).b32.s1 = (*metric).b32.s1.saturating_add(total);
        }
        tex::pitex_append_global_special(&format!(
            "pitex:font-letterspace {} {} {}",
            target - 1,
            total,
            *state::font_size.offset(target as isize)
        ));
    }
    target
}
pub unsafe fn extension(code: i32) {
    match code {
        130 => {
            tex::scan_font_ident();
            let font = state::cur_val;
            tex::scan_int();
            let stretch = state::cur_val;
            tex::scan_int();
            let shrink = state::cur_val;
            tex::scan_int();
            let step = state::cur_val;
            tex::scan_keyword(b"autoexpand\0".as_ptr().cast());
            set_expansion(font, stretch, shrink, step);
        }
        131 | 132 => {
            tex::get_r_token();
            let control = state::cur_cs;
            tex::scan_optional_equals();
            tex::scan_font_ident();
            let source = state::cur_val;
            let target = if code == 132 {
                tex::scan_int();
                new_letterspaced_font(source, state::cur_val)
            } else {
                new_letterspaced_font(source, 0)
            };
            tex::eq_define(control, tex::SET_FONT as u16, target);
        }
        133 => {
            tex::scan_font_ident();
            disable_ligatures(state::cur_val);
        }
        134 => {
            tex::scan_font_ident();
            let font = state::cur_val;
            let value = tex::pitex_scan_pdf_text();
            let codes = value
                .chars()
                .map(|c| (c as u32).to_string())
                .collect::<Vec<_>>()
                .join(" ");
            request_definition(font);
            tex::pitex_append_global_special(&format!("pitex:font-include {} {}", font - 1, codes));
        }
        135 => {
            tex::scan_font_ident();
            let font = state::cur_val;
            let value = tex::pitex_scan_pdf_text();
            request_definition(font);
            tex::pitex_append_global_special(&format!(
                "pitex:font-attribute {} << {} >>",
                font - 1,
                value
            ));
        }
        136 => tex::pitex_append_special("pitex:fake-space"),
        137 => tex::pitex_append_special("pitex:interword-space on"),
        138 => tex::pitex_append_special("pitex:interword-space off"),
        139 => {
            let value = tex::pitex_scan_pdf_text();
            tex::pitex_append_global_special(&format!("pitex:space-font {}", value));
        }
        140 => {
            if (state::cur_list.mode as i32).abs() == tex::VMODE {
                tex::new_graf(false);
            }
        }
        141 => {
            tex::scan_font_ident();
            let font = state::cur_val;
            tex::pitex_append_global_special(&format!("pitex:font-no-unicode {}", font - 1));
        }
        _ => unreachable!(),
    }
}
pub unsafe fn font_size_query() -> String {
    tex::scan_font_ident();
    dimension_text(*state::font_size.offset(state::cur_val as isize))
}
pub unsafe fn query(code: i32) {
    let result = if code == 133 {
        tex::scan_int();
        dimension_text(insertion_height(state::cur_val))
    } else if code == 130 {
        font_size_query()
    } else {
        tex::scan_font_ident();
        let font = state::cur_val;
        request_definition(font);
        if code == 131 {
            font.to_string()
        } else {
            let object = if let Some(object) = settings(font).object {
                object
            } else {
                let object = tex::pitex_reserve_font_object();
                settings(font).object = Some(object);
                object
            };
            tex::pitex_append_global_special(&format!(
                "pitex:font-resource {} @pitexobj{}",
                font - 1,
                object
            ));
            object.to_string()
        }
    };
    tex::pitex_insert_text(&result);
}
pub unsafe fn insertion_height(class: i32) -> i32 {
    let mut record = (*state::mem.offset(tex::PAGE_INS_HEAD as isize)).b32.s1;
    while record != tex::PAGE_INS_HEAD {
        if (*state::mem.offset(record as isize)).b16.s0 as i32 == class {
            return (*state::mem.offset((record + 3) as isize)).b32.s1;
        }
        record = (*state::mem.offset(record as isize)).b32.s1;
    }
    0
}
pub unsafe fn expansion_trace(node: i32) -> String {
    if !is_char_node(node) {
        return String::new();
    }
    let next = (*state::mem.offset(node as isize)).b32.s1;
    if next == tex::TEX_NULL
        || is_char_node(next)
        || (*state::mem.offset(next as isize)).b16.s1 as i32 != tex::KERN_NODE
        || (*state::mem.offset(next as isize)).b16.s0 != 126
    {
        return String::new();
    }
    let font = (*state::mem.offset(node as isize)).b16.s1 as i32;
    let character = (*state::mem.offset(node as isize)).b16.s0 as i32;
    let index = (*state::font_info
        .offset((*state::char_base.offset(font as isize) + character) as isize))
    .b16
    .s3 as i32;
    let width = (*state::font_info
        .offset((*state::width_base.offset(font as isize) + index) as isize))
    .b32
    .s1;
    if width == 0 {
        return String::new();
    }
    let ratio = ((*state::mem.offset((next + 1) as isize)).b32.s1 as f64 * 1000. / width as f64)
        .round() as i32;
    if parameter(b"pdftracingfonts\0") == 1 {
        let name = *state::font_name.offset(font as isize) - 65536;
        let begin = *state::str_start.offset(name as isize);
        let end = *state::str_start.offset((name + 1) as isize);
        let name = String::from_utf16_lossy(std::slice::from_raw_parts(
            state::str_pool.offset(begin as isize),
            (end - begin) as usize,
        ));
        format!(
            " ({name}{ratio:+}@{})",
            dimension_text(*state::font_size.offset(font as isize))
        )
    } else {
        format!(" ({ratio:+})")
    }
}

// Exact TeX fixed-point decimal formatting used by expandable font-size queries.
pub fn dimension_text(value: i32) -> String {
    let negative = value < 0;
    let value = (value as i64).abs();
    let mut result = format!("{}{}.", if negative { "-" } else { "" }, value / 65536);
    let mut s = 10 * (value % 65536) + 5;
    let mut delta = 10;
    loop {
        if delta > 65536 {
            s += 32768 - 50000;
        }
        result.push(char::from(b'0' + (s / 65536) as u8));
        s = 10 * (s % 65536);
        delta *= 10;
        if s <= delta {
            break;
        }
    }
    result.push_str("pt");
    result
}
