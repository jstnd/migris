use anyhow::anyhow;
use clap::Args;

#[derive(Args, Debug)]
pub struct MigrateArguments {
    /// The source to migrate data from.
    #[arg(long)]
    source: String,

    /// The table to migrate data from.
    #[arg(long)]
    source_table: Option<String>,

    /// The schema to migrate data from.
    #[arg(long)]
    source_schema: Option<String>,

    /// The target to migrate data to.
    #[arg(long)]
    target: String,

    /// The table to migrate data to.
    #[arg(long)]
    target_table: Option<String>,

    /// The schema to migrate data to.
    #[arg(long)]
    target_schema: Option<String>,
}

#[derive(Debug)]
pub struct MigrateEngine {
    args: MigrateArguments,
}

impl MigrateEngine {
    pub fn new(args: MigrateArguments) -> Self {
        Self { args }
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        let mut source = birb::connector_from_str(&self.args.source).ok_or_else(|| {
            anyhow!(
                "Failed to create connector for source: {}",
                self.args.source
            )
        })?;

        let mut target = birb::connector_from_str(&self.args.target).ok_or_else(|| {
            anyhow!(
                "Failed to create connector for target: {}",
                self.args.target
            )
        })?;

        let read_options = birb::ReadOptions::new().with_query(format!(
            "SELECT * FROM {}.{}",
            self.args.source_schema.as_ref().unwrap(),
            self.args.source_table.as_ref().unwrap()
        ));

        let write_options = birb::WriteOptions::new()
            .with_table_name(self.args.target_table.as_ref().unwrap())
            .with_table_schema(self.args.target_schema.as_ref().unwrap());

        let data = source.read(&read_options).await?;
        target.write(data, write_options).await?;

        Ok(())
    }
}
