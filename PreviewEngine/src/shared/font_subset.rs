// Original Pitex font subsetting: embedded programs keep only the glyphs a
// publication uses, and faces shared by several font resources are written
// once (ISO 32000-1 9.6.4, 9.8, 9.9). Glyph ids and character codes do not
// change, so widths, encodings and ToUnicode CMaps stay as written.
// Formats: OpenType 'glyf'/'loca' and 'CFF ' (Adobe TN 5176, Type 2
// charstrings TN 5177), Type 1 (Adobe Type 1 Font Format, eexec and
// charstring encryption).
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use std::collections::{BTreeSet, HashMap};

/// Session state on the converter: subset programs survive publications so an
/// edit that uses the same glyphs reuses the compressed bytes.
#[derive(Default)]
pub struct Cache {
    epoch: i32,
    descriptors: HashMap<usize, i32>,
    programs: HashMap<usize, Program>,
    encodings: HashMap<usize, Vec<Option<String>>>,
}
struct Program {
    key: Vec<u8>,
    /// `None` keeps the whole program (a format this module does not subset).
    deflated: Option<Vec<u8>>,
    lengths: [usize; 3],
    /// `deflated` is a bare CFF rather than an sfnt.
    bare: bool,
}

unsafe fn cache(c: *mut conv_ctx) -> &'static mut Cache {
    let w = (*c).w;
    let cache = &mut *(*w).subsets;
    if cache.epoch != (*w).epoch {
        cache.epoch = (*w).epoch;
        cache.descriptors.clear();
    }
    cache
}
unsafe fn fonts(c: *mut conv_ctx) -> impl Iterator<Item = *mut pdf_font> {
    (0..(*c).npfonts.max(0) as usize).map(move |i| (*c).pfonts.add(i))
}
unsafe fn deflate(data: &[u8]) -> Vec<u8> {
    let mut out = pbuf { data: std::ptr::null_mut(), len: 0, cap: 0 };
    pbuf_deflate(&mut out, data.as_ptr(), data.len(), 6);
    let bytes = std::slice::from_raw_parts(out.data, out.len).to_vec();
    pbuf_free(&mut out);
    bytes
}
unsafe fn put(o: *mut pbuf, text: &str) {
    pbuf_append(o, text.as_ptr().cast(), text.len());
}
unsafe fn reals(o: *mut pbuf, values: &[f64]) {
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            pbuf_putc(o, ' ' as i32);
        }
        pbuf_real(o, *value);
    }
}

/// Glyph names of the codes `font` uses, from its encoding vector or the
/// program's built-in encoding.
pub unsafe fn used_names(c: *mut conv_ctx, font: *mut pdf_font) -> Vec<(usize, String)> {
    let builtin = if (*font).enc.is_null() {
        let t = (*font).t1;
        let clear = std::slice::from_raw_parts((*t).data, (*t).len1);
        Some(cache(c).encodings.entry(t as usize).or_insert_with(|| builtin_encoding(clear)))
    } else {
        None
    };
    (0..256usize)
        .filter(|&code| *(*font).used.add(code) != 0)
        .filter_map(|code| {
            let name = match &builtin {
                Some(names) => names[code].clone()?,
                None => {
                    let name = (*(*font).enc).glyph[code];
                    if name.is_null() {
                        return None;
                    }
                    std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned()
                }
            };
            Some((code, name))
        })
        .collect()
}

/// FontDescriptor of an OpenType/TrueType face, written once per publication
/// with a program that holds the glyphs of every resource using the face.
pub unsafe fn native_descriptor(c: *mut conv_ctx, p: *mut pdf_font) -> i32 {
    let f = (*p).nf;
    let cache = cache(c);
    if let Some(&fd) = cache.descriptors.get(&(f as usize)) {
        return fd;
    }
    let mut used = vec![false; (*f).num_glyphs.max(0) as usize];
    for q in fonts(c).filter(|&q| (*q).native && (*q).nf == f) {
        for (g, slot) in used.iter_mut().enumerate().take((*q).nused.max(0) as usize) {
            *slot |= *(*q).used.add(g) != 0;
        }
    }
    let key = used.iter().map(|&u| u as u8).collect::<Vec<_>>();
    let program = cache.programs.entry(f as usize).or_insert(Program {
        key: Vec::new(),
        deflated: None,
        lengths: [0; 3],
        bare: false,
    });
    if program.key != key || program.lengths[0] == 0 {
        let font = std::slice::from_raw_parts((*f).sfnt, (*f).sfnt_len);
        let bare = if (*f).cff { bare_cff(font, &used) } else { None };
        program.bare = bare.is_some();
        let subset = bare.or_else(|| subset_sfnt(font, &used));
        program.lengths = [subset.as_ref().map_or(font.len(), Vec::len), 0, 0];
        program.deflated = subset.map(|s| deflate(&s));
        program.key = key;
    }
    let (fd, ff) = (pdfw_alloc((*c).pw), pdfw_alloc((*c).pw));
    let o = pdfw_out((*c).pw);
    pdfw_begin((*c).pw, fd);
    put(o, "<</Type/FontDescriptor/FontName");
    pdfw_name(o, (*f).psname.as_ptr());
    put(o, "/Flags 4/FontBBox[");
    reals(o, &(*f).bbox);
    put(o, "]/ItalicAngle ");
    reals(o, &[(*f).italic_angle]);
    put(o, "/Ascent ");
    reals(o, &[(*f).ascent]);
    put(o, "/Descent ");
    reals(o, &[(*f).descent]);
    put(o, "/CapHeight ");
    reals(o, &[(*f).cap_height]);
    put(o, &format!("/StemV 80/{} {ff} 0 R>>", if (*f).cff { "FontFile3" } else { "FontFile2" }));
    pdfw_end((*c).pw);
    let dict = if program.bare {
        "/Subtype/CIDFontType0C".to_string()
    } else if (*f).cff {
        "/Subtype/OpenType".to_string()
    } else {
        format!("/Length1 {}", program.lengths[0])
    };
    write_program(c, ff, &dict, program.deflated.as_deref(), &raw mut (*f).deflated, (*f).sfnt, (*f).sfnt_len);
    cache.descriptors.insert(f as usize, fd);
    fd
}

