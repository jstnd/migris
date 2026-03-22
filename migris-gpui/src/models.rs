use std::sync::Arc;

use migris::driver::{Driver, Entity};

pub struct ConnectionLoadData {
    pub driver: Arc<dyn Driver>,
    pub entities: Vec<Entity>,
}