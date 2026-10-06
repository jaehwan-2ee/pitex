/* Pitex embedded preview engine — minimal JSON reader/writer.
 * Pitex-authored (AGPL-3.0-or-later). */
// Translated from driver/json.c with C2Rust 0.22.1.
extern "C" {
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn abort() -> !;
    fn strtod(
        _: *const ::core::ffi::c_char,
        _: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn memcpy(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strchr(__s: *const ::core::ffi::c_char, __c: ::core::ffi::c_int)
        -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn pbuf_printf(b: *mut pbuf, fmt: *const ::core::ffi::c_char, ...);
}
pub type __darwin_size_t = usize;
pub type size_t = __darwin_size_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
}
pub type json_type = ::core::ffi::c_uint;
pub const JSON_OBJECT: json_type = 5;
pub const JSON_ARRAY: json_type = 4;
pub const JSON_STRING: json_type = 3;
pub const JSON_NUMBER: json_type = 2;
pub const JSON_BOOL: json_type = 1;
pub const JSON_NULL: json_type = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct json {
    pub type_0: json_type,
    pub b: bool,
    pub num: ::core::ffi::c_double,
    pub str_0: *mut ::core::ffi::c_char,
    pub len: size_t,
    pub items: *mut *mut json,
    pub keys: *mut *mut ::core::ffi::c_char,
    pub n: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct jparser {
    pub p: *const ::core::ffi::c_char,
    pub end: *const ::core::ffi::c_char,
    pub depth: ::core::ffi::c_int,
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
#[inline]
unsafe extern "C" fn pbuf_reserve(mut b: *mut pbuf, mut extra: size_t) {
    if (*b).len.wrapping_add(extra) <= (*b).cap {
        return;
    }
    let mut cap: size_t = if (*b).cap != 0 {
        (*b).cap
    } else {
        256 as size_t
    };
    while cap < (*b).len.wrapping_add(extra) {
        cap = cap.wrapping_mul(2 as size_t);
    }
    let mut p: *mut ::core::ffi::c_uchar =
        realloc((*b).data as *mut ::core::ffi::c_void, cap) as *mut ::core::ffi::c_uchar;
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
    memcpy(
        (*b).data.offset((*b).len as isize) as *mut ::core::ffi::c_void,
        data,
        len,
    );
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
unsafe extern "C" fn ws(mut j: *mut jparser) {
    while (*j).p < (*j).end
        && (*(*j).p as ::core::ffi::c_int == ' ' as i32
            || *(*j).p as ::core::ffi::c_int == '\t' as i32
            || *(*j).p as ::core::ffi::c_int == '\n' as i32
            || *(*j).p as ::core::ffi::c_int == '\r' as i32)
    {
        (*j).p = (*j).p.offset(1);
    }
}
unsafe extern "C" fn jnew(mut t: json_type) -> *mut json {
    let mut v: *mut json =
        calloc(1 as size_t, ::core::mem::size_of::<json>() as size_t) as *mut json;
    if v.is_null() {
        abort();
    }
    (*v).type_0 = t;
    return v;
}
unsafe extern "C" fn put_utf8(mut b: *mut pbuf, mut cp: ::core::ffi::c_uint) {
    if cp < 0x80 as ::core::ffi::c_uint {
        pbuf_putc(b, cp as ::core::ffi::c_int);
    } else if cp < 0x800 as ::core::ffi::c_uint {
        pbuf_putc(
            b,
            (0xc0 as ::core::ffi::c_uint | cp >> 6 as ::core::ffi::c_int) as ::core::ffi::c_int,
        );
        pbuf_putc(
            b,
            (0x80 as ::core::ffi::c_uint | cp & 0x3f as ::core::ffi::c_uint) as ::core::ffi::c_int,
        );
    } else if cp < 0x10000 as ::core::ffi::c_int as ::core::ffi::c_uint {
        pbuf_putc(
            b,
            (0xe0 as ::core::ffi::c_uint | cp >> 12 as ::core::ffi::c_int) as ::core::ffi::c_int,
        );
        pbuf_putc(
            b,
            (0x80 as ::core::ffi::c_uint
                | cp >> 6 as ::core::ffi::c_int & 0x3f as ::core::ffi::c_uint)
                as ::core::ffi::c_int,
        );
        pbuf_putc(
            b,
            (0x80 as ::core::ffi::c_uint | cp & 0x3f as ::core::ffi::c_uint) as ::core::ffi::c_int,
        );
    } else {
        pbuf_putc(
            b,
            (0xf0 as ::core::ffi::c_uint | cp >> 18 as ::core::ffi::c_int) as ::core::ffi::c_int,
        );
        pbuf_putc(
            b,
            (0x80 as ::core::ffi::c_uint
                | cp >> 12 as ::core::ffi::c_int & 0x3f as ::core::ffi::c_uint)
                as ::core::ffi::c_int,
        );
        pbuf_putc(
            b,
            (0x80 as ::core::ffi::c_uint
                | cp >> 6 as ::core::ffi::c_int & 0x3f as ::core::ffi::c_uint)
                as ::core::ffi::c_int,
        );
        pbuf_putc(
            b,
            (0x80 as ::core::ffi::c_uint | cp & 0x3f as ::core::ffi::c_uint) as ::core::ffi::c_int,
        );
    };
}
unsafe extern "C" fn hex4(
    mut p: *const ::core::ffi::c_char,
    mut out: *mut ::core::ffi::c_uint,
) -> ::core::ffi::c_int {
    let mut v: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 4 as ::core::ffi::c_int {
        let mut c: ::core::ffi::c_int = *p.offset(i as isize) as ::core::ffi::c_int;
        let mut d: ::core::ffi::c_int = 0;
        if c >= '0' as i32 && c <= '9' as i32 {
            d = c - '0' as i32;
        } else if c >= 'a' as i32 && c <= 'f' as i32 {
            d = c - 'a' as i32 + 10 as ::core::ffi::c_int;
        } else if c >= 'A' as i32 && c <= 'F' as i32 {
            d = c - 'A' as i32 + 10 as ::core::ffi::c_int;
        } else {
            return -(1 as ::core::ffi::c_int);
        }
        v = v
            .wrapping_mul(16 as ::core::ffi::c_uint)
            .wrapping_add(d as ::core::ffi::c_uint);
        i += 1;
    }
    *out = v;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn parse_string_raw(
    mut j: *mut jparser,
    mut len: *mut size_t,
) -> *mut ::core::ffi::c_char {
    let mut current_block: u64;
    if (*j).p >= (*j).end || *(*j).p as ::core::ffi::c_int != '"' as i32 {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    (*j).p = (*j).p.offset(1);
    let mut b: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    loop {
        if !((*j).p < (*j).end && *(*j).p as ::core::ffi::c_int != '"' as i32) {
            current_block = 11459959175219260272;
            break;
        }
        let fresh5 = (*j).p;
        (*j).p = (*j).p.offset(1);
        let mut c: ::core::ffi::c_uchar = *fresh5 as ::core::ffi::c_uchar;
        if c as ::core::ffi::c_int != '\\' as i32 {
            pbuf_putc(&raw mut b, c as ::core::ffi::c_int);
        } else {
            if (*j).p >= (*j).end {
                current_block = 12400818877071818078;
                break;
            }
            let fresh6 = (*j).p;
            (*j).p = (*j).p.offset(1);
            let mut e: ::core::ffi::c_char = *fresh6;
            match e as ::core::ffi::c_int {
                34 => {
                    pbuf_putc(&raw mut b, '"' as i32);
                }
                92 => {
                    pbuf_putc(&raw mut b, '\\' as i32);
                }
                47 => {
                    pbuf_putc(&raw mut b, '/' as i32);
                }
                98 => {
                    pbuf_putc(&raw mut b, '\u{8}' as i32);
                }
                102 => {
                    pbuf_putc(&raw mut b, '\u{c}' as i32);
                }
                110 => {
                    pbuf_putc(&raw mut b, '\n' as i32);
                }
                114 => {
                    pbuf_putc(&raw mut b, '\r' as i32);
                }
                116 => {
                    pbuf_putc(&raw mut b, '\t' as i32);
                }
                117 => {
                    let mut cp: ::core::ffi::c_uint = 0;
                    if ((*j).end.offset_from((*j).p) as ::core::ffi::c_long)
                        < 4 as ::core::ffi::c_long
                        || hex4((*j).p, &raw mut cp) != 0
                    {
                        current_block = 12400818877071818078;
                        break;
                    }
                    (*j).p = (*j).p.offset(4 as ::core::ffi::c_int as isize);
                    if cp >= 0xd800 as ::core::ffi::c_uint
                        && cp < 0xdc00 as ::core::ffi::c_uint
                        && (*j).end.offset_from((*j).p) as ::core::ffi::c_long
                            >= 6 as ::core::ffi::c_long
                        && *(*j).p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == '\\' as i32
                        && *(*j).p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == 'u' as i32
                    {
                        let mut lo: ::core::ffi::c_uint = 0;
                        if hex4((*j).p.offset(2 as ::core::ffi::c_int as isize), &raw mut lo) == 0
                            && lo >= 0xdc00 as ::core::ffi::c_uint
                            && lo < 0xe000 as ::core::ffi::c_uint
                        {
                            (*j).p = (*j).p.offset(6 as ::core::ffi::c_int as isize);
                            cp = (0x10000 as ::core::ffi::c_int as ::core::ffi::c_uint)
                                .wrapping_add(
                                    cp.wrapping_sub(0xd800 as ::core::ffi::c_uint)
                                        << 10 as ::core::ffi::c_int,
                                )
                                .wrapping_add(lo.wrapping_sub(0xdc00 as ::core::ffi::c_uint));
                        }
                    }
                    if cp >= 0xd800 as ::core::ffi::c_uint && cp < 0xe000 as ::core::ffi::c_uint {
                        cp = 0xfffd as ::core::ffi::c_uint;
                    }
                    put_utf8(&raw mut b, cp);
                }
                _ => {
                    current_block = 12400818877071818078;
                    break;
                }
            }
        }
    }
    match current_block {
        11459959175219260272 => {
            if !((*j).p >= (*j).end) {
                (*j).p = (*j).p.offset(1);
                pbuf_putc(&raw mut b, 0 as ::core::ffi::c_int);
                *len = b.len.wrapping_sub(1 as size_t);
                return b.data as *mut ::core::ffi::c_char;
            }
        }
        _ => {}
    }
    pbuf_free(&raw mut b);
    return ::core::ptr::null_mut::<::core::ffi::c_char>();
}
unsafe extern "C" fn parse_value(mut j: *mut jparser) -> *mut json {
    let mut current_block: u64;
    ws(j);
    if (*j).p >= (*j).end || {
        (*j).depth += 1;
        (*j).depth > 64 as ::core::ffi::c_int
    } {
        return ::core::ptr::null_mut::<json>();
    }
    let mut v: *mut json = ::core::ptr::null_mut::<json>();
    let mut c: ::core::ffi::c_char = *(*j).p;
    if c as ::core::ffi::c_int == '{' as i32 || c as ::core::ffi::c_int == '[' as i32 {
        let mut obj: bool = c as ::core::ffi::c_int == '{' as i32;
        (*j).p = (*j).p.offset(1);
        v = jnew(
            (if obj as ::core::ffi::c_int != 0 {
                JSON_OBJECT as ::core::ffi::c_int
            } else {
                JSON_ARRAY as ::core::ffi::c_int
            }) as json_type,
        );
        let mut cap: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        ws(j);
        if (*j).p < (*j).end
            && *(*j).p as ::core::ffi::c_int
                == (if obj as ::core::ffi::c_int != 0 {
                    '}' as i32
                } else {
                    ']' as i32
                })
        {
            (*j).p = (*j).p.offset(1);
            (*j).depth -= 1;
            return v;
        }
        loop {
            let mut key: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
            let mut klen: size_t = 0;
            if obj {
                ws(j);
                key = parse_string_raw(j, &raw mut klen);
                ws(j);
                if key.is_null()
                    || (*j).p >= (*j).end
                    || *(*j).p as ::core::ffi::c_int != ':' as i32
                {
                    free(key as *mut ::core::ffi::c_void);
                    current_block = 477035891447098712;
                    break;
                } else {
                    (*j).p = (*j).p.offset(1);
                }
            }
            let mut item: *mut json = parse_value(j);
            if item.is_null() {
                free(key as *mut ::core::ffi::c_void);
                current_block = 477035891447098712;
                break;
            } else {
                if (*v).n == cap {
                    cap = if cap != 0 {
                        cap * 2 as ::core::ffi::c_int
                    } else {
                        8 as ::core::ffi::c_int
                    };
                    (*v).items = realloc(
                        (*v).items as *mut ::core::ffi::c_void,
                        (::core::mem::size_of::<*mut json>() as size_t).wrapping_mul(cap as size_t),
                    ) as *mut *mut json;
                    if obj {
                        (*v).keys = realloc(
                            (*v).keys as *mut ::core::ffi::c_void,
                            (::core::mem::size_of::<*mut ::core::ffi::c_char>() as size_t)
                                .wrapping_mul(cap as size_t),
                        ) as *mut *mut ::core::ffi::c_char;
                    }
                    if (*v).items.is_null() || obj as ::core::ffi::c_int != 0 && (*v).keys.is_null()
                    {
                        abort();
                    }
                }
                let ref mut fresh1 = *(*v).items.offset((*v).n as isize);
                *fresh1 = item;
                if obj {
                    let ref mut fresh2 = *(*v).keys.offset((*v).n as isize);
                    *fresh2 = key;
                }
                (*v).n += 1;
                ws(j);
                if (*j).p < (*j).end && *(*j).p as ::core::ffi::c_int == ',' as i32 {
                    (*j).p = (*j).p.offset(1);
                } else {
                    if !((*j).p < (*j).end
                        && *(*j).p as ::core::ffi::c_int
                            == (if obj as ::core::ffi::c_int != 0 {
                                '}' as i32
                            } else {
                                ']' as i32
                            }))
                    {
                        current_block = 477035891447098712;
                        break;
                    }
                    (*j).p = (*j).p.offset(1);
                    current_block = 17233182392562552756;
                    break;
                }
            }
        }
    } else if c as ::core::ffi::c_int == '"' as i32 {
        v = jnew(JSON_STRING);
        (*v).str_0 = parse_string_raw(j, &raw mut (*v).len);
        if (*v).str_0.is_null() {
            current_block = 477035891447098712;
        } else {
            current_block = 17233182392562552756;
        }
    } else {
        if c as ::core::ffi::c_int == 't' as i32
            && (*j).end.offset_from((*j).p) as ::core::ffi::c_long >= 4 as ::core::ffi::c_long
            && memcmp(
                (*j).p as *const ::core::ffi::c_void,
                b"true\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
                4 as size_t,
            ) == 0
        {
            v = jnew(JSON_BOOL);
            (*v).b = true_0 != 0;
            (*j).p = (*j).p.offset(4 as ::core::ffi::c_int as isize);
        } else if c as ::core::ffi::c_int == 'f' as i32
            && (*j).end.offset_from((*j).p) as ::core::ffi::c_long >= 5 as ::core::ffi::c_long
            && memcmp(
                (*j).p as *const ::core::ffi::c_void,
                b"false\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
                5 as size_t,
            ) == 0
        {
            v = jnew(JSON_BOOL);
            (*j).p = (*j).p.offset(5 as ::core::ffi::c_int as isize);
        } else if c as ::core::ffi::c_int == 'n' as i32
            && (*j).end.offset_from((*j).p) as ::core::ffi::c_long >= 4 as ::core::ffi::c_long
            && memcmp(
                (*j).p as *const ::core::ffi::c_void,
                b"null\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
                4 as size_t,
            ) == 0
        {
            v = jnew(JSON_NULL);
            (*j).p = (*j).p.offset(4 as ::core::ffi::c_int as isize);
        } else if c as ::core::ffi::c_int == '-' as i32
            || c as ::core::ffi::c_int >= '0' as i32 && c as ::core::ffi::c_int <= '9' as i32
        {
            let mut tmp: [::core::ffi::c_char; 64] = [0; 64];
            let mut n: size_t = 0 as size_t;
            while (*j).p < (*j).end
                && n < (::core::mem::size_of::<[::core::ffi::c_char; 64]>() as usize)
                    .wrapping_sub(1 as usize)
                && (!strchr(
                    b"+-.eE\0" as *const u8 as *const ::core::ffi::c_char,
                    *(*j).p as ::core::ffi::c_int,
                )
                .is_null()
                    || *(*j).p as ::core::ffi::c_int >= '0' as i32
                        && *(*j).p as ::core::ffi::c_int <= '9' as i32)
            {
                let fresh3 = (*j).p;
                (*j).p = (*j).p.offset(1);
                let fresh4 = n;
                n = n.wrapping_add(1);
                tmp[fresh4 as usize] = *fresh3;
            }
            tmp[n as usize] = 0 as ::core::ffi::c_char;
            v = jnew(JSON_NUMBER);
            (*v).num = strtod(
                &raw mut tmp as *mut ::core::ffi::c_char,
                ::core::ptr::null_mut::<*mut ::core::ffi::c_char>(),
            );
        } else {
            return ::core::ptr::null_mut::<json>();
        }
        current_block = 17233182392562552756;
    }
    match current_block {
        477035891447098712 => {
            json_free(v);
            return ::core::ptr::null_mut::<json>();
        }
        _ => {
            (*j).depth -= 1;
            return v;
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn json_parse(
    mut text: *const ::core::ffi::c_char,
    mut len: size_t,
) -> *mut json {
    let mut j: jparser = jparser {
        p: text,
        end: text.offset(len as isize),
        depth: 0 as ::core::ffi::c_int,
    };
    let mut v: *mut json = parse_value(&raw mut j);
    if v.is_null() {
        return ::core::ptr::null_mut::<json>();
    }
    ws(&raw mut j);
    if j.p != j.end {
        json_free(v);
        return ::core::ptr::null_mut::<json>();
    }
    return v;
}
#[no_mangle]
pub unsafe extern "C" fn json_free(mut v: *mut json) {
    if v.is_null() {
        return;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*v).n {
        json_free(*(*v).items.offset(i as isize));
        if !(*v).keys.is_null() {
            free(*(*v).keys.offset(i as isize) as *mut ::core::ffi::c_void);
        }
        i += 1;
    }
    free((*v).items as *mut ::core::ffi::c_void);
    free((*v).keys as *mut ::core::ffi::c_void);
    free((*v).str_0 as *mut ::core::ffi::c_void);
    free(v as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn json_get(
    mut o: *const json,
    mut key: *const ::core::ffi::c_char,
) -> *mut json {
    if o.is_null()
        || (*o).type_0 as ::core::ffi::c_uint
            != JSON_OBJECT as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ::core::ptr::null_mut::<json>();
    }
    let mut i: ::core::ffi::c_int = (*o).n - 1 as ::core::ffi::c_int;
    while i >= 0 as ::core::ffi::c_int {
        if strcmp(*(*o).keys.offset(i as isize), key) == 0 as ::core::ffi::c_int {
            return *(*o).items.offset(i as isize);
        }
        i -= 1;
    }
    return ::core::ptr::null_mut::<json>();
}
#[no_mangle]
pub unsafe extern "C" fn json_string(mut v: *const json) -> *const ::core::ffi::c_char {
    return if !v.is_null()
        && (*v).type_0 as ::core::ffi::c_uint
            == JSON_STRING as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        (*v).str_0
    } else {
        ::core::ptr::null_mut::<::core::ffi::c_char>()
    };
}
#[no_mangle]
pub unsafe extern "C" fn json_number(
    mut v: *const json,
    mut out: *mut ::core::ffi::c_double,
) -> bool {
    if v.is_null()
        || (*v).type_0 as ::core::ffi::c_uint
            != JSON_NUMBER as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return false_0 != 0;
    }
    *out = (*v).num;
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn json_write_string(
    mut out: *mut pbuf,
    mut s: *const ::core::ffi::c_char,
    mut len: size_t,
) {
    pbuf_putc(out, '"' as i32);
    let mut i: size_t = 0 as size_t;
    while i < len {
        let mut c: ::core::ffi::c_uchar = *s.offset(i as isize) as ::core::ffi::c_uchar;
        match c as ::core::ffi::c_int {
            34 => {
                pbuf_puts(out, b"\\\"\0" as *const u8 as *const ::core::ffi::c_char);
            }
            92 => {
                pbuf_puts(out, b"\\\\\0" as *const u8 as *const ::core::ffi::c_char);
            }
            10 => {
                pbuf_puts(out, b"\\n\0" as *const u8 as *const ::core::ffi::c_char);
            }
            13 => {
                pbuf_puts(out, b"\\r\0" as *const u8 as *const ::core::ffi::c_char);
            }
            9 => {
                pbuf_puts(out, b"\\t\0" as *const u8 as *const ::core::ffi::c_char);
            }
            _ => {
                if (c as ::core::ffi::c_int) < 0x20 as ::core::ffi::c_int {
                    pbuf_printf(
                        out,
                        b"\\u%04x\0" as *const u8 as *const ::core::ffi::c_char,
                        c as ::core::ffi::c_int,
                    );
                } else if (c as ::core::ffi::c_int) < 0x80 as ::core::ffi::c_int {
                    pbuf_putc(out, c as ::core::ffi::c_int);
                } else {
                    let mut n: ::core::ffi::c_int = if c as ::core::ffi::c_int
                        >= 0xf0 as ::core::ffi::c_int
                        && (c as ::core::ffi::c_int) < 0xf5 as ::core::ffi::c_int
                    {
                        4 as ::core::ffi::c_int
                    } else if c as ::core::ffi::c_int >= 0xe0 as ::core::ffi::c_int
                        && (c as ::core::ffi::c_int) < 0xf0 as ::core::ffi::c_int
                    {
                        3 as ::core::ffi::c_int
                    } else if c as ::core::ffi::c_int >= 0xc2 as ::core::ffi::c_int
                        && (c as ::core::ffi::c_int) < 0xe0 as ::core::ffi::c_int
                    {
                        2 as ::core::ffi::c_int
                    } else {
                        0 as ::core::ffi::c_int
                    };
                    let mut ok: bool =
                        n > 0 as ::core::ffi::c_int && i.wrapping_add(n as size_t) <= len;
                    let mut k: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
                    while ok as ::core::ffi::c_int != 0 && k < n {
                        ok = *s.offset(i.wrapping_add(k as size_t) as isize) as ::core::ffi::c_uchar
                            as ::core::ffi::c_int
                            & 0xc0 as ::core::ffi::c_int
                            == 0x80 as ::core::ffi::c_int;
                        k += 1;
                    }
                    if ok {
                        pbuf_append(
                            out,
                            s.offset(i as isize) as *const ::core::ffi::c_void,
                            n as size_t,
                        );
                        i = i.wrapping_add((n as size_t).wrapping_sub(1 as size_t));
                    } else {
                        pbuf_puts(out, b"\\ufffd\0" as *const u8 as *const ::core::ffi::c_char);
                    }
                }
            }
        }
        i = i.wrapping_add(1);
    }
    pbuf_putc(out, '"' as i32);
}