/// FontDescriptor of a Type 1 face, shared by every resource that uses it.
pub unsafe fn type1_descriptor(c: *mut conv_ctx, p: *mut pdf_font) -> i32 {
    let t = (*p).t1;
    if let Some(&fd) = cache(c).descriptors.get(&(t as usize)) {
        return fd;
    }
    let mut names = BTreeSet::new();
    let mut complete = true;
    for q in fonts(c).filter(|&q| !(*q).native && !(*q).bitmap && (*q).t1 == t) {
        let named = used_names(c, q);
        let used = (0..256).filter(|&code| *(*q).used.add(code) != 0).count();
        // A used code without a glyph name could still select a glyph; keep the whole program.
        complete &= named.len() == used;
        names.extend(named.into_iter().map(|(_, name)| name));
    }
    let mut key = names.iter().flat_map(|n| n.bytes().chain([b'/'])).collect::<Vec<_>>();
    key.push(complete as u8);
    let cache = cache(c);
    let program = cache.programs.entry(t as usize).or_insert(Program {
        key: Vec::new(),
        deflated: None,
        lengths: [0; 3],
        bare: false,
    });
    let data = std::slice::from_raw_parts((*t).data, (*t).len1 + (*t).len2 + (*t).len3);
    if program.key != key || program.lengths[0] == 0 {
        let (clear, rest) = data.split_at((*t).len1);
        let (binary, trailer) = rest.split_at((*t).len2);
        let keep = names.iter().map(String::as_bytes).collect::<BTreeSet<_>>();
        let subset = complete.then(|| subset_type1(binary, &keep)).flatten();
        program.lengths = [clear.len(), subset.as_ref().map_or(binary.len(), Vec::len), trailer.len()];
        program.deflated = subset.map(|binary| deflate(&[clear, &binary, trailer].concat()));
        program.key = key;
    }
    let (fd, ff) = (pdfw_alloc((*c).pw), pdfw_alloc((*c).pw));
    let o = pdfw_out((*c).pw);
    pdfw_begin((*c).pw, fd);
    put(o, "<</Type/FontDescriptor/FontName");
    pdfw_name(o, (*t).fontname.as_ptr());
    put(o, "/Flags 4/FontBBox[");
    reals(o, &(*t).bbox);
    // pdfTeX takes these from the TFM; pdftotext sizes each glyph's box from Ascent and Descent.
    let [ascent, cap_height, descent] = match (*p).tfm.as_ref() {
        Some(tfm) if tfm.ok => tfm.descriptor,
        _ => [(*t).bbox[3], (*t).bbox[3], (*t).bbox[1]],
    };
    put(o, "]/ItalicAngle 0/Ascent ");
    reals(o, &[ascent]);
    put(o, "/Descent ");
    reals(o, &[descent]);
    put(o, "/CapHeight ");
    reals(o, &[cap_height]);
    put(o, &format!("/StemV 80/FontFile {ff} 0 R"));
    if (*(*c).document).backend.integer("pdfomitcharset") == 0 {
        let set = names.iter().map(|n| format!("/{n}")).collect::<String>();
        let set = set.replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)");
        put(o, &format!("/CharSet ({set})"));
    }
    put(o, ">>");
    pdfw_end((*c).pw);
    let [l1, l2, l3] = program.lengths;
    let dict = format!("/Length1 {l1}/Length2 {l2}/Length3 {l3}");
    write_program(c, ff, &dict, program.deflated.as_deref(), &raw mut (*t).deflated, (*t).data, data.len());
    cache.descriptors.insert(t as usize, fd);
    fd
}
unsafe fn write_program(
    c: *mut conv_ctx,
    object: i32,
    dict: &str,
    subset: Option<&[u8]>,
    whole: *mut pbuf,
    data: *const u8,
    len: usize,
) {
    let dict = std::ffi::CString::new(dict).unwrap();
    let bytes = match subset {
        Some(bytes) => bytes,
        None => {
            if (*whole).len == 0 {
                pbuf_deflate(whole, data, len, 6);
            }
            std::slice::from_raw_parts((*whole).data, (*whole).len)
        }
    };
    pdfw_stream_deflated((*c).pw, object, dict.as_ptr(), bytes.as_ptr().cast(), bytes.len());
}

fn be(d: &[u8], at: usize, size: usize) -> Option<usize> {
    Some(d.get(at..at.checked_add(size)?)?.iter().fold(0, |a, &b| a << 8 | b as usize))
}

/// The sfnt with unused glyph outlines removed and layout tables dropped.
/// `None` when the face has neither 'glyf' nor 'CFF ' outlines or is malformed.
pub fn subset_sfnt(font: &[u8], used: &[bool]) -> Option<Vec<u8>> {
    // fvar and avar stay so CFF2 blends keep their default-instance meaning.
    const KEEP: [&[u8; 4]; 19] = [
        b"CFF ", b"CFF2", b"OS/2", b"avar", b"cmap", b"cvt ", b"fpgm", b"fvar", b"glyf", b"head",
        b"hhea", b"hmtx", b"loca", b"maxp", b"name", b"post", b"prep", b"vhea", b"vmtx",
    ];
    let mut tables = Vec::new();
    for i in 0..be(font, 4, 2)? {
        let record = 12 + 16 * i;
        let tag: [u8; 4] = font.get(record..record + 4)?.try_into().ok()?;
        let offset = be(font, record + 8, 4)?;
        let data = font.get(offset..offset.checked_add(be(font, record + 12, 4)?)?)?;
        if KEEP.contains(&&tag) {
            tables.push((tag, data.to_vec()));
        }
    }
    let table = |tag: &[u8; 4]| tables.iter().position(|(t, _)| t == tag);
    let (glyf, loca, head, cff, cff2) = (table(b"glyf"), table(b"loca"), table(b"head"), table(b"CFF "), table(b"CFF2"));
    let mut subset = false;
    if let (Some(glyf), Some(loca), Some(head)) = (glyf, loca, head) {
        let long = be(&tables[head].1, 50, 2)? != 0;
        let (outlines, offsets) = subset_glyf(&tables[glyf].1, &tables[loca].1, long, used)?;
        tables[glyf].1 = outlines;
        tables[loca].1 = offsets;
        subset = true;
    }
    if let Some(cff) = cff {
        tables[cff].1 = subset_cff(&tables[cff].1, used)?;
        subset = true;
    }
    if let Some(cff2) = cff2 {
        tables[cff2].1 = subset_cff2(&tables[cff2].1, used)?;
        subset = true;
    }
    if !subset {
        return None;
    }
    tables.sort_by(|a, b| a.0.cmp(&b.0));
    let count = tables.len();
    let range = 16 << (usize::BITS - 1 - count.leading_zeros());
    let mut out = font[..4].to_vec();
    for v in [count, range, range.trailing_zeros() as usize - 4, count * 16 - range] {
        out.extend((v as u16).to_be_bytes());
    }
    let mut offset = 12 + 16 * count;
    for (tag, data) in &tables {
        let padded = data.len().div_ceil(4) * 4;
        let sum = data
            .chunks(4)
            .map(|w| be(&[w, &[0; 3]].concat(), 0, 4).unwrap() as u32)
            .fold(0u32, u32::wrapping_add);
        out.extend(tag);
        for v in [sum as usize, offset, data.len()] {
            out.extend((v as u32).to_be_bytes());
        }
        offset += padded;
    }
    for (_, data) in &tables {
        out.extend(data);
        out.resize(out.len().div_ceil(4) * 4, 0);
    }
    Some(out)
}

/// TrueType outlines of the used glyphs, .notdef and composite components.
fn subset_glyf(glyf: &[u8], loca: &[u8], long: bool, used: &[bool]) -> Option<(Vec<u8>, Vec<u8>)> {
    let offset = |g: usize| if long { be(loca, 4 * g, 4) } else { be(loca, 2 * g, 2).map(|v| v * 2) };
    let outline = |g: usize| glyf.get(offset(g)?..offset(g + 1)?);
    let mut keep = used.to_vec();
    if let Some(notdef) = keep.first_mut() {
        *notdef = true;
    }
    let mut pending = (0..keep.len()).filter(|&g| keep[g]).collect::<Vec<_>>();
    while let Some(g) = pending.pop() {
        let data = outline(g)?;
        if data.len() < 10 || data[0] & 0x80 == 0 {
            continue;
        }
        let mut at = 10;
        loop {
            let flags = be(data, at, 2)?;
            let component = be(data, at + 2, 2)?;
            if component < keep.len() && !keep[component] {
                keep[component] = true;
                pending.push(component);
            }
            at += 4 + if flags & 1 != 0 { 4 } else { 2 };
            at += if flags & 8 != 0 { 2 } else if flags & 0x40 != 0 { 4 } else if flags & 0x80 != 0 { 8 } else { 0 };
            if flags & 0x20 == 0 {
                break;
            }
        }
    }
    let (mut outlines, mut offsets) = (Vec::new(), Vec::new());
    for g in 0..=keep.len() {
        if long {
            offsets.extend((outlines.len() as u32).to_be_bytes());
        } else {
            offsets.extend(((outlines.len() / 2) as u16).to_be_bytes());
        }
        if g < keep.len() && keep[g] {
            outlines.extend(outline(g)?);
            outlines.resize(outlines.len().div_ceil(2) * 2, 0);
        }
    }
    Some((outlines, offsets))
}

