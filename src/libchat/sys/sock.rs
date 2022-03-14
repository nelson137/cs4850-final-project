use std::{
    ffi::{CStr, CString},
    fmt::{self, Display},
    io,
    mem::size_of,
};

use libc::{
    accept, bind, c_int, c_short, c_void, close, connect, in_addr, listen,
    poll, pollfd, read, setsockopt, sockaddr, sockaddr_in, socket, write,
    AF_INET, INADDR_LOOPBACK, SOCK_STREAM, SOL_SOCKET, SO_REUSEADDR,
};
use tracing::debug;

use super::hton;

use crate::{err::MyResult, LISTEN_BACKLOG, MSG_MAX};

macro_rules! SIZEOF {
    ($ty:ty) => {
        size_of::<$ty>() as u32
    };
}

//==============================================================================
// Common
//==============================================================================

/// Represent a socket address.
///
/// Utility methods are provided for easily passing this struct into socket API
/// function calls.
pub struct SockAddr {
    // Array has a method for casting to a mutable pointer, so use
    // single-element array so it's easy to get a pointer to the data.
    addr: [sockaddr_in; 1],
}

impl SockAddr {
    /// Create a new `SockAddr` describing any address and the given port.
    pub fn new(port: u16) -> Self {
        Self {
            #[cfg(target_os = "linux")]
            addr: [sockaddr_in {
                sin_family: AF_INET as u16,
                sin_port: hton(port),
                sin_addr: in_addr {
                    s_addr: hton(INADDR_LOOPBACK),
                },
                sin_zero: [0; 8],
            }],
            #[cfg(target_os = "macos")]
            addr: [sockaddr_in {
                sin_len: 0,
                sin_family: AF_INET as u8,
                sin_port: hton(port),
                sin_addr: in_addr {
                    s_addr: hton(INADDR_LOOPBACK),
                },
                sin_zero: [0; 8],
            }],
        }
    }

    /// Create a new empty `SockAddr`.
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

/// An interface for performing common socket operations.
///
/// Implement syscall wrappers for socket operations that can be used on both
/// the client-side and server-side are provided, including:
/// - `close()`
/// - `poll()`
/// - `send()`
/// - `recv()`
pub trait SocketCommon: From<c_int> {
    /// Create a socket and return its file descriptor.
    ///
    /// **For internal use only.**
    fn _create_raw() -> MyResult<c_int> {
        let fd = unsafe { socket(AF_INET, SOCK_STREAM, 0) };
        if fd < 0 {
            let err = io::Error::last_os_error();
            Err(format!("failed to create socket: {}", err).into())
        } else {
            Ok(fd)
        }
    }

    /// Return the file descriptor of this socket.
    fn fd(&self) -> c_int;

    /// Return an object that implements `Display` (i.e. can be printed).
    fn display(&self) -> SocketDisplay;

    /// Close this socket.
    ///
    /// Note: all types that implement the `SocketCommon` trait also implement
    /// `Drop` (i.e. the socket will be closed when the object goes out of
    /// scope).
    fn close(&self) {
        debug!(sock=%self.display(), "closing socket");
        unsafe {
            close(self.fd());
        }
    }

    /// Wrapper method that calls `poll()` on this socket.
    fn poll(&self, events: c_short) -> MyResult<bool> {
        let mut poll_fds = [pollfd {
            fd: self.fd(),
            events,
            revents: 0,
        }];

        let n_ready = unsafe { poll(poll_fds.as_mut_ptr(), 1, 0) };

        if n_ready < 0 {
            let err = io::Error::last_os_error();
            Err(format!("failed to poll: {}", err).into())
        } else {
            Ok(n_ready > 0)
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

        if unsafe { write(self.fd(), buf, size) < 0 } {
            let err = io::Error::last_os_error();
            Err(format!("failed to send(): {}", err).into())
        } else {
            Ok(())
        }
    }

    /// Wrapper for socket API `recv()`.
    fn recv(&self, size: usize) -> MyResult<String> {
        let mut buf = vec![0_u8; size];
        let buf_ptr = buf.as_mut_ptr() as *mut c_void;

        let n_bytes = unsafe { read(self.fd(), buf_ptr, size - 1) };
        if n_bytes < 0 {
            let err = io::Error::last_os_error();
            return Err(format!("failed to recv(): {}", err).into());
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

/// Store the necesssary data and implement `Display` such that this socket can
/// be formatted nicely and printed.
pub struct SocketDisplay {
    fd: c_int,
}

impl SocketDisplay {
    fn new(fd: c_int) -> Self {
        Self { fd }
    }
}

impl Display for SocketDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{}", self.fd))
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
/// - `bind()`
/// - `listen()`
/// - `accept()`
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
        SocketDisplay::new(self.sock)
    }
}

impl ServerSocket {
    pub fn new() -> MyResult<Self> {
        let fd = Self::_create_raw()?;

        // Set SO_REUSEADDR so a bind() doesn't fail on a socket that is in
        // the CLOSE_WAIT state.
        let value = [1 as c_int];
        let value_ptr = value.as_ptr() as *const c_void;
        let ret = unsafe {
            setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, value_ptr, SIZEOF!(c_int))
        };

        if ret < 0 {
            let err = io::Error::last_os_error();
            Err(format!("failed to set socket option SO_REUSEADDR: {}", err)
                .into())
        } else {
            Ok(fd.into())
        }
    }

    /// Wrapper for socket API `bind()`.
    pub fn bind(&self, addr: &mut SockAddr) -> MyResult<()> {
        let size = SIZEOF!(sockaddr_in);
        if unsafe { bind(self.sock, addr.as_mut_ptr(), size) < 0 } {
            let err = io::Error::last_os_error();
            Err(format!("failed to bind(): {}", err).into())
        } else {
            Ok(())
        }
    }

    /// Wrapper for socket API `listen()`.
    pub fn listen(&self) -> MyResult<()> {
        if unsafe { listen(self.sock, LISTEN_BACKLOG) < 0 } {
            let err = io::Error::last_os_error();
            Err(format!("failed to listen(): {}", err).into())
        } else {
            Ok(())
        }
    }

    /// Wrapper for socket API `accept()`.
    pub fn accept(&self) -> MyResult<Self> {
        let mut addr = SockAddr::zero();
        // Use single-element array because it provides a method for
        // converting to a mutable pointer.
        let mut size = [SIZEOF!(sockaddr_in)];

        let fd =
            unsafe { accept(self.sock, addr.as_mut_ptr(), size.as_mut_ptr()) };

        if fd < 0 {
            let err = io::Error::last_os_error();
            Err(format!("failed to accept(): {}", err).into())
        } else {
            debug!(sock = fd, "accepted client");
            Ok(fd.into())
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
/// - `connect()`
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
        SocketDisplay::new(self.sock)
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
