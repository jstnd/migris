use sqlx::{Column as SqlxColumn, Row as SqlxRow, TypeInfo};

use crate::{BirbError, BirbResult, Column, ColumnFlag, ColumnType, Row, Value};

impl Column {
    pub fn from_mysql(column: &sqlx::mysql::MySqlColumn) -> Self {
        let mut flags = Vec::new();
        let type_parts: Vec<&str> = column.type_info().name().split_whitespace().collect();

        if type_parts.len() > 1 && type_parts[1] == "UNSIGNED" {
            flags.push(ColumnFlag::Unsigned);
        }

        Self {
            column_type: ColumnType::MySql(MySqlColumnType::from(type_parts[0])),
            flags,
            name: column.name().to_string(),
            ordinal: column.ordinal(),
        }
    }
}

/// https://dev.mysql.com/doc/refman/8.4/en/data-types.html
#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, Debug)]
pub enum MySqlColumnType {
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
    UNKNOWN,
    VARBINARY,
    VARCHAR,
    YEAR,
}

impl From<&str> for MySqlColumnType {
    fn from(value: &str) -> Self {
        match value.to_uppercase().as_str() {
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
            _ => Self::UNKNOWN,
        }
    }
}

impl Row {
    pub fn from_mysql(sqlx_row: sqlx::mysql::MySqlRow, columns: &[Column]) -> BirbResult<Self> {
        let mut row = Self::new();

        for column in columns {
            let value = sqlx_row
                .try_get_raw(column.ordinal)
                .map_err(|err| BirbError::ValueError(err.to_string()))?;

            row.values.push(Value::from_mysql(value, column)?);
        }

        Ok(row)
    }
}