/// CFF INDEX at `at`: absolute start of each object plus the end of the last.
fn cff_index(d: &[u8], at: usize) -> Option<Vec<usize>> {
    sized_index(d, at, 2)
}
/// An INDEX whose count takes `count` bytes: 2 in CFF, 4 in CFF2.
fn sized_index(d: &[u8], at: usize, count: usize) -> Option<Vec<usize>> {
    let (count, at) = (be(d, at, count)?, at + count - 2);
    if count == 0 {
        return Some(vec![at + 2]);
    }
    let size = *d.get(at + 2)? as usize;
    if !(1..=4).contains(&size) {
        return None;
    }
    let base = at + 2 + (count + 1) * size;
    let starts = (0..=count)
        .map(|i| be(d, at + 3 + i * size, size).map(|v| base + v))
        .collect::<Option<Vec<_>>>()?;
    (*starts.last()? <= d.len()).then_some(starts)
}
/// Integer operands of `operator` in a CFF DICT (reals read as 0).
fn dict_operands(dict: &[u8], operator: u16) -> Option<Vec<i64>> {
    let (mut at, mut operands) = (0, Vec::new());
    while at < dict.len() {
        let b = dict[at] as i64;
        let next = dict.get(at + 1).map_or(0, |&v| v as i64);
        match dict[at] {
            0..=21 => {
                let op = if b == 12 { 0x0c00 | next as u16 } else { b as u16 };
                if op == operator {
                    return Some(operands);
                }
                at += if b == 12 { 2 } else { 1 };
                operands.clear();
                continue;
            }
            28 => operands.push(be(dict, at + 1, 2)? as i16 as i64),
            29 => operands.push(be(dict, at + 1, 4)? as i32 as i64),
            30 => {
                let length = dict[at + 1..].iter().position(|&v| v & 0x0f == 0x0f || v >> 4 == 0x0f)?;
                operands.push(0);
                at += length + 1;
            }
            32..=246 => operands.push(b - 139),
            247..=250 => operands.push((b - 247) * 256 + next + 108),
            251..=254 => operands.push(-(b - 251) * 256 - next - 108),
            _ => return None,
        }
        at += match dict[at] {
            28 => 3,
            29 => 5,
            247..=254 => 2,
            _ => 1,
        };
    }
    None
}
fn dict_offset(dict: &[u8], operator: u16) -> Option<usize> {
    usize::try_from(*dict_operands(dict, operator)?.last()?).ok()
}
/// Rewrites the INDEX at `at` in place, keeping the objects `keep` selects
/// and replacing the others with `filler`. The new INDEX is never longer, so
/// no other offset moves; the freed tail is zeroed and compresses away.
fn blank_index(cff: &[u8], out: &mut [u8], at: usize, keep: &[bool], filler: &[u8], count: usize) -> Option<()> {
    let starts = sized_index(cff, at, count)?;
    if starts.len() != keep.len() + 1 || keep.is_empty() {
        return None;
    }
    let at = at + count - 2;
    let size = cff[at + 2] as usize;
    let mut data = Vec::new();
    for (i, &kept) in keep.iter().enumerate() {
        let field = at + 3 + i * size;
        out[field..field + size].copy_from_slice(&(data.len() + 1).to_be_bytes()[8 - size..]);
        match kept {
            true => data.extend_from_slice(&cff[starts[i]..starts[i + 1]]),
            false => data.extend_from_slice(filler),
        }
    }
    let field = at + 3 + keep.len() * size;
    out[field..field + size].copy_from_slice(&(data.len() + 1).to_be_bytes()[8 - size..]);
    let end = starts[0] + data.len();
    if end > starts[keep.len()] {
        return None;
    }
    out[starts[0]..end].copy_from_slice(&data);
    out[end..starts[keep.len()]].fill(0);
    Some(())
}
/// A Type 2 subroutine INDEX and the subroutines the kept glyphs call.
struct Subrs {
    at: usize,
    starts: Vec<usize>,
    used: Vec<bool>,
}
impl Subrs {
    fn read(cff: &[u8], at: Option<usize>) -> Option<Self> {
        let Some(at) = at else {
            return Some(Self { at: 0, starts: vec![0], used: Vec::new() });
        };
        let starts = cff_index(cff, at)?;
        Some(Self { at, used: vec![false; starts.len() - 1], starts })
    }
    fn bias(&self) -> i64 {
        match self.used.len() {
            0..=1239 => 107,
            1240..=33899 => 1131,
            _ => 32768,
        }
    }
}
/// Follows a Type 2 charstring and marks the subroutines it calls. Hint
/// counts are tracked because `hintmask` data bytes depend on them.
fn walk(cff: &[u8], code: std::ops::Range<usize>, subrs: &mut [Subrs; 2], stems: &mut usize, stack: &mut Vec<i64>, depth: u8) -> Option<bool> {
    let mut at = code.start;
    while at < code.end {
        let b = cff[at] as i64;
        let next = cff.get(at + 1).map_or(0, |&v| v as i64);
        at += 1;
        match b {
            28 => {
                stack.push(be(cff, at, 2)? as i16 as i64);
                at += 2;
            }
            32..=246 => stack.push(b - 139),
            247..=250 => {
                stack.push((b - 247) * 256 + next + 108);
                at += 1;
            }
            251..=254 => {
                stack.push(-(b - 251) * 256 - next - 108);
                at += 1;
            }
            255 => {
                stack.push(be(cff, at, 4)? as i32 as i64 >> 16);
                at += 4;
            }
            1 | 3 | 18 | 23 => {
                *stems += stack.len() / 2;
                stack.clear();
            }
            19 | 20 => {
                *stems += stack.len() / 2;
                stack.clear();
                at += stems.div_ceil(8);
            }
            10 | 29 => {
                let set = &mut subrs[(b == 29) as usize];
                let index = usize::try_from(stack.pop()? + set.bias()).ok()?;
                *set.used.get_mut(index)? = true;
                let body = set.starts[index]..set.starts[index + 1];
                if depth > 10 {
                    return None;
                }
                if walk(cff, body, subrs, stems, stack, depth + 1)? {
                    return Some(true);
                }
            }
            11 => return Some(false),
            14 => return Some(true),
            12 => {
                at += 1;
                stack.clear();
            }
            _ => stack.clear(),
        }
    }
    Some(false)
}
/// Private DICT ranges of a CFF font and the one each glyph uses: CID-keyed
/// fonts pick one per glyph through FDSelect, others have a single one.
fn private_dicts(cff: &[u8], top: &[u8], glyphs: usize) -> Option<(Vec<Fd>, Vec<usize>)> {
    let private = |dict: &[u8]| -> Option<std::ops::Range<usize>> {
        let operands = dict_operands(dict, 18)?;
        let (size, offset) = (usize::try_from(*operands.first()?).ok()?, usize::try_from(*operands.get(1)?).ok()?);
        cff.get(offset..offset.checked_add(size)?)?;
        Some(offset..offset + size)
    };
    let (Some(fd_array), Some(fd_select)) = (dict_offset(top, 0x0c24), dict_offset(top, 0x0c25)) else {
        return Some((vec![Fd { private: private(top)?, dict: None }], vec![0; glyphs]));
    };
    let fds = cff_index(cff, fd_array)?;
    let privates = fds.windows(2).map(|w| Some(Fd { private: private(&cff[w[0]..w[1]])?, dict: Some(w[0]..w[1]) })).collect::<Option<Vec<_>>>()?;
    let select = match cff.get(fd_select)? {
        0 => cff.get(fd_select + 1..fd_select + 1 + glyphs)?.iter().map(|&fd| fd as usize).collect(),
        3 => {
            let mut select = vec![0; glyphs];
            for r in 0..be(cff, fd_select + 1, 2)? {
                let range = fd_select + 3 + 3 * r;
                let (first, fd, last) = (be(cff, range, 2)?, *cff.get(range + 2)? as usize, be(cff, range + 3, 2)?);
                select.get_mut(first..last.min(glyphs))?.fill(fd);
            }
            select
        }
        _ => return None,
    };
    select.iter().all(|&fd| fd < privates.len()).then_some((privates, select))
}
/// A Private DICT and, in a CID-keyed font, the Font DICT that points to it.
struct Fd {
    private: std::ops::Range<usize>,
    dict: Option<std::ops::Range<usize>>,
}
impl Fd {
    fn local_subrs(&self, cff: &[u8]) -> Option<Subrs> {
        Subrs::read(cff, dict_offset(&cff[self.private.clone()], 19).map(|subrs| self.private.start + subrs))
    }
}
/// CFF2 (variable CFF) with every unused charstring emptied, which CFF2
/// allows since it has no `endchar`. Subroutines stay: a used glyph's hint
/// count, which `hintmask` parsing needs, depends on `blend` operands.
// shortcut: CFF2 subroutines are kept whole; prune them like subset_cff if a
// large variable face leaves big subsets.
fn subset_cff2(cff: &[u8], used: &[bool]) -> Option<Vec<u8>> {
    if *cff.first()? != 2 {
        return None;
    }
    let top_at = *cff.get(2)? as usize;
    let top = cff.get(top_at..top_at + be(cff, 3, 2)?)?;
    let keep = (0..used.len()).map(|g| g == 0 || used[g]).collect::<Vec<_>>();
    let mut out = cff.to_vec();
    blank_index(cff, &mut out, dict_offset(top, 17)?, &keep, &[], 4)?;
    Some(out)
}
/// CFF with every unused charstring replaced by `endchar` and every
/// subroutine no kept charstring calls replaced by `return`.
// shortcut: Type 2 `endchar` used as seac is not followed; handle it if a CFF
// font shows missing accents.
pub fn subset_cff(cff: &[u8], used: &[bool]) -> Option<Vec<u8>> {
    if *cff.first()? != 1 {
        return None;
    }
    let names = cff_index(cff, *cff.get(2)? as usize)?;
    let tops = cff_index(cff, *names.last()?)?;
    let top = cff.get(*tops.first()?..*tops.get(1)?)?;
    let strings = cff_index(cff, *tops.last()?)?;
    let charstrings = dict_offset(top, 17)?;
    let starts = cff_index(cff, charstrings)?;
    let keep = (0..used.len()).map(|g| g == 0 || used[g]).collect::<Vec<_>>();
    let mut global = Subrs::read(cff, Some(*strings.last()?))?;
    let (privates, select) = private_dicts(cff, top, used.len())?;
    let mut locals = privates.iter().map(|fd| fd.local_subrs(cff)).collect::<Option<Vec<_>>>()?;
    for g in (0..used.len()).filter(|&g| keep[g]) {
        let fd = *select.get(g)?;
        let local = std::mem::replace(locals.get_mut(fd)?, Subrs::read(cff, None)?);
        let mut subrs = [local, global];
        walk(cff, starts[g]..starts[g + 1], &mut subrs, &mut 0, &mut Vec::new(), 0)?;
        let [local, kept_global] = subrs;
        locals[fd] = local;
        global = kept_global;
    }
    let mut out = cff.to_vec();
    blank_index(cff, &mut out, charstrings, &keep, &[14], 2)?;
    for subrs in locals.iter().chain([&global]).filter(|s| !s.used.is_empty()) {
        blank_index(cff, &mut out, subrs.at, &subrs.used, &[11], 2)?;
    }
    Some(out)
}

