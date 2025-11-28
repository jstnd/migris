pub mod connectors;

#[derive(thiserror::Error, Debug)]
pub enum BirbError {
    #[error("failed to connect to database at {identifier}: {message}")]
    DatabaseConnectFailed { identifier: String, message: String },
}
