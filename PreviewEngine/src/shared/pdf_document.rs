#[path = "pdf_threads.rs"]
mod pdf_threads;

// Pitex-authored PDF 1.7 document features. No external converter is used.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// PDF syntax and object relationships follow ISO 32000 and the documented
// dvipdfmx special interface; this implementation contains no converter code.
use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Document {
    pub backend: backend_options::Options,
    pub inclusion:pdf_inclusion::Policy,
    pub form_boxes: BTreeMap<i32,[f64;4]>,
    pub postscript_resources: BTreeMap<String,i32>,
    pub origin: Option<[f64;2]>,
    form_resources: BTreeMap<i32,usize>,
    threads:pdf_threads::Threads,
    pub catalog_object: i32,
    pub pages_object: i32,
    pub info_object: i32,
    pub names_object: i32,
    pub catalog: BTreeMap<String, String>,
    pub info: BTreeMap<String, String>,
    pub pages: Vec<i32>,
    logical_pages: BTreeMap<i32,usize>,
    image_attributes: BTreeMap<usize, BTreeMap<String,String>>,
    color_stacks: Vec<PitexColorStack>,
    version_request: Option<(i32,i32)>,
    page_dicts: BTreeMap<i32, BTreeMap<String, String>>,
    annotations: BTreeMap<i32, Vec<i32>>,
    annotation_arrays: BTreeMap<i32, i32>,
    destinations: BTreeMap<Vec<u8>, (String, String)>,
    names: BTreeMap<String, String>,
    name_trees: BTreeMap<String, BTreeMap<Vec<u8>, (String, String)>>,
    ctm: [f64; 6],
    graphics_stack: Vec<[f64; 6]>,
    outlines: Vec<Outline>,
    links: Vec<Link>,
}
#[derive(Clone)]
struct PitexColorStack { initial: String, values: Vec<String>, page: bool, direct: bool }
impl Default for PitexColorStack {
    fn default() -> Self { Self { initial:"0 g 0 G".into(),values:Vec::new(),page:false,direct:false } }
}
struct Outline {
    object: i32,
    level: i32,
    open: bool,
    dict: BTreeMap<String, String>,
}
struct Link {
    start_box:Option<usize>,
    level:Option<i32>,
    first_object:Option<i32>,
    fixed_width:Option<f64>,
    fixed_height:Option<f64>,
    fixed_depth:Option<f64>,
    matrix:[f64;6],
    dict: BTreeMap<String, String>,
    boxes: Vec<(i32, [f64; 4], f64)>,
}

unsafe fn text(s: *const ::core::ffi::c_char, n: usize) -> String {
    String::from_utf8_lossy(std::slice::from_raw_parts(s.cast::<u8>(), n)).into_owned()
}
unsafe fn append(out: *mut pbuf, value: &str) {
    pbuf_append(out, value.as_ptr().cast(), value.len());
}
unsafe fn substitute(c: *mut conv_ctx, value: &str, x: f64, y: f64) -> String {
    let p = subst_names(c, value.as_ptr().cast(), value.len(), x, y);
    let result = std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned();
    free(p.cast());
    result
}
// Keep nested PDF objects, strings (including UTF-16 octal escapes), and
// indirect references intact while merging dictionary entries.
unsafe fn dictionary(value: &str) -> BTreeMap<String, String> {
    let mut dict = BTreeMap::new();
    let data = value.as_bytes();
    let start = data.as_ptr().cast::<::core::ffi::c_char>();
    let end = start.add(data.len());
    let mut p = skip_ws(start, end);
    if end.offset_from(p) < 2 || *p != b'<' as _ || *p.add(1) != b'<' as _ {
        return dict;
    }
    p = p.add(2);
    while p < end {
        p = skip_ws(p, end);
        if p >= end || *p == b'>' as _ {
            break;
        }
        if *p != b'/' as _ {
            break;
        }
        let key = p.add(1);
        let mut ke = key;
        while ke < end
            && !(*ke as u8).is_ascii_whitespace()
            && !b"/()<>[]{}%".contains(&(*ke as u8))
        {
            ke = ke.add(1);
        }
        let vs = skip_ws(ke, end);
        let ve = skip_value(vs, end);
        if ve <= vs {
            break;
        }
        dict.insert(
            text(key, ke.offset_from(key) as usize),
            text(vs, ve.offset_from(vs) as usize),
        );
        p = ve;
    }
    dict
}
unsafe fn body(out: *mut pbuf, dict: &BTreeMap<String, String>) {
    for (key, value) in dict {
        append(out, &format!("/{key} {value}\n"));
    }
}
unsafe fn write_object(c: *mut conv_ctx, object: i32, dict: &BTreeMap<String, String>) {
    pdfw_begin((*c).pw, object);
    let out = pdfw_out((*c).pw);
    append(out, "<<");
    body(out, dict);
    append(out, ">>");
    pdfw_end((*c).pw);
}
fn token_end(value: &str) -> usize {
    value
        .bytes()
        .position(|b| b.is_ascii_whitespace() || b"()<>[]{}/%".contains(&b))
        .unwrap_or(value.len())
}
fn dimension(value: &str) -> Option<f64> {
    let i = value
        .bytes()
        .position(|b| !b.is_ascii_digit() && !b"+-.".contains(&b))
        .unwrap_or(value.len());
    let number = value[..i].parse::<f64>().ok()?;
    let unit = value[i..].trim();
    let factor = match unit {
        "" | "bp" => 1.,
        "pt" => 72. / 72.27,
        "sp" => 72. / 72.27 / 65536.,
        "in" => 72.,
        "cm" => 72. / 2.54,
        "mm" => 72. / 25.4,
        "pc" => 12. * 72. / 72.27,
        "dd" => 1238. / 1157. * 72. / 72.27,
        "cc" => 12. * 1238. / 1157. * 72. / 72.27,
        _ => return None,
    };
    Some(number * factor)
}

