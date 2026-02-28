use std::str::FromStr;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use futures_util::StreamExt;
use rust_decimal::Decimal;
use sqlx::{
    Encode, Executor, MySql, MySqlPool, QueryBuilder, Row as SqlxRow, Type, ValueRef,
    mysql::{MySqlArguments, MySqlConnectOptions, MySqlValueRef},
    query::Query,
};

use crate::{
    Column, ColumnFlag, ColumnType, Connector, ConnectorData, ConnectorKind, MigrisError,
    MigrisResult, ReadOptions, Row, Table, Value, WriteOptions,
    common::{self, DEFAULT_SCHEMA, decode_sqlx},
};

const MYSQL_MAX_PARAMETERS: usize = 65535;

pub struct MySqlConnector {
    url: String,
    pool: Option<MySqlPool>,
}

impl MySqlConnector {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            pool: None,
        }
    }

    async fn connect(&mut self) -> MigrisResult<&MySqlPool> {
        if self.pool.is_none() {
            self.pool = Some(
                sqlx::MySqlPool::connect(&self.url)
                    .await
                    .map_err(|err| MigrisError::DatabaseConnectFailed(err.to_string()))?,
            );
        }

        Ok(self.pool.as_ref().unwrap())
    }
}

#[async_trait::async_trait]
impl Connector for MySqlConnector {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::Database
    }

    async fn read<'a>(&mut self, options: &'a ReadOptions) -> MigrisResult<ConnectorData<'a>> {
        let table_schema = if let Some(schema) = schema_from_url(&self.url) {
            schema
        } else if let Some(schema) = &options.table_schema {
            schema.clone()
        } else {
            "".into()
        };

        let table_name = options
            .table_name
            .as_ref()
            .ok_or(MigrisError::DatabaseReadFailed(
                "no table name given".into(),
            ))?;

        let pool = self.connect().await?;
        let columns = columns(&table_schema, table_name, pool).await?;
        let stream_columns = columns.clone();
        let query = options
            .query
            .as_ref()
            .ok_or(MigrisError::DatabaseReadFailed("no query given".into()))?;

        let stream = sqlx::query(query).fetch(pool).map(move |row| {
            row.map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))
                .and_then(|row| Row::from_mysql(row, &stream_columns))
        });

        Ok(ConnectorData::new(columns, Box::pin(stream)))
    }

    async fn write<'a>(
        &mut self,
        data: ConnectorData<'a>,
        options: &WriteOptions,
    ) -> MigrisResult<()> {
        let table = get_write_table(&self.url, options);
        let pool = self.connect().await?;
        let mut txn = pool
            .begin()
            .await
            .map_err(|err| MigrisError::DatabaseWriteFailed(err.to_string()))?;

        // Create the table if it doesn't already exist.
        create_table(&table, &data.columns, pool).await?;

        if options.overwrite {
            truncate_table(&table, pool).await?;
        }

        let mut stream = data.stream.enumerate();
        let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
            "INSERT INTO `{}`.`{}` VALUES ",
            table.schema, table.name
        ));

        let mut rows_per_txn = 0;
        let mut current_rows_in_txn = 0;

        while let Some((idx, row)) = stream.next().await {
            let row = row?;

            // Perform data extraction using first row encountered.
            if idx == 0 {
                // Determine the maximum number of rows we can fit into a transaction.
                rows_per_txn = MYSQL_MAX_PARAMETERS / data.columns.len();
            }

            if current_rows_in_txn > 0 {
                builder.push(", ");
            }

            builder.push("(");
            let mut separated = builder.separated(", ");

            for value in row.values {
                separated.push_bind(value);
            }

            separated.push_unseparated(")");
            current_rows_in_txn += 1;

            if current_rows_in_txn == rows_per_txn {
                execute_query(builder.build(), &mut *txn).await?;
                builder.reset();
                current_rows_in_txn = 0;
            }
        }

        if current_rows_in_txn > 0 {
            execute_query(builder.build(), &mut *txn).await?;
        }

        txn.commit()
            .await
            .map_err(|err| MigrisError::DatabaseWriteFailed(err.to_string()))?;

        Ok(())
    }

    async fn exists(&mut self, options: &WriteOptions) -> bool {
        let table = get_write_table(&self.url, options);
        let Ok(pool) = self.connect().await else {
            return false;
        };

        let query = r#"
            SELECT EXISTS (
                SELECT *
                FROM information_schema.TABLES
                WHERE
                    TABLE_SCHEMA = ? AND
                    TABLE_NAME = ?
            )
        "#;

        sqlx::query_scalar(query)
            .bind(table.schema)
            .bind(table.name)
            .fetch_one(pool)
            .await
            .unwrap_or_default()
    }

    async fn tables(&mut self) -> MigrisResult<Vec<Table>> {
        if let Some(schema) = schema_from_url(&self.url) {
            let pool = self.connect().await?;
            let query = r#"
                SELECT
                    TABLE_SCHEMA AS `schema`,
                    TABLE_NAME AS `name`
                FROM information_schema.TABLES
                WHERE TABLE_SCHEMA = ?
            "#;

            let tables = sqlx::query_as::<_, Table>(query)
                .bind(schema)
                .fetch_all(pool)
                .await
                .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

            Ok(tables)
        } else {
            let pool = self.connect().await?;
            let query = r#"
                SELECT
                    TABLE_SCHEMA AS `schema`,
                    TABLE_NAME AS `name`
                FROM information_schema.TABLES
                WHERE
                    TABLE_SCHEMA NOT IN (
                        'information_schema', 'mysql',
                        'performance_schema', 'sys'
                    )
            "#;

            let tables = sqlx::query_as::<_, Table>(query)
                .fetch_all(pool)
                .await
                .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

            Ok(tables)
        }
    }
}

