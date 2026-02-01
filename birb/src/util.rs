use std::{ffi::OsStr, path::Path};

use sqlx::{Database, Decode, ValueRef};

use crate::{BirbError, BirbResult};

pub const DEFAULT_SCHEMA: &str = "birb";

pub(crate) fn decode_sqlx<'a, T, DB, V>(value: V) -> BirbResult<T>
where
    T: Decode<'a, DB>,
    DB: Database<ValueRef<'a> = V>,
    V: ValueRef<'a>,
{
    T::decode(value).map_err(|err| BirbError::ValueError(err.to_string()))
}

pub(crate) fn generate_table_name() -> String {
    format!("birb_{}", chrono::Local::now().format("%m%d%Y_%H%M%S%f"))
}

pub(crate) fn get_file_extension(file: &str) -> Option<&str> {
    Path::new(file).extension().and_then(OsStr::to_str)
}
