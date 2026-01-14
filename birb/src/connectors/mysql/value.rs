use chrono::TimeZone;
use sqlx::{Row, ValueRef, mysql::MySqlRow};

use crate::{
    BirbError, Column, Value,
    mysql::{MySqlColumn, MySqlColumnType},
    util::decode_sqlx,
};

impl Value {
    pub fn from_mysql(column: &MySqlColumn, row: &MySqlRow) -> Result<Self, BirbError> {
        let value =
            row.try_get_raw(column.ordinal())
                .map_err(|err| BirbError::ValueReadFailed {
                    message: err.to_string(),
                })?;

        // Use the raw value to check if it's null first
        if value.is_null() {
            return Ok(Value::Null);
        }

        match column.r#type() {
            MySqlColumnType::BIGINT => {
                if column.is_unsigned() {
                    Ok(Value::U64(decode_sqlx(column, row)?))
                } else {
                    Ok(Value::I64(decode_sqlx(column, row)?))
                }
            }
            MySqlColumnType::BINARY
            | MySqlColumnType::BLOB
            | MySqlColumnType::LONGBLOB
            | MySqlColumnType::MEDIUMBLOB
            | MySqlColumnType::TINYBLOB
            | MySqlColumnType::VARBINARY => Ok(Value::Bytes(decode_sqlx(column, row)?)),
            MySqlColumnType::BIT => Ok(Value::U64(decode_sqlx(column, row)?)),
            MySqlColumnType::CHAR
            | MySqlColumnType::JSON
            | MySqlColumnType::LONGTEXT
            | MySqlColumnType::MEDIUMTEXT
            | MySqlColumnType::TEXT
            | MySqlColumnType::TINYTEXT
            | MySqlColumnType::VARCHAR => Ok(Value::String(decode_sqlx(column, row)?)),
            MySqlColumnType::DATE => {
                let date: chrono::NaiveDate = decode_sqlx(column, row)?;
                let date: chrono::NaiveDateTime =
                    date.and_hms_opt(0, 0, 0)
                        .ok_or(BirbError::ValueReadFailed {
                            message: "failed to convert date to datetime".to_string(),
                        })?;

                Ok(Value::Date(chrono::Utc.from_utc_datetime(&date)))
            }
            MySqlColumnType::DATETIME => {
                let date: chrono::NaiveDateTime = decode_sqlx(column, row)?;
                Ok(Value::Date(chrono::Utc.from_utc_datetime(&date)))
            }
            MySqlColumnType::DECIMAL => Ok(Value::Decimal(decode_sqlx(column, row)?)),
            MySqlColumnType::DOUBLE => Ok(Value::F64(decode_sqlx(column, row)?)),
            MySqlColumnType::ENUM => Ok(Value::String(decode_sqlx(column, row)?)),
            MySqlColumnType::FLOAT => Ok(Value::F32(decode_sqlx(column, row)?)),
            MySqlColumnType::GEOMETRY
            | MySqlColumnType::GEOMETRYCOLLECTION
            | MySqlColumnType::LINESTRING
            | MySqlColumnType::MULTILINESTRING
            | MySqlColumnType::MULTIPOINT
            | MySqlColumnType::MULTIPOLYGON
            | MySqlColumnType::POINT
            | MySqlColumnType::POLYGON => Ok(Value::Bytes(decode_sqlx(column, row)?)),
            MySqlColumnType::INT | MySqlColumnType::MEDIUMINT => {
                if column.is_unsigned() {
                    Ok(Value::U32(decode_sqlx(column, row)?))
                } else {
                    Ok(Value::I32(decode_sqlx(column, row)?))
                }
            }
            MySqlColumnType::SET => Ok(Value::Bytes(decode_sqlx(column, row)?)),
            MySqlColumnType::SMALLINT => {
                if column.is_unsigned() {
                    Ok(Value::U16(decode_sqlx(column, row)?))
                } else {
                    Ok(Value::I16(decode_sqlx(column, row)?))
                }
            }
            MySqlColumnType::TIME => Ok(Value::Time(decode_sqlx(column, row)?)),
            MySqlColumnType::TIMESTAMP => Ok(Value::Date(decode_sqlx(column, row)?)),
            MySqlColumnType::TINYINT => {
                if column.is_unsigned() {
                    Ok(Value::U8(decode_sqlx(column, row)?))
                } else {
                    Ok(Value::I8(decode_sqlx(column, row)?))
                }
            }
            MySqlColumnType::UNKNOWN => Ok(Value::Bytes(decode_sqlx(column, row)?)),
            MySqlColumnType::YEAR => Ok(Value::U16(decode_sqlx(column, row)?)),
        }
    }
}
