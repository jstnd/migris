use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ConnectOptions {
    MySql(MySqlConnectOptions),
}

#[derive(Deserialize, Serialize)]
pub struct MySqlConnectOptions {
    host: String,
    port: u16,
    user: String,
    password: String,
}
