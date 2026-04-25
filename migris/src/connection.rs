use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConnectionOptions {
    MySql(MySqlOptions),
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
