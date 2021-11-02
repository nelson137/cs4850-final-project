use std::{
    ffi::{CStr, CString},
    fmt::{self, Display},
    mem::size_of,
};

use libc::{
    __errno_location, accept, bind, c_int, c_short, c_void, close, connect,
    in_addr, listen, poll, pollfd, read, sockaddr, sockaddr_in, socket,
    strerror_r, write, AF_INET, INADDR_ANY, SOCK_STREAM,
};
use tracing::info;

use super::hton;

use crate::{err::MyResult, MSG_MAX};

macro_rules! SIZEOF {
    ($ty:ty) => {
        size_of::<$ty>() as u32
    };
}

//==============================================================================
// Common
//==============================================================================

/// Represent a socket address suitable for use with `Socket`.
pub struct SockAddr {
    // Array has a method for casting to a mutable pointer, so use
    // single-element array to make ref->ptr cast easy
    addr: [sockaddr_in; 1],
}

impl SockAddr {
    /// Create a new SockAddr describing any address and the given port.
    pub fn new(port: u16) -> Self {
        Self {
            addr: [sockaddr_in {
                sin_family: AF_INET as u16,
                sin_port: hton(port),
                sin_addr: in_addr {
                    s_addr: hton(INADDR_ANY),
                },
                sin_zero: [0; 8],
            }],
        }
    }

    /// Create a new empty SockAddr.
    ///
    /// Use this when a buffer is needed.
    pub fn zero() -> Self {
        Self::new(0)
    }

    /// Return a pointer suitable for use in socket API functions.
    pub fn as_mut_ptr(&mut self) -> *mut sockaddr {
        self.addr.as_mut_ptr() as *mut sockaddr
    }
}

/// An interface for performing socket operations.
///
/// Implement syscall wrappers for socket operations that can be used on both
/// the client-side and server-side are provided, including:
/// - close
/// - poll
/// - send
/// - recv
pub trait SocketCommon: From<c_int> {
    fn _create_raw() -> MyResult<c_int> {
        unsafe {
            let sock = socket(AF_INET, SOCK_STREAM, 0);
            if sock < 0 {
                Err("failed to create socket".to_string().into())
            } else {
                Ok(sock)
            }
        }
    }

    fn fd(&self) -> c_int;

    fn display(&self) -> SocketDisplay;

    fn close(&self) {
        info!(sock=%self.display(), "closing socket");
        unsafe {
            close(self.fd());
        }
    }

    fn poll(&self, events: c_short) -> MyResult<bool> {
        unsafe {
            let fd = self.fd();

            let mut poll_fds = [pollfd {
                fd,
                events,
                revents: 0,
            }];

            let errno_loc = __errno_location();
            *errno_loc = 0;

            match poll(poll_fds.as_mut_ptr(), 1, 0) {
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

    /// Wrapper for socket API `send()`.
    fn send(&self, msg: impl AsRef<str>) -> MyResult<()> {
        // Make copy of msg and ensure it is null-terminated
        let msg = CString::new(msg.as_ref())?;

        let buf = msg.as_ptr() as *const c_void;
        let size = msg.as_bytes_with_nul().len();
        if size > MSG_MAX {
            return Err(
                format!("message too long: {} > {}", size, MSG_MAX).into()
            );
        }

        unsafe {
            if write(self.fd(), buf, size) as usize == size {
                Ok(())
            } else {
                Err("failed to send message".to_string().into())
            }
        }
    }

    /// Wrapper for socket API `recv()`.
    fn recv(&self, size: usize) -> MyResult<String> {
        // Create a buffer with `size` bytes initialized to 0
        let mut buf = vec![0_u8; size];
        let buf_ptr = buf.as_mut_ptr() as *mut c_void;
        // Call recv()
        unsafe {
            if read(self.fd(), buf_ptr, size - 1) < 0 {
                return Err("failed to receive message from client"
                    .to_string()
                    .into());
            }
        }
        // Make sure buffer is null-terminated just in case it gets completely
        // filled. This should never happen because the buffer is
        // zero-initialized and the length given to recv() was size-1 so the
        // last byte shouldn't be overwritten.
        buf[size - 1] = 0;

        // Convert message buffer to owned string:
        // - Get the size of the buffer contents with one null byte at the end.
        //   This is important because CStr considers the entire given value as
        //   a string, so if there are extra null bytes at the end (i.e. the
        //   buffer only gets partially filled) it will fail because of
        //   "interior null bytes".
        // - Get a slice of the buffer *with the terminating null byte*.
        // - Attempt conversion from slice to CStr. This will fail if the buffer
        //   contains invalid UTF-8 characters.
        let len = buf.iter().position(|&c| c == 0).unwrap_or(size - 1) + 1;
        let terminated_buf = buf.iter().cloned().take(len).collect::<Vec<_>>();
        let msg = CStr::from_bytes_with_nul(&terminated_buf)?;
        Ok(msg.to_str()?.to_string())
    }
}

pub struct SocketDisplay<'a> {
    name: &'a str,
    fd: c_int,
}

impl<'a> SocketDisplay<'a> {
    fn new(name: &'a str, fd: c_int) -> Self {
        Self { name, fd }
    }
}

impl Display for SocketDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{}{{{}}}", self.name, self.fd))
    }
}

