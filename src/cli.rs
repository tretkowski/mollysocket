use clap::{Parser, Subcommand};
use std::{env, path::PathBuf};

use crate::cli::{connection::ConnectionCommand, test::TestCommand};
use crate::config;

mod connection;
mod server;
mod test;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(infer_subcommands = true)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run webserver and websockets
    Server {},

    /// Add, remove and list connections
    Connection {
        #[command(subcommand)]
        command: ConnectionCommand,
    },

    /// Test account and endpoint validity
    Test {
        #[command(subcommand)]
        command: TestCommand,
    },
}

pub async fn cli() {
    let cli = Cli::parse();

    if cli.debug {
        env::set_var("RUST_LOG", "debug")
    }

    match &cli.command {
        Command::Server {} => (),
        _ => {
            if env::var("RUST_LOG").is_err() {
                env::set_var("RUST_LOG", "info");
            }
        }
    }
    env_logger::init();

    config::load_config(cli.config);

    match &cli.command {
        Command::Server {} => server::server().await,
        Command::Connection { command } => connection::connection(command).await,
        Command::Test { command } => test::test(command).await,
    }
}
