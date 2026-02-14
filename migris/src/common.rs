use std::{ffi::OsStr, path::Path};

use sqlx::{Database, Decode, ValueRef};

use crate::{FileType, MigrisError, MigrisResult};

pub(crate) const DEFAULT_SCHEMA: &str = "migris";

pub(crate) fn decode_sqlx<'a, T, DB, V>(value: V) -> MigrisResult<T>
where
    T: Decode<'a, DB>,
    DB: Database<ValueRef<'a> = V>,
    V: ValueRef<'a>,
{
    T::decode(value).map_err(|err| MigrisError::ValueError(err.to_string()))
}

pub(crate) fn generate_name() -> String {
    format!("migris_{}", chrono::Local::now().format("%m%d%Y_%H%M%S%f"))
}

pub fn get_safe_name(str: &str) -> String {
    // Track if the previous character seen was an underscore.
    let mut prev_underscore = false;

    str.chars()
        .filter_map(|c| match c {
            ' ' | '-' | '—' | '_' => {
                if prev_underscore {
                    // Do not use an underscore if the previous
                    // character seen was already an underscore.
                    None
                } else {
                    // Otherwise, use an underscore as the next character.
                    prev_underscore = true;
                    Some('_')
                }
            }
            c if c.is_alphanumeric() => {
                prev_underscore = false;
                Some(c)
            }
            _ => None,
        })
        .collect()
}

pub fn get_file_stem<P: AsRef<Path>>(path: &P) -> Option<&str> {
    path.as_ref().file_stem().and_then(OsStr::to_str)
}

pub fn get_file_type<P: AsRef<Path>>(path: &P) -> Option<FileType> {
    match path.as_ref().extension().and_then(OsStr::to_str) {
        Some("csv") => Some(FileType::Csv),
        _ => None,
    }
}
