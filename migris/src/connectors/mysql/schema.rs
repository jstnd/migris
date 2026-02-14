use sqlx::{Column as SqlxColumn, Row as SqlxRow, TypeInfo};

use crate::{Column, ColumnFlag, ColumnType, MigrisError, MigrisResult, Row, Value};

impl Column {
    pub fn from_mysql(column: &sqlx::mysql::MySqlColumn) -> MigrisResult<Self> {
        let mut flags = Vec::new();
        let type_parts: Vec<&str> = column.type_info().name().split_whitespace().collect();

        if type_parts.len() > 1 && type_parts[1] == "UNSIGNED" {
            flags.push(ColumnFlag::Unsigned);
        }

        Ok(Self {
            column_type: ColumnType::MySql(MySqlDataType::from_str(type_parts[0])?),
            flags,
            name: column.name().to_string(),
            ordinal: column.ordinal(),
        })
    }
}

/// https://dev.mysql.com/doc/refman/8.4/en/data-types.html
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug)]
pub enum MySqlDataType {
    BIGINT,
    BINARY,
    BIT,
    BLOB,
    CHAR,
    DATE,
    DATETIME,
    DECIMAL,
    DOUBLE,
    ENUM,
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
    SET,
    SMALLINT,
    TEXT,
    TIME,
    TIMESTAMP,
    TINYBLOB,
    TINYINT,
    TINYTEXT,
    VARBINARY,
    VARCHAR,
    YEAR,
}

impl MySqlDataType {
    fn from_str(str: &str) -> MigrisResult<Self> {
        Ok(match str.to_uppercase().as_str() {
            "BIGINT" => Self::BIGINT,
            "BINARY" => Self::BINARY,
            "BIT" => Self::BIT,
            "BLOB" => Self::BLOB,
            "CHAR" => Self::CHAR,
            "DATE" => Self::DATE,
            "DATETIME" => Self::DATETIME,
            "DECIMAL" => Self::DECIMAL,
            "DOUBLE" => Self::DOUBLE,
            "ENUM" => Self::ENUM,
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
            "SET" => Self::SET,
            "SMALLINT" => Self::SMALLINT,
            "TEXT" => Self::TEXT,
            "TIME" => Self::TIME,
            "TIMESTAMP" => Self::TIMESTAMP,
            "TINYBLOB" => Self::TINYBLOB,
            "TINYINT" => Self::TINYINT,
            "TINYTEXT" => Self::TINYTEXT,
            "VARBINARY" => Self::VARBINARY,
            "VARCHAR" => Self::VARCHAR,
            "YEAR" => Self::YEAR,
            _ => {
                return Err(MigrisError::UnsupportedAction(format!(
                    "attempted to convert '{}' to mysql data type",
                    str
                )));
            }
        })
    }
}

impl std::fmt::Display for MySqlDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = match self {
            Self::BIGINT => "BIGINT",
            Self::BINARY => "BINARY(50)",
            Self::BIT => "BIT(1)",
            Self::BLOB => "BLOB",
            Self::CHAR => "CHAR(500)",
            Self::DATE => "DATE",
            Self::DATETIME => "DATETIME",
            Self::DECIMAL => "DECIMAL(20,6)",
            Self::DOUBLE => "DOUBLE",
            Self::ENUM => "ENUM",
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
            Self::SET => "SET",
            Self::SMALLINT => "SMALLINT",
            Self::TEXT => "TEXT",
            Self::TIME => "TIME",
            Self::TIMESTAMP => "TIMESTAMP",
            Self::TINYBLOB => "TINYBLOB",
            Self::TINYINT => "TINYINT",
            Self::TINYTEXT => "TINYTEXT",
            Self::VARBINARY => "VARBINARY(50)",
            Self::VARCHAR => "VARCHAR(500)",
            Self::YEAR => "YEAR",
        };

        write!(f, "{}", display)
    }
}

impl Row {
    pub fn from_mysql(sqlx_row: sqlx::mysql::MySqlRow, columns: &[Column]) -> MigrisResult<Self> {
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
