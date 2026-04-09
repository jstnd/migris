use gpui::SharedString;

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
