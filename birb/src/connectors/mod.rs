mod mysql;
pub use mysql::MySqlConnector;

pub trait Connector {
    fn connect(&mut self)
    -> impl std::future::Future<Output = Result<(), crate::BirbError>> + Send;
}
