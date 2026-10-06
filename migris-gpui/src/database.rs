use anyhow::Result;
use migris::sqlite::SqliteConnection;
use sqlx::{
    QueryBuilder,
    types::chrono::{DateTime, Utc},
};

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
        sqlx::migrate!("./migrations").run(connection.pool()).await?;
        Ok(Self { connection })
    }

    pub async fn connections(&self) -> Result<Vec<Connection>> {
        let sql = r#"
            SELECT
                id, folder_id,
                name, kind, host, port,
                username, password,
                created_at, last_connected,
                color
            FROM connections
        "#;

        Ok(sqlx::query_as::<_, Connection>(sql)
            .fetch_all(self.connection.pool())
            .await?)
    }

    pub async fn connection_folders(&self) -> Result<Vec<ConnectionFolder>> {
        let sql = r#"
            SELECT
                id, folder_id, name
            FROM connection_folders
        "#;

        Ok(sqlx::query_as::<_, ConnectionFolder>(sql)
            .fetch_all(self.connection.pool())
            .await?)
    }

    pub async fn delete_connection(&self, id: &ConnectionId) -> Result<()> {
        let sql = r#"
            DELETE
            FROM connections
            WHERE
                id = ?
        "#;

        sqlx::query(sql).bind(id).execute(self.connection.pool()).await?;
        Ok(())
    }

    pub async fn delete_connection_folder(&self, id: &ConnectionFolderId) -> Result<()> {
        let sql = r#"
            DELETE
            FROM connection_folders
            WHERE
                id = ?
        "#;

        sqlx::query(sql).bind(id).execute(self.connection.pool()).await?;
        Ok(())
    }

    pub async fn insert_connection(&self, connection: &Connection) -> Result<()> {
        let sql = r#"
            INSERT INTO connections
                (id, folder_id, name, kind, host, port, username, password, color)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(sql)
            .bind(connection.id)
            .bind(connection.folder_id)
            .bind(&connection.name)
            .bind(connection.kind)
            .bind(&connection.host)
            .bind(connection.port)
            .bind(&connection.username)
            .bind(&connection.password)
            .bind(&connection.color)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn insert_connection_folder(&self, folder: &ConnectionFolder) -> Result<()> {
        let sql = r#"
            INSERT INTO connection_folders
                (id, folder_id, name)
            VALUES
                (?, ?, ?)
        "#;

        sqlx::query(sql)
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
        let sql = r#"
            UPDATE connections
            SET
                folder_id = ?,
                name = ?,
                kind = ?,
                host = ?,
                port = ?,
                username = ?,
                password = ?,
                last_connected = ?,
                color = ?
            WHERE
                id = ?
        "#;

        sqlx::query(sql)
            .bind(connection.folder_id)
            .bind(&connection.name)
            .bind(connection.kind)
            .bind(&connection.host)
            .bind(connection.port)
            .bind(&connection.username)
            .bind(&connection.password)
            .bind(connection.last_connected.map(|dt| dt.format(DATE_FORMAT).to_string()))
            .bind(&connection.color)
            .bind(connection.id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn update_connection_color(&self, id: &ConnectionId, color: &Option<String>) -> Result<()> {
        let sql = r#"
            UPDATE connections
            SET
                color = ?
            WHERE
                id = ?
        "#;

        sqlx::query(sql)
            .bind(color)
            .bind(id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn update_connection_folder(&self, folder: &ConnectionFolder) -> Result<()> {
        let sql = r#"
            UPDATE connection_folders
            SET
                folder_id = ?,
                name = ?
            WHERE
                id = ?
        "#;

        sqlx::query(sql)
            .bind(folder.folder_id)
            .bind(&folder.name)
            .bind(folder.id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn update_connection_last_connected(
        &self,
        id: &ConnectionId,
        last_connected: &DateTime<Utc>,
    ) -> Result<()> {
        let sql = r#"
            UPDATE connections
            SET
                last_connected = ?
            WHERE
                id = ?
        "#;

        sqlx::query(sql)
            .bind(last_connected.format(DATE_FORMAT).to_string())
            .bind(id)
            .execute(self.connection.pool())
            .await?;
        Ok(())
    }

    pub async fn query_history(&self) -> Result<Vec<QueryHistoryItem>> {
        let sql = r#"
            SELECT
                id, execute_id, connection_id,
                query, executed_at, duration_ms,
                status, error, rows_affected, rows_returned
            FROM query_history
            ORDER BY
                execute_id DESC,
                executed_at
        "#;

        Ok(sqlx::query_as::<_, QueryHistoryItem>(sql)
            .fetch_all(self.connection.pool())
            .await?)
    }
}
