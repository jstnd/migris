#[derive(Debug)]
pub enum Value {
    Null,
    Bytes(Vec<u8>),
    Date(chrono::DateTime<chrono::Utc>),
    Decimal(rust_decimal::Decimal),
    String(String),
    Time(chrono::NaiveTime),
    F32(f32),
    F64(f64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display = match self {
            Value::Null => String::from(""),
            Value::Bytes(value) => String::from_utf8(value.to_vec()).unwrap_or_default(),
            Value::Date(value) => value.to_string(),
            Value::Decimal(value) => value.to_string(),
            Value::String(value) => value.to_string(),
            Value::Time(value) => value.to_string(),
            Value::F32(value) => value.to_string(),
            Value::F64(value) => value.to_string(),
            Value::I8(value) => value.to_string(),
            Value::I16(value) => value.to_string(),
            Value::I32(value) => value.to_string(),
            Value::I64(value) => value.to_string(),
            Value::U8(value) => value.to_string(),
            Value::U16(value) => value.to_string(),
            Value::U32(value) => value.to_string(),
            Value::U64(value) => value.to_string(),
        };

        write!(f, "{}", display)
    }
}
