use libchat::{
    err::MyResult,
    sys::{SockAddr, Socket},
};

pub struct SocketClient {
    sock: Socket,
}

impl SocketClient {
    pub fn new(port: u16) -> MyResult<Self> {
        let sock = Socket::new()?;
        let mut addr = SockAddr::new(port);
        sock.connect(&mut addr)?;
        Ok(Self { sock })
    }

    pub fn run(&self) -> MyResult<()> {
        // Must send() no more than MSG_MAX-1 bytes
        self.sock.send("hello from client")?;
        Ok(())
    }
}
