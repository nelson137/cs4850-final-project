use std::ffi::CStr;

use libc::{self, __errno_location, strerror_r, EINTR};
use num_traits::{PrimInt, Unsigned};

/// Invoke the given function, which may set `errno`.
///
/// `errno` is set to `0` before the function call and is checked afterwards. If
/// the value is zero, return an `Ok` of the function's return value; otherwise
/// return an `Err` of a string description of the error.
#[inline]
pub fn errno_wrapper<Ret>(func: impl FnOnce() -> Ret) -> Result<Ret, String> {
    unsafe {
        let errno = __errno_location();
        *errno = 0;

        let ret = func();

        if *errno == 0 {
            Ok(ret)
        } else {
            let mut err_str_buf = [0_i8; 256];
            strerror_r(*errno, err_str_buf.as_mut_ptr(), err_str_buf.len());
            Err(CStr::from_ptr(err_str_buf.as_ptr())
                .to_string_lossy()
                .to_string())
        }
    }
}

/// Return whether the current value of `errno` is `EINTR` (Error INTeRrupt).
///
/// In most cases, an `EINTR` should be treated the same as any other error
/// condition. However, in special instances, it may be useful to know whether
/// the error was caused by a user interrupt (^C) and handle this accordingly.
/// For example, in an event loop that has signal handling for `EINT`.
#[inline]
pub fn errno_was_intr() -> bool {
    unsafe { *__errno_location() == EINTR }
}

/// Convert any unsigned int type from host byte order to network byte order.
#[inline]
pub fn hton<U: PrimInt + Unsigned>(u: U) -> U {
    u.to_be()
}
