// Translated from xetex/main/fork.c with C2Rust 0.22.1.
extern "C" {
    pub type __sFILEX;
    fn __error() -> *mut ::core::ffi::c_int;
    fn waitpid(_: pid_t, _: *mut ::core::ffi::c_int, _: ::core::ffi::c_int) -> pid_t;
    fn close(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn dup2(_: ::core::ffi::c_int, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn fork() -> pid_t;
    fn read(_: ::core::ffi::c_int, _: *mut ::core::ffi::c_void, __nbyte: size_t) -> ssize_t;
    fn sendmsg(_: ::core::ffi::c_int, _: *const msghdr, _: ::core::ffi::c_int) -> ssize_t;
    fn socketpair(
        _: ::core::ffi::c_int,
        _: ::core::ffi::c_int,
        _: ::core::ffi::c_int,
        _: *mut ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn abort() -> !;
    static mut __stderrp: *mut FILE;
    fn fprintf(_: *mut FILE, _: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn perror(_: *const ::core::ffi::c_char);
}
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __int64_t = i64;
pub type __darwin_size_t = usize;
pub type __darwin_socklen_t = __uint32_t;
pub type __darwin_ssize_t = isize;
pub type __darwin_off_t = __int64_t;
pub type __darwin_pid_t = __int32_t;
pub type pid_t = __darwin_pid_t;
pub type int32_t = i32;
pub type size_t = __darwin_size_t;
pub type uint32_t = u32;
pub type ssize_t = __darwin_ssize_t;
pub type socklen_t = __darwin_socklen_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct msghdr {
    pub msg_name: *mut ::core::ffi::c_void,
    pub msg_namelen: socklen_t,
    pub msg_iov: *mut iovec,
    pub msg_iovlen: ::core::ffi::c_int,
    pub msg_control: *mut ::core::ffi::c_void,
    pub msg_controllen: socklen_t,
    pub msg_flags: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cmsghdr {
    pub cmsg_len: socklen_t,
    pub cmsg_level: ::core::ffi::c_int,
    pub cmsg_type: ::core::ffi::c_int,
}
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
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const __DARWIN_ALIGNBYTES32: usize =
    (::core::mem::size_of::<__uint32_t>() as usize).wrapping_sub(1 as usize);
pub const SOL_SOCKET: ::core::ffi::c_int = 0xffff as ::core::ffi::c_int;
pub const SCM_RIGHTS: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub static mut txp_forked_child: bool = false_0 != 0;
unsafe extern "C" fn send_child_fd(
    mut chan_fd: ::core::ffi::c_int,
    mut pid: int32_t,
    mut time: uint32_t,
    mut child_fd: ::core::ffi::c_int,
) {
    let mut sent: ssize_t = 0;
    let mut msg_control: [::core::ffi::c_char; 16] = [
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
    ];
    let mut iov: [iovec; 3] = [
        iovec {
            iov_base: b"CHLD\0" as *const u8 as *const ::core::ffi::c_char
                as *mut ::core::ffi::c_void,
            iov_len: 4 as size_t,
        },
        iovec {
            iov_base: &raw mut time as *mut ::core::ffi::c_void,
            iov_len: 4 as size_t,
        },
        iovec {
            iov_base: &raw mut pid as *mut ::core::ffi::c_void,
            iov_len: 4 as size_t,
        },
    ];
    let mut msg: msghdr = msghdr {
        msg_name: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        msg_namelen: 0,
        msg_iov: &raw mut iov as *mut iovec,
        msg_iovlen: 3 as ::core::ffi::c_int,
        msg_control: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        msg_controllen: (::core::mem::size_of::<cmsghdr>().wrapping_add(__DARWIN_ALIGNBYTES32)
            & !__DARWIN_ALIGNBYTES32)
            .wrapping_add(
                (1 as usize)
                    .wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as usize)
                    .wrapping_add(__DARWIN_ALIGNBYTES32)
                    & !__DARWIN_ALIGNBYTES32,
            ) as socklen_t,
        msg_flags: 0,
    };
    msg.msg_control = &raw mut msg_control as *mut ::core::ffi::c_void;
    let mut cm: *mut cmsghdr =
        if msg.msg_controllen as usize >= ::core::mem::size_of::<cmsghdr>() as usize {
            msg.msg_control as *mut cmsghdr
        } else {
            ::core::ptr::null_mut::<cmsghdr>()
        };
    (*cm).cmsg_level = SOL_SOCKET;
    (*cm).cmsg_type = SCM_RIGHTS;
    (*cm).cmsg_len = (::core::mem::size_of::<cmsghdr>().wrapping_add(__DARWIN_ALIGNBYTES32)
        & !__DARWIN_ALIGNBYTES32)
        .wrapping_add(
            (1 as __darwin_size_t)
                .wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as __darwin_size_t),
        ) as socklen_t;
    let mut fds0: *mut ::core::ffi::c_int = (cm as *mut ::core::ffi::c_uchar).offset(
        (::core::mem::size_of::<cmsghdr>().wrapping_add(__DARWIN_ALIGNBYTES32)
            & !__DARWIN_ALIGNBYTES32) as isize,
    ) as *mut ::core::ffi::c_int;
    *fds0.offset(0 as ::core::ffi::c_int as isize) = child_fd;
    loop {
        sent = sendmsg(chan_fd, &raw mut msg, 0 as ::core::ffi::c_int);
        if !(sent == -(1 as ::core::ffi::c_int) as ssize_t && *__error() == EINTR) {
            break;
        }
    }
    if sent == -(1 as ::core::ffi::c_int) as ssize_t {
        perror(
            b"texpresso_fork_with_channel failure (xetex/main/fork.c:59)\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        abort();
    }
    if !(sent == 12 as ssize_t) {
        fprintf(
            __stderrp,
            b"texpresso_fork_with_channel failure (sent == 12) \0" as *const u8
                as *const ::core::ffi::c_char,
        );
        abort();
    }
}
#[no_mangle]
pub unsafe extern "C" fn texpresso_fork_with_channel(
    mut fd: ::core::ffi::c_int,
    mut time: uint32_t,
) -> pid_t {
    let mut sockets: [::core::ffi::c_int; 2] = [0; 2];
    if socketpair(
        1 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        &raw mut sockets as *mut ::core::ffi::c_int,
    ) == -(1 as ::core::ffi::c_int)
    {
        perror(
            b"texpresso_fork_with_channel failure (xetex/main/fork.c:68)\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        abort();
    }
    let mut child: pid_t = 0;
    child = fork();
    if child == -(1 as pid_t) {
        perror(
            b"texpresso_fork_with_channel failure (xetex/main/fork.c:72)\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        abort();
    }
    if child == 0 as pid_t {
        if dup2(sockets[1 as ::core::ffi::c_int as usize], fd) == -(1 as ::core::ffi::c_int) {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:77)\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            abort();
        }
        txp_forked_child = true_0 != 0;
        if close(sockets[0 as ::core::ffi::c_int as usize]) == -(1 as ::core::ffi::c_int) {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:79)\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            abort();
        }
        if close(sockets[1 as ::core::ffi::c_int as usize]) == -(1 as ::core::ffi::c_int) {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:80)\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            abort();
        }
    } else {
        send_child_fd(
            fd,
            child as int32_t,
            time,
            sockets[0 as ::core::ffi::c_int as usize],
        );
        if close(sockets[0 as ::core::ffi::c_int as usize]) == -(1 as ::core::ffi::c_int) {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:89)\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            abort();
        }
        if close(sockets[1 as ::core::ffi::c_int as usize]) == -(1 as ::core::ffi::c_int) {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:90)\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            abort();
        }
        let mut status: ::core::ffi::c_int = 0;
        while waitpid(child, &raw mut status, 0 as ::core::ffi::c_int) == -(1 as pid_t) {
            if *__error() == EINTR {
                continue;
            }
            perror(b"waitpid\0" as *const u8 as *const ::core::ffi::c_char);
            return 1 as pid_t;
        }
        let mut answer: [::core::ffi::c_char; 4] = [0; 4];
        let mut recvd: ::core::ffi::c_int = 0;
        loop {
            loop {
                recvd = read(
                    fd,
                    &raw mut answer as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
                    4 as size_t,
                ) as ::core::ffi::c_int;
                if !(recvd == -(1 as ::core::ffi::c_int) && *__error() == EINTR) {
                    break;
                }
            }
            if !(recvd == 4 as ::core::ffi::c_int
                && answer[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == 'F' as i32
                && answer[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == 'L' as i32
                && answer[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == 'S' as i32
                && answer[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == 'H' as i32)
            {
                break;
            }
        }
        if !(recvd == 4 as ::core::ffi::c_int
            && answer[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == 'D' as i32
            && answer[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == 'O' as i32
            && answer[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == 'N' as i32
            && answer[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int == 'E' as i32)
        {
            fprintf(
                __stderrp,
                b"texpresso_fork_with_channel failure (recvd == 4 && answer[0] == 'D' && answer[1] == 'O' && answer[2] == 'N' && answer[3] == 'E') recvd: %d, answer: %C%C%C%C\0"
                    as *const u8 as *const ::core::ffi::c_char,
                recvd,
                answer[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int,
                answer[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int,
                answer[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int,
                answer[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int,
            );
            abort();
        }
    }
    return child;
}
