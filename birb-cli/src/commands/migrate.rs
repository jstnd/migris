use std::path::Path;

use birb::ConnectorKind;
use clap::Args;

#[derive(Args, Debug)]
pub struct MigrateArguments {
    /// The source to migrate data from.
    #[arg(short, long, index = 1)]
    source: String,

    /// The table to migrate data from.
    #[arg(long)]
    source_table: Option<String>,

    /// The target to migrate data to.
    #[arg(short, long, index = 2)]
    target: String,

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

        if let Some(table) = &self.args.target_table {
            write_options = write_options.with_table_name(table);
        }

        for source_identifier in self.sources()? {
            let mut source = crate::create_connector(&source_identifier)?;

            match source.kind() {
                ConnectorKind::Database => {
                    if let Some(table) = &self.args.source_table {
                        read_options = read_options.with_query(format!("SELECT * FROM {}", table));

                        // Use source table name if a target table name was not given.
                        if self.args.target_table.is_none() {
                            write_options = write_options.with_table_name(table);
                        }

                        let data = source.read(&read_options).await?;
                        target.write(data, &write_options).await?;
                    } else {
                        // TODO: show info message if no tables were found
                        let tables = source.tables().await?;

                        for table in tables {
                            read_options = read_options.with_query(format!(
                                "SELECT * FROM {}.{}",
                                table.schema, table.name
                            ));

                            // Pass schema from source table as fallback option.
                            write_options = write_options.with_table_schema(table.schema);

                            // Use source table name if a target table name was not given.
                            if self.args.target_table.is_none() {
                                write_options = write_options.with_table_name(table.name);
                            }

                            let data = source.read(&read_options).await?;
                            target.write(data, &write_options).await?;
                        }
                    }
                }
                ConnectorKind::File => {
                    // Use source file name if a target table name was not given.
                    if self.args.target_table.is_none()
                        && let Some(stem) = birb::util::get_stem(&source_identifier)
                    {
                        let safe_name = birb::util::get_safe_name(stem);
                        write_options = write_options.with_table_name(safe_name);
                    }

                    let data = source.read(&read_options).await?;
                    target.write(data, &write_options).await?;
                }
            }
        }

        Ok(())
    }

    fn sources(&self) -> anyhow::Result<Vec<String>> {
        if Path::new(&self.args.source).is_dir() {
            let supported = birb::util::supported_extensions();
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
