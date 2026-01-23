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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnFlag {
    Unsigned,
}

#[derive(Debug)]
pub struct Row {
    pub values: Vec<Value>,
}

impl Row {
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}
