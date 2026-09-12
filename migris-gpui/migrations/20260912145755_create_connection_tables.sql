CREATE TABLE IF NOT EXISTS connection_folders (
    id BLOB PRIMARY KEY,
    folder_id BLOB,
    name TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (folder_id) REFERENCES connection_folders(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS connections (
    id BLOB PRIMARY KEY,
    folder_id BLOB,
    name TEXT NOT NULL DEFAULT '',
    kind TEXT NOT NULL DEFAULT '',
    host TEXT NOT NULL DEFAULT '',
    port INTEGER NOT NULL DEFAULT 0,
    username TEXT NOT NULL DEFAULT '',
    password TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_connected TEXT,
    FOREIGN KEY (folder_id) REFERENCES connection_folders(id) ON DELETE CASCADE
);