/*
 * MIT License
 *
 * Copyright (c) 2023 Frédéric Bour <frederic.bour@lakaban.net>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to
 * deal in the Software without restriction, including without limitation the
 * rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
 * sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
 * IN THE SOFTWARE.
 */
// Translated from driver/sprotocol.c with C2Rust 0.22.1.
extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    static mut stderr: *mut FILE;
    fn fflush(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn perror(__s: *const ::core::ffi::c_char);
    fn myabort_(
        file: *const ::core::ffi::c_char,
        line: ::core::ffi::c_int,
        msg: *const ::core::ffi::c_char,
        answer: uint32_t,
    ) -> !;
    fn print_backtrace();
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn abort() -> !;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memmove(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn write(
        __fd: ::core::ffi::c_int,
        __buf: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ssize_t;
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn poll(
        __fds: *mut pollfd,
        __nfds: nfds_t,
        __timeout: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn recvmsg(
        __fd: ::core::ffi::c_int,
        __message: *mut msghdr,
        __flags: ::core::ffi::c_int,
    ) -> ssize_t;
}
pub type size_t = usize;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type __ssize_t = ::core::ffi::c_long;
pub type __socklen_t = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut _IO_codecvt,
    pub _wide_data: *mut _IO_wide_data,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type ssize_t = __ssize_t;
pub type int32_t = __int32_t;
pub type uint32_t = __uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct channel_s {
    pub input: C2RustUnnamed_0,
    pub output: C2RustUnnamed,
    pub passed_fd: ::core::ffi::c_int,
    pub buf: *mut ::core::ffi::c_char,
    pub buf_size: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed {
    pub buffer: [::core::ffi::c_char; 4096],
    pub pos: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_0 {
    pub buffer: [::core::ffi::c_char; 4096],
    pub pos: ::core::ffi::c_int,
    pub len: ::core::ffi::c_int,
}
pub type channel_t = channel_s;
pub type file_id = ::core::ffi::c_int;
pub type query = ::core::ffi::c_uint;
pub const Q_FNTB: query = 1112821318;
pub const Q_CHLD: query = 1145849923;
pub const Q_SPIC: query = 1128878163;
pub const Q_GPIC: query = 1128878151;
pub const Q_SEEN: query = 1313162579;
pub const Q_MTIM: query = 1296651341;
pub const Q_SIZE: query = 1163544915;
pub const Q_CLOS: query = 1397705795;
pub const Q_APND: query = 1145983041;
pub const Q_READ: query = 1145128274;
pub const Q_OPWR: query = 1381453903;
pub const Q_OPRD: query = 1146245199;
pub type txp_file_kind = ::core::ffi::c_uint;
pub const TXP_KIND_OTHER: txp_file_kind = 1380471887;
pub const TXP_KIND_VF: txp_file_kind = 18006;
pub const TXP_KIND_TYPE1: txp_file_kind = 827349332;
pub const TXP_KIND_TRUE_TYPE: txp_file_kind = 4609108;
pub const TXP_KIND_TFM: txp_file_kind = 5064276;
pub const TXP_KIND_TEX_PS_HEADER: txp_file_kind = 1213419604;
pub const TXP_KIND_TEX: txp_file_kind = 5784916;
pub const TXP_KIND_PRIMARY: txp_file_kind = 1296650832;
pub const TXP_KIND_SFD: txp_file_kind = 4474451;
pub const TXP_KIND_PROGRAM_DATA: txp_file_kind = 1413563472;
pub const TXP_KIND_PK: txp_file_kind = 19280;
pub const TXP_KIND_PICT: txp_file_kind = 1413695824;
pub const TXP_KIND_OVF: txp_file_kind = 4609615;
pub const TXP_KIND_OPEN_TYPE: txp_file_kind = 4609103;
pub const TXP_KIND_OFM: txp_file_kind = 5064271;
pub const TXP_KIND_MISC_FONTS: txp_file_kind = 1414415949;
pub const TXP_KIND_FONT_MAP: txp_file_kind = 1346456902;
pub const TXP_KIND_FORMAT: txp_file_kind = 1414353478;
pub const TXP_KIND_ENC: txp_file_kind = 4410949;
pub const TXP_KIND_CNF: txp_file_kind = 4607555;
pub const TXP_KIND_CMAP: txp_file_kind = 1346456899;
pub const TXP_KIND_BST: txp_file_kind = 5526338;
pub const TXP_KIND_BIB: txp_file_kind = 4344130;
pub const TXP_KIND_AFM: txp_file_kind = 5064257;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pic_cache {
    pub type_0: ::core::ffi::c_int,
    pub page: ::core::ffi::c_int,
    pub bounds: [::core::ffi::c_float; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct query_t {
    pub time: ::core::ffi::c_int,
    pub tag: query,
    pub c2rust_unnamed: C2RustUnnamed_1,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_1 {
    pub open: C2RustUnnamed_11,
    pub read: C2RustUnnamed_10,
    pub apnd: C2RustUnnamed_9,
    pub clos: C2RustUnnamed_8,
    pub size: C2RustUnnamed_7,
    pub mtim: C2RustUnnamed_6,
    pub seen: C2RustUnnamed_5,
    pub chld: C2RustUnnamed_4,
    pub gpic: C2RustUnnamed_3,
    pub spic: C2RustUnnamed_2,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_2 {
    pub path: *mut ::core::ffi::c_char,
    pub cache: pic_cache,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_3 {
    pub path: *mut ::core::ffi::c_char,
    pub type_0: ::core::ffi::c_int,
    pub page: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_4 {
    pub fd: ::core::ffi::c_int,
    pub pid: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_5 {
    pub fid: file_id,
    pub pos: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_6 {
    pub fid: file_id,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_7 {
    pub fid: file_id,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_8 {
    pub fid: file_id,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_9 {
    pub fid: file_id,
    pub size: ::core::ffi::c_int,
    pub buf: *mut ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_10 {
    pub fid: file_id,
    pub pos: ::core::ffi::c_int,
    pub size: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_11 {
    pub fid: file_id,
    pub path: *mut ::core::ffi::c_char,
    pub kind: txp_file_kind,
}
pub type answer = ::core::ffi::c_uint;
pub const A_GPIC: answer = 1128878151;
pub const A_OPEN: answer = 1313165391;
pub const A_FORK: answer = 1263685446;
pub const A_READ: answer = 1145128274;
pub const A_MTIM: answer = 1296651341;
pub const A_SIZE: answer = 1163544915;
pub const A_PASS: answer = 1397965136;
pub const A_DONE: answer = 1162760004;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct answer_t {
    pub tag: answer,
    pub c2rust_unnamed: C2RustUnnamed_12,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_12 {
    pub size: C2RustUnnamed_17,
    pub mtim: C2RustUnnamed_16,
    pub read: C2RustUnnamed_15,
    pub open: C2RustUnnamed_14,
    pub gpic: C2RustUnnamed_13,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_13 {
    pub bounds: [::core::ffi::c_float; 4],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_14 {
    pub path_len: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_15 {
    pub size: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_16 {
    pub mtime: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_17 {
    pub size: uint32_t,
}
pub type ask = ::core::ffi::c_uint;
pub const C_FLSH: ask = 1213418566;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ask_t {
    pub tag: ask,
    pub c2rust_unnamed: C2RustUnnamed_18,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_18 {
    pub term: C2RustUnnamed_21,
    pub fenc: C2RustUnnamed_20,
    pub flsh: C2RustUnnamed_19,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_19 {
    pub fid: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_20 {
    pub fid: ::core::ffi::c_int,
    pub pos: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_21 {
    pub pid: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cmsghdr {
    pub cmsg_len: size_t,
    pub cmsg_level: ::core::ffi::c_int,
    pub cmsg_type: ::core::ffi::c_int,
    pub __cmsg_data: [::core::ffi::c_uchar; 0],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct msghdr {
    pub msg_name: *mut ::core::ffi::c_void,
    pub msg_namelen: socklen_t,
    pub msg_iov: *mut iovec,
    pub msg_iovlen: size_t,
    pub msg_control: *mut ::core::ffi::c_void,
    pub msg_controllen: size_t,
    pub msg_flags: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: size_t,
}
pub type socklen_t = __socklen_t;
pub type nfds_t = ::core::ffi::c_ulong;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct pollfd {
    pub fd: ::core::ffi::c_int,
    pub events: ::core::ffi::c_short,
    pub revents: ::core::ffi::c_short,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const LOG: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const ECONNRESET: ::core::ffi::c_int = 104 as ::core::ffi::c_int;
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const POLLRDNORM: ::core::ffi::c_int = 0x40 as ::core::ffi::c_int;
pub const BUF_SIZE: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
unsafe extern "C" fn read_(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut data: *mut ::core::ffi::c_void,
    mut len: size_t,
) -> ssize_t {
    let mut msg_control: [::core::ffi::c_char; 24] = [
        0 as ::core::ffi::c_int as ::core::ffi::c_char,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
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
    let mut pid: int32_t = 0;
    let mut iov: iovec = iovec {
        iov_base: data,
        iov_len: len,
    };
    let mut msg: msghdr = msghdr {
        msg_name: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        msg_namelen: 0,
        msg_iov: &raw mut iov,
        msg_iovlen: 1 as size_t,
        msg_control: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        msg_controllen: ::core::mem::size_of::<[::core::ffi::c_char; 24]>() as size_t,
        msg_flags: 0,
    };
    msg.msg_control = &raw mut msg_control as *mut ::core::ffi::c_void;
    let mut recvd: ssize_t = 0;
    loop {
        recvd = recvmsg(fd, &raw mut msg, 0 as ::core::ffi::c_int);
        if !(recvd == -(1 as ::core::ffi::c_int) as ssize_t
            && *__errno_location() == EINTR)
        {
            break;
        }
    }
    if recvd == -(1 as ::core::ffi::c_int) as ssize_t {
        if *__errno_location() == ECONNRESET {
            fprintf(
                stderr,
                b"sprotocol:read_: ECONNRESET\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            fflush(stderr);
            return 0 as ssize_t;
        }
        perror(b"recvmsg\0" as *const u8 as *const ::core::ffi::c_char);
        fprintf(
            stderr,
            b"Aborting from driver/sprotocol.c:75\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    let mut cm: *mut cmsghdr = if msg.msg_controllen
        >= ::core::mem::size_of::<cmsghdr>() as usize
    {
        msg.msg_control as *mut cmsghdr
    } else {
        ::core::ptr::null_mut::<cmsghdr>()
    };
    if !cm.is_null() {
        let mut fds0: *mut ::core::ffi::c_int = &raw mut (*cm).__cmsg_data
            as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_int;
        let mut nfds: ::core::ffi::c_int = (*cm)
            .cmsg_len
            .wrapping_sub(
                ((::core::mem::size_of::<cmsghdr>() as size_t)
                    .wrapping_add(::core::mem::size_of::<size_t>() as size_t)
                    .wrapping_sub(1 as size_t)
                    & !(::core::mem::size_of::<size_t>() as usize)
                        .wrapping_sub(1 as usize))
                    .wrapping_add(0 as size_t),
            )
            .wrapping_div(::core::mem::size_of::<::core::ffi::c_int>() as size_t)
            as ::core::ffi::c_int;
        if nfds != 1 as ::core::ffi::c_int {
            abort();
        }
        if (*t).passed_fd != -(1 as ::core::ffi::c_int) {
            abort();
        }
        (*t).passed_fd = *fds0.offset(0 as ::core::ffi::c_int as isize);
    }
    return recvd;
}
unsafe extern "C" fn read_all(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut buf: *mut ::core::ffi::c_char,
    mut size: ::core::ffi::c_int,
) -> bool {
    while size > 0 as ::core::ffi::c_int {
        let mut n: ::core::ffi::c_int = read_(
            t,
            fd,
            buf as *mut ::core::ffi::c_void,
            size as size_t,
        ) as ::core::ffi::c_int;
        if n == 0 as ::core::ffi::c_int {
            return 0 as ::core::ffi::c_int != 0;
        }
        buf = buf.offset(n as isize);
        size -= n;
    }
    return 1 as ::core::ffi::c_int != 0;
}
unsafe extern "C" fn write_all(
    mut fd: ::core::ffi::c_int,
    mut buf: *const ::core::ffi::c_char,
    mut size: ::core::ffi::c_int,
) {
    while size > 0 as ::core::ffi::c_int {
        let mut n: ::core::ffi::c_int = write(
            fd,
            buf as *const ::core::ffi::c_void,
            size as size_t,
        ) as ::core::ffi::c_int;
        if n == -(1 as ::core::ffi::c_int) {
            if *__errno_location() == EINTR {
                continue;
            }
            perror(
                b"sprotocol.c write_all\0" as *const u8 as *const ::core::ffi::c_char,
            );
            print_backtrace();
            if *__errno_location() == ECONNRESET {
                return;
            }
        }
        if n <= 0 as ::core::ffi::c_int {
            fprintf(
                stderr,
                b"Aborting from driver/sprotocol.c:148\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            print_backtrace();
            abort();
        }
        buf = buf.offset(n as isize);
        size -= n;
    }
}
unsafe extern "C" fn cflush(mut c: *mut channel_t, mut fd: ::core::ffi::c_int) {
    let mut pos: ::core::ffi::c_int = (*c).output.pos;
    if pos == 0 as ::core::ffi::c_int {
        return;
    }
    write_all(fd, &raw mut (*c).output.buffer as *mut ::core::ffi::c_char, pos);
    (*c).output.pos = 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn refill_at_least(
    mut c: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut at_least: ::core::ffi::c_int,
) -> bool {
    let mut avail: ::core::ffi::c_int = (*c).input.len - (*c).input.pos;
    if avail >= at_least {
        return 1 as ::core::ffi::c_int != 0;
    }
    memmove(
        &raw mut (*c).input.buffer as *mut ::core::ffi::c_char
            as *mut ::core::ffi::c_void,
        (&raw mut (*c).input.buffer as *mut ::core::ffi::c_char)
            .offset((*c).input.pos as isize) as *const ::core::ffi::c_void,
        avail as size_t,
    );
    (*c).input.pos = 0 as ::core::ffi::c_int;
    while avail < at_least {
        let mut n: ::core::ffi::c_int = read_(
            c,
            fd,
            (&raw mut (*c).input.buffer as *mut ::core::ffi::c_char)
                .offset(avail as isize) as *mut ::core::ffi::c_void,
            (BUF_SIZE - avail) as size_t,
        ) as ::core::ffi::c_int;
        if n == 0 as ::core::ffi::c_int {
            (*c).input.len = avail;
            return 0 as ::core::ffi::c_int != 0;
        }
        avail += n;
    }
    (*c).input.len = avail;
    return 1 as ::core::ffi::c_int != 0;
}
pub const HND_SERVER: [::core::ffi::c_char; 13] = unsafe {
    ::core::mem::transmute::<[u8; 13], [::core::ffi::c_char; 13]>(*b"TEXPRESSOS01\0")
};
pub const HND_CLIENT: [::core::ffi::c_char; 13] = unsafe {
    ::core::mem::transmute::<[u8; 13], [::core::ffi::c_char; 13]>(*b"TEXPRESSOC01\0")
};
#[no_mangle]
pub unsafe extern "C" fn channel_handshake(
    mut c: *mut channel_t,
    mut fd: ::core::ffi::c_int,
) -> bool {
    let mut answer: [::core::ffi::c_char; 12] = [0; 12];
    write_all(
        fd,
        HND_SERVER.as_ptr(),
        (::core::mem::size_of::<[::core::ffi::c_char; 13]>() as usize)
            .wrapping_sub(1 as usize) as ::core::ffi::c_int,
    );
    if !read_all(
        c,
        fd,
        &raw mut answer as *mut ::core::ffi::c_char,
        (::core::mem::size_of::<[::core::ffi::c_char; 13]>() as usize)
            .wrapping_sub(1 as usize) as ::core::ffi::c_int,
    ) {
        return 0 as ::core::ffi::c_int != 0;
    }
    (*c).input.pos = 0 as ::core::ffi::c_int;
    (*c).input.len = (*c).input.pos;
    (*c).output.pos = 0 as ::core::ffi::c_int;
    return strncmp(
        HND_CLIENT.as_ptr(),
        &raw mut answer as *mut ::core::ffi::c_char,
        (::core::mem::size_of::<[::core::ffi::c_char; 13]>() as size_t)
            .wrapping_sub(1 as size_t),
    ) == 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn query_to_string(mut q: query) -> *const ::core::ffi::c_char {
    match q as ::core::ffi::c_uint {
        1146245199 => return b"OPRD\0" as *const u8 as *const ::core::ffi::c_char,
        1381453903 => return b"OPWR\0" as *const u8 as *const ::core::ffi::c_char,
        1145128274 => return b"READ\0" as *const u8 as *const ::core::ffi::c_char,
        1145983041 => return b"APND\0" as *const u8 as *const ::core::ffi::c_char,
        1397705795 => return b"CLOS\0" as *const u8 as *const ::core::ffi::c_char,
        1163544915 => return b"SIZE\0" as *const u8 as *const ::core::ffi::c_char,
        1313162579 => return b"SEEN\0" as *const u8 as *const ::core::ffi::c_char,
        1128878151 => return b"GPIC\0" as *const u8 as *const ::core::ffi::c_char,
        1128878163 => return b"SPIC\0" as *const u8 as *const ::core::ffi::c_char,
        1145849923 => return b"CHLD\0" as *const u8 as *const ::core::ffi::c_char,
        1296651341 => return b"MTIM\0" as *const u8 as *const ::core::ffi::c_char,
        1112821318 => return b"FNTB\0" as *const u8 as *const ::core::ffi::c_char,
        _ => {}
    }
    return b"?\0" as *const u8 as *const ::core::ffi::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn answer_to_string(mut q: answer) -> *const ::core::ffi::c_char {
    match q as ::core::ffi::c_uint {
        1162760004 => return b"DONE\0" as *const u8 as *const ::core::ffi::c_char,
        1397965136 => return b"PASS\0" as *const u8 as *const ::core::ffi::c_char,
        1163544915 => return b"SIZE\0" as *const u8 as *const ::core::ffi::c_char,
        1296651341 => return b"MTIM\0" as *const u8 as *const ::core::ffi::c_char,
        1145128274 => return b"READ\0" as *const u8 as *const ::core::ffi::c_char,
        1263685446 => return b"FORK\0" as *const u8 as *const ::core::ffi::c_char,
        1313165391 => return b"OPEN\0" as *const u8 as *const ::core::ffi::c_char,
        1128878151 => return b"GPIC\0" as *const u8 as *const ::core::ffi::c_char,
        _ => {}
    }
    panic!("Reached end of non-void function without returning");
}
#[no_mangle]
pub unsafe extern "C" fn ask_to_string(mut q: ask) -> *const ::core::ffi::c_char {
    match q as ::core::ffi::c_uint {
        1213418566 => return b"FLSH\0" as *const u8 as *const ::core::ffi::c_char,
        _ => {}
    }
    panic!("Reached end of non-void function without returning");
}
#[no_mangle]
pub unsafe extern "C" fn channel_new() -> *mut channel_t {
    let mut c: *mut channel_t = calloc(
        ::core::mem::size_of::<channel_t>() as size_t,
        1 as size_t,
    ) as *mut channel_t;
    if c.is_null() {
        fprintf(
            stderr,
            b"Aborting from driver/sprotocol.c:253\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    (*c).buf = malloc(256 as size_t) as *mut ::core::ffi::c_char;
    if (*c).buf.is_null() {
        fprintf(
            stderr,
            b"Aborting from driver/sprotocol.c:255\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    (*c).buf_size = 256 as ::core::ffi::c_int;
    (*c).passed_fd = -(1 as ::core::ffi::c_int);
    return c;
}
#[no_mangle]
pub unsafe extern "C" fn channel_free(mut c: *mut channel_t) {
    free((*c).buf as *mut ::core::ffi::c_void);
    free(c as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn resize_buf(mut t: *mut channel_t) {
    let mut old_size: ::core::ffi::c_int = (*t).buf_size;
    let mut new_size: ::core::ffi::c_int = old_size * 2 as ::core::ffi::c_int;
    let mut buf: *mut ::core::ffi::c_char = malloc(new_size as size_t)
        as *mut ::core::ffi::c_char;
    if buf.is_null() {
        fprintf(
            stderr,
            b"Aborting from driver/sprotocol.c:272\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        print_backtrace();
        abort();
    }
    memcpy(
        buf as *mut ::core::ffi::c_void,
        (*t).buf as *const ::core::ffi::c_void,
        old_size as size_t,
    );
    free((*t).buf as *mut ::core::ffi::c_void);
    (*t).buf = buf;
    (*t).buf_size = new_size;
}
unsafe extern "C" fn cgetc(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if (*t).input.pos == (*t).input.len {
        if !refill_at_least(t, fd, 1 as ::core::ffi::c_int) {
            return 0 as ::core::ffi::c_int;
        }
    }
    let fresh0 = (*t).input.pos;
    (*t).input.pos = (*t).input.pos + 1;
    return (*t).input.buffer[fresh0 as usize] as ::core::ffi::c_int;
}
unsafe extern "C" fn read_zstr(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut pos: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut c: ::core::ffi::c_int = 0;
    let mut p0: ::core::ffi::c_int = *pos;
    loop {
        if *pos == (*t).buf_size {
            resize_buf(t);
        }
        c = cgetc(t, fd);
        *(*t).buf.offset(*pos as isize) = c as ::core::ffi::c_char;
        *pos += 1 as ::core::ffi::c_int;
        if !(c != 0 as ::core::ffi::c_int) {
            break;
        }
    }
    return p0;
}
unsafe extern "C" fn read_bytes(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut pos: ::core::ffi::c_int,
    mut size: ::core::ffi::c_int,
) -> bool {
    while (*t).buf_size < pos + size {
        resize_buf(t);
    }
    let mut ipos: ::core::ffi::c_int = (*t).input.pos;
    let mut ilen: ::core::ffi::c_int = (*t).input.len;
    if ipos + size <= ilen {
        memcpy(
            (*t).buf.offset(pos as isize) as *mut ::core::ffi::c_char
                as *mut ::core::ffi::c_void,
            (&raw mut (*t).input.buffer as *mut ::core::ffi::c_char)
                .offset(ipos as isize) as *const ::core::ffi::c_void,
            size as size_t,
        );
        (*t).input.pos += size;
        return 1 as ::core::ffi::c_int != 0;
    }
    let mut isize: ::core::ffi::c_int = ilen - ipos;
    memcpy(
        (*t).buf.offset(pos as isize) as *mut ::core::ffi::c_char
            as *mut ::core::ffi::c_void,
        (&raw mut (*t).input.buffer as *mut ::core::ffi::c_char).offset(ipos as isize)
            as *const ::core::ffi::c_void,
        isize as size_t,
    );
    pos += isize;
    size -= isize;
    (*t).input.len = 0 as ::core::ffi::c_int;
    (*t).input.pos = (*t).input.len;
    return read_all(
        t,
        fd,
        (*t).buf.offset(pos as isize) as *mut ::core::ffi::c_char,
        size,
    );
}
unsafe extern "C" fn write_bytes(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut buf: *mut ::core::ffi::c_void,
    mut size: ::core::ffi::c_int,
) {
    if (*t).output.pos + size <= BUF_SIZE {
        memcpy(
            (&raw mut (*t).output.buffer as *mut ::core::ffi::c_char)
                .offset((*t).output.pos as isize) as *mut ::core::ffi::c_void,
            buf,
            size as size_t,
        );
        (*t).output.pos += size;
        return;
    }
    cflush(t, fd);
    if size > BUF_SIZE {
        write_all(fd, buf as *const ::core::ffi::c_char, size);
    } else {
        memcpy(
            &raw mut (*t).output.buffer as *mut ::core::ffi::c_char
                as *mut ::core::ffi::c_void,
            buf,
            size as size_t,
        );
        (*t).output.pos = size;
    };
}
unsafe extern "C" fn try_read_u32(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut tag: *mut uint32_t,
) -> bool {
    if !refill_at_least(t, fd, 4 as ::core::ffi::c_int) {
        return 0 as ::core::ffi::c_int != 0;
    }
    memcpy(
        tag as *mut ::core::ffi::c_void,
        (&raw mut (*t).input.buffer as *mut ::core::ffi::c_char)
            .offset((*t).input.pos as isize) as *const ::core::ffi::c_void,
        4 as size_t,
    );
    (*t).input.pos += 4 as ::core::ffi::c_int;
    return 1 as ::core::ffi::c_int != 0;
}
unsafe extern "C" fn read_u32(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
) -> uint32_t {
    let mut avail: ::core::ffi::c_int = (*t).input.len - (*t).input.pos;
    if !refill_at_least(t, fd, 4 as ::core::ffi::c_int) {
        return 0 as uint32_t;
    }
    let mut tag: uint32_t = 0;
    memcpy(
        &raw mut tag as *mut ::core::ffi::c_void,
        (&raw mut (*t).input.buffer as *mut ::core::ffi::c_char)
            .offset((*t).input.pos as isize) as *const ::core::ffi::c_void,
        4 as size_t,
    );
    (*t).input.pos += 4 as ::core::ffi::c_int;
    return tag;
}
unsafe extern "C" fn write_u32(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut u: uint32_t,
) {
    write_bytes(t, fd, &raw mut u as *mut ::core::ffi::c_void, 4 as ::core::ffi::c_int);
}
unsafe extern "C" fn read_f32(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
) -> ::core::ffi::c_float {
    if !refill_at_least(t, fd, 4 as ::core::ffi::c_int) {
        return 0 as ::core::ffi::c_int as ::core::ffi::c_float;
    }
    let mut f: ::core::ffi::c_float = 0.;
    memcpy(
        &raw mut f as *mut ::core::ffi::c_void,
        (&raw mut (*t).input.buffer as *mut ::core::ffi::c_char)
            .offset((*t).input.pos as isize) as *const ::core::ffi::c_void,
        4 as size_t,
    );
    (*t).input.pos += 4 as ::core::ffi::c_int;
    return f;
}
unsafe extern "C" fn write_f32(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut f: ::core::ffi::c_float,
) {
    write_bytes(t, fd, &raw mut f as *mut ::core::ffi::c_void, 4 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn log_query(mut f: *mut FILE, mut r: *mut query_t) {
    fprintf(f, b"%04dms: \0" as *const u8 as *const ::core::ffi::c_char, (*r).time);
    match (*r).tag as ::core::ffi::c_uint {
        1146245199 => {
            fprintf(
                f,
                b"OPRD(%d, \"%s\")\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.open.fid,
                (*r).c2rust_unnamed.open.path,
            );
            return;
        }
        1381453903 => {
            fprintf(
                f,
                b"OPWR(%d, \"%s\")\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.open.fid,
                (*r).c2rust_unnamed.open.path,
            );
            return;
        }
        1145128274 => {
            fprintf(
                f,
                b"READ(%d, %d, %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.read.fid,
                (*r).c2rust_unnamed.read.pos,
                (*r).c2rust_unnamed.read.size,
            );
            return;
        }
        1145983041 => {
            fprintf(
                f,
                b"APND(%d, %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.apnd.fid,
                (*r).c2rust_unnamed.apnd.size,
            );
            return;
        }
        1397705795 => {
            fprintf(
                f,
                b"CLOS(%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.clos.fid,
            );
            return;
        }
        1163544915 => {
            fprintf(
                f,
                b"SIZE(%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.size.fid,
            );
            return;
        }
        1296651341 => {
            fprintf(
                f,
                b"MTIM(%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.mtim.fid,
            );
            return;
        }
        1313162579 => {
            fprintf(
                f,
                b"SEEN(%d, %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.seen.fid,
                (*r).c2rust_unnamed.seen.pos,
            );
            return;
        }
        1128878151 => {
            fprintf(
                f,
                b"GPIC(\"%s\",%d,%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.gpic.path,
                (*r).c2rust_unnamed.gpic.type_0,
                (*r).c2rust_unnamed.gpic.page,
            );
            return;
        }
        1128878163 => {
            fprintf(
                f,
                b"SPIC(\"%s\", %d, %d, %.02f, %.02f, %.02f, %.02f)\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.spic.path,
                (*r).c2rust_unnamed.spic.cache.type_0,
                (*r).c2rust_unnamed.spic.cache.page,
                (*r).c2rust_unnamed.spic.cache.bounds[0 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double,
                (*r).c2rust_unnamed.spic.cache.bounds[1 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double,
                (*r).c2rust_unnamed.spic.cache.bounds[2 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double,
                (*r).c2rust_unnamed.spic.cache.bounds[3 as ::core::ffi::c_int as usize]
                    as ::core::ffi::c_double,
            );
            return;
        }
        1145849923 => {
            fprintf(
                f,
                b"CHLD(pid:%d, fd:%d)\n\0" as *const u8 as *const ::core::ffi::c_char,
                (*r).c2rust_unnamed.chld.pid,
                (*r).c2rust_unnamed.chld.fd,
            );
            return;
        }
        1112821318 => {
            fprintf(f, b"FNTB()\n\0" as *const u8 as *const ::core::ffi::c_char);
            return;
        }
        _ => {}
    }
    fprintf(
        stderr,
        b"Aborting from driver/sprotocol.c:432\n\0" as *const u8
            as *const ::core::ffi::c_char,
    );
    print_backtrace();
    abort();
}
#[no_mangle]
pub unsafe extern "C" fn channel_has_pending_query(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut timeout: ::core::ffi::c_int,
) -> bool {
    if (*t).input.pos != (*t).input.len {
        return 1 as ::core::ffi::c_int != 0;
    }
    let mut pfd: pollfd = pollfd {
        fd: 0,
        events: 0,
        revents: 0,
    };
    let mut n: ::core::ffi::c_int = 0;
    loop {
        pfd.fd = fd;
        pfd.events = POLLRDNORM as ::core::ffi::c_short;
        pfd.revents = 0 as ::core::ffi::c_short;
        n = poll(&raw mut pfd, 1 as nfds_t, timeout);
        if !(n == -(1 as ::core::ffi::c_int) && *__errno_location() == EINTR) {
            break;
        }
    }
    if n == -(1 as ::core::ffi::c_int) {
        perror(b"driver/sprotocol.c:452\0" as *const u8 as *const ::core::ffi::c_char);
        myabort_(
            b"driver/sprotocol.c\0" as *const u8 as *const ::core::ffi::c_char,
            452 as ::core::ffi::c_int,
            b"\0" as *const u8 as *const ::core::ffi::c_char,
            42424242 as uint32_t,
        );
    }
    if n == 0 as ::core::ffi::c_int {
        return 0 as ::core::ffi::c_int != 0;
    }
    return 1 as ::core::ffi::c_int != 0;
}
#[no_mangle]
pub unsafe extern "C" fn channel_peek_query(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
) -> query {
    let mut result: uint32_t = read_u32(t, fd);
    if result == 0 as uint32_t {
        abort();
    }
    (*t).input.pos -= 4 as ::core::ffi::c_int;
    return result as query;
}
#[no_mangle]
pub unsafe extern "C" fn channel_read_query(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut r: *mut query_t,
) -> bool {
    let mut tag: uint32_t = 0;
    if !try_read_u32(t, fd, &raw mut tag) {
        return 0 as ::core::ffi::c_int != 0;
    }
    (*r).tag = tag as query;
    (*r).time = read_u32(t, fd) as ::core::ffi::c_int;
    let mut pos: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    match tag {
        1146245199 | 1381453903 => {
            (*r).c2rust_unnamed.open.fid = read_u32(t, fd) as file_id;
            let mut pos_path: ::core::ffi::c_int = read_zstr(t, fd, &raw mut pos);
            (*r).c2rust_unnamed.open.path = (*t).buf.offset(pos_path as isize)
                as *mut ::core::ffi::c_char;
            (*r).c2rust_unnamed.open.kind = read_u32(t, fd) as txp_file_kind;
        }
        1145128274 => {
            (*r).c2rust_unnamed.read.fid = read_u32(t, fd) as file_id;
            (*r).c2rust_unnamed.read.pos = read_u32(t, fd) as ::core::ffi::c_int;
            (*r).c2rust_unnamed.read.size = read_u32(t, fd) as ::core::ffi::c_int;
        }
        1145983041 => {
            (*r).c2rust_unnamed.apnd.fid = read_u32(t, fd) as file_id;
            (*r).c2rust_unnamed.apnd.size = read_u32(t, fd) as ::core::ffi::c_int;
            if !read_bytes(
                t,
                fd,
                0 as ::core::ffi::c_int,
                (*r).c2rust_unnamed.apnd.size,
            ) {
                return 0 as ::core::ffi::c_int != 0;
            }
            (*r).c2rust_unnamed.apnd.buf = (*t).buf;
        }
        1397705795 => {
            (*r).c2rust_unnamed.clos.fid = read_u32(t, fd) as file_id;
        }
        1163544915 => {
            (*r).c2rust_unnamed.size.fid = read_u32(t, fd) as file_id;
        }
        1296651341 => {
            (*r).c2rust_unnamed.mtim.fid = read_u32(t, fd) as file_id;
        }
        1313162579 => {
            (*r).c2rust_unnamed.seen.fid = read_u32(t, fd) as file_id;
            (*r).c2rust_unnamed.seen.pos = read_u32(t, fd) as ::core::ffi::c_int;
        }
        1128878151 => {
            let mut pos_path_0: ::core::ffi::c_int = read_zstr(t, fd, &raw mut pos);
            (*r).c2rust_unnamed.gpic.path = (*t).buf.offset(pos_path_0 as isize)
                as *mut ::core::ffi::c_char;
            (*r).c2rust_unnamed.gpic.type_0 = read_u32(t, fd) as ::core::ffi::c_int;
            (*r).c2rust_unnamed.gpic.page = read_u32(t, fd) as ::core::ffi::c_int;
        }
        1128878163 => {
            let mut pos_path_1: ::core::ffi::c_int = read_zstr(t, fd, &raw mut pos);
            (*r).c2rust_unnamed.spic.path = (*t).buf.offset(pos_path_1 as isize)
                as *mut ::core::ffi::c_char;
            (*r).c2rust_unnamed.spic.cache.type_0 = read_u32(t, fd)
                as ::core::ffi::c_int;
            (*r).c2rust_unnamed.spic.cache.page = read_u32(t, fd) as ::core::ffi::c_int;
            (*r).c2rust_unnamed.spic.cache.bounds[0 as ::core::ffi::c_int as usize] = read_f32(
                t,
                fd,
            );
            (*r).c2rust_unnamed.spic.cache.bounds[1 as ::core::ffi::c_int as usize] = read_f32(
                t,
                fd,
            );
            (*r).c2rust_unnamed.spic.cache.bounds[2 as ::core::ffi::c_int as usize] = read_f32(
                t,
                fd,
            );
            (*r).c2rust_unnamed.spic.cache.bounds[3 as ::core::ffi::c_int as usize] = read_f32(
                t,
                fd,
            );
        }
        1145849923 => {
            (*r).c2rust_unnamed.chld.pid = read_u32(t, fd) as ::core::ffi::c_int;
            if (*t).passed_fd == -(1 as ::core::ffi::c_int) {
                abort();
            }
            (*r).c2rust_unnamed.chld.fd = (*t).passed_fd;
            (*t).passed_fd = -(1 as ::core::ffi::c_int);
        }
        1112821318 => {}
        _ => {
            fprintf(
                stderr,
                b"unexpected tag: %c%c%c%c\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                tag & 0xff as uint32_t,
                tag >> 8 as ::core::ffi::c_int & 0xff as uint32_t,
                tag >> 16 as ::core::ffi::c_int & 0xff as uint32_t,
                tag >> 24 as ::core::ffi::c_int & 0xff as uint32_t,
            );
            fprintf(
                stderr,
                b"Aborting from driver/sprotocol.c:560\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            print_backtrace();
            abort();
        }
    }
    return 1 as ::core::ffi::c_int != 0;
}
#[no_mangle]
pub unsafe extern "C" fn channel_write_ask(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut a: *mut ask_t,
) {
    write_u32(t, fd, (*a).tag as uint32_t);
    match (*a).tag as ::core::ffi::c_uint {
        1213418566 => {}
        _ => {
            fprintf(
                stderr,
                b"Aborting from driver/sprotocol.c:577\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            print_backtrace();
            abort();
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn channel_write_answer(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
    mut a: *mut answer_t,
) {
    write_u32(t, fd, (*a).tag as uint32_t);
    match (*a).tag as ::core::ffi::c_uint {
        1162760004 | 1397965136 | 1263685446 => {}
        1145128274 => {
            write_u32(t, fd, (*a).c2rust_unnamed.read.size as uint32_t);
            write_bytes(
                t,
                fd,
                (*t).buf as *mut ::core::ffi::c_void,
                (*a).c2rust_unnamed.read.size,
            );
        }
        1163544915 => {
            write_u32(t, fd, (*a).c2rust_unnamed.size.size);
        }
        1296651341 => {
            write_u32(t, fd, (*a).c2rust_unnamed.mtim.mtime);
        }
        1313165391 => {
            write_u32(t, fd, (*a).c2rust_unnamed.open.path_len as uint32_t);
            write_bytes(
                t,
                fd,
                (*t).buf as *mut ::core::ffi::c_void,
                (*a).c2rust_unnamed.open.path_len,
            );
        }
        1128878151 => {
            write_f32(
                t,
                fd,
                (*a).c2rust_unnamed.gpic.bounds[0 as ::core::ffi::c_int as usize],
            );
            write_f32(
                t,
                fd,
                (*a).c2rust_unnamed.gpic.bounds[1 as ::core::ffi::c_int as usize],
            );
            write_f32(
                t,
                fd,
                (*a).c2rust_unnamed.gpic.bounds[2 as ::core::ffi::c_int as usize],
            );
            write_f32(
                t,
                fd,
                (*a).c2rust_unnamed.gpic.bounds[3 as ::core::ffi::c_int as usize],
            );
        }
        _ => {
            fprintf(
                stderr,
                b"Aborting from driver/sprotocol.c:620\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            print_backtrace();
            abort();
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn channel_flush(
    mut t: *mut channel_t,
    mut fd: ::core::ffi::c_int,
) {
    cflush(t, fd);
}
#[no_mangle]
pub unsafe extern "C" fn channel_reset(mut t: *mut channel_t) {
    (*t).input.len = 0 as ::core::ffi::c_int;
    (*t).input.pos = (*t).input.len;
    (*t).output.pos = 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn channel_get_buffer(
    mut t: *mut channel_t,
    mut n: size_t,
) -> *mut ::core::ffi::c_void {
    while n > (*t).buf_size as size_t {
        resize_buf(t);
    }
    return (*t).buf as *mut ::core::ffi::c_void;
}
