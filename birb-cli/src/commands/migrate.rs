use std::path::Path;

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

        for source in self.sources()? {
            let mut source = crate::create_connector(&source)?;

            let data = source.read(&read_options).await?;
            target.write(data, &write_options).await?;
        }

        Ok(())
    }

    fn sources(&self) -> anyhow::Result<Vec<String>> {
        if Path::new(&self.args.source).is_dir() {
            let supported = birb::util::get_supported_extensions();
            let mut sources = Vec::new();

            for entry in walkdir::WalkDir::new(&self.args.source) {
                let entry = entry?;
                let path = entry.path();

                if let Some(extension) = birb::util::get_extension(&path)
                    && supported.contains(&extension)
                {
                    sources.push(path.display().to_string());
                }
            }

            Ok(sources)
        } else {
            Ok(vec![self.args.source.clone()])
        }
    }
}
