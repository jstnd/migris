use std::sync::Arc;

use migris::{Driver, Entity};

pub struct ConnectionLoadData {
    pub driver: Arc<dyn Driver>,
    pub entities: Vec<Entity>,
}
