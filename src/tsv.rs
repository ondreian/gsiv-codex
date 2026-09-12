//! Loading a curated table from its TSV file.
//!
//! `docs/bootstrap-format.md` settles the committed format: one file per
//! table, one row per line, sorted, tabs banned inside values. The reason is
//! git rather than taste -- a machine writes these files and a human reads the
//! diff, and two contributors adding rows in different towns have to merge
//! without a conflict.
//!
//! The file name is the table. `060_failure_classes.tsv` loads into
//! `failure_classes`; the numeric prefix orders the load, the way the `.sql`
//! files in the same directory are ordered, so a child table never arrives
//! before the parent its foreign key needs.
//!
//! There is no header row. The column order is the schema's column order, and
//! a header would be a second declaration of it that can disagree with the
//! first.

use rusqlite::Connection;
use std::path::Path;

/// Table name from a file name: `060_failure_classes.tsv` -> `failure_classes`.
fn table_of(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let name = stem.trim_start_matches(|c: char| c.is_ascii_digit());
    let name = name.strip_prefix('_').unwrap_or(name);
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return None;
    }
    Some(name.to_string())
}

/// Load one TSV into the table its name denotes. Returns the rows inserted.
///
/// Every row must have the same number of columns as the first. A short row is
/// a corrupted file, not a row with trailing empties: SQLite would accept it
/// by shifting every value one column left, which is the kind of error that
/// shows up months later as a rule that never matches anything.
pub fn load(conn: &Connection, path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    let Some(table) = table_of(path) else {
        return Err(format!("{}: not a table name", path.display()).into());
    };
    let text = std::fs::read_to_string(path)?;

    let mut width = 0usize;
    let mut stmt = None;
    let mut n = 0usize;
    for (i, line) in text.lines().enumerate() {
        // A blank line is the end of the file, not an empty row.
        if line.is_empty() {
            continue;
        }
        let cells: Vec<&str> = line.split('\t').collect();
        if stmt.is_none() {
            width = cells.len();
            let holes = vec!["?"; width].join(", ");
            stmt = Some(conn.prepare(&format!("INSERT INTO {table} VALUES ({holes})"))?);
        }
        if cells.len() != width {
            return Err(format!(
                "{}:{}: {} columns, expected {width}",
                path.display(),
                i + 1,
                cells.len()
            )
            .into());
        }
        if let Some(s) = stmt.as_mut() {
            s.execute(rusqlite::params_from_iter(cells))?;
        }
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::open_in_memory;

    fn write(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, body).expect("write");
        p
    }

    #[test]
    fn the_file_name_is_the_table() {
        assert_eq!(
            table_of(Path::new("data/060_failure_classes.tsv")).as_deref(),
            Some("failure_classes")
        );
        assert_eq!(
            table_of(Path::new("data/rooms.tsv")).as_deref(),
            Some("rooms")
        );
    }

    /// The prefix orders the load and nothing else, so it must not survive
    /// into the table name -- and a name that is not a bare identifier is
    /// refused rather than interpolated into SQL.
    #[test]
    fn a_name_that_is_not_an_identifier_is_refused() {
        assert_eq!(table_of(Path::new("data/x-y.tsv")), None);
        assert_eq!(table_of(Path::new("data/007.tsv")), None);
    }

    #[test]
    fn rows_load_in_schema_column_order() {
        let dir = std::env::temp_dir().join(format!("codex-tsv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let conn = open_in_memory().expect("open");

        let f = write(
            &dir,
            "060_failure_classes.tsv",
            "slipped\tretry\t736\t0\tthe ice\tglobal_defs.rb:736\n\
             way-is-closed\tretry\t697\t1\tthe door\tglobal_defs.rb:697\n",
        );
        assert_eq!(load(&conn, &f).expect("load"), 2);

        let attempts: i64 = conn
            .query_row(
                "SELECT attempts FROM failure_classes WHERE id = 'way-is-closed'",
                [],
                |r| r.get(0),
            )
            .expect("row");
        assert_eq!(attempts, 1, "columns landed in schema order");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The failure mode worth guarding: SQLite would take a short row and
    /// shift every value one column left, and nothing would complain until a
    /// rule silently stopped matching.
    #[test]
    fn a_row_with_the_wrong_width_is_an_error_not_a_shift() {
        let dir = std::env::temp_dir().join(format!("codex-tsv-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let conn = open_in_memory().expect("open");

        let f = write(
            &dir,
            "060_failure_classes.tsv",
            "slipped\tretry\t736\t0\tthe ice\tglobal_defs.rb:736\n\
             way-is-closed\tretry\t697\n",
        );
        let err = load(&conn, &f).expect_err("short row");
        assert!(err.to_string().contains("expected 6"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
