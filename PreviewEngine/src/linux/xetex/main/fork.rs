// Translated from xetex/main/fork.c with C2Rust 0.22.1.
extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn waitpid(
        __pid: __pid_t,
        __stat_loc: *mut ::core::ffi::c_int,
        __options: ::core::ffi::c_int,
    ) -> __pid_t;
    fn close(__fd: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn read(
        __fd: ::core::ffi::c_int,
        __buf: *mut ::core::ffi::c_void,
        __nbytes: size_t,
    ) -> ssize_t;
    fn dup2(__fd: ::core::ffi::c_int, __fd2: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn fork() -> __pid_t;
    fn socketpair(
        __domain: ::core::ffi::c_int,
        __type: ::core::ffi::c_int,
        __protocol: ::core::ffi::c_int,
        __fds: *mut ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn sendmsg(
        __fd: ::core::ffi::c_int,
        __message: *const msghdr,
        __flags: ::core::ffi::c_int,
    ) -> ssize_t;
    fn abort() -> !;
    static mut stderr: *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn perror(__s: *const ::core::ffi::c_char);
}
pub type __int32_t = i32;
pub type __uint32_t = u32;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
pub type __pid_t = ::core::ffi::c_int;
pub type __ssize_t = ::core::ffi::c_long;
pub type __socklen_t = ::core::ffi::c_uint;
pub type pid_t = __pid_t;
pub type size_t = usize;
pub type ssize_t = __ssize_t;
pub type socklen_t = __socklen_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut ::core::ffi::c_void,
    pub iov_len: size_t,
}
pub type int32_t = __int32_t;
pub type __socket_type = ::core::ffi::c_uint;
pub const SOCK_NONBLOCK: __socket_type = 2048;
pub const SOCK_CLOEXEC: __socket_type = 524288;
pub const SOCK_PACKET: __socket_type = 10;
pub const SOCK_DCCP: __socket_type = 6;
pub const SOCK_SEQPACKET: __socket_type = 5;
pub const SOCK_RDM: __socket_type = 4;
pub const SOCK_RAW: __socket_type = 3;
pub const SOCK_DGRAM: __socket_type = 2;
pub const SOCK_STREAM: __socket_type = 1;
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
pub struct cmsghdr {
    pub cmsg_len: size_t,
    pub cmsg_level: ::core::ffi::c_int,
    pub cmsg_type: ::core::ffi::c_int,
    pub __cmsg_data: [::core::ffi::c_uchar; 0],
}
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const SCM_RIGHTS: C2RustUnnamed = 1;
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
pub type uint32_t = __uint32_t;
pub const EINTR: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const SOL_SOCKET: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
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
        msg_iovlen: 3 as size_t,
        msg_control: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        msg_controllen: ((1 as size_t)
            .wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t)
            .wrapping_add(::core::mem::size_of::<size_t>() as size_t)
            .wrapping_sub(1 as size_t)
            & !(::core::mem::size_of::<size_t>() as usize).wrapping_sub(1 as usize))
            .wrapping_add(
                (::core::mem::size_of::<cmsghdr>() as size_t)
                    .wrapping_add(::core::mem::size_of::<size_t>() as size_t)
                    .wrapping_sub(1 as size_t)
                    & !(::core::mem::size_of::<size_t>() as usize)
                        .wrapping_sub(1 as usize),
            ),
        msg_flags: 0,
    };
    msg.msg_control = &raw mut msg_control as *mut ::core::ffi::c_void;
    let mut cm: *mut cmsghdr = if msg.msg_controllen
        >= ::core::mem::size_of::<cmsghdr>() as usize
    {
        msg.msg_control as *mut cmsghdr
    } else {
        ::core::ptr::null_mut::<cmsghdr>()
    };
    (*cm).cmsg_level = SOL_SOCKET;
    (*cm).cmsg_type = SCM_RIGHTS as ::core::ffi::c_int;
    (*cm).cmsg_len = ((::core::mem::size_of::<cmsghdr>() as usize)
        .wrapping_add(::core::mem::size_of::<size_t>() as usize)
        .wrapping_sub(1 as usize)
        & !(::core::mem::size_of::<size_t>() as usize).wrapping_sub(1 as usize))
        .wrapping_add(
            (1 as usize)
                .wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as usize),
        ) as size_t;
    let mut fds0: *mut ::core::ffi::c_int = &raw mut (*cm).__cmsg_data
        as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_int;
    *fds0.offset(0 as ::core::ffi::c_int as isize) = child_fd;
    loop {
        sent = sendmsg(chan_fd, &raw mut msg, 0 as ::core::ffi::c_int);
        if !(sent == -(1 as ::core::ffi::c_int) as ssize_t
            && *__errno_location() == EINTR)
        {
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
            stderr,
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
        SOCK_STREAM as ::core::ffi::c_int,
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
    child = fork() as pid_t;
    if child == -(1 as ::core::ffi::c_int) {
        perror(
            b"texpresso_fork_with_channel failure (xetex/main/fork.c:72)\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        abort();
    }
    if child == 0 as ::core::ffi::c_int {
        if dup2(sockets[1 as ::core::ffi::c_int as usize], fd)
            == -(1 as ::core::ffi::c_int)
        {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:77)\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
            abort();
        }
        txp_forked_child = true_0 != 0;
        if close(sockets[0 as ::core::ffi::c_int as usize]) == -(1 as ::core::ffi::c_int)
        {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:79)\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
            abort();
        }
        if close(sockets[1 as ::core::ffi::c_int as usize]) == -(1 as ::core::ffi::c_int)
        {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:80)\0"
                    as *const u8 as *const ::core::ffi::c_char,
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
        if close(sockets[0 as ::core::ffi::c_int as usize]) == -(1 as ::core::ffi::c_int)
        {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:89)\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
            abort();
        }
        if close(sockets[1 as ::core::ffi::c_int as usize]) == -(1 as ::core::ffi::c_int)
        {
            perror(
                b"texpresso_fork_with_channel failure (xetex/main/fork.c:90)\0"
                    as *const u8 as *const ::core::ffi::c_char,
            );
            abort();
        }
        let mut status: ::core::ffi::c_int = 0;
        while waitpid(child as __pid_t, &raw mut status, 0 as ::core::ffi::c_int)
            == -(1 as ::core::ffi::c_int)
        {
            if *__errno_location() == EINTR {
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
                    &raw mut answer as *mut ::core::ffi::c_char
                        as *mut ::core::ffi::c_void,
                    4 as size_t,
                ) as ::core::ffi::c_int;
                if !(recvd == -(1 as ::core::ffi::c_int) && *__errno_location() == EINTR)
                {
                    break;
                }
            }
            if !(recvd == 4 as ::core::ffi::c_int
                && answer[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    == 'F' as i32
                && answer[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    == 'L' as i32
                && answer[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    == 'S' as i32
                && answer[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    == 'H' as i32)
            {
                break;
            }
        }
        if !(recvd == 4 as ::core::ffi::c_int
            && answer[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                == 'D' as i32
            && answer[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                == 'O' as i32
            && answer[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                == 'N' as i32
            && answer[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                == 'E' as i32)
        {
            fprintf(
                stderr,
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
