use std::os::raw::c_int;

use libc::size_t;

mod banner;
pub use banner::*;

pub mod err;

mod signal;
pub use signal::*;

pub mod sys;

mod users_dao;
pub use users_dao::UsersDao;

/// Port used by servers and clients
pub const CHAT_PORT: u16 = 10087;

// Maximum number of pending connections that can be in the queue.
pub const LISTEN_BACKLOG: c_int = 64;

/// Minimum length of username.
pub const USERNAME_MIN: usize = 3;

/// Maximum length of password.
pub const USERNAME_MAX: usize = 32;

/// Minimum length of password.
pub const PASSWORD_MIN: usize = 4;

/// Maximum length of username.
pub const PASSWORD_MAX: usize = 8;

/// Maximum length of a message that can be sent, *not* including a terminating
/// null byte.
pub const MSG_MAX: size_t = 256;

/// The maximum buffer size of a client command or a server reply.
///
/// The maximum message that can be sent is the reply of a send command from
/// the server, which is `USER: MSG`. Therefore, the largest size of a message,
/// and buffers that will hold a message, is: the maximum username length + 2
/// for the `": "` after the username + the maximum message size + a terminating
/// null byte.
pub const COMMAND_MAX: usize = USERNAME_MAX + 2 + MSG_MAX + 1;

/// The character to use to separate server command arguments.
pub const COMMAND_SEP: &str = "\x02";

/// Represent a server reply.
///
/// - An `Ok` represents a command that completed successfully.
/// - An `Err` represents a command that failed.
///
/// The reply will be sent to the client with the first byte being either
/// `RESPONSE_FLAG_OK` or `RESPONSE_FLAG_ERR`.
pub type ServerReply = Result<String, String>;

/// Magic number byte for handshake between client and server indicating that
/// the connection is accepted.
///
/// This server must reply with this exact message after connecting, or the
/// program will exit.
pub const HANDSHAKE_ACK: &str = "\x06";

/// Magic number byte for server command replies indicating a failure.
///
/// This must be the first byte of the reply string.
pub const REPLY_FLAG_ERR: u8 = 0x15;
