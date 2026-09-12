//! Building a codex database from urnon's store.
//!
//! The long-term source is the Lich mapdb itself, vendored and imported here.
//! This reads urnon's already-imported `map.db3` instead, which is the same
//! data one step further along and gets a usable database today rather than
//! after a JSON importer is rewritten.
//!
//! Everything it writes is derived. Nothing here is curation — the vocabulary
//! and the tag map come in as parameters so that the *decisions* stay in data
//! files and only the mechanical projection lives in code.

use rusqlite::Connection;

/// What an import did, for the operator and for the tests.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub rooms: usize,
    pub edges: usize,
    pub edges_skipped_script: usize,
    pub edges_skipped_ambiguous: usize,
    pub facets: usize,
    pub locations: usize,
}

/// Read urnon's store and write the codex tables.
///
/// `source` is opened read-only by the caller. Both connections are live at
/// once rather than staging through memory: the largest table is 439,383 tag
/// rows and streaming it costs nothing.
pub fn from_urnon_store(codex: &Connection, source_path: &str) -> rusqlite::Result<Stats> {
    let mut stats = Stats::default();
    codex.execute("ATTACH DATABASE ?1 AS src", [source_path])?;
    let result = ingest(codex, &mut stats);
    // Detached even on failure: a half-built database that still holds a
    // handle to its source is a worse thing to debug than a half-built one.
    let _ = codex.execute("DETACH DATABASE src", []);
    result?;
    Ok(stats)
}

fn ingest(codex: &Connection, stats: &mut Stats) -> rusqlite::Result<()> {
    codex.execute_batch("BEGIN IMMEDIATE")?;

    stats.rooms = codex.execute(
        "INSERT OR REPLACE INTO rooms(uid, title, climate, terrain)
         SELECT uid, title, climate, terrain FROM src.rooms",
        [],
    )?;

    // A command leading to two rooms is not an edge. Recorded first so the
    // edge insert below can exclude it in one pass — and detected statically
    // rather than waiting for somebody to walk one twice, which is how urnon
    // finds them.
    stats.edges_skipped_ambiguous = codex.execute(
        "INSERT OR REPLACE INTO ambiguous_commands(from_uid, command, reason)
         SELECT from_uid, command, 'names more than one destination in the mapdb'
           FROM src.edges
          WHERE class <> 'script'
          GROUP BY from_uid, command
         HAVING count(DISTINCT to_uid) > 1",
        [],
    )?;

    stats.edges_skipped_script = codex.query_row(
        "SELECT count(*) FROM src.edges WHERE class = 'script'",
        [],
        |r| r.get::<_, i64>(0),
    )? as usize;

    // `INSERT OR IGNORE` rather than OR REPLACE: a room the source names but
    // never defines would violate the foreign key, and silently dropping such
    // an edge is better than refusing the whole import over one dangling
    // wayto. The count says how many landed.
    stats.edges = codex.execute(
        "INSERT OR IGNORE INTO edges(from_uid, to_uid, command, class, time_ms)
         SELECT e.from_uid, e.to_uid, e.command, e.class, e.time_ms
           FROM src.edges e
          WHERE e.class IN ('walk','transit','teleport')
            AND NOT EXISTS (SELECT 1 FROM ambiguous_commands a
                             WHERE a.from_uid = e.from_uid AND a.command = e.command)
            AND EXISTS (SELECT 1 FROM rooms r WHERE r.uid = e.from_uid)
            AND EXISTS (SELECT 1 FROM rooms r WHERE r.uid = e.to_uid)",
        [],
    )?;

    // What `LOCATION` reports, as a facet. A room fact; the town is the set
    // over the several things one town answers.
    stats.locations = codex.execute(
        "INSERT OR IGNORE INTO room_facets(room_uid, type, detail)
         SELECT lr.uid, 'location', lr.lich_location
           FROM src.lich_rooms lr
          WHERE lr.uid IS NOT NULL AND lr.lich_location IS NOT NULL
            AND lr.lich_location <> ''
            AND EXISTS (SELECT 1 FROM rooms r WHERE r.uid = lr.uid)",
        [],
    )?;

    codex.execute_batch("COMMIT")?;
    Ok(())
}

