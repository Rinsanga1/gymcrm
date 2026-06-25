use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Resolve the database path: next to the running `.exe` (portable). Falls back
/// to the current working directory when running via `cargo run`.
pub fn db_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // In `cargo run`, the exe lives under target/debug — that's still a
            // real, writable folder, so portability holds in dev too.
            return dir.join("roche.db");
        }
    }
    PathBuf::from("roche.db")
}

/// Open (creating if needed) the SQLite database at `path` and run migrations.
pub fn open_db(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Open an in-memory database (used by tests).
pub fn open_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS members (
            id          INTEGER PRIMARY KEY,
            name        TEXT NOT NULL,
            phone       TEXT,
            join_date   TEXT NOT NULL,
            active      INTEGER NOT NULL DEFAULT 1,
            notes       TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_members_name  ON members(name);
        CREATE INDEX IF NOT EXISTS idx_members_phone ON members(phone);

        CREATE TABLE IF NOT EXISTS payments (
            id           INTEGER PRIMARY KEY,
            member_id    INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,
            period_month TEXT NOT NULL,
            amount       REAL NOT NULL,
            date         TEXT NOT NULL,
            note         TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_payments_period ON payments(period_month);
        CREATE INDEX IF NOT EXISTS idx_payments_member ON payments(member_id);

        CREATE TABLE IF NOT EXISTS products (
            id     INTEGER PRIMARY KEY,
            name   TEXT NOT NULL,
            price  REAL NOT NULL,
            stock  INTEGER NOT NULL DEFAULT 0,
            active INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS sales (
            id    INTEGER PRIMARY KEY,
            date  TEXT NOT NULL,
            total REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sales_date ON sales(date);

        CREATE TABLE IF NOT EXISTS sale_items (
            id         INTEGER PRIMARY KEY,
            sale_id    INTEGER NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
            product_id INTEGER REFERENCES products(id),
            qty        INTEGER NOT NULL,
            unit_price REAL NOT NULL
        );

        CREATE TABLE IF NOT EXISTS expenses (
            id     INTEGER PRIMARY KEY,
            amount REAL NOT NULL,
            date   TEXT NOT NULL,
            note   TEXT
        );

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;

    // Seed default settings if absent.
    conn.execute(
        "INSERT OR IGNORE INTO settings(key, value) VALUES ('default_monthly_fee', '1500')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings(key, value) VALUES ('currency', 'Rs')",
        [],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO settings(key, value) VALUES ('gym_name', 'My Gym')",
        [],
    )?;
    Ok(())
}
