use iced::widget::text;

use crate::{
    message::Message,
    widgets::{icon::Icon, tabs::TabView},
};

pub struct QueryView;

impl TabView for QueryView {
    fn icon(&self) -> Icon {
        Icon::Code
    }

    fn title(&self) -> String {
        "Query".into()
    }

    fn update(&mut self, message: Message) {}

    fn view(&self) -> iced::Element<'_, Message> {
        text("test").into()
    }
}