/// Operator and full bytes (operands included) of each CFF DICT entry.
fn dict_entries(dict: &[u8]) -> Option<Vec<(u16, &[u8])>> {
    let (mut entries, mut start, mut at) = (Vec::new(), 0, 0);
    while at < dict.len() {
        at += match dict[at] {
            12 => {
                entries.push((0x0c00 | *dict.get(at + 1)? as u16, &dict[start..at + 2]));
                start = at + 2;
                2
            }
            op @ 0..=21 => {
                entries.push((op as u16, &dict[start..at + 1]));
                start = at + 1;
                1
            }
            28 => 3,
            29 => 5,
            30 => 2 + dict[at + 1..].iter().position(|&v| v & 0x0f == 0x0f || v >> 4 == 0x0f)?,
            32..=246 => 1,
            247..=254 => 2,
            _ => return None,
        };
    }
    (start == dict.len()).then_some(entries)
}
/// DICT integer in its fixed five-byte form, so offsets can be filled in later.
fn int5(value: usize) -> [u8; 5] {
    let [a, b, c, d] = (value as i32).to_be_bytes();
    [29, a, b, c, d]
}
fn write_index(objects: &[Vec<u8>]) -> Vec<u8> {
    if objects.is_empty() {
        return vec![0, 0];
    }
    let end = 1 + objects.iter().map(Vec::len).sum::<usize>();
    let size = (1..4).find(|&s| end < 1 << (8 * s)).unwrap_or(4);
    let mut out = (objects.len() as u16).to_be_bytes().to_vec();
    out.push(size as u8);
    let mut offset = 1usize;
    for o in objects {
        out.extend(&offset.to_be_bytes()[8 - size..]);
        offset += o.len();
    }
    out.extend(&offset.to_be_bytes()[8 - size..]);
    objects.iter().for_each(|o| out.extend(o));
    out
}
/// A charstring being copied without subroutine calls. The operand count and
/// stem count carry across calls: a subroutine works on its caller's stack.
#[derive(Default)]
struct Flat {
    out: Vec<u8>,
    operands: usize,
    stems: usize,
    /// Output position and value of the number just copied.
    last: Option<(usize, i64)>,
}
/// Copies a Type 2 charstring with each subroutine call replaced by the
/// subroutine's body. Returns whether `endchar` ended the glyph. `None` for
/// what such a copy cannot keep exact: a subroutine number computed by
/// arithmetic, and `endchar` used as seac, whose accent glyphs a subset drops.
fn flatten(cff: &[u8], code: std::ops::Range<usize>, subrs: [&Subrs; 2], f: &mut Flat, depth: u8) -> Option<bool> {
    let mut at = code.start;
    while at < code.end {
        let b = cff[at];
        let width = match b {
            28 => 3,
            32..=246 => 1,
            247..=254 => 2,
            255 => 5,
            _ => 0,
        };
        if width > 0 {
            let token = cff.get(at..at + width)?;
            let value = match b {
                28 => Some(be(token, 1, 2)? as i16 as i64),
                32..=246 => Some(b as i64 - 139),
                247..=250 => Some((b as i64 - 247) * 256 + token[1] as i64 + 108),
                251..=254 => Some(-(b as i64 - 251) * 256 - token[1] as i64 - 108),
                _ => None,
            };
            f.last = value.map(|v| (f.out.len(), v));
            f.out.extend(token);
            f.operands += 1;
            at += width;
            continue;
        }
        at += 1;
        let mut copy = cff.get(at - 1..at)?;
        match b {
            10 | 29 => {
                let (start, number) = f.last.take()?;
                f.out.truncate(start);
                f.operands -= 1;
                let set = subrs[(b == 29) as usize];
                let index = usize::try_from(number + set.bias()).ok()?;
                let body = *set.starts.get(index)?..*set.starts.get(index + 1)?;
                if depth >= 10 {
                    return None;
                }
                if flatten(cff, body, subrs, f, depth + 1)? {
                    return Some(true);
                }
                continue;
            }
            11 => return (depth > 0).then_some(false),
            14 if f.operands >= 4 => return None,
            14 => {
                f.out.push(14);
                return Some(true);
            }
            1 | 3 | 18 | 23 => f.stems += f.operands / 2,
            19 | 20 => {
                f.stems += f.operands / 2;
                copy = cff.get(at - 1..at + f.stems.div_ceil(8))?;
            }
            // dotsection and the flex operators; the others are arithmetic.
            12 if [0, 34, 35, 36, 37].contains(cff.get(at)?) => copy = &cff[at - 1..at + 1],
            12 => return None,
            _ => {}
        }
        at += copy.len() - 1;
        f.out.extend(copy);
        f.operands = 0;
        f.last = None;
    }
    Some(false)
}
/// Bare CID-keyed CFF for FontFile3/CIDFontType0C with .notdef and the used
/// glyphs only, their charstrings flattened so no subroutines remain, as
/// xdvipdfmx writes OpenType CFF faces. Each glyph's CID is its glyph id in
/// the face, so content-stream codes, widths and ToUnicode stay as written.
pub fn bare_cff(font: &[u8], used: &[bool]) -> Option<Vec<u8>> {
    let cff = (0..be(font, 4, 2)?).find_map(|i| {
        let record = 12 + 16 * i;
        let offset = be(font, record + 8, 4)?;
        (font.get(record..record + 4)? == b"CFF ").then(|| font.get(offset..offset.checked_add(be(font, record + 12, 4)?)?))?
    })?;
    if *cff.first()? != 1 {
        return None;
    }
    let names_at = *cff.get(2)? as usize;
    let names = cff_index(cff, names_at)?;
    let tops = cff_index(cff, *names.last()?)?;
    let top = cff.get(*tops.first()?..*tops.get(1)?)?;
    let strings = cff_index(cff, *tops.last()?)?;
    let starts = cff_index(cff, dict_offset(top, 17)?)?;
    if starts.len() != used.len() + 1 || dict_operands(top, 0x0c06).is_some_and(|t| t != [2]) {
        return None;
    }
    let global = Subrs::read(cff, Some(*strings.last()?))?;
    let (fds, select) = private_dicts(cff, top, used.len())?;
    if fds.len() > 256 {
        return None;
    }
    let locals = fds.iter().map(|fd| fd.local_subrs(cff)).collect::<Option<Vec<_>>>()?;
    let glyphs = (0..used.len()).filter(|&g| g == 0 || used[g]).collect::<Vec<_>>();
    let charstrings = glyphs
        .iter()
        .map(|&g| {
            let mut flat = Flat::default();
            flatten(cff, starts[g]..starts[g + 1], [&locals[select[g]], &global], &mut flat, 0)?;
            Some(flat.out)
        })
        .collect::<Option<Vec<_>>>()?;
    // Glyph names fill most of a name-keyed font's String INDEX; keep only the
    // strings DICT entries name, renumbered (391 standard strings come first).
    let mut kept_strings = vec![b"Adobe".to_vec(), b"Identity".to_vec()];
    let mut entries = |dict: &[u8], dropped: &[u16]| -> Option<Vec<u8>> {
        let mut out = Vec::new();
        for (op, bytes) in dict_entries(dict)?.into_iter().filter(|e| !dropped.contains(&e.0)) {
            let sid = [0, 1, 2, 3, 4, 0x0c00, 0x0c15, 0x0c16, 0x0c26].contains(&op).then(|| dict_operands(bytes, op)).flatten();
            match sid.as_deref() {
                Some(&[sid]) if sid >= 391 => {
                    let at = usize::try_from(sid - 391).ok()?;
                    kept_strings.push(cff.get(*strings.get(at)?..*strings.get(at + 1)?)?.to_vec());
                    out.extend(int5(390 + kept_strings.len()));
                    out.extend(&bytes[bytes.len() - if op > 0xff { 2 } else { 1 }..]);
                }
                _ => out.extend(bytes),
            }
        }
        Some(out)
    };
    let copied = entries(top, &[15, 16, 17, 18, 0x0c14, 0x0c1e, 0x0c22, 0x0c24, 0x0c25])?;
    let fd_entries = fds
        .iter()
        .map(|fd| fd.dict.as_ref().map_or(Some(Vec::new()), |dict| entries(&cff[dict.clone()], &[18])))
        .collect::<Option<Vec<Vec<u8>>>>()?;
    let privates = fds.iter().map(|fd| entries(&cff[fd.private.clone()], &[19])).collect::<Option<Vec<Vec<u8>>>>()?;
    let strings = write_index(&kept_strings);
    // Format 0 charset (GID 1.. -> CID) and FDSelect (one FD byte per GID).
    let charset = std::iter::once(0).chain(glyphs[1..].iter().flat_map(|&g| (g as u16).to_be_bytes())).collect::<Vec<_>>();
    let fd_select = std::iter::once(0).chain(glyphs.iter().map(|&g| select[g] as u8)).collect::<Vec<_>>();
    let charstrings = write_index(&charstrings);
    // Offsets are five-byte integers, so a first pass with zeros fixes every length.
    let layout = |at: [usize; 4], private_at: &[usize]| {
        let mut top = [int5(391), int5(392), int5(0)].concat();
        top.extend([12, 30]);
        top.extend(copied.iter());
        for (value, op) in [(used.len(), &[12, 34][..]), (at[0], &[15]), (at[1], &[12, 37]), (at[2], &[17]), (at[3], &[12, 36])] {
            top.extend(int5(value));
            top.extend(op);
        }
        let fd_dicts = fd_entries
            .iter()
            .zip(privates.iter().zip(private_at))
            .map(|(entries, (private, &at))| [entries.as_slice(), &int5(private.len()), &int5(at), &[18]].concat())
            .collect::<Vec<_>>();
        (write_index(&[top]), write_index(&fd_dicts))
    };
    let head = [&[1, 0, 4, 4][..], &cff[names_at..*names.last()?]].concat();
    let (top_index, fd_array) = layout([0; 4], &vec![0; fds.len()]);
    let charset_at = head.len() + top_index.len() + strings.len() + 2;
    let fd_select_at = charset_at + charset.len();
    let charstrings_at = fd_select_at + fd_select.len();
    let fd_array_at = charstrings_at + charstrings.len();
    let mut private_at = Vec::new();
    let mut next = fd_array_at + fd_array.len();
    for private in &privates {
        private_at.push(next);
        next += private.len();
    }
    let (top_index, fd_array) = layout([charset_at, fd_select_at, charstrings_at, fd_array_at], &private_at);
    let out = [head, top_index, strings, vec![0, 0], charset, fd_select, charstrings, fd_array, privates.concat()].concat();
    (out.len() == next).then_some(out)
}

