use iced::{
    Alignment, Element, Length,
    widget::{Row, button, column, container, row, scrollable, text, text_input},
};
use migris::driver::EntityKind;

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
                    text_input("Filter...", app.tree_state.current_filter())
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
                Tree::new(&app.tree_state, |item| {
                    let mut row = Row::new();

                    if item.has_children() {
                        let chevron = if item.is_expanded() {
                            Icon::ChevronDown
                        } else {
                            Icon::ChevronRight
                        };

                        row = row.push(icon(chevron));
                    }

                    let entity_icon = match item.value().kind {
                        EntityKind::Schema => Icon::Database,
                        EntityKind::Table => Icon::Table,
                        EntityKind::View => Icon::Eye,
                    };

                    row.push(icon(entity_icon))
                        .push(text(&item.value().name))
                        .align_y(Alignment::Center)
                        .spacing(5)
                        .into()
                })
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
