use std::sync::{atomic::AtomicBool, Arc};

use signal_hook;

use crate::err::MyResult;

pub fn setup_int_handler(stop_flag: &Arc<AtomicBool>) -> MyResult<()> {
    signal_hook::flag::register(libc::SIGINT, stop_flag.clone())?;
    Ok(())
}
