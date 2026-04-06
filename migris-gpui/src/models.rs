use std::sync::Arc;

use migris::{Driver, Entity};

pub struct ConnectionLoadData {
    pub driver: Arc<dyn Driver>,
    pub entities: Vec<Entity>,
}

pub struct QueryProgress {
    complete: usize,
    total: usize,
    value: f32,
}

impl QueryProgress {
    pub fn new(total: usize) -> Self {
        Self {
            complete: 0,
            total,
            value: 0.0,
        }
    }

    pub fn label(&self) -> String {
        format!("Running query #{} of {}", self.complete + 1, self.total)
    }

    pub fn update(&mut self, complete: usize) {
        self.complete = complete;
        self.value = (complete as f32 / self.total as f32) * 100.0;
    }

    pub fn value(&self) -> f32 {
        self.value
    }
}
