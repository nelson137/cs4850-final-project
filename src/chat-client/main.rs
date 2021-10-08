use std::process::exit;

use libchat::{err::MyResult, CHAT_PORT};

mod client;
use client::SocketClient;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        exit(1);
    }
    exit(0);
}

fn run() -> MyResult<()> {
    SocketClient::new(CHAT_PORT)?.run()
}
