//! Can you get there from here.
//!
//! The other property tests ask whether the schema refuses bad rows. This one
//! asks whether the data is any good, which is a different question and the one
//! a person actually has: **Wehnimer's Landing and the Rift are in the same
//! game, and nothing in this database says so.**
//!
//! Connectivity, not routing. A recursive CTE over `edges` answers "is there
//! any path", which is a fact about the published data; how long it takes and
//! who can walk it belong to a client. That also keeps this honest about what
//! it measures — a regression here is a regression in the data, never in
//! somebody's cost model.
//!
//! # Why the failures are written down
//!
//! Seven of GemStone's towns are unreachable from Wehnimer's, and that is not a
//! surprise to be discovered again every time somebody runs the suite. Every
//! link into them is a `;e` script edge the importer drops -- the ferry to
//! River's Rest, the gnome cart to Zul Logoth, the caravan to Pinefar, the
//! sphere into the Rift. They are connectors, and the schema has had a table
//! for them since slice 3.
//!
//! So the expectation is the *current* state, and connecting one of them fails
//! this test. That is the point: the number only moves deliberately, and the
//! commit that moves it says which mechanism it expressed.

use std::sync::OnceLock;

use rusqlite::Connection;

/// One codex, built once, shared by every test here. Importing 41MB of JSON
/// per test would make a stability measure something nobody runs.
fn codex() -> &'static std::path::PathBuf {
    static BUILT: OnceLock<std::path::PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let out = std::env::temp_dir().join(format!("codex-reach-{}.db3", std::process::id()));
        let _ = std::fs::remove_file(&out);

        let conn = gsiv_codex::schema::open(&out).expect("open");
        for phase in ["vocabulary"] {
            load_dir(&conn, &root.join("data").join(phase));
        }
        let json = std::fs::read_to_string(root.join("vendor/map.json")).expect("vendor/map.json");
        gsiv_codex::mapdb::import(&conn, &json).expect("import");
        load_dir(&conn, &root.join("data").join("overlays"));
        gsiv_codex::overlays::apply(&conn).expect("overlays");
        out
    })
}

fn load_dir(conn: &Connection, dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    files.sort();
    for path in files {
        match path.extension().and_then(|e| e.to_str()) {
            Some("sql") => conn
                .execute_batch(&std::fs::read_to_string(&path).expect("read"))
                .unwrap_or_else(|e| panic!("{}: {e}", path.display())),
            Some("tsv") => {
                gsiv_codex::tsv::load(conn, &path)
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            }
            _ => {}
        }
    }
}

/// Is any room of `to` reachable from any room of `from`, following edges?
///
/// Breadth-first in SQL, seeded and tested from subqueries rather than an
/// interpolated list: Ta'Illistim is 1,311 rooms and `VALUES (a),(b),...`
/// over that many hits SQLite's compound-SELECT limit. Parameters also mean
/// no place name is ever spliced into SQL.
///
/// Bounded by the recursion itself: `UNION` discards a room already seen, so
/// the walk visits each room at most once and terminates on a cyclic graph
/// with no depth limit to tune.
fn connected(conn: &Connection, from: &str, to: &str) -> bool {
    for place in [from, to] {
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM room_facets WHERE type = 'location' AND detail = ?1",
                [place],
                |r| r.get(0),
            )
            .expect("count");
        assert!(n > 0, "no rooms are in {place:?}");
    }

    conn.query_row(
        "WITH RECURSIVE reached(uid) AS (
             SELECT room_uid FROM room_facets WHERE type = 'location' AND detail = ?1
             UNION
             SELECT e.to_uid FROM edges e JOIN reached r ON e.from_uid = r.uid
         )
         SELECT EXISTS (
             SELECT 1 FROM reached
              WHERE uid IN (SELECT room_uid FROM room_facets
                             WHERE type = 'location' AND detail = ?2))",
        [from, to],
        |r| r.get(0),
    )
    .expect("reachability")
}

