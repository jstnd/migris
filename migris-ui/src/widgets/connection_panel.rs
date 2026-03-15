use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, text, text_input},
};

use crate::{
    app::Application,
    message::Message,
    widgets::{
        icon::{self, Icon, icon},
        tree::Tree,
    },
};

pub fn connection_panel<'a>(app: &'a Application) -> Element<'a, Message> {
    scrollable(
        container(
            column![
                row![
                    text_input("Filter...", &app.connection_filter)
                        .icon(text_input::Icon {
                            font: icon::FONT_LUCIDE,
                            code_point: Icon::Search.unicode(),
                            size: None,
                            spacing: 5.0,
                            side: text_input::Side::Left,
                        })
                        .width(Length::Fill)
                        .on_input(Message::ConnectionFilterChanged),
                    button(icon(Icon::Plus))
                        .style(button::background)
                        .on_press(Message::ConnectionAdded)
                ]
                .align_y(Alignment::Center)
                .spacing(5),
                Tree::new(&app.tree_state, |item| text(&item.name).into())
                    .on_select(Message::TreeItemSelected)
                    .on_toggle(Message::TreeItemToggled)
            ]
            .spacing(5),
        )
        .clip(true)
        .padding(5),
    )
    .spacing(0)
    .into()
}
