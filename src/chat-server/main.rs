use std::process::exit;

use libchat::{err::MyResult, CHAT_PORT};

mod server;
use server::SocketServer;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        exit(1);
    }
    exit(0);
}

fn run() -> MyResult<()> {
    SocketServer::new(CHAT_PORT)?.run()
}
