use std::{ffi::CStr, os::raw::c_int};

use libc::{self, __errno_location, c_short, pollfd, strerror_r};
use num_traits::{PrimInt, Unsigned};

use crate::err::MyResult;

/// Convert any unsigned int type from host byte order to network byte order.
pub fn hton<U: PrimInt + Unsigned>(u: U) -> U {
    u.to_be()
}

pub fn poll(fd: c_int, events: c_short) -> MyResult<bool> {
    let mut poll_fds = [pollfd {
        fd,
        events,
        revents: 0,
    }];

    unsafe {
        let errno_loc = __errno_location();
        *errno_loc = 0;

        match libc::poll(poll_fds.as_mut_ptr(), 1, 0) {
            1.. => Ok(true),
            0 => Ok(false),
            _ => {
                let mut err_str_buf = [0_i8; 256];
                strerror_r(
                    *errno_loc,
                    err_str_buf.as_mut_ptr(),
                    err_str_buf.len(),
                );
                let err_str =
                    CStr::from_ptr(err_str_buf.as_ptr()).to_string_lossy();
                Err(format!("failed to poll fd {}: {}", fd, err_str).into())
            }
        }
    }
}
