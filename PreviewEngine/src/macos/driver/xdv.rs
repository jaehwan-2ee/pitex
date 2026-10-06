/* Pitex embedded preview engine — DVI/XDV stream index.
 * Independently written from the DVI format and XeTeX XDV extensions.
 * Pitex-authored (AGPL-3.0-or-later). */
// Translated from driver/xdv.c with C2Rust 0.22.1.
extern "C" {
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn abort() -> !;
    fn memset(
        __b: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type __darwin_size_t = usize;
pub type size_t = __darwin_size_t;
pub type uint32_t = u32;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const XDV_TEXT_AND_GLYPHS: C2RustUnnamed = 254;
pub const XDV_GLYPHS: C2RustUnnamed = 253;
pub const XDV_NATIVE_FONT_DEF: C2RustUnnamed = 252;
pub const DVI_POST_POST: C2RustUnnamed = 249;
pub const DVI_POST: C2RustUnnamed = 248;
pub const DVI_PRE: C2RustUnnamed = 247;
pub const DVI_FNT_DEF1: C2RustUnnamed = 243;
pub const DVI_XXX1: C2RustUnnamed = 239;
pub const DVI_FNT1: C2RustUnnamed = 235;
pub const DVI_FNT_NUM_0: C2RustUnnamed = 171;
pub const DVI_Z1: C2RustUnnamed = 167;
pub const DVI_Z0: C2RustUnnamed = 166;
pub const DVI_Y1: C2RustUnnamed = 162;
pub const DVI_Y0: C2RustUnnamed = 161;
pub const DVI_DOWN1: C2RustUnnamed = 157;
pub const DVI_X1: C2RustUnnamed = 153;
pub const DVI_X0: C2RustUnnamed = 152;
pub const DVI_W1: C2RustUnnamed = 148;
pub const DVI_W0: C2RustUnnamed = 147;
pub const DVI_RIGHT1: C2RustUnnamed = 143;
pub const DVI_POP: C2RustUnnamed = 142;
pub const DVI_PUSH: C2RustUnnamed = 141;
pub const DVI_EOP: C2RustUnnamed = 140;
pub const DVI_BOP: C2RustUnnamed = 139;
pub const DVI_NOP: C2RustUnnamed = 138;
pub const DVI_PUT_RULE: C2RustUnnamed = 137;
pub const DVI_PUT1: C2RustUnnamed = 133;
pub const DVI_SET_RULE: C2RustUnnamed = 132;
pub const DVI_SET1: C2RustUnnamed = 128;
pub const DVI_SET_CHAR_0: C2RustUnnamed = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xdv_page {
    pub bop: size_t,
    pub eop_end: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct xdv_index {
    pub have_pre: bool,
    pub num: uint32_t,
    pub den: uint32_t,
    pub mag: uint32_t,
    pub scan: size_t,
    pub in_page: bool,
    pub cur_bop: size_t,
    pub pages: *mut xdv_page,
    pub npages: ::core::ffi::c_int,
    pub cap: ::core::ffi::c_int,
    pub broken: bool,
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
unsafe extern "C" fn be(mut p: *const ::core::ffi::c_uchar, mut n: ::core::ffi::c_int) -> uint32_t {
    let mut v: uint32_t = 0 as uint32_t;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < n {
        v = v << 8 as ::core::ffi::c_int | *p.offset(i as isize) as uint32_t;
        i += 1;
    }
    return v;
}
#[no_mangle]
pub unsafe extern "C" fn xdv_insn_length(
    mut p: *const ::core::ffi::c_uchar,
    mut end: *const ::core::ffi::c_uchar,
) -> ::core::ffi::c_long {
    let mut avail: ::core::ffi::c_long = end.offset_from(p) as ::core::ffi::c_long;
    if avail < 1 as ::core::ffi::c_long {
        return 0 as ::core::ffi::c_long;
    }
    let mut op: ::core::ffi::c_int =
        *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
    if op < 128 as ::core::ffi::c_int
        || op >= DVI_FNT_NUM_0 as ::core::ffi::c_int && op < DVI_FNT1 as ::core::ffi::c_int
        || op == DVI_NOP as ::core::ffi::c_int
        || op == DVI_PUSH as ::core::ffi::c_int
        || op == DVI_POP as ::core::ffi::c_int
        || op == DVI_EOP as ::core::ffi::c_int
        || op == DVI_W0 as ::core::ffi::c_int
        || op == DVI_X0 as ::core::ffi::c_int
        || op == DVI_Y0 as ::core::ffi::c_int
        || op == DVI_Z0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_long;
    }
    if op >= DVI_SET1 as ::core::ffi::c_int
        && op < DVI_SET1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_SET1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op >= DVI_PUT1 as ::core::ffi::c_int
        && op < DVI_PUT1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_PUT1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op == DVI_SET_RULE as ::core::ffi::c_int || op == DVI_PUT_RULE as ::core::ffi::c_int {
        return 9 as ::core::ffi::c_long;
    }
    if op == DVI_BOP as ::core::ffi::c_int {
        return 45 as ::core::ffi::c_long;
    }
    if op >= DVI_RIGHT1 as ::core::ffi::c_int
        && op < DVI_RIGHT1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_RIGHT1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op >= DVI_W1 as ::core::ffi::c_int
        && op < DVI_W1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_W1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op >= DVI_X1 as ::core::ffi::c_int
        && op < DVI_X1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_X1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op >= DVI_DOWN1 as ::core::ffi::c_int
        && op < DVI_DOWN1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_DOWN1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op >= DVI_Y1 as ::core::ffi::c_int
        && op < DVI_Y1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_Y1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op >= DVI_Z1 as ::core::ffi::c_int
        && op < DVI_Z1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_Z1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op >= DVI_FNT1 as ::core::ffi::c_int
        && op < DVI_FNT1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        return (1 as ::core::ffi::c_int
            + (op - DVI_FNT1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int))
            as ::core::ffi::c_long;
    }
    if op >= DVI_XXX1 as ::core::ffi::c_int
        && op < DVI_XXX1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        let mut k: ::core::ffi::c_int =
            op - DVI_XXX1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int;
        if avail < (1 as ::core::ffi::c_int + k) as ::core::ffi::c_long {
            return 0 as ::core::ffi::c_long;
        }
        let mut len: uint32_t = be(p.offset(1 as ::core::ffi::c_int as isize), k);
        if len > (1 as uint32_t) << 30 as ::core::ffi::c_int {
            return -(1 as ::core::ffi::c_int) as ::core::ffi::c_long;
        }
        if avail < (1 as ::core::ffi::c_int + k) as ::core::ffi::c_long + len as ::core::ffi::c_long
        {
            return 0 as ::core::ffi::c_long;
        }
        return (1 as ::core::ffi::c_int + k) as ::core::ffi::c_long + len as ::core::ffi::c_long;
    }
    if op >= DVI_FNT_DEF1 as ::core::ffi::c_int
        && op < DVI_FNT_DEF1 as ::core::ffi::c_int + 4 as ::core::ffi::c_int
    {
        let mut k_0: ::core::ffi::c_int =
            op - DVI_FNT_DEF1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int;
        if avail < (1 as ::core::ffi::c_int + k_0 + 14 as ::core::ffi::c_int) as ::core::ffi::c_long
        {
            return 0 as ::core::ffi::c_long;
        }
        let mut a: ::core::ffi::c_int = *p
            .offset((1 as ::core::ffi::c_int + k_0 + 12 as ::core::ffi::c_int) as isize)
            as ::core::ffi::c_int;
        let mut l: ::core::ffi::c_int = *p
            .offset((1 as ::core::ffi::c_int + k_0 + 13 as ::core::ffi::c_int) as isize)
            as ::core::ffi::c_int;
        if avail
            < (1 as ::core::ffi::c_int + k_0 + 14 as ::core::ffi::c_int + a + l)
                as ::core::ffi::c_long
        {
            return 0 as ::core::ffi::c_long;
        }
        return (1 as ::core::ffi::c_int + k_0 + 14 as ::core::ffi::c_int + a + l)
            as ::core::ffi::c_long;
    }
    if op == DVI_PRE as ::core::ffi::c_int {
        if avail < 15 as ::core::ffi::c_int as ::core::ffi::c_long {
            return 0 as ::core::ffi::c_long;
        }
        if avail
            < (15 as ::core::ffi::c_int
                + *p.offset(14 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                as ::core::ffi::c_long
        {
            return 0 as ::core::ffi::c_long;
        }
        return (15 as ::core::ffi::c_int
            + *p.offset(14 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            as ::core::ffi::c_long;
    }
    if op == XDV_NATIVE_FONT_DEF as ::core::ffi::c_int {
        if avail < 12 as ::core::ffi::c_int as ::core::ffi::c_long {
            return 0 as ::core::ffi::c_long;
        }
        let mut flags: uint32_t = be(
            p.offset(9 as ::core::ffi::c_int as isize),
            2 as ::core::ffi::c_int,
        );
        let mut namelen: ::core::ffi::c_int =
            *p.offset(11 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
        let mut len_0: ::core::ffi::c_long =
            (12 as ::core::ffi::c_int + namelen + 4 as ::core::ffi::c_int) as ::core::ffi::c_long;
        if flags & 0x200 as uint32_t != 0 {
            len_0 += 4 as ::core::ffi::c_long;
        }
        if flags & 0x1000 as uint32_t != 0 {
            len_0 += 4 as ::core::ffi::c_long;
        }
        if flags & 0x2000 as uint32_t != 0 {
            len_0 += 4 as ::core::ffi::c_long;
        }
        if flags & 0x4000 as uint32_t != 0 {
            len_0 += 4 as ::core::ffi::c_long;
        }
        if avail < len_0 {
            return 0 as ::core::ffi::c_long;
        }
        return len_0;
    }
    if op == XDV_GLYPHS as ::core::ffi::c_int {
        if avail < 7 as ::core::ffi::c_int as ::core::ffi::c_long {
            return 0 as ::core::ffi::c_long;
        }
        let mut n: uint32_t = be(
            p.offset(5 as ::core::ffi::c_int as isize),
            2 as ::core::ffi::c_int,
        );
        if avail < 7 as ::core::ffi::c_long + 10 as ::core::ffi::c_long * n as ::core::ffi::c_long {
            return 0 as ::core::ffi::c_long;
        }
        return 7 as ::core::ffi::c_long + 10 as ::core::ffi::c_long * n as ::core::ffi::c_long;
    }
    if op == XDV_TEXT_AND_GLYPHS as ::core::ffi::c_int {
        if avail < 3 as ::core::ffi::c_int as ::core::ffi::c_long {
            return 0 as ::core::ffi::c_long;
        }
        let mut l_0: uint32_t = be(
            p.offset(1 as ::core::ffi::c_int as isize),
            2 as ::core::ffi::c_int,
        );
        if avail
            < (3 as uint32_t)
                .wrapping_add((2 as uint32_t).wrapping_mul(l_0))
                .wrapping_add(6 as uint32_t) as ::core::ffi::c_long
        {
            return 0 as ::core::ffi::c_long;
        }
        let mut n_0: uint32_t = be(
            p.offset(3 as ::core::ffi::c_int as isize)
                .offset((2 as uint32_t).wrapping_mul(l_0) as isize)
                .offset(4 as ::core::ffi::c_int as isize),
            2 as ::core::ffi::c_int,
        );
        if avail
            < 3 as ::core::ffi::c_long
                + 2 as ::core::ffi::c_long * l_0 as ::core::ffi::c_long
                + 6 as ::core::ffi::c_long
                + 10 as ::core::ffi::c_long * n_0 as ::core::ffi::c_long
        {
            return 0 as ::core::ffi::c_long;
        }
        return 3 as ::core::ffi::c_long
            + 2 as ::core::ffi::c_long * l_0 as ::core::ffi::c_long
            + 6 as ::core::ffi::c_long
            + 10 as ::core::ffi::c_long * n_0 as ::core::ffi::c_long;
    }
    return -(1 as ::core::ffi::c_int) as ::core::ffi::c_long;
}
#[no_mangle]
pub unsafe extern "C" fn xdv_index_new() -> *mut xdv_index {
    let mut x: *mut xdv_index =
        calloc(1 as size_t, ::core::mem::size_of::<xdv_index>() as size_t) as *mut xdv_index;
    if x.is_null() {
        abort();
    }
    return x;
}
#[no_mangle]
pub unsafe extern "C" fn xdv_index_free(mut x: *mut xdv_index) {
    if x.is_null() {
        return;
    }
    free((*x).pages as *mut ::core::ffi::c_void);
    free(x as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn xdv_index_reset(mut x: *mut xdv_index) {
    let mut pages: *mut xdv_page = (*x).pages;
    let mut cap: ::core::ffi::c_int = (*x).cap;
    memset(
        x as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<xdv_index>() as size_t,
    );
    (*x).pages = pages;
    (*x).cap = cap;
}
#[no_mangle]
pub unsafe extern "C" fn xdv_index_update(
    mut x: *mut xdv_index,
    mut data: *const ::core::ffi::c_uchar,
    mut len: size_t,
) {
    if len < (*x).scan {
        while (*x).npages > 0 as ::core::ffi::c_int
            && (*(*x)
                .pages
                .offset(((*x).npages - 1 as ::core::ffi::c_int) as isize))
            .eop_end
                > len
        {
            (*x).npages -= 1;
        }
        (*x).in_page = false_0 != 0;
        (*x).broken = false_0 != 0;
        if (*x).npages > 0 as ::core::ffi::c_int {
            (*x).scan = (*(*x)
                .pages
                .offset(((*x).npages - 1 as ::core::ffi::c_int) as isize))
            .eop_end;
        } else {
            xdv_index_reset(x);
        }
    }
    if (*x).broken {
        return;
    }
    let mut end: *const ::core::ffi::c_uchar = data.offset(len as isize);
    while (*x).scan < len {
        let mut p: *const ::core::ffi::c_uchar = data.offset((*x).scan as isize);
        let mut n: ::core::ffi::c_long = xdv_insn_length(p, end);
        if n == 0 as ::core::ffi::c_long {
            break;
        }
        if n < 0 as ::core::ffi::c_long {
            (*x).broken = true_0 != 0;
            break;
        } else {
            let mut op: ::core::ffi::c_int =
                *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
            if !(*x).have_pre {
                if op != DVI_PRE as ::core::ffi::c_int {
                    (*x).broken = true_0 != 0;
                    break;
                } else {
                    (*x).num = be(
                        p.offset(2 as ::core::ffi::c_int as isize),
                        4 as ::core::ffi::c_int,
                    );
                    (*x).den = be(
                        p.offset(6 as ::core::ffi::c_int as isize),
                        4 as ::core::ffi::c_int,
                    );
                    (*x).mag = be(
                        p.offset(10 as ::core::ffi::c_int as isize),
                        4 as ::core::ffi::c_int,
                    );
                    (*x).have_pre = true_0 != 0;
                }
            } else if op == DVI_BOP as ::core::ffi::c_int {
                (*x).in_page = true_0 != 0;
                (*x).cur_bop = (*x).scan;
            } else if op == DVI_EOP as ::core::ffi::c_int && (*x).in_page as ::core::ffi::c_int != 0
            {
                if (*x).npages == (*x).cap {
                    (*x).cap = if (*x).cap != 0 {
                        (*x).cap * 2 as ::core::ffi::c_int
                    } else {
                        64 as ::core::ffi::c_int
                    };
                    (*x).pages = realloc(
                        (*x).pages as *mut ::core::ffi::c_void,
                        (::core::mem::size_of::<xdv_page>() as size_t)
                            .wrapping_mul((*x).cap as size_t),
                    ) as *mut xdv_page;
                    if (*x).pages.is_null() {
                        abort();
                    }
                }
                (*(*x).pages.offset((*x).npages as isize)).bop = (*x).cur_bop;
                (*(*x).pages.offset((*x).npages as isize)).eop_end =
                    (*x).scan.wrapping_add(1 as size_t);
                (*x).npages += 1;
                (*x).in_page = false_0 != 0;
            } else if op == DVI_POST as ::core::ffi::c_int {
                (*x).scan = len;
                break;
            }
            (*x).scan = (*x).scan.wrapping_add(n as size_t);
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn xdv_index_page_count(mut x: *const xdv_index) -> ::core::ffi::c_int {
    return (*x).npages;
}
#[no_mangle]
pub unsafe extern "C" fn xdv_index_page(
    mut x: *const xdv_index,
    mut i: ::core::ffi::c_int,
) -> *const xdv_page {
    if i < 0 as ::core::ffi::c_int || i >= (*x).npages {
        return ::core::ptr::null::<xdv_page>();
    }
    return (*x).pages.offset(i as isize) as *mut xdv_page;
}
#[no_mangle]
pub unsafe extern "C" fn xdv_index_output_started(mut x: *const xdv_index) -> bool {
    return (*x).have_pre;
}
#[no_mangle]
pub unsafe extern "C" fn xdv_index_preamble(
    mut x: *const xdv_index,
    mut num: *mut uint32_t,
    mut den: *mut uint32_t,
    mut mag: *mut uint32_t,
) -> bool {
    if !(*x).have_pre {
        return false_0 != 0;
    }
    *num = (*x).num;
    *den = (*x).den;
    *mag = (*x).mag;
    return true_0 != 0;
}
