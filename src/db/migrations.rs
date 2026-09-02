//! Schema migrations — §12, §22.3 step 2.
//!
//! Migrations run at startup, on the writer connection, inside a single
//! transaction, before the writer actor accepts its first message. A failure
//! here is fatal: a half-migrated schema is not a state the rest of the system
//! has a story for.

use rusqlite::{Connection, TransactionBehavior};

use super::{DbError, DbResult};

/// One migration: a version, a name, and the SQL that applies it.
///
/// Embedded in the binary rather than read from disk, because "one deployable
/// unit of code" (§22.1) includes the schema that code expects.
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial",
    sql: include_str!("../../migrations/0001_initial.sql"),
}];

/// The schema version this build expects. Derived, so that adding a migration
/// cannot forget to update it.
#[must_use]
pub fn target_version() -> u32 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

/// Apply every migration this database has not seen.
///
/// Returns the version the database is at afterwards.
pub fn run(conn: &mut Connection) -> DbResult<u32> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             name       TEXT NOT NULL,
             applied_at TEXT NOT NULL DEFAULT (datetime('now'))
         );",
    )?;

    let current: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    // An older binary pointed at a newer database: applying no migration and
    // reporting `target_version()` would silently serve a schema this build
    // does not declare compatible. Refuse instead.
    if current > target_version() {
        return Err(DbError::SchemaTooNew {
            found: current,
            known: target_version(),
        });
    }

    for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        tx.execute_batch(migration.sql)
            .map_err(|source| DbError::Migration {
                version: migration.version,
                source,
            })?;

        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![migration.version, migration.name],
        )?;

        tx.commit()?;

        tracing::info!(
            version = migration.version,
            name = migration.name,
            "applied migration"
        );
    }

    // §12.1: enforcement must be clean before the process serves traffic.
    let violations: i64 =
        conn.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })?;
    if violations > 0 {
        return Err(DbError::Degraded(format!(
            "pragma_foreign_key_check reports {violations} violations; \
             refusing to serve traffic"
        )));
    }

    Ok(target_version())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated() -> Connection {
        let mut conn = Connection::open_in_memory().expect("in-memory database");
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        run(&mut conn).expect("migrations apply");
        conn
    }

    #[test]
    fn migrations_apply_to_an_empty_database() {
        let conn = migrated();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        for expected in [
            "face_events",
            "games",
            "lab_batches",
            "position_edges",
            "positions",
        ] {
            assert!(tables.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut conn = migrated();
        assert_eq!(run(&mut conn).unwrap(), target_version());
        assert_eq!(run(&mut conn).unwrap(), target_version());

        let applied: u32 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(applied, MIGRATIONS.len() as u32);
    }

    #[test]
    fn migration_versions_are_contiguous_and_ascending() {
        for (index, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(
                migration.version,
                index as u32 + 1,
                "migration versions must start at 1 and not skip"
            );
        }
    }

    /// A database newer than this binary must not be silently served: a
    /// rollback deployment must fail loudly rather than mis-read a schema it
    /// does not declare compatible.
    #[test]
    fn a_schema_newer_than_this_binary_is_refused() {
        let mut conn = migrated();
        conn.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![target_version() + 1, "from_the_future"],
        )
        .unwrap();

        let result = run(&mut conn);
        assert!(
            matches!(result, Err(DbError::SchemaTooNew { .. })),
            "expected SchemaTooNew, got {result:?}"
        );
    }

    /// §13.7 as a schema-level guarantee: every table carrying a BLOB whose
    /// meaning can change carries the version that governs it.
    #[test]
    fn every_blob_bearing_table_has_a_format_version() {
        let conn = migrated();

        for table in ["games", "positions"] {
            let columns: Vec<String> = conn
                .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap();

            assert!(
                columns.contains(&"format_version".to_string()),
                "{table} stores BLOBs without a format_version"
            );
        }
    }

    /// §12: `position_edges` is `WITHOUT ROWID` with a composite key. The table
    /// is the largest in the database by row count, and the rowid it does not
    /// have is 8 bytes × hundreds of millions.
    #[test]
    fn position_edges_is_without_rowid() {
        let conn = migrated();

        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name = 'position_edges'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(sql.to_uppercase().contains("WITHOUT ROWID"));
    }

    /// §13.3: a position sampled from a human match belongs to no batch.
    #[test]
    fn a_human_sample_needs_no_batch() {
        let conn = migrated();

        let batch_id_notnull: i64 = conn
            .query_row(
                "SELECT \"notnull\" FROM pragma_table_info('positions') WHERE name = 'batch_id'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(
            batch_id_notnull, 0,
            "NOT NULL here makes human_game_sample unrepresentable"
        );
    }
}
