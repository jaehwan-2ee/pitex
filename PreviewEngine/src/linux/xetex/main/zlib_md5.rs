// Translated from xetex/main/zlib_md5.c with C2Rust 0.22.1.
extern "C" {
    pub type internal_state;
    pub type ttbc_input_handle_t;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn inflate(strm: z_streamp, flush: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn inflateEnd(strm: z_streamp) -> ::core::ffi::c_int;
    fn uncompress(
        dest: *mut Bytef,
        destLen: *mut uLongf,
        source: *const Bytef,
        sourceLen: uLong,
    ) -> ::core::ffi::c_int;
    fn inflateInit_(
        strm: z_streamp,
        version: *const ::core::ffi::c_char,
        stream_size: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
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
    fn ttstub_input_open(
        path: *const ::core::ffi::c_char,
        format: ttbc_file_format,
        is_gz: ::core::ffi::c_int,
    ) -> rust_input_handle_t;
    fn ttstub_input_read(
        handle: rust_input_handle_t,
        data: *mut ::core::ffi::c_char,
        len: size_t,
    ) -> ssize_t;
    fn ttstub_input_close(handle: rust_input_handle_t) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type ssize_t = isize;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type uint_fast32_t = ::core::ffi::c_ulong;
pub type Byte = ::core::ffi::c_uchar;
pub type uInt = ::core::ffi::c_uint;
pub type uLong = ::core::ffi::c_ulong;
pub type Bytef = Byte;
pub type uLongf = uLong;
pub type voidpf = *mut ::core::ffi::c_void;
pub type alloc_func = Option<unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf>;
pub type free_func = Option<unsafe extern "C" fn(voidpf, voidpf) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct z_stream_s {
    pub next_in: *mut Bytef,
    pub avail_in: uInt,
    pub total_in: uLong,
    pub next_out: *mut Bytef,
    pub avail_out: uInt,
    pub total_out: uLong,
    pub msg: *mut ::core::ffi::c_char,
    pub state: *mut internal_state,
    pub zalloc: alloc_func,
    pub zfree: free_func,
    pub opaque: voidpf,
    pub data_type: ::core::ffi::c_int,
    pub adler: uLong,
    pub reserved: uLong,
}
pub type z_stream = z_stream_s;
pub type z_streamp = *mut z_stream;
pub type rust_input_handle_t = *mut ttbc_input_handle_t;
pub type ttbc_file_format = ::core::ffi::c_uint;
pub const TTBC_FILE_FORMAT_VF: ttbc_file_format = 33;
pub const TTBC_FILE_FORMAT_TYPE1: ttbc_file_format = 32;
pub const TTBC_FILE_FORMAT_TRUE_TYPE: ttbc_file_format = 36;
pub const TTBC_FILE_FORMAT_TFM: ttbc_file_format = 3;
pub const TTBC_FILE_FORMAT_TEX_PS_HEADER: ttbc_file_format = 30;
pub const TTBC_FILE_FORMAT_TEX: ttbc_file_format = 26;
pub const TTBC_FILE_FORMAT_TECTONIC_PRIMARY: ttbc_file_format = 59;
pub const TTBC_FILE_FORMAT_SFD: ttbc_file_format = 46;
pub const TTBC_FILE_FORMAT_PROGRAM_DATA: ttbc_file_format = 39;
pub const TTBC_FILE_FORMAT_PK: ttbc_file_format = 1;
pub const TTBC_FILE_FORMAT_PICT: ttbc_file_format = 25;
pub const TTBC_FILE_FORMAT_OVF: ttbc_file_format = 23;
pub const TTBC_FILE_FORMAT_OPEN_TYPE: ttbc_file_format = 47;
pub const TTBC_FILE_FORMAT_OFM: ttbc_file_format = 20;
pub const TTBC_FILE_FORMAT_MISC_FONTS: ttbc_file_format = 41;
pub const TTBC_FILE_FORMAT_FONT_MAP: ttbc_file_format = 11;
pub const TTBC_FILE_FORMAT_FORMAT: ttbc_file_format = 10;
pub const TTBC_FILE_FORMAT_ENC: ttbc_file_format = 44;
pub const TTBC_FILE_FORMAT_CNF: ttbc_file_format = 8;
pub const TTBC_FILE_FORMAT_CMAP: ttbc_file_format = 45;
pub const TTBC_FILE_FORMAT_BST: ttbc_file_format = 7;
pub const TTBC_FILE_FORMAT_BIB: ttbc_file_format = 6;
pub const TTBC_FILE_FORMAT_AFM: ttbc_file_format = 4;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _picohash_md5_ctx_t {
    pub lo: uint_fast32_t,
    pub hi: uint_fast32_t,
    pub a: uint_fast32_t,
    pub b: uint_fast32_t,
    pub c: uint_fast32_t,
    pub d: uint_fast32_t,
    pub buffer: [::core::ffi::c_uchar; 64],
    pub block: [uint_fast32_t; 16],
    pub _body: Option<
        unsafe extern "C" fn(
            *mut _picohash_md5_ctx_t,
            *const ::core::ffi::c_void,
            size_t,
        ) -> *const ::core::ffi::c_void,
    >,
}
pub type FlateResult = ::core::ffi::c_int;
pub const FlateResult_OtherError: FlateResult = -2;
pub const FlateResult_BufError: FlateResult = -1;
pub const FlateResult_StreamEnd: FlateResult = 1;
pub const FlateResult_Success: FlateResult = 0;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const ZLIB_VERSION: [::core::ffi::c_char; 4] = unsafe {
    ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"1.3\0")
};
pub const Z_NO_FLUSH: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const Z_OK: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const Z_STREAM_END: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn tectonic_flate_compress(
    mut output_ptr: *mut uint8_t,
    mut output_len: *mut uint64_t,
    mut input_ptr: *const uint8_t,
    mut input_len: uint64_t,
    mut compression_level: uint32_t,
) -> FlateResult {
    abort();
}
#[no_mangle]
pub unsafe extern "C" fn tectonic_flate_decompress(
    mut output_ptr: *mut uint8_t,
    mut output_len: *mut ::core::ffi::c_ulong,
    mut input_ptr: *const uint8_t,
    mut input_len: uint64_t,
) -> FlateResult {
    return uncompress(
        output_ptr as *mut Bytef,
        output_len as *mut uLongf,
        input_ptr as *const Bytef,
        input_len as uLong,
    ) as FlateResult;
}
#[no_mangle]
pub unsafe extern "C" fn tectonic_flate_new_decompressor(
    mut input_ptr: *const uint8_t,
    mut input_len: uint64_t,
) -> *mut ::core::ffi::c_void {
    let mut strm: *mut z_stream = calloc(
        ::core::mem::size_of::<z_stream>() as size_t,
        1 as size_t,
    ) as *mut z_stream;
    let mut ret: ::core::ffi::c_int = inflateInit_(
        strm as z_streamp,
        ZLIB_VERSION.as_ptr(),
        ::core::mem::size_of::<z_stream>() as ::core::ffi::c_int,
    );
    if ret != Z_OK {
        free(strm as *mut ::core::ffi::c_void);
        return NULL;
    }
    (*strm).avail_in = input_len as uInt;
    (*strm).next_in = input_ptr as *mut ::core::ffi::c_void as *mut Bytef;
    return strm as *mut ::core::ffi::c_void;
}
#[no_mangle]
pub unsafe extern "C" fn tectonic_flate_decompress_chunk(
    mut handle: *mut ::core::ffi::c_void,
    mut output_ptr: *mut uint8_t,
    mut output_len: *mut uint64_t,
) -> ::core::ffi::c_int {
    let mut strm: *mut z_stream = handle as *mut z_stream;
    (*strm).avail_out = *output_len as uInt;
    (*strm).next_out = output_ptr as *mut Bytef;
    let mut ret: ::core::ffi::c_int = inflate(strm as z_streamp, Z_NO_FLUSH);
    *output_len = (*output_len).wrapping_sub((*strm).avail_out as uint64_t);
    return (ret != Z_OK && ret != Z_STREAM_END) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tectonic_flate_free_decompressor(
    mut handle: *mut ::core::ffi::c_void,
) {
    let mut strm: *mut z_stream = handle as *mut z_stream;
    inflateEnd(strm as z_streamp);
    free(strm as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn ttbc_get_data_md5(
    mut data: *const uint8_t,
    mut len: size_t,
    mut digest: *mut uint8_t,
) -> ::core::ffi::c_int {
    let mut ctx: _picohash_md5_ctx_t = _picohash_md5_ctx_t {
        lo: 0,
        hi: 0,
        a: 0,
        b: 0,
        c: 0,
        d: 0,
        buffer: [0; 64],
        block: [0; 16],
        _body: None,
    };
    _picohash_md5_init(&raw mut ctx);
    _picohash_md5_update(&raw mut ctx, data as *const ::core::ffi::c_void, len);
    _picohash_md5_final(&raw mut ctx, digest as *mut ::core::ffi::c_void);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn ttstub_get_file_md5(
    mut path: *const ::core::ffi::c_char,
    mut digest: *mut ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut input: *mut ttbc_input_handle_t = ttstub_input_open(
        path,
        TTBC_FILE_FORMAT_PROGRAM_DATA,
        0 as ::core::ffi::c_int,
    ) as *mut ttbc_input_handle_t;
    if input.is_null() {
        return 1 as ::core::ffi::c_int;
    }
    let mut ctx: _picohash_md5_ctx_t = _picohash_md5_ctx_t {
        lo: 0,
        hi: 0,
        a: 0,
        b: 0,
        c: 0,
        d: 0,
        buffer: [0; 64],
        block: [0; 16],
        _body: None,
    };
    _picohash_md5_init(&raw mut ctx);
    let mut buf: [::core::ffi::c_char; 4096] = [0; 4096];
    let mut r: size_t = 0;
    loop {
        r = ttstub_input_read(
            input as rust_input_handle_t,
            &raw mut buf as *mut ::core::ffi::c_char,
            4096 as size_t,
        ) as size_t;
        if !(r > 0 as size_t) {
            break;
        }
        _picohash_md5_update(
            &raw mut ctx,
            &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            r,
        );
    }
    ttstub_input_close(input as rust_input_handle_t);
    _picohash_md5_final(&raw mut ctx, digest as *mut ::core::ffi::c_void);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn _picohash_md5_body(
    mut ctx: *mut _picohash_md5_ctx_t,
    mut data: *const ::core::ffi::c_void,
    mut size: size_t,
) -> *const ::core::ffi::c_void {
    let mut ptr: *const ::core::ffi::c_uchar = ::core::ptr::null::<
        ::core::ffi::c_uchar,
    >();
    let mut a: uint_fast32_t = 0;
    let mut b: uint_fast32_t = 0;
    let mut c: uint_fast32_t = 0;
    let mut d: uint_fast32_t = 0;
    let mut saved_a: uint_fast32_t = 0;
    let mut saved_b: uint_fast32_t = 0;
    let mut saved_c: uint_fast32_t = 0;
    let mut saved_d: uint_fast32_t = 0;
    ptr = data as *const ::core::ffi::c_uchar;
    a = (*ctx).a;
    b = (*ctx).b;
    c = (*ctx).c;
    d = (*ctx).d;
    loop {
        saved_a = a;
        saved_b = b;
        saved_c = c;
        saved_d = d;
        a = a
            .wrapping_add(
                (d ^ b & (c ^ d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (0 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xd76aa478 as uint_fast32_t),
            );
        a = a << 7 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 7 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (c ^ a & (b ^ c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (1 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xe8c7b756 as uint_fast32_t),
            );
        d = d << 12 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 12 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (b ^ d & (a ^ b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (2 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x242070db as uint_fast32_t),
            );
        c = c << 17 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 17 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (a ^ c & (d ^ a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (3 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xc1bdceee as uint_fast32_t),
            );
        b = b << 22 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 22 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (d ^ b & (c ^ d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (4 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xf57c0faf as uint_fast32_t),
            );
        a = a << 7 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 7 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (c ^ a & (b ^ c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (5 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x4787c62a as uint_fast32_t),
            );
        d = d << 12 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 12 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (b ^ d & (a ^ b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (6 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xa8304613 as uint_fast32_t),
            );
        c = c << 17 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 17 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (a ^ c & (d ^ a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (7 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xfd469501 as uint_fast32_t),
            );
        b = b << 22 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 22 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (d ^ b & (c ^ d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (8 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x698098d8 as uint_fast32_t),
            );
        a = a << 7 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 7 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (c ^ a & (b ^ c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (9 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x8b44f7af as uint_fast32_t),
            );
        d = d << 12 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 12 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (b ^ d & (a ^ b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (10 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xffff5bb1 as uint_fast32_t),
            );
        c = c << 17 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 17 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (a ^ c & (d ^ a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (11 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x895cd7be as uint_fast32_t),
            );
        b = b << 22 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 22 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (d ^ b & (c ^ d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (12 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x6b901122 as uint_fast32_t),
            );
        a = a << 7 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 7 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (c ^ a & (b ^ c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (13 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xfd987193 as uint_fast32_t),
            );
        d = d << 12 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 12 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (b ^ d & (a ^ b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (14 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xa679438e as uint_fast32_t),
            );
        c = c << 17 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 17 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (a ^ c & (d ^ a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (15 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x49b40821 as uint_fast32_t),
            );
        b = b << 22 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 22 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (c ^ d & (b ^ c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (1 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xf61e2562 as uint_fast32_t),
            );
        a = a << 5 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 5 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (b ^ c & (a ^ b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (6 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xc040b340 as uint_fast32_t),
            );
        d = d << 9 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 9 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (a ^ b & (d ^ a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (11 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x265e5a51 as uint_fast32_t),
            );
        c = c << 14 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 14 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (d ^ a & (c ^ d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (0 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xe9b6c7aa as uint_fast32_t),
            );
        b = b << 20 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 20 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (c ^ d & (b ^ c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (5 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xd62f105d as uint_fast32_t),
            );
        a = a << 5 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 5 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (b ^ c & (a ^ b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (10 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x2441453 as uint_fast32_t),
            );
        d = d << 9 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 9 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (a ^ b & (d ^ a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (15 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xd8a1e681 as uint_fast32_t),
            );
        c = c << 14 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 14 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (d ^ a & (c ^ d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (4 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xe7d3fbc8 as uint_fast32_t),
            );
        b = b << 20 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 20 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (c ^ d & (b ^ c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (9 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x21e1cde6 as uint_fast32_t),
            );
        a = a << 5 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 5 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (b ^ c & (a ^ b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (14 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xc33707d6 as uint_fast32_t),
            );
        d = d << 9 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 9 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (a ^ b & (d ^ a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (3 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xf4d50d87 as uint_fast32_t),
            );
        c = c << 14 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 14 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (d ^ a & (c ^ d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (8 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x455a14ed as uint_fast32_t),
            );
        b = b << 20 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 20 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (c ^ d & (b ^ c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (13 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xa9e3e905 as uint_fast32_t),
            );
        a = a << 5 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 5 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (b ^ c & (a ^ b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (2 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xfcefa3f8 as uint_fast32_t),
            );
        d = d << 9 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 9 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (a ^ b & (d ^ a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (7 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x676f02d9 as uint_fast32_t),
            );
        c = c << 14 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 14 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (d ^ a & (c ^ d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (12 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x8d2a4c8a as uint_fast32_t),
            );
        b = b << 20 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 20 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (b ^ c ^ d)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (5 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xfffa3942 as uint_fast32_t),
            );
        a = a << 4 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 4 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (a ^ b ^ c)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (8 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x8771f681 as uint_fast32_t),
            );
        d = d << 11 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 11 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (d ^ a ^ b)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (11 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x6d9d6122 as uint_fast32_t),
            );
        c = c << 16 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 16 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (c ^ d ^ a)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (14 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xfde5380c as uint_fast32_t),
            );
        b = b << 23 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 23 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (b ^ c ^ d)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (1 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xa4beea44 as uint_fast32_t),
            );
        a = a << 4 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 4 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (a ^ b ^ c)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (4 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x4bdecfa9 as uint_fast32_t),
            );
        d = d << 11 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 11 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (d ^ a ^ b)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (7 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xf6bb4b60 as uint_fast32_t),
            );
        c = c << 16 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 16 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (c ^ d ^ a)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (10 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xbebfbc70 as uint_fast32_t),
            );
        b = b << 23 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 23 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (b ^ c ^ d)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (13 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x289b7ec6 as uint_fast32_t),
            );
        a = a << 4 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 4 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (a ^ b ^ c)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (0 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xeaa127fa as uint_fast32_t),
            );
        d = d << 11 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 11 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (d ^ a ^ b)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (3 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xd4ef3085 as uint_fast32_t),
            );
        c = c << 16 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 16 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (c ^ d ^ a)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (6 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x4881d05 as uint_fast32_t),
            );
        b = b << 23 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 23 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (b ^ c ^ d)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (9 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xd9d4d039 as uint_fast32_t),
            );
        a = a << 4 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 4 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (a ^ b ^ c)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (12 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xe6db99e5 as uint_fast32_t),
            );
        d = d << 11 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 11 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (d ^ a ^ b)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (15 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x1fa27cf8 as uint_fast32_t),
            );
        c = c << 16 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 16 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (c ^ d ^ a)
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (2 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xc4ac5665 as uint_fast32_t),
            );
        b = b << 23 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 23 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (c ^ (b | !d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (0 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xf4292244 as uint_fast32_t),
            );
        a = a << 6 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 6 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (b ^ (a | !c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (7 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x432aff97 as uint_fast32_t),
            );
        d = d << 10 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 10 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (a ^ (d | !b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (14 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xab9423a7 as uint_fast32_t),
            );
        c = c << 15 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 15 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (d ^ (c | !a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (5 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xfc93a039 as uint_fast32_t),
            );
        b = b << 21 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 21 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (c ^ (b | !d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (12 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x655b59c3 as uint_fast32_t),
            );
        a = a << 6 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 6 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (b ^ (a | !c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (3 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x8f0ccc92 as uint_fast32_t),
            );
        d = d << 10 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 10 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (a ^ (d | !b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (10 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xffeff47d as uint_fast32_t),
            );
        c = c << 15 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 15 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (d ^ (c | !a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (1 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x85845dd1 as uint_fast32_t),
            );
        b = b << 21 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 21 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (c ^ (b | !d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (8 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x6fa87e4f as uint_fast32_t),
            );
        a = a << 6 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 6 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (b ^ (a | !c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (15 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xfe2ce6e0 as uint_fast32_t),
            );
        d = d << 10 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 10 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (a ^ (d | !b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (6 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xa3014314 as uint_fast32_t),
            );
        c = c << 15 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 15 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (d ^ (c | !a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (13 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x4e0811a1 as uint_fast32_t),
            );
        b = b << 21 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 21 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a
            .wrapping_add(
                (c ^ (b | !d))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (4 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xf7537e82 as uint_fast32_t),
            );
        a = a << 6 as ::core::ffi::c_int
            | (a & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 6 as ::core::ffi::c_int;
        a = a.wrapping_add(b);
        d = d
            .wrapping_add(
                (b ^ (a | !c))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (11 as ::core::ffi::c_int * 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xbd3af235 as uint_fast32_t),
            );
        d = d << 10 as ::core::ffi::c_int
            | (d & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 10 as ::core::ffi::c_int;
        d = d.wrapping_add(a);
        c = c
            .wrapping_add(
                (a ^ (d | !b))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (2 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0x2ad7d2bb as uint_fast32_t),
            );
        c = c << 15 as ::core::ffi::c_int
            | (c & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 15 as ::core::ffi::c_int;
        c = c.wrapping_add(d);
        b = b
            .wrapping_add(
                (d ^ (c | !a))
                    .wrapping_add(
                        *(ptr
                            .offset(
                                (9 as ::core::ffi::c_int * 4 as ::core::ffi::c_int) as isize,
                            ) as *const ::core::ffi::c_uchar as *const uint32_t)
                            as uint_fast32_t,
                    )
                    .wrapping_add(0xeb86d391 as uint_fast32_t),
            );
        b = b << 21 as ::core::ffi::c_int
            | (b & 0xffffffff as uint_fast32_t)
                >> 32 as ::core::ffi::c_int - 21 as ::core::ffi::c_int;
        b = b.wrapping_add(c);
        a = a.wrapping_add(saved_a);
        b = b.wrapping_add(saved_b);
        c = c.wrapping_add(saved_c);
        d = d.wrapping_add(saved_d);
        ptr = ptr.offset(64 as ::core::ffi::c_int as isize);
        size = size.wrapping_sub(64 as size_t);
        if !(size != 0) {
            break;
        }
    }
    (*ctx).a = a;
    (*ctx).b = b;
    (*ctx).c = c;
    (*ctx).d = d;
    return ptr as *const ::core::ffi::c_void;
}
#[inline]
unsafe extern "C" fn _picohash_md5_init(mut ctx: *mut _picohash_md5_ctx_t) {
    (*ctx).a = 0x67452301 as uint_fast32_t;
    (*ctx).b = 0xefcdab89 as uint_fast32_t;
    (*ctx).c = 0x98badcfe as uint_fast32_t;
    (*ctx).d = 0x10325476 as uint_fast32_t;
    (*ctx).lo = 0 as uint_fast32_t;
    (*ctx).hi = 0 as uint_fast32_t;
    (*ctx)._body = Some(
        _picohash_md5_body
            as unsafe extern "C" fn(
                *mut _picohash_md5_ctx_t,
                *const ::core::ffi::c_void,
                size_t,
            ) -> *const ::core::ffi::c_void,
    )
        as Option<
            unsafe extern "C" fn(
                *mut _picohash_md5_ctx_t,
                *const ::core::ffi::c_void,
                size_t,
            ) -> *const ::core::ffi::c_void,
        >;
}
#[inline]
unsafe extern "C" fn _picohash_md5_update(
    mut ctx: *mut _picohash_md5_ctx_t,
    mut data: *const ::core::ffi::c_void,
    mut size: size_t,
) {
    let mut saved_lo: uint_fast32_t = 0;
    let mut used: ::core::ffi::c_ulong = 0;
    let mut free_0: ::core::ffi::c_ulong = 0;
    saved_lo = (*ctx).lo;
    (*ctx).lo = ((saved_lo as size_t).wrapping_add(size)
        & 0x1fffffff as ::core::ffi::c_int as size_t) as uint_fast32_t;
    if (*ctx).lo < saved_lo {
        (*ctx).hi = (*ctx).hi.wrapping_add(1);
    }
    (*ctx).hi = ((*ctx).hi as ::core::ffi::c_ulong)
        .wrapping_add((size >> 29 as ::core::ffi::c_int) as ::core::ffi::c_ulong)
        as uint_fast32_t as uint_fast32_t;
    used = (saved_lo & 0x3f as uint_fast32_t) as ::core::ffi::c_ulong;
    if used != 0 {
        free_0 = (64 as ::core::ffi::c_ulong).wrapping_sub(used);
        if size < free_0 as size_t {
            memcpy(
                (&raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar)
                    .offset(used as isize) as *mut ::core::ffi::c_uchar
                    as *mut ::core::ffi::c_void,
                data,
                size,
            );
            return;
        }
        memcpy(
            (&raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar).offset(used as isize)
                as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
            data,
            free_0 as size_t,
        );
        data = (data as *const ::core::ffi::c_uchar).offset(free_0 as isize)
            as *const ::core::ffi::c_void;
        size = (size as ::core::ffi::c_ulong).wrapping_sub(free_0) as size_t as size_t;
        (*ctx)
            ._body
            .expect(
                "non-null function pointer",
            )(
            ctx as *mut _picohash_md5_ctx_t,
            &raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar
                as *const ::core::ffi::c_void,
            64 as size_t,
        );
    }
    if size >= 64 as size_t {
        data = (*ctx)
            ._body
            .expect(
                "non-null function pointer",
            )(
            ctx as *mut _picohash_md5_ctx_t,
            data,
            size & !(0x3f as ::core::ffi::c_int as size_t),
        );
        size &= 0x3f as size_t;
    }
    memcpy(
        &raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        data,
        size,
    );
}
#[inline]
unsafe extern "C" fn _picohash_md5_final(
    mut ctx: *mut _picohash_md5_ctx_t,
    mut _digest: *mut ::core::ffi::c_void,
) {
    let mut digest: *mut ::core::ffi::c_uchar = _digest as *mut ::core::ffi::c_uchar;
    let mut used: ::core::ffi::c_ulong = 0;
    let mut free_0: ::core::ffi::c_ulong = 0;
    used = ((*ctx).lo & 0x3f as uint_fast32_t) as ::core::ffi::c_ulong;
    let fresh0 = used;
    used = used.wrapping_add(1);
    (*ctx).buffer[fresh0 as usize] = 0x80 as ::core::ffi::c_uchar;
    free_0 = (64 as ::core::ffi::c_ulong).wrapping_sub(used);
    if free_0 < 8 as ::core::ffi::c_ulong {
        memset(
            (&raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar).offset(used as isize)
                as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            free_0 as size_t,
        );
        (*ctx)
            ._body
            .expect(
                "non-null function pointer",
            )(
            ctx as *mut _picohash_md5_ctx_t,
            &raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar
                as *const ::core::ffi::c_void,
            64 as size_t,
        );
        used = 0 as ::core::ffi::c_ulong;
        free_0 = 64 as ::core::ffi::c_ulong;
    }
    memset(
        (&raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar).offset(used as isize)
            as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        (free_0 as size_t).wrapping_sub(8 as size_t),
    );
    (*ctx).lo <<= 3 as ::core::ffi::c_int;
    (*ctx).buffer[56 as ::core::ffi::c_int as usize] = (*ctx).lo as ::core::ffi::c_uchar;
    (*ctx).buffer[57 as ::core::ffi::c_int as usize] = ((*ctx).lo
        >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    (*ctx).buffer[58 as ::core::ffi::c_int as usize] = ((*ctx).lo
        >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    (*ctx).buffer[59 as ::core::ffi::c_int as usize] = ((*ctx).lo
        >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    (*ctx).buffer[60 as ::core::ffi::c_int as usize] = (*ctx).hi as ::core::ffi::c_uchar;
    (*ctx).buffer[61 as ::core::ffi::c_int as usize] = ((*ctx).hi
        >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    (*ctx).buffer[62 as ::core::ffi::c_int as usize] = ((*ctx).hi
        >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    (*ctx).buffer[63 as ::core::ffi::c_int as usize] = ((*ctx).hi
        >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    (*ctx)
        ._body
        .expect(
            "non-null function pointer",
        )(
        ctx as *mut _picohash_md5_ctx_t,
        &raw mut (*ctx).buffer as *mut ::core::ffi::c_uchar
            as *const ::core::ffi::c_void,
        64 as size_t,
    );
    *digest.offset(0 as ::core::ffi::c_int as isize) = (*ctx).a as ::core::ffi::c_uchar;
    *digest.offset(1 as ::core::ffi::c_int as isize) = ((*ctx).a
        >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(2 as ::core::ffi::c_int as isize) = ((*ctx).a
        >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(3 as ::core::ffi::c_int as isize) = ((*ctx).a
        >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(4 as ::core::ffi::c_int as isize) = (*ctx).b as ::core::ffi::c_uchar;
    *digest.offset(5 as ::core::ffi::c_int as isize) = ((*ctx).b
        >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(6 as ::core::ffi::c_int as isize) = ((*ctx).b
        >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(7 as ::core::ffi::c_int as isize) = ((*ctx).b
        >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(8 as ::core::ffi::c_int as isize) = (*ctx).c as ::core::ffi::c_uchar;
    *digest.offset(9 as ::core::ffi::c_int as isize) = ((*ctx).c
        >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(10 as ::core::ffi::c_int as isize) = ((*ctx).c
        >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(11 as ::core::ffi::c_int as isize) = ((*ctx).c
        >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(12 as ::core::ffi::c_int as isize) = (*ctx).d as ::core::ffi::c_uchar;
    *digest.offset(13 as ::core::ffi::c_int as isize) = ((*ctx).d
        >> 8 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(14 as ::core::ffi::c_int as isize) = ((*ctx).d
        >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    *digest.offset(15 as ::core::ffi::c_int as isize) = ((*ctx).d
        >> 24 as ::core::ffi::c_int) as ::core::ffi::c_uchar;
    memset(
        ctx as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<_picohash_md5_ctx_t>() as size_t,
    );
}
