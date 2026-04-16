use std::ops::Deref;

use gpui::{Action, SharedString};
use migris::Entity as MigrisEntity;

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
pub enum AppAction {
    RunSql,
    RunSqlSelection,
}

#[derive(Debug, Clone)]
pub enum AppEventKind {
    AddConnection,
    OpenEntity(MigrisEntity),
    RunSql(RunSql),
}

#[derive(Debug, Clone)]
pub struct AppEvent {
    /// The kind for the event.
    kind: AppEventKind,

    /// The optional event id.
    id: Option<EventId>,

    /// The optional event source.
    source: Option<EventSource>,
}

impl AppEvent {
    /// Creates a new [`AppEvent`] with the given [`AppEventKind`].
    pub fn new(kind: AppEventKind) -> Self {
        Self {
            kind,
            id: None,
            source: None,
        }
    }

    /// Sets the id for the event.
    pub fn with_id(mut self, id: impl Into<EventId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the source for the event.
    pub fn with_source(mut self, source: EventSource) -> Self {
        self.source = Some(source);
        self
    }

    /// Returns the id for the event.
    pub fn id(&self) -> Option<&EventId> {
        self.id.as_ref()
    }

    /// Returns the kind for the event.
    pub fn kind(&self) -> &AppEventKind {
        &self.kind
    }

    /// Returns the source for the event.
    pub fn source(&self) -> Option<&EventSource> {
        self.source.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct EventId(SharedString);

impl Deref for EventId {
    type Target = SharedString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<SharedString> for EventId {
    fn from(value: SharedString) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EventSource {
    Tab(usize),
}

#[derive(Debug, Clone)]
pub struct RunSql {
    /// The SQL to run.
    pub sql: SharedString,

    /// Whether to show query progress.
    pub show_progress: bool,

    /// Whether the results should be returned as a stream.
    pub stream: bool,
}
