use sqlx::{Database, Decode, ValueRef};

use crate::{BirbError, BirbResult};

pub(crate) fn decode_sqlx<'a, T, DB, V>(value: V) -> BirbResult<T>
where
    T: Decode<'a, DB>,
    DB: Database<ValueRef<'a> = V>,
    V: ValueRef<'a>,
{
    T::decode(value).map_err(|err| BirbError::ValueError(err.to_string()))
}