/// Every town, and whether the data can get you there and home again.
///
/// `(place, there, back)`, measured rather than hoped for. Update it when a
/// mechanism lands, and name the mechanism in the commit.
///
/// The asymmetries are the interesting column. Four of these are one-way
/// roads, and in both directions -- you can reach River's Rest and not leave,
/// you can leave Zul Logoth and not arrive. A single "reachable" flag would
/// have hidden every one of them, because each looks fine from the side
/// somebody happened to test.
const TOWNS: &[(&str, bool, bool)] = &[
    // Whole, both ways. The walkable heart of the map.
    ("Icemule Trace", true, true),
    ("Solhaven", true, true),
    ("Moonsedge", true, true),
    ("Ta'Illistim", true, true),
    // One-way. You arrive and cannot leave.
    ("the Hinterwilds", true, false), // in by `climb sliver`, a 15,000s edge
    ("River's Rest", true, false),    // out is `ask portmaster about travel N`
    // One-way, the other way. You can leave and never arrive.
    ("Zul Logoth", false, true), // in is `buy ticket` -- the gnome cart
    ("Ta'Vaalor", false, true),
    // Sealed. Every link in or out is a `;e` script edge the importer drops.
    ("the Rift", false, false), // `fput 'go sphere'` + an ethereal-fog loop
    ("Kharam-Dzu", false, false), // `ask portmaster about travel 4` -- the ferry
    ("the Pinefar forests", false, false), // `inquire; order 2; order confirm`
];

#[test]
fn the_map_connects_the_towns_it_is_known_to_connect() {
    let conn = gsiv_codex::schema::open(codex()).expect("open");
    let mut wrong = Vec::new();
    for (place, there, back) in TOWNS {
        for (label, expected, got) in [
            (
                "there",
                *there,
                connected(&conn, "Wehnimer's Landing", place),
            ),
            ("back", *back, connected(&conn, place, "Wehnimer's Landing")),
        ] {
            if expected != got {
                wrong.push(format!("{place} ({label}): expected {expected}, got {got}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "reachability changed -- deliberately? name the mechanism in the commit:\n  {}",
        wrong.join("\n  ")
    );
}

/// The headline number, so CI prints the direction of travel rather than
/// burying it in a diff of the table above.
#[test]
fn seven_towns_are_not_yet_round_trips() {
    let broken = TOWNS.iter().filter(|(_, t, b)| !(*t && *b)).count();
    assert_eq!(broken, 7, "towns you cannot both reach and leave");
}

/// Which towns the data reaches in only one direction.
///
/// Named individually so that connecting one is a visible single-line change
/// rather than a count going down. Again: an artifact of dropped edges, not a
/// one-way road -- the fix is to express the mechanism, after which both
/// columns flip together.
#[test]
fn the_asymmetric_towns_are_the_ones_we_know_about() {
    let mut one_way: Vec<&str> = TOWNS
        .iter()
        .filter(|(_, t, b)| t != b)
        .map(|(p, _, _)| *p)
        .collect();
    one_way.sort_unstable();
    assert_eq!(
        one_way,
        ["River's Rest", "Ta'Vaalor", "Zul Logoth", "the Hinterwilds"]
    );
}

/// Not an assertion — a measurement, printed so the table above can be written
/// from evidence instead of from memory. `cargo test --test reachability --
/// --nocapture measure`
#[test]
fn measure() {
    let conn = gsiv_codex::schema::open(codex()).expect("open");
    let places = [
        "Wehnimer's Landing",
        "Icemule Trace",
        "Solhaven",
        "Moonsedge",
        "the Hinterwilds",
        "the Rift",
        "Zul Logoth",
        "River's Rest",
        "Kharam-Dzu",
        "Ta'Illistim",
        "Ta'Vaalor",
        "the Pinefar forests",
    ];
    for place in places {
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM room_facets WHERE type='location' AND detail=?1",
                [place],
                |r| r.get(0),
            )
            .expect("count");
        if n == 0 {
            println!("{place:>22}  no rooms");
            continue;
        }
        let out = connected(&conn, "Wehnimer's Landing", place);
        let back = connected(&conn, place, "Wehnimer's Landing");
        println!("{place:>22}  {n:>5} rooms   there={out:<5} back={back}");
    }
}
