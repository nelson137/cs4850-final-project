use std::process::exit;

use client::TcpClient;
use tracing::level_filters::STATIC_MAX_LEVEL;
use tracing_subscriber;

use libchat::{err::MyResult, print_client_banner, CHAT_PORT};

pub mod client;

pub mod repl;
use repl::Repl;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        exit(1);
    }
    exit(0);
}

fn run() -> MyResult<()> {
    tracing_subscriber::fmt()
        .with_max_level(STATIC_MAX_LEVEL)
        .init();

    print_client_banner();

    let client = TcpClient::new(CHAT_PORT)?;
    Repl::new(client).main_loop()?;

    Ok(())
}
