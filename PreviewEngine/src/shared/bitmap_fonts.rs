// Original Pitex PK loading and ISO 32000 Type 3 bitmap font output.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
#[path = "bitmap_pk.rs"]
mod pk;

#[derive(Default)]
pub struct State {
    slots: BTreeMap<usize, Rc<pk::Font>>,
    outputs: BTreeMap<i32, Rc<pk::Font>>,
}
unsafe fn load(c: *mut conv_ctx, name: &str) -> Option<pk::Font> {
    let name = std::ffi::CString::new(name).ok()?;
    let file = ((*(*c).w).res.load?)((*(*c).w).res.env, name.as_ptr(), 7);
    if file.is_null() {
        return None;
    }
    let parsed = pk::parse(std::slice::from_raw_parts((*file).data, (*file).len));
    tbuf_drop(file);
    if let Err(message) = parsed.as_ref() {
        if !(*c).warnings.is_null() {
            let warning = format!("Invalid PK font {}: {message}\n", name.to_string_lossy());
            pbuf_append((*c).warnings, warning.as_ptr().cast(), warning.len());
        }
    }
    parsed.ok()
}
fn mode_matches(path: &std::path::Path, mode: &str) -> bool {
    mode.is_empty()
        || path
            .components()
            .any(|component| component.as_os_str() == mode)
}
fn matching_resolution(font: &pk::Font, dpi: i32) -> bool {
    let (horizontal, _) = font.dpi();
    (horizontal - dpi as f64).abs() <= 1.
}
pub unsafe fn setup(c: *mut conv_ctx, slot: *mut font_slot) -> bool {
    if (*slot).tfm.is_null() || !(*(*slot).tfm).ok || (*(*slot).tfm).design <= 0 {
        return false;
    }
    let name = std::ffi::CStr::from_ptr((*slot).name)
        .to_string_lossy()
        .into_owned();
    let options = &(*(*c).document).backend;
    let mode = options
        .tokens
        .get("pdfpkmode")
        .map(String::as_str)
        .unwrap_or("")
        .trim()
        .to_owned();
    let base = options.integer("pdfpkresolution");
    let base = if base > 0 { base } else { 72 };
    let design_points = (*(*slot).tfm).design as f64 / 1048576.;
    let size_ratio = (*slot).size * (*c).conv / (design_points * 72. / 72.27);
    let dpi = (base as f64 * size_ratio).round().clamp(1., 100000.) as i32;
    let mut candidates = Vec::new();
    if !mode.is_empty() {
        candidates.extend([
            format!("{mode}/{name}.{dpi}pk"),
            format!("{mode}/dpi{dpi}/{name}.pk"),
        ]);
    }
    candidates.extend([format!("{name}.{dpi}pk"), format!("dpi{dpi}/{name}.pk")]);
    // The ordinary provider indexes prebuilt TeX Live files. Its basename entry
    // gives us the PK tree without invoking a font generator or another driver.
    let basename = std::ffi::CString::new(format!("{name}.pk")).unwrap();
    let installed = crate::xetex_common_texlive_provider::texlive_file_path(
        basename.as_ptr(),
        std::ptr::null_mut(),
    );
    if !installed.is_null() {
        let path = std::path::PathBuf::from(
            std::ffi::CStr::from_ptr(installed)
                .to_string_lossy()
                .as_ref(),
        );
        if mode_matches(&path, &mode) {
            candidates.push(path.to_string_lossy().into_owned());
        }
        if let Some(parent) = path.parent().and_then(|p| p.parent()) {
            let sibling = parent.join(format!("dpi{dpi}")).join(format!("{name}.pk"));
            if mode_matches(&sibling, &mode) {
                candidates.push(sibling.to_string_lossy().into_owned());
            }
        }
        // Mode trees can contain several typeface/vendor levels. Enumerate only
        // the requested installed mode, with an explicit directory budget.
        if !mode.is_empty() {
            if let Some(root) = path
                .ancestors()
                .find(|p| p.file_name().is_some_and(|s| s == "pk"))
            {
                let mut stack = vec![root.join(&mode)];
                let mut budget = 4096;
                while let Some(directory) = stack.pop() {
                    if budget == 0 {
                        break;
                    }
                    budget -= 1;
                    let Ok(entries) = std::fs::read_dir(&directory) else {
                        continue;
                    };
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                            stack.push(path);
                        } else if path.file_name().is_some_and(|s| {
                            s == basename.to_string_lossy().as_ref()
                                || s == format!("{name}.{dpi}pk").as_str()
                        }) {
                            candidates.push(path.to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
    }
    if mode.is_empty() || std::path::Path::new(&format!("{name}.pk")).is_file() {
        candidates.push(format!("{name}.pk"));
    }
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.clone()) {
            continue;
        }
        // Local exact filenames override the device mode, as in TeX's normal
        // file search. Installed basename hits must belong to that mode.
        let candidate_path = std::path::Path::new(&candidate);
        if !mode.is_empty() && !mode_matches(candidate_path, &mode) && !candidate_path.is_file() {
            let requested = std::ffi::CString::new(candidate.as_str()).unwrap();
            let resolved = crate::xetex_common_texlive_provider::texlive_file_path(
                requested.as_ptr(),
                std::ptr::null_mut(),
            );
            if resolved.is_null()
                || !mode_matches(
                    std::path::Path::new(
                        std::ffi::CStr::from_ptr(resolved)
                            .to_string_lossy()
                            .as_ref(),
                    ),
                    &mode,
                )
            {
                continue;
            }
        }
        let Some(font) = load(c, &candidate) else {
            continue;
        };
        if !matching_resolution(&font, dpi) {
            continue;
        }
        if font.checksum != 0
            && (*(*slot).tfm).checksum != 0
            && font.checksum != (*(*slot).tfm).checksum
        {
            if !(*c).warnings.is_null() {
                let warning = format!("[pk-checksum] PK checksum differs from TFM for {name}\n");
                pbuf_append((*c).warnings, warning.as_ptr().cast(), warning.len());
            }
        }
        (*(*c).font_output)
            .bitmaps
            .slots
            .insert(slot as usize, Rc::new(font));
        (*slot).kind = FS_BITMAP;
        return true;
    }
    false
}
pub unsafe fn attach(c: *mut conv_ctx, slot: *mut font_slot, index: i32) {
    if let Some(font) = (*(*c).font_output)
        .bitmaps
        .slots
        .get(&(slot as usize))
        .cloned()
    {
        (*(*c).font_output).bitmaps.outputs.insert(index, font);
    }
}
pub unsafe fn exists(c: *mut conv_ctx, slot: *mut font_slot, code: u32) -> bool {
    (*(*c).font_output)
        .bitmaps
        .slots
        .get(&(slot as usize))
        .is_some_and(|font| font.glyphs.contains_key(&code))
}
unsafe fn append(out: *mut pbuf, value: &str) {
    pbuf_append(out, value.as_ptr().cast(), value.len());
}
fn number(value: f64) -> String {
    backend_options::coordinate(value, 4)
}
unsafe fn glyph_name(font: *mut pdf_font, code: usize) -> String {
    if !(*font).enc.is_null() {
        let name = (*(*font).enc).glyph[code];
        if !name.is_null() {
            return std::ffi::CStr::from_ptr(name)
                .to_string_lossy()
                .into_owned();
        }
    }
    format!("a{code}")
}
unsafe fn unicode(c: *mut conv_ctx, font: *mut pdf_font, codes: &[usize]) -> Option<i32> {
    if !font_output::allow_unicode(c, font) {
        return None;
    }
    let mappings = codes
        .iter()
        .filter_map(|code| {
            backend_options::glyph_unicode(&glyph_name(font, *code), &(*(*c).document).backend)
                .map(|unicode| (*code, unicode))
        })
        .collect::<Vec<_>>();
    if mappings.is_empty() {
        return None;
    }
    let object = pdfw_alloc((*c).pw);
    let mut body=String::from("/CIDInit/ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo<</Registry(Pitex)/Ordering(UCS)/Supplement 0>>def /CMapName/PitexBitmapUnicode def /CMapType 2 def 1 begincodespacerange <00><FF> endcodespacerange\n");
    for chunk in mappings.chunks(100) {
        body += &format!("{} beginbfchar\n", chunk.len());
        for (code, value) in chunk {
            body += &format!("<{code:02X}><{value}>\n");
        }
        body += "endbfchar\n";
    }
    body += "endcmap CMapName currentdict/CMap defineresource pop end end\n";
    pdfw_stream(
        (*c).pw,
        object,
        std::ptr::null(),
        body.as_ptr().cast(),
        body.len(),
        true,
    );
    Some(object)
}
pub unsafe fn write(c: *mut conv_ctx, font: *mut pdf_font, index: i32) {
    let Some(pk) = (*(*c).font_output).bitmaps.outputs.get(&index).cloned() else {
        return;
    };
    let design = pk.design as f64 / 1048576.;
    let ux = 1000. * 65536. / (pk.horizontal_ppp as f64 * design);
    let uy = 1000. * 65536. / (pk.vertical_ppp as f64 * design);
    let mut codes = (0..256usize)
        .filter(|code| *(*font).used.add(*code) != 0)
        .collect::<Vec<_>>();
    if codes.is_empty() {
        codes.push(0);
    }
    let mut bounds = [0f64; 4];
    let mut procedures = Vec::new();
    let mut images = Vec::new();
    for code in &codes {
        let width = if !(*font).tfm.is_null() && (*(*font).tfm).exists[*code] {
            (*(*font).tfm).width[*code] as f64 / 1048.576
        } else {
            0.
        };
        let procedure = pdfw_alloc((*c).pw);
        let glyph = pk.glyphs.get(&(*code as u32));
        let mut body = format!("{} 0 d0\n", number(width));
        if let Some(glyph) = glyph.filter(|g| g.width > 0 && g.height > 0) {
            let x = -(glyph.x_offset as f64) * ux;
            let y = (glyph.y_offset as f64 + 1. - glyph.height as f64) * uy;
            let w = glyph.width as f64 * ux;
            let h = glyph.height as f64 * uy;
            bounds[0] = bounds[0].min(x);
            bounds[1] = bounds[1].min(y);
            bounds[2] = bounds[2].max(x + w);
            bounds[3] = bounds[3].max(y + h);
            body = format!(
                "{} 0 {} {} {} {} d1\nq {} 0 0 {} {} {} cm /M{code} Do Q\n",
                number(width),
                number(x),
                number(y),
                number(x + w),
                number(y + h),
                number(w),
                number(h),
                number(x),
                number(y)
            );
            let image = pdfw_alloc((*c).pw);
            let dict=std::ffi::CString::new(format!("/Type/XObject/Subtype/Image/Width {}/Height {}/ImageMask true/BitsPerComponent 1/Decode[1 0]/Interpolate false",glyph.width,glyph.height)).unwrap();
            pdfw_stream(
                (*c).pw,
                image,
                dict.as_ptr(),
                glyph.mask.as_ptr().cast(),
                glyph.mask.len(),
                true,
            );
            images.push((*code, image));
        }
        pdfw_stream(
            (*c).pw,
            procedure,
            std::ptr::null(),
            body.as_ptr().cast(),
            body.len(),
            true,
        );
        procedures.push((*code, procedure));
    }
    let unicode = unicode(c, font, &codes);
    let descriptor = pdfw_alloc((*c).pw);
    let out = pdfw_out((*c).pw);
    pdfw_begin((*c).pw, (*font).obj);
    let (dpi, vdpi) = pk.dpi();
    append(out,&format!("<</Type/Font/Subtype/Type3/Name/PitexPK{index}/FontMatrix[.001 0 0 .001 0 0]/FontBBox[{} {} {} {}]/FontDescriptor {descriptor} 0 R/PitexPKResolution[{} {}]/CharProcs<<",number(bounds[0]),number(bounds[1]),number(bounds[2]),number(bounds[3]),number(dpi),number(vdpi)));
    for (code, object) in &procedures {
        append(out, &format!("/a{code} {object} 0 R"));
    }
    append(out, ">>/Encoding<</Type/Encoding/Differences[");
    for code in &codes {
        append(out, &format!("{code}/a{code} "));
    }
    let first = *codes.first().unwrap();
    let last = *codes.last().unwrap();
    append(
        out,
        &format!("]>>/FirstChar {first}/LastChar {last}/Widths["),
    );
    for code in first..=last {
        let width = if !(*font).tfm.is_null() && (*(*font).tfm).exists[code] {
            (*(*font).tfm).width[code] as f64 / 1048.576
        } else {
            0.
        };
        append(out, &format!("{} ", number(width)));
    }
    append(out, "]/Resources<</XObject<<");
    for (code, object) in images {
        append(out, &format!("/M{code} {object} 0 R"));
    }
    append(out, ">>>>");
    if let Some(object) = unicode {
        append(out, &format!("/ToUnicode {object} 0 R"));
    }
    font_output::attributes(c, font, out);
    append(out, ">>");
    pdfw_end((*c).pw);
    pdfw_begin((*c).pw, descriptor);
    append(out,&format!("<</Type/FontDescriptor/FontName/PitexPK{index}/Flags 4/FontBBox[{} {} {} {}]/ItalicAngle 0/Ascent {}/Descent {}/CapHeight {}/StemV 0",number(bounds[0]),number(bounds[1]),number(bounds[2]),number(bounds[3]),number(bounds[3]),number(bounds[1]),number(bounds[3])));
    if (*(*c).document).backend.integer("pdfomitcharset") == 0 {
        append(
            out,
            &format!(
                "/CharSet ({})",
                codes
                    .iter()
                    .map(|code| format!("/a{code}"))
                    .collect::<String>()
            ),
        );
    }
    append(out, ">>");
    pdfw_end((*c).pw);
}
