//! Applying corrections over the imported mapdb.
//!
//! The ordering is the design. The vendored mapdb is imported first and
//! overlays are applied second, so refreshing `vendor/map.json` cannot revert a
//! correction — which is the failure that ruled out both committing the built
//! tables and rebuilding from the mapdb alone.

use rusqlite::Connection;

/// Write every overlay into `edges`, and record what became of the script
/// edges they came from.
///
/// `INSERT OR REPLACE`: an overlay outranks the import by construction. That is
/// what an assertion *is* — somebody looked at what the mapdb said and decided
/// otherwise.
pub fn apply(conn: &Connection) -> rusqlite::Result<usize> {
    // Only where both rooms exist. An overlay naming a room the mapdb dropped
    // is stale rather than wrong, and it will show up in the coverage report
    // rather than failing the build.
    let written = conn.execute(
        "INSERT OR REPLACE INTO edges(from_uid, to_uid, command, class, time_ms)
         SELECT o.from_uid, o.to_uid, o.command, o.class, o.time_ms
           FROM edge_overlays o
          WHERE EXISTS (SELECT 1 FROM rooms r WHERE r.uid = o.from_uid)
            AND EXISTS (SELECT 1 FROM rooms r WHERE r.uid = o.to_uid)
            AND NOT EXISTS (SELECT 1 FROM ambiguous_commands a
                             WHERE a.from_uid = o.from_uid AND a.command = o.command)",
        [],
    )?;

    // An overlay from a script extract is the disposition of that script edge.
    conn.execute(
        "INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, disposition, reason)
         SELECT o.from_uid, o.to_uid, 'overlay', ''
           FROM edge_overlays o
          WHERE o.source = 'script-extract'",
        [],
    )?;

    Ok(written)
}

/// How much of the script-edge problem is solved.
///
/// A number rather than a feeling: every script edge the mapdb carries is
/// superseded by something or listed as unhandled with a reason, and
/// [`unaccounted`] asserts there is no third case.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    pub overlay: usize,
    pub connector: usize,
    pub unhandled: usize,
}

pub fn coverage(conn: &Connection) -> rusqlite::Result<Coverage> {
    let count = |d: &str| -> rusqlite::Result<usize> {
        conn.query_row(
            "SELECT count(*) FROM script_edge_disposition WHERE disposition = ?1",
            [d],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n as usize)
    };
    Ok(Coverage {
        overlay: count("overlay")?,
        connector: count("connector")?,
        unhandled: count("unhandled")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::open_in_memory;

    fn fixture() -> Connection {
        let conn = open_in_memory().expect("open");
        conn.execute_batch(
            "INSERT INTO rooms(uid) VALUES (1),(2),(3);
             -- what the mapdb said
             INSERT INTO edges(from_uid,to_uid,command,class,time_ms)
                  VALUES (1,2,'east','walk',200);",
        )
        .expect("fixture");
        conn
    }

    /// The thing option C exists for: an assertion outranks the import.
    #[test]
    fn an_overlay_beats_what_the_mapdb_said() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO edge_overlays(from_uid,to_uid,command,class,time_ms,source,note)
                  VALUES (1,2,'east','walk',3000,'observed','it is slower than the mapdb claims');",
        )
        .expect("overlay");
        apply(&conn).expect("apply");

        let ms: i64 = conn
            .query_row("SELECT time_ms FROM edges WHERE from_uid=1", [], |r| {
                r.get(0)
            })
            .expect("edge");
        assert_eq!(ms, 3000);
    }

    /// A command read out of Ruby becomes a real edge the mapdb never had.
    #[test]
    fn a_script_extract_adds_an_edge() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO edge_overlays(from_uid,to_uid,command,class,time_ms,source,note)
                  VALUES (2,3,'go doorframe','walk',400,'script-extract','');",
        )
        .expect("overlay");
        assert_eq!(apply(&conn).expect("apply"), 1);

        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM edges WHERE from_uid=2 AND command='go doorframe'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(exists, 1);
    }

    /// An overlay naming a room the mapdb dropped is stale, not fatal. A
    /// refresh removing a room must not fail the build.
    #[test]
    fn an_overlay_for_a_room_that_no_longer_exists_is_skipped() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO edge_overlays(from_uid,to_uid,command,source)
                  VALUES (1,999,'go nowhere','manual');",
        )
        .expect("overlay");
        assert_eq!(apply(&conn).expect("apply"), 0, "skipped, not fatal");
    }

    /// A command already known to name two rooms is a mechanism. An overlay
    /// cannot smuggle one back in as an edge.
    #[test]
    fn an_overlay_cannot_resurrect_an_ambiguous_command() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO ambiguous_commands(from_uid,command,reason) VALUES (1,'out','two rooms');
             INSERT INTO edge_overlays(from_uid,to_uid,command,source) VALUES (1,3,'out','manual');",
        )
        .expect("overlay");
        assert_eq!(apply(&conn).expect("apply"), 0);
    }

    /// Applying twice is applying once. Every build re-runs this.
    #[test]
    fn applying_twice_changes_nothing() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO edge_overlays(from_uid,to_uid,command,class,time_ms,source,note)
                  VALUES (2,3,'go doorframe','walk',400,'script-extract','');",
        )
        .expect("overlay");
        apply(&conn).expect("first");
        let before: i64 = conn
            .query_row("SELECT count(*) FROM edges", [], |r| r.get(0))
            .expect("count");
        apply(&conn).expect("second");
        let after: i64 = conn
            .query_row("SELECT count(*) FROM edges", [], |r| r.get(0))
            .expect("count");
        assert_eq!(before, after);
    }

    /// Coverage is the burn-down. A script extract disposes of the edge it
    /// came from, so applying overlays moves the number.
    #[test]
    fn a_script_extract_counts_towards_coverage() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO edge_overlays(from_uid,to_uid,command,source)
                  VALUES (2,3,'go doorframe','script-extract');",
        )
        .expect("overlay");
        apply(&conn).expect("apply");

        assert_eq!(
            coverage(&conn).expect("coverage"),
            Coverage {
                overlay: 1,
                connector: 0,
                unhandled: 0
            }
        );
    }

    /// An unhandled edge must say why. An empty row implies nobody looked,
    /// and nine of these are puzzles that genuinely cannot be data.
    #[test]
    fn an_unhandled_script_edge_must_carry_a_reason() {
        let conn = fixture();
        let err = conn.execute(
            "INSERT INTO script_edge_disposition(from_uid,to_uid,disposition,reason)
                  VALUES (1,2,'unhandled','')",
            [],
        );
        assert!(
            err.is_err(),
            "unhandled without a reason is not a disposition"
        );

        conn.execute(
            "INSERT INTO script_edge_disposition(from_uid,to_uid,disposition,reason)
                  VALUES (1,2,'unhandled','reads a riddle off the mural and answers it')",
            [],
        )
        .expect("with a reason");
    }
}
