mod commands {
    pub mod migrate;
}

use anyhow::anyhow;
use birb::Connector;
use clap::{Parser, Subcommand};

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
    birb::connector_from_str(str)
        .ok_or_else(|| anyhow!("Failed to create connector for identifier: {}", str))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum FileType {
    Csv,
}

impl std::fmt::Display for FileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = match self {
            Self::Csv => "csv",
        };

        write!(f, "{}", display)
    }
}

impl From<birb::FileType> for FileType {
    fn from(value: birb::FileType) -> Self {
        match value {
            birb::FileType::Csv => Self::Csv,
        }
    }
}
