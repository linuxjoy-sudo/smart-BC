pub fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS conversations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            transcript TEXT NOT NULL,
            audio_path TEXT
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS conversations_fts USING fts5(
            transcript,
            tokenize = 'trigram'
        );
        CREATE TABLE IF NOT EXISTS people (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            relation TEXT DEFAULT '',
            note TEXT DEFAULT '',
            conversation_id INTEGER NOT NULL REFERENCES conversations(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
        CREATE INDEX IF NOT EXISTS idx_people_name ON people(name);
        CREATE TABLE IF NOT EXISTS preferences (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            topic TEXT NOT NULL,
            value TEXT NOT NULL,
            conversation_id INTEGER NOT NULL REFERENCES conversations(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
        CREATE TABLE IF NOT EXISTS episodes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            summary TEXT NOT NULL,
            place TEXT DEFAULT '',
            conversation_id INTEGER NOT NULL REFERENCES conversations(id),
            created_at TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );
        "#,
    )
}
