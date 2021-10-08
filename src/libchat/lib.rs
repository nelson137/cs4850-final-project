use libc::size_t;

pub mod err;

pub mod sys;

/// Port used by servers and clients
pub const CHAT_PORT: u16 = 10087;

/// Maximum number of bytes (including null) that can be sent in a message.
///
/// Use this for buffers.
pub const MSG_MAX: size_t = 256;
