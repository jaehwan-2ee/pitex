#[path = "../../shared/pdf_objects.rs"]
mod pdf_objects;

/* Pitex embedded preview engine — PDF object writer.
 * Written from the PDF 1.7 specification. Pitex-authored (AGPL-3.0-or-later). */
// Translated from driver/pdfw.c with C2Rust 0.22.1.
extern "C" {
    fn compressBound(source_len: ::core::ffi::c_ulong) -> ::core::ffi::c_ulong;
    fn compress2(dest: *mut u8,dest_len: *mut ::core::ffi::c_ulong,source:*const u8,source_len: ::core::ffi::c_ulong,level: ::core::ffi::c_int)->::core::ffi::c_int;
    fn uncompress(dest: *mut u8, dest_len: *mut ::core::ffi::c_ulong, source: *const u8, source_len: ::core::ffi::c_ulong) -> ::core::ffi::c_int;
    pub type pr_doc;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn pbuf_printf(b: *mut pbuf, fmt: *const ::core::ffi::c_char, ...);
    fn pbuf_real(b: *mut pbuf, v: ::core::ffi::c_double);
    fn pbuf_deflate(
        out: *mut pbuf,
        data: *const ::core::ffi::c_uchar,
        len: size_t,
        level: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn pr_load(doc: *mut pr_doc, num: ::core::ffi::c_int) -> *mut pr_obj;
    fn pr_stream_raw(
        doc: *mut pr_doc,
        stream: *mut pr_obj,
        len: *mut size_t,
    ) -> *const ::core::ffi::c_uchar;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
}
pub type pr_type = ::core::ffi::c_uint;
pub const PR_STREAM: pr_type = 9;
pub const PR_REF: pr_type = 8;
pub const PR_DICT: pr_type = 7;
pub const PR_ARRAY: pr_type = 6;
pub const PR_STRING: pr_type = 5;
pub const PR_NAME: pr_type = 4;
pub const PR_REAL: pr_type = 3;
pub const PR_INT: pr_type = 2;
pub const PR_BOOL: pr_type = 1;
pub const PR_NULL: pr_type = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pr_obj {
    pub type_0: pr_type,
    pub u: C2RustUnnamed,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub b: ::core::ffi::c_int,
    pub i: ::core::ffi::c_longlong,
    pub r: ::core::ffi::c_double,
    pub str_0: C2RustUnnamed_4,
    pub arr: C2RustUnnamed_3,
    pub dict: C2RustUnnamed_2,
    pub ref_0: C2RustUnnamed_1,
    pub stream: C2RustUnnamed_0,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_0 {
    pub dict: *mut pr_obj,
    pub offset: size_t,
    pub length: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_1 {
    pub num: ::core::ffi::c_int,
    pub gen: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_2 {
    pub keys: *mut *mut ::core::ffi::c_char,
    pub vals: *mut *mut pr_obj,
    pub n: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_3 {
    pub items: *mut *mut pr_obj,
    pub n: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_4 {
    pub s: *mut ::core::ffi::c_char,
    pub len: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pdfw {
    pub object_streams:*mut pdf_objects::ObjectStreams,
    pub out: *mut pbuf,
    pub compression: ::core::ffi::c_int,
    pub requested_version: i32,
    pub minimum_version: i32,
    pub object_start: usize,
    pub trailer: pbuf,
    pub offsets: *mut size_t,
    pub count: ::core::ffi::c_int,
    pub cap: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pdfw_import {
    pub redirects: *mut std::collections::BTreeSet<i32>,
    pub w: *mut pdfw,
    pub src: *mut pr_doc,
    pub map: *mut ::core::ffi::c_int,
    pub nmap: ::core::ffi::c_int,
    pub queue: *mut ::core::ffi::c_int,
    pub nq: ::core::ffi::c_int,
    pub capq: ::core::ffi::c_int,
    pub written: ::core::ffi::c_int,
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
#[inline]
unsafe extern "C" fn pbuf_reserve(mut b: *mut pbuf, mut extra: size_t) {
    if (*b).len.wrapping_add(extra) <= (*b).cap {
        return;
    }
    let mut cap: size_t = if (*b).cap != 0 { (*b).cap } else { 256 as size_t };
    while cap < (*b).len.wrapping_add(extra) {
        cap = cap.wrapping_mul(2 as size_t);
    }
    let mut p: *mut ::core::ffi::c_uchar = realloc(
        (*b).data as *mut ::core::ffi::c_void,
        cap,
    ) as *mut ::core::ffi::c_uchar;
    if p.is_null() {
        abort();
    }
    (*b).data = p;
    (*b).cap = cap;
}
#[inline]
unsafe extern "C" fn pbuf_append(
    mut b: *mut pbuf,
    mut data: *const ::core::ffi::c_void,
    mut len: size_t,
) {
    if len == 0 {
        return;
    }
    pbuf_reserve(b, len);
    memcpy((*b).data.offset((*b).len as isize) as *mut ::core::ffi::c_void, data, len);
    (*b).len = (*b).len.wrapping_add(len);
}
#[inline]
unsafe extern "C" fn pbuf_putc(mut b: *mut pbuf, mut c: ::core::ffi::c_int) {
    pbuf_reserve(b, 1 as size_t);
    let fresh0 = (*b).len;
    (*b).len = (*b).len.wrapping_add(1);
    *(*b).data.offset(fresh0 as isize) = c as ::core::ffi::c_uchar;
}
#[inline]
unsafe extern "C" fn pbuf_puts(mut b: *mut pbuf, mut s: *const ::core::ffi::c_char) {
    pbuf_append(b, s as *const ::core::ffi::c_void, strlen(s));
}
#[inline]
unsafe extern "C" fn pbuf_free(mut b: *mut pbuf) {
    free((*b).data as *mut ::core::ffi::c_void);
    (*b).data = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    (*b).cap = 0 as size_t;
    (*b).len = (*b).cap;
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_new(mut out: *mut pbuf) -> *mut pdfw {
    let mut w: *mut pdfw = calloc(1 as size_t, ::core::mem::size_of::<pdfw>() as size_t)
        as *mut pdfw;
    if w.is_null() {
        abort();
    }
    (*w).object_streams=Box::into_raw(Box::new(pdf_objects::ObjectStreams::default()));
    (*w).out = out;
    (*w).count = 1 as ::core::ffi::c_int;
    (*w).compression = 6;
    (*w).requested_version = 17;
    (*w).minimum_version = 10;
    pbuf_puts(
        out,
        b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    return w;
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_free(mut w: *mut pdfw) {
    if w.is_null() {
        return;
    }
    drop(Box::from_raw((*w).object_streams));
    pbuf_free(&raw mut (*w).trailer);
    free((*w).offsets as *mut ::core::ffi::c_void);
    free(w as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_configuration(w: *mut pdfw, major: i32, minor: i32, compression: i32) {
    (*w).requested_version = major*10+minor;
    if (1..=2).contains(&major) && (0..=9).contains(&minor) && (*(*w).out).len >= 8 {
        *(*(*w).out).data.add(5) = b'0' + major as u8;
        *(*(*w).out).data.add(7) = b'0' + minor as u8;
    }
    (*w).compression = compression.clamp(0,9);
}

#[no_mangle]
pub unsafe extern "C" fn pdfw_object_compression(w:*mut pdfw,level:i32) {pdf_objects::ObjectStreams::configure(w,level);}

#[no_mangle]
pub unsafe extern "C" fn pdfw_minimum_version(w:*mut pdfw,major:i32,minor:i32) {
    (*w).minimum_version=(*w).minimum_version.max(major*10+minor);
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_effective_version(w:*mut pdfw)->i32 {
    let version=(*w).minimum_version.max((*w).requested_version);
    if (*(*w).out).len>=8 {*(*(*w).out).data.add(5)=b'0'+(version/10) as u8;*(*(*w).out).data.add(7)=b'0'+(version%10) as u8;}
    version
}
unsafe fn pitex_pdf_features(w:*mut pdfw,bytes:&[u8]) {
    let mut at=0usize;
    while at<bytes.len() {
        match bytes[at] {
            b'%' => {while at<bytes.len() && bytes[at]!=b'\n' {at+=1;}},
            b'(' => {
                at+=1;let mut depth=1;
                while at<bytes.len() && depth>0 {
                    if bytes[at]==b'\\' {at=(at+2).min(bytes.len());continue;}
                    if bytes[at]==b'(' {depth+=1;}else if bytes[at]==b')' {depth-=1;}at+=1;
                }
            }
            b'<' if bytes.get(at+1)==Some(&b'<') => {at+=2;},
            b'<' if bytes.get(at+1)!=Some(&b'<') => {while at<bytes.len() && bytes[at]!=b'>' {at+=1;}at=(at+1).min(bytes.len());},
            b'/' => {
                at+=1;let start=at;
                while at<bytes.len() && !bytes[at].is_ascii_whitespace() && !b"/()<>[]{}%".contains(&bytes[at]) {at+=1;}
                let mut decoded=Vec::new();let mut pos=start;
                while pos<at {
                    if bytes[pos]==b'#' && pos+2<at {
                        let high=(bytes[pos+1] as char).to_digit(16);let low=(bytes[pos+2] as char).to_digit(16);
                        if let (Some(high),Some(low))=(high,low) {decoded.push((high*16+low) as u8);pos+=3;continue;}
                    }
                    decoded.push(bytes[pos]);pos+=1;
                }
                let name=std::str::from_utf8(&decoded).unwrap_or("");
                let version=match name {
                    "ActualText"|"OCGs"|"OCProperties"=>15,
                    "SMask"|"BM"=>14,
                    "CA"|"ca"=>{
                        let mut next=at;while next<bytes.len()&&bytes[next].is_ascii_whitespace(){next+=1;}
                        if bytes.get(next).is_some_and(|b| b.is_ascii_digit()||b".+-".contains(b)){14}else{10}
                    },
                    "UserUnit"|"OpenType"|"3D"|"GoToE"=>16,
                    "Collection"|"AF"|"RichMedia"=>17,
                    "ToUnicode"|"FlateDecode"|"Names"|"AcroForm"=>12,
                    "CIDFontType2"|"EmbeddedFiles"=>13,
                    _=>10
                };
                (*w).minimum_version=(*w).minimum_version.max(version);
            }
            _=>{at+=1;}
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_trailer(w: *mut pdfw, body: *const ::core::ffi::c_char, length: usize) {
    pbuf_putc(&raw mut (*w).trailer,b' ' as i32);
    pbuf_append(&raw mut (*w).trailer,body.cast(),length);
}
unsafe fn pitex_decode_deflated(data: *const u8, length: usize) -> Option<Vec<u8>> {
    let mut size = length.saturating_mul(3).saturating_add(1024);
    loop {
        let mut bytes = vec![0u8;size];
        let mut actual = size as ::core::ffi::c_ulong;
        let result = uncompress(bytes.as_mut_ptr(),&mut actual,data,length as ::core::ffi::c_ulong);
        if result==0 { bytes.truncate(actual as usize); return Some(bytes); }
        if result != -5 { return None; }
        size = size.checked_mul(2)?;
    }
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_out(mut w: *mut pdfw) -> *mut pbuf {
    return (*w).out;
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_alloc(mut w: *mut pdfw) -> ::core::ffi::c_int {
    if (*w).count >= (*w).cap {
        (*w).cap = if (*w).cap != 0 {
            (*w).cap * 2 as ::core::ffi::c_int
        } else {
            256 as ::core::ffi::c_int
        };
        (*w).offsets = realloc(
            (*w).offsets as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<size_t>() as size_t).wrapping_mul((*w).cap as size_t),
        ) as *mut size_t;
        if (*w).offsets.is_null() {
            abort();
        }
    }
    *(*w).offsets.offset((*w).count as isize) = 0 as size_t;
    let fresh1 = (*w).count;
    (*w).count = (*w).count + 1;
    return fresh1;
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_begin(mut w: *mut pdfw, mut num: ::core::ffi::c_int) {
    *(*w).offsets.offset(num as isize) = (*(*w).out).len;
    (*w).object_start=(*(*w).out).len;
    pbuf_printf(
        (*w).out,
        b"%d 0 obj\n\0" as *const u8 as *const ::core::ffi::c_char,
        num,
    );
    pdf_objects::ObjectStreams::begin(w,num,(*w).object_start,(*(*w).out).len);
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_end(mut w: *mut pdfw) {
    let bytes=std::slice::from_raw_parts((*(*w).out).data.add((*w).object_start),(*(*w).out).len-(*w).object_start);
    let length=bytes.windows(8).position(|part|part==b"\nstream\n").unwrap_or(bytes.len());
    pitex_pdf_features(w,&bytes[..length]);
    pbuf_puts((*w).out, b"\nendobj\n\0" as *const u8 as *const ::core::ffi::c_char);
    pdf_objects::ObjectStreams::end(w,(*(*w).out).len);
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_stream_deflated(
    mut w: *mut pdfw,
    mut num: ::core::ffi::c_int,
    mut dict_body: *const ::core::ffi::c_char,
    mut data: *const ::core::ffi::c_void,
    mut len: size_t,
) {
    let mut decoded = None;
    let mut recoded = None;
    let mut deflated = true;
    if (*w).compression != 6 {
        decoded = pitex_decode_deflated(data.cast(),len);
        if let Some(raw) = decoded.as_ref() {
            if (*w).compression==0 { data=raw.as_ptr().cast();len=raw.len();deflated=false; }
            else {
                let bound = compressBound(raw.len() as ::core::ffi::c_ulong);
                let mut bytes=vec![0u8;bound as usize];let mut actual=bound;
                if compress2(bytes.as_mut_ptr(),&mut actual,raw.as_ptr(),raw.len() as ::core::ffi::c_ulong,(*w).compression)==0 {
                    bytes.truncate(actual as usize);recoded=Some(bytes);
                }
                if let Some(bytes)=recoded.as_ref() {data=bytes.as_ptr().cast();len=bytes.len();}
            }
        }
    }
    pdfw_begin(w, num);
    pdf_objects::ObjectStreams::stream(w);
    pbuf_printf(
        (*w).out,
        if deflated { b"<<%s/Filter/FlateDecode/Length %zu>>\nstream\n\0".as_ptr().cast() }
        else { b"<<%s/Length %zu>>\nstream\n\0".as_ptr().cast() },
        if !dict_body.is_null() {
            dict_body
        } else {
            b"\0" as *const u8 as *const ::core::ffi::c_char
        },
        len,
    );
    pbuf_append((*w).out, data, len);
    pbuf_puts((*w).out, b"\nendstream\0" as *const u8 as *const ::core::ffi::c_char);
    pdfw_end(w);
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_stream(
    mut w: *mut pdfw,
    mut num: ::core::ffi::c_int,
    mut dict_body: *const ::core::ffi::c_char,
    mut data: *const ::core::ffi::c_void,
    mut len: size_t,
    mut compress: bool,
) {
    let dictionary=if dict_body.is_null(){"".into()}else{std::ffi::CStr::from_ptr(dict_body).to_string_lossy()};
    if len>0 && (dictionary.is_empty() || dictionary.contains("/Subtype/Form") || dictionary.contains("/Subtype /Form")) {
        let raw=std::slice::from_raw_parts(data.cast::<u8>(),len);
        pitex_pdf_features(w,raw);
    }
    if compress as ::core::ffi::c_int != 0 && (*w).compression > 0 && len > 64 as size_t {
        let mut z: pbuf = pbuf {
            data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
            len: 0,
            cap: 0,
        };
        if pbuf_deflate(
            &raw mut z,
            data as *const ::core::ffi::c_uchar,
            len,
            3 as ::core::ffi::c_int,
        ) == 0 as ::core::ffi::c_int
        {
            pdfw_stream_deflated(
                w,
                num,
                dict_body,
                z.data as *const ::core::ffi::c_void,
                z.len,
            );
            pbuf_free(&raw mut z);
            return;
        }
        pbuf_free(&raw mut z);
    }
    pdfw_begin(w, num);
    pdf_objects::ObjectStreams::stream(w);
    pbuf_printf(
        (*w).out,
        b"<<%s/Length %zu>>\nstream\n\0" as *const u8 as *const ::core::ffi::c_char,
        if !dict_body.is_null() {
            dict_body
        } else {
            b"\0" as *const u8 as *const ::core::ffi::c_char
        },
        len,
    );
    pbuf_append((*w).out, data, len);
    pbuf_puts((*w).out, b"\nendstream\0" as *const u8 as *const ::core::ffi::c_char);
    pdfw_end(w);
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_finish(
    mut w: *mut pdfw,
    mut root: ::core::ffi::c_int,
    mut info: ::core::ffi::c_int,
) {
    if pdf_objects::ObjectStreams::finish(w,root,info){return;}
    let mut xref: size_t = (*(*w).out).len;
    pbuf_printf(
        (*w).out,
        b"xref\n0 %d\n0000000000 65535 f \n\0" as *const u8
            as *const ::core::ffi::c_char,
        (*w).count,
    );
    let mut i: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while i < (*w).count {
        if *(*w).offsets.offset(i as isize) != 0 {
            pbuf_printf(
                (*w).out,
                b"%010zu 00000 n \n\0" as *const u8 as *const ::core::ffi::c_char,
                *(*w).offsets.offset(i as isize),
            );
        } else {
            pbuf_puts(
                (*w).out,
                b"0000000000 00000 f \n\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        i += 1;
    }
    pbuf_printf(
        (*w).out,
        b"trailer\n<</Size %d/Root %d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
        (*w).count,
        root,
    );
    if info > 0 as ::core::ffi::c_int {
        pbuf_printf(
            (*w).out,
            b"/Info %d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
            info,
        );
    }
    pbuf_append((*w).out,(*w).trailer.data.cast(),(*w).trailer.len);
    pbuf_printf(
        (*w).out,
        b">>\nstartxref\n%zu\n%%%%EOF\n\0" as *const u8 as *const ::core::ffi::c_char,
        xref,
    );
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_name(
    mut b: *mut pbuf,
    mut name: *const ::core::ffi::c_char,
) {
    pbuf_putc(b, '/' as i32);
    let mut p: *const ::core::ffi::c_uchar = name as *const ::core::ffi::c_uchar;
    while *p != 0 {
        if (*p as ::core::ffi::c_int) < 0x21 as ::core::ffi::c_int
            || *p as ::core::ffi::c_int > 0x7e as ::core::ffi::c_int
            || !strchr(
                    b"#()<>[]{}/%\0" as *const u8 as *const ::core::ffi::c_char,
                    *p as ::core::ffi::c_int,
                )
                .is_null()
        {
            pbuf_printf(
                b,
                b"#%02X\0" as *const u8 as *const ::core::ffi::c_char,
                *p as ::core::ffi::c_int,
            );
        } else {
            pbuf_putc(b, *p as ::core::ffi::c_int);
        }
        p = p.offset(1);
    }
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_hexstring(
    mut b: *mut pbuf,
    mut s: *const ::core::ffi::c_uchar,
    mut len: size_t,
) {
    static mut hex: [::core::ffi::c_char; 17] = unsafe {
        ::core::mem::transmute::<
            [u8; 17],
            [::core::ffi::c_char; 17],
        >(*b"0123456789ABCDEF\0")
    };
    pbuf_putc(b, '<' as i32);
    let mut i: size_t = 0 as size_t;
    while i < len {
        pbuf_putc(
            b,
            hex[(*s.offset(i as isize) as ::core::ffi::c_int >> 4 as ::core::ffi::c_int)
                as usize] as ::core::ffi::c_int,
        );
        pbuf_putc(
            b,
            hex[(*s.offset(i as isize) as ::core::ffi::c_int & 15 as ::core::ffi::c_int)
                as usize] as ::core::ffi::c_int,
        );
        i = i.wrapping_add(1);
    }
    pbuf_putc(b, '>' as i32);
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_import_begin(
    mut w: *mut pdfw,
    mut src: *mut pr_doc,
) -> *mut pdfw_import {
    let mut imp: *mut pdfw_import = calloc(
        1 as size_t,
        ::core::mem::size_of::<pdfw_import>() as size_t,
    ) as *mut pdfw_import;
    if imp.is_null() {
        abort();
    }
    (*imp).redirects=Box::into_raw(Box::new(std::collections::BTreeSet::new()));
    (*imp).w = w;
    (*imp).src = src;
    return imp;
}
unsafe extern "C" fn map_ref(
    mut imp: *mut pdfw_import,
    mut num: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if num < 0 as ::core::ffi::c_int || num > 8000000 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int;
    }
    if num >= (*imp).nmap {
        let mut n: ::core::ffi::c_int = if (*imp).nmap != 0 {
            (*imp).nmap
        } else {
            64 as ::core::ffi::c_int
        };
        while n <= num {
            n *= 2 as ::core::ffi::c_int;
        }
        (*imp).map = realloc(
            (*imp).map as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<::core::ffi::c_int>() as size_t)
                .wrapping_mul(n as size_t),
        ) as *mut ::core::ffi::c_int;
        if (*imp).map.is_null() {
            abort();
        }
        memset(
            (*imp).map.offset((*imp).nmap as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            (::core::mem::size_of::<::core::ffi::c_int>() as size_t)
                .wrapping_mul((n - (*imp).nmap) as size_t),
        );
        (*imp).nmap = n;
    }
    if *(*imp).map.offset(num as isize) == 0 {
        *(*imp).map.offset(num as isize) = pdfw_alloc((*imp).w);
        pdf_objects::ObjectStreams::imported((*imp).w,*(*imp).map.offset(num as isize));
        if (*imp).nq == (*imp).capq {
            (*imp).capq = if (*imp).capq != 0 {
                (*imp).capq * 2 as ::core::ffi::c_int
            } else {
                64 as ::core::ffi::c_int
            };
            (*imp).queue = realloc(
                (*imp).queue as *mut ::core::ffi::c_void,
                (::core::mem::size_of::<::core::ffi::c_int>() as size_t)
                    .wrapping_mul((*imp).capq as size_t),
            ) as *mut ::core::ffi::c_int;
            if (*imp).queue.is_null() {
                abort();
            }
        }
        let fresh2 = (*imp).nq;
        (*imp).nq = (*imp).nq + 1;
        *(*imp).queue.offset(fresh2 as isize) = num;
    }
    return *(*imp).map.offset(num as isize);
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_import_redirect(imp:*mut pdfw_import,source:i32,destination:i32) {
    map_ref(imp,source);
    if source>=0&&source<(*imp).nmap {(*imp).map.add(source as usize).write(destination);(*(*imp).redirects).insert(source);}
}
unsafe extern "C" fn skip_key(mut k: *const ::core::ffi::c_char) -> bool {
    return strcmp(k, b"Parent\0" as *const u8 as *const ::core::ffi::c_char) == 0
        || strcmp(k, b"PieceInfo\0" as *const u8 as *const ::core::ffi::c_char) == 0
        || strcmp(k, b"Metadata\0" as *const u8 as *const ::core::ffi::c_char) == 0
        || strcmp(k, b"Thumb\0" as *const u8 as *const ::core::ffi::c_char) == 0
        || strcmp(k, b"StructParent\0" as *const u8 as *const ::core::ffi::c_char) == 0
        || strcmp(k, b"StructParents\0" as *const u8 as *const ::core::ffi::c_char) == 0
        || strcmp(k, b"Annots\0" as *const u8 as *const ::core::ffi::c_char) == 0
        || strcmp(k, b"B\0" as *const u8 as *const ::core::ffi::c_char) == 0;
}
unsafe extern "C" fn import_dict_body(
    mut imp: *mut pdfw_import,
    mut b: *mut pbuf,
    mut d: *mut pr_obj,
    mut stream: bool,
) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*d).u.dict.n {
        let mut k: *const ::core::ffi::c_char = *(*d).u.dict.keys.offset(i as isize);
        if !skip_key(k) {
            if !(stream as ::core::ffi::c_int != 0
                && strcmp(k, b"Length\0" as *const u8 as *const ::core::ffi::c_char)
                    == 0)
            {
                pdfw_name(b, k);
                pbuf_putc(b, ' ' as i32);
                pdfw_import_value(imp, b, *(*d).u.dict.vals.offset(i as isize));
            }
        }
        i += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_import_value(
    mut imp: *mut pdfw_import,
    mut b: *mut pbuf,
    mut o: *mut pr_obj,
) {
    if o.is_null() {
        pbuf_puts(b, b"null\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    match (*o).type_0 as ::core::ffi::c_uint {
        0 => {
            pbuf_puts(b, b"null\0" as *const u8 as *const ::core::ffi::c_char);
        }
        1 => {
            pbuf_puts(
                b,
                if (*o).u.b != 0 {
                    b"true\0" as *const u8 as *const ::core::ffi::c_char
                } else {
                    b"false\0" as *const u8 as *const ::core::ffi::c_char
                },
            );
        }
        2 => {
            pbuf_printf(
                b,
                b"%lld\0" as *const u8 as *const ::core::ffi::c_char,
                (*o).u.i,
            );
        }
        3 => {
            pbuf_real(b, (*o).u.r);
        }
        4 => {
            pdfw_name(b, (*o).u.str_0.s);
        }
        5 => {
            pdfw_hexstring(
                b,
                (*o).u.str_0.s as *mut ::core::ffi::c_uchar,
                (*o).u.str_0.len,
            );
        }
        6 => {
            pbuf_putc(b, '[' as i32);
            let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while i < (*o).u.arr.n {
                if i != 0 {
                    pbuf_putc(b, ' ' as i32);
                }
                pdfw_import_value(imp, b, *(*o).u.arr.items.offset(i as isize));
                i += 1;
            }
            pbuf_putc(b, ']' as i32);
        }
        7 => {
            pbuf_puts(b, b"<<\0" as *const u8 as *const ::core::ffi::c_char);
            import_dict_body(imp, b, o, false_0 != 0);
            pbuf_puts(b, b">>\0" as *const u8 as *const ::core::ffi::c_char);
        }
        8 => {
            let mut t: *mut pr_obj = pr_load((*imp).src, (*o).u.ref_0.num);
            if t.is_null()
                || (*t).type_0 as ::core::ffi::c_uint
                    == PR_NULL as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                pbuf_puts(b, b"null\0" as *const u8 as *const ::core::ffi::c_char);
            } else {
                pbuf_printf(
                    b,
                    b"%d 0 R\0" as *const u8 as *const ::core::ffi::c_char,
                    map_ref(imp, (*o).u.ref_0.num),
                );
            }
        }
        9 => {
            pbuf_puts(b, b"null\0" as *const u8 as *const ::core::ffi::c_char);
        }
        _ => {}
    };
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_import_flush(mut imp: *mut pdfw_import) {
    while (*imp).written < (*imp).nq {
        let fresh3 = (*imp).written;
        (*imp).written = (*imp).written + 1;
        let mut src: ::core::ffi::c_int = *(*imp).queue.offset(fresh3 as isize);
        if (*(*imp).redirects).contains(&src){continue;}
        let mut dst: ::core::ffi::c_int = *(*imp).map.offset(src as isize);
        let mut o: *mut pr_obj = pr_load((*imp).src, src);
        let mut body: pbuf = pbuf {
            data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
            len: 0,
            cap: 0,
        };
        if !o.is_null()
            && (*o).type_0 as ::core::ffi::c_uint
                == PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            let mut len: size_t = 0 as size_t;
            let mut raw: *const ::core::ffi::c_uchar = pr_stream_raw(
                (*imp).src,
                o,
                &raw mut len,
            );
            import_dict_body(imp, &raw mut body, (*o).u.stream.dict, true_0 != 0);
            pbuf_putc(&raw mut body, 0 as ::core::ffi::c_int);
            pdfw_begin((*imp).w, dst);
            pbuf_printf(
                (*(*imp).w).out,
                b"<<%s/Length %zu>>\nstream\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                body.data as *mut ::core::ffi::c_char,
                len,
            );
            pbuf_append((*(*imp).w).out, raw as *const ::core::ffi::c_void, len);
            pbuf_puts(
                (*(*imp).w).out,
                b"\nendstream\0" as *const u8 as *const ::core::ffi::c_char,
            );
            pdfw_end((*imp).w);
        } else {
            pdfw_import_value(imp, &raw mut body, o);
            pdfw_begin((*imp).w, dst);
            pbuf_append(
                (*(*imp).w).out,
                body.data as *const ::core::ffi::c_void,
                body.len,
            );
            pdfw_end((*imp).w);
        }
        pbuf_free(&raw mut body);
    }
}
#[no_mangle]
pub unsafe extern "C" fn pdfw_import_end(mut imp: *mut pdfw_import) {
    if imp.is_null() {
        return;
    }
    pdfw_import_flush(imp);
    drop(Box::from_raw((*imp).redirects));
    free((*imp).map as *mut ::core::ffi::c_void);
    free((*imp).queue as *mut ::core::ffi::c_void);
    free(imp as *mut ::core::ffi::c_void);
}
