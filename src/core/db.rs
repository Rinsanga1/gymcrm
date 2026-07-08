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
            id                    INTEGER PRIMARY KEY,
            name                  TEXT NOT NULL,
            phone                 TEXT,
            join_date             TEXT NOT NULL,
            active                INTEGER NOT NULL DEFAULT 1,
            notes                 TEXT,
            registration_fee_paid INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_members_name  ON members(name);
        CREATE INDEX IF NOT EXISTS idx_members_phone ON members(phone);

        CREATE TABLE IF NOT EXISTS payments (
            id           INTEGER PRIMARY KEY,
            member_id    INTEGER NOT NULL REFERENCES members(id) ON DELETE CASCADE,
            period_month TEXT NOT NULL,
            amount       REAL NOT NULL,
            date         TEXT NOT NULL,
            note         TEXT,
            category     TEXT NOT NULL DEFAULT 'membership'
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
            name   TEXT NOT NULL DEFAULT '',
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

    add_missing_columns(conn)?;

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
    conn.execute(
        "INSERT OR IGNORE INTO settings(key, value) VALUES ('registration_fee', '500')",
        [],
    )?;
    Ok(())
}

/// True if `table` already has a column named `column`.
fn has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get("name")?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Add columns introduced after the initial schema to databases created by an
/// earlier version. `CREATE TABLE IF NOT EXISTS` never alters existing tables,
/// so new columns must be patched in here.
fn add_missing_columns(conn: &Connection) -> rusqlite::Result<()> {
    if !has_column(conn, "members", "registration_fee_paid")? {
        conn.execute(
            "ALTER TABLE members ADD COLUMN registration_fee_paid INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    if !has_column(conn, "payments", "category")? {
        conn.execute(
            "ALTER TABLE payments ADD COLUMN category TEXT NOT NULL DEFAULT 'membership'",
            [],
        )?;
    }
    if !has_column(conn, "expenses", "name")? {
        conn.execute(
            "ALTER TABLE expenses ADD COLUMN name TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    // Records the moment a row was entered, so same-day transactions order by
    // real time. ALTER ADD COLUMN can't take a dynamic default, so add it
    // nullable and backfill existing rows from their (day-only) date.
    for table in ["payments", "sales", "expenses"] {
        if !has_column(conn, table, "created_at")? {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN created_at TEXT"),
                [],
            )?;
            conn.execute(
                &format!("UPDATE {table} SET created_at = date WHERE created_at IS NULL"),
                [],
            )?;
        }
    }
    Ok(())
}
