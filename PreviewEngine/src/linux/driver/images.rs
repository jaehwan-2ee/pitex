/* Pitex embedded preview engine — raster images to PDF image XObjects.
 * Independently written from the PNG, JPEG/JFIF and BMP specifications.
 * Pitex-authored (AGPL-3.0-or-later). */
// Translated from driver/images.c with C2Rust 0.22.1.
#[path = "../../shared/png_highcolor.rs"]
mod png_highcolor;
static mut IMAGE_APPLY_GAMMA: i32 = 0;
static mut IMAGE_TARGET_GAMMA: i32 = 1000;
static mut IMAGE_ASSUMED_GAMMA: i32 = 2200;
static mut IMAGE_HIGHCOLOR: i32 = 1;
static mut IMAGE_RESOLUTION: i32 = 72;
#[no_mangle]
pub unsafe extern "C" fn pitex_image_configuration(apply:i32,target:i32,assumed:i32,high:i32,resolution:i32) {
    IMAGE_APPLY_GAMMA=apply;IMAGE_TARGET_GAMMA=target;IMAGE_ASSUMED_GAMMA=assumed;IMAGE_HIGHCOLOR=high;IMAGE_RESOLUTION=resolution;crate::shared_imginfo::pitex_img_info_resolution(resolution);
}
unsafe fn png_gamma(data:&[u8])->f64 {
    if IMAGE_APPLY_GAMMA==0{return 1.}
    let mut gamma=IMAGE_ASSUMED_GAMMA.max(1) as f64/1000.;let mut at=8usize;
    while at+12<=data.len(){let n=u32::from_be_bytes(data[at..at+4].try_into().unwrap()) as usize;
        let Some(body)=data.get(at+8..at+8+n)else{break};
        if &data[at+4..at+8]==b"gAMA"&&body.len()==4{let file=u32::from_be_bytes(body.try_into().unwrap()) as f64/100000.;if file>0.{gamma=1./file;}}
        if &data[at+4..at+8]==b"sRGB"{gamma=2.2;}
        at+=n+12;
    }gamma/(IMAGE_TARGET_GAMMA.max(1) as f64/1000.)
}
unsafe fn png16(data:&[u8],img:*mut pdf_image)->Option<()> {
    let mut decoded=png_highcolor::decode(data)?;let exponent=png_gamma(data);
    if (exponent-1.).abs()>1e-10{for pixel in decoded.color.chunks_exact_mut(2){let n=u16::from_be_bytes([pixel[0],pixel[1]]);let value=((n as f64/65535.).powf(exponent)*65535.+0.5).clamp(0.,65535.) as u16;pixel.copy_from_slice(&value.to_be_bytes());}}
    if pbuf_deflate(&raw mut (*img).data,decoded.color.as_ptr(),decoded.color.len(),6)!=0{return None}
    if decoded.alpha.iter().any(|b|*b!=255){if pbuf_deflate(&raw mut (*img).smask,decoded.alpha.as_ptr(),decoded.alpha.len(),6)!=0{return None}}
    (*img).width=decoded.width as i32;(*img).height=decoded.height as i32;
    snprintf((*img).dict.as_mut_ptr(),(*img).dict.len(),b"/Width %u/Height %u/ColorSpace/%s/BitsPerComponent 16\0".as_ptr().cast(),decoded.width,decoded.height,if decoded.channels==1{b"DeviceGray\0".as_ptr().cast::<::core::ffi::c_char>()}else{b"DeviceRGB\0".as_ptr().cast()});Some(())
}
extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
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
    fn pbuf_printf(b: *mut pbuf, fmt: *const ::core::ffi::c_char, ...);
    fn pbuf_deflate(
        out: *mut pbuf,
        data: *const ::core::ffi::c_uchar,
        len: size_t,
        level: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn pbuf_inflate(
        out: *mut pbuf,
        data: *const ::core::ffi::c_uchar,
        len: size_t,
    ) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn pitex_img_info_read(
        data: *const ::core::ffi::c_uchar,
        len: size_t,
        out: *mut pitex_img_info,
    ) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type int32_t = __int32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pbuf {
    pub data: *mut ::core::ffi::c_uchar,
    pub len: size_t,
    pub cap: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pdf_image {
    pub ok: bool,
    pub width: ::core::ffi::c_int,
    pub height: ::core::ffi::c_int,
    pub xdpi: ::core::ffi::c_double,
    pub ydpi: ::core::ffi::c_double,
    pub dict: [::core::ffi::c_char; 192],
    pub dct: bool,
    pub data: pbuf,
    pub smask: pbuf,
}
pub type uint32_t = __uint32_t;
pub const PITEX_IMG_BMP: pitex_img_kind = 3;
pub const PITEX_IMG_JPEG: pitex_img_kind = 2;
pub const PITEX_IMG_PNG: pitex_img_kind = 1;
pub type pitex_img_kind = ::core::ffi::c_uint;
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
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
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
unsafe extern "C" fn pbuf_free(mut b: *mut pbuf) {
    free((*b).data as *mut ::core::ffi::c_void);
    (*b).data = ::core::ptr::null_mut::<::core::ffi::c_uchar>();
    (*b).cap = 0 as size_t;
    (*b).len = (*b).cap;
}
unsafe extern "C" fn be32(mut p: *const ::core::ffi::c_uchar) -> uint32_t {
    return (*p.offset(0 as ::core::ffi::c_int as isize) as uint32_t)
        << 24 as ::core::ffi::c_int
        | (*p.offset(1 as ::core::ffi::c_int as isize) as uint32_t)
            << 16 as ::core::ffi::c_int
        | (*p.offset(2 as ::core::ffi::c_int as isize) as uint32_t)
            << 8 as ::core::ffi::c_int
        | *p.offset(3 as ::core::ffi::c_int as isize) as uint32_t;
}
unsafe extern "C" fn be16(mut p: *const ::core::ffi::c_uchar) -> ::core::ffi::c_uint {
    return (*p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint)
        << 8 as ::core::ffi::c_int
        | *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint;
}
unsafe extern "C" fn le32(mut p: *const ::core::ffi::c_uchar) -> uint32_t {
    return (*p.offset(3 as ::core::ffi::c_int as isize) as uint32_t)
        << 24 as ::core::ffi::c_int
        | (*p.offset(2 as ::core::ffi::c_int as isize) as uint32_t)
            << 16 as ::core::ffi::c_int
        | (*p.offset(1 as ::core::ffi::c_int as isize) as uint32_t)
            << 8 as ::core::ffi::c_int
        | *p.offset(0 as ::core::ffi::c_int as isize) as uint32_t;
}
unsafe extern "C" fn le16(mut p: *const ::core::ffi::c_uchar) -> ::core::ffi::c_uint {
    return (*p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint)
        << 8 as ::core::ffi::c_int
        | *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint;
}
#[no_mangle]
pub unsafe extern "C" fn image_free(mut img: *mut pdf_image) {
    pbuf_free(&raw mut (*img).data);
    pbuf_free(&raw mut (*img).smask);
    memset(
        img as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<pdf_image>() as size_t,
    );
}
unsafe extern "C" fn paeth(
    mut a: ::core::ffi::c_int,
    mut b: ::core::ffi::c_int,
    mut c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut p: ::core::ffi::c_int = a + b - c;
    let mut pa: ::core::ffi::c_int = abs(p - a);
    let mut pb: ::core::ffi::c_int = abs(p - b);
    let mut pc: ::core::ffi::c_int = abs(p - c);
    return if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { c };
}
unsafe extern "C" fn unfilter(
    mut raw: *mut ::core::ffi::c_uchar,
    mut rawlen: size_t,
    mut pos: *mut size_t,
    mut stride: size_t,
    mut h: ::core::ffi::c_int,
    mut bpp: ::core::ffi::c_int,
    mut out: *mut ::core::ffi::c_uchar,
) -> bool {
    let mut prev: *mut ::core::ffi::c_uchar = ::core::ptr::null_mut::<
        ::core::ffi::c_uchar,
    >();
    let mut y: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while y < h {
        if (*pos).wrapping_add(1 as size_t).wrapping_add(stride) > rawlen {
            return false_0 != 0;
        }
        let fresh0 = *pos;
        *pos = (*pos).wrapping_add(1);
        let mut type_0: ::core::ffi::c_int = *raw.offset(fresh0 as isize)
            as ::core::ffi::c_int;
        let mut src: *mut ::core::ffi::c_uchar = raw.offset(*pos as isize);
        let mut dst: *mut ::core::ffi::c_uchar = out
            .offset((y as size_t).wrapping_mul(stride) as isize);
        let mut i: size_t = 0 as size_t;
        while i < stride {
            let mut a: ::core::ffi::c_int = if i >= bpp as size_t {
                *dst.offset(i.wrapping_sub(bpp as size_t) as isize) as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            };
            let mut b: ::core::ffi::c_int = if !prev.is_null() {
                *prev.offset(i as isize) as ::core::ffi::c_int
            } else {
                0 as ::core::ffi::c_int
            };
            let mut c: ::core::ffi::c_int = if !prev.is_null() && i >= bpp as size_t {
                *prev.offset(i.wrapping_sub(bpp as size_t) as isize)
                    as ::core::ffi::c_int
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
                    x += a + b >> 1 as ::core::ffi::c_int;
                }
                4 => {
                    x += paeth(a, b, c);
                }
                _ => {}
            }
            *dst.offset(i as isize) = x as ::core::ffi::c_uchar;
            i = i.wrapping_add(1);
        }
        *pos = (*pos).wrapping_add(stride);
        prev = dst;
        y += 1;
    }
    return true_0 != 0;
}
unsafe extern "C" fn sample(
    mut row: *const ::core::ffi::c_uchar,
    mut idx: ::core::ffi::c_int,
    mut bd: ::core::ffi::c_int,
) -> ::core::ffi::c_uint {
    if bd == 8 as ::core::ffi::c_int {
        return *row.offset(idx as isize) as ::core::ffi::c_uint;
    }
    if bd == 16 as ::core::ffi::c_int {
        return *row.offset((idx * 2 as ::core::ffi::c_int) as isize)
            as ::core::ffi::c_uint;
    }
    let mut bit: ::core::ffi::c_int = idx * bd;
    return (*row.offset((bit >> 3 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
        >> 8 as ::core::ffi::c_int - bd - (bit & 7 as ::core::ffi::c_int))
        as ::core::ffi::c_uint
        & ((1 as ::core::ffi::c_uint) << bd).wrapping_sub(1 as ::core::ffi::c_uint);
}
unsafe extern "C" fn sample16(
    mut row: *const ::core::ffi::c_uchar,
    mut idx: ::core::ffi::c_int,
    mut bd: ::core::ffi::c_int,
) -> ::core::ffi::c_uint {
    if bd == 16 as ::core::ffi::c_int {
        return be16(row.offset((idx * 2 as ::core::ffi::c_int) as isize));
    }
    return sample(row, idx, bd);
}
unsafe extern "C" fn png_encode(
    mut d: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut img: *mut pdf_image,
    mut warnings: *mut pbuf,
    mut name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if n < 33 as size_t {
        return -(1 as ::core::ffi::c_int);
    }
    let mut w: uint32_t = be32(d.offset(16 as ::core::ffi::c_int as isize));
    let mut h: uint32_t = be32(d.offset(20 as ::core::ffi::c_int as isize));
    let mut bd: ::core::ffi::c_int = *d.offset(24 as ::core::ffi::c_int as isize)
        as ::core::ffi::c_int;
    let mut ct: ::core::ffi::c_int = *d.offset(25 as ::core::ffi::c_int as isize)
        as ::core::ffi::c_int;
    let mut interlace: ::core::ffi::c_int = *d.offset(28 as ::core::ffi::c_int as isize)
        as ::core::ffi::c_int;
    if w == 0 as uint32_t || h == 0 as uint32_t || w > 30000 as uint32_t
        || h > 30000 as uint32_t
    {
        return -(1 as ::core::ffi::c_int);
    }
    let mut channels: ::core::ffi::c_int = if ct == 0 as ::core::ffi::c_int {
        1 as ::core::ffi::c_int
    } else if ct == 2 as ::core::ffi::c_int {
        3 as ::core::ffi::c_int
    } else if ct == 3 as ::core::ffi::c_int {
        1 as ::core::ffi::c_int
    } else if ct == 4 as ::core::ffi::c_int {
        2 as ::core::ffi::c_int
    } else if ct == 6 as ::core::ffi::c_int {
        4 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
    if channels == 0
        || bd != 1 as ::core::ffi::c_int && bd != 2 as ::core::ffi::c_int
            && bd != 4 as ::core::ffi::c_int && bd != 8 as ::core::ffi::c_int
            && bd != 16 as ::core::ffi::c_int
    {
        return -(1 as ::core::ffi::c_int);
    }
    if bd==16 && IMAGE_HIGHCOLOR!=0 {return if png16(std::slice::from_raw_parts(d,n),img).is_some(){0}else{-1};}
    let mut idat: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    let mut palette: [::core::ffi::c_uchar; 768] = [
        0 as ::core::ffi::c_int as ::core::ffi::c_uchar,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let mut palpha: [::core::ffi::c_uchar; 256] = [0; 256];
    memset(
        &raw mut palpha as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        255 as ::core::ffi::c_int,
        ::core::mem::size_of::<[::core::ffi::c_uchar; 256]>() as size_t,
    );
    let mut npal: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut has_trns: bool = false_0 != 0;
    let mut trns_key: [::core::ffi::c_uint; 3] = [
        0 as ::core::ffi::c_int as ::core::ffi::c_uint,
        0,
        0,
    ];
    let mut pos: size_t = 8 as size_t;
    while pos.wrapping_add(12 as size_t) <= n {
        let mut len: uint32_t = be32(d.offset(pos as isize));
        let mut type_0: *const ::core::ffi::c_uchar = d
            .offset(pos as isize)
            .offset(4 as ::core::ffi::c_int as isize);
        let mut data: *const ::core::ffi::c_uchar = d
            .offset(pos as isize)
            .offset(8 as ::core::ffi::c_int as isize);
        if len as size_t > n.wrapping_sub(pos).wrapping_sub(12 as size_t) {
            break;
        }
        if memcmp(
            type_0 as *const ::core::ffi::c_void,
            b"IDAT\0" as *const u8 as *const ::core::ffi::c_char
                as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0
        {
            pbuf_append(
                &raw mut idat,
                data as *const ::core::ffi::c_void,
                len as size_t,
            );
        } else if memcmp(
            type_0 as *const ::core::ffi::c_void,
            b"PLTE\0" as *const u8 as *const ::core::ffi::c_char
                as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0
        {
            npal = (if len.wrapping_div(3 as uint32_t) > 256 as uint32_t {
                256 as uint32_t
            } else {
                len.wrapping_div(3 as uint32_t)
            }) as ::core::ffi::c_int;
            memcpy(
                &raw mut palette as *mut ::core::ffi::c_uchar
                    as *mut ::core::ffi::c_void,
                data as *const ::core::ffi::c_void,
                (npal as size_t).wrapping_mul(3 as size_t),
            );
        } else if memcmp(
            type_0 as *const ::core::ffi::c_void,
            b"tRNS\0" as *const u8 as *const ::core::ffi::c_char
                as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0
        {
            has_trns = true_0 != 0;
            if ct == 3 as ::core::ffi::c_int {
                memcpy(
                    &raw mut palpha as *mut ::core::ffi::c_uchar
                        as *mut ::core::ffi::c_void,
                    data as *const ::core::ffi::c_void,
                    (if len > 256 as uint32_t { 256 as uint32_t } else { len }) as size_t,
                );
            } else if ct == 0 as ::core::ffi::c_int && len >= 2 as uint32_t {
                trns_key[0 as ::core::ffi::c_int as usize] = be16(data);
            } else if ct == 2 as ::core::ffi::c_int && len >= 6 as uint32_t {
                trns_key[0 as ::core::ffi::c_int as usize] = be16(data);
                trns_key[1 as ::core::ffi::c_int as usize] = be16(
                    data.offset(2 as ::core::ffi::c_int as isize),
                );
                trns_key[2 as ::core::ffi::c_int as usize] = be16(
                    data.offset(4 as ::core::ffi::c_int as isize),
                );
            }
        } else if memcmp(
            type_0 as *const ::core::ffi::c_void,
            b"IEND\0" as *const u8 as *const ::core::ffi::c_char
                as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0
        {
            break;
        }
        pos = pos.wrapping_add((12 as uint32_t).wrapping_add(len) as size_t);
    }
    let mut raw: pbuf = pbuf {
        data: ::core::ptr::null_mut::<::core::ffi::c_uchar>(),
        len: 0,
        cap: 0,
    };
    if idat.len == 0
        || pbuf_inflate(&raw mut raw, idat.data, idat.len) != 0 as ::core::ffi::c_int
    {
        pbuf_free(&raw mut idat);
        pbuf_free(&raw mut raw);
        return -(1 as ::core::ffi::c_int);
    }
    pbuf_free(&raw mut idat);
    let mut gray: bool = ct == 0 as ::core::ffi::c_int || ct == 4 as ::core::ffi::c_int;
    let mut outc: ::core::ffi::c_int = if gray as ::core::ffi::c_int != 0 {
        1 as ::core::ffi::c_int
    } else {
        3 as ::core::ffi::c_int
    };
    let mut alpha: bool = ct == 4 as ::core::ffi::c_int || ct == 6 as ::core::ffi::c_int
        || has_trns as ::core::ffi::c_int != 0;
    let mut npix: size_t = (w as size_t).wrapping_mul(h as size_t);
    let mut color: *mut ::core::ffi::c_uchar = malloc(npix.wrapping_mul(outc as size_t))
        as *mut ::core::ffi::c_uchar;
    let mut amask: *mut ::core::ffi::c_uchar = (if alpha as ::core::ffi::c_int != 0 {
        malloc(npix)
    } else {
        NULL_0
    }) as *mut ::core::ffi::c_uchar;
    if color.is_null() || alpha as ::core::ffi::c_int != 0 && amask.is_null() {
        abort();
    }
    if !amask.is_null() {
        memset(amask as *mut ::core::ffi::c_void, 255 as ::core::ffi::c_int, npix);
    }
    static mut ax0: [::core::ffi::c_int; 7] = [
        0 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    ];
    static mut ay0: [::core::ffi::c_int; 7] = [
        0 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
    ];
    static mut adx: [::core::ffi::c_int; 7] = [
        8 as ::core::ffi::c_int,
        8 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
    ];
    static mut ady: [::core::ffi::c_int; 7] = [
        8 as ::core::ffi::c_int,
        8 as ::core::ffi::c_int,
        8 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
    ];
    let mut passes: ::core::ffi::c_int = if interlace != 0 {
        7 as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    };
    let mut rpos: size_t = 0 as size_t;
    let mut bpp: ::core::ffi::c_int = (channels * bd + 7 as ::core::ffi::c_int)
        / 8 as ::core::ffi::c_int;
    let mut ok: bool = true_0 != 0;
    let mut maxv: ::core::ffi::c_uint = ((1 as ::core::ffi::c_uint)
        << (if bd == 16 as ::core::ffi::c_int { 8 as ::core::ffi::c_int } else { bd }))
        .wrapping_sub(1 as ::core::ffi::c_uint);
    let mut p: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while p < passes && ok as ::core::ffi::c_int != 0 {
        let mut x0: ::core::ffi::c_int = if interlace != 0 {
            ax0[p as usize]
        } else {
            0 as ::core::ffi::c_int
        };
        let mut y0: ::core::ffi::c_int = if interlace != 0 {
            ay0[p as usize]
        } else {
            0 as ::core::ffi::c_int
        };
        let mut dx: ::core::ffi::c_int = if interlace != 0 {
            adx[p as usize]
        } else {
            1 as ::core::ffi::c_int
        };
        let mut dy: ::core::ffi::c_int = if interlace != 0 {
            ady[p as usize]
        } else {
            1 as ::core::ffi::c_int
        };
        let mut pw: ::core::ffi::c_int = w
            .wrapping_sub(x0 as uint32_t)
            .wrapping_add(dx as uint32_t)
            .wrapping_sub(1 as uint32_t)
            .wrapping_div(dx as uint32_t) as ::core::ffi::c_int;
        let mut ph: ::core::ffi::c_int = h
            .wrapping_sub(y0 as uint32_t)
            .wrapping_add(dy as uint32_t)
            .wrapping_sub(1 as uint32_t)
            .wrapping_div(dy as uint32_t) as ::core::ffi::c_int;
        if !(x0 as uint32_t >= w || y0 as uint32_t >= h || pw <= 0 as ::core::ffi::c_int
            || ph <= 0 as ::core::ffi::c_int)
        {
            let mut stride: size_t = (pw as size_t)
                .wrapping_mul(channels as size_t)
                .wrapping_mul(bd as size_t)
                .wrapping_add(7 as size_t)
                .wrapping_div(8 as size_t);
            let mut rows: *mut ::core::ffi::c_uchar = malloc(
                stride.wrapping_mul(ph as size_t).wrapping_add(1 as size_t),
            ) as *mut ::core::ffi::c_uchar;
            if rows.is_null() {
                abort();
            }
            ok = unfilter(raw.data, raw.len, &raw mut rpos, stride, ph, bpp, rows);
            let mut yy: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while ok as ::core::ffi::c_int != 0 && yy < ph {
                let mut row: *const ::core::ffi::c_uchar = rows
                    .offset((yy as size_t).wrapping_mul(stride) as isize);
                let mut xx: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while xx < pw {
                    let mut o: size_t = ((y0 + yy * dy) as size_t)
                        .wrapping_mul(w as size_t)
                        .wrapping_add((x0 + xx * dx) as size_t);
                    if ct == 3 as ::core::ffi::c_int {
                        let mut idx: ::core::ffi::c_uint = sample(row, xx, bd);
                        memcpy(
                            color.offset(o.wrapping_mul(3 as size_t) as isize)
                                as *mut ::core::ffi::c_void,
                            (&raw mut palette as *mut ::core::ffi::c_uchar)
                                .offset(
                                    (if idx < 256 as ::core::ffi::c_uint {
                                        idx
                                    } else {
                                        0 as ::core::ffi::c_uint
                                    })
                                        .wrapping_mul(3 as ::core::ffi::c_uint) as isize,
                                ) as *const ::core::ffi::c_void,
                            3 as size_t,
                        );
                        if !amask.is_null() {
                            *amask.offset(o as isize) = palpha[(idx
                                & 255 as ::core::ffi::c_uint) as usize];
                        }
                    } else if gray {
                        let mut v: ::core::ffi::c_uint = sample(row, xx * channels, bd);
                        *color.offset(o as isize) = (if bd >= 8 as ::core::ffi::c_int {
                            v
                        } else {
                            v.wrapping_mul(255 as ::core::ffi::c_uint).wrapping_div(maxv)
                        }) as ::core::ffi::c_uchar;
                        if ct == 4 as ::core::ffi::c_int {
                            *amask.offset(o as isize) = sample(
                                row,
                                xx * channels + 1 as ::core::ffi::c_int,
                                bd,
                            ) as ::core::ffi::c_uchar;
                        } else if has_trns as ::core::ffi::c_int != 0
                            && sample16(row, xx, bd)
                                == trns_key[0 as ::core::ffi::c_int as usize]
                        {
                            *amask.offset(o as isize) = 0 as ::core::ffi::c_uchar;
                        }
                    } else {
                        let mut c: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        while c < 3 as ::core::ffi::c_int {
                            *color
                                .offset(
                                    o.wrapping_mul(3 as size_t).wrapping_add(c as size_t)
                                        as isize,
                                ) = sample(row, xx * channels + c, bd)
                                as ::core::ffi::c_uchar;
                            c += 1;
                        }
                        if ct == 6 as ::core::ffi::c_int {
                            *amask.offset(o as isize) = sample(
                                row,
                                xx * channels + 3 as ::core::ffi::c_int,
                                bd,
                            ) as ::core::ffi::c_uchar;
                        } else if has_trns as ::core::ffi::c_int != 0
                            && sample16(row, xx * 3 as ::core::ffi::c_int, bd)
                                == trns_key[0 as ::core::ffi::c_int as usize]
                            && sample16(
                                row,
                                xx * 3 as ::core::ffi::c_int + 1 as ::core::ffi::c_int,
                                bd,
                            ) == trns_key[1 as ::core::ffi::c_int as usize]
                            && sample16(
                                row,
                                xx * 3 as ::core::ffi::c_int + 2 as ::core::ffi::c_int,
                                bd,
                            ) == trns_key[2 as ::core::ffi::c_int as usize]
                        {
                            *amask.offset(o as isize) = 0 as ::core::ffi::c_uchar;
                        }
                    }
                    xx += 1;
                }
                yy += 1;
            }
            free(rows as *mut ::core::ffi::c_void);
        }
        p += 1;
    }
    pbuf_free(&raw mut raw);
    if !ok {
        pbuf_printf(
            warnings,
            b"truncated PNG data: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            name,
        );
        free(color as *mut ::core::ffi::c_void);
        free(amask as *mut ::core::ffi::c_void);
        return -(1 as ::core::ffi::c_int);
    }
    if !amask.is_null() {
        let mut opaque: bool = true_0 != 0;
        let mut i: size_t = 0 as size_t;
        while i < npix && opaque as ::core::ffi::c_int != 0 {
            opaque = *amask.offset(i as isize) as ::core::ffi::c_int
                == 255 as ::core::ffi::c_int;
            i = i.wrapping_add(1);
        }
        if !opaque {
            pbuf_deflate(&raw mut (*img).smask, amask, npix, 6 as ::core::ffi::c_int);
        }
    }
    let exponent=png_gamma(std::slice::from_raw_parts(d,n));
    if (exponent-1.).abs()>1e-10 {for i in 0..npix*outc as usize {let value=*color.add(i) as f64/255.;*color.add(i)=(value.powf(exponent)*255.+0.5).clamp(0.,255.) as u8;}}
    pbuf_deflate(
        &raw mut (*img).data,
        color,
        npix.wrapping_mul(outc as size_t),
        6 as ::core::ffi::c_int,
    );
    free(color as *mut ::core::ffi::c_void);
    free(amask as *mut ::core::ffi::c_void);
    (*img).width = w as ::core::ffi::c_int;
    (*img).height = h as ::core::ffi::c_int;
    snprintf(
        &raw mut (*img).dict as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 192]>() as size_t,
        b"/Width %u/Height %u/ColorSpace/%s/BitsPerComponent 8\0" as *const u8
            as *const ::core::ffi::c_char,
        w,
        h,
        if gray as ::core::ffi::c_int != 0 {
            b"DeviceGray\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            b"DeviceRGB\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn jpeg_encode(
    mut d: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut img: *mut pdf_image,
) -> ::core::ffi::c_int {
    let mut pos: size_t = 2 as size_t;
    let mut adobe: bool = false_0 != 0;
    let mut comps: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while pos.wrapping_add(4 as size_t) <= n {
        if *d.offset(pos as isize) as ::core::ffi::c_int != 0xff as ::core::ffi::c_int {
            pos = pos.wrapping_add(1);
        } else {
            let mut m: ::core::ffi::c_uint = *d
                .offset(pos.wrapping_add(1 as size_t) as isize) as ::core::ffi::c_uint;
            if m == 0xff as ::core::ffi::c_uint || m == 0xd8 as ::core::ffi::c_uint
                || m >= 0xd0 as ::core::ffi::c_uint && m <= 0xd7 as ::core::ffi::c_uint
                || m == 0x1 as ::core::ffi::c_uint
            {
                pos = pos
                    .wrapping_add(
                        (if m == 0xff as ::core::ffi::c_uint {
                            1 as ::core::ffi::c_int
                        } else {
                            2 as ::core::ffi::c_int
                        }) as size_t,
                    );
            } else {
                if m == 0xd9 as ::core::ffi::c_uint || m == 0xda as ::core::ffi::c_uint {
                    break;
                }
                let mut len: ::core::ffi::c_uint = be16(
                    d.offset(pos as isize).offset(2 as ::core::ffi::c_int as isize),
                );
                if len < 2 as ::core::ffi::c_uint
                    || pos.wrapping_add(2 as size_t).wrapping_add(len as size_t) > n
                {
                    break;
                }
                let mut seg: *const ::core::ffi::c_uchar = d
                    .offset(pos as isize)
                    .offset(4 as ::core::ffi::c_int as isize);
                if m == 0xee as ::core::ffi::c_uint && len >= 7 as ::core::ffi::c_uint
                    && memcmp(
                        seg as *const ::core::ffi::c_void,
                        b"Adobe\0" as *const u8 as *const ::core::ffi::c_char
                            as *const ::core::ffi::c_void,
                        5 as size_t,
                    ) == 0 as ::core::ffi::c_int
                {
                    adobe = true_0 != 0;
                }
                if m >= 0xc0 as ::core::ffi::c_uint && m <= 0xcf as ::core::ffi::c_uint
                    && m != 0xc4 as ::core::ffi::c_uint
                    && m != 0xc8 as ::core::ffi::c_uint
                    && m != 0xcc as ::core::ffi::c_uint
                    && len >= 8 as ::core::ffi::c_uint
                {
                    (*img).height = be16(seg.offset(1 as ::core::ffi::c_int as isize))
                        as ::core::ffi::c_int;
                    (*img).width = be16(seg.offset(3 as ::core::ffi::c_int as isize))
                        as ::core::ffi::c_int;
                    comps = *seg.offset(5 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int;
                }
                pos = pos
                    .wrapping_add(
                        (2 as ::core::ffi::c_uint).wrapping_add(len) as size_t,
                    );
            }
        }
    }
    if (*img).width == 0 || (*img).height == 0
        || comps != 1 as ::core::ffi::c_int && comps != 3 as ::core::ffi::c_int
            && comps != 4 as ::core::ffi::c_int
    {
        return -(1 as ::core::ffi::c_int);
    }
    snprintf(
        &raw mut (*img).dict as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 192]>() as size_t,
        b"/Width %d/Height %d/ColorSpace/%s/BitsPerComponent 8%s\0" as *const u8
            as *const ::core::ffi::c_char,
        (*img).width,
        (*img).height,
        if comps == 1 as ::core::ffi::c_int {
            b"DeviceGray\0" as *const u8 as *const ::core::ffi::c_char
        } else if comps == 3 as ::core::ffi::c_int {
            b"DeviceRGB\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            b"DeviceCMYK\0" as *const u8 as *const ::core::ffi::c_char
        },
        if comps == 4 as ::core::ffi::c_int && adobe as ::core::ffi::c_int != 0 {
            b"/Decode[1 0 1 0 1 0 1 0]\0" as *const u8 as *const ::core::ffi::c_char
        } else {
            b"\0" as *const u8 as *const ::core::ffi::c_char
        },
    );
    (*img).dct = true_0 != 0;
    pbuf_append(&raw mut (*img).data, d as *const ::core::ffi::c_void, n);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn bmp_encode(
    mut d: *const ::core::ffi::c_uchar,
    mut n: size_t,
    mut img: *mut pdf_image,
    mut warnings: *mut pbuf,
    mut name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if n < 54 as size_t {
        return -(1 as ::core::ffi::c_int);
    }
    let mut off: uint32_t = le32(d.offset(10 as ::core::ffi::c_int as isize));
    let mut hsize: uint32_t = le32(d.offset(14 as ::core::ffi::c_int as isize));
    if hsize < 40 as uint32_t {
        pbuf_printf(
            warnings,
            b"unsupported BMP header: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            name,
        );
        return -(1 as ::core::ffi::c_int);
    }
    let mut w: int32_t = le32(d.offset(18 as ::core::ffi::c_int as isize)) as int32_t;
    let mut h: int32_t = le32(d.offset(22 as ::core::ffi::c_int as isize)) as int32_t;
    let mut bpp: ::core::ffi::c_uint = le16(d.offset(28 as ::core::ffi::c_int as isize));
    let mut comp: uint32_t = le32(d.offset(30 as ::core::ffi::c_int as isize));
    let mut ncol: uint32_t = le32(d.offset(46 as ::core::ffi::c_int as isize));
    let mut topdown: bool = h < 0 as int32_t;
    if h < 0 as int32_t {
        h = -h;
    }
    if w <= 0 as int32_t || h <= 0 as int32_t || w > 30000 as int32_t
        || h > 30000 as int32_t || comp != 0 as uint32_t
        || bpp != 8 as ::core::ffi::c_uint && bpp != 24 as ::core::ffi::c_uint
            && bpp != 32 as ::core::ffi::c_uint
    {
        pbuf_printf(
            warnings,
            b"unsupported BMP variant (compressed or %u-bit): %s\n\0" as *const u8
                as *const ::core::ffi::c_char,
            bpp,
            name,
        );
        return -(1 as ::core::ffi::c_int);
    }
    let mut stride: size_t = (w as size_t)
        .wrapping_mul(bpp as size_t)
        .wrapping_add(31 as size_t)
        .wrapping_div(32 as size_t)
        .wrapping_mul(4 as size_t);
    if off as size_t > n
        || stride.wrapping_mul(h as size_t) > n.wrapping_sub(off as size_t)
    {
        return -(1 as ::core::ffi::c_int);
    }
    let mut pal: *const ::core::ffi::c_uchar = d
        .offset(14 as ::core::ffi::c_int as isize)
        .offset(hsize as isize);
    if bpp == 8 as ::core::ffi::c_uint && ncol == 0 as uint32_t {
        ncol = 256 as uint32_t;
    }
    let mut rgb: *mut ::core::ffi::c_uchar = malloc(
        (w as size_t).wrapping_mul(h as size_t).wrapping_mul(3 as size_t),
    ) as *mut ::core::ffi::c_uchar;
    if rgb.is_null() {
        abort();
    }
    let mut y: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while (y as int32_t) < h {
        let mut row: *const ::core::ffi::c_uchar = d
            .offset(off as isize)
            .offset(
                stride
                    .wrapping_mul(
                        (if topdown as ::core::ffi::c_int != 0 {
                            y as int32_t
                        } else {
                            h - 1 as int32_t - y as int32_t
                        }) as size_t,
                    ) as isize,
            );
        let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while (x as int32_t) < w {
            let mut o: *mut ::core::ffi::c_uchar = rgb
                .offset(
                    (y as size_t)
                        .wrapping_mul(w as size_t)
                        .wrapping_add(x as size_t)
                        .wrapping_mul(3 as size_t) as isize,
                );
            if bpp == 8 as ::core::ffi::c_uint {
                let mut i: ::core::ffi::c_uint = *row.offset(x as isize)
                    as ::core::ffi::c_uint;
                let mut c: *const ::core::ffi::c_uchar = pal
                    .offset(
                        (4 as ::core::ffi::c_uint)
                            .wrapping_mul(
                                (if (i as uint32_t) < ncol {
                                    i
                                } else {
                                    0 as ::core::ffi::c_uint
                                }),
                            ) as isize,
                    );
                if c.offset(3 as ::core::ffi::c_int as isize) > d.offset(n as isize) {
                    c = pal;
                }
                *o.offset(0 as ::core::ffi::c_int as isize) = *c
                    .offset(2 as ::core::ffi::c_int as isize);
                *o.offset(1 as ::core::ffi::c_int as isize) = *c
                    .offset(1 as ::core::ffi::c_int as isize);
                *o.offset(2 as ::core::ffi::c_int as isize) = *c
                    .offset(0 as ::core::ffi::c_int as isize);
            } else {
                let mut c_0: *const ::core::ffi::c_uchar = row
                    .offset(
                        (x as ::core::ffi::c_uint)
                            .wrapping_mul(bpp.wrapping_div(8 as ::core::ffi::c_uint))
                            as isize,
                    );
                *o.offset(0 as ::core::ffi::c_int as isize) = *c_0
                    .offset(2 as ::core::ffi::c_int as isize);
                *o.offset(1 as ::core::ffi::c_int as isize) = *c_0
                    .offset(1 as ::core::ffi::c_int as isize);
                *o.offset(2 as ::core::ffi::c_int as isize) = *c_0
                    .offset(0 as ::core::ffi::c_int as isize);
            }
            x += 1;
        }
        y += 1;
    }
    pbuf_deflate(
        &raw mut (*img).data,
        rgb,
        (w as size_t).wrapping_mul(h as size_t).wrapping_mul(3 as size_t),
        6 as ::core::ffi::c_int,
    );
    free(rgb as *mut ::core::ffi::c_void);
    (*img).width = w as ::core::ffi::c_int;
    (*img).height = h as ::core::ffi::c_int;
    snprintf(
        &raw mut (*img).dict as *mut ::core::ffi::c_char,
        ::core::mem::size_of::<[::core::ffi::c_char; 192]>() as size_t,
        b"/Width %d/Height %d/ColorSpace/DeviceRGB/BitsPerComponent 8\0" as *const u8
            as *const ::core::ffi::c_char,
        w,
        h,
    );
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn image_encode(
    mut data: *const ::core::ffi::c_uchar,
    mut len: size_t,
    mut name: *const ::core::ffi::c_char,
    mut out: *mut pdf_image,
    mut warnings: *mut pbuf,
) -> ::core::ffi::c_int {
    memset(
        out as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<pdf_image>() as size_t,
    );
    let mut info: pitex_img_info = pitex_img_info {
        kind: PITEX_IMG_NONE,
        width: 0,
        height: 0,
        xdpi: 0.,
        ydpi: 0.,
    };
    if pitex_img_info_read(data, len, &raw mut info) != 0 as ::core::ffi::c_int {
        pbuf_printf(
            warnings,
            b"unrecognized image format: %s\n\0" as *const u8
                as *const ::core::ffi::c_char,
            name,
        );
        return -(1 as ::core::ffi::c_int);
    }
    (*out).xdpi = info.xdpi;
    (*out).ydpi = info.ydpi;
    let mut r: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
    match info.kind as ::core::ffi::c_uint {
        1 => {
            r = png_encode(data, len, out, warnings, name);
        }
        2 => {
            r = jpeg_encode(data, len, out);
        }
        3 => {
            r = bmp_encode(data, len, out, warnings, name);
        }
        _ => {}
    }
    if r != 0 as ::core::ffi::c_int {
        image_free(out);
        pbuf_printf(
            warnings,
            b"could not decode image: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            name,
        );
        return -(1 as ::core::ffi::c_int);
    }
    (*out).ok = true_0 != 0;
    return 0 as ::core::ffi::c_int;
}