async fn columns(
    table_schema: &str,
    table_name: &str,
    pool: &MySqlPool,
) -> MigrisResult<Vec<Column>> {
    let query = r#"
        SELECT
            COLUMN_NAME,
            ORDINAL_POSITION - 1 AS `ORDINAL_POSITION`,
            IF(IS_NULLABLE = 'YES', TRUE, FALSE) AS `IS_NULLABLE`,
            CAST(COLUMN_TYPE AS CHAR) AS `COLUMN_TYPE`
        FROM information_schema.COLUMNS
        WHERE
            TABLE_SCHEMA = ? AND
            TABLE_NAME = ?
        ORDER BY
            ORDINAL_POSITION
    "#;

    let mut columns = Vec::new();
    let rows = sqlx::query(query)
        .bind(table_schema)
        .bind(table_name)
        .fetch_all(pool)
        .await
        .map_err(|err| MigrisError::DatabaseReadFailed(err.to_string()))?;

    for row in rows {
        let mut flags = Vec::new();
        let column_type: String = row.get("COLUMN_TYPE");

        if row.get("IS_NULLABLE") {
            flags.push(ColumnFlag::Nullable);
        }

        if column_type.ends_with("unsigned") {
            flags.push(ColumnFlag::Unsigned);
        }

        columns.push(Column {
            column_type: ColumnType::MySql(MySqlDataType::from_type(
                column_type.trim_end_matches(" unsigned"),
            )?),
            flags,
            name: row.get("COLUMN_NAME"),
            ordinal: row.get::<u32, _>("ORDINAL_POSITION") as usize,
        });
    }

    Ok(columns)
}

async fn create_table(table: &Table, columns: &[Column], pool: &MySqlPool) -> MigrisResult<()> {
    let query = format!("CREATE SCHEMA IF NOT EXISTS `{}`", table.schema);
    execute_query(sqlx::query(&query), pool).await?;

    let mut builder: QueryBuilder<MySql> = QueryBuilder::new(format!(
        "CREATE TABLE IF NOT EXISTS `{}`.`{}` (",
        table.schema, table.name
    ));

    let mut separated = builder.separated(", ");
    for column in columns {
        let mut definition = format!("`{}` {}", column.name, column.column_type.as_mysql());

        if column.is_unsigned() {
            definition.push_str(" UNSIGNED");
        }

        if column.is_nullable() {
            definition.push_str(" NULL");
        } else {
            definition.push_str(" NOT NULL");
        }

        separated.push(definition);
    }

    builder.push(")");
    execute_query(builder.build(), pool).await?;

    Ok(())
}

async fn truncate_table(table: &Table, pool: &MySqlPool) -> MigrisResult<()> {
    let query = format!("TRUNCATE `{}`.`{}`", table.schema, table.name);
    execute_query(sqlx::query(&query), pool).await?;

    Ok(())
}

