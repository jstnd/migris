use gpui::{Action, SharedString};

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
pub enum AppAction {
    RunSql,
    RunSqlSelection,
}

#[derive(Clone, Copy)]
pub enum EventSource {
    Tab(usize),
}

#[derive(Clone)]
pub enum ApplicationEvent {
    AddConnection,
    RunSql(SharedString, EventSource),
}

#[derive(Clone)]
pub enum TabEvent {
    RunSql(SharedString),
}
