use std::fmt::Display;

use tracing::trace;

use libchat::{
    err::MyResult,
    sys::{ClientSocket, SockAddr, SocketCommon},
    CmdResult, COMMAND_SEP, MSG_MAX, RESPONSE_FLAG_ERR, RESPONSE_FLAG_OK,
};

pub struct TcpClient {
    pub sock: ClientSocket,
}

impl TcpClient {
    pub fn new(port: u16) -> MyResult<Self> {
        let sock = ClientSocket::new()?;
        let mut addr = SockAddr::new(port);
        sock.connect(&mut addr)?;
        Ok(Self { sock })
    }

    pub fn send_cmd<'a>(&self, parts: impl AsRef<[&'a str]>) -> MyResult<()> {
        let parts = parts.as_ref();
        let mut server_cmd = parts[0].to_string();
        for &arg in &parts[1..] {
            server_cmd += COMMAND_SEP;
            server_cmd += arg;
        }
        self.sock.send(server_cmd)?;
        Ok(())
    }

    pub fn recv_reply(&self) -> CmdResult {
        let reply = self.sock.recv(MSG_MAX)?;
        trace!(msg = ?reply, "server response");
        match reply.as_bytes() {
            [RESPONSE_FLAG_OK, rest @ ..] => {
                Ok(Ok(String::from_utf8_lossy(rest).to_string()))
            }
            [RESPONSE_FLAG_ERR, rest @ ..] => {
                Ok(Err(String::from_utf8_lossy(rest).to_string()))
            }
            // This should never happen
            [f, ..] => {
                Err(format!("server reply starts with invalid byte: {:?}", f)
                    .into())
            }
            // This should never happen
            [] => Err("server reply is empty string".to_string().into()),
        }
    }
}

impl Display for TcpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.sock.display())
    }
}
