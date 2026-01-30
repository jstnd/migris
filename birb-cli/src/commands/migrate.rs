use clap::Args;

#[derive(Args, Debug)]
pub struct MigrateArguments {
    /// The source to migrate data from.
    #[arg(long)]
    source: String,

    /// The schema to migrate data from.
    #[arg(long)]
    source_schema: Option<String>,

    /// The table to migrate data from.
    #[arg(long)]
    source_table: Option<String>,

    /// The target to migrate data to.
    #[arg(long)]
    target: String,

    /// The schema to migrate data to.
    #[arg(long)]
    target_schema: Option<String>,

    /// The table to migrate data to.
    #[arg(long)]
    target_table: Option<String>,
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
        let mut source = crate::create_connector(&self.args.source)?;
        let mut target = crate::create_connector(&self.args.target)?;
        let mut read_options = birb::ReadOptions::new();
        let mut write_options = birb::WriteOptions::new();

        if let Some(schema) = &self.args.source_schema
            && let Some(table) = &self.args.source_table
        {
            read_options = read_options.with_query(format!("SELECT * FROM {}.{}", schema, table));
        }

        if let Some(schema) = &self.args.target_schema {
            write_options = write_options.with_table_schema(schema);
        }

        if let Some(table) = &self.args.target_table {
            write_options = write_options.with_table_name(table);
        }

        let data = source.read(&read_options).await?;
        target.write(data, write_options).await?;

        Ok(())
    }
}
