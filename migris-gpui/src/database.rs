use anyhow::{Result, anyhow};
use directories::BaseDirs;
use migris::sqlite::SqliteConnection;

use crate::{
    connections::{Connection, ConnectionFolder, ConnectionFolderId, ConnectionId},
    shared,
};

const DATABASE_FILE: &str = "migris.db";

pub struct Database {
    connection: SqliteConnection,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let path = BaseDirs::new()
            .ok_or(anyhow!("failed to create BaseDirs struct"))?
            .config_dir()
            .join(shared::APPLICATION_NAME)
            .join(DATABASE_FILE);

        let connection = SqliteConnection::new(&path.to_string_lossy()).await?;
        sqlx::migrate!("./migrations")
            .run(connection.pool())
            .await?;

        Ok(Self { connection })
    }

    pub async fn connections(&self) -> Result<Vec<Connection>> {
        let query = r#"
            SELECT
                id, folder_id,
                name, kind, host, port,
                username, password,
                created_at, last_connected
            FROM connections
        "#;

        Ok(sqlx::query_as::<_, Connection>(query)
            .fetch_all(self.connection.pool())
            .await?)
    }

    pub async fn connection_folders(&self) -> Result<Vec<ConnectionFolder>> {
        let query = r#"
            SELECT
                id, folder_id, name
            FROM connection_folders
        "#;

        Ok(sqlx::query_as::<_, ConnectionFolder>(query)
            .fetch_all(self.connection.pool())
            .await?)
    }

    pub async fn delete_connection(&self, connection_id: &ConnectionId) -> Result<()> {
        let query = r#"
            DELETE
            FROM connections
            WHERE
                id = ?
        "#;

        sqlx::query(query)
            .bind(connection_id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_connection_folder(&self, folder_id: &ConnectionFolderId) -> Result<()> {
        let query = r#"
            DELETE
            FROM connection_folders
            WHERE
                id = ?
        "#;

        sqlx::query(query)
            .bind(folder_id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn insert_connection(&self, connection: &Connection) -> Result<()> {
        let query = r#"
            INSERT INTO connections
                (id, folder_id, name, kind, host, port, username, password)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(connection.id)
            .bind(connection.folder_id)
            .bind(&connection.name)
            .bind(connection.kind)
            .bind(&connection.host)
            .bind(connection.port)
            .bind(&connection.username)
            .bind(&connection.password)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn insert_connection_folder(&self, folder: &ConnectionFolder) -> Result<()> {
        let query = r#"
            INSERT INTO connection_folders
                (id, folder_id, name)
            VALUES
                (?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(folder.id)
            .bind(folder.folder_id)
            .bind(&folder.name)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn update_connection(&self, connection: &Connection) -> Result<()> {
        let query = r#"
            UPDATE connections
            SET
                folder_id = ?,
                name = ?,
                kind = ?,
                host = ?,
                port = ?,
                username = ?,
                password = ?,
                last_connected = ?
            WHERE
                id = ?
        "#;

        sqlx::query(query)
            .bind(connection.folder_id)
            .bind(&connection.name)
            .bind(connection.kind)
            .bind(&connection.host)
            .bind(connection.port)
            .bind(&connection.username)
            .bind(&connection.password)
            .bind(connection.last_connected)
            .bind(connection.id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn update_connection_folder(&self, folder: &ConnectionFolder) -> Result<()> {
        let query = r#"
            UPDATE connection_folders
            SET
                folder_id = ?,
                name = ?
            WHERE
                id = ?
        "#;

        sqlx::query(query)
            .bind(folder.folder_id)
            .bind(&folder.name)
            .bind(folder.id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }
}
