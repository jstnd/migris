use crate::Value;

pub trait Column {
    type Type;

    fn flags(&self) -> &Vec<ColumnFlag>;

    fn name(&self) -> &str;

    fn ordinal(&self) -> usize;

    fn r#type(&self) -> Self::Type;

    fn is_unsigned(&self) -> bool {
        self.flags().contains(&ColumnFlag::Unsigned)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ColumnFlag {
    Unsigned,
}

#[derive(Debug)]
pub struct Row<T>
where
    T: Column,
{
    pub columns: Vec<T>,
    pub values: Vec<Value>,
}

impl<T> Row<T>
where
    T: Column,
{
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            values: Vec::new(),
        }
    }
}

impl<T> Default for Row<T>
where
    T: Column,
{
    fn default() -> Self {
        Self::new()
    }
}