const EEXEC: u16 = 55665;
const CHARSTRING: u16 = 4330;
fn decrypt(data: &[u8], mut r: u16) -> Vec<u8> {
    data.iter()
        .map(|&c| {
            let plain = c ^ (r >> 8) as u8;
            r = (c as u16).wrapping_add(r).wrapping_mul(52845).wrapping_add(22719);
            plain
        })
        .collect()
}
fn encrypt(data: &[u8], mut r: u16) -> Vec<u8> {
    data.iter()
        .map(|&p| {
            let c = p ^ (r >> 8) as u8;
            r = (c as u16).wrapping_add(r).wrapping_mul(52845).wrapping_add(22719);
            c
        })
        .collect()
}
fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    hay.get(from..)?.windows(needle.len()).position(|w| w == needle).map(|i| i + from)
}
/// Next whitespace-delimited token at or after `at`: (start, end).
fn token(d: &[u8], at: usize) -> Option<(usize, usize)> {
    let start = at + d.get(at..)?.iter().position(|b| !b.is_ascii_whitespace())?;
    let end = start + d[start..].iter().position(|b| b.is_ascii_whitespace()).unwrap_or(d.len() - start);
    Some((start, end))
}

fn number(plain: &[u8], (s, e): (usize, usize)) -> Option<usize> {
    std::str::from_utf8(&plain[s..e]).ok()?.parse().ok()
}

