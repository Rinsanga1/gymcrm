use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::Local;

pub const KEEP_LAST: usize = 7;

/// Backups live in `backups/` next to the running database.
pub fn backups_dir(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("backups")
}

/// Copy the DB to `backups/tenne_<YYYYMMDD-HHMMSS>.db`. Returns the new path.
/// SQLite WAL journal is flushed via the rusqlite Drop, but for an actively-
/// open DB we rely on WAL checkpointing — callers should call this either at
/// startup, on close, or after `PRAGMA wal_checkpoint(FULL)`.
pub fn backup_now(db_path: &Path) -> io::Result<PathBuf> {
    let dir = backups_dir(db_path);
    fs::create_dir_all(&dir)?;
    let ts = Local::now().format("%Y%m%d-%H%M%S");
    let dest = dir.join(format!("tenne_{}.db", ts));
    fs::copy(db_path, &dest)?;
    prune(&dir, KEEP_LAST)?;
    Ok(dest)
}

/// Back up only if the newest existing backup is older than `min_gap` (or none
/// exists yet). Used on exit so repeatedly opening and closing the app doesn't
/// copy a possibly-large database every time; the manual "Back up now" button in
/// Settings bypasses this and always writes one.
pub fn backup_now_if_stale(db_path: &Path, min_gap: Duration) -> io::Result<Option<PathBuf>> {
    if let Some(age) = newest_backup_age(db_path) {
        if age < min_gap {
            return Ok(None);
        }
    }
    backup_now(db_path).map(Some)
}

/// Time since the most recent backup file was written, or `None` if there are no
/// backups yet (or the clock/mtime can't be read).
fn newest_backup_age(db_path: &Path) -> Option<Duration> {
    let dir = backups_dir(db_path);
    let newest = fs::read_dir(&dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("tenne_"))
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .max()?;
    SystemTime::now().duration_since(newest).ok()
}

/// Keep only the most recent `keep` backups (by mtime); delete older ones.
fn prune(dir: &Path, keep: usize) -> io::Result<()> {
    let mut entries: Vec<(PathBuf, std::time::SystemTime)> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("tenne_")
                && e.path()
                    .extension()
                    .map(|x| x == "db")
                    .unwrap_or(false)
        })
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            let t = m.modified().ok()?;
            Some((e.path(), t))
        })
        .collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    for (path, _) in entries.into_iter().skip(keep) {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

/// List backups newest-first.
pub fn list_backups(db_path: &Path) -> io::Result<Vec<PathBuf>> {
    let dir = backups_dir(db_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<(PathBuf, std::time::SystemTime)> = fs::read_dir(&dir)?
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("tenne_")
                && e.path()
                    .extension()
                    .map(|x| x == "db")
                    .unwrap_or(false)
        })
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            let t = m.modified().ok()?;
            Some((e.path(), t))
        })
        .collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(entries.into_iter().map(|(p, _)| p).collect())
}

/// Replace the live DB with `source` (caller must close the DB connection first).
/// A safety copy of the current DB is written to `tenne.db.pre-restore`.
pub fn restore(db_path: &Path, source: &Path) -> io::Result<()> {
    if db_path.exists() {
        let safety = db_path.with_extension("db.pre-restore");
        fs::copy(db_path, &safety)?;
    }
    fs::copy(source, db_path)?;
    Ok(())
}
