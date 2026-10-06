// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// Original PDF inclusion policy and compatible mapped-Type1 font substitution.
use super::*;
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Policy {
    pub fatal: bool,
    pub page_groups: BTreeMap<i32, usize>,
    font_programs: BTreeMap<String, i32>,
}
unsafe fn warning(c: *mut conv_ctx, key: &str, message: &str) {
    let key = std::ffi::CString::new(key).unwrap();
    let message = std::ffi::CString::new(message).unwrap();
    warn_once(c, key.as_ptr(), b"%s\0".as_ptr().cast(), message.as_ptr());
}
pub unsafe fn version(
    c: *mut conv_ctx,
    doc: *mut pr_doc,
    file: *const ::core::ffi::c_char,
) -> bool {
    let source = pr_version(doc);
    let requested = (*(*c).document).backend.requested_version;
    if source <= requested {
        return true;
    }
    let legacy = (*(*c).document)
        .backend
        .integer("pdfoptionpdfinclusionerrorlevel");
    let level = if legacy != 0 {
        legacy
    } else {
        (*(*c).document).backend.integer("pdfinclusionerrorlevel")
    };
    let filename = std::ffi::CStr::from_ptr(file).to_string_lossy();
    if level >= 0 {
        warning(
            c,
            &format!("pdf-version-{filename}"),
            &format!(
                "PDF inclusion: {} has PDF version {}.{}, newer than requested {}.{}",
                filename,
                source / 10,
                source % 10,
                requested / 10,
                requested % 10
            ),
        );
    }
    if level > 0 {
        (*(*c).document).inclusion.fatal = true;
        false
    } else {
        true
    }
}
pub unsafe fn used(c: *mut conv_ctx, image: *mut cached_image, page: i32, box_kind: i32) {
    if image.is_null() || !(*image).pdf || (*image).doc.is_null() {
        return;
    }
    let mut info = pr_page_info {
        box_0: [0.; 4],
        rotate: 0,
        page: std::ptr::null_mut(),
        resources: std::ptr::null_mut(),
    };
    if !pr_page(
        (*image).doc,
        pr_normalize_page((*image).doc, page) - 1,
        if box_kind != 0 {
            box_kind
        } else {
            PR_BOX_CROP as i32
        },
        &mut info,
    ) {
        return;
    }
    if pr_get((*image).doc, info.page, b"Group\0".as_ptr().cast()).is_null() {
        return;
    }
    let current = (*c).this_page;
    let count = (*(*c).document)
        .inclusion
        .page_groups
        .entry(current)
        .or_default();
    *count += 1;
    if *count > 1
        && (*(*c).document)
            .backend
            .integer("pdfsuppresswarningpagegroup")
            <= 0
    {
        warning(
            c,
            &format!("pdf-page-group-{current}"),
            "PDF inclusion: multiple page groups on one page; imported Form groups remain isolated",
        );
    }
}
unsafe fn name(doc: *mut pr_doc, dict: *mut pr_obj, key: &'static [u8]) -> Option<String> {
    let value = pr_get(doc, dict, key.as_ptr().cast());
    if value.is_null() || (*value).type_0 != PR_NAME {
        return None;
    }
    Some(
        String::from_utf8_lossy(std::slice::from_raw_parts(
            (*value).u.str_0.s.cast::<u8>(),
            (*value).u.str_0.len,
        ))
        .into_owned(),
    )
}
unsafe fn font_dictionary(
    imp: *mut pdfw_import,
    out: *mut pbuf,
    source: *mut pr_obj,
    skip: &[&str],
) {
    if source.is_null() || (*source).type_0 != PR_DICT {
        return;
    }
    for index in 0..(*source).u.dict.n {
        let key = *(*source).u.dict.keys.add(index as usize);
        let key_text = std::ffi::CStr::from_ptr(key).to_string_lossy();
        if skip.iter().any(|item| *item == key_text) {
            continue;
        }
        pdfw_name(out, key);
        pbuf_putc(out, b' ' as i32);
        pdfw_import_value(imp, out, *(*source).u.dict.vals.add(index as usize));
        pbuf_putc(out, b'\n' as i32);
    }
}
pub unsafe fn fonts(
    c: *mut conv_ctx,
    doc: *mut pr_doc,
    resources: *mut pr_obj,
    imp: *mut pdfw_import,
) {
    if (*(*c).document).backend.integer("pdfinclusioncopyfonts") > 0 {
        return;
    }
    let fonts = pr_get(doc, resources, b"Font\0".as_ptr().cast());
    if fonts.is_null() || (*fonts).type_0 != PR_DICT {
        return;
    }
    for index in 0..(*fonts).u.dict.n {
        let reference = *(*fonts).u.dict.vals.add(index as usize);
        if reference.is_null() || (*reference).type_0 != PR_REF {
            continue;
        }
        let source = pr_resolve(doc, reference);
        if name(doc, source, b"Subtype\0").as_deref() != Some("Type1") {
            continue;
        }
        let Some(base) = name(doc, source, b"BaseFont\0") else {
            continue;
        };
        let canonical = if base.len() > 7
            && base.as_bytes()[6] == b'+'
            && base.as_bytes()[..6].iter().all(u8::is_ascii_uppercase)
        {
            &base[7..]
        } else {
            &base
        };
        let Ok(psname) = std::ffi::CString::new(canonical) else {
            continue;
        };
        let map = font_map_ps_lookup((*(*c).w).fonts, psname.as_ptr());
        if map.is_null() || (*map).fontfile.is_null() {
            continue;
        }
        let file = std::ffi::CStr::from_ptr((*map).fontfile)
            .to_string_lossy()
            .into_owned();
        if (*map).slant.abs() > 1e-9
            || ((*map).extend - 1.).abs() > 1e-9
            || [".ttf", ".otf", ".ttc"]
                .iter()
                .any(|extension| file.to_ascii_lowercase().ends_with(extension))
        {
            continue;
        }

        let local = font_type1((*(*c).w).fonts, (*map).fontfile, (*c).warnings);
        if local.is_null() || !(*local).ok {
            continue;
        }
        let descriptor = pr_get(doc, source, b"FontDescriptor\0".as_ptr().cast());
        if descriptor.is_null() || (*descriptor).type_0 != PR_DICT {
            continue;
        }
        let program = if let Some(object) = (*(*c).document).inclusion.font_programs.get(&file) {
            *object
        } else {
            let object = pdfw_alloc((*c).pw);
            let dict = std::ffi::CString::new(format!(
                "/Length1 {}/Length2 {}/Length3 {}",
                (*local).len1,
                (*local).len2,
                (*local).len3
            ))
            .unwrap();
            pdfw_stream(
                (*c).pw,
                object,
                dict.as_ptr(),
                (*local).data.cast(),
                (*local).len1 + (*local).len2 + (*local).len3,
                true,
            );
            (*(*c).document)
                .inclusion
                .font_programs
                .insert(file, object);
            object
        };
        let output_descriptor = pdfw_alloc((*c).pw);
        pdfw_begin((*c).pw, output_descriptor);
        let out = pdfw_out((*c).pw);
        pbuf_puts(out, b"<<\0".as_ptr().cast());
        font_dictionary(
            imp,
            out,
            descriptor,
            &["FontFile", "FontFile2", "FontFile3", "FontName"],
        );
        pbuf_puts(out, b"/FontName\0".as_ptr().cast());
        pdfw_name(out, (*local).fontname.as_ptr());
        pbuf_printf(out, b"/FontFile %d 0 R>>\0".as_ptr().cast(), program);
        pdfw_end((*c).pw);
        let output_font = pdfw_alloc((*c).pw);
        pdfw_begin((*c).pw, output_font);
        pbuf_puts(out, b"<<\0".as_ptr().cast());
        font_dictionary(imp, out, source, &["BaseFont", "FontDescriptor"]);
        pbuf_puts(out, b"/BaseFont\0".as_ptr().cast());
        pdfw_name(out, (*local).fontname.as_ptr());
        pbuf_printf(
            out,
            b"/FontDescriptor %d 0 R>>\0".as_ptr().cast(),
            output_descriptor,
        );
        pdfw_end((*c).pw);
        pdfw_import_redirect(imp, (*reference).u.ref_0.num, output_font);
    }
}
