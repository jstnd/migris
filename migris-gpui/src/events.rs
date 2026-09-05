use std::{collections::HashMap, rc::Rc};

use gpui::{Action, App, Global, SharedString, Window};
use migris::{Entity as MigrisEntity, EntityData, data::QueryResult};
use uuid::Uuid;

use crate::connections::ConnectionId;

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
pub struct EventEmitted(pub EventId);

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
    pub fn emit(window: &mut Window, cx: &mut App, event: Event) {
        let id = event.id;
        Self::global_mut(cx).push(event);
        window.dispatch_action(Box::new(EventEmitted(id)), cx);
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
    pub id: EventId,

    /// The callbacks for the event.
    pub callbacks: EventCallbacks,

    /// The variant of the event.
    pub variant: EventVariant,
}

impl Event {
    /// Creates a new [`Event`].
    pub fn new(variant: impl Into<EventVariant>) -> Self {
        Self {
            id: EventId(Uuid::now_v7()),
            callbacks: EventCallbacks::new(),
            variant: variant.into(),
        }
    }

    /// Sets the callback used when the event successfully completes.
    pub fn on_complete(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.callbacks.on_complete = Some(Rc::new(f));
        self
    }

    /// Sets the callback used when the event errors.
    pub fn on_error(mut self, f: impl Fn(&mut Window, &mut App, &str) + 'static) -> Self {
        self.callbacks.on_error = Some(Rc::new(f));
        self
    }
}

#[derive(Clone)]
pub struct EventCallbacks {
    /// An optional callback used when the event successfully completes.
    on_complete: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,

    /// An optional callback used when the event errors.
    on_error: Option<Rc<dyn Fn(&mut Window, &mut App, &str) + 'static>>,
}

impl EventCallbacks {
    /// Creates a new [`EventCallbacks`].
    fn new() -> Self {
        Self {
            on_complete: None,
            on_error: None,
        }
    }

    /// Calls the callback used when the event successfully completes, if one exists.
    pub fn on_complete(&self, window: &mut Window, cx: &mut App) {
        if let Some(on_complete) = self.on_complete.clone() {
            on_complete(window, cx);
        }
    }

    /// Calls the callback used when the event errors, if one exists.
    pub fn on_error(&self, window: &mut Window, cx: &mut App, error: &str) {
        if let Some(on_error) = self.on_error.clone() {
            on_error(window, cx, error);
        }
    }
}

pub enum EventVariant {
    LoadEntity(LoadEntityEvent),
    OpenConnection(ConnectionId),
    OpenEntity(MigrisEntity),
    RunSql(RunSqlEvent),
}

#[derive(Clone)]
pub struct LoadEntityEvent {
    /// The entity to load data for.
    pub entity: MigrisEntity,

    /// The callback used when the entity data is retrieved.
    pub on_result: Rc<dyn Fn(&mut Window, &mut App, EntityData) + 'static>,
}

impl LoadEntityEvent {
    /// Creates a new [`LoadEntityEvent`].
    pub fn new(
        entity: MigrisEntity,
        on_result: impl Fn(&mut Window, &mut App, EntityData) + 'static,
    ) -> Self {
        Self {
            entity,
            on_result: Rc::new(on_result),
        }
    }
}

impl From<LoadEntityEvent> for EventVariant {
    fn from(value: LoadEntityEvent) -> Self {
        Self::LoadEntity(value)
    }
}

#[derive(Clone)]
pub struct RunSqlEvent {
    /// The SQL to run.
    pub sql: SharedString,

    /// Whether to show query progress.
    pub show_progress: bool,

    /// Whether the results should be returned as a stream.
    pub stream: bool,

    /// The callback used when a query result is retrieved.
    pub on_result: Rc<dyn Fn(&mut Window, &mut App, QueryResult) + 'static>,
}

impl RunSqlEvent {
    /// Creates a new [`RunSqlEvent`].
    pub fn new(
        sql: impl Into<SharedString>,
        on_result: impl Fn(&mut Window, &mut App, QueryResult) + 'static,
    ) -> Self {
        Self {
            sql: sql.into(),
            show_progress: false,
            stream: false,
            on_result: Rc::new(on_result),
        }
    }

    /// Creates a new [`RunSqlEvent`] that will return results as streams.
    pub fn stream(
        sql: impl Into<SharedString>,
        on_result: impl Fn(&mut Window, &mut App, QueryResult) + 'static,
    ) -> Self {
        Self {
            sql: sql.into(),
            show_progress: false,
            stream: true,
            on_result: Rc::new(on_result),
        }
    }

    /// Sets the event to show progress.
    pub fn show_progress(mut self) -> Self {
        self.show_progress = true;
        self
    }
}

impl From<RunSqlEvent> for EventVariant {
    fn from(event: RunSqlEvent) -> Self {
        EventVariant::RunSql(event)
    }
}
