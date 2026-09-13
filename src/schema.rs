//! Applying the schema.
//!
//! Migrations are `.sql` files under `schema/`, embedded at compile time and
//! applied in name order. `user_version` records how many have run.
//!
//! **Files, not a string array.** urnon keeps its migrations as Rust string
//! literals and that has cost it twice: an 800-line data seed living inside
//! `schema.rs`, and a migration appended to patch a schema already slated for
//! replacement. A `.sql` file is readable by every tool that reads SQL,
//! diffable as SQL, and does not tempt anyone to put data in a source file.
//!
//! No ORM. See the roadmap: the query mix is not known yet, and the property
//! assertions this repo mostly consists of are SQL rather than anything a
//! query builder improves.

use rusqlite::Connection;

/// Every migration, in order, embedded so a built binary needs no files beside
/// it. Adding one means adding a line here — deliberately manual, because the
/// order is the contract and a directory listing is not a promise.
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_world", include_str!("../schema/001_world.sql")),
    ("002_room_sets", include_str!("../schema/002_room_sets.sql")),
    (
        "003_connectors",
        include_str!("../schema/003_connectors.sql"),
    ),
    ("004_edges", include_str!("../schema/004_edges.sql")),
    ("005_tag_map", include_str!("../schema/005_tag_map.sql")),
    ("006_overlays", include_str!("../schema/006_overlays.sql")),
    (
        "007_conditions",
        include_str!("../schema/007_conditions.sql"),
    ),
    ("008_preludes", include_str!("../schema/008_preludes.sql")),
    (
        "009_traversal_failures",
        include_str!("../schema/009_traversal_failures.sql"),
    ),
    (
        "010_room_titles",
        include_str!("../schema/010_room_titles.sql"),
    ),
    (
        "011_connector_costs",
        include_str!("../schema/011_connector_costs.sql"),
    ),
    (
        "012_replenishers",
        include_str!("../schema/012_replenishers.sql"),
    ),
];

/// How many migrations this build knows.
pub fn latest_version() -> u32 {
    MIGRATIONS.len() as u32
}

/// Apply anything this database has not seen.
///
/// Each migration runs in its own transaction with its `user_version` bump, so
/// a failure leaves the database at the last version that fully applied rather
/// than halfway through one.
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    for (i, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as u32;
        if version > current {
            conn.execute_batch(&format!(
                "BEGIN IMMEDIATE;\n{sql}\nPRAGMA user_version = {version};\nCOMMIT;"
            ))
            .map_err(|e| {
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(1),
                    Some(format!("migration {name}: {e}")),
                )
            })?;
        }
    }
    Ok(())
}

/// Open a database and bring it up to date.
///
/// Foreign keys are enforced, which is not SQLite's default and is the entire
/// point of the schema: a facet type that does not exist must be a failure at
/// the moment somebody writes it, not a puzzle at read time.
pub fn open(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

/// Open an existing database without migrating it.
pub fn open_any(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

/// An in-memory database at the current schema, for tests.
pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    migrate(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_database_lands_on_the_latest_version() {
        let conn = open_in_memory().expect("open");
        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("user_version");
        assert_eq!(version, latest_version());
    }

    /// Applying twice must be a no-op, because every build re-opens the file.
    #[test]
    fn migrating_an_up_to_date_database_changes_nothing() {
        let conn = open_in_memory().expect("open");
        migrate(&conn).expect("second migrate");
        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("user_version");
        assert_eq!(version, latest_version());
    }
}
