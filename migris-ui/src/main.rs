mod widgets;

use iced::{
    Border, Color, Element,
    Length::Fill,
    widget::{
        PaneGrid, container,
        pane_grid::{self, Configuration},
        text,
    },
};

use migris::driver::Driver;

use crate::widgets::tree::{Tree, TreeItem, TreeItemId};

#[derive(Debug, Clone)]
enum Message {
    PanelResized(pane_grid::ResizeEvent),
    TreeItemSelected(TreeItemId),
    TreeItemToggled(TreeItemId),
}

#[derive(Clone, Copy)]
enum Panel {
    Connections,
    Tabs,
}

struct Application {
    grid_state: pane_grid::State<Panel>,
    tree_state: widgets::tree::TreeState<DatabaseTable>,
}

struct DatabaseTable {
    pub name: String,
}

impl Application {
    fn new() -> Self {
        let grid_state = pane_grid::State::with_configuration(Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: 0.25,
            a: Box::new(Configuration::Pane(Panel::Connections)),
            b: Box::new(Configuration::Pane(Panel::Tabs)),
        });

        let tree_state = widgets::tree::TreeState::new(vec![
            TreeItem::new(DatabaseTable {
                name: "test_one".into(),
            })
            .child(TreeItem::new(DatabaseTable {
                name: "test_one_child".into(),
            })),
            TreeItem::new(DatabaseTable {
                name: "test_two".into(),
            }),
        ]);

        Self {
            grid_state,
            tree_state,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::PanelResized(pane_grid::ResizeEvent { split, ratio }) => {
                println!("{:?} {}", split, ratio);
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

    fn view(&self) -> Element<'_, Message> {
        let pane_grid = PaneGrid::new(&self.grid_state, |_, pane, _| {
            pane_grid::Content::new(
                match pane {
                    Panel::Connections => container(
                        Tree::new(&self.tree_state, |item| text(&item.name).into())
                            .on_select(Message::TreeItemSelected)
                            .on_toggle(Message::TreeItemToggled),
                    ),
                    Panel::Tabs => container(text("TAB VIEW").width(Fill).height(Fill).center()),
                }
                .width(Fill)
                .height(Fill)
                .style(move |_| container::Style {
                    border: match pane {
                        Panel::Connections => Border::default(),
                        Panel::Tabs => Border {
                            color: Color::WHITE,
                            width: 2.0,
                            radius: 5.0.into(),
                        },
                    },
                    ..Default::default()
                }),
            )
        })
        .width(Fill)
        .height(Fill)
        .on_resize(10, Message::PanelResized);

        container(pane_grid).padding(0).into()
    }
}

#[tokio::main]
async fn main() -> iced::Result {
    let mut connection = migris::mysql::MySqlConnector::new("mysql://root:root@localhost");
    let entities = connection.entities().await.unwrap();
    println!("{:?}", entities);

    iced::application(Application::new, Application::update, Application::view)
        .title("migris")
        .run()
}
