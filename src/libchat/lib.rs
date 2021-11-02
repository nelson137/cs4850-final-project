use libc::size_t;

mod banner;
pub use banner::*;

pub mod err;
use err::MyResult;

mod signal;
pub use signal::*;

pub mod sys;

mod users_dao;
pub use users_dao::UsersDao;

/// Port used by servers and clients
pub const CHAT_PORT: u16 = 10087;

/// Maximum number of bytes (including null) that can be sent in a message.
///
/// Use this for buffers.
pub const MSG_MAX: size_t = 256;

/// The character to use to separate server command arguments.
pub const COMMAND_SEP: &str = "\x02";

/// Represent a result whose `Ok` is a logical result:
///
/// - An `Err` represents a server error that must be handled.
/// - An `Ok` with a logical `Ok` represents a command that completed
/// successfully. This will be sent back to the client.
/// - An `Ok` with a logical `Err` represents a command that failed.
///
/// Logical `Ok` and `Err` values will be sent back to the client as a response
/// with either `RESPONSE_FLAG_OK` or `RESPONSE_FLAG_ERR` as a prefix.
pub type CmdResult = MyResult<Result<String, String>>;

pub const RESPONSE_FLAG_OK: u8 = 0x06;

pub const RESPONSE_FLAG_ERR: u8 = 0x15;
