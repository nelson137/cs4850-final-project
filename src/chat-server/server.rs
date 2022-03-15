use std::{
    collections::hash_map::Entry,
    fmt,
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
    sys::{errno_was_intr, ServerSocket, SockAddr, SocketCommon},
    UsersDao, COMMAND_MAX, COMMAND_SEP, REPLY_FLAG_ERR, REPLY_FLAG_OK,
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

        let mut maybe_client = None;
        let mut client_logout = false;

        loop {
            if should_stop.load(Ordering::Relaxed) {
                break;
            }

            match self.sock.poll(POLLIN) {
                Ok(has_incoming) if has_incoming => {
                    maybe_client
                        .get_or_insert(Client::new(self.sock.accept()?));
                }
                err @ Err(_) => {
                    if errno_was_intr() {
                        break;
                    } else {
                        // `Result::and()` must be used to convert
                        // `Result<bool, _>` to a `Result<(), _>`, the return
                        // type, even though we know the value is an `Err`.
                        return err.and(Ok(()));
                    }
                }
                _ => (),
            }

            let client = if let Some(c) = &mut maybe_client {
                c
            } else {
                thread::sleep(delay);
                continue;
            };

            self.handle_connection(client, &mut client_logout)?;

            if client_logout {
                // Drop and close client socket.
                maybe_client.take();
            }
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
        logout: &mut bool,
    ) -> MyResult<()> {
        let cmd = client.sock.recv(COMMAND_MAX)?;
        debug!(sock = %client.sock.display(), ?cmd, "received command");

        let cmd: Vec<_> = cmd.split(COMMAND_SEP).collect();
        if cmd.is_empty() {
            return Err("command contains no separators".to_string().into());
        }

        macro_rules! reply_invalid_num_args {
            ($expected:expr, $actual:expr) => {
                client.reply_err(format!(
                    "expected {} arguments but got {}",
                    $expected, $actual
                ))
            };
        }

        match cmd.as_slice() {
            ["newuser", user, pass] => self.cmd_newuser(client, user, pass),
            ["newuser", rest @ ..] => reply_invalid_num_args!(2, rest.len()),

            ["login", user, pass] => self.cmd_login(client, user, pass),
            ["login", rest @ ..] => reply_invalid_num_args!(2, rest.len()),

            ["logout"] => {
                *logout = true;
                self.cmd_logout(client)
            }
            ["logout", rest @ ..] => reply_invalid_num_args!(0, rest.len()),

            ["send", msg] => self.cmd_send(client, msg),
            ["send", rest @ ..] => reply_invalid_num_args!(2, rest.len()),

            _ => {
                client.reply_err(format!("command not recognized: {}", cmd[0]))
            }
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
    ) -> MyResult<()> {
        if client.is_logged_in() {
            client.reply_err("you may not create a new user while logged in")
        } else {
            if self.users.insert(user.to_string(), pass.to_string()) {
                info!(name = user, "created user account");
                client.reply_ok(format!("user account created: {}", user))
            } else {
                client.reply_err(format!("user already exists: {}", user))
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
    ) -> MyResult<()> {
        if client.is_logged_in() {
            client.reply_err("you are already logged in")
        } else {
            match &self.users.entry(user) {
                Entry::Occupied(oe) if oe.get() == pass => {
                    client.login(user);
                    info!(name = ?user, "user login");
                    client.reply_ok(format!("{} joined the room.", user))
                }
                _ => client.reply_err("incorrect username or password"),
            }
        }
    }

    /// Invoke the logout command.
    ///
    /// This command can only be called when logged in.
    fn cmd_logout(&self, client: &mut Client) -> MyResult<()> {
        match client.logout() {
            Some(user) => {
                info!(name = ?user, "user logout");
                client.reply_ok(format!("{} left the room.", user))
            }
            None => client.reply_ok("you must be logged in to logout"),
        }
    }

    /// Invoke the send command.
    ///
    /// This command can only be called when logged in.
    fn cmd_send(&self, client: &Client, msg: &str) -> MyResult<()> {
        if let Some(user) = &client.username {
            info!(name = ?user, msg, "user send");
            client.reply_ok(format!("{}: {}", user, msg))
        } else {
            client.reply_err("you must be logged in to send")
        }
    }
}

impl fmt::Display for TcpServer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

    /// Send an ok reply to this client with the correct first byte,
    /// `REPLY_FLAG_OK`.
    #[inline]
    fn reply_ok(&self, msg: impl AsRef<str>) -> MyResult<()> {
        self.sock
            .send(format!("{}{}", REPLY_FLAG_OK as char, msg.as_ref()))
    }

    /// Send an error reply to this client with the correct first byte,
    /// `REPLY_FLAG_ERR`.
    #[inline]
    fn reply_err(&self, msg: impl AsRef<str>) -> MyResult<()> {
        self.sock
            .send(format!("{}{}", REPLY_FLAG_ERR as char, msg.as_ref()))
    }
}
