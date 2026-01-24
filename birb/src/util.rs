use sqlx::{Database, Decode, ValueRef};

use crate::BirbError;

pub(crate) fn decode_sqlx<'a, T, DB, V>(value: V) -> Result<T, BirbError>
where
    T: Decode<'a, DB>,
    DB: Database<ValueRef<'a> = V>,
    V: ValueRef<'a>,
{
    T::decode(value).map_err(|err| BirbError::ValueError {
        message: err.to_string(),
    })
}
