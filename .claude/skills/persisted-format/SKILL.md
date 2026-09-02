---
name: persisted-format
description: The format_version discipline for draughts — writing or reading a persisted BLOB, adding an INSERT, changing a column, writing a migration, regenerating the Zobrist key table, or changing any stored encoding. Use whenever touching src/db, migrations/, the schema, board_hash, or anything serialized into SQLite.
---

# `format_version`

**Rule 1 of the project: reading a persisted BLOB without dispatching on its
`format_version` is a review-blocking defect** (§13.7).

The column has a `DEFAULT` in the schema. That default exists for the v1.0
migration (§12.1) and for nothing else. A `DEFAULT` is what makes a version
column stop meaning anything — it lets a write path that has never heard of
versioning produce rows claiming to be version 1.

## Writing

Every insert into a table carrying `format_version` names the column
explicitly, from `draughts::CURRENT_FORMAT_VERSION`:

```rust
// §13.7: named explicitly. The column's DEFAULT is for the v1.0 migration.
INSERT INTO positions (..., format_version) VALUES (..., ?)
```

`just format-version-check` (`scripts/check-format-version.sh`) greps every
`INSERT INTO games` and `INSERT INTO positions` in `src/` and fails if the
statement does not name the column within twelve lines. It is a floor, not a
proof — the check passing does not mean you passed the right value.

## Reading

Dispatch. Never default, never skip, never panic:

```rust
match row.format_version {
    1 => decode_v1(blob)?,
    other => return Err(ApiError::UnsupportedFormatVersion { found: other }),
}
```

`ApiError::UnsupportedFormatVersion` already exists in `src/error.rs` with the
stable code `unsupported_format_version`. A stored row this build cannot decode
is an error the client sees, not a row silently dropped from a result set.

## When to bump `CURRENT_FORMAT_VERSION`

Bump it in `src/lib.rs` whenever a persisted encoding changes. Including:

- **Regenerating the Zobrist key table.** This is the one that catches people.
  New keys invalidate every stored `positions.board_hash` — the rows are still
  there and still parse, and they now describe positions that never occurred.
  A fingerprint test pins the table. When it fails, that is the check working:
  bump the version and write the migration. **Do not update the expected
  constant to make it stop.**
- Changing the packing of a move, a board, or a sampled MCTS statistic.
- Changing the meaning of a field inside an existing BLOB, even at the same size.

A bump means: the new value in `CURRENT_FORMAT_VERSION`, a decode arm for the
old version kept for as long as old rows can exist, and a note in `CHANGELOG.md`.

## Schema changes

- Migrations live in `migrations/`, applied at startup inside one transaction
  (§12). `0001_initial.sql` is the MVP schema.
- A new migration is a new numbered file. Never edit an applied one — the
  deployment it already ran against will not run it again.
- A new column that is persisted and decodable needs its row in the data
  dictionary (§13), not just in the SQL.
- SQLite pragmas and the WAL configuration are §11.1. They are derived from the
  host in §2.4; changing one without redoing that derivation makes the number
  arbitrary.

## The write path

Every write goes through the one actor in `src/db/writer.rs` (§11.2). SQLite
permits exactly one writer; the design does not fight that, it makes that writer
maximally efficient and absorbs bursts in RAM. A component that opens its own
write connection has broken the design, and `WriteQueueSaturated` is
backpressure working as intended (§11.2.5), not a failure to paper over.

Durability classes are §11.4 — not every write is equally urgent, and the class
is part of the message, not a property of the caller.
