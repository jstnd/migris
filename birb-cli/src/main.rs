mod commands {
    pub mod migrate;
}

use clap::{Parser, Subcommand};

use crate::commands::migrate::{MigrateArguments, MigrateEngine};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
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