async fn execute_query<'e, E>(
    query: Query<'_, MySql, MySqlArguments>,
    executor: E,
) -> MigrisResult<()>
where
    E: Executor<'e, Database = MySql>,
{
    query
        .execute(executor)
        .await
        .map_err(|err| MigrisError::DatabaseWriteFailed(err.to_string()))?;

    Ok(())
}

fn schema_from_url(url: &str) -> Option<String> {
    if let Ok(options) = MySqlConnectOptions::from_str(url)
        && let Some(schema) = options.get_database()
    {
        return Some(schema.to_string());
    }

    None
}

fn get_write_table(url: &str, options: &WriteOptions) -> Table {
    let generated = common::generate_name();
    let table_name = options.table_name.as_deref().unwrap_or(&generated);
    let table_schema = if let Some(schema) = schema_from_url(url) {
        schema
    } else if let Some(schema) = &options.table_schema {
        schema.clone()
    } else {
        DEFAULT_SCHEMA.to_string()
    };

    Table::new(table_schema, table_name)
}

/// https://dev.mysql.com/doc/refman/8.4/en/data-types.html
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug)]
pub enum MySqlDataType {
    BIGINT,
    BINARY(u8),
    BIT(u8),
    BLOB,
    CHAR(u8),
    DATE,
    DATETIME,
    DECIMAL(u8, u8),
    DOUBLE,
    ENUM(Vec<String>),
    FLOAT,
    GEOMETRY,
    GEOMETRYCOLLECTION,
    INT,
    JSON,
    LINESTRING,
    LONGBLOB,
    LONGTEXT,
    MEDIUMBLOB,
    MEDIUMINT,
    MEDIUMTEXT,
    MULTILINESTRING,
    MULTIPOINT,
    MULTIPOLYGON,
    POINT,
    POLYGON,
    SET(Vec<String>),
    SMALLINT,
    TEXT,
    TIME,
    TIMESTAMP,
    TINYBLOB,
    TINYINT,
    TINYTEXT,
    VARBINARY(u16),
    VARCHAR(u16),
    YEAR,
}

