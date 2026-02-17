use sqlx::Row as SqlxRow;

use crate::{Column, MigrisError, MigrisResult, Row, Value};

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
    pub(crate) fn from_type(column_type: &str) -> MigrisResult<Self> {
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
