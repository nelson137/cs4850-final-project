use libchat::{
    err::MyResult,
    sys::{SockAddr, Socket},
    MSG_MAX,
};

pub struct SocketServer {
    sock: Socket,
}

impl SocketServer {
    pub fn new(port: u16) -> MyResult<Self> {
        let sock = Socket::new()?;
        let mut addr = SockAddr::new(port);
        sock.bind(&mut addr)?;
        sock.listen()?;
        Ok(Self { sock })
    }

    pub fn run(&self) -> MyResult<()> {
        let conn = self.sock.accept()?;
        print!("{}", conn.recv(MSG_MAX)?);
        Ok(())
    }
}
