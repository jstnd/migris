use chrono::TimeZone;
use rust_decimal::Decimal;
use sqlx::{Encode, MySql, Type, ValueRef, mysql::MySqlValueRef};

use crate::{BirbError, BirbResult, Column, Value, mysql::MySqlColumnType, util::decode_sqlx};

impl Value {
    pub fn from_mysql(value: MySqlValueRef, column: &Column) -> BirbResult<Self> {
        // Check if the value is null first.
        if value.is_null() {
            return Ok(Value::Null);
        }

        match column.column_type.as_mysql() {
            MySqlColumnType::BIGINT => {
                if column.is_unsigned() {
                    Ok(Value::U64(decode_sqlx(value)?))
                } else {
                    Ok(Value::I64(decode_sqlx::<_, MySql, _>(value)?))
                }
            }
            MySqlColumnType::BINARY
            | MySqlColumnType::BLOB
            | MySqlColumnType::LONGBLOB
            | MySqlColumnType::MEDIUMBLOB
            | MySqlColumnType::TINYBLOB
            | MySqlColumnType::VARBINARY => Ok(Value::Bytes(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlColumnType::BIT => Ok(Value::U64(decode_sqlx(value)?)),
            MySqlColumnType::CHAR
            | MySqlColumnType::JSON
            | MySqlColumnType::LONGTEXT
            | MySqlColumnType::MEDIUMTEXT
            | MySqlColumnType::TEXT
            | MySqlColumnType::TINYTEXT
            | MySqlColumnType::VARCHAR => Ok(Value::String(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlColumnType::DATE => {
                let date: chrono::NaiveDate = decode_sqlx(value)?;
                let date: chrono::NaiveDateTime = date.and_hms_opt(0, 0, 0).ok_or(
                    BirbError::ValueError("failed to convert date to datetime".into()),
                )?;

                Ok(Value::Date(chrono::Utc.from_utc_datetime(&date)))
            }
            MySqlColumnType::DATETIME => {
                let date: chrono::NaiveDateTime = decode_sqlx(value)?;
                Ok(Value::Date(chrono::Utc.from_utc_datetime(&date)))
            }
            MySqlColumnType::DECIMAL => Ok(Value::Decimal(decode_sqlx(value)?)),
            MySqlColumnType::DOUBLE => Ok(Value::F64(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlColumnType::ENUM => Ok(Value::String(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlColumnType::FLOAT => Ok(Value::F32(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlColumnType::GEOMETRY
            | MySqlColumnType::GEOMETRYCOLLECTION
            | MySqlColumnType::LINESTRING
            | MySqlColumnType::MULTILINESTRING
            | MySqlColumnType::MULTIPOINT
            | MySqlColumnType::MULTIPOLYGON
            | MySqlColumnType::POINT
            | MySqlColumnType::POLYGON => Ok(Value::Bytes(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlColumnType::INT | MySqlColumnType::MEDIUMINT => {
                if column.is_unsigned() {
                    Ok(Value::U32(decode_sqlx(value)?))
                } else {
                    Ok(Value::I32(decode_sqlx::<_, MySql, _>(value)?))
                }
            }
            MySqlColumnType::SET => Ok(Value::Bytes(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlColumnType::SMALLINT => {
                if column.is_unsigned() {
                    Ok(Value::U16(decode_sqlx(value)?))
                } else {
                    Ok(Value::I16(decode_sqlx::<_, MySql, _>(value)?))
                }
            }
            MySqlColumnType::TIME => Ok(Value::Time(decode_sqlx(value)?)),
            MySqlColumnType::TIMESTAMP => Ok(Value::Date(decode_sqlx(value)?)),
            MySqlColumnType::TINYINT => {
                if column.is_unsigned() {
                    Ok(Value::U8(decode_sqlx(value)?))
                } else {
                    Ok(Value::I8(decode_sqlx(value)?))
                }
            }
            MySqlColumnType::UNKNOWN => Ok(Value::Bytes(decode_sqlx::<_, MySql, _>(value)?)),
            MySqlColumnType::YEAR => Ok(Value::U16(decode_sqlx(value)?)),
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
            Value::Date(_) => Some(<chrono::DateTime<chrono::Utc> as Type<MySql>>::type_info()),
            Value::Decimal(_) => Some(<Decimal as Type<MySql>>::type_info()),
            Value::String(_) => Some(<String as Type<MySql>>::type_info()),
            Value::Time(_) => Some(<chrono::NaiveTime as Type<MySql>>::type_info()),
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
