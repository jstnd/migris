use iced::{
    Element, Length, Padding, padding,
    widget::{Row, button, column, container, row, text},
};

use crate::{
    message::Message,
    widgets::{
        icon::{Icon, icon},
        views::query::QueryView,
    },
};

pub trait TabView {
    fn icon(&self) -> Icon;
    fn title(&self) -> String;
    fn update(&mut self, message: Message);
    fn view(&self) -> Element<'_, Message>;
}

pub struct TabId(usize);

pub struct TabsState {
    tabs: Vec<Box<dyn TabView>>,
    active_tab: TabId,
}

impl TabsState {
    pub fn new() -> Self {
        Self {
            tabs: vec![
                Box::new(QueryView),
                Box::new(QueryView),
                Box::new(QueryView),
            ],
            active_tab: TabId(0),
        }
    }

    pub fn select(&mut self, id: TabId) {
        self.active_tab = id;
    }

    pub fn update(&mut self, id: TabId, message: Message) {
        self.tabs[id.0].update(message);
    }
}

pub struct Tabs<'a> {
    state: &'a TabsState,
}

impl<'a> Tabs<'a> {
    pub fn new(state: &'a TabsState) -> Self {
        Self { state }
    }

    fn view(&self) -> Element<'a, Message> {
        let mut tabs_row = Row::new();

        for tab in &self.state.tabs {
            //let button = button(row![icon(tab.icon()), text(tab.title())].spacing(5));
            tabs_row = tabs_row.push(button(
                row![icon(tab.icon()), text(tab.title())]
                    //.padding(padding::top(5))
                    .spacing(5),
            ));
        }

        container(column![
            tabs_row.padding(padding::top(5)),
            text("TABS VIEW")
                .width(Length::Fill)
                .height(Length::Fill)
                .center()
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl<'a> From<Tabs<'a>> for Element<'a, Message> {
    fn from(tabs: Tabs<'a>) -> Self {
        tabs.view()
    }
}
