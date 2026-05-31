//! SQLite access.
//!
//! The database is a DERIVED CACHE, never the source of truth. Two tables:
//!
//!   * `discovered_commands` — a cache of what `discovery::scan_root` found.
//!     It is CLEARED and REBUILT on every rescan. Deleting the db file and
//!     relaunching must reproduce an equivalent index from the repos alone.
//!   * `run_history` — an append-only log of executions. This is the one thing
//!     that is not re-derivable from the repos, but it is purely a record of
//!     past actions; it is never read back to decide what commands exist.
//!   * `settings` — small key/value bag (e.g. the chosen root directory).
//!
//! Schema is created idempotently on open via `migrate`.

use crate::discovery::{DiscoveredCommand, SourceType};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

/// Thread-safe handle to the single connection. Tauri commands run on a pool,
/// so we guard the connection with a mutex.
pub struct Db {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryRow {
    pub id: i64,
    pub name: String,
    pub source: String,
    pub resolved_command: String,
    pub cwd: String,
    pub exit_code: Option<i64>,
    pub duration_ms: i64,
    pub started_at: i64,
}

impl Db {
    /// Open (creating if needed) the database at `path` and run migrations.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&conn)?;
        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory database (used by tests).
    #[cfg(test)]
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?;
        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    // ---- settings -------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---- discovered-commands cache -------------------------------------

    /// Clear and rebuild the discovered-commands cache. This is the ONLY way
    /// the cache is written: a rescan replaces it wholesale so it can never
    /// drift from the repos.
    pub fn replace_discovered(&self, cmds: &[DiscoveredCommand]) -> rusqlite::Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM discovered_commands", [])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO discovered_commands
                   (id, repo, source, name, description, invocation, cwd)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for c in cmds {
                stmt.execute(params![
                    c.id,
                    c.repo,
                    c.source.as_str(),
                    c.name,
                    c.description,
                    c.invocation,
                    c.cwd,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Read the whole cache back as `DiscoveredCommand`s.
    pub fn list_discovered(&self) -> rusqlite::Result<Vec<DiscoveredCommand>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, repo, source, name, description, invocation, cwd
             FROM discovered_commands
             ORDER BY repo, source, name",
        )?;
        let rows = stmt.query_map([], |row| {
            let source_str: String = row.get(2)?;
            Ok(DiscoveredCommand {
                id: row.get(0)?,
                repo: row.get(1)?,
                source: SourceType::from_str(&source_str).unwrap_or(SourceType::Npm),
                name: row.get(3)?,
                description: row.get(4)?,
                invocation: row.get(5)?,
                cwd: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    // ---- run history ----------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn insert_history(
        &self,
        name: &str,
        source: &str,
        resolved_command: &str,
        cwd: &str,
        exit_code: Option<i64>,
        duration_ms: i64,
        started_at: i64,
    ) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO run_history
               (name, source, resolved_command, cwd, exit_code, duration_ms, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![name, source, resolved_command, cwd, exit_code, duration_ms, started_at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// History newest-first. `query`, when non-empty, filters on command/cwd/name.
    pub fn list_history(&self, query: &str, limit: i64) -> rusqlite::Result<Vec<HistoryRow>> {
        let conn = self.conn.lock().unwrap();
        let like = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, name, source, resolved_command, cwd, exit_code, duration_ms, started_at
             FROM run_history
             WHERE ?1 = '' OR resolved_command LIKE ?2 OR cwd LIKE ?2 OR name LIKE ?2
             ORDER BY started_at DESC, id DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![query, like, limit], |row| {
            Ok(HistoryRow {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                resolved_command: row.get(3)?,
                cwd: row.get(4)?,
                exit_code: row.get(5)?,
                duration_ms: row.get(6)?,
                started_at: row.get(7)?,
            })
        })?;
        rows.collect()
    }
}

/// Idempotent schema creation. Safe to run on every startup.
fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Derived cache of discovered commands. Rebuilt wholesale on rescan.
        CREATE TABLE IF NOT EXISTS discovered_commands (
            id          TEXT PRIMARY KEY,
            repo        TEXT NOT NULL,
            source      TEXT NOT NULL,
            name        TEXT NOT NULL,
            description TEXT,
            invocation  TEXT NOT NULL,
            cwd         TEXT NOT NULL
        );

        -- Append-only log of executions.
        CREATE TABLE IF NOT EXISTS run_history (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            name             TEXT NOT NULL,
            source           TEXT NOT NULL,
            resolved_command TEXT NOT NULL,
            cwd              TEXT NOT NULL,
            exit_code        INTEGER,
            duration_ms      INTEGER NOT NULL,
            started_at       INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_history_started_at
            ON run_history (started_at DESC);
        ",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::DiscoveredCommand;

    fn sample(name: &str) -> DiscoveredCommand {
        DiscoveredCommand::new(
            "/repo".into(),
            SourceType::Npm,
            name.into(),
            Some("desc".into()),
            format!("npm run {}", name),
            "/repo".into(),
        )
    }

    #[test]
    fn settings_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.get_setting("root").unwrap(), None);
        db.set_setting("root", "/home/me/repos").unwrap();
        assert_eq!(db.get_setting("root").unwrap().as_deref(), Some("/home/me/repos"));
        // Upsert overwrites.
        db.set_setting("root", "/other").unwrap();
        assert_eq!(db.get_setting("root").unwrap().as_deref(), Some("/other"));
    }

    #[test]
    fn discovered_cache_is_replaced_wholesale() {
        let db = Db::open_in_memory().unwrap();
        db.replace_discovered(&[sample("a"), sample("b")]).unwrap();
        assert_eq!(db.list_discovered().unwrap().len(), 2);

        // A rescan with fewer items must not leave stale rows behind.
        db.replace_discovered(&[sample("c")]).unwrap();
        let after = db.list_discovered().unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].name, "c");
    }

    #[test]
    fn history_inserts_and_filters_newest_first() {
        let db = Db::open_in_memory().unwrap();
        db.insert_history("build", "npm", "npm run build", "/repo", Some(0), 120, 1000)
            .unwrap();
        db.insert_history("deploy", "make", "make deploy", "/repo", Some(1), 50, 2000)
            .unwrap();

        let all = db.list_history("", 50).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "deploy"); // newest first

        let filtered = db.list_history("build", 50).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].resolved_command, "npm run build");
    }
}
