use std::{
    cell::RefCell,
    io::{self, Stdin, Stdout, Write},
};

use colored::{ColoredString, Colorize};
use regex::Regex;
use tracing::trace;

use super::client::TcpClient;

use libchat::{
    err::MyResult, PASSWORD_MAX, PASSWORD_MIN, USERNAME_MAX, USERNAME_MIN,
};

macro_rules! _HELP_FORMAT {
    () => {
        "
Commands always available:

  help                 Print this help message.

Commands only available when {} logged in:

  quit                 Quit Chat Boat.
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
            prompt_out_err: "> error: ".red().bold(),
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
        let mut raw_line = String::new();
        let re_cmd = Regex::new(r"^\s*(\S+) ?(.*)$")?;

        loop {
            raw_line.clear();
            self.print(self.get_user_prompt().to_string())?;
            self.stdin.read_line(&mut raw_line)?;
            let line = raw_line.trim_end_matches('\n');
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

            match cmd {
                "help" => self.print(self.help_msg.clone())?,
                "quit" => {
                    if self.cmd_quit(args)? {
                        break;
                    }
                }
                "newuser" => self.cmd_newuser(args)?,
                "login" => self.cmd_login(args)?,
                "logout" => {
                    self.cmd_logout(args)?;
                    break;
                }
                "send" => self.cmd_send(args)?,
                cmd => {
                    self.print_err(format!("command not recognized: {}", cmd))?;
                }
            }
        }

        Ok(())
    }

    //==================================================
    // Commands
    //==================================================

    /// Exit the program if not logged in.
    ///
    /// syntax: quit [ARGS...]
    ///
    /// This command has special behavior since it is client-side only and may
    /// only be used when not logged in to exit the program. If logged in, it
    /// will forward the command to the server like the other commands, the
    /// server will reply with an error since it is not implemented, and false
    /// will be returned indicating not to quit. If not logged in, true will be
    /// returned indicating to quit.
    ///
    /// Note: This command has less strict syntax; extra arguments are allowed
    /// and will be ignored.
    fn cmd_quit(&self, args: &str) -> MyResult<bool> {
        trace!("command QUIT");

        if self.logged_in {
            self.client.send_cmd(&["quit", args])?;
            self.server_reply()?;
            Ok(false)
        } else {
            self.println("goodbye.")?;
            Ok(true)
        }
    }

    /// Parse `args` for the newuser command and send them to the server.
    ///
    /// syntax: newuser USER PASS
    fn cmd_newuser(&self, args: &str) -> MyResult<()> {
        let re_newuser_args = Regex::new(r"^\s*(\S+)\s+(\S+)\s*$")?;
        let newuser_args = match re_newuser_args.captures(args) {
            None => None,
            Some(caps) => match (caps.get(1), caps.get(2)) {
                (Some(u), Some(p)) => Some((u.as_str(), p.as_str())),
                _ => None,
            },
        };
        trace!(args = ?newuser_args, "command NEWUSER");

        if let Some((user, pass)) = newuser_args {
            if user.len() < USERNAME_MIN || user.len() > USERNAME_MAX {
                self.print_err(format!(
                    "newuser: USER must be {} to {} characters long, inclusive",
                    USERNAME_MIN, USERNAME_MAX
                ))?;
            } else if pass.len() < PASSWORD_MIN || pass.len() > PASSWORD_MAX {
                self.print_err(format!(
                    "newuser: PASS must be {} to {} characters long, inclusive",
                    PASSWORD_MIN, PASSWORD_MAX
                ))?;
            } else {
                self.client.send_cmd(&["newuser", user, pass])?;
                self.server_reply()?;
            }
        } else {
            self.print_err("syntax: newuser USER PASS")?;
        }

        Ok(())
    }

    /// Parse `args` for the login command and send them to the server.
    ///
    /// syntax: login USER PASS
    fn cmd_login(&mut self, args: &str) -> MyResult<()> {
        let re_login_args = Regex::new(r"^\s*(\S+)\s+(\S+)\s*$")?;
        let login_args = match re_login_args.captures(args) {
            None => None,
            Some(caps) => match (caps.get(1), caps.get(2)) {
                (Some(u), Some(p)) => Some((u.as_str(), p.as_str())),
                _ => None,
            },
        };
        trace!(args = ?login_args, "command LOGIN");

        if let Some((user, pass)) = login_args {
            self.client.send_cmd(&["login", user, pass])?;
            if self.server_reply()? {
                self.logged_in = true;
            }
        } else {
            self.print_err("syntax: newuser USER PASS")?;
        }

        Ok(())
    }

    /// Parse `args` for the logout command and send them to the server.
    ///
    /// syntax: logout
    fn cmd_logout(&mut self, args: &str) -> MyResult<()> {
        if !Regex::new(r"^\s*$")?.is_match(args) {
            self.print_err("syntax: logout")?;
            return Ok(());
        }
        trace!("command LOGOUT");

        self.client.send_cmd(&["logout"])?;
        if self.server_reply()? {
            self.logged_in = false;
        }

        Ok(())
    }

    /// Parse `args` for the send command and send them to the server.
    ///
    /// syntax: send MSG...
    fn cmd_send(&self, args: &str) -> MyResult<()> {
        if Regex::new(r"^\s*$")?.is_match(args) {
            self.print_err("syntax: send MSG...")?;
            return Ok(());
        }
        trace!(args = ?args, "command SEND");

        self.client.send_cmd(&["send", args])?;
        self.server_reply()?;

        Ok(())
    }
}
