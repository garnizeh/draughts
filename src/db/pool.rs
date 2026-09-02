//! The read pool — §11.1, §11.3.
//!
//! Read-only connections for status pages and export. They never write, and
//! they must stay responsive while the writer sustains 50k-row commits (§20.4).

use rusqlite::{Connection, OpenFlags};

use super::DbResult;
use crate::config::DatabaseConfig;
use crate::config::validate::{GB, MB};

/// Open one read-only connection with the reader pragmas applied.
///
/// `reader_cache_mb` is **per connection**, not per pool. Reading it the other
/// way is how a 6-connection pool quietly claims 24 GB (§16.1).
pub fn open_reader(config: &DatabaseConfig) -> DbResult<Connection> {
    let conn = Connection::open_with_flags(
        &config.path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    conn.execute_batch(&format!(
        "PRAGMA busy_timeout = {busy};
         PRAGMA cache_size = -{cache_kib};
         PRAGMA mmap_size = {mmap};
         PRAGMA query_only = ON;
         PRAGMA foreign_keys = ON;",
        busy = config.busy_timeout_ms,
        cache_kib = config.reader_cache_mb * 1024,
        mmap = config.mmap_size_gb.saturating_mul(GB),
    ))?;

    Ok(conn)
}

/// Open the single write connection, with the writer pragmas applied.
///
/// Called exactly once, at startup, and the result is moved onto the writer
/// thread (§22.3 step 2).
pub fn open_writer(config: &DatabaseConfig) -> DbResult<Connection> {
    if let Some(parent) = config.path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|error| {
            super::DbError::Degraded(format!("could not create {}: {error}", parent.display()))
        })?;
    }

    let conn = Connection::open(&config.path)?;

    // `page_size` only takes effect on an empty database, which is why it is
    // set before anything else and why §23 documents it as creation-only.
    conn.execute_batch(&format!("PRAGMA page_size = {};", config.page_size))?;

    conn.execute_batch(&format!(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = {busy};
         PRAGMA cache_size = -{cache_kib};
         PRAGMA mmap_size = {mmap};
         PRAGMA journal_size_limit = {journal};
         PRAGMA temp_store = MEMORY;",
        busy = config.busy_timeout_ms,
        cache_kib = config.writer_cache_mb * 1024,
        mmap = config.mmap_size_gb.saturating_mul(GB),
        journal = config.journal_size_limit_mb.saturating_mul(MB),
    ))?;

    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_write_connection_is_in_wal_mode() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = DatabaseConfig {
            path: dir.path().join("nested").join("draughts.db"),
            ..DatabaseConfig::default()
        };

        let conn = open_writer(&config).expect("writer opens");

        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");

        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);
    }

    /// A reader that can write is a second writer, and there is only one.
    #[test]
    fn readers_cannot_write() {
        let dir = tempfile::tempdir().expect("temp dir");
        let config = DatabaseConfig {
            path: dir.path().join("draughts.db"),
            ..DatabaseConfig::default()
        };

        {
            let mut conn = open_writer(&config).expect("writer opens");
            crate::db::migrations::run(&mut conn).expect("migrations apply");
        }

        let reader = open_reader(&config).expect("reader opens");
        let attempted = reader.execute(
            "INSERT INTO lab_batches (name, config_json) VALUES ('x', '{}')",
            [],
        );

        assert!(attempted.is_err(), "a read pool connection wrote");
    }
}
