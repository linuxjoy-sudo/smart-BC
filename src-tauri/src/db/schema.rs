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
        "#,
    )
}
