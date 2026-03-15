use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, space, text},
};
use migris::driver::Entity;

use crate::{
    message::Message,
    widgets::{
        icon::{Icon, icon},
        tree::{Tree, TreeState},
    },
};

pub fn connection_panel<'a>(tree_state: &'a TreeState<Entity>) -> Element<'a, Message> {
    scrollable(
        container(column![
            row![
                text("Connections"),
                space::horizontal().width(Length::Fill),
                button(icon(Icon::Plus))
                    .style(button::background)
                    .on_press(Message::ConnectionAdded)
            ]
            .align_y(Alignment::Center),
            Tree::new(tree_state, |item| text(&item.name).into())
                .on_select(Message::TreeItemSelected)
                .on_toggle(Message::TreeItemToggled)
        ])
        .padding(10)
        .clip(true),
    )
    .into()
}