impl Document {
    pub unsafe fn begin_page(c: *mut conv_ctx) {
        let doc = &mut *(*c).document;
        doc.ctm = [1., 0., 0., 1., 0., 0.];
        doc.graphics_stack.clear();
        pdf_threads::Threads::page(c);
    }
    pub unsafe fn form_resource(c:*mut conv_ctx,object:i32)->String {
        let doc=&*(*c).document;
        let Some(ordinal)=doc.form_resources.get(&object) else{return format!("X{}",object)};
        let suffix=if doc.backend.integer("pdfuniqueresname")>0{font_output::namespace(c)}else{String::new()};
        format!("Fm{}{}",ordinal,suffix)
    }
    pub unsafe fn save(c: *mut conv_ctx) {
        let doc = &mut *(*c).document;
        doc.graphics_stack.push(doc.ctm);
    }
    pub unsafe fn restore(c: *mut conv_ctx) {
        let doc = &mut *(*c).document;
        if let Some(matrix) = doc.graphics_stack.pop() {
            doc.ctm = matrix;
        }
    }
    pub unsafe fn concat(c: *mut conv_ctx, m: [f64; 6]) {
        if !m.iter().all(|v| v.is_finite()) {
            return;
        }
        let doc = &mut *(*c).document;
        let t = doc.ctm;
        doc.ctm = [
            t[0] * m[0] + t[2] * m[1],
            t[1] * m[0] + t[3] * m[1],
            t[0] * m[2] + t[2] * m[3],
            t[1] * m[2] + t[3] * m[3],
            t[0] * m[4] + t[2] * m[5] + t[4],
            t[1] * m[4] + t[3] * m[5] + t[5],
        ];
    }
    pub unsafe fn point(c: *mut conv_ctx, x: f64, y: f64) -> (f64, f64) {
        let t = (*(*c).document).ctm;
        (t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5])
    }
    pub unsafe fn rectangle(c: *mut conv_ctx, r: [f64; 4]) -> [f64; 4] {
        let points = [
            Self::point(c, r[0], r[1]),
            Self::point(c, r[0], r[3]),
            Self::point(c, r[2], r[1]),
            Self::point(c, r[2], r[3]),
        ];
        [
            points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
            points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
            points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
            points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
        ]
    }
    pub unsafe fn track_raw(c: *mut conv_ctx, raw: *const ::core::ffi::c_char, len: usize) {
        let mut p = raw;
        let end = raw.add(len);
        let mut operands = Vec::new();
        while p < end {
            p = pdf_space(p, end);
            if p >= end {
                break;
            }
            let next = skip_value(p, end);
            if next <= p {
                break;
            }
            let word = text(p, next.offset_from(p) as usize);
            if let Ok(n) = word.parse::<f64>() {
                operands.push(n);
                if operands.len() > 6 {
                    operands.remove(0);
                }
            } else {
                match word.as_str() {
                    "q" => Self::save(c),
                    "Q" => Self::restore(c),
                    "cm" if operands.len() == 6 => {
                        Self::concat(c, operands[..].try_into().unwrap())
                    }
                    _ => {}
                }
                operands.clear();
            }
            p = next;
        }
    }
    unsafe fn destination(c:*mut conv_ctx,name:String,mut value:String) {
        if let Some(bytes)=pdf_string_bytes(&name) {
            let doc=&mut *(*c).document;
            if doc.destinations.contains_key(&bytes){if doc.backend.integer("pdfsuppresswarningdupdest")<=0{diagnostic(c,"duplicate-destination","duplicate destination ignored; the first definition is kept");}return;}
            let margin=doc.backend.dimension("pdfdestmargin");
            let elements=pdf_array_elements(&value);
            if elements.len()==6&&elements[1]=="/FitR" {
                if let Some(mut r)=elements[2..].iter().map(|s|s.parse::<f64>().ok()).collect::<Option<Vec<_>>>(){r[0]-=margin;r[1]-=margin;r[2]+=margin;r[3]+=margin;value=format!("[{} /FitR {} {} {} {}]",elements[0],r[0],r[1],r[2],r[3]);}
            }
            doc.destinations.insert(bytes,(name,value));
        }
    }
    pub unsafe fn reserve_pages(c: *mut conv_ctx, count: usize) {
        let doc = &mut *(*c).document;
        doc.pages = (0..count).map(|_| pdfw_alloc((*c).pw)).collect();
        for page in &doc.pages {
            doc.annotation_arrays.insert(*page, pdfw_alloc((*c).pw));
        }
        let pages=doc.pages.clone();pdf_threads::Threads::reserve(c,&pages);
    }
    pub unsafe fn named_reference(c: *mut conv_ctx, name: &str) -> Option<i32> {
        if let Some(id)=name.strip_prefix("@pitexobj").and_then(|id|id.parse::<i32>().ok()) {
            let doc=&*(*c).document;
            if let Some(page)=doc.logical_pages.get(&id) {return doc.pages.get(*page).copied();}
        }
        match name {
            "@catalog" => Some((*(*c).document).catalog_object),
            "@pages" => Some((*(*c).document).pages_object),
            "@docinfo" => Some((*(*c).document).info_object),
            "@names" => Some((*(*c).document).names_object),
            "@thispage" => Some((*c).this_page),
            "@prevpage" | "@nextpage" => {
                let doc = &*(*c).document;
                let index = doc.pages.iter().position(|p| *p == (*c).this_page)?;
                let next = if name == "@prevpage" {
                    index.checked_sub(1)?
                } else {
                    index + 1
                };
                doc.pages.get(next).copied()
            }
            _ => name
                .strip_prefix("@page")
                .and_then(|s| s.parse::<usize>().ok())
                .and_then(|n| n.checked_sub(1))
                .and_then(|n| (&*(*c).document).pages.get(n).copied()),
        }
    }
    pub unsafe fn touch(c: *mut conv_ctx, x: f64, y: f64, width: f64, height: f64, depth: f64) {
        if (*c).document.is_null() || !(*c).render {
            return;
        }
        pdf_threads::Threads::touch(c,x,y,width,height,depth);
        let rectangle = Self::rectangle(c, [x, y - depth, x + width, y + height]);
        for link in &mut (*(*c).document).links {
            if !pdf_threads::Threads::link_enabled(c,link.start_box,link.level){continue;}
            // A link spanning multiple lines has one annotation per line/page.
            if let Some((_, r, _)) = link.boxes.iter_mut().find(|(page, _, baseline)| {
                *page == (*c).this_page && (*baseline - y).abs() < height.max(depth).max(1.) * 0.65
            }) {
                r[0] = r[0].min(rectangle[0]);
                r[1] = r[1].min(rectangle[1]);
                r[2] = r[2].max(rectangle[2]);
                r[3] = r[3].max(rectangle[3]);
            } else {
                link.boxes.push(((*c).this_page, rectangle, y));
            }
        }
    }
    pub unsafe fn touch_rectangle(c:*mut conv_ctx,r:[f64;4]) {
        if !(*c).render{return;}
        pdf_threads::Threads::touch_rectangle(c,r);
        for link in &mut (*(*c).document).links {
            if !pdf_threads::Threads::link_enabled(c,link.start_box,link.level){continue;}
            let baseline=inverse_matrix(link.matrix).map(|m|transform_point(m,r[0],r[1]).1).unwrap_or(r[1]);
            if let Some((_,box_,_))=link.boxes.iter_mut().find(|(page,_,y)|*page==(*c).this_page&&(*y-baseline).abs()<(r[3]-r[1]).max(1.)*0.65){box_[0]=box_[0].min(r[0]);box_[1]=box_[1].min(r[1]);box_[2]=box_[2].max(r[2]);box_[3]=box_[3].max(r[3]);}
            else{link.boxes.push(((*c).this_page,r,baseline));}
        }
    }
    unsafe fn annotation(
        c: *mut conv_ctx,
        object: i32,
        mut dict: BTreeMap<String, String>,
        page: i32,
        rect: [f64; 4],
    ) {
        dict.entry("Type".into()).or_insert_with(|| "/Annot".into());
        dict.entry("P".into())
            .or_insert_with(|| format!("{page} 0 R"));
        dict.insert(
            "Rect".into(),
            format!(
                "[{:.6} {:.6} {:.6} {:.6}]",
                rect[0], rect[1], rect[2], rect[3]
            ),
        );
        write_object(c, object, &dict);
        (*(*c).document)
            .annotations
            .entry(page)
            .or_default()
            .push(object);
    }
    pub unsafe fn reset_colors(c:*mut conv_ctx) {
        (*(*c).document).color_stacks.clear();
        let mut main=PitexColorStack::default();main.page=true;
        (*(*c).document).color_stacks.push(main);
    }
    pub unsafe fn restore_colors(c:*mut conv_ctx) {
        let values=(*(*c).document).color_stacks.iter().filter(|stack|stack.page)
            .map(|stack|stack.values.last().unwrap_or(&stack.initial).clone()).collect::<Vec<_>>();
        for value in values { emit_raw(c,value.as_ptr().cast(),value.len()); }
    }
    pub unsafe fn colorstack(c:*mut conv_ctx,start:*const ::core::ffi::c_char,end:*const ::core::ffi::c_char,initial:bool,x:f64,y:f64) {
        let raw=text(start,end.offset_from(start) as usize);
        let raw=raw.trim();let split=raw.find(char::is_whitespace).unwrap_or(raw.len());
        let Ok(number)=raw[..split].parse::<usize>() else {return};
        let mut remainder=raw[split..].trim();
        if initial {
            let page=if let Some(rest)=remainder.strip_prefix("page ") {remainder=rest.trim_start();true} else {false};
            let direct=if let Some(rest)=remainder.strip_prefix("direct ") {remainder=rest.trim_start();true} else {false};
            let pointer=remainder.as_ptr().cast::<::core::ffi::c_char>();
            let value=paren_content(pointer,pointer.add(remainder.len()));
            let value_string=std::ffi::CStr::from_ptr(value).to_string_lossy().into_owned();free(value.cast());
            if number> (*(*c).document).color_stacks.len() {diagnostic(c,"colorstack","color stacks must be initialized in order");return;}
            if number==(*(*c).document).color_stacks.len() {(*(*c).document).color_stacks.push(PitexColorStack::default());}
            (*(*c).document).color_stacks[number]=PitexColorStack {initial:value_string.clone(),values:Vec::new(),page,direct};
            if page {Self::color_operations(c,&value_string,direct,x,y);}
            return;
        }
        let split=remainder.find(char::is_whitespace).unwrap_or(remainder.len());
        let action=&remainder[..split];let body=remainder[split..].trim();
        let Some(stack)=(*(*c).document).color_stacks.get_mut(number) else {diagnostic(c,"colorstack","unknown color stack");return;};
        match action {
            "push" | "set" => {
                let pointer=body.as_ptr().cast::<::core::ffi::c_char>();
                let value=paren_content(pointer,pointer.add(body.len()));
                let value_string=std::ffi::CStr::from_ptr(value).to_string_lossy().into_owned();free(value.cast());
                if action=="push" {stack.values.push(value_string);}
                else if let Some(last)=stack.values.last_mut() {*last=value_string;} else {stack.initial=value_string;}
            }
            "pop" => {if stack.values.pop().is_none() {diagnostic(c,"colorstack","cannot pop the initial color state");}},
            "current" => {}, _=>{diagnostic(c,"colorstack","unknown stack action");return;}
        }
        let value=stack.values.last().unwrap_or(&stack.initial).clone();
        let direct=stack.direct;
        Self::color_operations(c,&value,direct,x,y);
    }
    unsafe fn color_operations(c:*mut conv_ctx,value:&str,direct:bool,x:f64,y:f64) {
        if !direct {emit_cm(c,mat_translate(x,y));}
        emit_raw(c,value.as_ptr().cast(),value.len());
        if !direct {emit_cm(c,mat_translate(-x,-y));}
    }
    pub unsafe fn finish_version(c:*mut conv_ctx) {
        let actual=pdfw_effective_version((*c).pw);
        if let Some((major,minor))=(*(*c).document).version_request {
            if actual>major*10+minor {diagnostic(c,"pdf-version",&format!("PDF features require version {}.{}; requested {}.{} was raised to match",actual/10,actual%10,major,minor));}
        }
    }
    pub unsafe fn image_dictionary(c: *mut conv_ctx, image: *mut cached_image, original: *const ::core::ffi::c_char) -> std::ffi::CString {
        let mut body = std::ffi::CStr::from_ptr(original).to_string_lossy().into_owned();
        if let Some(attributes) = (*(*c).document).image_attributes.get(&(image as usize)) {
            for (key,value) in attributes { body.push_str(&format!(" /{} {}", key, value)); }
        }
        std::ffi::CString::new(body.replace('\0', "")).unwrap()
    }
    pub unsafe fn special(
        c: *mut conv_ctx,
        command: *const ::core::ffi::c_char,
        n: usize,
        arg: *const ::core::ffi::c_char,
        end: *const ::core::ffi::c_char,
        x: f64,
        y: f64,
    ) -> bool {
        let command = text(command, n);
        let raw = text(arg, end.offset_from(arg) as usize);
        let raw = raw.trim();
        if pdf_threads::Threads::special(c,&command,raw,x,y){return true;}
        match command.as_str() {
            "pitextokenconfig" => {
                if let Some((name,value))=raw.split_once(' ') {
                    if let Some(bytes)=pdf_string_bytes(value.trim()) {
                        (*(*c).document).backend.tokens.insert(name.to_owned(),String::from_utf8_lossy(&bytes).into_owned());
                    }
                }
                true
            }
            "pitexconfig" => {
                let values=dictionary(raw);backend_options::configure(c,&values);true
            }
            "glyphtounicode" | "pitexglyphmap" => {
                let mut values=Vec::new();let mut remaining=raw;
                while !remaining.is_empty() {
                    let start=remaining.as_ptr().cast::<::core::ffi::c_char>();let end=start.add(remaining.len());let after=skip_value(start,end);let length=after.offset_from(start) as usize;
                    if length==0 {break}
                    if let Some(bytes)=pdf_string_bytes(&remaining[..length]) {values.push(String::from_utf8_lossy(&bytes).into_owned());}else{break}
                    remaining=remaining[length..].trim_start();
                    if command=="glyphtounicode"&&values.len()==2{break}
                }
                if values.len()%2==0 {for pair in values.chunks_exact(2){if let Some(value)=backend_options::unicode_hex(&pair[1]){(*(*c).document).backend.glyph_unicode.insert(pair[0].clone(),value);}else{diagnostic(c,"unicode-map","invalid Unicode hexadecimal mapping");}}}
                else{diagnostic(c,"unicode-map","glyph mapping requires string pairs");}true
            }
            "mapline" | "fontmapline" | "mapfile" | "fontmapfile" => {
                let bytes=pdf_string_bytes(raw).unwrap_or_else(||raw.as_bytes().to_vec());
                if let Ok(spec)=std::ffi::CString::new(bytes) {
                    let before=font_map_duplicate_count((*(*c).w).fonts);
                    if command=="mapline" || command=="fontmapline" {font_map_line((*(*c).w).fonts,spec.as_ptr());}
                    else if !font_map_file((*(*c).w).fonts,spec.as_ptr()) {diagnostic(c,"font-map","font map file was not found");}
                    if font_map_duplicate_count((*(*c).w).fonts)>before&&(*(*c).document).backend.integer("pdfsuppresswarningdupmap")<=0{diagnostic(c,"duplicate-font-map","duplicate font map entry ignored; use replacement mode to override it");}
                }
                true
            }

            "pitexorigin" => {
                let values = raw.split_whitespace().filter_map(|s|s.parse::<i32>().ok()).collect::<Vec<_>>();
                if values.len()==2 { (*(*c).document).origin=Some([values[0] as f64 * 72. / 72.27 / 65536., values[1] as f64 * 72. / 72.27 / 65536.]); }
                true
            }
            "pitexoutput" => {
                let values=raw.split_whitespace().filter_map(|s|s.parse::<i32>().ok()).collect::<Vec<_>>();
                if values.len()==3 {
                    let mut major=values[0];let mut minor=values[1];
                    if major==1 && minor>7 {diagnostic(c,"pdf-version","supported PDF 1 output versions range from 1.0 through 1.7");minor=7;}
                    if major==2 && minor!=0 {diagnostic(c,"pdf-version","PDF 2 output uses version 2.0");minor=0;}
                    if !(1..=2).contains(&major) || !(0..=9).contains(&minor) {diagnostic(c,"pdf-version","invalid output version; using PDF 1.7");major=1;minor=7;}
                    (*(*c).document).version_request=Some((major,minor));
                    pdfw_configuration((*c).pw,major,minor,values[2]);
                    (*(*c).document).backend.requested_version=major*10+minor;
                    backend_options::configure(c,&BTreeMap::new());
                }
                true
            }
            "pdftexnames" => {
                let values=substitute(c,&pdftex_references(raw),x,y);
                (*(*c).document).names.extend(dictionary(&values));
                true
            }
            "pdftextrailer" => {
                let values=substitute(c,&pdftex_references(raw),x,y);
                let dictionary=dictionary(&values);
                let mut body=String::new();for(key,value) in dictionary {body.push_str(&format!("/{} {} ",key,value));}
                pdfw_trailer((*c).pw,body.as_ptr().cast(),body.len());true
            }
            "pdftextrailerid" => {
                if !raw.is_empty() {let id=format!("/ID {}",raw);pdfw_trailer((*c).pw,id.as_ptr().cast(),id.len());}
                true
            }

            "pitexpageref" => {
                let mut words=raw.split_whitespace();
                if let (Some(id),Some(page))=(words.next().and_then(|s|s.parse::<i32>().ok()),words.next().and_then(|s|s.parse::<usize>().ok())) {
                    if let Some(index)=page.checked_sub(1) {
                        (*(*c).document).logical_pages.insert(id,index);
                        if index>=(*(*c).document).pages.len() {diagnostic(c,"pageref",&format!("Page {} has been referenced but does not exist",page));}
                        let name=format!("@pitexobj{}",id);
                        let alias=named_get(c,name.as_ptr().cast(),name.len(),false);
                        if !alias.is_null() {if let Some(real)=(&*(*c).document).pages.get(index){(*alias).obj=*real;(*alias).written=true;}}
                    }
                }
                true
            }
            "pitexformbind" => {
                let mut words=raw.split_whitespace();
                let id=words.next();let ordinal=words.next().and_then(|s|s.parse::<usize>().ok());let name=words.next();
                if let (Some(id),Some(ordinal),Some(name)) = (id,ordinal,name) {
                    let alias = format!("@pitexobj{}",id);
                    let alias = named_get(c,alias.as_ptr().cast(),alias.len(),true);
                    let object = (*alias).obj;
                    (*(*c).document).form_resources.insert(object,ordinal);
                    (*alias).written = true;
                    let form_name = name.trim();
                    let form = named_get(c,form_name.as_ptr().cast(),form_name.len(),true);
                    (*form).obj = object;
                }
                true
            }
            "piteximagebind" => {
                let mut words = raw.splitn(4,' ');
                let id = words.next().and_then(|s|s.parse::<i32>().ok());
                let page = words.next().and_then(|s|s.parse::<i32>().ok());
                let box_number = words.next().and_then(|s|s.parse::<i32>().ok());
                let filename = words.next().and_then(pdf_string_bytes);
                if let (Some(id),Some(page),Some(box_number),Some(filename)) = (id,page,box_number,filename) {
                    if let Ok(filename) = std::ffi::CString::new(filename) {
                        let image = get_image(c,filename.as_ptr());
                        if !image.is_null() {
                            let previous_count = (*c).nxobjs;
                            let usage = use_xobject(c,image,page,box_number);
                            let name = format!("@pitexobj{}",id);
                            let alias = named_get(c,name.as_ptr().cast(),name.len(),true);
                            if (*c).nxobjs > previous_count { (*usage).obj = (*alias).obj; }
                            else { (*alias).obj = (*usage).obj; }
                            (*alias).written = true;
                        }
                    }
                }
                true
            }
            "pitexresources" => {
                let resources = substitute(c,&pdftex_references(raw),x,y);
                if let Ok(resources) = std::ffi::CString::new(resources) { merge_resources(&raw mut (*cur(c)).res,resources.as_ptr()); }
                true
            }
            "piteximageattributes" => {
                let start = raw.as_ptr().cast::<::core::ffi::c_char>();
                let end = start.add(raw.len());
                let tail = skip_value(start,end);
                let length = tail.offset_from(start) as usize;
                if let Some(bytes) = pdf_string_bytes(&raw[..length]) {
                    if let Ok(filename) = std::ffi::CString::new(bytes) {
                        let image = get_image(c,filename.as_ptr());
                        if !image.is_null() {
                            let attributes = substitute(c,&pdftex_references(raw[length..].trim()),x,y);
                            (*(*c).document).image_attributes.insert(image as usize,dictionary(&attributes));
                        }
                    }
                }
                true
            }
            "pdftexmatrix" => {
                let values = raw
                    .split_ascii_whitespace()
                    .filter_map(|s| s.parse::<f64>().ok())
                    .collect::<Vec<_>>();
                if values.len() == 4 && values.iter().all(|v| v.is_finite()) {
                    let (a, b, cc, d) = (values[0], values[1], values[2], values[3]);
                    emit_cm(
                        c,
                        mat {
                            a,
                            b,
                            c: cc,
                            d,
                            e: x * (1. - a) - y * cc,
                            f: y * (1. - d) - x * b,
                        },
                    );
                } else {
                    diagnostic(c, "matrix", "expected four finite matrix coefficients");
                }
                true
            }
            "literal" => {
                if let Some(value) = raw.strip_prefix("page ") {
                    emit_raw(c, value.as_ptr().cast(), value.len());
                    return true;
                }
                false
            }
            "info" | "docinfo" | "docview" | "pdftexcatalog" => {
                let input = if command == "info" || command == "pdftexcatalog" {
                    pdftex_references(raw)
                } else {
                    raw.to_owned()
                };
                let value = substitute(c, &input, x, y);
                let dict = dictionary(&value);
                let doc = &mut *(*c).document;
                if command == "docview" || command == "pdftexcatalog" {
                    doc.catalog.extend(dict);
                } else {
                    doc.info.extend(dict);
                }
                true
            }
            "put" => {
                let ne = token_end(raw);
                let name = &raw[..ne];
                if name.starts_with('@') {
                    let object = named_get(c, name.as_ptr().cast(), name.len(), false);
                    if !object.is_null() && (*object).written {
                        diagnostic(c, "closed-object", "cannot modify a closed object");
                        return true;
                    }
                    if !object.is_null() && (*object).kind == NO_STREAM {
                        let value = substitute(c, raw[ne..].trim(), x, y);
                        for (key, value) in dictionary(&value) {
                            if key != "Length" && key != "Filter" {
                                kv_set(
                                    &raw mut (*object).dict,
                                    key.as_ptr().cast(),
                                    key.len(),
                                    value.as_ptr().cast(),
                                    value.len(),
                                );
                            }
                        }
                        return true;
                    }
                }

                let target = match name {
                    "@catalog" => 0,
                    "@pages" => -1,
                    "@docinfo" => -2,
                    "@names" => -3,
                    _ => match Self::named_reference(c, name) {
                        Some(n) => n,
                        None => return false,
                    },
                };
                let value = substitute(c, raw[ne..].trim(), x, y);
                let dict = dictionary(&value);
                let doc = &mut *(*c).document;
                if target == 0 {
                    doc.catalog.extend(dict);
                } else if target == -2 {
                    doc.info.extend(dict);
                } else if target == -3 {
                    doc.names.extend(dict);
                } else {
                    doc.page_dicts.entry(target).or_default().extend(dict);
                }
                true
            }
            "ann" | "annot" => {
                if !(*c).render {
                    return true;
                }
                let pdftex_annotation=raw.starts_with("@pitexobj");
                let mut value = raw;
                let object = if value.starts_with('@') {
                    let n = token_end(value);
                    let o = named_get(c, value.as_ptr().cast(), n, true);
                    (*o).written = true;
                    value = value[n..].trim_start();
                    (*o).obj
                } else {
                    pdfw_alloc((*c).pw)
                };
                let (mut width, mut height, mut depth) = (0., 0., 0.);
                let mut bbox = None;
                while !value.starts_with("<<") && !value.is_empty() {
                    let key_end=["width","height","depth","bbox"].iter().find(|key|value.starts_with(**key)).map(|key|key.len()).unwrap_or_else(||token_end(value));
                    if key_end == 0 {
                        diagnostic(c, "annotation", "malformed annotation dimensions");
                        return true;
                    }
                    let key = &value[..key_end];
                    value = value[key_end..].trim_start();
                    if key == "bbox" {
                        let mut r = [0.; 4];
                        for coordinate in &mut r {
                            let Some((n, used)) = read_dimension(value) else {
                                diagnostic(c, "annotation", "bbox requires four numbers");
                                return true;
                            };
                            *coordinate = n;
                            value = value[used..].trim_start();
                        }
                        bbox = Some(r);
                    } else {
                        if !matches!(key, "width" | "height" | "depth") {
                            diagnostic(c, "annotation", "unknown annotation dimension");
                            return true;
                        }
                        let Some((number, used)) = read_dimension(value) else {
                            diagnostic(c, "annotation", "invalid annotation dimension");
                            return true;
                        };
                        value = value[used..].trim_start();
                        match key {
                            "width" => width = number,
                            "height" => height = number,
                            _ => depth = number,
                        }
                    }
                }
                if !value.starts_with("<<") {
                    diagnostic(c, "annotation", "missing annotation dictionary");
                    return true;
                }
                let input=if pdftex_annotation{pdftex_references(value)}else{value.to_owned()};
                let value = substitute(c, &input, x, y);
                let dict = dictionary(&value);
                let r = bbox
                    .map(|r| [x + r[0], y + r[1], x + r[2], y + r[3]])
                    .unwrap_or([x, y - depth, x + width, y + height]);
                let margin=(*(*c).document).threads.annotation_margin;let mut r=Self::rectangle(c,r);
                r=[r[0]-margin,r[1]-margin,r[2]+margin,r[3]+margin];
                Self::annotation(c,object,dict,(*c).this_page,r);
                true
            }
            "bann" | "bannot" | "beginann" | "pitexbann"=> {
                if (*c).render {
                    let (input,first_object)=if command=="pitexbann"&&raw.starts_with('@'){
                        let n=token_end(raw);let object=named_get(c,raw.as_ptr().cast(),n,true);(*object).written=true;(&raw[n..],Some((*object).obj))
                    }else{(raw,None)};
                    let mut input=input.trim_start();let(mut fixed_width,mut fixed_height,mut fixed_depth)=(None,None,None);
                    if command=="pitexbann" {
                        while !input.is_empty()&&!input.starts_with("<<"){
                            let Some(key)=["width","height","depth"].iter().find(|key|input.starts_with(**key))else{diagnostic(c,"link","invalid link dimensions");return true;};
                            input=input[key.len()..].trim_start();let Some((value,used))=read_dimension(input)else{diagnostic(c,"link","invalid link dimension");return true;};input=input[used..].trim_start();
                            match *key{"width"=>fixed_width=Some(value),"height"=>fixed_height=Some(value),_=>fixed_depth=Some(value)}
                        }
                    }
                    let input=if command=="pitexbann"{pdftex_references(input)}else{input.to_owned()};
                    let value = substitute(c, &input, x, y);
                    let dict = dictionary(&value);
                    let (start_box,level)=pdf_threads::Threads::link_box(c);
                    (*(*c).document).links.push(Link {start_box,level,first_object,fixed_width,fixed_height,fixed_depth,matrix:(*(*c).document).ctm,dict,boxes:Vec::new()});
                }
                true
            }
            "eann" | "eannot" | "endann" => {
                if let Some(link) = (*(*c).document).links.pop() {
                    for (index,(page,mut rectangle,baseline))in link.boxes.into_iter().enumerate(){
                        if link.fixed_width.is_some()||link.fixed_height.is_some()||link.fixed_depth.is_some(){
                            let mut local=inverse_matrix(link.matrix).map(|m|transform_rectangle(m,rectangle)).unwrap_or(rectangle);
                            if let Some(w)=link.fixed_width{local[2]=local[0]+w;}
                            if let Some(h)=link.fixed_height{local[3]=baseline+h;}
                            if let Some(d)=link.fixed_depth{local[1]=baseline-d;}
                            rectangle=transform_rectangle(link.matrix,local);
                        }
                        let object=if index==0{link.first_object.unwrap_or_else(||pdfw_alloc((*c).pw))}else{pdfw_alloc((*c).pw)};
                        let margin=(*(*c).document).threads.link_margin;
                        rectangle=[rectangle[0]-margin,rectangle[1]-margin,rectangle[2]+margin,rectangle[3]+margin];
                        Self::annotation(c,object,link.dict.clone(),page,rectangle);
                    }
                }
                true
            }
            "fitrdest"=>{
                if !(*c).render{return true;}
                let ptr=raw.as_ptr().cast::<::core::ffi::c_char>();let end=ptr.add(raw.len());let ne=skip_value(ptr,end).offset_from(ptr) as usize;
                if ne==0{diagnostic(c,"destination","FitR destination requires a name");return true;}
                let name=raw[..ne].to_owned();let mut input=raw[ne..].trim_start();
                let(mut width,mut height,mut depth)=pdf_threads::Threads::dimensions(c).unwrap_or((0.,0.,0.));
                while !input.is_empty(){let Some(key)=["width","height","depth"].iter().find(|key|input.starts_with(**key))else{diagnostic(c,"destination","invalid FitR dimensions");return true;};input=input[key.len()..].trim_start();let Some((value,used))=read_dimension(input)else{diagnostic(c,"destination","invalid FitR dimension");return true;};input=input[used..].trim_start();match *key{"width"=>width=value,"height"=>height=value,_=>depth=value}}
                let r=Self::rectangle(c,[x,y-depth,x+width,y+height]);
                Self::destination(c,name,format!("[{} 0 R /FitR {} {} {} {}]",(*c).this_page,r[0],r[1],r[2],r[3]));true
            },
            "dest" => {
                if !(*c).render {
                    return true;
                }
                let ptr = raw.as_ptr().cast::<::core::ffi::c_char>();
                let end = ptr.add(raw.len());
                let ne = skip_value(ptr, end).offset_from(ptr) as usize;
                if ne > 0 {
                    let name = raw[..ne].to_owned();
                    let (x, y) = Self::point(c, x, y);
                    let value = substitute(c, raw[ne..].trim(), x, y);
                    Self::destination(c,name,value);
                }
                true
            }
            "outline" | "out" => {
                if !(*c).render {
                    return true;
                }
                let mut raw = raw;
                let mut open = true;
                if raw.starts_with('[') {
                    if let Some(i) = raw.find(']') {
                        open = !raw[..=i].contains('-');
                        raw = raw[i + 1..].trim_start();
                    }
                }
                let n = raw
                    .bytes()
                    .position(|b| !b.is_ascii_digit() && b != b'-')
                    .unwrap_or(raw.len());
                let level = raw[..n].parse::<i32>().unwrap_or(1);
                raw = raw[n..].trim_start();
                let value = substitute(c, raw, x, y);
                let dict = dictionary(&value);
                let object = pdfw_alloc((*c).pw);
                (*(*c).document).outlines.push(Outline {
                    object,
                    level,
                    open,
                    dict,
                });
                true
            }
            "close" => {
                let n = token_end(raw);
                let object = named_get(c, raw.as_ptr().cast(), n, false);
                if !object.is_null() {
                    write_named(c, object);
                } else {
                    diagnostic(c, "close", "named object was not defined");
                }
                true
            }
            "refobj" => true,
            "names" => {
                let n = token_end(raw.strip_prefix('/').unwrap_or(raw));
                let category = raw.trim_start_matches('/');
                let category = &category[..n];
                let tail = raw.strip_prefix('/').unwrap_or(raw)[n..].trim();
                let value = substitute(c, tail, x, y);
                let ptr = value.as_ptr().cast::<::core::ffi::c_char>();
                let end = ptr.add(value.len());
                let mut p = pdf_space(ptr, end);
                if p < end && *p == b'[' as _ {
                    p = p.add(1);
                }
                while p < end && *p != b']' as _ {
                    let ne = skip_value(p, end);
                    let vs = pdf_space(ne, end);
                    let ve = skip_value(vs, end);
                    if ne <= p || ve <= vs {
                        diagnostic(c, "names", "malformed name-tree entry");
                        break;
                    }
                    let name = text(p, ne.offset_from(p) as usize);
                    let object = text(vs, ve.offset_from(vs) as usize);
                    if category == "Dests" {
                        Self::destination(c,name,object);
                    } else if let Some(key) = pdf_string_bytes(&name) {
                        (*(*c).document)
                            .name_trees
                            .entry(category.to_owned())
                            .or_default()
                            .insert(key, (name, object));
                    } else {
                        diagnostic(c, "names", "name-tree key must be a PDF string");
                    }
                    p = pdf_space(ve, end);
                }
                true
            }
            "stream" | "fstream" => {
                if !raw.starts_with('@') {
                    diagnostic(c, "stream", "stream requires a named object");
                    return true;
                }
                let n = token_end(raw);
                let name = &raw[..n];
                let tail = raw[n..].trim_start();
                let ptr = tail.as_ptr().cast::<::core::ffi::c_char>();
                let end = ptr.add(tail.len());
                let de = skip_value(ptr, end);
                let len = de.offset_from(ptr) as usize;
                let Some(mut bytes) = pdf_string_bytes(&tail[..len]) else {
                    diagnostic(c, "stream", "stream source must be a PDF string");
                    return true;
                };
                if command == "fstream" {
                    let Ok(filename) = std::ffi::CString::new(bytes) else {
                        diagnostic(c, "stream", "invalid stream file name");
                        return true;
                    };
                    let Some(load) = (*(*c).w).res.load else {
                        diagnostic(c, "stream", "stream file resolver unavailable");
                        return true;
                    };
                    let file = load((*(*c).w).res.env, filename.as_ptr(), RES_IMAGE);
                    if file.is_null() {
                        diagnostic(c, "stream", "stream file was not found");
                        return true;
                    }
                    bytes = std::slice::from_raw_parts((*file).data, (*file).len).to_vec();
                    tbuf_drop(file);
                }
                let input = if name.starts_with("@pitexobj") {
                    pdftex_references(tail[len..].trim())
                } else {
                    tail[len..].to_owned()
                };
                let value = substitute(c, &input, x, y);
                let dict = dictionary(&value);
                let object = named_get(c, name.as_ptr().cast(), name.len(), true);
                if (*object).written {
                    diagnostic(c, "stream", "stream object is already closed");
                    return true;
                }
                (*object).kind = NO_STREAM;
                pbuf_clear(&raw mut (*object).array);
                pbuf_append(&raw mut (*object).array, bytes.as_ptr().cast(), bytes.len());
                for (key, value) in dict {
                    if key != "Length" {
                        kv_set(
                            &raw mut (*object).dict,
                            key.as_ptr().cast(),
                            key.len(),
                            value.as_ptr().cast(),
                            value.len(),
                        );
                    }
                }
                true
            }
            _ => false,
        }
    }
    pub unsafe fn page_body(c: *mut conv_ctx, out: *mut pbuf, page: i32) {
        pdf_threads::Threads::page_body(c,out,page);
        let doc = &*(*c).document;
        if let Some(dict) = doc.page_dicts.get(&page) {
            body(out, dict);
        }
        if let Some(array) = doc.annotation_arrays.get(&page) {
            append(out, &format!("/Annots {array} 0 R"));
        }
    }
    pub unsafe fn pages_body(c: *mut conv_ctx, out: *mut pbuf) {
        if let Some(dict) = (*(*c).document).page_dicts.get(&-1) {
            body(out, dict);
        }
    }
    pub unsafe fn catalog_body(c: *mut conv_ctx, out: *mut pbuf) {
        body(out, &(*(*c).document).catalog);
    }
    pub unsafe fn info_body(c: *mut conv_ctx, out: *mut pbuf) {
        let mut dict = (*(*c).document).info.clone();
        dict.entry("Producer".into())
            .or_insert_with(|| "(Pitex embedded editing preview)".into());
        dict.entry("Subject".into())
            .or_insert_with(|| "(Editing preview - not final compiler output)".into());
        (*(*c).document).backend.info_policy().automatic_info(&mut dict,crate::pdf_metadata::creation_date(),"Pitex embedded XeTeX editing preview");
        body(out, &dict);
    }
    pub unsafe fn finish(c: *mut conv_ctx) {
        if font_map_duplicate_count((*(*c).w).fonts)>0&&(*(*c).document).backend.integer("pdfsuppresswarningdupmap")<=0{diagnostic(c,"duplicate-font-map","duplicate font map entry ignored; use replacement mode to override it");}
        pdf_threads::Threads::finish(c);
        let doc = &mut *(*c).document;
        // Arrays are written after every page, including links ending on a
        // later page; page dictionaries already hold these stable references.
        for (page, object) in &doc.annotation_arrays {
            pdfw_begin((*c).pw, *object);
            let out = pdfw_out((*c).pw);
            append(out, "[");
            if let Some(annots) = doc.annotations.get(page) {
                for annotation in annots {
                    append(out, &format!("{annotation} 0 R "));
                }
            }
            append(out, "]");
            pdfw_end((*c).pw);
        }
        for (category, tree) in &doc.name_trees {
            let object = pdfw_alloc((*c).pw);
            pdfw_begin((*c).pw, object);
            let out = pdfw_out((*c).pw);
            append(out, "<</Names[");
            for (_, (name, value)) in tree {
                append(out, &format!("{name} {value}\n"));
            }
            append(out, "]>>");
            pdfw_end((*c).pw);
            doc.names.insert(category.clone(), format!("{object} 0 R"));
        }
        if !doc.destinations.is_empty() {
            let object = pdfw_alloc((*c).pw);
            pdfw_begin((*c).pw, object);
            let out = pdfw_out((*c).pw);
            append(out, "<</Names[");
            for (_, (name, value)) in &doc.destinations {
                append(out, &format!("{name} {value}\n"));
            }
            append(out, "]>>");
            pdfw_end((*c).pw);
            doc.names.insert("Dests".into(), format!("{object} 0 R"));
        }
        if !doc.names.is_empty() || doc.catalog.contains_key("Names") {
            let existing = doc.catalog.remove("Names").unwrap_or_else(|| "<<>>".into());
            let mut names = dictionary(&existing);
            let indirect = existing
                .split_ascii_whitespace()
                .next()
                .and_then(|n| n.parse::<i32>().ok())
                .and_then(|number| {
                    (0..(*c).nnamed)
                        .find(|i| (*(*c).named.offset(*i as isize)).obj == number)
                        .map(|i| (*c).named.offset(i as isize))
                });
            if let Some(object) = indirect {
                for (key, value) in &doc.names {
                    kv_set(
                        &raw mut (*object).dict,
                        key.as_ptr().cast(),
                        key.len(),
                        value.as_ptr().cast(),
                        value.len(),
                    );
                }
                let mut merged = BTreeMap::new();
                for i in 0..(*object).dict.n {
                    let entry = &*(*object).dict.items.add(i as usize);
                    merged.insert(std::ffi::CStr::from_ptr(entry.key).to_string_lossy().into_owned(),std::ffi::CStr::from_ptr(entry.value).to_string_lossy().into_owned());
                }
                write_object(c,doc.names_object,&merged);
                doc.catalog.insert("Names".into(),format!("{} 0 R",doc.names_object));
            } else {
                names.extend(doc.names.clone());
                write_object(c,doc.names_object,&names);
                doc.catalog.insert("Names".into(),format!("{} 0 R",doc.names_object));
            }
        } else {
            write_object(c,doc.names_object,&BTreeMap::new());
        }
        if !doc.outlines.is_empty() {
            let root = pdfw_alloc((*c).pw);
            let n = doc.outlines.len();
            let mut parent = vec![None; n];
            let mut children = vec![Vec::new(); n + 1];
            let mut stack: Vec<usize> = Vec::new();
            for i in 0..n {
                while stack
                    .last()
                    .is_some_and(|j| doc.outlines[*j].level >= doc.outlines[i].level)
                {
                    stack.pop();
                }
                parent[i] = stack.last().copied();
                children[parent[i].map(|p| p + 1).unwrap_or(0)].push(i);
                stack.push(i);
            }
            fn visible(children: &[Vec<usize>], items: &[Outline], node: usize) -> usize {
                children[node]
                    .iter()
                    .map(|i| {
                        1 + if items[*i].open {
                            visible(children, items, i + 1)
                        } else {
                            0
                        }
                    })
                    .sum()
            }
            for i in 0..n {
                let item = &doc.outlines[i];
                let mut dict = item.dict.clone();
                let p = parent[i].map(|p| doc.outlines[p].object).unwrap_or(root);
                dict.insert("Parent".into(), format!("{p} 0 R"));
                let siblings = &children[parent[i].map(|p| p + 1).unwrap_or(0)];
                let at = siblings.iter().position(|v| *v == i).unwrap();
                if at > 0 {
                    dict.insert(
                        "Prev".into(),
                        format!("{} 0 R", doc.outlines[siblings[at - 1]].object),
                    );
                }
                if at + 1 < siblings.len() {
                    dict.insert(
                        "Next".into(),
                        format!("{} 0 R", doc.outlines[siblings[at + 1]].object),
                    );
                }
                if let (Some(first), Some(last)) = (children[i + 1].first(), children[i + 1].last())
                {
                    dict.insert(
                        "First".into(),
                        format!("{} 0 R", doc.outlines[*first].object),
                    );
                    dict.insert("Last".into(), format!("{} 0 R", doc.outlines[*last].object));
                    let count = visible(&children, &doc.outlines, i + 1) as i32;
                    dict.insert(
                        "Count".into(),
                        (if item.open { count } else { -count }).to_string(),
                    );
                }
                write_object(c, item.object, &dict);
            }
            let mut dict = BTreeMap::new();
            dict.insert("Type".into(), "/Outlines".into());
            dict.insert(
                "Count".into(),
                visible(&children, &doc.outlines, 0).to_string(),
            );
            dict.insert(
                "First".into(),
                format!("{} 0 R", doc.outlines[children[0][0]].object),
            );
            dict.insert(
                "Last".into(),
                format!("{} 0 R", doc.outlines[*children[0].last().unwrap()].object),
            );
            write_object(c, root, &dict);
            doc.catalog.insert("Outlines".into(), format!("{root} 0 R"));
        }
    }
}

