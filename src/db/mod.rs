//! Persistence — §11, §12, §13.
//!
//! SQLite permits exactly one writer. Rather than fight that, this layer makes
//! that writer maximally efficient and absorbs bursts in RAM: one connection,
//! owned by one thread, reached only through a channel.
//!
//! The write connection is **not** behind a mutex, because it is not shared. It
//! lives on the writer thread's stack. That is the difference between "we
//! serialize access with a lock and hope contention is low" and "concurrent
//! access is unrepresentable".

pub mod ids;
pub mod migrations;
pub mod pool;
pub mod records;
pub mod writer;

pub use records::{
    BatchStatus, EdgeRecord, FaceEventRecord, GameMode, GameRecord, PositionRecord, SampleKind,
};
pub use writer::{WriteOp, WriterHandle, WriterStats};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration {version} failed: {source}")]
    Migration {
        version: u32,
        #[source]
        source: rusqlite::Error,
    },

    /// §13.7. Not a panic, not a default, not a silently skipped row.
    #[error("stored row uses format_version {found}, which this build cannot decode")]
    UnsupportedFormatVersion { found: u32 },

    /// The database's `schema_migrations` table names a version this binary
    /// has no migration for — an older binary pointed at a newer database,
    /// most likely a rollback deployment. Serving traffic would mean reading
    /// columns and BLOB layouts this build does not know about.
    #[error(
        "database schema is at version {found}, but this binary only knows migrations up to \
         {known}; refusing to serve a schema newer than this build"
    )]
    SchemaTooNew { found: u32, known: u32 },

    /// A BLOB whose encoding a `format_version` dispatch let through, but
    /// whose length or shape does not match that encoding.
    #[error("corrupt {0}")]
    CorruptEncoding(String),

    /// The disk is full or the database is unwritable. Durable writes return
    /// `503`, bulk writes drop with a counter, and the read pool keeps serving
    /// (§20.6).
    #[error("the writer is degraded: {0}")]
    Degraded(String),

    #[error("the writer has shut down")]
    WriterGone,
}

pub type DbResult<T> = Result<T, DbError>;