//==============================================================================
// Server
//==============================================================================

/// A `SocketCommon` wrapper for a server-side socket.
///
/// Calls `close()` when dropped.
///
/// Implement syscall wrappers for server-side socket operations, including:
/// - bind
/// - listen
/// - accept
pub struct ServerSocket {
    sock: c_int,
}

impl Drop for ServerSocket {
    fn drop(&mut self) {
        self.close();
    }
}

impl From<c_int> for ServerSocket {
    /// Create a new `ServerSocket` from an existing file descriptor.
    fn from(sock: c_int) -> Self {
        Self { sock }
    }
}

impl SocketCommon for ServerSocket {
    #[inline]
    fn fd(&self) -> c_int {
        self.sock
    }

    #[inline]
    fn display(&self) -> SocketDisplay {
        SocketDisplay::new("ServerSocket", self.sock)
    }
}

impl ServerSocket {
    pub fn new() -> MyResult<Self> {
        Ok(Self::_create_raw()?.into())
    }

    /// Wrapper for socket API `bind()`.
    pub fn bind(&self, addr: &mut SockAddr) -> MyResult<()> {
        unsafe {
            match bind(self.sock, addr.as_mut_ptr(), SIZEOF!(sockaddr_in)) {
                0 => Ok(()),
                _ => Err("failed to bind socket".to_string().into()),
            }
        }
    }

    /// Wrapper for socket API `listen()`.
    pub fn listen(&self) -> MyResult<()> {
        unsafe {
            match listen(self.sock, 64) {
                0 => Ok(()),
                _ => Err("failed to listen to socket".to_string().into()),
            }
        }
    }

    /// Wrapper for socket API `accept()`.
    pub fn accept(&self) -> MyResult<Self> {
        unsafe {
            let mut addr = SockAddr::zero();
            // Use single-element array because it provides a method for
            // converting to a mutable pointer.
            let mut size = [SIZEOF!(sockaddr_in)];
            match accept(self.sock, addr.as_mut_ptr(), size.as_mut_ptr()) {
                -1 => Err("failed to accept connection".to_string().into()),
                s => Ok(s.into()),
            }
        }
    }
}

//==============================================================================
// Client
//==============================================================================

/// A `SocketCommon` wrapper for a server-side socket.
///
/// Calls `close()` when dropped.
///
/// Implement syscall wrappers for client-side socket operations, including:
/// - connect
pub struct ClientSocket {
    sock: c_int,
}

impl Drop for ClientSocket {
    fn drop(&mut self) {
        self.close();
    }
}

impl From<c_int> for ClientSocket {
    /// Create a new `ServerSocket` from an existing file descriptor.
    fn from(sock: c_int) -> Self {
        Self { sock }
    }
}

impl SocketCommon for ClientSocket {
    #[inline]
    fn fd(&self) -> c_int {
        self.sock
    }

    #[inline]
    fn display(&self) -> SocketDisplay {
        SocketDisplay::new("ClientSocket", self.sock)
    }
}

impl ClientSocket {
    pub fn new() -> MyResult<Self> {
        Ok(Self::_create_raw()?.into())
    }

    /// Wrapper for socket API `connect()`.
    pub fn connect(&self, addr: &mut SockAddr) -> MyResult<()> {
        unsafe {
            match connect(self.sock, addr.as_mut_ptr(), SIZEOF!(sockaddr_in)) {
                0 => Ok(()),
                _ => Err("failed to connect to socket".to_string().into()),
            }
        }
    }
}
