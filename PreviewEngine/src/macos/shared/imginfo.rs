/* Pitex embedded preview engine — raster image header parsing.
 * Independently written from the PNG, JPEG/JFIF/Exif and BMP format
 * specifications. Pitex-authored (AGPL-3.0-or-later). */
// Translated from shared/imginfo.c with C2Rust 0.22.1.
static mut PICTURE_DEFAULT_DPI:f64=72.;
#[no_mangle]
pub unsafe extern "C" fn pitex_img_info_resolution(dpi:i32) {
    PICTURE_DEFAULT_DPI=if dpi<=0{72.}else{dpi.min(65535) as f64};
}
extern "C" {
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn memset(
        __b: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type __darwin_size_t = usize;
pub type int32_t = i32;
pub type size_t = __darwin_size_t;
pub type pitex_img_kind = ::core::ffi::c_uint;
pub const PITEX_IMG_BMP: pitex_img_kind = 3;
pub const PITEX_IMG_JPEG: pitex_img_kind = 2;
pub const PITEX_IMG_PNG: pitex_img_kind = 1;
pub const PITEX_IMG_NONE: pitex_img_kind = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pitex_img_info {
    pub kind: pitex_img_kind,
    pub width: ::core::ffi::c_uint,
    pub height: ::core::ffi::c_uint,
    pub xdpi: ::core::ffi::c_double,
    pub ydpi: ::core::ffi::c_double,
}
unsafe extern "C" fn be16(mut p: *const ::core::ffi::c_uchar) -> ::core::ffi::c_uint {
    return (*p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint)
        << 8 as ::core::ffi::c_int
        | *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint;
}
unsafe extern "C" fn be32(mut p: *const ::core::ffi::c_uchar) -> ::core::ffi::c_ulong {
    return (*p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_ulong)
        << 24 as ::core::ffi::c_int
        | (*p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_ulong)
            << 16 as ::core::ffi::c_int
        | (*p.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_ulong)
            << 8 as ::core::ffi::c_int
        | *p.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_ulong;
}
unsafe extern "C" fn le16(mut p: *const ::core::ffi::c_uchar) -> ::core::ffi::c_uint {
    return (*p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint)
        << 8 as ::core::ffi::c_int
        | *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint;
}
unsafe extern "C" fn le32(mut p: *const ::core::ffi::c_uchar) -> ::core::ffi::c_ulong {
    return (*p.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_ulong)
        << 24 as ::core::ffi::c_int
        | (*p.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_ulong)
            << 16 as ::core::ffi::c_int
        | (*p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_ulong)
            << 8 as ::core::ffi::c_int
        | *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_ulong;
}
#[no_mangle]
pub unsafe extern "C" fn pitex_img_sniff(
    mut d: *const ::core::ffi::c_uchar,
    mut n: size_t,
) -> pitex_img_kind {
    if n >= 8 as size_t
        && memcmp(
            d as *const ::core::ffi::c_void,
            b"\x89PNG\r\n\x1A\n\0" as *const u8 as *const ::core::ffi::c_char
                as *const ::core::ffi::c_void,
            8 as size_t,
        ) == 0 as ::core::ffi::c_int
    {
        return PITEX_IMG_PNG;
    }
    if n >= 3 as size_t
        && *d.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == 0xff as ::core::ffi::c_int
        && *d.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == 0xd8 as ::core::ffi::c_int
        && *d.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == 0xff as ::core::ffi::c_int
    {
        return PITEX_IMG_JPEG;
    }
    if n >= 26 as size_t
        && *d.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == 'B' as i32
        && *d.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == 'M' as i32
    {
        return PITEX_IMG_BMP;
    }
    return PITEX_IMG_NONE;
}
unsafe extern "C" fn png_info(
    mut d: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut out: *mut pitex_img_info,
) -> ::core::ffi::c_int {
    if n < 33 as size_t
        || memcmp(
            d.offset(12 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
            b"IHDR\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            4 as size_t,
        ) != 0 as ::core::ffi::c_int
    {
        return -(1 as ::core::ffi::c_int);
    }
    (*out).width = be32(d.offset(16 as ::core::ffi::c_int as isize)) as ::core::ffi::c_uint;
    (*out).height = be32(d.offset(20 as ::core::ffi::c_int as isize)) as ::core::ffi::c_uint;
    let mut pos: size_t = 8 as size_t;
    while pos.wrapping_add(12 as size_t) <= n {
        let mut len: ::core::ffi::c_ulong = be32(d.offset(pos as isize));
        let mut type_0: *const ::core::ffi::c_uchar = d
            .offset(pos as isize)
            .offset(4 as ::core::ffi::c_int as isize);
        if len as size_t > n.wrapping_sub(pos).wrapping_sub(12 as size_t) {
            break;
        }
        if memcmp(
            type_0 as *const ::core::ffi::c_void,
            b"IDAT\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0 as ::core::ffi::c_int
            || memcmp(
                type_0 as *const ::core::ffi::c_void,
                b"IEND\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
                4 as size_t,
            ) == 0 as ::core::ffi::c_int
        {
            break;
        }
        if memcmp(
            type_0 as *const ::core::ffi::c_void,
            b"pHYs\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0 as ::core::ffi::c_int
            && len >= 9 as ::core::ffi::c_ulong
        {
            let mut p: *const ::core::ffi::c_uchar = d
                .offset(pos as isize)
                .offset(8 as ::core::ffi::c_int as isize);
            let mut px: ::core::ffi::c_double = be32(p) as ::core::ffi::c_double;
            let mut py: ::core::ffi::c_double =
                be32(p.offset(4 as ::core::ffi::c_int as isize)) as ::core::ffi::c_double;
            if px > 0 as ::core::ffi::c_int as ::core::ffi::c_double
                && py > 0 as ::core::ffi::c_int as ::core::ffi::c_double
            {
                if *p.offset(8 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    == 1 as ::core::ffi::c_int
                {
                    (*out).xdpi = px * 0.0254f64;
                    (*out).ydpi = py * 0.0254f64;
                } else {
                    (*out).ydpi = (*out).xdpi * py / px;
                }
            }
        }
        pos = (pos as ::core::ffi::c_ulong)
            .wrapping_add((12 as ::core::ffi::c_ulong).wrapping_add(len)) as size_t
            as size_t;
    }
    return if (*out).width != 0 && (*out).height != 0 {
        0 as ::core::ffi::c_int
    } else {
        -(1 as ::core::ffi::c_int)
    };
}
unsafe extern "C" fn exif_resolution(
    mut t: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut out: *mut pitex_img_info,
) {
    if n < 8 as size_t {
        return;
    }
    let mut le: ::core::ffi::c_int = 0;
    if *t.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == 'I' as i32
        && *t.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == 'I' as i32
    {
        le = 1 as ::core::ffi::c_int;
    } else if *t.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == 'M' as i32
        && *t.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == 'M' as i32
    {
        le = 0 as ::core::ffi::c_int;
    } else {
        return;
    }
    let mut ifd: ::core::ffi::c_ulong = if le != 0 {
        le32(t.offset(4 as ::core::ffi::c_int as isize))
    } else {
        be32(t.offset(4 as ::core::ffi::c_int as isize))
    };
    if (ifd as size_t).wrapping_add(2 as size_t) > n {
        return;
    }
    let mut count: ::core::ffi::c_uint = if le != 0 {
        le16(t.offset(ifd as isize))
    } else {
        be16(t.offset(ifd as isize))
    };
    let mut xr: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut yr: ::core::ffi::c_double = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
    let mut unit: ::core::ffi::c_uint = 2 as ::core::ffi::c_uint;
    let mut i: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while i < count {
        let mut e: size_t = (ifd as size_t)
            .wrapping_add(2 as size_t)
            .wrapping_add((i as size_t).wrapping_mul(12 as size_t));
        if e.wrapping_add(12 as size_t) > n {
            break;
        }
        let mut tag: ::core::ffi::c_uint = if le != 0 {
            le16(t.offset(e as isize))
        } else {
            be16(t.offset(e as isize))
        };
        let mut type_0: ::core::ffi::c_uint = if le != 0 {
            le16(
                t.offset(e as isize)
                    .offset(2 as ::core::ffi::c_int as isize),
            )
        } else {
            be16(
                t.offset(e as isize)
                    .offset(2 as ::core::ffi::c_int as isize),
            )
        };
        if (tag == 0x11a as ::core::ffi::c_uint || tag == 0x11b as ::core::ffi::c_uint)
            && type_0 == 5 as ::core::ffi::c_uint
        {
            let mut off: ::core::ffi::c_ulong = if le != 0 {
                le32(
                    t.offset(e as isize)
                        .offset(8 as ::core::ffi::c_int as isize),
                )
            } else {
                be32(
                    t.offset(e as isize)
                        .offset(8 as ::core::ffi::c_int as isize),
                )
            };
            if (off as size_t).wrapping_add(8 as size_t) <= n {
                let mut num: ::core::ffi::c_ulong = if le != 0 {
                    le32(t.offset(off as isize))
                } else {
                    be32(t.offset(off as isize))
                };
                let mut den: ::core::ffi::c_ulong = if le != 0 {
                    le32(
                        t.offset(off as isize)
                            .offset(4 as ::core::ffi::c_int as isize),
                    )
                } else {
                    be32(
                        t.offset(off as isize)
                            .offset(4 as ::core::ffi::c_int as isize),
                    )
                };
                let mut v: ::core::ffi::c_double = if den != 0 {
                    num as ::core::ffi::c_double / den as ::core::ffi::c_double
                } else {
                    0 as ::core::ffi::c_int as ::core::ffi::c_double
                };
                if tag == 0x11a as ::core::ffi::c_uint {
                    xr = v;
                } else {
                    yr = v;
                }
            }
        } else if tag == 0x128 as ::core::ffi::c_uint && type_0 == 3 as ::core::ffi::c_uint {
            unit = if le != 0 {
                le16(
                    t.offset(e as isize)
                        .offset(8 as ::core::ffi::c_int as isize),
                )
            } else {
                be16(
                    t.offset(e as isize)
                        .offset(8 as ::core::ffi::c_int as isize),
                )
            };
        }
        i = i.wrapping_add(1);
    }
    if xr > 0 as ::core::ffi::c_int as ::core::ffi::c_double
        && yr > 0 as ::core::ffi::c_int as ::core::ffi::c_double
        && (unit == 2 as ::core::ffi::c_uint || unit == 3 as ::core::ffi::c_uint)
    {
        let mut k: ::core::ffi::c_double = if unit == 3 as ::core::ffi::c_uint {
            2.54f64
        } else {
            1.0f64
        };
        (*out).xdpi = xr * k;
        (*out).ydpi = yr * k;
    }
}
unsafe extern "C" fn jpeg_info(
    mut d: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut out: *mut pitex_img_info,
) -> ::core::ffi::c_int {
    let mut pos: size_t = 2 as size_t;
    let mut have_jfif_density: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while pos.wrapping_add(4 as size_t) <= n {
        if *d.offset(pos as isize) as ::core::ffi::c_int != 0xff as ::core::ffi::c_int {
            pos = pos.wrapping_add(1);
        } else {
            let mut m: ::core::ffi::c_uint =
                *d.offset(pos.wrapping_add(1 as size_t) as isize) as ::core::ffi::c_uint;
            if m == 0xff as ::core::ffi::c_uint {
                pos = pos.wrapping_add(1);
            } else if m == 0xd8 as ::core::ffi::c_uint
                || m >= 0xd0 as ::core::ffi::c_uint && m <= 0xd7 as ::core::ffi::c_uint
                || m == 0x1 as ::core::ffi::c_uint
            {
                pos = pos.wrapping_add(2 as size_t);
            } else {
                if m == 0xd9 as ::core::ffi::c_uint || m == 0xda as ::core::ffi::c_uint {
                    break;
                }
                let mut len: ::core::ffi::c_uint = be16(
                    d.offset(pos as isize)
                        .offset(2 as ::core::ffi::c_int as isize),
                );
                if len < 2 as ::core::ffi::c_uint
                    || pos.wrapping_add(2 as size_t).wrapping_add(len as size_t) > n
                {
                    break;
                }
                let mut seg: *const ::core::ffi::c_uchar = d
                    .offset(pos as isize)
                    .offset(4 as ::core::ffi::c_int as isize);
                let mut slen: size_t = len.wrapping_sub(2 as ::core::ffi::c_uint) as size_t;
                if m == 0xe0 as ::core::ffi::c_uint
                    && slen >= 12 as size_t
                    && memcmp(
                        seg as *const ::core::ffi::c_void,
                        b"JFIF\0\0" as *const u8 as *const ::core::ffi::c_char
                            as *const ::core::ffi::c_void,
                        5 as size_t,
                    ) == 0 as ::core::ffi::c_int
                {
                    let mut units: ::core::ffi::c_uint =
                        *seg.offset(7 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint;
                    let mut xd: ::core::ffi::c_uint =
                        be16(seg.offset(8 as ::core::ffi::c_int as isize));
                    let mut yd: ::core::ffi::c_uint =
                        be16(seg.offset(10 as ::core::ffi::c_int as isize));
                    if xd != 0 && yd != 0 {
                        if units == 1 as ::core::ffi::c_uint || units == 2 as ::core::ffi::c_uint {
                            let mut k: ::core::ffi::c_double = if units == 2 as ::core::ffi::c_uint
                            {
                                2.54f64
                            } else {
                                1.0f64
                            };
                            (*out).xdpi = xd as ::core::ffi::c_double * k;
                            (*out).ydpi = yd as ::core::ffi::c_double * k;
                            have_jfif_density = 1 as ::core::ffi::c_int;
                        } else if units == 0 as ::core::ffi::c_uint {
                            (*out).ydpi =
                                (*out).xdpi * yd as ::core::ffi::c_double / xd as ::core::ffi::c_double;
                        }
                    }
                } else if m == 0xe1 as ::core::ffi::c_uint
                    && slen >= 14 as size_t
                    && memcmp(
                        seg as *const ::core::ffi::c_void,
                        b"Exif\0\0\0" as *const u8 as *const ::core::ffi::c_char
                            as *const ::core::ffi::c_void,
                        6 as size_t,
                    ) == 0 as ::core::ffi::c_int
                    && have_jfif_density == 0
                {
                    exif_resolution(
                        seg.offset(6 as ::core::ffi::c_int as isize),
                        slen.wrapping_sub(6 as size_t),
                        out,
                    );
                } else if m >= 0xc0 as ::core::ffi::c_uint
                    && m <= 0xcf as ::core::ffi::c_uint
                    && m != 0xc4 as ::core::ffi::c_uint
                    && m != 0xc8 as ::core::ffi::c_uint
                    && m != 0xcc as ::core::ffi::c_uint
                    && slen >= 6 as size_t
                {
                    (*out).height = be16(seg.offset(1 as ::core::ffi::c_int as isize));
                    (*out).width = be16(seg.offset(3 as ::core::ffi::c_int as isize));
                    return if (*out).width != 0 && (*out).height != 0 {
                        0 as ::core::ffi::c_int
                    } else {
                        -(1 as ::core::ffi::c_int)
                    };
                }
                pos = pos.wrapping_add((2 as ::core::ffi::c_uint).wrapping_add(len) as size_t);
            }
        }
    }
    return -(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn bmp_info(
    mut d: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut out: *mut pitex_img_info,
) -> ::core::ffi::c_int {
    let mut hsize: ::core::ffi::c_ulong = le32(d.offset(14 as ::core::ffi::c_int as isize));
    if hsize == 12 as ::core::ffi::c_ulong && n >= 26 as size_t {
        (*out).width = le16(d.offset(18 as ::core::ffi::c_int as isize));
        (*out).height = le16(d.offset(20 as ::core::ffi::c_int as isize));
    } else if hsize >= 40 as ::core::ffi::c_ulong && n >= 54 as size_t {
        let mut w: ::core::ffi::c_long =
            le32(d.offset(18 as ::core::ffi::c_int as isize)) as int32_t as ::core::ffi::c_long;
        let mut h: ::core::ffi::c_long =
            le32(d.offset(22 as ::core::ffi::c_int as isize)) as int32_t as ::core::ffi::c_long;
        (*out).width = (if w < 0 as ::core::ffi::c_long { -w } else { w }) as ::core::ffi::c_uint;
        (*out).height = (if h < 0 as ::core::ffi::c_long { -h } else { h }) as ::core::ffi::c_uint;
        let mut xppm: ::core::ffi::c_long =
            le32(d.offset(38 as ::core::ffi::c_int as isize)) as int32_t as ::core::ffi::c_long;
        let mut yppm: ::core::ffi::c_long =
            le32(d.offset(42 as ::core::ffi::c_int as isize)) as int32_t as ::core::ffi::c_long;
        if xppm > 0 as ::core::ffi::c_long && yppm > 0 as ::core::ffi::c_long {
            (*out).xdpi = xppm as ::core::ffi::c_double * 0.0254f64;
            (*out).ydpi = yppm as ::core::ffi::c_double * 0.0254f64;
        }
    } else {
        return -(1 as ::core::ffi::c_int);
    }
    return if (*out).width != 0 && (*out).height != 0 {
        0 as ::core::ffi::c_int
    } else {
        -(1 as ::core::ffi::c_int)
    };
}
#[no_mangle]
pub unsafe extern "C" fn pitex_img_info_read(
    mut d: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut out: *mut pitex_img_info,
) -> ::core::ffi::c_int {
    memset(
        out as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<pitex_img_info>() as size_t,
    );
    (*out).ydpi = PICTURE_DEFAULT_DPI;
    (*out).xdpi = (*out).ydpi;
    (*out).kind = pitex_img_sniff(d, n);
    match (*out).kind as ::core::ffi::c_uint {
        1 => return png_info(d, n, out),
        2 => return jpeg_info(d, n, out),
        3 => return bmp_info(d, n, out),
        _ => return -(1 as ::core::ffi::c_int),
    };
}
