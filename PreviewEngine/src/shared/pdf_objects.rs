// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// Original PDF 1.5 object-stream and cross-reference-stream writer.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy)]
struct Record {
    start: usize,
    body: usize,
    end: usize,
    stream: bool,
}
#[derive(Default)]
pub struct ObjectStreams {
    level: i32,
    active: Option<(i32, Record)>,
    objects: BTreeMap<i32, Record>,
    imported: BTreeSet<i32>,
    repacking: bool,
}
impl ObjectStreams {
    pub unsafe fn begin(w: *mut pdfw, object: i32, start: usize, body: usize) {
        let state = &mut *(*w).object_streams;
        if !state.repacking {
            state.active = Some((
                object,
                Record {
                    start,
                    body,
                    end: body,
                    stream: false,
                },
            ));
        }
    }
    pub unsafe fn stream(w: *mut pdfw) {
        let state = &mut *(*w).object_streams;
        if let Some((_, record)) = &mut state.active {
            record.stream = true;
        }
    }
    pub unsafe fn end(w: *mut pdfw, end: usize) {
        let state = &mut *(*w).object_streams;
        if state.repacking {
            return;
        }
        if let Some((object, mut record)) = state.active.take() {
            record.end = end;
            state.objects.insert(object, record);
        }
    }
    pub unsafe fn imported(w: *mut pdfw, object: i32) {
        (*(*w).object_streams).imported.insert(object);
    }
    pub unsafe fn configure(w: *mut pdfw, level: i32) {
        (*(*w).object_streams).level = level.clamp(0, 3);
    }
    pub unsafe fn finish(w: *mut pdfw, root: i32, info: i32) -> bool {
        let state = &mut *(*w).object_streams;
        if state.level == 0 || (*w).requested_version < 15 {
            return false;
        }
        let original = std::slice::from_raw_parts((*(*w).out).data, (*(*w).out).len).to_vec();
        let selected = state
            .objects
            .iter()
            .filter_map(|(object, record)| {
                if record.stream
                    || record.end <= record.body
                    || record.end - record.body > 65536
                    || (state.level < 3 && *object == info)
                    || (state.level == 1 && state.imported.contains(object))
                {
                    return None;
                }
                // Objects explicitly written as streams by an import or special
                // remain standalone, including non-Flate-encoded streams.
                let body = &original[record.body..record.end];
                if body.windows(8).any(|p| p == b"\nstream\n") {
                    return None;
                }
                Some(*object)
            })
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return false;
        }
        let records = state.objects.clone();
        state.repacking = true;
        let selected_set = selected.iter().copied().collect::<BTreeSet<_>>();
        let first = records
            .values()
            .map(|r| r.start)
            .min()
            .unwrap_or(original.len());
        let mut rewritten = original[..first].to_vec();
        let mut ordered = records.iter().map(|(id, r)| (*id, *r)).collect::<Vec<_>>();
        ordered.sort_by_key(|(_, r)| r.start);
        for (object, record) in &ordered {
            *(*w).offsets.add(*object as usize) = 0;
            if !selected_set.contains(object) {
                *(*w).offsets.add(*object as usize) = rewritten.len();
                rewritten.extend_from_slice(&original[record.start..record.end]);
            }
        }
        (*(*w).out).len = 0;
        pbuf_append((*w).out, rewritten.as_ptr().cast(), rewritten.len());
        let mut compressed = BTreeMap::<i32, (i32, u32)>::new();
        for group in selected.chunks(100) {
            let object = pdfw_alloc(w);
            let mut header = String::new();
            let mut bodies = Vec::new();
            for (index, id) in group.iter().enumerate() {
                header.push_str(&format!("{id} {} ", bodies.len()));
                let record = records[id];
                let body_end = record.end - b"\nendobj\n".len();
                bodies.extend_from_slice(&original[record.body..body_end]);
                bodies.push(b'\n');
                compressed.insert(*id, (object, index as u32));
            }
            let first = header.len();
            let mut data = header.into_bytes();
            data.extend_from_slice(&bodies);
            let dict =
                std::ffi::CString::new(format!("/Type/ObjStm/N {}/First {first}", group.len()))
                    .unwrap();
            pdfw_stream(
                w,
                object,
                dict.as_ptr(),
                data.as_ptr().cast(),
                data.len(),
                true,
            );
        }
        let xref_object = pdfw_alloc(w);
        let xref = (*(*w).out).len;
        *(*w).offsets.add(xref_object as usize) = xref;
        let mut entries = Vec::with_capacity((*w).count as usize * 13);
        for id in 0..(*w).count {
            let (kind, field, generation) = if id == 0 {
                (0u8, 0u64, 65535u32)
            } else if let Some((stream, index)) = compressed.get(&id) {
                (2, *stream as u64, *index)
            } else {
                let offset = *(*w).offsets.add(id as usize);
                if offset == 0 {
                    (0, 0, 0)
                } else {
                    (1, offset as u64, 0)
                }
            };
            entries.push(kind);
            entries.extend_from_slice(&field.to_be_bytes());
            entries.extend_from_slice(&generation.to_be_bytes());
        }
        let mut dict = format!("/Type/XRef/Size {}/W[1 8 4]/Root {root} 0 R", (*w).count);
        if info > 0 {
            dict.push_str(&format!("/Info {info} 0 R"));
        }
        if (*w).trailer.len > 0 {
            dict.push(' ');
            dict.push_str(&String::from_utf8_lossy(std::slice::from_raw_parts(
                (*w).trailer.data,
                (*w).trailer.len,
            )));
        }
        let dict = std::ffi::CString::new(dict).unwrap();
        pdfw_stream(
            w,
            xref_object,
            dict.as_ptr(),
            entries.as_ptr().cast(),
            entries.len(),
            true,
        );
        pbuf_printf(
            (*w).out,
            b"startxref\n%zu\n%%%%EOF\n\0".as_ptr().cast(),
            xref,
        );
        (*w).minimum_version = (*w).minimum_version.max(15);
        true
    }
}
