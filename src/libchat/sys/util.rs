use std::io;

use libc::{self, c_int, c_short, pollfd};
use num_traits::{PrimInt, Unsigned};

use crate::err::MyResult;

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

/// Wrapper for `poll()`.
pub fn poll(fd: c_int, events: c_short) -> MyResult<bool> {
    let mut poll_fds = [pollfd {
        fd,
        events,
        revents: 0,
    }];

    let n_ready = unsafe { libc::poll(poll_fds.as_mut_ptr(), 1, 0) };

    if n_ready < 0 {
        let err = io::Error::last_os_error();
        Err(format!("failed to poll: {}", err).into())
    } else {
        Ok(n_ready > 0)
    }
}
