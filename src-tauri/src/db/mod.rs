pub mod conversations;
pub mod schema;
pub mod search;

use rusqlite::Connection;

pub fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    schema::migrate(&conn)?;
    Ok(conn)
}