impl MySqlDataType {
    fn from_type(column_type: &str) -> MigrisResult<Self> {
        let type_parts: Vec<&str> = column_type.trim_end_matches(')').splitn(2, '(').collect();

        Ok(match type_parts[0].to_uppercase().as_str() {
            "BIGINT" => Self::BIGINT,
            "BINARY" => {
                if type_parts.len() < 2 {
                    return Err(MigrisError::GeneralError(format!(
                        "missing length specifier in column type '{}'",
                        column_type
                    )));
                }

                Self::BINARY(type_parts[1].parse().map_err(|err| {
                    MigrisError::GeneralError(format!(
                        "failed to parse '{}' into length specifier for column type '{}': {}",
                        type_parts[1], column_type, err
                    ))
                })?)
            }
            "BIT" => {
                if type_parts.len() < 2 {
                    return Err(MigrisError::GeneralError(format!(
                        "missing length specifier in column type '{}'",
                        column_type
                    )));
                }

                Self::BIT(type_parts[1].parse().map_err(|err| {
                    MigrisError::GeneralError(format!(
                        "failed to parse '{}' into length specifier for column type '{}': {}",
                        type_parts[1], column_type, err
                    ))
                })?)
            }
            "BLOB" => Self::BLOB,
            "CHAR" => {
                if type_parts.len() < 2 {
                    return Err(MigrisError::GeneralError(format!(
                        "missing length specifier in column type '{}'",
                        column_type
                    )));
                }

                Self::CHAR(type_parts[1].parse().map_err(|err| {
                    MigrisError::GeneralError(format!(
                        "failed to parse '{}' into length specifier for column type '{}': {}",
                        type_parts[1], column_type, err
                    ))
                })?)
            }
            "DATE" => Self::DATE,
            "DATETIME" => Self::DATETIME,
            "DECIMAL" => {
                if type_parts.len() < 2 {
                    return Err(MigrisError::GeneralError(format!(
                        "missing precision or scale specifier in column type '{}'",
                        column_type
                    )));
                }

                let (precision, scale) = type_parts[1]
                    .split_once(',')
                    .ok_or_else(|| {
                        MigrisError::GeneralError(format!(
                            "failed to split '{}' into precision and scale specifiers for column type '{}'",
                            type_parts[1], column_type
                        ))
                    })?;

                Self::DECIMAL(precision.parse().map_err(|err| {
                    MigrisError::GeneralError(format!(
                        "failed to parse '{}' into precision specifier for column type '{}': {}",
                        type_parts[1], column_type, err
                    ))
                })?, scale.parse().map_err(|err| {
                    MigrisError::GeneralError(format!(
                        "failed to parse '{}' into scale specifier for column type '{}': {}",
                        type_parts[2], column_type, err
                    ))
                })?)
            }
            "DOUBLE" => Self::DOUBLE,
            "ENUM" => {
                if type_parts.len() < 2 {
                    return Err(MigrisError::GeneralError(format!(
                        "missing enum values in column type '{}'",
                        column_type
                    )));
                }

                Self::ENUM(
                    type_parts[1]
                        .split(',')
                        .map(|v| v.trim_matches('\'').to_string())
                        .collect(),
                )
            }
            "FLOAT" => Self::FLOAT,
            "GEOMETRY" => Self::GEOMETRY,
            "GEOMCOLLECTION" => Self::GEOMETRYCOLLECTION,
            "INT" => Self::INT,
            "JSON" => Self::JSON,
            "LINESTRING" => Self::LINESTRING,
            "LONGBLOB" => Self::LONGBLOB,
            "LONGTEXT" => Self::LONGTEXT,
            "MEDIUMBLOB" => Self::MEDIUMBLOB,
            "MEDIUMINT" => Self::MEDIUMINT,
            "MEDIUMTEXT" => Self::MEDIUMTEXT,
            "MULTILINESTRING" => Self::MULTILINESTRING,
            "MULTIPOINT" => Self::MULTIPOINT,
            "MULTIPOLYGON" => Self::MULTIPOLYGON,
            "POINT" => Self::POINT,
            "POLYGON" => Self::POLYGON,
            "SET" => {
                if type_parts.len() < 2 {
                    return Err(MigrisError::GeneralError(format!(
                        "missing set values in column type '{}'",
                        column_type
                    )));
                }

                Self::SET(
                    type_parts[1]
                        .split(',')
                        .map(|v| v.trim_matches('\'').to_string())
                        .collect(),
                )
            }
            "SMALLINT" => Self::SMALLINT,
            "TEXT" => Self::TEXT,
            "TIME" => Self::TIME,
            "TIMESTAMP" => Self::TIMESTAMP,
            "TINYBLOB" => Self::TINYBLOB,
            "TINYINT" => Self::TINYINT,
            "TINYTEXT" => Self::TINYTEXT,
            "VARBINARY" => {
                if type_parts.len() < 2 {
                    return Err(MigrisError::GeneralError(format!(
                        "missing length specifier in column type '{}'",
                        column_type
                    )));
                }

                Self::VARBINARY(type_parts[1].parse().map_err(|err| {
                    MigrisError::GeneralError(format!(
                        "failed to parse '{}' into length specifier for column type '{}': {}",
                        type_parts[1], column_type, err
                    ))
                })?)
            }
            "VARCHAR" => {
                if type_parts.len() < 2 {
                    return Err(MigrisError::GeneralError(format!(
                        "missing length specifier in column type '{}'",
                        column_type
                    )));
                }

                Self::VARCHAR(type_parts[1].parse().map_err(|err| {
                    MigrisError::GeneralError(format!(
                        "failed to parse '{}' into length specifier for column type '{}': {}",
                        type_parts[1], column_type, err
                    ))
                })?)
            }
            "YEAR" => Self::YEAR,
            _ => {
                return Err(MigrisError::GeneralError(format!(
                    "failed to convert '{}' to mysql data type",
                    column_type
                )));
            }
        })
    }
}

