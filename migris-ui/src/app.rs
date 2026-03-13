use iced::{
    Border, Color, Element, Length,
    widget::{
        PaneGrid, container,
        pane_grid::{self, Configuration},
        text,
    },
};
use migris::driver::Entity;

use crate::{
    message::Message,
    widgets::{self, tree::Tree},
};

#[derive(Clone, Copy)]
pub enum Panel {
    Connections,
    Tabs,
}

pub struct Application {
    pub grid_state: pane_grid::State<Panel>,
    pub tree_state: widgets::tree::TreeState<Entity>,
}

impl Application {
    pub fn new() -> Self {
        let grid_state = pane_grid::State::with_configuration(Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: 0.25,
            a: Box::new(Configuration::Pane(Panel::Connections)),
            b: Box::new(Configuration::Pane(Panel::Tabs)),
        });

        let tree_state = widgets::tree::TreeState::new(vec![]);

        Self {
            grid_state,
            tree_state,
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::PanelResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.grid_state.resize(split, ratio);
            }
            Message::TreeItemSelected(id) => {
                println!("{:?}", id);
            }
            Message::TreeItemToggled(id) => {
                self.tree_state.toggle(&id);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let pane_grid = PaneGrid::new(&self.grid_state, |_, pane, _| {
            pane_grid::Content::new(
                match pane {
                    Panel::Connections => container(
                        Tree::new(&self.tree_state, |item| text(&item.name).into())
                            .on_select(Message::TreeItemSelected)
                            .on_toggle(Message::TreeItemToggled),
                    ),
                    Panel::Tabs => container(
                        text("TAB VIEW")
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .center(),
                    ),
                }
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    border: match pane {
                        Panel::Connections => Border::default(),
                        Panel::Tabs => Border {
                            color: Color::WHITE,
                            width: 1.0,
                            radius: 5.0.into(),
                        },
                    },
                    ..Default::default()
                }),
            )
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .on_resize(10, Message::PanelResized);

        container(pane_grid).padding(0).into()
    }
}
