use std::{
    cell::RefCell,
    io::{self, Stdin, Stdout, Write},
    os::unix::prelude::AsRawFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use colored::{ColoredString, Colorize};
use libc::POLLIN;
use regex::Regex;
use tracing::{info, trace};

use super::client::TcpClient;

use libchat::{
    err::MyResult, setup_int_handler, sys::poll, PASSWORD_MAX, PASSWORD_MIN,
    USERNAME_MAX, USERNAME_MIN,
};

static E_NOT_LOGGED_OUT: &str = "Denied. Must be logged out.";
static E_NOT_LOGGED_IN: &str = "Denied. Please login first.";

macro_rules! _HELP_FORMAT {
    () => {
        "
Commands always available:

  help                 Print this help message.

Commands only available when {} logged in:

  newuser USER PASS    Create a new user with the given credentials.
  login USER PASS      Login to the chat room with the given credentials.

Commands only available when logged in:

  logout               Logout of the chat room and quit Chat Boat.
  send MSG             Broadcast a message to everyone in the chat room.

"
    };
}

/// Return the commands help message with styalized text.
fn build_help() -> String {
    format!(_HELP_FORMAT!(), "not".italic())
}

/// The Client REPL.
///
/// This type manages reading commands in from the user, verifying their syntax,
/// and sending them to the server via a `TcpClient`.
///
/// The only exposed method is `main_loop()` which runs the REPL.
pub struct Repl {
    client: TcpClient,
    logged_in: bool,
    stdin: Stdin,
    stdout: RefCell<Stdout>,
    help_msg: String,
    prompt_in_notlogged: ColoredString,
    prompt_in_logged: ColoredString,
    prompt_out_err: ColoredString,
    prompt_out_info: ColoredString,
}

impl Repl {
    pub fn new(client: TcpClient) -> Self {
        Self {
            client,
            logged_in: false,
            stdin: io::stdin(),
            stdout: RefCell::new(io::stdout()),
            help_msg: build_help(),
            prompt_in_notlogged: "< ".bold(),
            prompt_in_logged: "< ".green().bold(),
            prompt_out_err: "> ".red().bold(),
            prompt_out_info: "> ".bright_black(),
        }
    }

    //==================================================
    // Utilities
    //==================================================

    /// Print the server reply with the correct prompt and return whether the
    /// reply indicates a success or failure of the previous sent command.
    #[inline]
    fn server_reply(&self) -> MyResult<bool> {
        let reply = self.client.recv_reply()?;
        match &reply {
            Ok(msg) => self.print_info(msg)?,
            Err(msg) => self.print_err(msg)?,
        }
        Ok(reply.is_ok())
    }

    //==================================================
    // Utilities - Printing
    //==================================================

    /// Return the styalized string of the prompt according to the login state.
    #[inline]
    fn get_user_prompt(&self) -> &ColoredString {
        if self.logged_in {
            &self.prompt_in_logged
        } else {
            &self.prompt_in_notlogged
        }
    }

    /// Print `msg`, ensuring that it appears on the screen even if it contains
    /// no newline by calling `flush()`.
    #[inline]
    fn print(&self, msg: impl AsRef<[u8]>) -> MyResult<()> {
        let mut stdout = self.stdout.borrow_mut();
        stdout.write_all(msg.as_ref())?;
        stdout.flush()?;
        Ok(())
    }

    /// Print `msg` with a newline.
    #[inline]
    fn println(&self, msg: impl AsRef<[u8]>) -> MyResult<()> {
        let mut stdout = self.stdout.borrow_mut();
        stdout.write_all(msg.as_ref())?;
        stdout.write_all(&['\n' as u8])?;
        stdout.flush()?;
        Ok(())
    }

    /// Print `msg` with the error prompt.
    #[inline]
    fn print_err(&self, msg: impl AsRef<str>) -> MyResult<()> {
        self.print(self.prompt_out_err.to_string())?;
        self.println(msg.as_ref())?;
        Ok(())
    }

    /// Print `msg` with the server info prompt.
    ///
    /// This is for command responses from the server that indicate success.
    #[inline]
    fn print_info(&self, msg: impl AsRef<str>) -> MyResult<()> {
        self.print(self.prompt_out_info.to_string())?;
        self.println(msg.as_ref())?;
        Ok(())
    }

    //==================================================
    // Main Loop
    //==================================================

    /// Run the REPL.
    pub fn main_loop(&mut self) -> MyResult<()> {
        let stdin = io::stdin();

        let should_stop = Arc::new(AtomicBool::new(false));
        setup_int_handler(&should_stop)?;

        let mut raw_line = String::new();
        let re_cmd = Regex::new(r"^\s*(\S+) ?(.*)$")?;

        let delay = Duration::from_millis(25);

        let mut did_prompt = false;

        loop {
            thread::sleep(delay);

            if should_stop.load(Ordering::Relaxed) {
                break;
            }

            if !did_prompt {
                self.print(self.get_user_prompt().to_string())?;
                did_prompt = true;
            }

            if !poll(stdin.as_raw_fd(), POLLIN)? {
                continue;
            }

            raw_line.clear();
            self.stdin.read_line(&mut raw_line)?;
            let line = raw_line.trim_end_matches('\n');
            did_prompt = false;
            trace!(line, "input");

            let (cmd, args) = match re_cmd.captures(line) {
                // If the line matches the command regex, the existance of the 2
                // match groups is guaranteed.
                Some(caps) => (
                    caps.get(1).unwrap().as_str(),
                    caps.get(2).unwrap().as_str(),
                ),
                None => continue,
            };

            let mut exit = false;

            let cmd_re = match cmd {
                "help" => self.print(self.help_msg.clone()),
                "newuser" => self.cmd_newuser(args),
                "login" => self.cmd_login(args),
                "logout" => match self.cmd_logout(args) {
                    Ok(logout) => {
                        if logout {
                            exit = true;
                        }
                        Ok(())
                    }
                    Err(err) => Err(err),
                },
                "send" => self.cmd_send(args),
                _ => self.print_err(format!(
                    "Error. Command not recognized: {}",
                    cmd
                )),
            };

            if let Err(error) = cmd_re {
                info!(%error, "error while executing command");
            }

            if exit {
                break;
            }
        }

        Ok(())
    }

    //==================================================
    // Commands
    //==================================================

    /// Parse `args` for the newuser command and send them to the server.
    ///
    /// syntax: newuser USER PASS
    ///
    /// This command may only be executed when logged out.
    fn cmd_newuser(&self, args: &str) -> MyResult<()> {
        if self.logged_in {
            return self.print_err(E_NOT_LOGGED_OUT);
        }
        trace!(args = ?args, "command NEWUSER");

        let mut a = args.split_ascii_whitespace();
        let (user, pass) = match (a.next(), a.next(), a.next()) {
            (Some(u), Some(p), None) => (u, p),
            _ => {
                self.print_err("Error. Syntax: newuser USER PASS")?;
                return Ok(());
            }
        };

        if user.len() < USERNAME_MIN || user.len() > USERNAME_MAX {
            self.print_err(format!(
                "Error. User name must be {}-{} characters",
                USERNAME_MIN, USERNAME_MAX
            ))?;
        } else if pass.len() < PASSWORD_MIN || pass.len() > PASSWORD_MAX {
            self.print_err(format!(
                "Error. Password must be {}-{} characters",
                PASSWORD_MIN, PASSWORD_MAX
            ))?;
        } else {
            self.client.send_cmd(&["newuser", user, pass])?;
            self.server_reply()?;
        }

        Ok(())
    }

    /// Parse `args` for the login command and send them to the server.
    ///
    /// syntax: login USER PASS
    ///
    /// This command may only be executed when logged out.
    fn cmd_login(&mut self, args: &str) -> MyResult<()> {
        if self.logged_in {
            return self.print_err(E_NOT_LOGGED_OUT);
        }
        trace!(args = ?args, "command LOGIN");

        let mut a = args.split_ascii_whitespace();
        let (user, pass) = match (a.next(), a.next(), a.next()) {
            (Some(u), Some(p), None) => (u, p),
            _ => {
                self.print_err("Error. Syntax: login USER PASS")?;
                return Ok(());
            }
        };

        self.client.send_cmd(&["login", user, pass])?;
        if self.server_reply()? {
            self.logged_in = true;
        }

        Ok(())
    }

    /// Parse `args` for the logout command and send them to the server.
    ///
    /// syntax: logout
    ///
    /// This command may only be executed when logged in.
    fn cmd_logout(&mut self, args: &str) -> MyResult<bool> {
        if !self.logged_in {
            self.print_err(E_NOT_LOGGED_IN)?;
            return Ok(false);
        }

        if !args.chars().all(|c| c.is_ascii_whitespace()) {
            self.print_err("Error. Syntax: logout")?;
            return Ok(false);
        }
        trace!("command LOGOUT");

        self.client.send_cmd(&["logout"])?;
        if self.server_reply()? {
            self.logged_in = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Parse `args` for the send command and send them to the server.
    ///
    /// syntax: send MSG...
    ///
    /// This command may only be executed when logged in.
    fn cmd_send(&self, args: &str) -> MyResult<()> {
        if !self.logged_in {
            return self.print_err(E_NOT_LOGGED_IN);
        }

        if Regex::new(r"^\s*$")?.is_match(args) {
            self.print_err("Error. Syntax: send MSG...")?;
            return Ok(());
        }
        trace!(args = ?args, "command SEND");

        self.client.send_cmd(&["send", args])?;
        self.server_reply()?;

        Ok(())
    }
}