impl std::fmt::Display for MySqlDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = match self {
            Self::BIGINT => "BIGINT",
            Self::BINARY(len) => &format!("BINARY({})", len),
            Self::BIT(len) => &format!("BIT({})", len),
            Self::BLOB => "BLOB",
            Self::CHAR(len) => &format!("CHAR({})", len),
            Self::DATE => "DATE",
            Self::DATETIME => "DATETIME",
            Self::DECIMAL(precision, scale) => &format!("DECIMAL({},{})", precision, scale),
            Self::DOUBLE => "DOUBLE",
            Self::ENUM(values) => &format!("ENUM('{}')", values.join("','")),
            Self::FLOAT => "FLOAT",
            Self::GEOMETRY => "GEOMETRY",
            Self::GEOMETRYCOLLECTION => "GEOMCOLLECTION",
            Self::INT => "INT",
            Self::JSON => "JSON",
            Self::LINESTRING => "LINESTRING",
            Self::LONGBLOB => "LONGBLOB",
            Self::LONGTEXT => "LONGTEXT",
            Self::MEDIUMBLOB => "MEDIUMBLOB",
            Self::MEDIUMINT => "MEDIUMINT",
            Self::MEDIUMTEXT => "MEDIUMTEXT",
            Self::MULTILINESTRING => "MULTILINESTRING",
            Self::MULTIPOINT => "MULTIPOINT",
            Self::MULTIPOLYGON => "MULTIPOLYGON",
            Self::POINT => "POINT",
            Self::POLYGON => "POLYGON",
            Self::SET(values) => &format!("SET('{}')", values.join("','")),
            Self::SMALLINT => "SMALLINT",
            Self::TEXT => "TEXT",
            Self::TIME => "TIME",
            Self::TIMESTAMP => "TIMESTAMP",
            Self::TINYBLOB => "TINYBLOB",
            Self::TINYINT => "TINYINT",
            Self::TINYTEXT => "TINYTEXT",
            Self::VARBINARY(len) => &format!("VARBINARY({})", len),
            Self::VARCHAR(len) => &format!("VARCHAR({})", len),
            Self::YEAR => "YEAR",
        };

        write!(f, "{}", display)
    }
}

impl Row {
    fn from_mysql(sqlx_row: sqlx::mysql::MySqlRow, columns: &[Column]) -> MigrisResult<Self> {
        let mut row = Self::new();

        for column in columns {
            let value = sqlx_row
                .try_get_raw(column.ordinal)
                .map_err(|err| MigrisError::ValueError(err.to_string()))?;

            row.values.push(Value::from_mysql(value, column)?);
        }

        Ok(row)
    }
}