// OpenType vertical metrics. Positions are normalized to PDF glyph space;
// malformed optional font tables use the PDF-defined vertical defaults.
fn u16be(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}
fn i16be(bytes: &[u8], offset: usize) -> Option<i16> {
    Some(i16::from_be_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}
fn u32be(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
fn table<'a>(sfnt: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    let count = u16be(sfnt, 4)? as usize;
    for index in 0..count {
        let at = 12 + index * 16;
        if sfnt.get(at..at + 4)? == tag {
            let offset = u32be(sfnt, at + 8)? as usize;
            let length = u32be(sfnt, at + 12)? as usize;
            return sfnt.get(offset..offset.checked_add(length)?);
        }
    }
    None
}
pub unsafe fn vertical_metrics(c: *mut conv_ctx, font: *mut pdf_font, out: *mut pbuf) {
    let face = &*(*font).nf;
    let sfnt = std::slice::from_raw_parts(face.sfnt, face.sfnt_len);
    let scale = 1000. / face.units_per_em.max(1) as f64;
    let vhea = table(sfnt, b"vhea");
    let vmtx = table(sfnt, b"vmtx");
    let vorg = table(sfnt, b"VORG");
    let count = vhea.and_then(|t| u16be(t, 34)).unwrap_or(0) as usize;
    if count == 0 {
        return;
    }
    append(out, "/DW2[880 -1000]/W2[");
    for gid in 0..(*font).nused as usize {
        if *(*font).used.add(gid) == 0 {
            continue;
        }
        let advance = vmtx
            .and_then(|t| u16be(t, gid.min(count - 1) * 4))
            .map(|v| v as f64 * scale)
            .unwrap_or(1000.);
        let horizontal = native_advance((*font).nf, gid as i32) as f64 * scale / 2.;
        let origin = if let Some(t) = vorg {
            let mut origin = i16be(t, 4).unwrap_or((880. / scale) as i16) as f64 * scale;
            for index in 0..u16be(t, 6).unwrap_or(0) as usize {
                let offset = 8 + index * 4;
                if u16be(t, offset) == Some(gid as u16) {
                    origin = i16be(t, offset + 2).unwrap_or(0) as f64 * scale;
                    break;
                }
            }
            origin
        } else {
            let bearing_offset = if gid < count {
                gid * 4 + 2
            } else {
                count * 4 + (gid - count) * 2
            };
            let bearing = vmtx.and_then(|t| i16be(t, bearing_offset)).unwrap_or(0) as f64;
            let ymax = table(sfnt, b"head").and_then(|head| {
                let format = i16be(head, 50)?;
                let loca = table(sfnt, b"loca")?;
                let glyf = table(sfnt, b"glyf")?;
                let offset = if format == 0 {
                    u16be(loca, gid * 2)? as usize * 2
                } else {
                    u32be(loca, gid * 4)? as usize
                };
                Some(i16be(glyf, offset + 8)? as f64)
            });
            ymax.map(|y| (y + bearing) * scale).unwrap_or(880.)
        };
        append(
            out,
            &format!("{gid}[{:.6} {:.6} {:.6}]", -advance, horizontal, origin),
        );
    }
    append(out, "]");
    let _ = c;
}

// pdfTeX's public object numbers are local engine IDs, not converter object
// numbers. Translate indirect references at token boundaries, leaving PDF
// strings, hex strings, names, comments, and generation numbers intact.
pub(super) fn pdftex_references(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = String::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        if bytes[at] == b'(' {
            at += 1;
            let mut depth = 1;
            while at < bytes.len() && depth > 0 {
                match bytes[at] {
                    b'\\' => {
                        at = (at + 2).min(bytes.len());
                        continue;
                    }
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                at += 1;
            }
        } else if bytes[at] == b'<' && bytes.get(at + 1) == Some(&b'<') {
            at += 2;
        } else if bytes[at] == b'<' {
            at += 1;
            while at < bytes.len() && bytes[at] != b'>' {
                at += 1;
            }
            at = (at + 1).min(bytes.len());
        } else if bytes[at] == b'%' {
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
        } else if bytes[at] == b'/' {
            at += 1;
            while at < bytes.len()
                && !bytes[at].is_ascii_whitespace()
                && !b"()<>[]{}/%".contains(&bytes[at])
            {
                at += 1;
            }
        } else if bytes[at].is_ascii_digit()
            && (at == 0 || bytes[at - 1].is_ascii_whitespace() || b"[<>".contains(&bytes[at - 1]))
        {
            at += 1;
            while at < bytes.len() && bytes[at].is_ascii_digit() {
                at += 1;
            }
            let number = &value[start..at];
            let mut q = at;
            while q < bytes.len() && bytes[q].is_ascii_whitespace() {
                q += 1;
            }
            if bytes.get(q) == Some(&b'0') && bytes.get(q + 1).is_some_and(u8::is_ascii_whitespace)
            {
                q += 1;
                while q < bytes.len() && bytes[q].is_ascii_whitespace() {
                    q += 1;
                }
                if bytes.get(q) == Some(&b'R')
                    && (q + 1 == bytes.len()
                        || bytes[q + 1].is_ascii_whitespace()
                        || b"]<>/".contains(&bytes[q + 1]))
                {
                    out.push_str(&format!("@pitexobj{number}"));
                    at = q + 1;
                    continue;
                }
            }
        } else {
            at += value[at..].chars().next().map(char::len_utf8).unwrap_or(1);
        }
        out.push_str(&value[start..at]);
    }
    out
}
pub unsafe fn substitute_pdftex(
    c: *mut conv_ctx,
    s: *const ::core::ffi::c_char,
    len: usize,
    x: f64,
    y: f64,
) -> *mut ::core::ffi::c_char {
    let value = pdftex_references(&text(s, len));
    subst_names(c, value.as_ptr().cast(), value.len(), x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn primitive_references_preserve_pdf_strings_and_names() {
        assert_eq!(
            pdftex_references(
                "<< /Obj 17 0 R /Array[3 0 R] /Literal(17 0 R) /Hex<313720302052> /17 0 R >>"
            ),
            "<< /Obj @pitexobj17 /Array[@pitexobj3] /Literal(17 0 R) /Hex<313720302052> /17 0 R >>"
        );
        assert_eq!(pdftex_references("% 8 0 R\n9 0 R"), "% 8 0 R\n@pitexobj9");
    }
    #[test]
    fn dictionaries_keep_nested_actions_and_indirect_references() {
        unsafe {
            let parsed = dictionary(
                "<< /Title(Outer (nested) title) /A<</S/GoTo/D(section.1)>> /Next 12 0 R >>",
            );
            assert_eq!(parsed.get("Title").unwrap(), "(Outer (nested) title)");
            assert_eq!(parsed.get("A").unwrap(), "<</S/GoTo/D(section.1)>>");
            assert_eq!(parsed.get("Next").unwrap(), "12 0 R");
        }
    }
    #[test]
    fn annotation_dimensions_use_pdf_points() {
        assert!((dimension("72.27pt").unwrap() - 72.).abs() < 1e-10);
        assert!((dimension("25.4mm").unwrap() - 72.).abs() < 1e-10);
        assert!((dimension("65536sp").unwrap() - 72. / 72.27).abs() < 1e-10);
    }
}

unsafe fn diagnostic(c: *mut conv_ctx, feature: &str, message: &str) {
    let key = std::ffi::CString::new(format!("pdf-{feature}")).unwrap();
    let message = std::ffi::CString::new(message).unwrap();
    warn_once(
        c,
        key.as_ptr(),
        b"PDF special: %s\0".as_ptr().cast(),
        message.as_ptr(),
    );
}
unsafe fn pdf_space(
    mut p: *const ::core::ffi::c_char,
    end: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    loop {
        while p < end && (*p as u8).is_ascii_whitespace() {
            p = p.add(1);
        }
        if p >= end || *p != b'%' as _ {
            return p;
        }
        while p < end && !matches!(*p as u8, b'\r' | b'\n') {
            p = p.add(1);
        }
    }
}
fn read_dimension(value: &str) -> Option<(f64, usize)> {
    let bytes = value.as_bytes();
    let mut at = 0;
    while at < bytes.len() && (bytes[at].is_ascii_digit() || b"+-.".contains(&bytes[at])) {
        at += 1;
    }
    if at == 0 {
        return None;
    }
    let number = &value[..at];
    let end = at;
    while at < bytes.len() && bytes[at].is_ascii_whitespace() {
        at += 1;
    }
    let unit_start = at;
    while at < bytes.len() && bytes[at].is_ascii_alphabetic() {
        at += 1;
    }
    let unit = &value[unit_start..at];
    let unit = unit.strip_prefix("true").unwrap_or(unit);
    let (n, used) = if let Some(n) = dimension(&format!("{number}{unit}")) {
        (n, at)
    } else if unit_start > end {
        (dimension(number)?, end)
    } else {
        return None;
    };
    n.is_finite().then_some((n, used))
}
fn pdf_string_bytes(value: &str) -> Option<Vec<u8>> {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.first() == Some(&b'<') && bytes.get(1) != Some(&b'<') && bytes.last() == Some(&b'>') {
        let mut digits = bytes[1..bytes.len() - 1]
            .iter()
            .copied()
            .filter(|b| !b.is_ascii_whitespace())
            .collect::<Vec<_>>();
        if digits.len() % 2 != 0 {
            digits.push(b'0');
        }
        return digits
            .chunks_exact(2)
            .map(|p| {
                let hi = (p[0] as char).to_digit(16)?;
                let lo = (p[1] as char).to_digit(16)?;
                Some((hi * 16 + lo) as u8)
            })
            .collect();
    }
    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
        return None;
    }
    let mut result = Vec::new();
    let mut at = 1;
    while at + 1 < bytes.len() {
        let byte = bytes[at];
        at += 1;
        if byte == b'\\' {
            if at + 1 >= bytes.len() {
                break;
            }
            let escaped = bytes[at];
            at += 1;
            match escaped {
                b'n' => result.push(b'\n'),
                b'r' => result.push(b'\r'),
                b't' => result.push(b'\t'),
                b'b' => result.push(8),
                b'f' => result.push(12),
                b'\r' => {
                    if bytes.get(at) == Some(&b'\n') {
                        at += 1;
                    }
                }
                b'\n' => {}
                b'0'..=b'7' => {
                    let mut number = (escaped - b'0') as u16;
                    for _ in 0..2 {
                        if at + 1 < bytes.len() && (b'0'..=b'7').contains(&bytes[at]) {
                            number = number * 8 + (bytes[at] - b'0') as u16;
                            at += 1;
                        } else {
                            break;
                        }
                    }
                    result.push(number as u8);
                }
                _ => result.push(escaped),
            }
        } else if byte == b'\r' {
            if bytes.get(at) == Some(&b'\n') {
                at += 1;
            }
            result.push(b'\n');
        } else {
            result.push(byte);
        }
    }
    Some(result)
}

fn transform_point(m:[f64;6],x:f64,y:f64)->(f64,f64){(m[0]*x+m[2]*y+m[4],m[1]*x+m[3]*y+m[5])}
fn transform_rectangle(m:[f64;6],r:[f64;4])->[f64;4]{
    let points=[(r[0],r[1]),(r[0],r[3]),(r[2],r[1]),(r[2],r[3])].map(|(x,y)|transform_point(m,x,y));
    [points.iter().map(|p|p.0).fold(f64::INFINITY,f64::min),points.iter().map(|p|p.1).fold(f64::INFINITY,f64::min),points.iter().map(|p|p.0).fold(f64::NEG_INFINITY,f64::max),points.iter().map(|p|p.1).fold(f64::NEG_INFINITY,f64::max)]
}
fn inverse_matrix(m:[f64;6])->Option<[f64;6]>{let det=m[0]*m[3]-m[1]*m[2];if det.abs()<1e-15||!det.is_finite(){return None;}let(a,b,c,d)=(m[3]/det,-m[1]/det,-m[2]/det,m[0]/det);Some([a,b,c,d,-a*m[4]-c*m[5],-b*m[4]-d*m[5]])}

fn pdf_array_elements(value:&str)->Vec<String>{
    let bytes=value.as_bytes();if bytes.first()!=Some(&b'['){return Vec::new();}
    unsafe{let start=bytes.as_ptr().cast::<::core::ffi::c_char>();let end=start.add(bytes.len());let mut p=start.add(1);let mut result=Vec::new();while p<end{p=pdf_space(p,end);if p>=end||*p==b']' as _{break;}let next=skip_value(p,end);if next<=p{break;}result.push(text(p,next.offset_from(p) as usize));p=next;}result}
}