/// The eexec section with only the named charstrings (plus `.notdef` and
/// `seac` accent components) and the Subrs they call; other Subrs become
/// `return` so indices stay valid.
pub fn subset_type1(binary: &[u8], keep: &BTreeSet<&[u8]>) -> Option<Vec<u8>> {
    let plain = decrypt(binary, EEXEC);
    let skip = find(&plain, b"/lenIV", 0)
        .and_then(|at| token(&plain, at + 6))
        .and_then(|(s, e)| std::str::from_utf8(&plain[s..e]).ok()?.parse::<i32>().ok())
        .unwrap_or(4);
    let decode = |code: &[u8]| match usize::try_from(skip) {
        Ok(skip) => decrypt(code, CHARSTRING).split_off(skip.min(code.len())),
        Err(_) => code.to_vec(),
    };
    // Subrs: "dup <i> <len> RD <bytes> NP", where NP may be "|" or "noaccess put".
    let mut subrs = Vec::new();
    let subrs_start = find(&plain, b"/Subrs", 0).and_then(|at| token(&plain, token(&plain, at + 6)?.1)).map(|t| t.1);
    let mut at = subrs_start.unwrap_or(0);
    while let Some((s, e)) = token(&plain, at).filter(|_| subrs_start.is_some()) {
        if &plain[s..e] != b"dup" {
            break;
        }
        let index = token(&plain, e)?;
        let length = token(&plain, index.1)?;
        let rd = token(&plain, length.1)?;
        let body = rd.1 + 1..rd.1 + 1 + number(&plain, length)?;
        at = body.end;
        let close_start = at;
        while let Some((s, e)) = token(&plain, at) {
            at = e;
            if matches!(&plain[s..e], b"NP" | b"|" | b"put") {
                break;
            }
        }
        plain.get(body.clone())?;
        subrs.push((number(&plain, index)?, rd, body, close_start..at));
    }
    let subrs_end = at;
    let start = find(&plain, b"begin", find(&plain, b"/CharStrings", subrs_end)?)? + 5;
    let mut entries = Vec::new();
    let mut at = start;
    loop {
        let (name_start, name_end) = token(&plain, at)?;
        if plain[name_start] != b'/' {
            break;
        }
        let length = token(&plain, name_end)?;
        let (_, rd) = token(&plain, length.1)?;
        let charstring = rd + 1..rd + 1 + number(&plain, length)?;
        at = charstring.end;
        // Skip the closing token(s) ("ND", "|-", "noaccess def") up to the next entry or "end".
        while let Some((s, e)) = token(&plain, at) {
            if plain[s] == b'/' || &plain[s..e] == b"end" {
                break;
            }
            at = e;
        }
        plain.get(charstring.clone())?;
        entries.push((&plain[name_start + 1..name_end], name_start..at, charstring));
    }
    let mut wanted = keep.iter().map(|n| n.to_vec()).collect::<BTreeSet<_>>();
    wanted.insert(b".notdef".to_vec());
    // Subrs 0-3 are the flex and hint-replacement conventions.
    let mut called = (0..4).collect::<BTreeSet<usize>>();
    let mut pending = Vec::new();
    for _ in 0..2 {
        // A second pass picks up seac components added by the first.
        for (name, _, charstring) in &entries {
            if wanted.contains(*name) {
                let (calls, accent) = references(&decode(&plain[charstring.clone()]));
                pending.extend(calls);
                for code in accent.into_iter().flatten() {
                    wanted.extend(STANDARD.get(code as usize).filter(|n| !n.is_empty()).map(|n| n.as_bytes().to_vec()));
                }
            }
        }
    }
    pending.extend(called.iter().copied());
    while let Some(index) = pending.pop() {
        called.insert(index);
        if let Some((_, _, body, _)) = subrs.iter().find(|s| s.0 == index) {
            pending.extend(references(&decode(&plain[body.clone()])).0.into_iter().filter(|i| !called.contains(i)));
        }
    }
    let mut out = plain[..subrs_start.unwrap_or(0)].to_vec();
    let empty = match usize::try_from(skip) {
        Ok(skip) => encrypt(&[vec![0; skip], vec![11]].concat(), CHARSTRING),
        Err(_) => vec![11],
    };
    for (index, rd, body, close) in &subrs {
        let body = if called.contains(index) { &plain[body.clone()] } else { &empty[..] };
        out.extend(format!("\ndup {index} {} ", body.len()).bytes());
        out.extend(&plain[rd.0..rd.1]);
        out.push(b' ');
        out.extend(body);
        out.extend(&plain[close.clone()]);
    }
    out.extend(&plain[subrs_end..start]);
    for (name, span, _) in &entries {
        if wanted.contains(*name) {
            out.push(b'\n');
            out.extend(&plain[span.clone()]);
        }
    }
    out.push(b'\n');
    out.extend(&plain[at..]);
    Some(encrypt(&out, EEXEC))
}
/// Subrs a Type 1 charstring calls and the standard-encoding codes of a
/// `seac` accented character's base and accent.
fn references(code: &[u8]) -> (Vec<usize>, Option<[u8; 2]>) {
    let (mut stack, mut calls, mut at) = (Vec::new(), Vec::new(), 0);
    while at < code.len() {
        let v = code[at] as i32;
        let w = code.get(at + 1).map_or(0, |&b| b as i32);
        at += 1;
        match v {
            32..=246 => stack.push(v - 139),
            247..=250 => {
                stack.push((v - 247) * 256 + w + 108);
                at += 1;
            }
            251..=254 => {
                stack.push(-(v - 251) * 256 - w - 108);
                at += 1;
            }
            255 => {
                stack.push(be(code, at, 4).unwrap_or(0) as u32 as i32);
                at += 4;
            }
            // callsubr leaves the rest of the stack to the subroutine.
            10 => calls.extend(stack.pop().and_then(|i| usize::try_from(i).ok())),
            11 => {}
            12 => {
                at += 1;
                match w {
                    6 if stack.len() >= 5 => {
                        let n = stack.len();
                        return (calls, Some([stack[n - 2] as u8, stack[n - 1] as u8]));
                    }
                    // callothersubr consumes its number and argument count; "pop" then
                    // returns the arguments, as hint replacement's "subr# 1 3 callothersubr pop callsubr".
                    16 => stack.truncate(stack.len().saturating_sub(2)),
                    17 => {}
                    _ => stack.clear(),
                }
            }
            _ => stack.clear(),
        }
    }
    (calls, None)
}
/// Built-in encoding of a Type 1 program's cleartext part.
pub fn builtin_encoding(clear: &[u8]) -> Vec<Option<String>> {
    let text = String::from_utf8_lossy(clear);
    let mut names = vec![None; 256];
    let words = text.split_ascii_whitespace().collect::<Vec<_>>();
    for row in words.windows(4) {
        if let ("dup", Ok(code), Some(name), "put") = (row[0], row[1].parse::<usize>(), row[2].strip_prefix('/'), row[3]) {
            if code < 256 {
                names[code] = Some(name.to_string());
            }
        }
    }
    if names.iter().all(Option::is_none) && text.contains("StandardEncoding") {
        for (code, name) in STANDARD.iter().enumerate().filter(|(_, n)| !n.is_empty()) {
            names[code] = Some(name.to_string());
        }
    }
    names
}
/// Adobe StandardEncoding (PostScript Language Reference, E.6).
const STANDARD: [&str; 256] = {
    let mut t = [""; 256];
    let ascii = [
        "space", "exclam", "quotedbl", "numbersign", "dollar", "percent", "ampersand", "quoteright",
        "parenleft", "parenright", "asterisk", "plus", "comma", "hyphen", "period", "slash", "zero",
        "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "colon", "semicolon",
        "less", "equal", "greater", "question", "at", "A", "B", "C", "D", "E", "F", "G", "H", "I",
        "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
        "bracketleft", "backslash", "bracketright", "asciicircum", "underscore", "quoteleft", "a",
        "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s",
        "t", "u", "v", "w", "x", "y", "z", "braceleft", "bar", "braceright", "asciitilde",
    ];
    let mut i = 0;
    while i < ascii.len() {
        t[32 + i] = ascii[i];
        i += 1;
    }
    let high: &[(usize, &str)] = &[
        (161, "exclamdown"), (162, "cent"), (163, "sterling"), (164, "fraction"), (165, "yen"),
        (166, "florin"), (167, "section"), (168, "currency"), (169, "quotesingle"),
        (170, "quotedblleft"), (171, "guillemotleft"), (172, "guilsinglleft"),
        (173, "guilsinglright"), (174, "fi"), (175, "fl"), (177, "endash"), (178, "dagger"),
        (179, "daggerdbl"), (180, "periodcentered"), (182, "paragraph"), (183, "bullet"),
        (184, "quotesinglbase"), (185, "quotedblbase"), (186, "quotedblright"),
        (187, "guillemotright"), (188, "ellipsis"), (189, "perthousand"), (191, "questiondown"),
        (193, "grave"), (194, "acute"), (195, "circumflex"), (196, "tilde"), (197, "macron"),
        (198, "breve"), (199, "dotaccent"), (200, "dieresis"), (202, "ring"), (203, "cedilla"),
        (205, "hungarumlaut"), (206, "ogonek"), (207, "caron"), (208, "emdash"), (225, "AE"),
        (227, "ordfeminine"), (232, "Lslash"), (233, "Oslash"), (234, "OE"),
        (235, "ordmasculine"), (241, "ae"), (245, "dotlessi"), (248, "lslash"), (249, "oslash"),
        (250, "oe"), (251, "germandbls"),
    ];
    let mut j = 0;
    while j < high.len() {
        t[high[j].0] = high[j].1;
        j += 1;
    }
    t
};

