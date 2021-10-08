use num_traits::{PrimInt, Unsigned};

/// Convert any unsigned int type from host byte order to network byte order.
pub fn hton<U: PrimInt + Unsigned>(u: U) -> U {
    u.to_be()
}
