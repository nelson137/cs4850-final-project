use std::io;

use num_traits::{PrimInt, Unsigned};

/// Invoke the given function, which may set `errno`.
///
/// `errno` is set to `0` before the function call and is checked afterwards. If
/// the value is zero, return an `Ok` of the function's return value; otherwise
/// return an `Err` of a string description of the error.
#[inline]
pub fn errno_wrapper<Ret>(func: impl FnOnce() -> Ret) -> Result<Ret, String> {
    let ret = func();

    let err = io::Error::last_os_error();

    // SAFETY: Error::raw_os_error() returns `Some` iff the `Error` is an OS
    //         error, which is guaranteed by `Error::last_os_error()`
    if err.raw_os_error().unwrap() == 0 {
        Ok(ret)
    } else {
        Err(err.to_string())
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
    io::Error::last_os_error().kind() == io::ErrorKind::Interrupted
}

/// Convert any unsigned int type from host byte order to network byte order.
#[inline]
pub fn hton<U: PrimInt + Unsigned>(u: U) -> U {
    u.to_be()
}
