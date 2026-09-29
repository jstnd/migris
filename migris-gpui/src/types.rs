use std::{sync::Arc, time::Instant};

use migris::drivers::Driver;

use crate::{connections::Connection, shared};

pub struct OpenConnection {
    /// The information for the connection.
    pub connection: Connection,

    /// The driver for the connection.
    pub driver: Arc<dyn Driver>,
}

pub struct QueryProgress {
    /// The time the queries were started at.
    started_at: Instant,

    /// The number of queries completed.
    complete: usize,

    /// The total number of queries.
    total: usize,

    /// The percentage value of the current progress.
    value: f32,
}

impl QueryProgress {
    /// Creates a new [`QueryProgress`].
    pub fn new(total: usize) -> Self {
        Self {
            started_at: Instant::now(),
            complete: 0,
            total,
            value: 0.0,
        }
    }

    /// Returns the label describing the current progress.
    pub fn label(&self) -> String {
        format!(
            "Running query #{} of {}... {}",
            self.complete + 1,
            self.total,
            shared::format_timer(self.started_at.elapsed())
        )
    }

    /// Updates the progress with the given complete number.
    pub fn update(&mut self, complete: usize) {
        self.complete = complete;
        self.value = (complete as f32 / self.total as f32) * 100.0;
    }

    /// Returns the current progress percentage.
    pub fn value(&self) -> f32 {
        self.value
    }
}