impl Value {
    fn from_mysql(value: MySqlValueRef, column: &Column) -> MigrisResult<Self> {
        // Check if the value is null first.
        if value.is_null() {
            return Ok(Value::Null);
        }

        match column.column_type.as_mysql() {
            MySqlDataType::BIGINT => {
                if column.is_unsigned() {
                    Ok(Value::U64(decode_sqlx(value)?))
                } else {
                    Ok(Value::I64(decode_sqlx::<_, MySql, _>(value)?))
                }
            }
            MySqlDataType::BINARY(_)
            | MySqlDataType::BLOB
            | MySqlDataType::LONGBLOB
            | MySqlDataType::MEDIUMBLOB
            | MySqlDataType::TINYBLOB
            | MySqlDataType::VARBINARY(_) => Ok(Value::Bytes(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlDataType::BIT(_) => Ok(Value::U64(decode_sqlx(value)?)),
            MySqlDataType::CHAR(_)
            | MySqlDataType::JSON
            | MySqlDataType::LONGTEXT
            | MySqlDataType::MEDIUMTEXT
            | MySqlDataType::TEXT
            | MySqlDataType::TINYTEXT
            | MySqlDataType::VARCHAR(_) => Ok(Value::String(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlDataType::DATE => {
                let date: NaiveDate = decode_sqlx(value)?;
                let date: NaiveDateTime = date.and_hms_opt(0, 0, 0).ok_or(
                    MigrisError::ValueError("failed to convert date to datetime".into()),
                )?;

                Ok(Value::Date(Utc.from_utc_datetime(&date)))
            }
            MySqlDataType::DATETIME => {
                let date: NaiveDateTime = decode_sqlx(value)?;
                Ok(Value::Date(Utc.from_utc_datetime(&date)))
            }
            MySqlDataType::DECIMAL(_, _) => Ok(Value::Decimal(decode_sqlx(value)?)),
            MySqlDataType::DOUBLE => Ok(Value::F64(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlDataType::ENUM(_) => Ok(Value::String(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlDataType::FLOAT => Ok(Value::F32(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlDataType::GEOMETRY
            | MySqlDataType::GEOMETRYCOLLECTION
            | MySqlDataType::LINESTRING
            | MySqlDataType::MULTILINESTRING
            | MySqlDataType::MULTIPOINT
            | MySqlDataType::MULTIPOLYGON
            | MySqlDataType::POINT
            | MySqlDataType::POLYGON => Ok(Value::Bytes(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlDataType::INT | MySqlDataType::MEDIUMINT => {
                if column.is_unsigned() {
                    Ok(Value::U32(decode_sqlx(value)?))
                } else {
                    Ok(Value::I32(decode_sqlx::<_, MySql, _>(value)?))
                }
            }
            MySqlDataType::SET(_) => Ok(Value::Bytes(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlDataType::SMALLINT => {
                if column.is_unsigned() {
                    Ok(Value::U16(decode_sqlx(value)?))
                } else {
                    Ok(Value::I16(decode_sqlx::<_, MySql, _>(value)?))
                }
            }
            MySqlDataType::TIME => Ok(Value::Time(decode_sqlx(value)?)),
            MySqlDataType::TIMESTAMP => Ok(Value::Date(decode_sqlx(value)?)),
            MySqlDataType::TINYINT => {
                if column.is_unsigned() {
                    Ok(Value::U8(decode_sqlx(value)?))
                } else {
                    Ok(Value::I8(decode_sqlx(value)?))
                }
            }
            MySqlDataType::YEAR => Ok(Value::U16(decode_sqlx(value)?)),
        }
    }
}

impl Encode<'_, MySql> for Value {
    fn encode_by_ref(
        &self,
        buf: &mut <MySql as sqlx::Database>::ArgumentBuffer<'_>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        match self {
            Value::Null => Ok(sqlx::encode::IsNull::Yes),
            Value::Bytes(value) => <Vec<u8> as Encode<'_, MySql>>::encode_by_ref(value, buf),
            Value::Date(value) => value.encode_by_ref(buf),
            Value::Decimal(value) => <Decimal as Encode<'_, MySql>>::encode_by_ref(value, buf),
            Value::String(value) => <String as Encode<'_, MySql>>::encode_by_ref(value, buf),
            Value::Time(value) => value.encode_by_ref(buf),
            Value::F32(value) => <f32 as Encode<'_, MySql>>::encode_by_ref(value, buf),
            Value::F64(value) => <f64 as Encode<'_, MySql>>::encode_by_ref(value, buf),
            Value::I8(value) => value.encode_by_ref(buf),
            Value::I16(value) => <i16 as Encode<'_, MySql>>::encode_by_ref(value, buf),
            Value::I32(value) => <i32 as Encode<'_, MySql>>::encode_by_ref(value, buf),
            Value::I64(value) => <i64 as Encode<'_, MySql>>::encode_by_ref(value, buf),
            Value::U8(value) => value.encode_by_ref(buf),
            Value::U16(value) => value.encode_by_ref(buf),
            Value::U32(value) => value.encode_by_ref(buf),
            Value::U64(value) => value.encode_by_ref(buf),
        }
    }

    fn produces(&self) -> Option<<MySql as sqlx::Database>::TypeInfo> {
        match self {
            Value::Null => None,
            Value::Bytes(_) => Some(<Vec<u8> as Type<MySql>>::type_info()),
            Value::Date(_) => Some(<DateTime<Utc> as Type<MySql>>::type_info()),
            Value::Decimal(_) => Some(<Decimal as Type<MySql>>::type_info()),
            Value::String(_) => Some(<String as Type<MySql>>::type_info()),
            Value::Time(_) => Some(<NaiveTime as Type<MySql>>::type_info()),
            Value::F32(_) => Some(<f32 as Type<MySql>>::type_info()),
            Value::F64(_) => Some(<f64 as Type<MySql>>::type_info()),
            Value::I8(_) => Some(<i8 as Type<MySql>>::type_info()),
            Value::I16(_) => Some(<i16 as Type<MySql>>::type_info()),
            Value::I32(_) => Some(<i32 as Type<MySql>>::type_info()),
            Value::I64(_) => Some(<i64 as Type<MySql>>::type_info()),
            Value::U8(_) => Some(<u8 as Type<MySql>>::type_info()),
            Value::U16(_) => Some(<u16 as Type<MySql>>::type_info()),
            Value::U32(_) => Some(<u32 as Type<MySql>>::type_info()),
            Value::U64(_) => Some(<u64 as Type<MySql>>::type_info()),
        }
    }
}

impl Type<MySql> for Value {
    fn type_info() -> <MySql as sqlx::Database>::TypeInfo {
        // This can be set to any type's info as it will be overridden by the `produces` method above.
        <u8 as Type<MySql>>::type_info()
    }
}
