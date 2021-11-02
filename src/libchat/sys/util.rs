use std::ffi::CStr;

use libc::{self, __errno_location, strerror_r};
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

/// Convert any unsigned int type from host byte order to network byte order.
#[inline]
pub fn hton<U: PrimInt + Unsigned>(u: U) -> U {
    u.to_be()
}
