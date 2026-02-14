mod commands {
    pub mod migrate;
}

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use migris::Connector;

use crate::commands::migrate::{MigrateArguments, MigrateEngine};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Migrate(MigrateArguments),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Migrate(args) => {
            let engine = MigrateEngine::new(args);
            let result = engine.migrate().await;

            match result {
                Ok(()) => println!("Migrate success!"),
                Err(err) => println!("{}", err),
            }
        }
    }
}

pub fn create_connector(str: &str) -> anyhow::Result<Box<dyn Connector>> {
    migris::connector_from_str(str)
        .ok_or_else(|| anyhow!("Failed to create connector for identifier: {}", str))
}
