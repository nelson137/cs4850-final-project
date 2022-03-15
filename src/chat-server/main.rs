use std::{path::PathBuf, process::exit};

use dotenv;
use tracing::level_filters::STATIC_MAX_LEVEL;
use tracing_subscriber;

use libchat::{err::MyResult, print_server_banner, UsersDao, CHAT_PORT};

mod server;
use server::TcpServer;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {}", err);
        exit(1);
    }
    exit(0);
}

fn run() -> MyResult<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_max_level(STATIC_MAX_LEVEL)
        .init();

    print_server_banner();

    let users_db = UsersDao::from(PathBuf::from(dotenv::var("USERS_DB")?))?;
    TcpServer::new(CHAT_PORT, users_db)?.main_loop()?;

    Ok(())
}
