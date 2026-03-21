use std::time::Duration;

use iced::{
    Element,
    widget::{Tooltip, container},
};

pub mod icon;
pub mod panels;
pub mod tabs;
pub mod tree;

pub mod views {
    pub mod query;
}

pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip: &'a str,
) -> Tooltip<'a, Message> {
    iced::widget::tooltip(
        content,
        container(tooltip)
            .style(container::bordered_box)
            .padding(2.5),
        iced::widget::tooltip::Position::Bottom,
    )
    .delay(Duration::from_millis(500))
    .padding(2.5)
}