/// Project Lich tags into facets, using this database's own tag map.
///
/// Separate from [`from_urnon_store`] because it is the one step that consumes
/// curation: `tag_map` says which tag means which facet type, and that is a
/// decision somebody made rather than a fact the mapdb states. Run after the
/// vocabulary is loaded.
pub fn project_tags(codex: &Connection, source_path: &str) -> rusqlite::Result<usize> {
    codex.execute("ATTACH DATABASE ?1 AS src", [source_path])?;
    let written = codex.execute(
        "INSERT OR IGNORE INTO room_facets(room_uid, type, detail)
         SELECT r.uid, m.type,
                CASE m.mode
                  WHEN 'prefix' THEN ltrim(substr(t.tag, length(m.pattern) + 1), ':')
                  ELSE m.detail
                END
           FROM src.lich_room_tags t
           JOIN src.lich_rooms r ON r.lich_id = t.lich_id
           JOIN tag_map m
             ON (m.mode = 'exact'  AND m.pattern = t.tag)
             OR (m.mode = 'prefix' AND substr(t.tag, 1, length(m.pattern)) = m.pattern)
          WHERE r.uid IS NOT NULL
            AND EXISTS (SELECT 1 FROM rooms x WHERE x.uid = r.uid)",
        [],
    );
    let _ = codex.execute("DETACH DATABASE src", []);
    written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::open_in_memory;

    /// A miniature urnon store, written to a temp file because `ATTACH` needs
    /// a path and an in-memory database has none to give.
    fn fake_urnon_store() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("map.db3");
        let conn = Connection::open(&path).expect("open");
        conn.execute_batch(
            "
            CREATE TABLE rooms (uid INTEGER PRIMARY KEY, title TEXT, climate TEXT, terrain TEXT);
            CREATE TABLE edges (edge_id INTEGER PRIMARY KEY, from_uid INTEGER, to_uid INTEGER,
                                command TEXT, class TEXT, time_ms INTEGER);
            CREATE TABLE lich_rooms (lich_id INTEGER PRIMARY KEY, uid INTEGER, lich_location TEXT);
            CREATE TABLE lich_room_tags (lich_id INTEGER, tag TEXT);

            INSERT INTO rooms(uid, title) VALUES
                (1,'[Town Center]'), (2,'[East Road]'), (3,'[Bank]'), (4,'[Guild]');

            INSERT INTO edges(from_uid,to_uid,command,class,time_ms) VALUES
                (1,2,'east','walk',200),
                (2,1,'west','walk',200),
                (2,3,'go archway','walk',200),
                -- script: carries Ruby, must never be published
                (1,4,';e fput \"x\"' || char(10) || 'waitfor \"y\"','script',0),
                -- ambiguous: one command, two destinations
                (3,1,'out','walk',200),
                (3,4,'out','walk',200),
                -- dangling: room 99 does not exist
                (1,99,'north','walk',200);

            INSERT INTO lich_rooms(lich_id, uid, lich_location) VALUES
                (10,1,'Icemule Trace'), (11,2,'Icemule Trace'),
                (12,3,'Icemule Trace'), (13,4,'the town of Icemule Trace');

            INSERT INTO lich_room_tags(lich_id, tag) VALUES
                (12,'bank'), (10,'urchin guide tc'), (13,'urchin guide guild');
            ",
        )
        .expect("seed");
        (dir, path.to_string_lossy().into_owned())
    }

    fn codex_with_vocabulary() -> Connection {
        let conn = open_in_memory().expect("open");
        conn.execute_batch(
            "INSERT INTO facet_types(type, class) VALUES ('bank','poi'), ('urchin_guide','poi');
             INSERT INTO tag_map(pattern, type, mode, detail) VALUES
                ('bank','bank','exact',''),
                ('urchin guide ','urchin_guide','prefix','');",
        )
        .expect("vocabulary");
        conn
    }

    #[test]
    fn rooms_and_walk_edges_land() {
        let (_dir, src) = fake_urnon_store();
        let codex = codex_with_vocabulary();
        let stats = from_urnon_store(&codex, &src).expect("ingest");

        assert_eq!(stats.rooms, 4);
        assert_eq!(stats.edges, 3, "east, west, go archway");
    }

    /// The rule the whole format depends on: no Ruby crosses the boundary.
    #[test]
    fn script_edges_never_arrive() {
        let (_dir, src) = fake_urnon_store();
        let codex = codex_with_vocabulary();
        let stats = from_urnon_store(&codex, &src).expect("ingest");

        assert_eq!(stats.edges_skipped_script, 1);
        let with_newlines: i64 = codex
            .query_row(
                "SELECT count(*) FROM edges WHERE command LIKE '%'||char(10)||'%'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(with_newlines, 0, "a newline would break the published file");
    }

    /// `out` leads to two rooms, so it is a mechanism, not an edge. Caught
    /// statically rather than by walking it twice.
    #[test]
    fn a_command_naming_two_rooms_is_excluded_and_recorded() {
        let (_dir, src) = fake_urnon_store();
        let codex = codex_with_vocabulary();
        let stats = from_urnon_store(&codex, &src).expect("ingest");

        assert_eq!(stats.edges_skipped_ambiguous, 1);
        let left: i64 = codex
            .query_row(
                "SELECT count(*) FROM edges WHERE command = 'out'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(left, 0, "neither destination is publishable");

        let recorded: i64 = codex
            .query_row("SELECT count(*) FROM ambiguous_commands", [], |r| r.get(0))
            .expect("count");
        assert_eq!(recorded, 1, "and the reason is written down");
    }

    /// A wayto to a room the source never defines is dropped rather than
    /// failing the import. One dangling exit should not cost the other 62,015.
    #[test]
    fn an_edge_to_a_room_that_does_not_exist_is_dropped() {
        let (_dir, src) = fake_urnon_store();
        let codex = codex_with_vocabulary();
        from_urnon_store(&codex, &src).expect("ingest");

        let dangling: i64 = codex
            .query_row("SELECT count(*) FROM edges WHERE to_uid = 99", [], |r| {
                r.get(0)
            })
            .expect("count");
        assert_eq!(dangling, 0);
    }

    /// `LOCATION`'s answer is a room fact; the town is the set over both of
    /// the things one town answers.
    #[test]
    fn locations_arrive_as_facets() {
        let (_dir, src) = fake_urnon_store();
        let codex = codex_with_vocabulary();
        let stats = from_urnon_store(&codex, &src).expect("ingest");

        assert_eq!(stats.locations, 4);
        let spellings: Vec<String> = codex
            .prepare(
                "SELECT DISTINCT detail FROM room_facets WHERE type='location' ORDER BY detail",
            )
            .expect("prepare")
            .query_map([], |r| r.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows");
        assert_eq!(
            spellings,
            vec!["Icemule Trace", "the town of Icemule Trace"],
            "one town, two answers -- exactly the live case"
        );
    }

    /// Exact and prefix tag rules, the latter keeping the tag's remainder as
    /// the detail so `urchin guide tc` becomes `urchin_guide(tc)`.
    #[test]
    fn tags_project_through_the_curated_map() {
        let (_dir, src) = fake_urnon_store();
        let codex = codex_with_vocabulary();
        from_urnon_store(&codex, &src).expect("ingest");
        let written = project_tags(&codex, &src).expect("project");

        assert_eq!(written, 3);
        let guides: Vec<String> = codex
            .prepare("SELECT detail FROM room_facets WHERE type='urchin_guide' ORDER BY detail")
            .expect("prepare")
            .query_map([], |r| r.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows");
        assert_eq!(guides, vec!["guild", "tc"]);
    }

    /// A tag with no rule in the map is not a facet. The vocabulary is closed,
    /// and an unmapped tag is a curation gap rather than a new kind of thing.
    #[test]
    fn an_unmapped_tag_becomes_nothing() {
        let (_dir, src) = fake_urnon_store();
        let codex = codex_with_vocabulary();
        Connection::open(&src)
            .expect("open")
            .execute(
                "INSERT INTO lich_room_tags(lich_id, tag) VALUES (10, 'mystery')",
                [],
            )
            .expect("tag");

        from_urnon_store(&codex, &src).expect("ingest");
        project_tags(&codex, &src).expect("project");

        let mystery: i64 = codex
            .query_row(
                "SELECT count(*) FROM room_facets WHERE type='mystery'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(mystery, 0);
    }

    /// Importing twice is importing once. Every build re-runs this.
    #[test]
    fn ingesting_twice_changes_nothing() {
        let (_dir, src) = fake_urnon_store();
        let codex = codex_with_vocabulary();
        from_urnon_store(&codex, &src).expect("first");
        project_tags(&codex, &src).expect("first project");
        let count = |t: &str| -> i64 {
            codex
                .query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get(0))
                .expect("count")
        };
        let (rooms, edges, facets) = (count("rooms"), count("edges"), count("room_facets"));

        from_urnon_store(&codex, &src).expect("second");
        project_tags(&codex, &src).expect("second project");
        assert_eq!(
            (count("rooms"), count("edges"), count("room_facets")),
            (rooms, edges, facets)
        );
    }
}
