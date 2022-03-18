use tracing::{debug, trace};

use libchat::{
    err::{MyError, MyResult},
    sys::{ClientSocket, SockAddr, SocketCommon},
    ServerReply, COMMAND_MAX, COMMAND_SEP, HANDSHAKE_ACK, REPLY_FLAG_ERR,
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
        let reply = sock.recv(COMMAND_MAX)?;
        debug!(msg = ?reply, "handshake reply");
        if reply == HANDSHAKE_ACK {
            Ok(Self { sock })
        } else {
            Err(MyError::ClientRejected)
        }
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
        let msg = self.sock.recv(COMMAND_MAX)?;
        trace!(msg = ?msg, "server response");
        let msg_b = msg.as_bytes();
        if !msg_b.is_empty() && msg_b[0] == REPLY_FLAG_ERR {
            // Received string with error flag for first byte
            Ok(Err(String::from_utf8_lossy(&msg_b[1..]).to_string()))
        } else {
            // Received non-error string
            Ok(Ok(String::from_utf8_lossy(msg_b).to_string()))
        }
    }
}
