use std::{ffi::OsStr, path::Path};

use sqlx::{Database, Decode, ValueRef};

use crate::{BirbError, BirbResult};

pub(crate) const DEFAULT_SCHEMA: &str = "birb";

const SUPPORTED_FILE_EXT: [&str; 1] = ["csv"];

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

pub fn get_extension<P: AsRef<Path>>(path: &P) -> Option<&str> {
    path.as_ref().extension().and_then(OsStr::to_str)
}

pub fn get_supported_extensions() -> &'static [&'static str] {
    &SUPPORTED_FILE_EXT
}
