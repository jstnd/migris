use serde::{Deserialize, Serialize};

use crate::shared;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConnectionOptions {
    MySql(MySqlOptions),
}

impl Default for ConnectionOptions {
    fn default() -> Self {
        Self::MySql(MySqlOptions::default())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MySqlOptions {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

impl MySqlOptions {
    /// Returns the generated connection url.
    pub fn url(&self) -> String {
        format!(
            "mysql://{}:{}@{}:{}",
            self.user, self.password, self.host, self.port
        )
    }
}

impl Default for MySqlOptions {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: shared::DEFAULT_MYSQL_PORT,
            user: String::new(),
            password: String::new(),
        }
    }
}
