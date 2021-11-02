use std::{
    cell::RefCell,
    io::{self, Stdin, Stdout, Write},
};

use colored::{ColoredString, Colorize};
use regex::Regex;
use tracing::trace;

use super::client::TcpClient;

use libchat::err::MyResult;

macro_rules! _HELP_FORMAT {
    () => {
        "
Commands always available:

  help                 Print this help message.
  quit                 Quit Chat Boat. Will log out if logged in.

Commands only available when {} logged in:

  newuser USER PASS    Create a new user with the given credentials.
  login USER PASS      Login to the chat room with the given credentials.

Commands only available when logged in:

  logout               Logout of the chat room.
  send MSG             Broadcast a message to everyone in the chat room.

"
    };
}

fn build_help() -> String {
    format!(_HELP_FORMAT!(), "not".italic())
}

pub struct Repl {
    client: TcpClient,
    logged_in: bool,
    stdin: Stdin,
    stdout: RefCell<Stdout>,
    help_msg: String,
    prompt_out_err: ColoredString,
    prompt_out_err_server: ColoredString,
    prompt_out_msg: ColoredString,
    prompt_in_notlogged: ColoredString,
    prompt_in_logged: ColoredString,
}

impl Repl {
    pub fn new(client: TcpClient) -> Self {
        Self {
            client,
            logged_in: false,
            stdin: io::stdin(),
            stdout: RefCell::new(io::stdout()),
            help_msg: build_help(),
            prompt_out_err: "error: ".red().bold(),
            prompt_out_err_server: "> error: ".red().bold(),
            prompt_out_msg: "> ".bright_black(),
            prompt_in_notlogged: "< ".bold(),
            prompt_in_logged: "< ".green().bold(),
        }
    }

    //==================================================
    // Utilities
    //==================================================

    #[inline]
    fn server_reply(&self) -> MyResult<bool> {
        let reply = self.client.recv_reply()?;
        match &reply {
            Ok(msg) => self.print_info(msg)?,
            Err(msg) => self.print_err_server(msg)?,
        }
        Ok(reply.is_ok())
    }

    //==================================================
    // Utilities - IO
    //==================================================

    #[inline]
    fn get_prompt_user(&self) -> &ColoredString {
        if self.logged_in {
            &self.prompt_in_logged
        } else {
            &self.prompt_in_notlogged
        }
    }

    #[inline]
    fn print(&self, msg: impl AsRef<[u8]>) -> MyResult<()> {
        let mut stdout = self.stdout.borrow_mut();
        stdout.write_all(msg.as_ref())?;
        stdout.flush()?;
        Ok(())
    }

    #[inline]
    fn println(&self, msg: impl AsRef<[u8]>) -> MyResult<()> {
        let mut stdout = self.stdout.borrow_mut();
        stdout.write_all(msg.as_ref())?;
        stdout.write_all(&['\n' as u8])?;
        stdout.flush()?;
        Ok(())
    }

    #[inline]
    fn print_err(&self, msg: impl AsRef<str>) -> MyResult<()> {
        self.print(self.prompt_out_err.to_string())?;
        self.println(msg.as_ref())?;
        Ok(())
    }

    #[inline]
    fn print_err_server(&self, msg: impl AsRef<str>) -> MyResult<()> {
        self.print(self.prompt_out_err_server.to_string())?;
        self.println(msg.as_ref())?;
        Ok(())
    }

    #[inline]
    fn print_info(&self, msg: impl AsRef<str>) -> MyResult<()> {
        self.print(self.prompt_out_msg.to_string())?;
        self.println(msg.as_ref())?;
        Ok(())
    }

    //==================================================
    // Main Loop
    //==================================================

    pub fn main_loop(&mut self) -> MyResult<()> {
        let mut raw_line = String::new();
        let re_cmd = Regex::new(r"^\s*(\S+) ?(.*)$")?;

        loop {
            raw_line.clear();
            self.print(self.get_prompt_user().to_string())?;
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
                "quit" => break,
                "newuser" => self.cmd_newuser(args)?,
                "login" => self.cmd_login(args)?,
                "logout" => self.cmd_logout(args)?,
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

    fn cmd_newuser(&self, args: &str) -> MyResult<()> {
        let re_newuser_args = Regex::new(r"^\s*(\S+)\s+(\S+)\s*$")?;
        let newuser_args = match re_newuser_args.captures(args) {
            None => None,
            Some(caps) => match (caps.get(1), caps.get(2)) {
                (Some(u), Some(p)) => Some((u.as_str(), p.as_str())),
                _ => None,
            },
        };
        trace!(args = ?newuser_args, "parsed command newuser");

        if let Some((user, pass)) = newuser_args {
            self.client.send_cmd(&["newuser", user, pass])?;
            self.server_reply()?;
        } else {
            self.print_err("syntax: newuser USER PASS")?;
        }

        Ok(())
    }

    fn cmd_login(&mut self, args: &str) -> MyResult<()> {
        let re_login_args = Regex::new(r"^\s*(\S+)\s+(\S+)\s*$")?;
        let login_args = match re_login_args.captures(args) {
            None => None,
            Some(caps) => match (caps.get(1), caps.get(2)) {
                (Some(u), Some(p)) => Some((u.as_str(), p.as_str())),
                _ => None,
            },
        };

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

    fn cmd_logout(&mut self, args: &str) -> MyResult<()> {
        if !Regex::new(r"^\s*$")?.is_match(args) {
            self.print_err("syntax: logout")?;
            return Ok(());
        }

        self.client.send_cmd(&["logout"])?;
        if self.server_reply()? {
            self.logged_in = false;
        }

        Ok(())
    }

    fn cmd_send(&self, args: &str) -> MyResult<()> {
        self.client.send_cmd(&["send", args])?;
        self.server_reply()?;

        Ok(())
    }
}
