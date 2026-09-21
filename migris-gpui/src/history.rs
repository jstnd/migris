use std::fmt::Display;

use chrono::Datelike;
use indexmap::IndexMap;
use sqlx::types::chrono::{DateTime, Local, Utc};
use uuid::Uuid;

use crate::{connections::ConnectionId, shared};

#[derive(Debug)]
pub struct QueryHistory {
    pub groups: Vec<QueryHistoryGroup>,
}

impl QueryHistory {
    /// Creates a new [`QueryHistory`].
    pub fn new(items: Vec<QueryHistoryItem>) -> Self {
        let items_by_id: IndexMap<Uuid, Vec<QueryHistoryItem>> =
            items.into_iter().fold(IndexMap::new(), |mut map, item| {
                map.entry(item.execute_id).or_default().push(item);
                map
            });

        Self {
            groups: items_by_id
                .into_iter()
                .map(|(id, items)| QueryHistoryGroup {
                    connection_id: items[0].connection_id,
                    execute_id: id,
                    items,
                })
                .collect(),
        }
    }
}

#[derive(Debug)]
pub struct QueryHistoryGroup {
    pub connection_id: Option<ConnectionId>,
    pub execute_id: Uuid,
    pub items: Vec<QueryHistoryItem>,
}

impl QueryHistoryGroup {
    /// Creates a new [`QueryHistoryGroup`].
    pub fn new(connection_id: ConnectionId) -> Self {
        Self {
            connection_id: Some(connection_id),
            execute_id: Uuid::now_v7(),
            items: Vec::new(),
        }
    }

    /// Adds a new history item to the group and returns a mutable reference to the item.
    pub fn add(&mut self, query: &str) -> &mut QueryHistoryItem {
        self.items.push_mut(QueryHistoryItem::new(
            self.execute_id,
            self.connection_id,
            query,
        ))
    }

    /// Returns the date display for the group.
    pub fn date_display(&self) -> String {
        let Some(item) = self.items.first() else {
            return String::new();
        };

        let date_now = Local::now().date_naive();
        let executed_local = item.executed_at.with_timezone(&Local);
        let executed_date = executed_local.date_naive();
        let format = if executed_date == date_now {
            "Today • %-I:%M %p"
        } else if let Some(yesterday) = date_now.pred_opt()
            && executed_date == yesterday
        {
            "Yesterday • %-I:%M %p"
        } else if executed_local.year() == date_now.year() {
            "%b %-d • %-I:%M %p"
        } else {
            "%b %-d, %Y • %-I:%M %p"
        };

        executed_local.format(format).to_string()
    }

    /// Returns the duration display for the group.
    pub fn duration_display(&self) -> String {
        let total_duration: u64 = self.items.iter().map(|item| item.duration_ms).sum();
        shared::format_ms(total_duration)
    }

    /// Returns the header display for the group.
    pub fn header(&self) -> String {
        format!("{} • {}", self.date_display(), self.duration_display())
    }
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct QueryHistoryId(Uuid);

impl QueryHistoryId {
    /// Creates a new [`QueryHistoryId`].
    fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Display for QueryHistoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, Default, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum QueryStatus {
    #[default]
    None,
    Success,
    Failed,
}

#[derive(Debug, sqlx::FromRow)]
pub struct QueryHistoryItem {
    pub id: QueryHistoryId,
    pub execute_id: Uuid,
    pub connection_id: Option<ConnectionId>,
    pub query: String,
    pub executed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub status: QueryStatus,
    pub error: String,
    pub rows_affected: Option<u64>,
    pub rows_returned: Option<u64>,
}

impl QueryHistoryItem {
    /// Creates a new [`QueryHistoryItem`].
    fn new(execute_id: Uuid, connection_id: Option<ConnectionId>, query: &str) -> Self {
        Self {
            id: QueryHistoryId::new(),
            execute_id,
            connection_id,
            query: query.to_string(),
            executed_at: Utc::now(),
            duration_ms: 0,
            status: QueryStatus::default(),
            error: String::new(),
            rows_affected: None,
            rows_returned: None,
        }
    }

    /// Returns the row count display for the item.
    pub fn row_display(&self) -> String {
        // TODO: handle rows affected everywhere
        let rows = self.rows_returned.unwrap_or(0);
        if rows == 1 {
            "1 row".to_string()
        } else {
            format!("{} rows", rows)
        }
    }
}
