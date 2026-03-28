#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Entity {
    pub kind: EntityKind,
    pub schema: String,
    pub name: String,
}

impl Entity {
    pub fn schema(schema: impl Into<String>) -> Self {
        Self {
            kind: EntityKind::Schema,
            schema: schema.into(),
            name: String::from(""),
        }
    }

    pub fn id(&self) -> String {
        let mut id = self.schema.clone();

        if !self.name.is_empty() {
            id.push_str(&self.name);
        }

        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Decode, sqlx::Encode)]
#[sqlx(rename_all = "lowercase")]
pub enum EntityKind {
    Schema,
    Table,
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