#[cfg(test)]
mod tests {
    use super::*;

    fn type1_charstring(code: &[u8]) -> Vec<u8> {
        encrypt(&[&[0u8; 4][..], code].concat(), CHARSTRING)
    }
    fn type1_private(subrs: &[Vec<u8>], glyphs: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut plain = b"\0\0\0\0dup /Private 8 dict dup begin /RD{string currentfile exch readstring pop}executeonly def\n".to_vec();
        plain.extend(format!("/Subrs {} array\n", subrs.len()).bytes());
        for (i, code) in subrs.iter().enumerate() {
            let cs = type1_charstring(code);
            plain.extend(format!("dup {i} {} RD ", cs.len()).bytes());
            plain.extend(&cs);
            plain.extend(b" NP\n");
        }
        plain.extend(b"ND\n2 index /CharStrings 4 dict dup begin\n");
        for (name, code) in glyphs {
            let cs = type1_charstring(code);
            plain.extend(format!("/{name} {} RD ", cs.len()).bytes());
            plain.extend(&cs);
            plain.extend(b" ND\n");
        }
        plain.extend(b"end\nend\nreadonly put\nmark currentfile closefile\n");
        encrypt(&plain, EEXEC)
    }
    fn subr_lengths(binary: &[u8]) -> Vec<usize> {
        let plain = decrypt(binary, EEXEC);
        let text = String::from_utf8_lossy(&plain);
        let words = text.split_ascii_whitespace().collect::<Vec<_>>();
        words.windows(4).filter(|w| w[0] == "dup" && w[3] == "RD").map(|w| w[2].parse().unwrap()).collect()
    }
    fn charstring_names(binary: &[u8]) -> Vec<String> {
        let plain = decrypt(binary, EEXEC);
        let text = String::from_utf8_lossy(&plain);
        let at = text.find("/CharStrings").unwrap();
        text[at + 1..]
            .split_ascii_whitespace()
            .filter_map(|w| w.strip_prefix('/'))
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn type1_subset_keeps_used_glyphs_notdef_and_seac_parts() {
        // Aacute = seac(asb 0, adx 0, ady 0, bchar A = 65, achar acute = 194).
        let aacute = vec![139, 139, 139, 204, 247, 86, 12, 6];
        let binary = type1_private(&[vec![11]], &[
            (".notdef", vec![139, 139, 13, 14]),
            ("A", vec![139, 139, 13, 14]),
            ("B", vec![139, 139, 13, 14]),
            ("acute", vec![139, 139, 13, 14]),
            ("Aacute", aacute),
        ]);
        let keep = [b"Aacute".as_slice()].into_iter().collect();
        let subset = subset_type1(&binary, &keep).unwrap();
        assert_eq!(charstring_names(&subset), [".notdef", "A", "acute", "Aacute"]);
        let plain = decrypt(&subset, EEXEC);
        assert!(plain.ends_with(b"end\nend\nreadonly put\nmark currentfile closefile\n"));
        assert!(subset.len() < binary.len());
    }

    #[test]
    fn type1_subset_keeps_called_subrs_and_empties_the_rest() {
        let filler = |n: usize| [vec![139; n], vec![11]].concat();
        // Subrs 0-3 are always kept; "A" calls 4 (143 = 4), which calls 5 through hint replacement
        // ("5 1 3 callothersubr pop callsubr"); 6 is unused.
        let subrs = [filler(3), filler(3), filler(3), filler(3), vec![144, 140, 142, 12, 16, 12, 17, 10, 11], filler(9), filler(30)];
        let binary = type1_private(&subrs, &[(".notdef", vec![139, 139, 13, 14]), ("A", vec![143, 10, 14])]);
        let keep = [b"A".as_slice()].into_iter().collect();
        let subset = subset_type1(&binary, &keep).unwrap();
        assert_eq!(subr_lengths(&binary), [8, 8, 8, 8, 13, 14, 35]);
        assert_eq!(subr_lengths(&subset), [8, 8, 8, 8, 13, 14, 5]);
        assert_eq!(charstring_names(&subset), [".notdef", "A"]);
    }

    #[test]
    fn builtin_encoding_reads_arrays_and_standard_encoding() {
        let custom = builtin_encoding(b"/Encoding 256 array\n0 1 255 {1 index exch /.notdef put} for\ndup 65 /A put\ndup 0 /Gamma put\nreadonly def");
        assert_eq!(custom[65].as_deref(), Some("A"));
        assert_eq!(custom[0].as_deref(), Some("Gamma"));
        let standard = builtin_encoding(b"/Encoding StandardEncoding def");
        assert_eq!(standard[39].as_deref(), Some("quoteright"));
        assert_eq!(standard[194].as_deref(), Some("acute"));
        assert_eq!(standard[1], None);
    }

    fn index(objects: &[&[u8]]) -> Vec<u8> {
        if objects.is_empty() {
            return vec![0, 0];
        }
        let mut out = (objects.len() as u16).to_be_bytes().to_vec();
        out.push(2);
        let mut offset = 1u16;
        out.extend(offset.to_be_bytes());
        for o in objects {
            offset += o.len() as u16;
            out.extend(offset.to_be_bytes());
        }
        objects.iter().for_each(|o| out.extend(*o));
        out
    }

    #[test]
    fn cff_subset_blanks_unused_charstrings_and_subroutines() {
        // Glyph 2 calls global subroutine 0 (operand -107 plus bias 107); subroutine 1 is unused.
        let glyphs: [&[u8]; 3] = [&[139, 14], &[139, 139, 21, 14], &[32, 29, 139, 139, 8, 14]];
        let gsubrs = index(&[&[139, 139, 21, 11], &[139; 11]]);
        let head = [1u8, 0, 4, 1];
        let names = index(&[b"F"]);
        // Top DICT: CharStrings offset as a 5-byte integer (29), operator 17, then an empty Private (0 0 18).
        let top_len = 2 + 1 + 2 * 2 + 9;
        let gsubrs_at = head.len() + names.len() + top_len + 2;
        let charstrings_at = gsubrs_at + gsubrs.len();
        let mut top = vec![29];
        top.extend((charstrings_at as i32).to_be_bytes());
        top.extend([17, 139, 139, 18]);
        let mut cff = [&head[..], &names, &index(&[&top]), &index(&[]), &gsubrs].concat();
        assert_eq!(cff.len(), charstrings_at);
        cff.extend(index(&glyphs));
        let subset = subset_cff(&cff, &[false, false, true]).unwrap();
        assert_eq!(subset.len(), cff.len());
        let objects = |at: usize| {
            let starts = cff_index(&subset, at).unwrap();
            starts.windows(2).map(|w| subset[w[0]..w[1]].to_vec()).collect::<Vec<_>>()
        };
        assert_eq!(objects(charstrings_at), [glyphs[0], &[14], glyphs[2]]);
        assert_eq!(objects(gsubrs_at), [&[139, 139, 21, 11][..], &[11]]);
        assert!(subset[cff_index(&subset, charstrings_at).unwrap()[3]..].iter().all(|&b| b == 0));
    }

    /// Name-keyed CFF with a global and a local subroutine, wrapped in an sfnt.
    /// FullName is string 392; string 391 stands in for the glyph names.
    fn cff_face(glyphs: &[&[u8]]) -> Vec<u8> {
        let head = [1u8, 0, 4, 1];
        let names = index(&[b"F"]);
        let strings = index(&[b"glyphname", b"Full"]);
        let gsubrs = index(&[&[139, 139, 21, 11]]);
        let lsubrs = index(&[&[139, 139, 5, 11]]);
        let charstrings = index(glyphs);
        // Top DICT: FullName (2), CharStrings (17) and Private (18) as five-byte integers.
        let top = |charstrings_at: usize, private_at: usize| {
            [&int5(392)[..], &[2], &int5(charstrings_at), &[17], &int5(6), &int5(private_at), &[18]].concat()
        };
        let charstrings_at = head.len() + names.len() + index(&[&top(0, 0)]).len() + strings.len() + gsubrs.len();
        let private_at = charstrings_at + charstrings.len();
        let private = [&int5(6)[..], &[19]].concat();
        let top = top(charstrings_at, private_at);
        let cff = [&head[..], &names, &index(&[&top]), &strings, &gsubrs, &charstrings, &private, &lsubrs].concat();
        assert_eq!(cff_index(&cff, private_at + private.len()).unwrap().len(), 2);
        sfnt(&[(b"CFF ", cff), (b"head", vec![0; 54])])
    }
    fn objects(cff: &[u8], at: usize) -> Vec<Vec<u8>> {
        cff_index(cff, at).unwrap().windows(2).map(|w| cff[w[0]..w[1]].to_vec()).collect()
    }

    #[test]
    fn bare_cff_keeps_used_glyphs_as_cids_with_subroutines_inlined() {
        // Glyph 2: hstemhm 0 10, hintmask (one byte), callgsubr 0, callsubr 0, endchar.
        let glyph = [139, 149, 18, 19, 0x80, 32, 29, 32, 10, 14];
        let face = cff_face(&[&[139, 14], &[139, 139, 21, 14], &glyph]);
        let bare = bare_cff(&face, &[false, false, true]).unwrap();
        let names = cff_index(&bare, 4).unwrap();
        let top = &objects(&bare, *names.last().unwrap())[0];
        assert_eq!(dict_operands(top, 0x0c1e).unwrap(), [391, 392, 0]);
        assert_eq!(dict_operands(top, 0x0c22).unwrap(), [3]);
        let charstrings = objects(&bare, dict_offset(top, 17).unwrap());
        assert_eq!(charstrings, [vec![139, 14], vec![139, 149, 18, 19, 0x80, 139, 139, 21, 139, 139, 5, 14]]);
        let charset = dict_offset(top, 15).unwrap();
        assert_eq!(bare[charset..charset + 3], [0, 0, 2]);
        let fd_select = dict_offset(top, 0x0c25).unwrap();
        assert_eq!(bare[fd_select..fd_select + 3], [0, 0, 0]);
        let fd = &objects(&bare, dict_offset(top, 0x0c24).unwrap())[0];
        let private = dict_operands(fd, 18).unwrap();
        let private = &bare[private[1] as usize..(private[0] + private[1]) as usize];
        assert!(dict_entries(private).unwrap().iter().all(|e| e.0 != 19));
        assert_eq!(dict_operands(top, 2).unwrap(), [393]);
        let strings_at = cff_index(&bare, *names.last().unwrap()).unwrap()[1];
        assert_eq!(objects(&bare, strings_at), [b"Adobe".to_vec(), b"Identity".to_vec(), b"Full".to_vec()]);
        assert_eq!(bare[*cff_index(&bare, strings_at).unwrap().last().unwrap()..][..2], [0, 0]);
    }

    #[test]
    fn bare_cff_refuses_seac_and_computed_subroutine_numbers() {
        // endchar with four operands is seac; "1 1 add callsubr" computes the number.
        assert!(bare_cff(&cff_face(&[&[139, 14], &[139, 139, 140, 140, 14]]), &[false, true]).is_none());
        assert!(bare_cff(&cff_face(&[&[139, 14], &[140, 140, 12, 10, 10, 14]]), &[false, true]).is_none());
        assert!(bare_cff(&cff_face(&[&[139, 14], &[139, 139, 21, 14]]), &[false, true]).is_some());
    }

    #[test]
    fn cff2_subset_empties_unused_charstrings_inside_the_sfnt() {
        // CFF2: header (5 bytes), Top DICT with CharStrings, empty Global Subrs, then
        // a CharStrings INDEX with a four-byte count. CFF2 charstrings have no endchar.
        let glyphs: [&[u8]; 3] = [&[139, 139, 21], &[140, 140, 21], &[141, 141, 21, 139, 139, 5]];
        let charstrings_at = 5 + 6 + 4;
        let top = [&int5(charstrings_at)[..], &[17]].concat();
        let mut cff2 = vec![2, 0, 5, 0, top.len() as u8];
        cff2.extend(&top);
        cff2.extend([0; 4]);
        cff2.extend([0, 0, 0, 3, 1, 1]);
        let mut offset = 1;
        for g in glyphs {
            offset += g.len() as u8;
            cff2.push(offset);
        }
        glyphs.iter().for_each(|g| cff2.extend(*g));
        let font = sfnt(&[(b"CFF2", cff2.clone()), (b"GSUB", vec![9; 30]), (b"fvar", vec![1; 16]), (b"head", vec![0; 54])]);
        let subset = subset_sfnt(&font, &[false, false, true]).unwrap();
        let tags = (0..be(&subset, 4, 2).unwrap()).map(|i| subset[12 + 16 * i..16 + 16 * i].to_vec()).collect::<Vec<_>>();
        assert_eq!(tags, [b"CFF2".to_vec(), b"fvar".to_vec(), b"head".to_vec()]);
        let (at, len) = (be(&subset, 20, 4).unwrap(), be(&subset, 24, 4).unwrap());
        let table = &subset[at..at + len];
        assert_eq!(table.len(), cff2.len());
        let starts = sized_index(table, charstrings_at, 4).unwrap();
        let kept = starts.windows(2).map(|w| table[w[0]..w[1]].to_vec()).collect::<Vec<_>>();
        assert_eq!(kept, [glyphs[0], &[], glyphs[2]]);
        assert!(table[starts[3]..].iter().all(|&b| b == 0));
    }

    fn sfnt(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut out = vec![0, 1, 0, 0];
        out.extend((tables.len() as u16).to_be_bytes());
        out.extend([0; 6]);
        let mut offset = 12 + 16 * tables.len();
        for (tag, data) in tables {
            out.extend(*tag);
            out.extend([0; 4]);
            out.extend((offset as u32).to_be_bytes());
            out.extend((data.len() as u32).to_be_bytes());
            offset += data.len();
        }
        tables.iter().for_each(|(_, d)| out.extend(d));
        out
    }

    #[test]
    fn truetype_subset_keeps_composite_components_and_drops_layout_tables() {
        let simple = |fill: u8| {
            let mut g = vec![0, 1];
            g.extend([fill; 10]);
            g
        };
        // Glyph 3 is a composite of glyph 1 (flags: words args, no more components).
        let mut composite = vec![0xff, 0xff];
        composite.extend([0; 8]);
        composite.extend([0, 1, 0, 1, 0, 0, 0, 0]);
        let glyphs = [simple(1), simple(2), simple(3), composite];
        let mut glyf = Vec::new();
        let mut loca = Vec::new();
        for g in &glyphs {
            loca.extend(((glyf.len() / 2) as u16).to_be_bytes());
            glyf.extend(g);
        }
        loca.extend(((glyf.len() / 2) as u16).to_be_bytes());
        let head = vec![0; 54];
        let font = sfnt(&[(b"GPOS", vec![7; 40]), (b"glyf", glyf), (b"head", head), (b"loca", loca)]);
        let subset = subset_sfnt(&font, &[false, false, false, true]).unwrap();
        let tags = (0..be(&subset, 4, 2).unwrap())
            .map(|i| subset[12 + 16 * i..16 + 16 * i].to_vec())
            .collect::<Vec<_>>();
        assert_eq!(tags, [b"glyf".to_vec(), b"head".to_vec(), b"loca".to_vec()]);
        let record = |i: usize| (be(&subset, 12 + 16 * i + 8, 4).unwrap(), be(&subset, 12 + 16 * i + 12, 4).unwrap());
        let (glyf_at, glyf_len) = record(0);
        let (loca_at, _) = record(2);
        let lengths = (0..4)
            .map(|g| 2 * (be(&subset, loca_at + 2 * g + 2, 2).unwrap() - be(&subset, loca_at + 2 * g, 2).unwrap()))
            .collect::<Vec<_>>();
        assert_eq!(lengths, [12, 12, 0, 18]);
        assert_eq!(glyf_len, 42);
        assert_eq!(&subset[glyf_at + 12..glyf_at + 24], &glyphs[1][..]);
    }
}
