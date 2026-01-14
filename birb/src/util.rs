use crate::{BirbError, Column};

pub(crate) fn decode_sqlx<'a, T, C, R>(column: &C, row: &'a R) -> Result<T, BirbError>
where
    T: sqlx::Decode<'a, <R as sqlx::Row>::Database> + sqlx::Type<<R as sqlx::Row>::Database>,
    C: Column,
    R: sqlx::Row,
    usize: sqlx::ColumnIndex<R>,
{
    row.try_get(column.ordinal())
        .map_err(|err| BirbError::ValueReadFailed {
            message: err.to_string(),
        })
}
