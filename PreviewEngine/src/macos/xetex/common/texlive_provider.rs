// Translated from xetex/common/texlive_provider.c with C2Rust 0.22.1.
extern "C" {
    pub type __sFILEX;
    static mut __stdoutp: *mut FILE;
    static mut __stderrp: *mut FILE;
    fn fclose(_: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __mode: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fscanf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn fwrite(
        __ptr: *const ::core::ffi::c_void,
        __size: size_t,
        __nitems: size_t,
        __stream: *mut FILE,
    ) -> ::core::ffi::c_ulong;
    fn perror(_: *const ::core::ffi::c_char);
    fn pclose(_: *mut FILE) -> ::core::ffi::c_int;
    fn popen(_: *const ::core::ffi::c_char, _: *const ::core::ffi::c_char) -> *mut FILE;
    fn getline(
        __linep: *mut *mut ::core::ffi::c_char,
        __linecapp: *mut size_t,
        __stream: *mut FILE,
    ) -> ssize_t;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__count: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn abort() -> !;
    fn memmove(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strdup(__s1: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn stat(_: *const ::core::ffi::c_char, _: *mut stat) -> ::core::ffi::c_int;
}
pub type __uint16_t = u16;
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __int64_t = i64;
pub type __uint64_t = u64;
pub type __darwin_size_t = usize;
pub type __darwin_ssize_t = isize;
pub type __darwin_time_t = ::core::ffi::c_long;
pub type __darwin_blkcnt_t = __int64_t;
pub type __darwin_blksize_t = __int32_t;
pub type __darwin_dev_t = __int32_t;
pub type __darwin_gid_t = __uint32_t;
pub type __darwin_ino64_t = __uint64_t;
pub type __darwin_mode_t = __uint16_t;
pub type __darwin_off_t = __int64_t;
pub type __darwin_uid_t = __uint32_t;
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
pub type off_t = __darwin_off_t;
pub type ssize_t = __darwin_ssize_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct table {
    pub hash: C2RustUnnamed_0,
    pub entries: C2RustUnnamed,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed {
    pub buffer: *mut ::core::ffi::c_char,
    pub cap: ::core::ffi::c_int,
    pub len: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed_0 {
    pub pow: ::core::ffi::c_int,
    pub count: ::core::ffi::c_int,
    pub cells: *mut cell,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cell {
    pub hash: ::core::ffi::c_ulong,
    pub offset: ::core::ffi::c_ulong,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct timespec {
    pub tv_sec: __darwin_time_t,
    pub tv_nsec: ::core::ffi::c_long,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct stat {
    pub st_dev: dev_t,
    pub st_mode: mode_t,
    pub st_nlink: nlink_t,
    pub st_ino: __darwin_ino64_t,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    pub st_rdev: dev_t,
    pub st_atimespec: timespec,
    pub st_mtimespec: timespec,
    pub st_ctimespec: timespec,
    pub st_birthtimespec: timespec,
    pub st_size: off_t,
    pub st_blocks: blkcnt_t,
    pub st_blksize: blksize_t,
    pub st_flags: __uint32_t,
    pub st_gen: __uint32_t,
    pub st_lspare: __int32_t,
    pub st_qspare: [__int64_t; 2],
}
pub type blksize_t = __darwin_blksize_t;
pub type blkcnt_t = __darwin_blkcnt_t;
pub type dev_t = __darwin_dev_t;
pub type gid_t = __darwin_gid_t;
pub type uid_t = __darwin_uid_t;
pub type nlink_t = __uint16_t;
pub type mode_t = __darwin_mode_t;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const LOG: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
unsafe extern "C" fn sdbm_hash(mut p: *const ::core::ffi::c_void) -> ::core::ffi::c_ulong {
    let mut hash: ::core::ffi::c_ulong = 0 as ::core::ffi::c_ulong;
    let mut c: ::core::ffi::c_int = 0;
    let mut str: *const ::core::ffi::c_uchar = p as *const ::core::ffi::c_uchar;
    loop {
        c = *str as ::core::ffi::c_int;
        if !(c != 0) {
            break;
        }
        hash = (c as ::core::ffi::c_ulong)
            .wrapping_add(hash << 6 as ::core::ffi::c_int)
            .wrapping_add(hash << 16 as ::core::ffi::c_int)
            .wrapping_sub(hash);
        str = str.offset(1);
    }
    return hash.wrapping_mul(2654435761 as ::core::ffi::c_ulong);
}
unsafe extern "C" fn init(mut table_0: *mut table) {
    (*table_0).hash.pow = 18 as ::core::ffi::c_int;
    (*table_0).hash.count = 0 as ::core::ffi::c_int;
    (*table_0).hash.cells = calloc(
        ::core::mem::size_of::<cell>() as size_t,
        ((1 as ::core::ffi::c_int) << (*table_0).hash.pow) as size_t,
    ) as *mut cell;
    if (*table_0).hash.cells.is_null() {
        perror(b"calloc for cells\0" as *const u8 as *const ::core::ffi::c_char);
        abort();
    }
    (*table_0).entries.cap = 256 as ::core::ffi::c_int;
    (*table_0).entries.len = 1 as ::core::ffi::c_int;
    (*table_0).entries.buffer = malloc(256 as size_t) as *mut ::core::ffi::c_char;
    if (*table_0).entries.buffer.is_null() {
        perror(b"malloc for entries buffer\0" as *const u8 as *const ::core::ffi::c_char);
        free((*table_0).hash.cells as *mut ::core::ffi::c_void);
        abort();
    }
    *(*table_0)
        .entries
        .buffer
        .offset(0 as ::core::ffi::c_int as isize) = '\0' as i32 as ::core::ffi::c_char;
}
unsafe extern "C" fn lookup(
    mut table_0: *mut table,
    mut key: *const ::core::ffi::c_char,
) -> *mut cell {
    let mut hash: ::core::ffi::c_ulong = sdbm_hash(key as *const ::core::ffi::c_void);
    let mut mask: ::core::ffi::c_ulong = (((1 as ::core::ffi::c_int) << (*table_0).hash.pow)
        - 1 as ::core::ffi::c_int) as ::core::ffi::c_ulong;
    let mut index: ::core::ffi::c_uint = (hash & mask) as ::core::ffi::c_uint;
    let mut cells: *mut cell = (*table_0).hash.cells;
    let mut buffer: *mut ::core::ffi::c_char = (*table_0).entries.buffer;
    while (*cells.offset(index as isize)).offset != 0 as ::core::ffi::c_ulong {
        if (*cells.offset(index as isize)).hash == hash
            && strcmp(
                key,
                buffer.offset((*cells.offset(index as isize)).offset as isize)
                    as *mut ::core::ffi::c_char,
            ) == 0 as ::core::ffi::c_int
        {
            return cells.offset(index as isize) as *mut cell;
        }
        index = (index.wrapping_add(1 as ::core::ffi::c_uint) as ::core::ffi::c_ulong & mask)
            as ::core::ffi::c_uint;
    }
    (*cells.offset(index as isize)).hash = hash;
    return cells.offset(index as isize) as *mut cell;
}
unsafe extern "C" fn grow(mut table_0: *mut table) {
    let mut ocells: *mut cell = (*table_0).hash.cells;
    let mut count: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << (*table_0).hash.pow;
    (*table_0).hash.pow += 1 as ::core::ffi::c_int;
    (*table_0).hash.cells = calloc(
        ::core::mem::size_of::<cell>() as size_t,
        ((1 as ::core::ffi::c_int) << (*table_0).hash.pow) as size_t,
    ) as *mut cell;
    if (*table_0).hash.cells.is_null() {
        perror(b"calloc for growing table\0" as *const u8 as *const ::core::ffi::c_char);
        abort();
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < count {
        let mut ocell: *mut cell = ocells.offset(i as isize) as *mut cell;
        if (*ocell).offset != 0 as ::core::ffi::c_ulong {
            let mut ncell: *mut cell = lookup(
                table_0,
                (*table_0).entries.buffer.offset((*ocell).offset as isize)
                    as *mut ::core::ffi::c_char,
            );
            if (*ncell).offset != 0 as ::core::ffi::c_ulong {
                abort();
            }
            *ncell = *ocell;
        }
        i += 1;
    }
    free(ocells as *mut ::core::ffi::c_void);
}
unsafe extern "C" fn find(
    mut table_0: *mut table,
    mut key: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    let mut c: *mut cell = lookup(table_0, key);
    if (*c).offset == 0 as ::core::ffi::c_ulong {
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    let mut p: *const ::core::ffi::c_char =
        (*table_0).entries.buffer.offset((*c).offset as isize) as *mut ::core::ffi::c_char;
    while *p.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int != '\0' as i32 {
        p = p.offset(-1);
    }
    return p;
}
unsafe extern "C" fn has_component(
    mut path: *const ::core::ffi::c_char,
    mut component: *const ::core::ffi::c_char,
) -> bool {
    while *path as ::core::ffi::c_int == *component as ::core::ffi::c_int
        && *component as ::core::ffi::c_int != 0
    {
        path = path.offset(1);
        component = component.offset(1);
    }
    return *component == 0 && (*path == 0 || *path as ::core::ffi::c_int == '/' as i32);
}
unsafe extern "C" fn rank_component(
    mut existing: *const ::core::ffi::c_char,
    mut dir: *const ::core::ffi::c_char,
) -> bool {
    static mut components: [*const ::core::ffi::c_char; 5] = [
        b"xelatex\0" as *const u8 as *const ::core::ffi::c_char,
        b"latex\0" as *const u8 as *const ::core::ffi::c_char,
        b"xetex\0" as *const u8 as *const ::core::ffi::c_char,
        b"generic\0" as *const u8 as *const ::core::ffi::c_char,
        ::core::ptr::null::<::core::ffi::c_char>(),
    ];
    let mut component: *mut *const ::core::ffi::c_char =
        &raw mut components as *mut *const ::core::ffi::c_char;
    while !(*component).is_null() {
        if has_component(dir, *component) {
            return 1 as ::core::ffi::c_int != 0;
        }
        if has_component(existing, *component) {
            return 0 as ::core::ffi::c_int != 0;
        }
        component = component.offset(1);
    }
    return 0 as ::core::ffi::c_int != 0;
}
unsafe extern "C" fn rank(
    mut existing: *const ::core::ffi::c_char,
    mut root: *const ::core::ffi::c_char,
    mut dir: *const ::core::ffi::c_char,
) -> bool {
    while *root as ::core::ffi::c_int != 0
        && *existing as ::core::ffi::c_int != 0
        && *existing as ::core::ffi::c_int == *root as ::core::ffi::c_int
    {
        existing = existing.offset(1);
        root = root.offset(1);
    }
    if *root as ::core::ffi::c_int != 0 || *existing as ::core::ffi::c_int != '/' as i32 {
        fprintf(
            __stderrp,
            b"rank: root differ, skipping\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return 0 as ::core::ffi::c_int != 0;
    }
    existing = existing.offset(1);
    let mut eslash: *const ::core::ffi::c_char = existing;
    let mut dslash: *const ::core::ffi::c_char = dir;
    while *existing as ::core::ffi::c_int != 0
        && *dir as ::core::ffi::c_int == *existing as ::core::ffi::c_int
    {
        if *dir as ::core::ffi::c_int == '/' as i32 {
            eslash = existing.offset(1 as ::core::ffi::c_int as isize);
            dslash = dir.offset(1 as ::core::ffi::c_int as isize);
        }
        existing = existing.offset(1);
        dir = dir.offset(1);
    }
    if rank_component(eslash, dslash) {
        fprintf(
            __stderrp,
            b"%s has priority over %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            dslash,
            eslash,
        );
        return 1 as ::core::ffi::c_int != 0;
    } else {
        fprintf(
            __stderrp,
            b"%s does not have priority over %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            dslash,
            eslash,
        );
        return 0 as ::core::ffi::c_int != 0;
    };
}
unsafe extern "C" fn add(
    mut table_0: *mut table,
    mut root: *const ::core::ffi::c_char,
    mut dir: *const ::core::ffi::c_char,
    mut name: *const ::core::ffi::c_char,
) {
    let mut has_dot: bool = 0 as ::core::ffi::c_int != 0;
    let mut p: *const ::core::ffi::c_char = name;
    while *p != 0 {
        if *p as ::core::ffi::c_int == '.' as i32 {
            has_dot = 1 as ::core::ffi::c_int != 0;
            break;
        } else {
            p = p.offset(1);
        }
    }
    if !has_dot {
        return;
    }
    if has_component(dir, b"doc\0" as *const u8 as *const ::core::ffi::c_char) as ::core::ffi::c_int
        != 0
        || has_component(dir, b"source\0" as *const u8 as *const ::core::ffi::c_char)
            as ::core::ffi::c_int
            != 0
    {
        return;
    }
    let mut c: *mut cell = lookup(table_0, name);
    if (*c).offset != 0 as ::core::ffi::c_ulong {
        let mut p_0: *mut ::core::ffi::c_char =
            (*table_0).entries.buffer.offset((*c).offset as isize) as *mut ::core::ffi::c_char;
        while *p_0.offset(-(1 as ::core::ffi::c_int) as isize) != 0 {
            p_0 = p_0.offset(-(1 as ::core::ffi::c_int as isize));
        }
        if rank(p_0, root, dir) {
            fprintf(
                __stderrp,
                b"add: %s/%s/%s shadows previous binding\n  (already having: %s)\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                root,
                dir,
                name,
                p_0,
            );
        } else {
            return;
        }
    } else {
        (*table_0).hash.count += 1 as ::core::ffi::c_int;
    }
    let mut rlen: ::core::ffi::c_int = strlen(root) as ::core::ffi::c_int;
    let mut dlen: ::core::ffi::c_int = strlen(dir) as ::core::ffi::c_int;
    let mut nlen: ::core::ffi::c_int = strlen(name) as ::core::ffi::c_int;
    let mut tlen: ::core::ffi::c_int = (*table_0).entries.len
        + rlen
        + 1 as ::core::ffi::c_int
        + dlen
        + 1 as ::core::ffi::c_int
        + nlen
        + 1 as ::core::ffi::c_int;
    if (*table_0).entries.cap < tlen {
        while (*table_0).entries.cap < tlen {
            (*table_0).entries.cap *= 2 as ::core::ffi::c_int;
        }
        (*table_0).entries.buffer = realloc(
            (*table_0).entries.buffer as *mut ::core::ffi::c_void,
            (*table_0).entries.cap as size_t,
        ) as *mut ::core::ffi::c_char;
        if (*table_0).entries.buffer.is_null() {
            perror(b"realloc for growing buffer\0" as *const u8 as *const ::core::ffi::c_char);
            abort();
        }
    }
    let mut data: *mut ::core::ffi::c_char = (*table_0)
        .entries
        .buffer
        .offset((*table_0).entries.len as isize);
    memmove(
        data as *mut ::core::ffi::c_void,
        root as *const ::core::ffi::c_void,
        rlen as size_t,
    );
    data = data.offset(rlen as isize);
    let fresh0 = data;
    data = data.offset(1);
    *fresh0 = '/' as i32 as ::core::ffi::c_char;
    memmove(
        data as *mut ::core::ffi::c_void,
        dir as *const ::core::ffi::c_void,
        dlen as size_t,
    );
    data = data.offset(dlen as isize);
    let fresh1 = data;
    data = data.offset(1);
    *fresh1 = '/' as i32 as ::core::ffi::c_char;
    memmove(
        data as *mut ::core::ffi::c_void,
        name as *const ::core::ffi::c_void,
        nlen as size_t,
    );
    *data.offset(nlen as isize) = 0 as ::core::ffi::c_char;
    (*c).offset =
        data.offset_from((*table_0).entries.buffer) as ::core::ffi::c_long as ::core::ffi::c_ulong;
    (*table_0).entries.len = tlen;
    if (*table_0).hash.count * 3 as ::core::ffi::c_int
        == ((1 as ::core::ffi::c_int) << (*table_0).hash.pow) * 2 as ::core::ffi::c_int
    {
        grow(table_0);
    }
}
unsafe extern "C" fn process_line(mut table_0: *mut table, mut path: *mut ::core::ffi::c_char) {
    let mut f: *mut FILE =
        fopen(path, b"rb\0" as *const u8 as *const ::core::ffi::c_char) as *mut FILE;
    if f.is_null() {
        perror(b"Cannot open file\0" as *const u8 as *const ::core::ffi::c_char);
        fprintf(
            __stderrp,
            b"File: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
            path,
        );
        return;
    }
    let mut slash: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut p: *mut ::core::ffi::c_char = path;
    while *p != 0 {
        if *p as ::core::ffi::c_int == '/' as i32 {
            slash = p;
        }
        p = p.offset(1);
    }
    if !slash.is_null() {
        *slash = 0 as ::core::ffi::c_char;
    }
    let mut sub: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut line: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut cap: size_t = 0 as size_t;
    let mut len: ssize_t = 0;
    loop {
        len = getline(&raw mut line, &raw mut cap, f);
        if !(len != -(1 as ::core::ffi::c_int) as ssize_t) {
            break;
        }
        if len == 0 as ssize_t {
            continue;
        }
        if *line.offset((len - 1 as ssize_t) as isize) as ::core::ffi::c_int == '\n' as i32 {
            *line.offset((len - 1 as ssize_t) as isize) = '\0' as i32 as ::core::ffi::c_char;
            len -= 1;
        }
        if len == 0 as ssize_t {
            continue;
        }
        if *line.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int == '.' as i32
            && *line.offset((len - 1 as ssize_t) as isize) as ::core::ffi::c_int == ':' as i32
        {
            if *line.offset((len - 2 as ssize_t) as isize) as ::core::ffi::c_int == '/' as i32 {
                *line.offset((len - 2 as ssize_t) as isize) = 0 as ::core::ffi::c_char;
            } else {
                *line.offset((len - 1 as ssize_t) as isize) = 0 as ::core::ffi::c_char;
            }
            free(sub as *mut ::core::ffi::c_void);
            sub = strdup(
                if *line.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    == '/' as i32
                {
                    line.offset(2 as ::core::ffi::c_int as isize)
                } else {
                    line.offset(1 as ::core::ffi::c_int as isize)
                },
            );
            if sub.is_null() {
                perror(b"strdup\0" as *const u8 as *const ::core::ffi::c_char);
                free(line as *mut ::core::ffi::c_void);
                fclose(f);
                return;
            }
        } else {
            add(
                table_0,
                path,
                if !sub.is_null() {
                    sub as *const ::core::ffi::c_char
                } else {
                    b"\0" as *const u8 as *const ::core::ffi::c_char
                },
                line,
            );
        }
    }
    free(sub as *mut ::core::ffi::c_void);
    free(line as *mut ::core::ffi::c_void);
    if fclose(f) != 0 as ::core::ffi::c_int {
        perror(b"fclose\0" as *const u8 as *const ::core::ffi::c_char);
    }
}
#[no_mangle]
pub static mut table: table = table {
    hash: C2RustUnnamed_0 {
        pow: 0,
        count: 0,
        cells: ::core::ptr::null::<cell>() as *mut cell,
    },
    entries: C2RustUnnamed {
        buffer: ::core::ptr::null::<::core::ffi::c_char>() as *mut ::core::ffi::c_char,
        cap: 0,
        len: 0,
    },
};
unsafe extern "C" fn list_texlive_files() -> bool {
    static mut loaded: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if loaded != 0 {
        return loaded == 1 as ::core::ffi::c_int;
    }
    loaded = -(1 as ::core::ffi::c_int);
    init(&raw mut table);
    let mut line: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut cap: size_t = 0 as size_t;
    let mut len: ssize_t = 0;
    let mut ret: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut p: *mut FILE = popen(
        b"kpsewhich --all -engine=xetex ls-R\0" as *const u8 as *const ::core::ffi::c_char,
        b"r\0" as *const u8 as *const ::core::ffi::c_char,
    );
    if p.is_null() {
        perror(b"popen\0" as *const u8 as *const ::core::ffi::c_char);
        return 0 as ::core::ffi::c_int != 0;
    }
    loop {
        len = getline(&raw mut line, &raw mut cap, p);
        if !(len != -(1 as ::core::ffi::c_int) as ssize_t) {
            break;
        }
        if len > 0 as ssize_t
            && *line.offset((len - 1 as ssize_t) as isize) as ::core::ffi::c_int == '\n' as i32
        {
            *line.offset((len - 1 as ssize_t) as isize) = '\0' as i32 as ::core::ffi::c_char;
        }
        if len == 0 as ssize_t {
            continue;
        }
        process_line(&raw mut table, line);
    }
    free(line as *mut ::core::ffi::c_void);
    ret = pclose(p);
    if ret == -(1 as ::core::ffi::c_int) {
        perror(b"pclose\0" as *const u8 as *const ::core::ffi::c_char);
    } else if ret > 0 as ::core::ffi::c_int {
        fprintf(
            __stderrp,
            b"Exit code: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
            ret,
        );
    } else {
        loaded = 1 as ::core::ffi::c_int;
    }
    return loaded == 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn stat_path(
    mut path: *const ::core::ffi::c_char,
    mut size: *mut ::core::ffi::c_int,
    mut mtime: *mut ::core::ffi::c_int,
) {
    let mut st: stat = stat {
        st_dev: 0,
        st_mode: 0,
        st_nlink: 0,
        st_ino: 0,
        st_uid: 0,
        st_gid: 0,
        st_rdev: 0,
        st_atimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_mtimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_ctimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_birthtimespec: timespec {
            tv_sec: 0,
            tv_nsec: 0,
        },
        st_size: 0,
        st_blocks: 0,
        st_blksize: 0,
        st_flags: 0,
        st_gen: 0,
        st_lspare: 0,
        st_qspare: [0; 2],
    };
    if !path.is_null() && stat(path, &raw mut st) == 0 as ::core::ffi::c_int {
        *size = st.st_size as ::core::ffi::c_int;
        *mtime = st.st_mtimespec.tv_sec as ::core::ffi::c_int;
    } else {
        *size = -(1 as ::core::ffi::c_int);
        *mtime = -(1 as ::core::ffi::c_int);
    };
}
#[no_mangle]
pub unsafe extern "C" fn texlive_file_path(
    mut name: *const ::core::ffi::c_char,
    mut record_dependency: *mut FILE,
) -> *const ::core::ffi::c_char {
    if !list_texlive_files() {
        fprintf(
            __stderrp,
            b"TeXlive is not available\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return ::core::ptr::null::<::core::ffi::c_char>();
    }
    let mut path: *const ::core::ffi::c_char = find(&raw mut table, name);
    if !record_dependency.is_null() {
        let mut size: ::core::ffi::c_int = 0;
        let mut mtime: ::core::ffi::c_int = 0;
        stat_path(path, &raw mut size, &raw mut mtime);
        fprintf(
            record_dependency,
            b"%s\n%d:%d\n\0" as *const u8 as *const ::core::ffi::c_char,
            name,
            size,
            mtime,
        );
    }
    return path;
}
#[no_mangle]
pub unsafe extern "C" fn texlive_check_dependencies(mut record: *mut FILE) -> bool {
    let mut name: [::core::ffi::c_char; 1024] = [0; 1024];
    let mut size: ::core::ffi::c_int = 0;
    let mut mtime: ::core::ffi::c_int = 0;
    while fscanf(
        record,
        b"%1023[^\n]\n%d:%d\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut name as *mut ::core::ffi::c_char,
        &raw mut size,
        &raw mut mtime,
    ) == 3 as ::core::ffi::c_int
    {
        let mut size2: ::core::ffi::c_int = 0;
        let mut mtime2: ::core::ffi::c_int = 0;
        stat_path(
            find(&raw mut table, &raw mut name as *mut ::core::ffi::c_char),
            &raw mut size2,
            &raw mut mtime2,
        );
        if size != size2 || mtime != mtime2 {
            return 0 as ::core::ffi::c_int != 0;
        }
    }
    return 1 as ::core::ffi::c_int != 0;
}
#[no_mangle]
pub unsafe extern "C" fn texlive_available() -> bool {
    return list_texlive_files() as ::core::ffi::c_int == 1 as ::core::ffi::c_int;
}
