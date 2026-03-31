use gpui::SharedString;

#[derive(Clone, Copy)]
pub enum EventSource {
    Tab(usize),
}

#[derive(Clone)]
pub enum ApplicationEvent {
    AddConnection,
    RunQuery(SharedString, EventSource),
}

#[derive(Clone)]
pub enum TabEvent {
    RunQuery(SharedString),
}
