// Original Pitex native PDF resource bridge for interpreted PostScript shading.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
pub unsafe fn resources(c: *mut conv_ctx) {
    for shading in (*(*c).postscript).shading_resources() {
        let existing = (*(*c).document)
            .postscript_resources
            .get(&shading.name)
            .copied();
        let object = if let Some(object) = existing {
            object
        } else {
            let ids = shading
                .objects
                .iter()
                .map(|_| pdfw_alloc((*c).pw))
                .collect::<Vec<_>>();
            for (index, object) in shading.objects.iter().enumerate() {
                let mut dictionary = object.dictionary.clone();
                for (local, id) in ids.iter().enumerate() {
                    dictionary = dictionary.replace(&format!("@@{local}@@"), &format!("{id} 0 R"));
                }
                if let Some(data) = &object.data {
                    let text = std::ffi::CString::new(dictionary).unwrap();
                    pdfw_stream(
                        (*c).pw,
                        ids[index],
                        text.as_ptr(),
                        data.as_ptr().cast(),
                        data.len(),
                        true,
                    );
                } else {
                    let text = std::ffi::CString::new(if dictionary.trim_start().starts_with('['){dictionary}else{format!("<<{dictionary}>>")}).unwrap();
                    pdfw_begin((*c).pw, ids[index]);
                    pbuf_puts(pdfw_out((*c).pw), text.as_ptr());
                    pdfw_end((*c).pw);
                }
            }
            let object = ids[shading.root];
            (*(*c).document)
                .postscript_resources
                .insert(shading.name.clone(), object);
            object
        };
        let category=if shading.name.starts_with("PSTimage"){CAT_XOBJECT}else if shading.name.starts_with("PSTcolor"){CAT_COLORSPACE}else{CAT_SHADING};
        let key = std::ffi::CString::new(shading.name).unwrap();
        let reference = format!("{object} 0 R");
        kv_set(
            &raw mut (*cur(c)).res.cat[category as usize],
            key.as_ptr(),
            key.as_bytes().len(),
            reference.as_ptr().cast(),
            reference.len(),
        );
    }
}
