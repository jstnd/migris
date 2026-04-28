use std::{collections::HashMap, rc::Rc};

use gpui::{Action, App, Context, EventEmitter, Global, SharedString, Window};
use migris::{Entity as MigrisEntity, data::QueryResult};
use uuid::Uuid;

use crate::connections::ConnectionId;

pub struct EventManager {
    /// Tracks the active events by [`EventId`].
    events: HashMap<EventId, Event>,
}

impl Global for EventManager {}

impl EventManager {
    /// Creates a new [`EventManager`].
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
        }
    }

    /// Emits the given [`Event`].
    pub fn emit<E>(cx: &mut Context<E>, event: Event)
    where
        E: EventEmitter<EventId>,
    {
        let id = event.id;
        Self::global_mut(cx).push(event);
        cx.emit(id);
    }

    /// Returns a reference to the global [`EventManager`].
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Returns a mutable reference to the global [`EventManager`].
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    /// Completes the event with the given [`EventId`].
    pub fn complete(&mut self, id: &EventId) {
        self.events.remove(id);
    }

    /// Returns a reference to the event with the given [`EventId`], if one is found.
    pub fn get(&self, id: &EventId) -> Option<&Event> {
        self.events.get(id)
    }

    /// Inserts an event into the event map.
    fn push(&mut self, event: Event) {
        self.events.insert(event.id, event);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(Uuid);

pub struct Event {
    /// The id of the event.
    id: EventId,

    /// The variant of the event.
    variant: EventVariant,
}

impl Event {
    /// Creates a new [`Event`].
    pub fn new(variant: impl Into<EventVariant>) -> Self {
        Self {
            id: EventId(Uuid::now_v7()),
            variant: variant.into(),
        }
    }

    /// Returns the variant of the event.
    pub fn variant(&self) -> &EventVariant {
        &self.variant
    }
}

pub enum EventVariant {
    OpenConnection(ConnectionId),
    OpenEntity(MigrisEntity),
    RunSql(RunSqlEvent),
}

#[derive(Clone)]
pub struct RunSqlEvent {
    /// The SQL to run.
    pub sql: SharedString,

    /// Whether to show query progress.
    pub show_progress: bool,

    /// Whether the results should be returned as a stream.
    pub stream: bool,

    /// An optional callback used when a query result is retrieved.
    pub on_result: Option<Rc<dyn Fn(QueryResult, &mut Window, &mut App) + 'static>>,
}

impl RunSqlEvent {
    /// Creates a new [`RunSqlEvent`].
    pub fn new(sql: impl Into<SharedString>) -> Self {
        Self {
            sql: sql.into(),
            show_progress: false,
            stream: false,
            on_result: None,
        }
    }

    /// Creates a new [`RunSqlEvent`] that will return results as streams.
    pub fn stream(sql: impl Into<SharedString>) -> Self {
        Self {
            sql: sql.into(),
            show_progress: false,
            stream: true,
            on_result: None,
        }
    }

    /// Sets the event to show progress.
    pub fn show_progress(mut self) -> Self {
        self.show_progress = true;
        self
    }

    /// Sets the callback used when a query result is retrieved.
    pub fn on_result(mut self, f: impl Fn(QueryResult, &mut Window, &mut App) + 'static) -> Self {
        self.on_result = Some(Rc::new(f));
        self
    }
}

impl From<RunSqlEvent> for EventVariant {
    fn from(event: RunSqlEvent) -> Self {
        EventVariant::RunSql(event)
    }
}

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
pub enum AppAction {
    RunSql,
    RunSqlSelection,
}
