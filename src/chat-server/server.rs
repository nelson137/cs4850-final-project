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
    CmdResult, UsersDao, COMMAND_MAX, COMMAND_SEP, REPLY_FLAG_ERR,
    REPLY_FLAG_OK,
};
use tracing::{debug, info};

pub struct TcpServer {
    sock: ServerSocket,
    users: UsersDao,
}

/// Wrapper type that manages server-side networking.
///
/// The only provided method is `main_loop()` which runs the server, accepting
/// connections and processing commands from the client.
impl TcpServer {
    pub fn new(port: u16, users: UsersDao) -> MyResult<Self> {
        let sock = ServerSocket::new()?;
        let mut addr = SockAddr::new(port);
        sock.bind(&mut addr)?;
        sock.listen()?;
        debug!(sock=%sock.display(), "created server socket");
        Ok(Self { sock, users })
    }

    //==================================================
    // Main loop
    //==================================================

    /// Run the server.
    pub fn main_loop(&mut self) -> MyResult<()> {
        let should_stop = Arc::new(AtomicBool::new(false));
        setup_int_handler(&should_stop)?;

        // Sleep after each loop iter to prevent CPU overload
        let delay = Duration::from_millis(25);

        let mut client = None;
        let mut client_quit = false;

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
                    debug!(sock = %c.display(), "client connected");
                    client.insert(Client::new(c))
                }
            };

            if !c.sock.poll(POLLIN)? {
                thread::sleep(delay);
                continue;
            }

            let reply = match self.handle_connection(c, &mut client_quit)? {
                _ if client_quit => {
                    // Drop and close client socket
                    client.take();
                    continue;
                }
                Ok(msg) => format!("{}{}", REPLY_FLAG_OK as char, msg),
                Err(msg) => format!("{}{}", REPLY_FLAG_ERR as char, msg),
            };
            debug!(?reply);
            c.sock.send(reply)?;
        }

        Ok(())
    }

    /// Parse and process a command from the client and return the response or
    /// an error, and whether the client has quit.
    ///
    /// Note: this method will block if no message is waiting to be read.
    fn handle_connection(
        &mut self,
        client: &mut Client,
        quit: &mut bool,
    ) -> CmdResult {
        let cmd = client.sock.recv(COMMAND_MAX)?;
        debug!(%cmd);
        let cmd: Vec<_> = cmd.split(COMMAND_SEP).collect();
        if cmd.is_empty() {
            return Ok(Err("command contains no separators".to_string()));
        }

        match cmd.as_slice() {
            ["quit"] => {
                *quit = true;
                Ok(Ok(String::default()))
            }
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

    /// Invoke the newuser command.
    ///
    /// This command can only be called when **not** logged in.
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
                info!(name = user, "created user account");
                Ok(Ok(format!("user account created: {}", user)))
            } else {
                Ok(Err(format!("user already exists: {}", user)))
            }
        }
    }

    /// Invoke the login command.
    ///
    /// This command can only be called when **not** logged in.
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
                    info!(name = ?user, "user login");
                    Ok(Ok(format!("{} joined the room.", user)))
                }
                _ => Ok(Err("incorrect username or password".to_string())),
            }
        }
    }

    /// Invoke the logout command.
    ///
    /// This command can only be called when logged in.
    fn cmd_logout(&self, client: &mut Client) -> CmdResult {
        match client.logout() {
            Some(user) => {
                info!(name = ?user, "user logout");
                Ok(Ok(format!("{} left the room.", user)))
            }
            None => Ok(Err("you must be logged in to logout".to_string())),
        }
    }

    /// Invoke the send command.
    ///
    /// This command can only be called when logged in.
    fn cmd_send(&self, client: &Client, msg: &str) -> CmdResult {
        if let Some(user) = &client.username {
            info!(name = ?user, msg = msg, "user send");
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

/// Represent a client.
///
/// This type contains the open socket for the client and the client's username,
/// if logged in.
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

    /// Return whether this client is logged in.
    #[inline]
    fn is_logged_in(&self) -> bool {
        self.username.is_some()
    }

    /// Update this client's state to be logged in.
    #[inline]
    fn login(&mut self, user: impl AsRef<str>) {
        self.username = Some(user.as_ref().to_string());
    }

    /// Update this client's state to be logged out.
    ///
    /// The username is returned if this client was logged in, otherwise `None`.
    #[inline]
    fn logout(&mut self) -> Option<String> {
        self.username.take()
    }
}
