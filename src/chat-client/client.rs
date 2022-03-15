use std::fmt::Display;

use tracing::trace;

use libchat::{
    err::MyResult,
    sys::{ClientSocket, SockAddr, SocketCommon},
    ServerReply, COMMAND_MAX, COMMAND_SEP, REPLY_FLAG_ERR, REPLY_FLAG_OK,
};

/// Wrapper type that manages client-side networking.
///
/// Methods are provided for sending a command to the server (`send_cmd`) and
/// receiving the reply (`recv_reply`).
///
/// A command invocation must be implemented as one call to `send_cmd()`
/// followed by one call to `recv_reply()`. Special control bytes are used as
/// delimiters for command arguments and reply status and information.
pub struct TcpClient {
    pub sock: ClientSocket,
}

impl TcpClient {
    /// Create a new TCP client which immediately attempts to connect to the
    /// server.
    pub fn new(port: u16) -> MyResult<Self> {
        let sock = ClientSocket::new()?;
        let mut addr = SockAddr::new(port);
        sock.connect(&mut addr)?;
        Ok(Self { sock })
    }

    /// Send the given command to the server.
    ///
    /// The command name and arguments are separated by a special byte that is
    /// expected by the server.
    ///
    /// For example, if the separator is "|" and the command parts are
    /// ["cmd", "arg1", "arg2"], then "cmd|arg1|arg2" is sent. If the command
    /// parts are ["cmd"], then "cmd" is sent with no separators.
    pub fn send_cmd<'a>(&self, parts: impl AsRef<[&'a str]>) -> MyResult<()> {
        self.sock.send(parts.as_ref().join(COMMAND_SEP))
    }

    /// Return the reply from the server indicating whether the previous command
    /// succeeded or failed.
    pub fn recv_reply(&self) -> MyResult<ServerReply> {
        let reply = self.sock.recv(COMMAND_MAX)?;
        trace!(msg = ?reply, "server response");
        match reply.as_bytes() {
            // Received string with Ok flag for first byte
            [REPLY_FLAG_OK, rest @ ..] => {
                Ok(Ok(String::from_utf8_lossy(rest).to_string()))
            }
            // Received string with Error flag for first byte
            [REPLY_FLAG_ERR, rest @ ..] => {
                Ok(Err(String::from_utf8_lossy(rest).to_string()))
            }
            // Received string with first byte being neither Ok nor Error flag
            // This should never happen
            [f, ..] => {
                Err(format!("server reply starts with invalid byte: {:?}", f)
                    .into())
            }
            // Received empty string
            // This should never happen
            [] => Err("no reply".to_string().into()),
        }
    }
}

impl Display for TcpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.sock.display())
    }
}
