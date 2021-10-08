use std::{
    ffi::{CStr, CString},
    iter::repeat,
    mem::size_of,
};

use libc::{
    accept, bind, c_int, c_void, close, connect, in_addr, listen, recv, send,
    sockaddr, sockaddr_in, socket, ssize_t, AF_INET, INADDR_ANY, SOCK_STREAM,
};

use crate::err::{MyError, MyResult};

use super::hton;

macro_rules! SIZEOF {
    ($ty:ty) => {
        size_of::<$ty>() as u32
    };
}

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

/// Socket object with wrappers for socket API functions.
///
/// Calls `close()` when dropped.
///
/// All wrapper functions return a MyResult which contain the return value (if
/// it has meaning) or a MyError with an error message.
///
/// Server-side function support:
/// - bind
/// - listen
/// - accept
/// - recv
///
/// Client-side function support:
/// - connect
/// - send
pub struct Socket {
    sock: c_int,
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe {
            close(self.sock);
        }
        eprintln!("Socket closed");
    }
}

impl Socket {
    /// Create a new socket.
    pub fn new() -> MyResult<Self> {
        unsafe {
            let sock = socket(AF_INET, SOCK_STREAM, 0);
            if sock < 0 {
                Err(MyError::from("failed to create socket"))
            } else {
                eprintln!("Socket opened");
                Ok(Self { sock })
            }
        }
    }

    /// Create a new `Socket` from an existing file descriptor.
    fn from_fd(sock: c_int) -> Self {
        Self { sock }
    }

    /**
     * Server functions
     */

    /// Wrapper for socket API `bind()`.
    pub fn bind(&self, addr: &mut SockAddr) -> MyResult<()> {
        unsafe {
            match bind(self.sock, addr.as_mut_ptr(), SIZEOF!(sockaddr_in)) {
                0 => Ok(()),
                _ => Err(MyError::from("failed to bind socket")),
            }
        }
    }

    /// Wrapper for socket API `listen()`.
    pub fn listen(&self) -> MyResult<()> {
        unsafe {
            match listen(self.sock, 64) {
                0 => Ok(()),
                _ => Err(MyError::from("failed to listen to socket")),
            }
        }
    }

    /// Wrapper for socket API `accept()`.
    pub fn accept(&self) -> MyResult<Socket> {
        unsafe {
            let mut addr = SockAddr::zero();
            // Use single-element array because it provides a method for
            // converting to a mutable pointer.
            let mut size = [SIZEOF!(sockaddr_in)];
            match accept(self.sock, addr.as_mut_ptr(), size.as_mut_ptr()) {
                -1 => Err(MyError::from("failed to accept connection")),
                s => Ok(Socket::from_fd(s)),
            }
        }
    }

    /// Wrapper for socket API `recv()`.
    pub fn recv(&self, size: usize) -> MyResult<String> {
        // Create a buffer with `size` bytes initialized to 0
        let mut buf: Vec<u8> = repeat(0u8).take(size).collect::<Vec<_>>();
        let buf_ptr = buf.as_mut_ptr() as *mut c_void;
        // Call recv()
        unsafe {
            if recv(self.sock, buf_ptr, size - 1, 0) < 0 {
                return Err(MyError::from(
                    "failed to receive message from client",
                ));
            }
        }
        // Make sure buffer is null-terminated just in case it gets completely
        // filled. This should never happen because the buffer is
        // zero-initialized and the length given to recv() was size-1 so it
        // shouldn't overwrite the last byte.
        buf[size - 1] = 0;

        // Convert message buffer to owned string:
        // - Get the size of buffer contents with one null byte at the end. This
        //   is important because CStr considers the entire given value as a
        //   string, so if there are extra null bytes at the end (i.e. the
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

    /**
     * Client functions
     */

    /// Wrapper for socket API `connect()`.
    pub fn connect(&self, addr: &mut SockAddr) -> MyResult<()> {
        unsafe {
            match connect(self.sock, addr.as_mut_ptr(), SIZEOF!(sockaddr_in)) {
                0 => Ok(()),
                _ => Err(MyError::from("failed to connect to socket")),
            }
        }
    }

    /// Wrapper for socket API `send()`.
    pub fn send<S: AsRef<str>>(&self, msg: S) -> MyResult<ssize_t> {
        // Make copy of msg and ensure it is null-terminated
        let msg = CString::new(msg.as_ref())?;

        let buf = msg.as_ptr() as *const c_void;
        let size = msg.as_bytes_with_nul().len();
        unsafe {
            match send(self.sock, buf, size, 0) {
                -1 => Err(MyError::from("failed to send message")),
                bytes_sent => Ok(bytes_sent),
            }
        }
    }
}
