// Translated from xetex/main/texpresso_protocol.c with C2Rust 0.22.1.
extern "C" {
    pub type __sFILEX;
    static mut __stderrp: *mut FILE;
    fn fflush(_: *mut FILE) -> ::core::ffi::c_int;
    fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fread(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
        __nitems: size_t,
        __stream: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    fn fwrite(
        __ptr: *const ::core::ffi::c_void,
        __size: size_t,
        __nitems: size_t,
        __stream: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    fn perror(_: *const ::core::ffi::c_char);
    fn fileno(_: *mut FILE) -> ::core::ffi::c_int;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn exit(_: ::core::ffi::c_int) -> !;
    fn texpresso_fork_with_channel(fd: ::core::ffi::c_int, time: uint32_t) -> pid_t;
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
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    static mut xetex_tokens: ::core::ffi::c_int;
}
pub type __int32_t = i32;
pub type __int64_t = i64;
pub type __darwin_size_t = usize;
pub type __darwin_off_t = __int64_t;
pub type __darwin_pid_t = __int32_t;
pub type int32_t = i32;
pub type size_t = __darwin_size_t;
pub type fpos_t = __darwin_off_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sbuf {
    pub _base: *mut ::core::ffi::c_uchar,
    pub _size: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sFILE {
    pub _p: *mut ::core::ffi::c_uchar,
    pub _r: ::core::ffi::c_int,
    pub _w: ::core::ffi::c_int,
    pub _flags: ::core::ffi::c_short,
    pub _file: ::core::ffi::c_short,
    pub _bf: __sbuf,
    pub _lbfsize: ::core::ffi::c_int,
    pub _cookie: *mut ::core::ffi::c_void,
    pub _close: Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ::core::ffi::c_int>,
    pub _read: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut ::core::ffi::c_char,
            ::core::ffi::c_int,
        ) -> ::core::ffi::c_int,
    >,
    pub _seek: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, fpos_t, ::core::ffi::c_int) -> fpos_t,
    >,
    pub _write: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *const ::core::ffi::c_char,
            ::core::ffi::c_int,
        ) -> ::core::ffi::c_int,
    >,
    pub _ub: __sbuf,
    pub _extra: *mut __sFILEX,
    pub _ur: ::core::ffi::c_int,
    pub _ubuf: [::core::ffi::c_uchar; 3],
    pub _nbuf: [::core::ffi::c_uchar; 1],
    pub _lb: __sbuf,
    pub _blksize: ::core::ffi::c_int,
    pub _offset: fpos_t,
}
pub type FILE = __sFILE;
pub type pid_t = __darwin_pid_t;
pub type uint8_t = u8;
pub type uint32_t = u32;
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
pub type txp_open_mode = ::core::ffi::c_uint;
pub const TXP_WRITE: txp_open_mode = 1;
pub const TXP_READ: txp_open_mode = 0;
pub type txp_file_id = int32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct txp_client {
    pub file: *mut FILE,
    pub generation: uint32_t,
    pub seen_pos: uint32_t,
    pub seen_file: txp_file_id,
    pub append_file: txp_file_id,
    pub append_len: ::core::ffi::c_int,
    pub append_buf: [::core::ffi::c_char; 4096],
    pub stdout_len: ::core::ffi::c_int,
    pub stdout_buf: [::core::ffi::c_char; 4096],
}
pub type tag = ::core::ffi::c_uint;
pub const T_FNTB: tag = 1112821318;
pub const T_MTIM: tag = 1296651341;
pub const T_APND: tag = 1145983041;
pub const T_SPIC: tag = 1128878163;
pub const T_SIZE: tag = 1163544915;
pub const T_SEEN: tag = 1313162579;
pub const T_READ: tag = 1145128274;
pub const T_PASS: tag = 1397965136;
pub const T_OPEN: tag = 1313165391;
pub const T_OPWR: tag = 1381453903;
pub const T_OPRD: tag = 1146245199;
pub const T_GPIC: tag = 1128878151;
pub const T_FORK: tag = 1263685446;
pub const T_FLSH: tag = 1213418566;
pub const T_DONE: tag = 1162760004;
pub const T_CLOS: tag = 1397705795;
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed {
    pub f: ::core::ffi::c_float,
    pub u: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union C2RustUnnamed_0 {
    pub f: ::core::ffi::c_float,
    pub u: uint32_t,
}
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
unsafe extern "C" fn ppanic(mut txt: *const ::core::ffi::c_char) {
    perror(txt);
    exit(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn write_or_panic(
    mut file: *mut FILE,
    mut data: *const ::core::ffi::c_void,
    mut len: ::core::ffi::c_int,
) {
    if fwrite(data, 1 as size_t, len as size_t, file) != len as ::core::ffi::c_ulong {
        ppanic(b"Cannot write to server\0" as *const u8 as *const ::core::ffi::c_char);
    }
}
unsafe extern "C" fn read_exact(
    mut file: *mut FILE,
    mut data: *mut ::core::ffi::c_void,
    mut len: ::core::ffi::c_int,
) {
    if fread(data, 1 as size_t, len as size_t, file) != len as ::core::ffi::c_ulong {
        ppanic(b"Cannot read from server\0" as *const u8 as *const ::core::ffi::c_char);
    }
}
#[no_mangle]
pub unsafe extern "C" fn txp_connect(mut file: *mut FILE) -> *mut txp_client {
    write_or_panic(
        file,
        b"TEXPRESSOC01\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
        12 as ::core::ffi::c_int,
    );
    if fflush(file) != 0 as ::core::ffi::c_int {
        ppanic(b"Cannot flush\0" as *const u8 as *const ::core::ffi::c_char);
    }
    let mut buf: [::core::ffi::c_char; 12] = [0; 12];
    read_exact(
        file,
        &raw mut buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        12 as ::core::ffi::c_int,
    );
    if memcmp(
        &raw mut buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
        b"TEXPRESSOS01\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
        12 as size_t,
    ) != 0 as ::core::ffi::c_int
    {
        ppanic(b"Invalid handshake\0" as *const u8 as *const ::core::ffi::c_char);
    }
    fprintf(
        __stderrp,
        b"texpresso: handshake success\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut client: *mut txp_client =
        calloc(::core::mem::size_of::<txp_client>() as size_t, 1 as size_t) as *mut txp_client;
    if client.is_null() {
        ppanic(b"Cannot allocate client\0" as *const u8 as *const ::core::ffi::c_char);
    }
    (*client).file = file;
    (*client).generation = 0 as uint32_t;
    (*client).seen_pos = 0 as uint32_t;
    (*client).append_len = 0 as ::core::ffi::c_int;
    (*client).stdout_len = 0 as ::core::ffi::c_int;
    return client;
}
unsafe extern "C" fn txp_io_send_str(mut io: *mut txp_client, mut str: *const ::core::ffi::c_char) {
    write_or_panic(
        (*io).file,
        str as *const ::core::ffi::c_void,
        strlen(str).wrapping_add(1 as size_t) as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn txp_io_send_u32(mut io: *mut txp_client, mut i: uint32_t) {
    let mut buf: [uint8_t; 4] = [
        (i & 0xff as uint32_t) as uint8_t,
        (i >> 8 as ::core::ffi::c_int & 0xff as uint32_t) as uint8_t,
        (i >> 16 as ::core::ffi::c_int & 0xff as uint32_t) as uint8_t,
        (i >> 24 as ::core::ffi::c_int & 0xff as uint32_t) as uint8_t,
    ];
    write_or_panic(
        (*io).file,
        &raw mut buf as *mut uint8_t as *const ::core::ffi::c_void,
        4 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn txp_io_recv_u32(mut io: *mut txp_client) -> uint32_t {
    let mut buf: [uint8_t; 4] = [0; 4];
    read_exact(
        (*io).file,
        &raw mut buf as *mut uint8_t as *mut ::core::ffi::c_void,
        4 as ::core::ffi::c_int,
    );
    return (buf[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
        | (buf[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int) << 8 as ::core::ffi::c_int
        | (buf[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int) << 16 as ::core::ffi::c_int
        | (buf[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int) << 24 as ::core::ffi::c_int)
        as uint32_t;
}
unsafe extern "C" fn txp_io_send_f32(mut io: *mut txp_client, mut f: ::core::ffi::c_float) {
    let mut x: C2RustUnnamed_0 = C2RustUnnamed_0 { f: 0. };
    x.f = f;
    txp_io_send_u32(io, x.u);
}
unsafe extern "C" fn txp_io_recv_f32(mut io: *mut txp_client) -> ::core::ffi::c_float {
    let mut x: C2RustUnnamed = C2RustUnnamed { f: 0. };
    x.u = txp_io_recv_u32(io);
    return x.f;
}
unsafe extern "C" fn txp_io_send_tag_raw(mut io: *mut txp_client, mut t: tag) {
    txp_io_send_u32(io, t as uint32_t);
    let mut time: uint32_t = xetex_tokens as uint32_t;
    txp_io_send_u32(io, time);
}
unsafe extern "C" fn txp_io_flush(mut io: *mut txp_client) {
    if fflush((*io).file) != 0 as ::core::ffi::c_int {
        ppanic(b"Cannot flush\0" as *const u8 as *const ::core::ffi::c_char);
    }
}
unsafe extern "C" fn txp_io_recv_tag(mut io: *mut txp_client) -> tag {
    loop {
        let mut t: tag = txp_io_recv_u32(io) as tag;
        if t as ::core::ffi::c_uint != T_FLSH as ::core::ffi::c_int as ::core::ffi::c_uint {
            return t;
        }
        (*io).generation = (*io).generation.wrapping_add(1 as uint32_t);
    }
}
unsafe extern "C" fn panic_tag(mut t: tag) {
    fprintf(
        __stderrp,
        b"TeXpresso: unexpected tag %c%c%c%c\0" as *const u8 as *const ::core::ffi::c_char,
        t as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint,
        t as ::core::ffi::c_uint >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint,
        t as ::core::ffi::c_uint >> 16 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint,
        t as ::core::ffi::c_uint >> 24 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint,
    );
    exit(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn txp_io_check_done(mut io: *mut txp_client) {
    let mut t: tag = txp_io_recv_tag(io);
    if t as ::core::ffi::c_uint != T_DONE as ::core::ffi::c_int as ::core::ffi::c_uint {
        panic_tag(t);
    }
}
unsafe extern "C" fn txp_io_seen(
    mut io: *mut txp_client,
    mut file: txp_file_id,
    mut pos: uint32_t,
) {
    txp_io_send_tag_raw(io, T_SEEN);
    txp_io_send_u32(io, file as uint32_t);
    txp_io_send_u32(io, pos);
}
unsafe extern "C" fn txp_flush_pending(mut io: *mut txp_client) {
    if (*io).seen_pos != 0 as uint32_t {
        txp_io_seen(io, (*io).seen_file, (*io).seen_pos);
        (*io).seen_pos = 0 as uint32_t;
    }
    if (*io).stdout_len != 0 as ::core::ffi::c_int {
        txp_io_send_tag_raw(io, T_APND);
        txp_io_send_u32(io, -(1 as ::core::ffi::c_int) as uint32_t);
        txp_io_send_u32(io, (*io).stdout_len as uint32_t);
        write_or_panic(
            (*io).file,
            &raw mut (*io).stdout_buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            (*io).stdout_len,
        );
        txp_io_check_done(io);
        (*io).stdout_len = 0 as ::core::ffi::c_int;
    }
    if (*io).append_len != 0 as ::core::ffi::c_int {
        txp_io_send_tag_raw(io, T_APND);
        txp_io_send_u32(io, (*io).append_file as uint32_t);
        txp_io_send_u32(io, (*io).append_len as uint32_t);
        write_or_panic(
            (*io).file,
            &raw mut (*io).append_buf as *mut ::core::ffi::c_char as *const ::core::ffi::c_void,
            (*io).append_len,
        );
        txp_io_check_done(io);
        (*io).append_len = 0 as ::core::ffi::c_int;
    }
}
#[no_mangle]
pub unsafe extern "C" fn txp_seen(
    mut io: *mut txp_client,
    mut file: txp_file_id,
    mut pos: uint32_t,
) {
    if (*io).seen_file != file {
        txp_flush_pending(io);
        (*io).seen_file = file;
    }
    if (*io).seen_pos < pos {
        (*io).seen_pos = pos;
    }
}
unsafe extern "C" fn txp_io_send_tag(mut io: *mut txp_client, mut t: tag) {
    txp_flush_pending(io);
    txp_io_send_tag_raw(io, t);
}
#[no_mangle]
pub unsafe extern "C" fn txp_open(
    mut io: *mut txp_client,
    mut file: txp_file_id,
    mut path: *const ::core::ffi::c_char,
    mut kind: txp_file_kind,
    mut mode: txp_open_mode,
) -> *mut ::core::ffi::c_char {
    txp_io_send_tag(
        io,
        (if mode as ::core::ffi::c_uint == TXP_READ as ::core::ffi::c_int as ::core::ffi::c_uint {
            T_OPRD as ::core::ffi::c_int
        } else {
            T_OPWR as ::core::ffi::c_int
        }) as tag,
    );
    txp_io_send_u32(io, file as uint32_t);
    txp_io_send_str(io, path);
    txp_io_send_u32(io, kind as uint32_t);
    let mut t: tag = txp_io_recv_tag(io);
    match t as ::core::ffi::c_uint {
        1397965136 => return ::core::ptr::null_mut::<::core::ffi::c_char>(),
        1313165391 => {
            let mut size: uint32_t = txp_io_recv_u32(io);
            let mut buf: *mut ::core::ffi::c_char =
                calloc(1 as size_t, size.wrapping_add(1 as uint32_t) as size_t)
                    as *mut ::core::ffi::c_char;
            if buf.is_null() {
                fprintf(
                    __stderrp,
                    b"Cannot allocate filename (length: %d)\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                    size,
                );
                exit(1 as ::core::ffi::c_int);
            }
            read_exact(
                (*io).file,
                buf as *mut ::core::ffi::c_void,
                size as ::core::ffi::c_int,
            );
            *buf.offset(size as isize) = 0 as ::core::ffi::c_char;
            return buf;
        }
        _ => {
            panic_tag(t);
        }
    }
    panic!("Reached end of non-void function without returning");
}
#[no_mangle]
pub unsafe extern "C" fn txp_read(
    mut io: *mut txp_client,
    mut file: txp_file_id,
    mut pos: uint32_t,
    mut buf: *mut ::core::ffi::c_void,
    mut len: size_t,
) -> size_t {
    loop {
        txp_io_send_tag(io, T_READ);
        txp_io_send_u32(io, file as uint32_t);
        txp_io_send_u32(io, pos);
        txp_io_send_u32(io, len as uint32_t);
        let mut t: tag = txp_io_recv_tag(io);
        match t as ::core::ffi::c_uint {
            1263685446 => {
                txp_fork(io);
            }
            1145128274 => {
                let mut size: uint32_t = txp_io_recv_u32(io);
                if size as size_t > len {
                    exit(1 as ::core::ffi::c_int);
                }
                read_exact((*io).file, buf, size as ::core::ffi::c_int);
                return size as size_t;
            }
            _ => {
                panic_tag(t);
            }
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn txp_append(
    mut io: *mut txp_client,
    mut file: txp_file_id,
    mut buf: *const ::core::ffi::c_void,
    mut len: size_t,
) {
    if len == 0 as size_t {
        return;
    }
    if (*io).append_file == file
        && ((*io).append_len as size_t).wrapping_add(len)
            <= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>() as usize
    {
        memcpy(
            (&raw mut (*io).append_buf as *mut ::core::ffi::c_char)
                .offset((*io).append_len as isize) as *mut ::core::ffi::c_void,
            buf,
            len,
        );
        (*io).append_len = ((*io).append_len as size_t).wrapping_add(len) as ::core::ffi::c_int
            as ::core::ffi::c_int;
        return;
    }
    txp_flush_pending(io);
    if len <= ::core::mem::size_of::<[::core::ffi::c_char; 4096]>() as usize {
        (*io).append_file = file;
        (*io).append_len = len as ::core::ffi::c_int;
        memcpy(
            &raw mut (*io).append_buf as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
            buf,
            len,
        );
        return;
    }
    txp_io_send_tag(io, T_APND);
    txp_io_send_u32(io, file as uint32_t);
    txp_io_send_u32(io, len as uint32_t);
    write_or_panic((*io).file, buf, len as ::core::ffi::c_int);
    txp_io_check_done(io);
}
#[no_mangle]
pub unsafe extern "C" fn txp_putc(
    mut io: *mut txp_client,
    mut file: txp_file_id,
    mut c: ::core::ffi::c_int,
) {
    if file == -(1 as txp_file_id)
        && ((*io).stdout_len as usize)
            < ::core::mem::size_of::<[::core::ffi::c_char; 4096]>() as usize
    {
        let fresh0 = (*io).stdout_len;
        (*io).stdout_len = (*io).stdout_len + 1;
        (*io).stdout_buf[fresh0 as usize] = c as ::core::ffi::c_char;
        return;
    }
    if (*io).append_file == file
        && ((*io).append_len as usize)
            < ::core::mem::size_of::<[::core::ffi::c_char; 4096]>() as usize
    {
        let fresh1 = (*io).append_len;
        (*io).append_len = (*io).append_len + 1;
        (*io).append_buf[fresh1 as usize] = c as ::core::ffi::c_char;
        return;
    }
    txp_flush_pending(io);
    (*io).append_file = file;
    (*io).append_buf[0 as ::core::ffi::c_int as usize] = c as ::core::ffi::c_char;
    (*io).append_len = 1 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn txp_close(mut io: *mut txp_client, mut file: txp_file_id) {
    txp_io_send_tag(io, T_CLOS);
    txp_io_send_u32(io, file as uint32_t);
    txp_io_check_done(io);
}
#[no_mangle]
pub unsafe extern "C" fn txp_fork(mut io: *mut txp_client) -> pid_t {
    (*io).generation = (*io).generation.wrapping_add(1 as uint32_t);
    txp_flush(io);
    let mut fd: ::core::ffi::c_int = fileno((*io).file);
    if fd == -(1 as ::core::ffi::c_int) {
        ppanic(b"fork_with_channel: fileno\0" as *const u8 as *const ::core::ffi::c_char);
    }
    let mut result: pid_t = texpresso_fork_with_channel(fd, xetex_tokens as uint32_t);
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn txp_gpic(
    mut io: *mut txp_client,
    mut path: *const ::core::ffi::c_char,
    mut typ: int32_t,
    mut page: int32_t,
    mut bounds: *mut ::core::ffi::c_float,
) -> bool {
    txp_io_send_tag(io, T_GPIC);
    txp_io_send_str(io, path);
    txp_io_send_u32(io, typ as uint32_t);
    txp_io_send_u32(io, page as uint32_t);
    let mut t: tag = txp_io_recv_tag(io);
    match t as ::core::ffi::c_uint {
        1397965136 => return 0 as ::core::ffi::c_int != 0,
        1128878151 => {
            *bounds.offset(0 as ::core::ffi::c_int as isize) = txp_io_recv_f32(io);
            *bounds.offset(1 as ::core::ffi::c_int as isize) = txp_io_recv_f32(io);
            *bounds.offset(2 as ::core::ffi::c_int as isize) = txp_io_recv_f32(io);
            *bounds.offset(3 as ::core::ffi::c_int as isize) = txp_io_recv_f32(io);
            return 1 as ::core::ffi::c_int != 0;
        }
        _ => {
            panic_tag(t);
        }
    }
    panic!("Reached end of non-void function without returning");
}
#[no_mangle]
pub unsafe extern "C" fn txp_spic(
    mut io: *mut txp_client,
    mut path: *const ::core::ffi::c_char,
    mut typ: int32_t,
    mut page: int32_t,
    mut bounds: *const ::core::ffi::c_float,
) {
    txp_io_send_tag(io, T_SPIC);
    txp_io_send_str(io, path);
    txp_io_send_u32(io, typ as uint32_t);
    txp_io_send_u32(io, page as uint32_t);
    txp_io_send_f32(io, *bounds.offset(0 as ::core::ffi::c_int as isize));
    txp_io_send_f32(io, *bounds.offset(1 as ::core::ffi::c_int as isize));
    txp_io_send_f32(io, *bounds.offset(2 as ::core::ffi::c_int as isize));
    txp_io_send_f32(io, *bounds.offset(3 as ::core::ffi::c_int as isize));
    txp_io_check_done(io);
}
#[no_mangle]
pub unsafe extern "C" fn txp_generation(mut client: *mut txp_client) -> uint32_t {
    return (*client).generation;
}
#[no_mangle]
pub unsafe extern "C" fn txp_bump_generation(mut client: *mut txp_client) {
    (*client).generation = (*client).generation.wrapping_add(1 as uint32_t);
}
#[no_mangle]
pub unsafe extern "C" fn txp_flush(mut io: *mut txp_client) {
    txp_flush_pending(io);
    txp_io_flush(io);
}
#[no_mangle]
pub unsafe extern "C" fn txp_font_barrier(mut io: *mut txp_client) {
    txp_io_send_tag(io, T_FNTB);
    txp_flush(io);
}
#[no_mangle]
pub unsafe extern "C" fn txp_mtime(mut io: *mut txp_client, mut file: txp_file_id) -> uint32_t {
    txp_io_send_tag(io, T_MTIM);
    txp_io_send_u32(io, file as uint32_t);
    let mut t: tag = txp_io_recv_tag(io);
    if t as ::core::ffi::c_uint != T_MTIM as ::core::ffi::c_int as ::core::ffi::c_uint {
        panic_tag(t);
    }
    return txp_io_recv_u32(io);
}
#[no_mangle]
pub unsafe extern "C" fn txp_size(mut io: *mut txp_client, mut file: txp_file_id) -> uint32_t {
    txp_io_send_tag(io, T_SIZE);
    txp_io_send_u32(io, file as uint32_t);
    let mut t: tag = txp_io_recv_tag(io);
    if t as ::core::ffi::c_uint != T_SIZE as ::core::ffi::c_int as ::core::ffi::c_uint {
        panic_tag(t);
    }
    return txp_io_recv_u32(io);
}
