use std::io;

use num_traits::{PrimInt, Unsigned};

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
