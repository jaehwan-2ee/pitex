/* Pitex embedded preview engine — minimal PDF reader.
 * Independently written from the PDF 1.7 / ISO 32000 specification.
 * Pitex-authored (AGPL-3.0-or-later, see repository LICENSE). */
// Translated from shared/pdfread.c with C2Rust 0.22.1.
extern "C" {
    fn strtod(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn strtoll(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_longlong;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn abs(__x: ::core::ffi::c_int) -> ::core::ffi::c_int;
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
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn pbuf_inflate(
        out: *mut pbuf,
        data: *const ::core::ffi::c_uchar,
        len: size_t,
    ) -> ::core::ffi::c_int;
    fn __ctype_b_loc() -> *mut *const ::core::ffi::c_ushort;
    fn fmax(
        __x: ::core::ffi::c_double,
        __y: ::core::ffi::c_double,
    ) -> ::core::ffi::c_double;
    fn fmin(
        __x: ::core::ffi::c_double,
        __y: ::core::ffi::c_double,
    ) -> ::core::ffi::c_double;
}
pub type size_t = usize;
pub type __uint32_t = u32;
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
pub struct pr_doc {
    pub data: *const ::core::ffi::c_uchar,
    pub len: size_t,
    pub arena: *mut arena_chunk,
    pub xref: *mut xref_entry,
    pub nxref: ::core::ffi::c_int,
    pub trailer: *mut pr_obj,
    pub pages: *mut page_entry,
    pub npages: ::core::ffi::c_int,
    pub cappages: ::core::ffi::c_int,
    pub pages_loaded: bool,
    pub resolve_budget: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct page_entry {
    pub dict: *mut pr_obj,
    pub resources: *mut pr_obj,
    pub mediabox: *mut pr_obj,
    pub cropbox: *mut pr_obj,
    pub rotate: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xref_entry {
    pub type_0: ::core::ffi::c_uchar,
    pub loading: ::core::ffi::c_uchar,
    pub off: size_t,
    pub idx: ::core::ffi::c_int,
    pub obj: *mut pr_obj,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct arena_chunk {
    pub next: *mut arena_chunk,
    pub used: size_t,
    pub cap: size_t,
    pub data: [::core::ffi::c_uchar; 0],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pr_page_info {
    pub box_0: [::core::ffi::c_double; 4],
    pub rotate: ::core::ffi::c_int,
    pub page: *mut pr_obj,
    pub resources: *mut pr_obj,
}
pub type C2RustUnnamed_5 = ::core::ffi::c_uint;
pub const PR_BOX_ART: C2RustUnnamed_5 = 5;
pub const PR_BOX_TRIM: C2RustUnnamed_5 = 4;
pub const PR_BOX_BLEED: C2RustUnnamed_5 = 3;
pub const PR_BOX_MEDIA: C2RustUnnamed_5 = 2;
pub const PR_BOX_CROP: C2RustUnnamed_5 = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct lexer {
    pub p: *const ::core::ffi::c_uchar,
    pub pos: size_t,
    pub len: size_t,
}
pub const _ISdigit: C2RustUnnamed_6 = 2048;
pub type uint32_t = __uint32_t;
pub type C2RustUnnamed_6 = ::core::ffi::c_uint;
pub const _ISalnum: C2RustUnnamed_6 = 8;
pub const _ISpunct: C2RustUnnamed_6 = 4;
pub const _IScntrl: C2RustUnnamed_6 = 2;
pub const _ISblank: C2RustUnnamed_6 = 1;
pub const _ISgraph: C2RustUnnamed_6 = 32768;
pub const _ISprint: C2RustUnnamed_6 = 16384;
pub const _ISspace: C2RustUnnamed_6 = 8192;
pub const _ISxdigit: C2RustUnnamed_6 = 4096;
pub const _ISalpha: C2RustUnnamed_6 = 1024;
pub const _ISlower: C2RustUnnamed_6 = 512;
pub const _ISupper: C2RustUnnamed_6 = 256;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
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
unsafe extern "C" fn pbuf_free(mut b: *mut pbuf) {
    free((*b).data as *mut ::core::ffi::c_void);
    (*b).data = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    (*b).cap = 0 as size_t;
    (*b).len = (*b).cap;
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const MAX_DEPTH: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const MAX_OBJECTS: ::core::ffi::c_int = 8000000 as ::core::ffi::c_int;
unsafe extern "C" fn arena_alloc(
    mut d: *mut pr_doc,
    mut n: size_t,
) -> *mut ::core::ffi::c_void {
    n = n.wrapping_add(15 as size_t) & !(15 as ::core::ffi::c_int as size_t);
    let mut c: *mut arena_chunk = (*d).arena;
    if c.is_null() || (*c).used.wrapping_add(n) > (*c).cap {
        let mut cap: size_t = if n > 65536 as ::core::ffi::c_int as size_t {
            n
        } else {
            65536 as ::core::ffi::c_int as size_t
        };
        c = malloc((::core::mem::size_of::<arena_chunk>() as size_t).wrapping_add(cap))
            as *mut arena_chunk;
        if c.is_null() {
            abort();
        }
        (*c).next = (*d).arena as *mut arena_chunk;
        (*c).used = 0 as size_t;
        (*c).cap = cap;
        (*d).arena = c;
    }
    let mut p: *mut ::core::ffi::c_void = (&raw mut (*c).data
        as *mut ::core::ffi::c_uchar)
        .offset((*c).used as isize) as *mut ::core::ffi::c_void;
    (*c).used = (*c).used.wrapping_add(n);
    memset(p, 0 as ::core::ffi::c_int, n);
    return p;
}
unsafe extern "C" fn new_obj(mut d: *mut pr_doc, mut t: pr_type) -> *mut pr_obj {
    let mut o: *mut pr_obj = arena_alloc(d, ::core::mem::size_of::<pr_obj>() as size_t)
        as *mut pr_obj;
    (*o).type_0 = t;
    return o;
}
static mut null_obj: pr_obj = pr_obj {
    type_0: PR_NULL,
    u: C2RustUnnamed { b: 0 },
};
unsafe extern "C" fn is_ws(mut c: ::core::ffi::c_int) -> bool {
    return c == 0 as ::core::ffi::c_int || c == '\t' as i32 || c == '\n' as i32
        || c == '\u{c}' as i32 || c == '\r' as i32 || c == ' ' as i32;
}
unsafe extern "C" fn is_delim(mut c: ::core::ffi::c_int) -> bool {
    return c == '(' as i32 || c == ')' as i32 || c == '<' as i32 || c == '>' as i32
        || c == '[' as i32 || c == ']' as i32 || c == '{' as i32 || c == '}' as i32
        || c == '/' as i32 || c == '%' as i32;
}
unsafe extern "C" fn is_regular(mut c: ::core::ffi::c_int) -> bool {
    return !is_ws(c) && !is_delim(c);
}
unsafe extern "C" fn skip_ws(mut lx: *mut lexer) {
    while (*lx).pos < (*lx).len {
        let mut c: ::core::ffi::c_int = *(*lx).p.offset((*lx).pos as isize)
            as ::core::ffi::c_int;
        if is_ws(c) {
            (*lx).pos = (*lx).pos.wrapping_add(1);
        } else {
            if !(c == '%' as i32) {
                break;
            }
            while (*lx).pos < (*lx).len
                && *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int
                    != '\n' as i32
                && *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int
                    != '\r' as i32
            {
                (*lx).pos = (*lx).pos.wrapping_add(1);
            }
        }
    }
}
unsafe extern "C" fn at_keyword(
    mut lx: *mut lexer,
    mut kw: *const ::core::ffi::c_char,
) -> bool {
    let mut n: size_t = strlen(kw);
    if (*lx).pos.wrapping_add(n) > (*lx).len
        || memcmp(
            (*lx).p.offset((*lx).pos as isize) as *const ::core::ffi::c_void,
            kw as *const ::core::ffi::c_void,
            n,
        ) != 0 as ::core::ffi::c_int
    {
        return false_0 != 0;
    }
    if (*lx).pos.wrapping_add(n) < (*lx).len
        && is_regular(
            *(*lx).p.offset((*lx).pos.wrapping_add(n) as isize) as ::core::ffi::c_int,
        ) as ::core::ffi::c_int != 0
    {
        return false_0 != 0;
    }
    return true_0 != 0;
}
unsafe extern "C" fn peek_uint(
    mut lx: *mut lexer,
    mut at: size_t,
    mut out: *mut ::core::ffi::c_longlong,
    mut end: *mut size_t,
) -> bool {
    let mut i: size_t = at;
    let mut v: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    if i >= (*lx).len
        || *(*__ctype_b_loc())
            .offset(*(*lx).p.offset(i as isize) as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int
            & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort
                as ::core::ffi::c_int == 0
    {
        return false_0 != 0;
    }
    while i < (*lx).len
        && *(*__ctype_b_loc())
            .offset(*(*lx).p.offset(i as isize) as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int
            & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort
                as ::core::ffi::c_int != 0
    {
        v = v * 10 as ::core::ffi::c_longlong
            + (*(*lx).p.offset(i as isize) as ::core::ffi::c_int - '0' as i32)
                as ::core::ffi::c_longlong;
        if v > (1 as ::core::ffi::c_longlong) << 40 as ::core::ffi::c_int {
            return false_0 != 0;
        }
        i = i.wrapping_add(1);
    }
    if i < (*lx).len
        && is_regular(*(*lx).p.offset(i as isize) as ::core::ffi::c_int)
            as ::core::ffi::c_int != 0
    {
        return false_0 != 0;
    }
    *out = v;
    *end = i;
    return true_0 != 0;
}
unsafe extern "C" fn parse_number(
    mut d: *mut pr_doc,
    mut lx: *mut lexer,
) -> *mut pr_obj {
    let mut start: size_t = (*lx).pos;
    let mut real: bool = false_0 != 0;
    if (*lx).pos < (*lx).len
        && (*(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int == '+' as i32
            || *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int == '-' as i32)
    {
        (*lx).pos = (*lx).pos.wrapping_add(1);
    }
    while (*lx).pos < (*lx).len
        && (*(*__ctype_b_loc())
            .offset(*(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int
            & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort
                as ::core::ffi::c_int != 0
            || *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int == '.' as i32)
    {
        if *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int == '.' as i32 {
            real = true_0 != 0;
        }
        (*lx).pos = (*lx).pos.wrapping_add(1);
    }
    while (*lx).pos < (*lx).len
        && is_regular(*(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int)
            as ::core::ffi::c_int != 0
    {
        (*lx).pos = (*lx).pos.wrapping_add(1);
    }
    let mut tmp: [::core::ffi::c_char; 64] = [0; 64];
    let mut n: size_t = (*lx).pos.wrapping_sub(start);
    if n >= ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as usize {
        n = (::core::mem::size_of::<[::core::ffi::c_char; 64]>() as usize)
            .wrapping_sub(1 as usize) as size_t;
    }
    memcpy(
        &raw mut tmp as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        (*lx).p.offset(start as isize) as *const ::core::ffi::c_void,
        n,
    );
    tmp[n as usize] = 0 as ::core::ffi::c_char;
    if real {
        let mut o: *mut pr_obj = new_obj(d, PR_REAL);
        (*o).u.r = strtod(
            &raw mut tmp as *mut ::core::ffi::c_char,
            ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
        );
        return o;
    }
    let mut o_0: *mut pr_obj = new_obj(d, PR_INT);
    (*o_0).u.i = strtoll(
        &raw mut tmp as *mut ::core::ffi::c_char,
        ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
        10 as ::core::ffi::c_int,
    );
    return o_0;
}
unsafe extern "C" fn hexval(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    if c >= '0' as i32 && c <= '9' as i32 {
        return c - '0' as i32;
    }
    if c >= 'a' as i32 && c <= 'f' as i32 {
        return c - 'a' as i32 + 10 as ::core::ffi::c_int;
    }
    if c >= 'A' as i32 && c <= 'F' as i32 {
        return c - 'A' as i32 + 10 as ::core::ffi::c_int;
    }
    return -(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn parse_name(mut d: *mut pr_doc, mut lx: *mut lexer) -> *mut pr_obj {
    (*lx).pos = (*lx).pos.wrapping_add(1);
    let mut start: size_t = (*lx).pos;
    while (*lx).pos < (*lx).len
        && is_regular(*(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int)
            as ::core::ffi::c_int != 0
    {
        (*lx).pos = (*lx).pos.wrapping_add(1);
    }
    let mut n: size_t = (*lx).pos.wrapping_sub(start);
    let mut o: *mut pr_obj = new_obj(d, PR_NAME);
    let mut s: *mut ::core::ffi::c_char = arena_alloc(d, n.wrapping_add(1 as size_t))
        as *mut ::core::ffi::c_char;
    let mut j: size_t = 0 as size_t;
    let mut current_block_5: u64;
    let mut i: size_t = 0 as size_t;
    while i < n {
        let mut c: ::core::ffi::c_int = *(*lx).p.offset(start.wrapping_add(i) as isize)
            as ::core::ffi::c_int;
        if c == '#' as i32 && i.wrapping_add(2 as size_t) < n {
            let mut h1: ::core::ffi::c_int = hexval(
                *(*lx).p.offset(start.wrapping_add(i).wrapping_add(1 as size_t) as isize)
                    as ::core::ffi::c_int,
            );
            let mut h2: ::core::ffi::c_int = hexval(
                *(*lx).p.offset(start.wrapping_add(i).wrapping_add(2 as size_t) as isize)
                    as ::core::ffi::c_int,
            );
            if h1 >= 0 as ::core::ffi::c_int && h2 >= 0 as ::core::ffi::c_int {
                let fresh10 = j;
                j = j.wrapping_add(1);
                *s.offset(fresh10 as isize) = (h1 * 16 as ::core::ffi::c_int + h2)
                    as ::core::ffi::c_char;
                i = i.wrapping_add(2 as size_t);
                current_block_5 = 820271813250567934;
            } else {
                current_block_5 = 13536709405535804910;
            }
        } else {
            current_block_5 = 13536709405535804910;
        }
        match current_block_5 {
            13536709405535804910 => {
                let fresh11 = j;
                j = j.wrapping_add(1);
                *s.offset(fresh11 as isize) = c as ::core::ffi::c_char;
            }
            _ => {}
        }
        i = i.wrapping_add(1);
    }
    *s.offset(j as isize) = 0 as ::core::ffi::c_char;
    (*o).u.str_0.s = s;
    (*o).u.str_0.len = j;
    return o;
}
unsafe extern "C" fn parse_literal_string(
    mut d: *mut pr_doc,
    mut lx: *mut lexer,
) -> *mut pr_obj {
    (*lx).pos = (*lx).pos.wrapping_add(1);
    let mut tmp: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut nest: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    while (*lx).pos < (*lx).len {
        let fresh12 = (*lx).pos;
        (*lx).pos = (*lx).pos.wrapping_add(1);
        let mut c: ::core::ffi::c_int = *(*lx).p.offset(fresh12 as isize)
            as ::core::ffi::c_int;
        if c == '(' as i32 {
            nest += 1;
            pbuf_putc(&raw mut tmp, c);
        } else if c == ')' as i32 {
            nest -= 1;
            if nest == 0 as ::core::ffi::c_int {
                break;
            }
            pbuf_putc(&raw mut tmp, c);
        } else if c == '\\' as i32 && (*lx).pos < (*lx).len {
            let fresh13 = (*lx).pos;
            (*lx).pos = (*lx).pos.wrapping_add(1);
            let mut e: ::core::ffi::c_int = *(*lx).p.offset(fresh13 as isize)
                as ::core::ffi::c_int;
            match e {
                110 => {
                    pbuf_putc(&raw mut tmp, '\n' as i32);
                }
                114 => {
                    pbuf_putc(&raw mut tmp, '\r' as i32);
                }
                116 => {
                    pbuf_putc(&raw mut tmp, '\t' as i32);
                }
                98 => {
                    pbuf_putc(&raw mut tmp, '\u{8}' as i32);
                }
                102 => {
                    pbuf_putc(&raw mut tmp, '\u{c}' as i32);
                }
                13 => {
                    if (*lx).pos < (*lx).len
                        && *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int
                            == '\n' as i32
                    {
                        (*lx).pos = (*lx).pos.wrapping_add(1);
                    }
                }
                10 => {}
                _ => {
                    if e >= '0' as i32 && e <= '7' as i32 {
                        let mut v: ::core::ffi::c_int = e - '0' as i32;
                        let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while k < 2 as ::core::ffi::c_int && (*lx).pos < (*lx).len
                            && *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int
                                >= '0' as i32
                            && *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int
                                <= '7' as i32
                        {
                            let fresh14 = (*lx).pos;
                            (*lx).pos = (*lx).pos.wrapping_add(1);
                            v = v * 8 as ::core::ffi::c_int
                                + (*(*lx).p.offset(fresh14 as isize) as ::core::ffi::c_int
                                    - '0' as i32);
                            k += 1;
                        }
                        pbuf_putc(&raw mut tmp, v & 0xff as ::core::ffi::c_int);
                    } else {
                        pbuf_putc(&raw mut tmp, e);
                    }
                }
            }
        } else {
            pbuf_putc(&raw mut tmp, c);
        }
    }
    let mut o: *mut pr_obj = new_obj(d, PR_STRING);
    (*o).u.str_0.s = arena_alloc(d, tmp.len.wrapping_add(1 as size_t))
        as *mut ::core::ffi::c_char;
    if tmp.len != 0 {
        memcpy(
            (*o).u.str_0.s as *mut ::core::ffi::c_void,
            tmp.data as *const ::core::ffi::c_void,
            tmp.len,
        );
    }
    (*o).u.str_0.len = tmp.len;
    pbuf_free(&raw mut tmp);
    return o;
}
unsafe extern "C" fn parse_hex_string(
    mut d: *mut pr_doc,
    mut lx: *mut lexer,
) -> *mut pr_obj {
    (*lx).pos = (*lx).pos.wrapping_add(1);
    let mut tmp: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut hi: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    while (*lx).pos < (*lx).len {
        let fresh7 = (*lx).pos;
        (*lx).pos = (*lx).pos.wrapping_add(1);
        let mut c: ::core::ffi::c_int = *(*lx).p.offset(fresh7 as isize)
            as ::core::ffi::c_int;
        if c == '>' as i32 {
            break;
        }
        let mut v: ::core::ffi::c_int = hexval(c);
        if v < 0 as ::core::ffi::c_int {
            continue;
        }
        if hi < 0 as ::core::ffi::c_int {
            hi = v;
        } else {
            pbuf_putc(&raw mut tmp, hi * 16 as ::core::ffi::c_int + v);
            hi = -(1 as ::core::ffi::c_int);
        }
    }
    if hi >= 0 as ::core::ffi::c_int {
        pbuf_putc(&raw mut tmp, hi * 16 as ::core::ffi::c_int);
    }
    let mut o: *mut pr_obj = new_obj(d, PR_STRING);
    (*o).u.str_0.s = arena_alloc(d, tmp.len.wrapping_add(1 as size_t))
        as *mut ::core::ffi::c_char;
    if tmp.len != 0 {
        memcpy(
            (*o).u.str_0.s as *mut ::core::ffi::c_void,
            tmp.data as *const ::core::ffi::c_void,
            tmp.len,
        );
    }
    (*o).u.str_0.len = tmp.len;
    pbuf_free(&raw mut tmp);
    return o;
}
unsafe extern "C" fn parse_array(
    mut d: *mut pr_doc,
    mut lx: *mut lexer,
    mut depth: ::core::ffi::c_int,
) -> *mut pr_obj {
    (*lx).pos = (*lx).pos.wrapping_add(1);
    let mut cap: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut items: *mut *mut pr_obj = malloc(
        (::core::mem::size_of::<*mut pr_obj>() as size_t).wrapping_mul(cap as size_t),
    ) as *mut *mut pr_obj;
    if items.is_null() {
        abort();
    }
    loop {
        skip_ws(lx);
        if (*lx).pos >= (*lx).len {
            break;
        }
        if *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int == ']' as i32 {
            (*lx).pos = (*lx).pos.wrapping_add(1);
            break;
        } else {
            let mut before: size_t = (*lx).pos;
            let mut item: *mut pr_obj = parse_obj(
                d,
                lx,
                depth + 1 as ::core::ffi::c_int,
            );
            if (*lx).pos == before {
                (*lx).pos = (*lx).pos.wrapping_add(1);
            } else {
                if n == cap {
                    cap *= 2 as ::core::ffi::c_int;
                    items = realloc(
                        items as *mut ::core::ffi::c_void,
                        (::core::mem::size_of::<*mut pr_obj>() as size_t)
                            .wrapping_mul(cap as size_t),
                    ) as *mut *mut pr_obj;
                    if items.is_null() {
                        abort();
                    }
                }
                let fresh5 = n;
                n = n + 1;
                let ref mut fresh6 = *items.offset(fresh5 as isize);
                *fresh6 = item;
            }
        }
    }
    let mut o: *mut pr_obj = new_obj(d, PR_ARRAY);
    (*o).u.arr.items = arena_alloc(
        d,
        (::core::mem::size_of::<*mut pr_obj>() as size_t)
            .wrapping_mul((if n != 0 { n } else { 1 as ::core::ffi::c_int }) as size_t),
    ) as *mut *mut pr_obj;
    memcpy(
        (*o).u.arr.items as *mut ::core::ffi::c_void,
        items as *const ::core::ffi::c_void,
        (::core::mem::size_of::<*mut pr_obj>() as size_t).wrapping_mul(n as size_t),
    );
    (*o).u.arr.n = n;
    free(items as *mut ::core::ffi::c_void);
    return o;
}
unsafe extern "C" fn parse_dict(
    mut d: *mut pr_doc,
    mut lx: *mut lexer,
    mut depth: ::core::ffi::c_int,
) -> *mut pr_obj {
    (*lx).pos = (*lx).pos.wrapping_add(2 as size_t);
    let mut cap: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
    let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut keys: *mut *mut ::core::ffi::c_char = malloc(
        (::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t)
            .wrapping_mul(cap as size_t),
    ) as *mut *mut ::core::ffi::c_char;
    let mut vals: *mut *mut pr_obj = malloc(
        (::core::mem::size_of::<*mut pr_obj>() as size_t).wrapping_mul(cap as size_t),
    ) as *mut *mut pr_obj;
    if keys.is_null() || vals.is_null() {
        abort();
    }
    loop {
        skip_ws(lx);
        if (*lx).pos >= (*lx).len {
            break;
        }
        if *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int == '>' as i32
            && (*lx).pos.wrapping_add(1 as size_t) < (*lx).len
            && *(*lx).p.offset((*lx).pos.wrapping_add(1 as size_t) as isize)
                as ::core::ffi::c_int == '>' as i32
        {
            (*lx).pos = (*lx).pos.wrapping_add(2 as size_t);
            break;
        } else if *(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int != '/' as i32
        {
            let mut before: size_t = (*lx).pos;
            parse_obj(d, lx, depth + 1 as ::core::ffi::c_int);
            if (*lx).pos == before {
                (*lx).pos = (*lx).pos.wrapping_add(1);
            }
        } else {
            let mut key: *mut pr_obj = parse_name(d, lx);
            skip_ws(lx);
            let mut val: *mut pr_obj = &raw mut null_obj;
            if (*lx).pos < (*lx).len
                && !(*(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int
                    == '>' as i32 && (*lx).pos.wrapping_add(1 as size_t) < (*lx).len
                    && *(*lx).p.offset((*lx).pos.wrapping_add(1 as size_t) as isize)
                        as ::core::ffi::c_int == '>' as i32)
            {
                val = parse_obj(d, lx, depth + 1 as ::core::ffi::c_int);
            }
            if n == cap {
                cap *= 2 as ::core::ffi::c_int;
                keys = realloc(
                    keys as *mut ::core::ffi::c_void,
                    (::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t)
                        .wrapping_mul(cap as size_t),
                ) as *mut *mut ::core::ffi::c_char;
                vals = realloc(
                    vals as *mut ::core::ffi::c_void,
                    (::core::mem::size_of::<*mut pr_obj>() as size_t)
                        .wrapping_mul(cap as size_t),
                ) as *mut *mut pr_obj;
                if keys.is_null() || vals.is_null() {
                    abort();
                }
            }
            let ref mut fresh8 = *keys.offset(n as isize);
            *fresh8 = (*key).u.str_0.s;
            let ref mut fresh9 = *vals.offset(n as isize);
            *fresh9 = val;
            n += 1;
        }
    }
    let mut o: *mut pr_obj = new_obj(d, PR_DICT);
    (*o).u.dict.keys = arena_alloc(
        d,
        (::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t)
            .wrapping_mul((if n != 0 { n } else { 1 as ::core::ffi::c_int }) as size_t),
    ) as *mut *mut ::core::ffi::c_char;
    (*o).u.dict.vals = arena_alloc(
        d,
        (::core::mem::size_of::<*mut pr_obj>() as size_t)
            .wrapping_mul((if n != 0 { n } else { 1 as ::core::ffi::c_int }) as size_t),
    ) as *mut *mut pr_obj;
    memcpy(
        (*o).u.dict.keys as *mut ::core::ffi::c_void,
        keys as *const ::core::ffi::c_void,
        (::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t)
            .wrapping_mul(n as size_t),
    );
    memcpy(
        (*o).u.dict.vals as *mut ::core::ffi::c_void,
        vals as *const ::core::ffi::c_void,
        (::core::mem::size_of::<*mut pr_obj>() as size_t).wrapping_mul(n as size_t),
    );
    (*o).u.dict.n = n;
    free(keys as *mut ::core::ffi::c_void);
    free(vals as *mut ::core::ffi::c_void);
    return o;
}
unsafe extern "C" fn parse_obj(
    mut d: *mut pr_doc,
    mut lx: *mut lexer,
    mut depth: ::core::ffi::c_int,
) -> *mut pr_obj {
    skip_ws(lx);
    if (*lx).pos >= (*lx).len || depth > MAX_DEPTH {
        return &raw mut null_obj;
    }
    let mut c: ::core::ffi::c_int = *(*lx).p.offset((*lx).pos as isize)
        as ::core::ffi::c_int;
    if c == '/' as i32 {
        return parse_name(d, lx);
    }
    if c == '(' as i32 {
        return parse_literal_string(d, lx);
    }
    if c == '<' as i32 {
        if (*lx).pos.wrapping_add(1 as size_t) < (*lx).len
            && *(*lx).p.offset((*lx).pos.wrapping_add(1 as size_t) as isize)
                as ::core::ffi::c_int == '<' as i32
        {
            return parse_dict(d, lx, depth);
        }
        return parse_hex_string(d, lx);
    }
    if c == '[' as i32 {
        return parse_array(d, lx, depth);
    }
    if *(*__ctype_b_loc()).offset(c as isize) as ::core::ffi::c_int
        & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
        != 0
    {
        let mut num: ::core::ffi::c_longlong = 0;
        let mut gen: ::core::ffi::c_longlong = 0;
        let mut e1: size_t = 0;
        let mut e2: size_t = 0;
        if peek_uint(lx, (*lx).pos, &raw mut num, &raw mut e1) {
            let mut t: lexer = *lx;
            t.pos = e1;
            skip_ws(&raw mut t);
            if peek_uint(&raw mut t, t.pos, &raw mut gen, &raw mut e2) {
                t.pos = e2;
                skip_ws(&raw mut t);
                if t.pos < t.len
                    && *t.p.offset(t.pos as isize) as ::core::ffi::c_int == 'R' as i32
                    && (t.pos.wrapping_add(1 as size_t) >= t.len
                        || !is_regular(
                            *t.p.offset(t.pos.wrapping_add(1 as size_t) as isize)
                                as ::core::ffi::c_int,
                        ))
                {
                    let mut o: *mut pr_obj = new_obj(d, PR_REF);
                    (*o).u.ref_0.num = num as ::core::ffi::c_int;
                    (*o).u.ref_0.gen = gen as ::core::ffi::c_int;
                    (*lx).pos = t.pos.wrapping_add(1 as size_t);
                    return o;
                }
            }
        }
        return parse_number(d, lx);
    }
    if c == '+' as i32 || c == '-' as i32 || c == '.' as i32 {
        return parse_number(d, lx);
    }
    if at_keyword(lx, b"true\0" as *const u8 as *const ::core::ffi::c_char)
        as ::core::ffi::c_int != 0
        || at_keyword(lx, b"false\0" as *const u8 as *const ::core::ffi::c_char)
            as ::core::ffi::c_int != 0
    {
        let mut o_0: *mut pr_obj = new_obj(d, PR_BOOL);
        (*o_0).u.b = (*(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int
            == 't' as i32) as ::core::ffi::c_int;
        (*lx).pos = (*lx)
            .pos
            .wrapping_add(
                (if (*o_0).u.b != 0 {
                    4 as ::core::ffi::c_int
                } else {
                    5 as ::core::ffi::c_int
                }) as size_t,
            );
        return o_0;
    }
    if at_keyword(lx, b"null\0" as *const u8 as *const ::core::ffi::c_char) {
        (*lx).pos = (*lx).pos.wrapping_add(4 as size_t);
        return &raw mut null_obj;
    }
    while (*lx).pos < (*lx).len
        && is_regular(*(*lx).p.offset((*lx).pos as isize) as ::core::ffi::c_int)
            as ::core::ffi::c_int != 0
    {
        (*lx).pos = (*lx).pos.wrapping_add(1);
    }
    return &raw mut null_obj;
}
#[no_mangle]
pub unsafe extern "C" fn pr_dict_raw(
    mut dict: *mut pr_obj,
    mut key: *const ::core::ffi::c_char,
) -> *mut pr_obj {
    if dict.is_null() {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    if (*dict).type_0 as ::core::ffi::c_uint
        == PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        dict = (*dict).u.stream.dict;
    }
    if dict.is_null()
        || (*dict).type_0 as ::core::ffi::c_uint
            != PR_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*dict).u.dict.n {
        if strcmp(*(*dict).u.dict.keys.offset(i as isize), key)
            == 0 as ::core::ffi::c_int
        {
            return *(*dict).u.dict.vals.offset(i as isize);
        }
        i += 1;
    }
    return ::core::ptr::null_mut::<pr_obj>();
}
#[no_mangle]
pub unsafe extern "C" fn pr_resolve(
    mut d: *mut pr_doc,
    mut o: *mut pr_obj,
) -> *mut pr_obj {
    let mut guard: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while !o.is_null()
        && (*o).type_0 as ::core::ffi::c_uint
            == PR_REF as ::core::ffi::c_int as ::core::ffi::c_uint
        && guard < 32 as ::core::ffi::c_int
    {
        o = pr_load(d, (*o).u.ref_0.num);
        guard += 1;
    }
    if !o.is_null()
        && (*o).type_0 as ::core::ffi::c_uint
            == PR_REF as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    return o;
}
#[no_mangle]
pub unsafe extern "C" fn pr_get(
    mut d: *mut pr_doc,
    mut dict: *mut pr_obj,
    mut key: *const ::core::ffi::c_char,
) -> *mut pr_obj {
    let mut o: *mut pr_obj = pr_resolve(d, pr_dict_raw(dict, key));
    if !o.is_null()
        && (*o).type_0 as ::core::ffi::c_uint
            == PR_NULL as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    return o;
}
#[no_mangle]
pub unsafe extern "C" fn pr_number(
    mut o: *mut pr_obj,
    mut out: *mut ::core::ffi::c_double,
) -> bool {
    if o.is_null() {
        return false_0 != 0;
    }
    if (*o).type_0 as ::core::ffi::c_uint
        == PR_INT as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        *out = (*o).u.i as ::core::ffi::c_double;
        return true_0 != 0;
    }
    if (*o).type_0 as ::core::ffi::c_uint
        == PR_REAL as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        *out = (*o).u.r;
        return true_0 != 0;
    }
    return false_0 != 0;
}
unsafe extern "C" fn name_is(
    mut o: *mut pr_obj,
    mut s: *const ::core::ffi::c_char,
) -> bool {
    return !o.is_null()
        && (*o).type_0 as ::core::ffi::c_uint
            == PR_NAME as ::core::ffi::c_int as ::core::ffi::c_uint
        && strcmp((*o).u.str_0.s, s) == 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn get_int(
    mut d: *mut pr_doc,
    mut dict: *mut pr_obj,
    mut key: *const ::core::ffi::c_char,
    mut dflt: ::core::ffi::c_longlong,
) -> ::core::ffi::c_longlong {
    let mut v: ::core::ffi::c_double = 0.;
    if pr_number(pr_get(d, dict, key), &raw mut v) {
        return v as ::core::ffi::c_longlong;
    }
    return dflt;
}
unsafe extern "C" fn ensure_xref(
    mut d: *mut pr_doc,
    mut num: ::core::ffi::c_int,
) -> bool {
    if num < 0 as ::core::ffi::c_int || num >= MAX_OBJECTS {
        return false_0 != 0;
    }
    if num < (*d).nxref {
        return true_0 != 0;
    }
    let mut n: ::core::ffi::c_int = if (*d).nxref != 0 {
        (*d).nxref
    } else {
        64 as ::core::ffi::c_int
    };
    while n <= num {
        n *= 2 as ::core::ffi::c_int;
    }
    if n > MAX_OBJECTS {
        n = MAX_OBJECTS;
    }
    let mut x: *mut xref_entry = realloc(
        (*d).xref as *mut ::core::ffi::c_void,
        (::core::mem::size_of::<xref_entry>() as size_t).wrapping_mul(n as size_t),
    ) as *mut xref_entry;
    if x.is_null() {
        abort();
    }
    memset(
        x.offset((*d).nxref as isize) as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        (::core::mem::size_of::<xref_entry>() as size_t)
            .wrapping_mul((n - (*d).nxref) as size_t),
    );
    (*d).xref = x;
    (*d).nxref = n;
    return true_0 != 0;
}
unsafe extern "C" fn set_entry(
    mut d: *mut pr_doc,
    mut num: ::core::ffi::c_int,
    mut type_0: ::core::ffi::c_int,
    mut off: size_t,
    mut idx: ::core::ffi::c_int,
    mut overwrite: bool,
) {
    if !ensure_xref(d, num) {
        return;
    }
    let mut e: *mut xref_entry = (*d).xref.offset(num as isize) as *mut xref_entry;
    if (*e).type_0 as ::core::ffi::c_int != 0 && !overwrite {
        return;
    }
    (*e).type_0 = type_0 as ::core::ffi::c_uchar;
    (*e).off = off;
    (*e).idx = idx;
}
unsafe extern "C" fn parse_indirect_at(
    mut d: *mut pr_doc,
    mut off: size_t,
    mut expect_num: ::core::ffi::c_int,
) -> *mut pr_obj {
    let mut lx: lexer = lexer {
        p: (*d).data,
        pos: off,
        len: (*d).len,
    };
    let mut num: ::core::ffi::c_longlong = 0;
    let mut gen: ::core::ffi::c_longlong = 0;
    let mut e: size_t = 0;
    skip_ws(&raw mut lx);
    if !peek_uint(&raw mut lx, lx.pos, &raw mut num, &raw mut e) {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    lx.pos = e;
    skip_ws(&raw mut lx);
    if !peek_uint(&raw mut lx, lx.pos, &raw mut gen, &raw mut e) {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    lx.pos = e;
    skip_ws(&raw mut lx);
    if !at_keyword(&raw mut lx, b"obj\0" as *const u8 as *const ::core::ffi::c_char) {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    if expect_num >= 0 as ::core::ffi::c_int
        && num != expect_num as ::core::ffi::c_longlong
    {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    lx.pos = lx.pos.wrapping_add(3 as size_t);
    let mut o: *mut pr_obj = parse_obj(d, &raw mut lx, 0 as ::core::ffi::c_int);
    skip_ws(&raw mut lx);
    if (*o).type_0 as ::core::ffi::c_uint
        == PR_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
        && at_keyword(
            &raw mut lx,
            b"stream\0" as *const u8 as *const ::core::ffi::c_char,
        ) as ::core::ffi::c_int != 0
    {
        lx.pos = lx.pos.wrapping_add(6 as size_t);
        if lx.pos < lx.len
            && *lx.p.offset(lx.pos as isize) as ::core::ffi::c_int == '\r' as i32
        {
            lx.pos = lx.pos.wrapping_add(1);
        }
        if lx.pos < lx.len
            && *lx.p.offset(lx.pos as isize) as ::core::ffi::c_int == '\n' as i32
        {
            lx.pos = lx.pos.wrapping_add(1);
        }
        let mut start: size_t = lx.pos;
        let mut s: *mut pr_obj = new_obj(d, PR_STREAM);
        (*s).u.stream.dict = o;
        (*s).u.stream.offset = start;
        let mut length: ::core::ffi::c_longlong = -(1 as ::core::ffi::c_int)
            as ::core::ffi::c_longlong;
        let mut lo: *mut pr_obj = pr_dict_raw(
            o,
            b"Length\0" as *const u8 as *const ::core::ffi::c_char,
        );
        if !lo.is_null()
            && (*lo).type_0 as ::core::ffi::c_uint
                == PR_INT as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            length = (*lo).u.i;
        } else if !lo.is_null()
            && (*lo).type_0 as ::core::ffi::c_uint
                == PR_REF as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            let mut r: *mut pr_obj = pr_resolve(d, lo);
            if !r.is_null()
                && (*r).type_0 as ::core::ffi::c_uint
                    == PR_INT as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                length = (*r).u.i;
            }
        }
        let mut ok: bool = length >= 0 as ::core::ffi::c_longlong
            && length as size_t <= (*d).len.wrapping_sub(start);
        if ok {
            let mut t: lexer = lexer {
                p: (*d).data,
                pos: start.wrapping_add(length as size_t),
                len: (*d).len,
            };
            skip_ws(&raw mut t);
            ok = at_keyword(
                &raw mut t,
                b"endstream\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        if !ok {
            let mut hit: *const ::core::ffi::c_uchar = ::core::ptr::null::<
                ::core::ffi::c_uchar,
            >();
            let mut i: size_t = start;
            while i.wrapping_add(9 as size_t) <= (*d).len {
                if *(*d).data.offset(i as isize) as ::core::ffi::c_int == 'e' as i32
                    && memcmp(
                        (*d).data.offset(i as isize) as *const ::core::ffi::c_void,
                        b"endstream\0" as *const u8 as *const ::core::ffi::c_char
                            as *const ::core::ffi::c_void,
                        9 as size_t,
                    ) == 0 as ::core::ffi::c_int
                {
                    hit = (*d).data.offset(i as isize);
                    break;
                } else {
                    i = i.wrapping_add(1);
                }
            }
            if hit.is_null() {
                return ::core::ptr::null_mut::<pr_obj>();
            }
            let mut end: size_t = hit.offset_from((*d).data) as ::core::ffi::c_long
                as size_t;
            while end > start
                && (*(*d).data.offset(end.wrapping_sub(1 as size_t) as isize)
                    as ::core::ffi::c_int == '\n' as i32
                    || *(*d).data.offset(end.wrapping_sub(1 as size_t) as isize)
                        as ::core::ffi::c_int == '\r' as i32)
            {
                end = end.wrapping_sub(1);
            }
            length = end.wrapping_sub(start) as ::core::ffi::c_longlong;
        }
        (*s).u.stream.length = length as size_t;
        return s;
    }
    return o;
}
#[no_mangle]
pub unsafe extern "C" fn pr_load(
    mut d: *mut pr_doc,
    mut num: ::core::ffi::c_int,
) -> *mut pr_obj {
    if num < 0 as ::core::ffi::c_int || num >= (*d).nxref {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    let mut e: *mut xref_entry = (*d).xref.offset(num as isize) as *mut xref_entry;
    if !(*e).obj.is_null() {
        return (*e).obj;
    }
    if (*e).loading as ::core::ffi::c_int != 0
        || (*d).resolve_budget <= 0 as ::core::ffi::c_int
    {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    (*d).resolve_budget -= 1;
    (*e).loading = 1 as ::core::ffi::c_uchar;
    let mut o: *mut pr_obj = ::core::ptr::null_mut::<pr_obj>();
    if (*e).type_0 as ::core::ffi::c_int == 1 as ::core::ffi::c_int
        && (*e).off < (*d).len
    {
        o = parse_indirect_at(d, (*e).off, num);
    } else if (*e).type_0 as ::core::ffi::c_int == 2 as ::core::ffi::c_int {
        o = load_from_objstm(d, (*e).off as ::core::ffi::c_int, (*e).idx, num);
    }
    (*e).loading = 0 as ::core::ffi::c_uchar;
    (*e).obj = if !o.is_null() { o } else { &raw mut null_obj };
    (*d).resolve_budget += 1;
    return (*e).obj;
}
unsafe extern "C" fn load_from_objstm(
    mut d: *mut pr_doc,
    mut stmnum: ::core::ffi::c_int,
    mut idx: ::core::ffi::c_int,
    mut want: ::core::ffi::c_int,
) -> *mut pr_obj {
    let mut stm: *mut pr_obj = pr_load(d, stmnum);
    if stm.is_null()
        || (*stm).type_0 as ::core::ffi::c_uint
            != PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ::core::ptr::null_mut::<pr_obj>();
    }
    let mut buf: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    if !pr_stream_decode(d, stm, &raw mut buf) {
        pbuf_free(&raw mut buf);
        return ::core::ptr::null_mut::<pr_obj>();
    }
    let mut n: ::core::ffi::c_longlong = get_int(
        d,
        stm,
        b"N\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_longlong,
    );
    let mut first: ::core::ffi::c_longlong = get_int(
        d,
        stm,
        b"First\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_longlong,
    );
    let mut result: *mut pr_obj = ::core::ptr::null_mut::<pr_obj>();
    let mut copy: *mut ::core::ffi::c_uchar = arena_alloc(
        d,
        buf.len.wrapping_add(1 as size_t),
    ) as *mut ::core::ffi::c_uchar;
    memcpy(
        copy as *mut ::core::ffi::c_void,
        buf.data as *const ::core::ffi::c_void,
        buf.len,
    );
    let mut blen: size_t = buf.len;
    pbuf_free(&raw mut buf);
    let mut lx: lexer = lexer {
        p: copy,
        pos: 0 as size_t,
        len: blen,
    };
    let mut i: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
    while i < n && i < 1000000 as ::core::ffi::c_longlong {
        let mut onum: ::core::ffi::c_longlong = 0;
        let mut ooff: ::core::ffi::c_longlong = 0;
        let mut e: size_t = 0;
        skip_ws(&raw mut lx);
        if !peek_uint(&raw mut lx, lx.pos, &raw mut onum, &raw mut e) {
            break;
        }
        lx.pos = e;
        skip_ws(&raw mut lx);
        if !peek_uint(&raw mut lx, lx.pos, &raw mut ooff, &raw mut e) {
            break;
        }
        lx.pos = e;
        if i == idx as ::core::ffi::c_longlong || onum == want as ::core::ffi::c_longlong
        {
            if first + ooff >= 0 as ::core::ffi::c_longlong
                && ((first + ooff) as size_t) < blen
            {
                let mut ol: lexer = lexer {
                    p: copy,
                    pos: (first + ooff) as size_t,
                    len: blen,
                };
                result = parse_obj(d, &raw mut ol, 0 as ::core::ffi::c_int);
            }
            if onum == want as ::core::ffi::c_longlong {
                break;
            }
        }
        i += 1;
    }
    return result;
}
unsafe extern "C" fn apply_predictor(
    mut data: *mut pbuf,
    mut predictor: ::core::ffi::c_int,
    mut colors: ::core::ffi::c_int,
    mut bpc: ::core::ffi::c_int,
    mut columns: ::core::ffi::c_int,
) -> bool {
    if predictor < 10 as ::core::ffi::c_int {
        return predictor == 1 as ::core::ffi::c_int;
    }
    let mut bpp: size_t = ((colors * bpc + 7 as ::core::ffi::c_int)
        / 8 as ::core::ffi::c_int) as size_t;
    let mut row: size_t = ((colors * bpc * columns + 7 as ::core::ffi::c_int)
        / 8 as ::core::ffi::c_int) as size_t;
    if bpp == 0 as size_t || row == 0 as size_t {
        return false_0 != 0;
    }
    let mut nrows: size_t = (*data).len.wrapping_div(row.wrapping_add(1 as size_t));
    let mut out: *mut ::core::ffi::c_uchar = malloc(
        nrows.wrapping_mul(row).wrapping_add(1 as size_t),
    ) as *mut ::core::ffi::c_uchar;
    let mut prev: *mut ::core::ffi::c_uchar = calloc(row, 1 as size_t)
        as *mut ::core::ffi::c_uchar;
    if out.is_null() || prev.is_null() {
        abort();
    }
    let mut r: size_t = 0 as size_t;
    while r < nrows {
        let mut src: *mut ::core::ffi::c_uchar = (*data)
            .data
            .offset(r.wrapping_mul(row.wrapping_add(1 as size_t)) as isize);
        let mut type_0: ::core::ffi::c_int = *src
            .offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
        src = src.offset(1);
        let mut dst: *mut ::core::ffi::c_uchar = out
            .offset(r.wrapping_mul(row) as isize);
        let mut i: size_t = 0 as size_t;
        while i < row {
            let mut a: ::core::ffi::c_int = if i >= bpp {
                *dst.offset(i.wrapping_sub(bpp) as isize) as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            };
            let mut b: ::core::ffi::c_int = *prev.offset(i as isize)
                as ::core::ffi::c_int;
            let mut c: ::core::ffi::c_int = if i >= bpp {
                *prev.offset(i.wrapping_sub(bpp) as isize) as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            };
            let mut x: ::core::ffi::c_int = *src.offset(i as isize)
                as ::core::ffi::c_int;
            match type_0 {
                1 => {
                    x += a;
                }
                2 => {
                    x += b;
                }
                3 => {
                    x += (a + b) / 2 as ::core::ffi::c_int;
                }
                4 => {
                    let mut p: ::core::ffi::c_int = a + b - c;
                    let mut pa: ::core::ffi::c_int = abs(p - a);
                    let mut pb: ::core::ffi::c_int = abs(p - b);
                    let mut pc: ::core::ffi::c_int = abs(p - c);
                    x += if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { c };
                }
                0 | _ => {}
            }
            *dst.offset(i as isize) = x as ::core::ffi::c_uchar;
            i = i.wrapping_add(1);
        }
        memcpy(prev as *mut ::core::ffi::c_void, dst as *const ::core::ffi::c_void, row);
        r = r.wrapping_add(1);
    }
    free(prev as *mut ::core::ffi::c_void);
    free((*data).data as *mut ::core::ffi::c_void);
    (*data).data = out;
    (*data).len = nrows.wrapping_mul(row);
    (*data).cap = nrows.wrapping_mul(row).wrapping_add(1 as size_t);
    return true_0 != 0;
}
unsafe extern "C" fn decode_ahx(
    mut out: *mut pbuf,
    mut s: *const ::core::ffi::c_uchar,
    mut n: size_t,
) -> bool {
    let mut hi: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    let mut i: size_t = 0 as size_t;
    while i < n {
        if *s.offset(i as isize) as ::core::ffi::c_int == '>' as i32 {
            break;
        }
        let mut v: ::core::ffi::c_int = hexval(
            *s.offset(i as isize) as ::core::ffi::c_int,
        );
        if !(v < 0 as ::core::ffi::c_int) {
            if hi < 0 as ::core::ffi::c_int {
                hi = v;
            } else {
                pbuf_putc(out, hi * 16 as ::core::ffi::c_int + v);
                hi = -(1 as ::core::ffi::c_int);
            }
        }
        i = i.wrapping_add(1);
    }
    if hi >= 0 as ::core::ffi::c_int {
        pbuf_putc(out, hi * 16 as ::core::ffi::c_int);
    }
    return true_0 != 0;
}
unsafe extern "C" fn decode_a85(
    mut out: *mut pbuf,
    mut s: *const ::core::ffi::c_uchar,
    mut n: size_t,
) -> bool {
    let mut tuple: uint32_t = 0 as uint32_t;
    let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut i: size_t = 0 as size_t;
    if n >= 2 as size_t
        && *s.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == '<' as i32
        && *s.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == '~' as i32
    {
        i = 2 as size_t;
    }
    while i < n {
        let mut c: ::core::ffi::c_int = *s.offset(i as isize) as ::core::ffi::c_int;
        if c == '~' as i32 {
            break;
        }
        if !is_ws(c) {
            if c == 'z' as i32 && count == 0 as ::core::ffi::c_int {
                pbuf_append(
                    out,
                    b"\0\0\0\0\0" as *const u8 as *const ::core::ffi::c_char
                        as *const ::core::ffi::c_void,
                    4 as size_t,
                );
            } else {
                if c < '!' as i32 || c > 'u' as i32 {
                    return false_0 != 0;
                }
                tuple = tuple
                    .wrapping_mul(85 as uint32_t)
                    .wrapping_add((c - '!' as i32) as uint32_t);
                count += 1;
                if count == 5 as ::core::ffi::c_int {
                    let mut b: [::core::ffi::c_uchar; 4] = [
                        (tuple >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar,
                        (tuple >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar,
                        (tuple >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar,
                        tuple as ::core::ffi::c_uchar,
                    ];
                    pbuf_append(
                        out,
                        &raw mut b as *mut ::core::ffi::c_uchar
                            as *const ::core::ffi::c_void,
                        4 as size_t,
                    );
                    tuple = 0 as uint32_t;
                    count = 0 as ::core::ffi::c_int;
                }
            }
        }
        i = i.wrapping_add(1);
    }
    if count > 1 as ::core::ffi::c_int {
        let mut k: ::core::ffi::c_int = count;
        while k < 5 as ::core::ffi::c_int {
            tuple = tuple.wrapping_mul(85 as uint32_t).wrapping_add(84 as uint32_t);
            k += 1;
        }
        let mut b_0: [::core::ffi::c_uchar; 4] = [
            (tuple >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar,
            (tuple >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar,
            (tuple >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar,
            tuple as ::core::ffi::c_uchar,
        ];
        pbuf_append(
            out,
            &raw mut b_0 as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
            (count - 1 as ::core::ffi::c_int) as size_t,
        );
    }
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn pr_stream_raw(
    mut d: *mut pr_doc,
    mut s: *mut pr_obj,
    mut len: *mut size_t,
) -> *const ::core::ffi::c_uchar {
    if s.is_null()
        || (*s).type_0 as ::core::ffi::c_uint
            != PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*s).u.stream.offset > (*d).len
    {
        return ::core::ptr::null::<::core::ffi::c_uchar>();
    }
    let mut n: size_t = (*s).u.stream.length;
    if n > (*d).len.wrapping_sub((*s).u.stream.offset) {
        n = (*d).len.wrapping_sub((*s).u.stream.offset);
    }
    *len = n;
    return (*d).data.offset((*s).u.stream.offset as isize);
}
#[no_mangle]
pub unsafe extern "C" fn pr_stream_decode(
    mut d: *mut pr_doc,
    mut s: *mut pr_obj,
    mut out: *mut pbuf,
) -> bool {
    let mut n: size_t = 0;
    let mut raw: *const ::core::ffi::c_uchar = pr_stream_raw(d, s, &raw mut n);
    if raw.is_null() {
        return false_0 != 0;
    }
    let mut filter: *mut pr_obj = pr_get(
        d,
        s,
        b"Filter\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut parms: *mut pr_obj = pr_get(
        d,
        s,
        b"DecodeParms\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut nf: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut filters: [*mut pr_obj; 8] = [::core::ptr::null_mut::<pr_obj>(); 8];
    let mut params: [*mut pr_obj; 8] = [::core::ptr::null_mut::<pr_obj>(); 8];
    if !filter.is_null()
        && (*filter).type_0 as ::core::ffi::c_uint
            == PR_NAME as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        filters[0 as ::core::ffi::c_int as usize] = filter;
        params[0 as ::core::ffi::c_int as usize] = if !parms.is_null()
            && (*parms).type_0 as ::core::ffi::c_uint
                == PR_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            if (*parms).u.arr.n != 0 {
                pr_resolve(
                    d,
                    *(*parms).u.arr.items.offset(0 as ::core::ffi::c_int as isize),
                )
            } else {
                ::core::ptr::null_mut::<pr_obj>()
            }
        } else {
            parms
        };
        nf = 1 as ::core::ffi::c_int;
    } else if !filter.is_null()
        && (*filter).type_0 as ::core::ffi::c_uint
            == PR_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < (*filter).u.arr.n && nf < 8 as ::core::ffi::c_int {
            filters[nf as usize] = pr_resolve(
                d,
                *(*filter).u.arr.items.offset(i as isize),
            );
            params[nf as usize] = if !parms.is_null()
                && (*parms).type_0 as ::core::ffi::c_uint
                    == PR_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
                && i < (*parms).u.arr.n
            {
                pr_resolve(d, *(*parms).u.arr.items.offset(i as isize))
            } else {
                ::core::ptr::null_mut::<pr_obj>()
            };
            nf += 1;
            i += 1;
        }
    }
    let mut cur: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    pbuf_append(&raw mut cur, raw as *const ::core::ffi::c_void, n);
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < nf {
        let mut next: pbuf = pbuf {
            data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
            len: 0,
            cap: 0,
        };
        let mut ok: bool = false;
        if name_is(
            filters[i_0 as usize],
            b"FlateDecode\0" as *const u8 as *const ::core::ffi::c_char,
        ) as ::core::ffi::c_int != 0
            || name_is(
                filters[i_0 as usize],
                b"Fl\0" as *const u8 as *const ::core::ffi::c_char,
            ) as ::core::ffi::c_int != 0
        {
            ok = pbuf_inflate(&raw mut next, cur.data, cur.len)
                == 0 as ::core::ffi::c_int;
            let mut p: *mut pr_obj = params[i_0 as usize];
            if ok as ::core::ffi::c_int != 0 && !p.is_null()
                && (*p).type_0 as ::core::ffi::c_uint
                    == PR_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                let mut pred: ::core::ffi::c_int = get_int(
                    d,
                    p,
                    b"Predictor\0" as *const u8 as *const ::core::ffi::c_char,
                    1 as ::core::ffi::c_longlong,
                ) as ::core::ffi::c_int;
                if pred > 1 as ::core::ffi::c_int {
                    ok = apply_predictor(
                        &raw mut next,
                        pred,
                        get_int(
                            d,
                            p,
                            b"Colors\0" as *const u8 as *const ::core::ffi::c_char,
                            1 as ::core::ffi::c_longlong,
                        ) as ::core::ffi::c_int,
                        get_int(
                            d,
                            p,
                            b"BitsPerComponent\0" as *const u8
                                as *const ::core::ffi::c_char,
                            8 as ::core::ffi::c_longlong,
                        ) as ::core::ffi::c_int,
                        get_int(
                            d,
                            p,
                            b"Columns\0" as *const u8 as *const ::core::ffi::c_char,
                            1 as ::core::ffi::c_longlong,
                        ) as ::core::ffi::c_int,
                    );
                }
            }
        } else if name_is(
            filters[i_0 as usize],
            b"ASCIIHexDecode\0" as *const u8 as *const ::core::ffi::c_char,
        ) as ::core::ffi::c_int != 0
            || name_is(
                filters[i_0 as usize],
                b"AHx\0" as *const u8 as *const ::core::ffi::c_char,
            ) as ::core::ffi::c_int != 0
        {
            ok = decode_ahx(&raw mut next, cur.data, cur.len);
        } else if name_is(
            filters[i_0 as usize],
            b"ASCII85Decode\0" as *const u8 as *const ::core::ffi::c_char,
        ) as ::core::ffi::c_int != 0
            || name_is(
                filters[i_0 as usize],
                b"A85\0" as *const u8 as *const ::core::ffi::c_char,
            ) as ::core::ffi::c_int != 0
        {
            ok = decode_a85(&raw mut next, cur.data, cur.len);
        } else {
            ok = false_0 != 0;
        }
        pbuf_free(&raw mut cur);
        cur = next;
        if !ok {
            pbuf_free(&raw mut cur);
            return false_0 != 0;
        }
        i_0 += 1;
    }
    pbuf_append(out, cur.data as *const ::core::ffi::c_void, cur.len);
    pbuf_free(&raw mut cur);
    return true_0 != 0;
}
unsafe extern "C" fn load_xref_stream(mut d: *mut pr_doc, mut s: *mut pr_obj) -> bool {
    if s.is_null()
        || (*s).type_0 as ::core::ffi::c_uint
            != PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return false_0 != 0;
    }
    let mut w: *mut pr_obj = pr_get(
        d,
        s,
        b"W\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if w.is_null()
        || (*w).type_0 as ::core::ffi::c_uint
            != PR_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*w).u.arr.n < 3 as ::core::ffi::c_int
    {
        return false_0 != 0;
    }
    let mut W: [::core::ffi::c_int; 3] = [0; 3];
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        let mut v: ::core::ffi::c_double = 0.;
        if !pr_number(pr_resolve(d, *(*w).u.arr.items.offset(i as isize)), &raw mut v)
            || v < 0 as ::core::ffi::c_int as ::core::ffi::c_double
            || v > 8 as ::core::ffi::c_int as ::core::ffi::c_double
        {
            return false_0 != 0;
        }
        W[i as usize] = v as ::core::ffi::c_int;
        i += 1;
    }
    let mut buf: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    if !pr_stream_decode(d, s, &raw mut buf) {
        pbuf_free(&raw mut buf);
        return false_0 != 0;
    }
    let mut size: ::core::ffi::c_longlong = get_int(
        d,
        s,
        b"Size\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_longlong,
    );
    let mut index: *mut pr_obj = pr_get(
        d,
        s,
        b"Index\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut ranges: [::core::ffi::c_longlong; 128] = [0; 128];
    let mut nr: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if !index.is_null()
        && (*index).type_0 as ::core::ffi::c_uint
            == PR_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while (i_0 + 1 as ::core::ffi::c_int) < (*index).u.arr.n
            && nr < 64 as ::core::ffi::c_int
        {
            let mut a: ::core::ffi::c_double = 0.;
            let mut b: ::core::ffi::c_double = 0.;
            if pr_number(
                pr_resolve(d, *(*index).u.arr.items.offset(i_0 as isize)),
                &raw mut a,
            ) as ::core::ffi::c_int != 0
                && pr_number(
                    pr_resolve(
                        d,
                        *(*index)
                            .u
                            .arr
                            .items
                            .offset((i_0 + 1 as ::core::ffi::c_int) as isize),
                    ),
                    &raw mut b,
                ) as ::core::ffi::c_int != 0
            {
                ranges[(2 as ::core::ffi::c_int * nr) as usize] = a
                    as ::core::ffi::c_longlong;
                ranges[(2 as ::core::ffi::c_int * nr + 1 as ::core::ffi::c_int)
                    as usize] = b as ::core::ffi::c_longlong;
                nr += 1;
            }
            i_0 += 2 as ::core::ffi::c_int;
        }
    } else {
        ranges[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_longlong;
        ranges[1 as ::core::ffi::c_int as usize] = size;
        nr = 1 as ::core::ffi::c_int;
    }
    let mut rowlen: size_t = (W[0 as ::core::ffi::c_int as usize]
        + W[1 as ::core::ffi::c_int as usize] + W[2 as ::core::ffi::c_int as usize])
        as size_t;
    let mut pos: size_t = 0 as size_t;
    let mut r: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while r < nr {
        let mut k: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
        while k
            < ranges[(2 as ::core::ffi::c_int * r + 1 as ::core::ffi::c_int) as usize]
        {
            if pos.wrapping_add(rowlen) > buf.len {
                break;
            }
            let mut f: [::core::ffi::c_ulonglong; 3] = [
                1 as ::core::ffi::c_int as ::core::ffi::c_ulonglong,
                0 as ::core::ffi::c_int as ::core::ffi::c_ulonglong,
                0 as ::core::ffi::c_int as ::core::ffi::c_ulonglong,
            ];
            if W[0 as ::core::ffi::c_int as usize] == 0 as ::core::ffi::c_int {
                f[0 as ::core::ffi::c_int as usize] = 1 as ::core::ffi::c_ulonglong;
            }
            let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while j < 3 as ::core::ffi::c_int {
                if !(W[j as usize] == 0 as ::core::ffi::c_int) {
                    let mut v_0: ::core::ffi::c_ulonglong = 0
                        as ::core::ffi::c_ulonglong;
                    let mut b_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    while b_0 < W[j as usize] {
                        let fresh15 = pos;
                        pos = pos.wrapping_add(1);
                        v_0 = v_0 << 8 as ::core::ffi::c_int
                            | *buf.data.offset(fresh15 as isize)
                                as ::core::ffi::c_ulonglong;
                        b_0 += 1;
                    }
                    f[j as usize] = v_0;
                }
                j += 1;
            }
            let mut num: ::core::ffi::c_longlong = ranges[(2 as ::core::ffi::c_int * r)
                as usize] + k;
            if !(num < 0 as ::core::ffi::c_longlong
                || num >= MAX_OBJECTS as ::core::ffi::c_longlong)
            {
                if f[0 as ::core::ffi::c_int as usize] == 1 as ::core::ffi::c_ulonglong {
                    set_entry(
                        d,
                        num as ::core::ffi::c_int,
                        1 as ::core::ffi::c_int,
                        f[1 as ::core::ffi::c_int as usize] as size_t,
                        0 as ::core::ffi::c_int,
                        false_0 != 0,
                    );
                } else if f[0 as ::core::ffi::c_int as usize]
                    == 2 as ::core::ffi::c_ulonglong
                {
                    set_entry(
                        d,
                        num as ::core::ffi::c_int,
                        2 as ::core::ffi::c_int,
                        f[1 as ::core::ffi::c_int as usize] as size_t,
                        f[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int,
                        false_0 != 0,
                    );
                } else if f[0 as ::core::ffi::c_int as usize]
                    == 0 as ::core::ffi::c_ulonglong
                {
                    if ensure_xref(d, num as ::core::ffi::c_int) as ::core::ffi::c_int
                        != 0 && (*(*d).xref.offset(num as isize)).type_0 == 0
                    {
                        (*(*d).xref.offset(num as isize)).type_0 = 3
                            as ::core::ffi::c_uchar;
                    }
                }
            }
            k += 1;
        }
        r += 1;
    }
    pbuf_free(&raw mut buf);
    return true_0 != 0;
}
unsafe extern "C" fn load_xref_at(
    mut d: *mut pr_doc,
    mut off: size_t,
    mut depth: ::core::ffi::c_int,
) -> bool {
    if depth > 32 as ::core::ffi::c_int || off >= (*d).len {
        return false_0 != 0;
    }
    let mut lx: lexer = lexer {
        p: (*d).data,
        pos: off,
        len: (*d).len,
    };
    skip_ws(&raw mut lx);
    let mut trailer: *mut pr_obj = ::core::ptr::null_mut::<pr_obj>();
    if at_keyword(&raw mut lx, b"xref\0" as *const u8 as *const ::core::ffi::c_char) {
        lx.pos = lx.pos.wrapping_add(4 as size_t);
        loop {
            skip_ws(&raw mut lx);
            let mut start: ::core::ffi::c_longlong = 0;
            let mut count: ::core::ffi::c_longlong = 0;
            let mut e: size_t = 0;
            if !peek_uint(&raw mut lx, lx.pos, &raw mut start, &raw mut e) {
                break;
            }
            lx.pos = e;
            skip_ws(&raw mut lx);
            if !peek_uint(&raw mut lx, lx.pos, &raw mut count, &raw mut e) {
                return false_0 != 0;
            }
            lx.pos = e;
            let mut k: ::core::ffi::c_longlong = 0 as ::core::ffi::c_longlong;
            while k < count {
                let mut o: ::core::ffi::c_longlong = 0;
                let mut g: ::core::ffi::c_longlong = 0;
                skip_ws(&raw mut lx);
                if !peek_uint(&raw mut lx, lx.pos, &raw mut o, &raw mut e) {
                    return false_0 != 0;
                }
                lx.pos = e;
                skip_ws(&raw mut lx);
                if !peek_uint(&raw mut lx, lx.pos, &raw mut g, &raw mut e) {
                    return false_0 != 0;
                }
                lx.pos = e;
                skip_ws(&raw mut lx);
                if lx.pos >= lx.len {
                    return false_0 != 0;
                }
                let fresh16 = lx.pos;
                lx.pos = lx.pos.wrapping_add(1);
                let mut kind: ::core::ffi::c_int = *lx.p.offset(fresh16 as isize)
                    as ::core::ffi::c_int;
                if !(start + k >= MAX_OBJECTS as ::core::ffi::c_longlong) {
                    if kind == 'n' as i32 {
                        set_entry(
                            d,
                            (start + k) as ::core::ffi::c_int,
                            1 as ::core::ffi::c_int,
                            o as size_t,
                            0 as ::core::ffi::c_int,
                            false_0 != 0,
                        );
                    } else if ensure_xref(d, (start + k) as ::core::ffi::c_int)
                        as ::core::ffi::c_int != 0
                        && (*(*d).xref.offset((start + k) as isize)).type_0 == 0
                    {
                        (*(*d).xref.offset((start + k) as isize)).type_0 = 3
                            as ::core::ffi::c_uchar;
                    }
                }
                k += 1;
            }
        }
        skip_ws(&raw mut lx);
        if !at_keyword(
            &raw mut lx,
            b"trailer\0" as *const u8 as *const ::core::ffi::c_char,
        ) {
            return false_0 != 0;
        }
        lx.pos = lx.pos.wrapping_add(7 as size_t);
        trailer = parse_obj(d, &raw mut lx, 0 as ::core::ffi::c_int);
        if (*trailer).type_0 as ::core::ffi::c_uint
            != PR_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            return false_0 != 0;
        }
        let mut xs: *mut pr_obj = pr_dict_raw(
            trailer,
            b"XRefStm\0" as *const u8 as *const ::core::ffi::c_char,
        );
        if !xs.is_null()
            && (*xs).type_0 as ::core::ffi::c_uint
                == PR_INT as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            let mut s: *mut pr_obj = parse_indirect_at(
                d,
                (*xs).u.i as size_t,
                -(1 as ::core::ffi::c_int),
            );
            load_xref_stream(d, s);
        }
    } else {
        let mut s_0: *mut pr_obj = parse_indirect_at(
            d,
            lx.pos,
            -(1 as ::core::ffi::c_int),
        );
        if s_0.is_null()
            || (*s_0).type_0 as ::core::ffi::c_uint
                != PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
            || !name_is(
                pr_dict_raw(s_0, b"Type\0" as *const u8 as *const ::core::ffi::c_char),
                b"XRef\0" as *const u8 as *const ::core::ffi::c_char,
            )
        {
            return false_0 != 0;
        }
        if !load_xref_stream(d, s_0) {
            return false_0 != 0;
        }
        trailer = (*s_0).u.stream.dict;
    }
    if (*d).trailer.is_null() {
        (*d).trailer = trailer;
    }
    let mut prev: *mut pr_obj = pr_dict_raw(
        trailer,
        b"Prev\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if !prev.is_null()
        && (*prev).type_0 as ::core::ffi::c_uint
            == PR_INT as ::core::ffi::c_int as ::core::ffi::c_uint
        && (*prev).u.i >= 0 as ::core::ffi::c_longlong && (*prev).u.i as size_t != off
    {
        load_xref_at(d, (*prev).u.i as size_t, depth + 1 as ::core::ffi::c_int);
    }
    return true_0 != 0;
}
unsafe extern "C" fn reconstruct(mut d: *mut pr_doc) -> bool {
    free((*d).xref as *mut ::core::ffi::c_void);
    (*d).xref = ::core::ptr::null_mut::<xref_entry>();
    (*d).nxref = 0 as ::core::ffi::c_int;
    (*d).trailer = ::core::ptr::null_mut::<pr_obj>();
    let mut i: size_t = 0 as size_t;
    while i.wrapping_add(5 as size_t) < (*d).len {
        if !(*(*d).data.offset(i as isize) as ::core::ffi::c_int != 'o' as i32
            || memcmp(
                (*d).data.offset(i as isize) as *const ::core::ffi::c_void,
                b"obj\0" as *const u8 as *const ::core::ffi::c_char
                    as *const ::core::ffi::c_void,
                3 as size_t,
            ) != 0 as ::core::ffi::c_int)
        {
            if !(i.wrapping_add(3 as size_t) < (*d).len
                && is_regular(
                    *(*d).data.offset(i.wrapping_add(3 as size_t) as isize)
                        as ::core::ffi::c_int,
                ) as ::core::ffi::c_int != 0)
            {
                let mut j: size_t = i;
                while j > 0 as size_t
                    && is_ws(
                        *(*d).data.offset(j.wrapping_sub(1 as size_t) as isize)
                            as ::core::ffi::c_int,
                    ) as ::core::ffi::c_int != 0
                {
                    j = j.wrapping_sub(1);
                }
                let mut gend: size_t = j;
                while j > 0 as size_t
                    && *(*__ctype_b_loc())
                        .offset(
                            *(*d).data.offset(j.wrapping_sub(1 as size_t) as isize)
                                as ::core::ffi::c_int as isize,
                        ) as ::core::ffi::c_int
                        & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort
                            as ::core::ffi::c_int != 0
                {
                    j = j.wrapping_sub(1);
                }
                if !(j == gend) {
                    while j > 0 as size_t
                        && is_ws(
                            *(*d).data.offset(j.wrapping_sub(1 as size_t) as isize)
                                as ::core::ffi::c_int,
                        ) as ::core::ffi::c_int != 0
                    {
                        j = j.wrapping_sub(1);
                    }
                    let mut nend: size_t = j;
                    while j > 0 as size_t
                        && *(*__ctype_b_loc())
                            .offset(
                                *(*d).data.offset(j.wrapping_sub(1 as size_t) as isize)
                                    as ::core::ffi::c_int as isize,
                            ) as ::core::ffi::c_int
                            & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort
                                as ::core::ffi::c_int != 0
                    {
                        j = j.wrapping_sub(1);
                    }
                    if !(j == nend
                        || j > 0 as size_t
                            && is_regular(
                                *(*d).data.offset(j.wrapping_sub(1 as size_t) as isize)
                                    as ::core::ffi::c_int,
                            ) as ::core::ffi::c_int != 0)
                    {
                        let mut num: ::core::ffi::c_longlong = strtoll(
                            ((*d).data as *const ::core::ffi::c_char).offset(j as isize),
                            ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
                            10 as ::core::ffi::c_int,
                        );
                        if num > 0 as ::core::ffi::c_longlong
                            && num < MAX_OBJECTS as ::core::ffi::c_longlong
                        {
                            set_entry(
                                d,
                                num as ::core::ffi::c_int,
                                1 as ::core::ffi::c_int,
                                j,
                                0 as ::core::ffi::c_int,
                                true_0 != 0,
                            );
                        }
                    }
                }
            }
        }
        i = i.wrapping_add(1);
    }
    let mut i_0: size_t = (*d).len;
    loop {
        let fresh2 = i_0;
        i_0 = i_0.wrapping_sub(1);
        if !(fresh2 > 7 as size_t) {
            break;
        }
        if !(memcmp(
            (*d).data.offset(i_0 as isize) as *const ::core::ffi::c_void,
            b"trailer\0" as *const u8 as *const ::core::ffi::c_char
                as *const ::core::ffi::c_void,
            7 as size_t,
        ) == 0 as ::core::ffi::c_int)
        {
            continue;
        }
        let mut lx: lexer = lexer {
            p: (*d).data,
            pos: i_0.wrapping_add(7 as size_t),
            len: (*d).len,
        };
        let mut t: *mut pr_obj = parse_obj(d, &raw mut lx, 0 as ::core::ffi::c_int);
        if !((*t).type_0 as ::core::ffi::c_uint
            == PR_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
            && !pr_dict_raw(t, b"Root\0" as *const u8 as *const ::core::ffi::c_char)
                .is_null())
        {
            continue;
        }
        (*d).trailer = t;
        break;
    }
    if (*d).trailer.is_null() {
        let mut n: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while n < (*d).nxref {
            if !((*(*d).xref.offset(n as isize)).type_0 as ::core::ffi::c_int
                != 1 as ::core::ffi::c_int)
            {
                let mut o: *mut pr_obj = pr_load(d, n);
                if !o.is_null()
                    && (*o).type_0 as ::core::ffi::c_uint
                        == PR_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
                    && name_is(
                        pr_dict_raw(
                            o,
                            b"Type\0" as *const u8 as *const ::core::ffi::c_char,
                        ),
                        b"Catalog\0" as *const u8 as *const ::core::ffi::c_char,
                    ) as ::core::ffi::c_int != 0
                {
                    let mut t_0: *mut pr_obj = new_obj(d, PR_DICT);
                    (*t_0).u.dict.keys = arena_alloc(
                        d,
                        ::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t,
                    ) as *mut *mut ::core::ffi::c_char;
                    (*t_0).u.dict.vals = arena_alloc(
                        d,
                        ::core::mem::size_of::<*mut pr_obj>() as size_t,
                    ) as *mut *mut pr_obj;
                    let ref mut fresh3 = *(*t_0)
                        .u
                        .dict
                        .keys
                        .offset(0 as ::core::ffi::c_int as isize);
                    *fresh3 = b"Root\0" as *const u8 as *const ::core::ffi::c_char
                        as *mut ::core::ffi::c_char;
                    let mut ref_0: *mut pr_obj = new_obj(d, PR_REF);
                    (*ref_0).u.ref_0.num = n;
                    let ref mut fresh4 = *(*t_0)
                        .u
                        .dict
                        .vals
                        .offset(0 as ::core::ffi::c_int as isize);
                    *fresh4 = ref_0;
                    (*t_0).u.dict.n = 1 as ::core::ffi::c_int;
                    (*d).trailer = t_0;
                    break;
                } else if !o.is_null()
                    && (*o).type_0 as ::core::ffi::c_uint
                        == PR_STREAM as ::core::ffi::c_int as ::core::ffi::c_uint
                    && name_is(
                        pr_dict_raw(
                            o,
                            b"Type\0" as *const u8 as *const ::core::ffi::c_char,
                        ),
                        b"XRef\0" as *const u8 as *const ::core::ffi::c_char,
                    ) as ::core::ffi::c_int != 0
                    && !pr_dict_raw(
                            o,
                            b"Root\0" as *const u8 as *const ::core::ffi::c_char,
                        )
                        .is_null()
                {
                    (*d).trailer = (*o).u.stream.dict;
                    load_xref_stream(d, o);
                    break;
                }
            }
            n += 1;
        }
    }
    return !(*d).trailer.is_null();
}
#[no_mangle]
pub unsafe extern "C" fn pr_open(
    mut data: *const ::core::ffi::c_uchar,
    mut len: size_t,
) -> *mut pr_doc {
    if data.is_null() || len < 8 as size_t {
        return ::core::ptr::null_mut::<pr_doc>();
    }
    let mut d: *mut pr_doc = calloc(
        1 as size_t,
        ::core::mem::size_of::<pr_doc>() as size_t,
    ) as *mut pr_doc;
    if d.is_null() {
        abort();
    }
    (*d).data = data;
    (*d).len = len;
    (*d).resolve_budget = 256 as ::core::ffi::c_int;
    let mut ok: bool = false_0 != 0;
    let mut from: size_t = if len > 4096 as size_t {
        len.wrapping_sub(4096 as size_t)
    } else {
        0 as size_t
    };
    let mut i: size_t = len.wrapping_sub(9 as size_t);
    loop {
        let fresh1 = i;
        i = i.wrapping_sub(1);
        if !(fresh1 > from) {
            break;
        }
        if !(memcmp(
            data.offset(i as isize) as *const ::core::ffi::c_void,
            b"startxref\0" as *const u8 as *const ::core::ffi::c_char
                as *const ::core::ffi::c_void,
            9 as size_t,
        ) == 0 as ::core::ffi::c_int)
        {
            continue;
        }
        let mut lx: lexer = lexer {
            p: data,
            pos: i.wrapping_add(9 as size_t),
            len: len,
        };
        skip_ws(&raw mut lx);
        let mut off: ::core::ffi::c_longlong = 0;
        let mut e: size_t = 0;
        if peek_uint(&raw mut lx, lx.pos, &raw mut off, &raw mut e) {
            ok = load_xref_at(d, off as size_t, 0 as ::core::ffi::c_int);
        }
        break;
    }
    if ok as ::core::ffi::c_int != 0
        && pr_get(d, (*d).trailer, b"Root\0" as *const u8 as *const ::core::ffi::c_char)
            .is_null()
    {
        ok = false_0 != 0;
    }
    if !ok && !reconstruct(d) {
        pr_close(d);
        return ::core::ptr::null_mut::<pr_doc>();
    }
    return d;
}
#[no_mangle]
pub unsafe extern "C" fn pr_close(mut d: *mut pr_doc) {
    if d.is_null() {
        return;
    }
    let mut c: *mut arena_chunk = (*d).arena;
    while !c.is_null() {
        let mut n: *mut arena_chunk = (*c).next as *mut arena_chunk;
        free(c as *mut ::core::ffi::c_void);
        c = n;
    }
    free((*d).xref as *mut ::core::ffi::c_void);
    free((*d).pages as *mut ::core::ffi::c_void);
    free(d as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn pr_is_encrypted(mut d: *mut pr_doc) -> bool {
    return !pr_dict_raw(
            (*d).trailer,
            b"Encrypt\0" as *const u8 as *const ::core::ffi::c_char,
        )
        .is_null();
}
unsafe extern "C" fn collect_pages(
    mut d: *mut pr_doc,
    mut node: *mut pr_obj,
    mut inherit: page_entry,
    mut depth: ::core::ffi::c_int,
) {
    if node.is_null()
        || (*node).type_0 as ::core::ffi::c_uint
            != PR_DICT as ::core::ffi::c_int as ::core::ffi::c_uint
        || depth > 64 as ::core::ffi::c_int
        || (*d).npages > 1000000 as ::core::ffi::c_int
    {
        return;
    }
    let mut v: *mut pr_obj = ::core::ptr::null_mut::<pr_obj>();
    v = pr_get(d, node, b"Resources\0" as *const u8 as *const ::core::ffi::c_char);
    if !v.is_null() {
        inherit.resources = v;
    }
    v = pr_get(d, node, b"MediaBox\0" as *const u8 as *const ::core::ffi::c_char);
    if !v.is_null() {
        inherit.mediabox = v;
    }
    v = pr_get(d, node, b"CropBox\0" as *const u8 as *const ::core::ffi::c_char);
    if !v.is_null() {
        inherit.cropbox = v;
    }
    let mut rot: ::core::ffi::c_double = 0.;
    if pr_number(
        pr_get(d, node, b"Rotate\0" as *const u8 as *const ::core::ffi::c_char),
        &raw mut rot,
    ) {
        inherit.rotate = rot as ::core::ffi::c_int;
    }
    let mut kids: *mut pr_obj = pr_get(
        d,
        node,
        b"Kids\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut type_0: *mut pr_obj = pr_get(
        d,
        node,
        b"Type\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if !kids.is_null()
        && (*kids).type_0 as ::core::ffi::c_uint
            == PR_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        && !name_is(type_0, b"Page\0" as *const u8 as *const ::core::ffi::c_char)
    {
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < (*kids).u.arr.n {
            let mut k: *mut pr_obj = *(*kids).u.arr.items.offset(i as isize);
            if (*k).type_0 as ::core::ffi::c_uint
                == PR_REF as ::core::ffi::c_int as ::core::ffi::c_uint
                && (*k).u.ref_0.num >= 0 as ::core::ffi::c_int
                && (*k).u.ref_0.num < (*d).nxref
            {
                let mut e: *mut xref_entry = (*d).xref.offset((*k).u.ref_0.num as isize)
                    as *mut xref_entry;
                if !((*e).loading != 0) {
                    let mut kd: *mut pr_obj = pr_resolve(d, k);
                    (*e).loading = 1 as ::core::ffi::c_uchar;
                    collect_pages(d, kd, inherit, depth + 1 as ::core::ffi::c_int);
                    (*e).loading = 0 as ::core::ffi::c_uchar;
                }
            } else {
                collect_pages(
                    d,
                    pr_resolve(d, k),
                    inherit,
                    depth + 1 as ::core::ffi::c_int,
                );
            }
            i += 1;
        }
        return;
    }
    if (*d).npages == (*d).cappages {
        (*d).cappages = if (*d).cappages != 0 {
            (*d).cappages * 2 as ::core::ffi::c_int
        } else {
            16 as ::core::ffi::c_int
        };
        (*d).pages = realloc(
            (*d).pages as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<page_entry>() as size_t)
                .wrapping_mul((*d).cappages as size_t),
        ) as *mut page_entry;
        if (*d).pages.is_null() {
            abort();
        }
    }
    inherit.dict = node;
    let fresh17 = (*d).npages;
    (*d).npages = (*d).npages + 1;
    *(*d).pages.offset(fresh17 as isize) = inherit;
}
unsafe extern "C" fn load_pages(mut d: *mut pr_doc) {
    if (*d).pages_loaded {
        return;
    }
    (*d).pages_loaded = true_0 != 0;
    let mut root: *mut pr_obj = pr_get(
        d,
        (*d).trailer,
        b"Root\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut pages: *mut pr_obj = pr_get(
        d,
        root,
        b"Pages\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut base: page_entry = page_entry {
        dict: ::core::ptr::null_mut::<pr_obj>(),
        resources: ::core::ptr::null_mut::<pr_obj>(),
        mediabox: ::core::ptr::null_mut::<pr_obj>(),
        cropbox: ::core::ptr::null_mut::<pr_obj>(),
        rotate: 0,
    };
    collect_pages(d, pages, base, 0 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn pr_page_count(mut d: *mut pr_doc) -> ::core::ffi::c_int {
    load_pages(d);
    return (*d).npages;
}
#[no_mangle]
pub unsafe extern "C" fn pr_normalize_page(
    mut d: *mut pr_doc,
    mut page: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut pages: ::core::ffi::c_int = pr_page_count(d);
    if page > pages {
        page = pages;
    }
    if page < 0 as ::core::ffi::c_int {
        page = pages + 1 as ::core::ffi::c_int + page;
    }
    if page < 1 as ::core::ffi::c_int {
        page = 1 as ::core::ffi::c_int;
    }
    return page;
}
unsafe extern "C" fn read_rect(
    mut d: *mut pr_doc,
    mut a: *mut pr_obj,
    mut r: *mut ::core::ffi::c_double,
) -> bool {
    a = pr_resolve(d, a);
    if a.is_null()
        || (*a).type_0 as ::core::ffi::c_uint
            != PR_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*a).u.arr.n < 4 as ::core::ffi::c_int
    {
        return false_0 != 0;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        if !pr_number(
            pr_resolve(d, *(*a).u.arr.items.offset(i as isize)),
            r.offset(i as isize) as *mut ::core::ffi::c_double,
        ) {
            return false_0 != 0;
        }
        i += 1;
    }
    let mut x0: ::core::ffi::c_double = fmin(
        *r.offset(0 as ::core::ffi::c_int as isize),
        *r.offset(2 as ::core::ffi::c_int as isize),
    );
    let mut x1: ::core::ffi::c_double = fmax(
        *r.offset(0 as ::core::ffi::c_int as isize),
        *r.offset(2 as ::core::ffi::c_int as isize),
    );
    let mut y0: ::core::ffi::c_double = fmin(
        *r.offset(1 as ::core::ffi::c_int as isize),
        *r.offset(3 as ::core::ffi::c_int as isize),
    );
    let mut y1: ::core::ffi::c_double = fmax(
        *r.offset(1 as ::core::ffi::c_int as isize),
        *r.offset(3 as ::core::ffi::c_int as isize),
    );
    *r.offset(0 as ::core::ffi::c_int as isize) = x0;
    *r.offset(1 as ::core::ffi::c_int as isize) = y0;
    *r.offset(2 as ::core::ffi::c_int as isize) = x1;
    *r.offset(3 as ::core::ffi::c_int as isize) = y1;
    return true_0 != 0;
}
unsafe extern "C" fn intersect(
    mut a: *mut ::core::ffi::c_double,
    mut b: *const ::core::ffi::c_double,
) {
    *a.offset(0 as ::core::ffi::c_int as isize) = fmax(
        *a.offset(0 as ::core::ffi::c_int as isize),
        *b.offset(0 as ::core::ffi::c_int as isize),
    );
    *a.offset(1 as ::core::ffi::c_int as isize) = fmax(
        *a.offset(1 as ::core::ffi::c_int as isize),
        *b.offset(1 as ::core::ffi::c_int as isize),
    );
    *a.offset(2 as ::core::ffi::c_int as isize) = fmin(
        *a.offset(2 as ::core::ffi::c_int as isize),
        *b.offset(2 as ::core::ffi::c_int as isize),
    );
    *a.offset(3 as ::core::ffi::c_int as isize) = fmin(
        *a.offset(3 as ::core::ffi::c_int as isize),
        *b.offset(3 as ::core::ffi::c_int as isize),
    );
    if *a.offset(2 as ::core::ffi::c_int as isize)
        < *a.offset(0 as ::core::ffi::c_int as isize)
    {
        *a.offset(2 as ::core::ffi::c_int as isize) = *a
            .offset(0 as ::core::ffi::c_int as isize);
    }
    if *a.offset(3 as ::core::ffi::c_int as isize)
        < *a.offset(1 as ::core::ffi::c_int as isize)
    {
        *a.offset(3 as ::core::ffi::c_int as isize) = *a
            .offset(1 as ::core::ffi::c_int as isize);
    }
}
#[no_mangle]
pub unsafe extern "C" fn pr_page(
    mut d: *mut pr_doc,
    mut index0: ::core::ffi::c_int,
    mut kind: ::core::ffi::c_int,
    mut out: *mut pr_page_info,
) -> bool {
    load_pages(d);
    if index0 < 0 as ::core::ffi::c_int || index0 >= (*d).npages {
        return false_0 != 0;
    }
    let mut p: *mut page_entry = (*d).pages.offset(index0 as isize) as *mut page_entry;
    let mut media: [::core::ffi::c_double; 4] = [
        0 as ::core::ffi::c_int as ::core::ffi::c_double,
        0 as ::core::ffi::c_int as ::core::ffi::c_double,
        612 as ::core::ffi::c_int as ::core::ffi::c_double,
        792 as ::core::ffi::c_int as ::core::ffi::c_double,
    ];
    let mut crop: [::core::ffi::c_double; 4] = [0.; 4];
    let mut box_0: [::core::ffi::c_double; 4] = [0.; 4];
    read_rect(d, (*p).mediabox, &raw mut media as *mut ::core::ffi::c_double);
    memcpy(
        &raw mut crop as *mut ::core::ffi::c_double as *mut ::core::ffi::c_void,
        &raw mut media as *mut ::core::ffi::c_double as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[::core::ffi::c_double; 4]>() as size_t,
    );
    if read_rect(d, (*p).cropbox, &raw mut crop as *mut ::core::ffi::c_double) {
        intersect(
            &raw mut crop as *mut ::core::ffi::c_double,
            &raw mut media as *mut ::core::ffi::c_double as *const ::core::ffi::c_double,
        );
    }
    memcpy(
        &raw mut box_0 as *mut ::core::ffi::c_double as *mut ::core::ffi::c_void,
        &raw mut crop as *mut ::core::ffi::c_double as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[::core::ffi::c_double; 4]>() as size_t,
    );
    let mut key: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    match kind {
        2 => {
            memcpy(
                &raw mut box_0 as *mut ::core::ffi::c_double as *mut ::core::ffi::c_void,
                &raw mut media as *mut ::core::ffi::c_double
                    as *const ::core::ffi::c_void,
                ::core::mem::size_of::<[::core::ffi::c_double; 4]>() as size_t,
            );
        }
        3 => {
            key = b"BleedBox\0" as *const u8 as *const ::core::ffi::c_char;
        }
        4 => {
            key = b"TrimBox\0" as *const u8 as *const ::core::ffi::c_char;
        }
        5 => {
            key = b"ArtBox\0" as *const u8 as *const ::core::ffi::c_char;
        }
        _ => {}
    }
    if !key.is_null()
        && read_rect(
            d,
            pr_get(d, (*p).dict, key),
            &raw mut box_0 as *mut ::core::ffi::c_double,
        ) as ::core::ffi::c_int != 0
    {
        intersect(
            &raw mut box_0 as *mut ::core::ffi::c_double,
            &raw mut media as *mut ::core::ffi::c_double as *const ::core::ffi::c_double,
        );
    }
    memcpy(
        &raw mut (*out).box_0 as *mut ::core::ffi::c_double as *mut ::core::ffi::c_void,
        &raw mut box_0 as *mut ::core::ffi::c_double as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[::core::ffi::c_double; 4]>() as size_t,
    );
    let mut r: ::core::ffi::c_int = (*p).rotate % 360 as ::core::ffi::c_int;
    if r < 0 as ::core::ffi::c_int {
        r += 360 as ::core::ffi::c_int;
    }
    (*out).rotate = r / 90 as ::core::ffi::c_int * 90 as ::core::ffi::c_int;
    (*out).page = (*p).dict;
    (*out).resources = (*p).resources;
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn pr_rotation_matrix(
    mut rotate: ::core::ffi::c_int,
    mut m: *mut ::core::ffi::c_double,
) {
    let mut a: ::core::ffi::c_double = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut b: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut c: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut dd: ::core::ffi::c_double = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
    match (rotate % 360 as ::core::ffi::c_int + 360 as ::core::ffi::c_int)
        % 360 as ::core::ffi::c_int
    {
        90 => {
            a = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
            b = -(1 as ::core::ffi::c_int) as ::core::ffi::c_double;
            c = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
            dd = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
        }
        180 => {
            a = -(1 as ::core::ffi::c_int) as ::core::ffi::c_double;
            b = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
            c = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
            dd = -(1 as ::core::ffi::c_int) as ::core::ffi::c_double;
        }
        270 => {
            a = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
            b = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
            c = -(1 as ::core::ffi::c_int) as ::core::ffi::c_double;
            dd = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
        }
        _ => {}
    }
    *m.offset(0 as ::core::ffi::c_int as isize) = a;
    *m.offset(1 as ::core::ffi::c_int as isize) = b;
    *m.offset(2 as ::core::ffi::c_int as isize) = c;
    *m.offset(3 as ::core::ffi::c_int as isize) = dd;
    *m.offset(4 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int
        as ::core::ffi::c_double;
    *m.offset(5 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_int
        as ::core::ffi::c_double;
}
#[no_mangle]
pub unsafe extern "C" fn pr_transform_box(
    mut m: *const ::core::ffi::c_double,
    mut box_0: *const ::core::ffi::c_double,
    mut out: *mut ::core::ffi::c_double,
) {
    let mut xs: [::core::ffi::c_double; 4] = [
        *box_0.offset(0 as ::core::ffi::c_int as isize),
        *box_0.offset(2 as ::core::ffi::c_int as isize),
        *box_0.offset(2 as ::core::ffi::c_int as isize),
        *box_0.offset(0 as ::core::ffi::c_int as isize),
    ];
    let mut ys: [::core::ffi::c_double; 4] = [
        *box_0.offset(1 as ::core::ffi::c_int as isize),
        *box_0.offset(1 as ::core::ffi::c_int as isize),
        *box_0.offset(3 as ::core::ffi::c_int as isize),
        *box_0.offset(3 as ::core::ffi::c_int as isize),
    ];
    let mut x0: ::core::ffi::c_double = ::core::f32::INFINITY as ::core::ffi::c_double;
    let mut y0: ::core::ffi::c_double = ::core::f32::INFINITY as ::core::ffi::c_double;
    let mut x1: ::core::ffi::c_double = -::core::f32::INFINITY as ::core::ffi::c_double;
    let mut y1: ::core::ffi::c_double = -::core::f32::INFINITY as ::core::ffi::c_double;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        let mut x: ::core::ffi::c_double = *m.offset(0 as ::core::ffi::c_int as isize)
            * xs[i as usize]
            + *m.offset(2 as ::core::ffi::c_int as isize) * ys[i as usize]
            + *m.offset(4 as ::core::ffi::c_int as isize);
        let mut y: ::core::ffi::c_double = *m.offset(1 as ::core::ffi::c_int as isize)
            * xs[i as usize]
            + *m.offset(3 as ::core::ffi::c_int as isize) * ys[i as usize]
            + *m.offset(5 as ::core::ffi::c_int as isize);
        x0 = fmin(x0, x);
        y0 = fmin(y0, y);
        x1 = fmax(x1, x);
        y1 = fmax(y1, y);
        i += 1;
    }
    *out.offset(0 as ::core::ffi::c_int as isize) = x0;
    *out.offset(1 as ::core::ffi::c_int as isize) = y0;
    *out.offset(2 as ::core::ffi::c_int as isize) = x1;
    *out.offset(3 as ::core::ffi::c_int as isize) = y1;
}

// Native policy client reads the source information dictionary as data.
#[no_mangle]
pub unsafe extern "C" fn pr_document_info(doc:*mut pr_doc)->*mut pr_obj {
    if doc.is_null(){return std::ptr::null_mut()}
    pr_get(doc,(*doc).trailer,b"Info\0".as_ptr().cast())
}

// Effective declared version: the catalog may raise a document's header.
#[no_mangle]
pub unsafe extern "C" fn pr_version(doc:*mut pr_doc)->i32 {
    if doc.is_null(){return 10;}
    let data=std::slice::from_raw_parts((*doc).data,(*doc).len.min(1024));let mut version=10;
    if let Some(at)=data.windows(5).position(|p|p==b"%PDF-"){if let(Some(major),Some(dot),Some(minor))=(data.get(at+5),data.get(at+6),data.get(at+7)){if *dot==b'.'&&major.is_ascii_digit()&&minor.is_ascii_digit(){version=(major-b'0') as i32*10+(minor-b'0') as i32;}}}
    let root=pr_get(doc,(*doc).trailer,b"Root\0".as_ptr().cast());let value=pr_get(doc,root,b"Version\0".as_ptr().cast());
    if !value.is_null()&&(*value).type_0==PR_NAME {let bytes=std::slice::from_raw_parts((*value).u.str_0.s.cast::<u8>(),(*value).u.str_0.len);if bytes.len()>=3&&bytes[0].is_ascii_digit()&&bytes[1]==b'.'&&bytes[2].is_ascii_digit(){version=version.max((bytes[0]-b'0') as i32*10+(bytes[2]-b'0') as i32);}}
    version
}
