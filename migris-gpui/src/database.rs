use anyhow::Result;
use migris::sqlite::SqliteConnection;
use sqlx::QueryBuilder;

use crate::{
    connections::{Connection, ConnectionFolder, ConnectionFolderId, ConnectionId},
    history::{QueryHistoryGroup, QueryHistoryItem},
    shared,
};

const DATABASE_FILE: &str = "migris.db";
const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub struct Database {
    connection: SqliteConnection,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let path = shared::config_dir()?.join(DATABASE_FILE);
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

    pub async fn delete_connection(&self, id: &ConnectionId) -> Result<()> {
        let query = r#"
            DELETE
            FROM connections
            WHERE
                id = ?
        "#;

        sqlx::query(query)
            .bind(id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_connection_folder(&self, id: &ConnectionFolderId) -> Result<()> {
        let query = r#"
            DELETE
            FROM connection_folders
            WHERE
                id = ?
        "#;

        sqlx::query(query)
            .bind(id)
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

    pub async fn insert_query_history_group(&self, group: &QueryHistoryGroup) -> Result<()> {
        let mut builder = QueryBuilder::new(
            "INSERT INTO query_history (id, execute_id, connection_id, query, executed_at, duration_ms, status, error, rows_affected, rows_returned) ",
        );

        // TODO: handle sqlite parameter limit
        builder.push_values(&group.items, |mut b, item| {
            b.push_bind(item.id)
                .push_bind(item.execute_id)
                .push_bind(item.connection_id)
                .push_bind(&item.query)
                .push_bind(item.executed_at.format(DATE_FORMAT).to_string())
                .push_bind(item.duration_ms as i64)
                .push_bind(item.status)
                .push_bind(&item.error)
                .push_bind(item.rows_affected.map(|r| r as i64))
                .push_bind(item.rows_returned.map(|r| r as i64));
        });

        builder.build().execute(self.connection.pool()).await?;
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

    pub async fn query_history(&self) -> Result<Vec<QueryHistoryItem>> {
        let query = r#"
            SELECT
                id, execute_id, connection_id,
                query, executed_at, duration_ms,
                status, error, rows_affected, rows_returned
            FROM query_history
            ORDER BY
                execute_id DESC,
                executed_at
        "#;

        Ok(sqlx::query_as::<_, QueryHistoryItem>(query)
            .fetch_all(self.connection.pool())
            .await?)
    }
}
