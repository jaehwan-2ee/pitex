/* tectonic/xetex-stringpool.c: preloaded "string pool" constants
   Copyright 2017-2018 the Tectonic Project
   Licensed under the MIT License.
*/
// Translated from xetex/engine/xetex-stringpool.c with C2Rust 0.22.1.
extern "C" {
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    static mut buffer: *mut UnicodeScalar;
    static mut max_strings: int32_t;
    static mut pool_size: int32_t;
    static mut str_pool: *mut packed_UTF16_code;
    static mut str_start: *mut pool_pointer;
    static mut pool_ptr: pool_pointer;
    static mut str_ptr: str_number;
    static mut init_pool_ptr: pool_pointer;
    static mut init_str_ptr: str_number;
    fn overflow(s: *const ::core::ffi::c_char, n: int32_t) -> !;
}
pub type __int32_t = i32;
pub type int32_t = __int32_t;
pub type size_t = usize;
pub type UnicodeScalar = int32_t;
pub type pool_pointer = int32_t;
pub type str_number = int32_t;
pub type packed_UTF16_code = ::core::ffi::c_ushort;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
static mut string_constants: [*const ::core::ffi::c_char; 3] = [
    b"this marks the start of the stringpool\0" as *const u8
        as *const ::core::ffi::c_char,
    b"\0" as *const u8 as *const ::core::ffi::c_char,
    ::core::ptr::null::<::core::ffi::c_char>(),
];
#[no_mangle]
pub unsafe extern "C" fn load_pool_strings(
    mut spare_size: int32_t,
) -> ::core::ffi::c_int {
    let mut s: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut total_len: size_t = 0 as size_t;
    let mut g: str_number = 0 as str_number;
    loop {
        let fresh0 = i;
        i = i + 1;
        s = string_constants[fresh0 as usize];
        if s.is_null() {
            break;
        }
        let mut len: size_t = strlen(s);
        total_len = total_len.wrapping_add(len);
        if total_len >= spare_size as size_t {
            return 0 as ::core::ffi::c_int;
        }
        loop {
            let fresh1 = len;
            len = len.wrapping_sub(1);
            if !(fresh1 > 0 as size_t) {
                break;
            }
            let fresh2 = s;
            s = s.offset(1);
            let fresh3 = pool_ptr;
            pool_ptr = pool_ptr + 1;
            *str_pool.offset(fresh3 as isize) = *fresh2 as packed_UTF16_code;
        }
        g = make_string();
    }
    return g as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn length(mut s: str_number) -> int32_t {
    if s as ::core::ffi::c_long >= 65536 as ::core::ffi::c_long {
        return *str_start
            .offset(
                ((s + 1 as str_number) as ::core::ffi::c_long
                    - 65536 as ::core::ffi::c_long) as isize,
            ) as int32_t
            - *str_start
                .offset(
                    (s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize,
                ) as int32_t;
    }
    if s >= 32 as str_number && s < 127 as str_number {
        return 1 as int32_t;
    }
    if s <= 127 as str_number {
        return 3 as int32_t;
    }
    if s < 256 as str_number {
        return 4 as int32_t;
    }
    return 8 as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn make_string() -> str_number {
    if str_ptr == max_strings {
        overflow(
            b"number of strings\0" as *const u8 as *const ::core::ffi::c_char,
            max_strings - init_str_ptr as int32_t,
        );
    }
    str_ptr += 1;
    *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize) = pool_ptr;
    return str_ptr - 1 as str_number;
}
#[no_mangle]
pub unsafe extern "C" fn append_str(mut s: str_number) {
    let mut i: int32_t = 0;
    let mut j: pool_pointer = 0;
    i = length(s);
    if pool_ptr + i as pool_pointer > pool_size {
        overflow(
            b"pool size\0" as *const u8 as *const ::core::ffi::c_char,
            pool_size - init_pool_ptr as int32_t,
        );
    }
    j = *str_start
        .offset((s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize);
    while i > 0 as int32_t {
        *str_pool.offset(pool_ptr as isize) = *str_pool.offset(j as isize);
        pool_ptr += 1;
        j += 1;
        i -= 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn str_eq_buf(mut s: str_number, mut k: int32_t) -> bool {
    let mut j: pool_pointer = 0;
    j = *str_start
        .offset((s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize);
    while j
        < *str_start
            .offset(
                ((s + 1 as str_number) as ::core::ffi::c_long
                    - 65536 as ::core::ffi::c_long) as isize,
            )
    {
        if *buffer.offset(k as isize) as ::core::ffi::c_long
            >= 65536 as ::core::ffi::c_long
        {
            if *str_pool.offset(j as isize) as ::core::ffi::c_long
                != 55296 as ::core::ffi::c_long
                    + (*buffer.offset(k as isize) as ::core::ffi::c_long
                        - 65536 as ::core::ffi::c_long) / 1024 as ::core::ffi::c_long
            {
                return false_0 != 0
            } else if *str_pool.offset((j + 1 as pool_pointer) as isize)
                as ::core::ffi::c_long
                != 56320 as ::core::ffi::c_long
                    + (*buffer.offset(k as isize) as ::core::ffi::c_long
                        - 65536 as ::core::ffi::c_long) % 1024 as ::core::ffi::c_long
            {
                return false_0 != 0
            } else {
                j += 1;
            }
        } else if *str_pool.offset(j as isize) as UnicodeScalar
            != *buffer.offset(k as isize)
        {
            return false_0 != 0
        }
        j += 1;
        k += 1;
    }
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn str_eq_str(mut s: str_number, mut t: str_number) -> bool {
    let mut j: pool_pointer = 0;
    let mut k: pool_pointer = 0;
    if length(s) != length(t) {
        return false_0 != 0;
    }
    if length(s) == 1 as int32_t {
        if (s as ::core::ffi::c_long) < 65536 as ::core::ffi::c_long {
            if (t as ::core::ffi::c_long) < 65536 as ::core::ffi::c_long {
                if s != t {
                    return false_0 != 0;
                }
            } else if s
                != *str_pool
                    .offset(
                        *str_start
                            .offset(
                                (t as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                                    as isize,
                            ) as isize,
                    ) as str_number
            {
                return false_0 != 0
            }
        } else if (t as ::core::ffi::c_long) < 65536 as ::core::ffi::c_long {
            if *str_pool
                .offset(
                    *str_start
                        .offset(
                            (s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                                as isize,
                        ) as isize,
                ) as str_number != t
            {
                return false_0 != 0;
            }
        } else if *str_pool
            .offset(
                *str_start
                    .offset(
                        (s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                            as isize,
                    ) as isize,
            ) as ::core::ffi::c_int
            != *str_pool
                .offset(
                    *str_start
                        .offset(
                            (t as ::core::ffi::c_long - 65536 as ::core::ffi::c_long)
                                as isize,
                        ) as isize,
                ) as ::core::ffi::c_int
        {
            return false_0 != 0
        }
    } else {
        j = *str_start
            .offset((s as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize);
        k = *str_start
            .offset((t as ::core::ffi::c_long - 65536 as ::core::ffi::c_long) as isize);
        while j
            < *str_start
                .offset(
                    ((s + 1 as str_number) as ::core::ffi::c_long
                        - 65536 as ::core::ffi::c_long) as isize,
                )
        {
            if *str_pool.offset(j as isize) as ::core::ffi::c_int
                != *str_pool.offset(k as isize) as ::core::ffi::c_int
            {
                return false_0 != 0;
            }
            j += 1;
            k += 1;
        }
    }
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn search_string(mut search: str_number) -> str_number {
    let mut s: str_number = 0;
    let mut len: int32_t = 0;
    len = length(search);
    if len == 0 as int32_t {
        return EMPTY_STRING as str_number
    } else {
        s = search - 1 as str_number;
        while s as ::core::ffi::c_long > 65535 as ::core::ffi::c_long {
            if length(s) == len {
                if str_eq_str(s, search) {
                    return s;
                }
            }
            s -= 1;
        }
    }
    return 0 as str_number;
}
#[no_mangle]
pub unsafe extern "C" fn slow_make_string() -> str_number {
    let mut s: str_number = 0;
    let mut t: str_number = 0;
    t = make_string();
    s = search_string(t);
    if s > 0 as str_number {
        str_ptr -= 1;
        pool_ptr = *str_start.offset((str_ptr - TOO_BIG_CHAR as str_number) as isize);
        return s;
    }
    return t;
}
pub const EMPTY_STRING: ::core::ffi::c_long = 65536 as ::core::ffi::c_long
    + 1 as ::core::ffi::c_long;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const TOO_BIG_CHAR: ::core::ffi::c_int = 65536 as ::core::ffi::c_int;
