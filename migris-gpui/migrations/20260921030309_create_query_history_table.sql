CREATE TABLE IF NOT EXISTS query_history (
	id BLOB PRIMARY KEY,
	execute_id BLOB NOT NULL,
	connection_id BLOB,
	query TEXT NOT NULL DEFAULT '',
	executed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
	duration_ms INTEGER NOT NULL DEFAULT 0,
	status TEXT NOT NULL DEFAULT '',
	error TEXT NOT NULL DEFAULT '',
	rows_affected INTEGER,
	rows_returned INTEGER,
	FOREIGN KEY (connection_id) REFERENCES connections(id) ON DELETE SET NULL
);