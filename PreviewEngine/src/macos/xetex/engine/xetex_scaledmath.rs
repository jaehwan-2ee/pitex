/* tectonic/xetex-scaledmath.c: low-level math functions
   Copyright 2017 The Tectonic Project
   Licensed under the MIT License.
*/
// Translated from xetex/engine/xetex-scaledmath.c with C2Rust 0.22.1.
extern "C" {
    pub type ttbc_diagnostic_t;
    fn abs(_: ::core::ffi::c_int) -> ::core::ffi::c_int;
    static mut help_line: [*const ::core::ffi::c_char; 6];
    static mut help_ptr: ::core::ffi::c_uchar;
    static mut arith_error: bool;
    static mut tex_remainder: scaled_t;
    static mut randoms: [int32_t; 55];
    static mut j_random: ::core::ffi::c_uchar;
    static mut two_to_the: [int32_t; 31];
    static mut spec_log: [int32_t; 29];
    fn error();
    fn capture_to_diagnostic(diagnostic: *mut ttbc_diagnostic_t);
    fn error_here_with_diagnostic(message: *const ::core::ffi::c_char) -> *mut ttbc_diagnostic_t;
    fn print_cstr(s: *const ::core::ffi::c_char);
    fn print_scaled(s: scaled_t);
}
pub type int32_t = i32;
pub type scaled_t = int32_t;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
#[no_mangle]
pub unsafe extern "C" fn tex_round(mut r: ::core::ffi::c_double) -> int32_t {
    if r > 2147483647.0f64 {
        return 2147483647 as int32_t;
    }
    if r < -2147483648.0f64 {
        return -(2147483648 as ::core::ffi::c_long) as int32_t;
    }
    if r >= 0.0f64 {
        return (r + 0.5f64) as int32_t;
    }
    return (r - 0.5f64) as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn half(mut x: int32_t) -> int32_t {
    if x & 1 as int32_t != 0 {
        return (x + 1 as int32_t) / 2 as int32_t;
    }
    return x / 2 as int32_t;
}
#[no_mangle]
pub unsafe extern "C" fn mult_and_add(
    mut n: int32_t,
    mut x: scaled_t,
    mut y: scaled_t,
    mut max_answer: scaled_t,
) -> scaled_t {
    if n < 0 as int32_t {
        x = -x as scaled_t;
        n = -n;
    }
    if n == 0 as int32_t {
        return y;
    } else if x <= (max_answer - y) / n as scaled_t && -x <= (max_answer + y) / n as scaled_t {
        return n as scaled_t * x + y;
    } else {
        arith_error = true_0 != 0;
        return 0 as scaled_t;
    };
}
#[no_mangle]
pub unsafe extern "C" fn x_over_n(mut x: scaled_t, mut n: int32_t) -> scaled_t {
    if n == 0 as int32_t {
        arith_error = true_0 != 0;
        tex_remainder = x;
        return 0 as scaled_t;
    } else {
        if n < 0 as int32_t {
            x = -x as scaled_t;
            n = -n;
            tex_remainder = -tex_remainder as scaled_t;
        }
        if x >= 0 as scaled_t {
            tex_remainder = x % n as scaled_t;
            return x / n as scaled_t;
        } else {
            tex_remainder = -(-x % n) as scaled_t;
            return -(-x / n);
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn xn_over_d(mut x: scaled_t, mut n: int32_t, mut d: int32_t) -> scaled_t {
    let mut positive: bool = false;
    let mut t: int32_t = 0;
    let mut u: int32_t = 0;
    let mut v: int32_t = 0;
    if x >= 0 as scaled_t {
        positive = true_0 != 0;
    } else {
        x = -x as scaled_t;
        positive = false_0 != 0;
    }
    t = (x as ::core::ffi::c_long % 32768 as ::core::ffi::c_long * n as ::core::ffi::c_long)
        as int32_t;
    u = (x as ::core::ffi::c_long / 32768 as ::core::ffi::c_long * n as ::core::ffi::c_long
        + t as ::core::ffi::c_long / 32768 as ::core::ffi::c_long) as int32_t;
    v = ((u % d) as ::core::ffi::c_long * 32768 as ::core::ffi::c_long
        + t as ::core::ffi::c_long % 32768 as ::core::ffi::c_long) as int32_t;
    if (u / d) as ::core::ffi::c_long >= 32768 as ::core::ffi::c_long {
        arith_error = true_0 != 0;
    } else {
        u = (32768 as ::core::ffi::c_long * (u / d) as ::core::ffi::c_long
            + (v / d) as ::core::ffi::c_long) as int32_t;
    }
    if positive {
        tex_remainder = (v % d) as scaled_t;
        return u as scaled_t;
    } else {
        tex_remainder = -(v % d) as scaled_t;
        return -u;
    };
}
#[no_mangle]
pub unsafe extern "C" fn round_xn_over_d(
    mut x: scaled_t,
    mut n: int32_t,
    mut d: int32_t,
) -> scaled_t {
    let mut positive: bool = false;
    let mut t: int32_t = 0;
    let mut u: int32_t = 0;
    let mut v: int32_t = 0;
    if x >= 0 as scaled_t {
        positive = true_0 != 0;
    } else {
        x = -x as scaled_t;
        positive = false_0 != 0;
    }
    t = (x as ::core::ffi::c_long % 32768 as ::core::ffi::c_long * n as ::core::ffi::c_long)
        as int32_t;
    u = (x as ::core::ffi::c_long / 32768 as ::core::ffi::c_long * n as ::core::ffi::c_long
        + t as ::core::ffi::c_long / 32768 as ::core::ffi::c_long) as int32_t;
    v = ((u % d) as ::core::ffi::c_long * 32768 as ::core::ffi::c_long
        + t as ::core::ffi::c_long % 32768 as ::core::ffi::c_long) as int32_t;
    if (u / d) as ::core::ffi::c_long >= 32768 as ::core::ffi::c_long {
        arith_error = true_0 != 0;
    } else {
        u = (32768 as ::core::ffi::c_long * (u / d) as ::core::ffi::c_long
            + (v / d) as ::core::ffi::c_long) as int32_t;
    }
    v = v % d;
    if 2 as int32_t * v >= d {
        u += 1;
    }
    if positive {
        return u as scaled_t;
    } else {
        return -u;
    };
}
unsafe extern "C" fn make_frac(mut p: int32_t, mut q: int32_t) -> int32_t {
    let mut f: int32_t = 0;
    let mut n: int32_t = 0;
    let mut negative: bool = false;
    let mut be_careful: int32_t = 0;
    if p >= 0 as int32_t {
        negative = false_0 != 0;
    } else {
        p = -p;
        negative = true_0 != 0;
    }
    if q <= 0 as int32_t {
        q = -q;
        negative = !negative;
    }
    n = p / q;
    p = p % q;
    if n >= 8 as int32_t {
        arith_error = true_0 != 0;
        if negative {
            return -(0x7fffffff as int32_t);
        } else {
            return 0x7fffffff as int32_t;
        }
    } else {
        n = (n - 1 as int32_t) * 0x10000000 as int32_t;
        f = 1 as ::core::ffi::c_int as int32_t;
        loop {
            be_careful = p - q;
            p = be_careful + p;
            if p >= 0 as int32_t {
                f = f + f + 1 as int32_t;
            } else {
                f = f + f;
                p = p + q;
            }
            if !(f < 0x10000000 as int32_t) {
                break;
            }
        }
        be_careful = p - q;
        if be_careful + p >= 0 as int32_t {
            f = (f as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as int32_t;
        }
        if negative {
            return -(f + n);
        } else {
            return f + n;
        }
    };
}
unsafe extern "C" fn take_frac(mut q: int32_t, mut f: int32_t) -> int32_t {
    let mut p: int32_t = 0;
    let mut negative: bool = false;
    let mut n: int32_t = 0;
    let mut be_careful: int32_t = 0;
    if f >= 0 as int32_t {
        negative = false_0 != 0;
    } else {
        f = -f;
        negative = true_0 != 0;
    }
    if q < 0 as int32_t {
        q = -q;
        negative = !negative;
    }
    if f < 0x10000000 as int32_t {
        n = 0 as ::core::ffi::c_int as int32_t;
    } else {
        n = f / 0x10000000 as int32_t;
        f = f % 0x10000000 as int32_t;
        if q <= 0x7fffffff as int32_t / n {
            n = n * q;
        } else {
            arith_error = true_0 != 0;
            n = 0x7fffffff as ::core::ffi::c_int as int32_t;
        }
    }
    f = f + 0x10000000 as int32_t;
    p = 0x8000000 as ::core::ffi::c_int as int32_t;
    if q < 0x40000000 as int32_t {
        loop {
            if f & 1 as int32_t != 0 {
                p = (p + q) / 2 as int32_t;
            } else {
                p = p / 2 as int32_t;
            }
            f = f / 2 as int32_t;
            if !(f != 1 as int32_t) {
                break;
            }
        }
    } else {
        loop {
            if f & 1 as int32_t != 0 {
                p = p + (q - p) / 2 as int32_t;
            } else {
                p = p / 2 as int32_t;
            }
            f = f / 2 as int32_t;
            if !(f != 1 as int32_t) {
                break;
            }
        }
    }
    be_careful = n - 0x7fffffff as int32_t;
    if be_careful + p > 0 as int32_t {
        arith_error = true_0 != 0;
        n = 0x7fffffff as int32_t - p;
    }
    if negative {
        return -(n + p);
    } else {
        return n + p;
    };
}
unsafe extern "C" fn m_log(mut x: int32_t) -> int32_t {
    let mut y: int32_t = 0;
    let mut z: int32_t = 0;
    let mut k: int32_t = 0;
    if x <= 0 as int32_t {
        error_here_with_diagnostic(b"Logarithm of \0" as *const u8 as *const ::core::ffi::c_char);
        print_scaled(x as scaled_t);
        print_cstr(b" has been replaced by 0\0" as *const u8 as *const ::core::ffi::c_char);
        capture_to_diagnostic(::core::ptr::null_mut::<ttbc_diagnostic_t>());
        help_ptr = 2 as ::core::ffi::c_uchar;
        help_line[1 as ::core::ffi::c_int as usize] =
            b"Since I don't take logs of non-positive numbers,\0" as *const u8
                as *const ::core::ffi::c_char;
        help_line[0 as ::core::ffi::c_int as usize] =
            b"I'm zeroing this one. Proceed, with fingers crossed.\0" as *const u8
                as *const ::core::ffi::c_char;
        error();
        return 0 as int32_t;
    } else {
        y = 1302456860 as int32_t;
        z = 6581195 as int32_t;
        while x < 0x40000000 as int32_t {
            x = x + x;
            y = (y as ::core::ffi::c_long - 93032639 as ::core::ffi::c_long) as int32_t;
            z = (z as ::core::ffi::c_long - 48782 as ::core::ffi::c_long) as int32_t;
        }
        y = (y as ::core::ffi::c_long + z as ::core::ffi::c_long / 65536 as ::core::ffi::c_long)
            as int32_t;
        k = 2 as ::core::ffi::c_int as int32_t;
        while x > 0x40000004 as int32_t {
            z = (x - 1 as int32_t) / two_to_the[k as usize] + 1 as int32_t;
            while x < 0x40000000 as int32_t + z {
                z = (z + 1 as int32_t) / 2 as int32_t;
                k = k + 1 as int32_t;
            }
            y = y + spec_log[k as usize];
            x = x - z;
        }
        return y / 8 as int32_t;
    };
}
unsafe extern "C" fn ab_vs_cd(
    mut a: int32_t,
    mut b: int32_t,
    mut c: int32_t,
    mut d: int32_t,
) -> int32_t {
    let mut q: int32_t = 0;
    let mut r: int32_t = 0;
    if a < 0 as int32_t {
        a = -a;
        b = -b;
    }
    if c < 0 as int32_t {
        c = -c;
        d = -d;
    }
    if d <= 0 as int32_t {
        if b >= 0 as int32_t {
            if (a == 0 as int32_t || b == 0 as int32_t) && (c == 0 as int32_t || d == 0 as int32_t)
            {
                return 0 as int32_t;
            } else {
                return 1 as int32_t;
            }
        }
        if d == 0 as int32_t {
            if a == 0 as int32_t {
                return 0 as int32_t;
            } else {
                return -(1 as int32_t);
            }
        }
        q = a;
        a = c;
        c = q;
        q = -b;
        b = -d;
        d = q;
    } else if b <= 0 as int32_t {
        if b < 0 as int32_t {
            if a > 0 as int32_t {
                return -(1 as int32_t);
            }
        }
        if c == 0 as int32_t {
            return 0 as int32_t;
        } else {
            return -(1 as int32_t);
        }
    }
    loop {
        q = a / d;
        r = c / b;
        if q != r {
            if q > r {
                return 1 as int32_t;
            } else {
                return -(1 as int32_t);
            }
        }
        q = a % d;
        r = c % b;
        if r == 0 as int32_t {
            if q == 0 as int32_t {
                return 0 as int32_t;
            } else {
                return 1 as int32_t;
            }
        }
        if q == 0 as int32_t {
            return -(1 as int32_t);
        }
        a = b;
        b = q;
        c = d;
        d = r;
    }
}
unsafe extern "C" fn new_randoms() {
    let mut k: ::core::ffi::c_uchar = 0;
    let mut x: int32_t = 0;
    k = 0 as ::core::ffi::c_uchar;
    while (k as ::core::ffi::c_int) < 24 as ::core::ffi::c_int {
        x = randoms[k as usize]
            - randoms[(k as ::core::ffi::c_int + 31 as ::core::ffi::c_int) as usize];
        if x < 0 as int32_t {
            x = x + 0x10000000 as int32_t;
        }
        randoms[k as usize] = x;
        k = k.wrapping_add(1);
    }
    k = 24 as ::core::ffi::c_uchar;
    while (k as ::core::ffi::c_int) < 55 as ::core::ffi::c_int {
        x = randoms[k as usize]
            - randoms[(k as ::core::ffi::c_int - 24 as ::core::ffi::c_int) as usize];
        if x < 0 as int32_t {
            x = x + 0x10000000 as int32_t;
        }
        randoms[k as usize] = x;
        k = k.wrapping_add(1);
    }
    j_random = 54 as ::core::ffi::c_uchar;
}
#[no_mangle]
pub unsafe extern "C" fn init_randoms(mut seed: int32_t) {
    let mut j: int32_t = 0;
    let mut jj: int32_t = 0;
    let mut k: int32_t = 0;
    let mut i: ::core::ffi::c_uchar = 0;
    j = abs(seed as ::core::ffi::c_int) as int32_t;
    while j >= 0x10000000 as int32_t {
        j = j / 2 as int32_t;
    }
    k = 1 as ::core::ffi::c_int as int32_t;
    i = 0 as ::core::ffi::c_uchar;
    while (i as ::core::ffi::c_int) < 55 as ::core::ffi::c_int {
        jj = k;
        k = j - k;
        j = jj;
        if k < 0 as int32_t {
            k = k + 0x10000000 as int32_t;
        }
        randoms[(i as ::core::ffi::c_int * 21 as ::core::ffi::c_int % 55 as ::core::ffi::c_int)
            as usize] = j;
        i = i.wrapping_add(1);
    }
    new_randoms();
    new_randoms();
    new_randoms();
}
#[no_mangle]
pub unsafe extern "C" fn unif_rand(mut x: int32_t) -> int32_t {
    let mut y: int32_t = 0;
    if j_random as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        new_randoms();
    } else {
        j_random = j_random.wrapping_sub(1);
    }
    y = take_frac(
        abs(x as ::core::ffi::c_int) as int32_t,
        randoms[j_random as usize],
    );
    if y == abs(x as ::core::ffi::c_int) as int32_t {
        return 0 as int32_t;
    } else if x > 0 as int32_t {
        return y;
    } else {
        return -y;
    };
}
#[no_mangle]
pub unsafe extern "C" fn norm_rand() -> int32_t {
    let mut x: int32_t = 0;
    let mut u: int32_t = 0;
    let mut l: int32_t = 0;
    loop {
        loop {
            if j_random as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                new_randoms();
            } else {
                j_random = j_random.wrapping_sub(1);
            }
            x = take_frac(
                112429 as int32_t,
                randoms[j_random as usize] - 0x8000000 as int32_t,
            );
            if j_random as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                new_randoms();
            } else {
                j_random = j_random.wrapping_sub(1);
            }
            u = randoms[j_random as usize];
            if !(abs(x as ::core::ffi::c_int) as int32_t >= u) {
                break;
            }
        }
        x = make_frac(x, u);
        l = (139548960 as ::core::ffi::c_long - m_log(u) as ::core::ffi::c_long) as int32_t;
        if !(ab_vs_cd(1024 as int32_t, l, x, x) < 0 as int32_t) {
            break;
        }
    }
    return x;
}
pub const true_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
