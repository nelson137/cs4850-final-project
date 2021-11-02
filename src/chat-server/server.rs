use std::{
    collections::hash_map::Entry,
    fmt::Display,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use libc::POLLIN;
use libchat::{
    err::MyResult,
    setup_int_handler,
    sys::{ServerSocket, SockAddr, SocketCommon},
    CmdResult, UsersDao, COMMAND_SEP, MSG_MAX, RESPONSE_FLAG_ERR,
    RESPONSE_FLAG_OK,
};
use tracing::{debug, info};

pub struct TcpServer {
    sock: ServerSocket,
    users: UsersDao,
}

impl TcpServer {
    pub fn new(port: u16, users: UsersDao) -> MyResult<Self> {
        let sock = ServerSocket::new()?;
        let mut addr = SockAddr::new(port);
        sock.bind(&mut addr)?;
        sock.listen()?;
        info!(sock=%sock.display(), "created server socket");
        Ok(Self { sock, users })
    }

    //==================================================
    // Main loop
    //==================================================

    pub fn main_loop(&mut self) -> MyResult<()> {
        let should_stop = Arc::new(AtomicBool::new(false));
        setup_int_handler(&should_stop)?;

        // Sleep after each loop iter to prevent CPU overload
        let delay = Duration::from_millis(25);

        let mut client = None;

        loop {
            if should_stop.load(Ordering::Relaxed) {
                break;
            }

            let c = match &mut client {
                Some(c) => c,
                None => {
                    if !self.sock.poll(POLLIN)? {
                        thread::sleep(delay);
                        continue;
                    }
                    let c = self.sock.accept()?;
                    info!(sock = %c.display(), "client connected");
                    client.insert(Client::new(c))
                }
            };

            if !c.sock.poll(POLLIN)? {
                thread::sleep(delay);
                continue;
            }

            let reply = match self.handle_connection(c)? {
                Ok(msg) => format!("{}{}", RESPONSE_FLAG_OK as char, msg),
                Err(msg) => format!("{}{}", RESPONSE_FLAG_ERR as char, msg),
            };
            debug!(?reply, "command handled");
            c.sock.send(reply)?;
        }

        Ok(())
    }

    fn handle_connection(&mut self, client: &mut Client) -> CmdResult {
        let cmd = client.sock.recv(MSG_MAX)?;
        info!(%cmd);
        let cmd: Vec<_> = cmd.split(COMMAND_SEP).collect();
        if cmd.is_empty() {
            return Ok(Err("command contains no separators".to_string()));
        }

        match cmd.as_slice() {
            ["newuser", user, pass] => self.cmd_newuser(client, user, pass),
            ["newuser", rest @ ..] => {
                Ok(Err(format!("expected 2 arguments but got {}", rest.len())))
            }
            ["login", user, pass] => self.cmd_login(client, user, pass),
            ["login", rest @ ..] => {
                Ok(Err(format!("expected 2 arguments but got {}", rest.len())))
            }
            ["logout"] => self.cmd_logout(client),
            ["logout", rest @ ..] => {
                Ok(Err(format!("expected 1 argument but got {}", rest.len())))
            }
            ["send", msg] => self.cmd_send(client, msg),
            _ => return Ok(Err("command not recognized".to_string())),
        }
    }

    //==================================================
    // Commands
    //==================================================

    fn cmd_newuser(
        &mut self,
        client: &Client,
        user: &str,
        pass: &str,
    ) -> CmdResult {
        if client.is_logged_in() {
            Ok(Err(
                "you may not create a new user while logged in".to_string()
            ))
        } else {
            if self.users.insert(user.to_string(), pass.to_string()) {
                Ok(Ok(format!("user account created: {}", user)))
            } else {
                Ok(Err(format!("user already exists: {}", user)))
            }
        }
    }

    fn cmd_login(
        &mut self,
        client: &mut Client,
        user: &str,
        pass: &str,
    ) -> CmdResult {
        if client.is_logged_in() {
            Ok(Err("you are already logged in".to_string()))
        } else {
            match &self.users.entry(user) {
                Entry::Occupied(oe) if oe.get() == pass => {
                    client.login(user);
                    Ok(Ok(format!("{} joined the room.", user)))
                }
                _ => Ok(Err("incorrect username or password".to_string())),
            }
        }
    }

    fn cmd_logout(&self, client: &mut Client) -> CmdResult {
        match client.logout() {
            Some(user) => Ok(Ok(format!("{} left the room.", user))),
            None => Ok(Err("you must be logged in to logout".to_string())),
        }
    }

    fn cmd_send(&self, client: &Client, msg: &str) -> CmdResult {
        if let Some(user) = &client.username {
            Ok(Ok(format!("{}: {}", user, msg)))
        } else {
            Ok(Err("you must be logged in to send".to_string()))
        }
    }
}

impl Display for TcpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.sock.display())
    }
}

struct Client {
    sock: ServerSocket,
    username: Option<String>,
}

impl Client {
    #[inline]
    fn new(sock: ServerSocket) -> Self {
        Self {
            sock,
            username: None,
        }
    }

    #[inline]
    fn is_logged_in(&self) -> bool {
        self.username.is_some()
    }

    #[inline]
    fn login(&mut self, user: impl AsRef<str>) {
        self.username = Some(user.as_ref().to_string());
    }

    #[inline]
    fn logout(&mut self) -> Option<String> {
        self.username.take()
    }
}
