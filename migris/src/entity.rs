use crate::schema::Index;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct Entity {
    /// The type of database object represented by the entity.
    pub kind: EntityKind,

    /// The schema containing the entity.
    pub schema: String,

    /// The name of the entity within its schema.
    pub name: String,
}

impl Entity {
    pub fn schema(schema: impl Into<String>) -> Self {
        Self {
            kind: EntityKind::Schema,
            schema: schema.into(),
            name: String::new(),
        }
    }

    pub fn id(&self) -> String {
        let mut id = self.schema.clone();

        if !self.name.is_empty() {
            id.push_str(&self.name);
        }

        id
    }

    /// Returns whether the entity is a schema.
    pub fn is_schema(&self) -> bool {
        self.kind == EntityKind::Schema
    }
}

#[derive(Debug)]
pub enum EntityData {
    Table(TableData),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Decode, sqlx::Encode)]
#[sqlx(rename_all = "lowercase")]
pub enum EntityKind {
    Event,
    Function,
    Procedure,
    Schema,
    Table,
    Trigger,
    View,
}

impl<DB> sqlx::Type<DB> for EntityKind
where
    DB: sqlx::Database,
    String: sqlx::Type<DB>,
{
    fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
        <String as sqlx::Type<DB>>::compatible(ty)
    }

    fn type_info() -> <DB as sqlx::Database>::TypeInfo {
        <String as sqlx::Type<DB>>::type_info()
    }
}

// TODO: migrate this into Table struct inside schema.rs
#[derive(Debug)]
pub struct TableData {
    /// The indexes associated with the table.
    pub(crate) indexes: Vec<Index>,
}

impl TableData {
    /// Returns the indexes associated with the table.
    pub fn indexes(&self) -> &[Index] {
        &self.indexes
    }
}
