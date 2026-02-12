use std::path::Path;

use anyhow::anyhow;
use birb::{Connector, ConnectorKind, FileType};
use clap::Args;

#[derive(Args, Debug)]
pub struct MigrateArguments {
    /// The source to migrate data from.
    #[arg(short, long, index = 1)]
    source: String,

    /// The table to migrate data from.
    #[arg(long)]
    source_table: Option<String>,

    /// The source file type (used when passing a directory as source).
    #[arg(long)]
    source_type: Option<FileType>,

    /// The target to migrate data to.
    #[arg(short, long, index = 2)]
    target: String,

    /// The table to migrate data to.
    #[arg(long)]
    target_table: Option<String>,

    /// The target file type (used when passing a directory as target).
    #[arg(long, default_value_t = FileType::Csv)]
    target_type: FileType,
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
                        read_options =
                            read_options.with_query(format!("SELECT * FROM `{}`", table));

                        // Use source table name if a target table name was not given.
                        if self.args.target_table.is_none() {
                            write_options = write_options.with_table_name(table);
                        }

                        let mut target = self.target(table)?;
                        let data = source.read(&read_options).await?;
                        target.write(data, &write_options).await?;
                    } else {
                        // TODO: show info message if no tables were found
                        let tables = source.tables().await?;

                        for table in tables {
                            read_options = read_options.with_query(format!(
                                "SELECT * FROM `{}`.`{}`",
                                table.schema, table.name
                            ));

                            // Pass schema from source table as fallback option.
                            write_options = write_options.with_table_schema(table.schema);

                            // Use source table name if a target table name was not given.
                            if self.args.target_table.is_none() {
                                write_options = write_options.with_table_name(&table.name);
                            }

                            let mut target = self.target(&table.name)?;
                            let data = source.read(&read_options).await?;
                            target.write(data, &write_options).await?;
                        }
                    }
                }
                ConnectorKind::File => {
                    let stem =
                        birb::common::get_file_stem(&source_identifier).ok_or_else(|| {
                            anyhow!("Failed to get stem for file: {}", source_identifier)
                        })?;

                    // Use source file name if a target table name was not given.
                    if self.args.target_table.is_none() {
                        let safe_name = birb::common::get_safe_name(stem);
                        write_options = write_options.with_table_name(safe_name);
                    }

                    let mut target = self.target(stem)?;
                    let data = source.read(&read_options).await?;
                    target.write(data, &write_options).await?;
                }
            }
        }

        Ok(())
    }

    fn sources(&self) -> anyhow::Result<Vec<String>> {
        if Path::new(&self.args.source).is_dir() {
            let mut sources = Vec::new();

            for entry in walkdir::WalkDir::new(&self.args.source) {
                let entry = entry?;
                let path = entry.path();

                if let Some(file_type) = birb::common::get_file_type(&path) {
                    // Skip this entry if the entry's file type does not match the given source type.
                    if let Some(source_type) = self.args.source_type
                        && source_type != file_type
                    {
                        continue;
                    }

                    sources.push(path.display().to_string());
                }
            }

            Ok(sources)
        } else {
            Ok(vec![self.args.source.clone()])
        }
    }

    fn target(&self, file_name: &str) -> anyhow::Result<Box<dyn Connector>> {
        if let Ok(connector) = crate::create_connector(&self.args.target) {
            Ok(connector)
        } else {
            let target_path = Path::new(&self.args.target);

            // Handle the case where the user passes a directory that does not exist, by creating the directory here.
            // We ignore any error here in favor of a more definite error message later.
            let _ = std::fs::create_dir_all(target_path);

            if target_path.is_dir() {
                let target_path =
                    target_path.join(format!("{}.{}", file_name, self.args.target_type));
                crate::create_connector(&target_path.display().to_string())
            } else {
                Err(anyhow!(
                    "Failed to create connector for identifier: {}",
                    self.args.target
                ))
            }
        }
    }
}
