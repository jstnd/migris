use std::{collections::BTreeMap, sync::Arc};

use iced::{
    Element, Length, Task,
    widget::{
        PaneGrid, container,
        pane_grid::{self, Configuration},
        text,
    },
};
use migris::{
    driver::{Driver, Entity, EntityKind},
    mysql::MySqlConnector,
};

use crate::{
    message::Message,
    widgets::{
        self,
        connection_panel::connection_panel,
        tree::{TreeItem, TreeState},
    },
};

#[derive(Clone, Copy)]
pub enum Panel {
    Connections,
    Tabs,
}

pub struct Application {
    pub driver: Option<Arc<dyn Driver>>,
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

        let tree_state = TreeState::new(vec![]).on_filter(Box::new(Self::is_item_visible));

        Self {
            driver: None,
            grid_state,
            tree_state,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConnectionAdded => {
                return Task::perform(
                    async {
                        let driver: Arc<dyn Driver> = Arc::new(
                            MySqlConnector::new_with_pool("mysql://root:root@localhost").await?,
                        );

                        let entities = driver.entities().await?;
                        Ok((driver, entities))
                    },
                    |result: Result<(Arc<dyn Driver>, Vec<Entity>), anyhow::Error>| match result {
                        Ok((driver, entities)) => Message::ConnectionLoaded(driver, entities),
                        Err(err) => Message::ErrorEncountered(err.to_string()),
                    },
                );
            }
            Message::ConnectionLoaded(driver, entities) => {
                self.driver = Some(driver);

                let items = entities_to_tree(entities);
                self.tree_state.load(items);
            }
            Message::ConnectionFilterChanged(filter) => {
                self.tree_state.filter(filter);
            }
            Message::PanelResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.grid_state.resize(split, ratio);
            }
            Message::TreeItemSelected(id) => {
                println!("{:?}", id);
            }
            Message::TreeItemToggled(id) => {
                self.tree_state.toggle(&id);
            }
            Message::ErrorEncountered(message) => {
                println!("{message}");
            }
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let pane_grid = PaneGrid::new(&self.grid_state, |_, pane, _| {
            pane_grid::Content::new(match pane {
                Panel::Connections => connection_panel(self),
                Panel::Tabs => container(
                    text("TAB VIEW")
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center(),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            })
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .on_resize(10, Message::PanelResized);

        container(pane_grid).padding(0).into()
    }

    fn is_item_visible(item: &TreeItem<Entity>, filter: &str) -> bool {
        if item.value().kind != EntityKind::Schema {
            // TODO: maybe improve this to not make as many allocations?
            let name = item.value().name.to_lowercase();
            return name.contains(&filter.to_lowercase());
        }

        true
    }
}

fn entities_to_tree(entities: Vec<Entity>) -> Vec<TreeItem<Entity>> {
    let mut items = Vec::new();
    let entities_by_schema = entities
        .into_iter()
        .fold(BTreeMap::new(), |mut map, entity| {
            map.entry(entity.schema.clone())
                .or_insert(Vec::new())
                .push(entity);
            map
        });

    for (schema, entities) in entities_by_schema {
        let schema_entity = Entity {
            schema: "".into(),
            name: schema,
            kind: EntityKind::Schema,
        };

        let mut children: Vec<TreeItem<Entity>> = entities.into_iter().map(TreeItem::new).collect();
        children.sort_unstable_by(|a, b| a.value().name.cmp(&b.value().name));

        let item = TreeItem::new(schema_entity).children(children);
        items.push(item);
    }

    items
}
