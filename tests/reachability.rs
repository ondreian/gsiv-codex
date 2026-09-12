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

/// How much of each region you can walk to, and how much of it can walk home.
///
/// `(reachable, total)` per place, both directions, in two queries rather than
/// two per place: the forward walk follows `from_uid -> to_uid` out of
/// Wehnimer's, and the backward one follows the same edges the other way, so a
/// single traversal answers "can this room reach Wehnimer's" for every room at
/// once.
///
/// # Coverage, not "is anything reachable"
///
/// The first version of this asked whether *any* room of a region was
/// reachable, and reported the shadow of the Sanctum as connected. Two of its
/// seventy-eight rooms are. The other seventy-six -- which is to say the
/// hunting ground -- are not, and a measure that calls that connected is worse
/// than no measure, because it answers "yes" to the question somebody is
/// actually asking with a fact about something else.
fn coverage(conn: &Connection) -> Vec<(String, usize, usize, usize)> {
    let sql = "
        WITH RECURSIVE
          out_of_wl(uid) AS (
            SELECT room_uid FROM room_facets
             WHERE type = 'location' AND detail = ?1
            UNION
            SELECT e.to_uid FROM edges e JOIN out_of_wl r ON e.from_uid = r.uid
          ),
          -- The same edges, walked backwards: every room that can get home.
          into_wl(uid) AS (
            SELECT room_uid FROM room_facets
             WHERE type = 'location' AND detail = ?1
            UNION
            SELECT e.from_uid FROM edges e JOIN into_wl c ON e.to_uid = c.uid
          )
        SELECT f.detail,
               count(*),
               sum(CASE WHEN o.uid IS NOT NULL THEN 1 ELSE 0 END),
               sum(CASE WHEN i.uid IS NOT NULL THEN 1 ELSE 0 END)
          FROM room_facets f
          LEFT JOIN out_of_wl o ON o.uid = f.room_uid
          LEFT JOIN into_wl  i ON i.uid = f.room_uid
         WHERE f.type = 'location'
         GROUP BY f.detail";
    let mut stmt = conn.prepare(sql).expect("prepare");
    stmt.query_map(["Wehnimer's Landing"], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)? as usize,
            r.get::<_, i64>(2)? as usize,
            r.get::<_, i64>(3)? as usize,
        ))
    })
    .expect("query")
    .map(|r| r.expect("row"))
    .collect()
}

fn pct(part: usize, whole: usize) -> usize {
    if whole == 0 { 0 } else { part * 100 / whole }
}

/// What fraction of each region the published data reaches, as a floor.
///
/// Floors rather than exact counts: a mapdb refresh moves a few rooms and
/// should not fail a suite, while a mechanism going missing moves a region by
/// tens of points and must. `(place, from Wehnimer's, back to Wehnimer's)`.
///
/// A zero is a region no published mechanism touches at all. The comment names
/// what has to be expressed to change that, because "0%" on its own invites
/// somebody to go looking for a bug in the router.
const REGIONS: &[(&str, usize, usize)] = &[
    ("Wehnimer's Landing", 100, 100),
    ("Moonsedge", 98, 98),
    ("Icemule Trace", 94, 94),
    ("the Hinterwilds", 90, 0), // in by `climb sliver`; the caravan is the way out
    ("River's Rest", 84, 0),
    ("Solhaven", 89, 98),
    // Two of seventy-eight rooms: the Fangs of the Serpent gateway, and one
    // more. The hunting ground is the other seventy-six, and the way in is a
    // small bone periapt -- `rub` it for a viridian portal, `go` the portal.
    ("the shadow of the Sanctum", 0, 0),
    // Nothing at all. Each is one `;e` mechanism away.
    ("the Rift", 0, 0),            // `fput 'go sphere'` + an ethereal-fog loop
    ("Zul Logoth", 0, 0),          // `buy ticket` -- the gnome cart
    ("Kharam-Dzu", 0, 0),          // `ask portmaster about travel 4` -- the ferry
    ("the Pinefar forests", 0, 0), // `inquire; order 2; order confirm`
];

#[test]
fn the_data_reaches_as_much_of_each_region_as_we_think() {
    let conn = gsiv_codex::schema::open(codex()).expect("open");
    let measured = coverage(&conn);
    let mut wrong = Vec::new();
    for (place, out_floor, back_floor) in REGIONS {
        let Some((_, total, out, back)) = measured.iter().find(|(d, ..)| d == place) else {
            wrong.push(format!("{place}: no rooms carry this location"));
            continue;
        };
        for (label, floor, got) in [
            ("there", *out_floor, pct(*out, *total)),
            ("back", *back_floor, pct(*back, *total)),
        ] {
            if got < floor {
                wrong.push(format!("{place} ({label}): {got}%, floor is {floor}%"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "a region got harder to reach:\n  {}",
        wrong.join("\n  ")
    );
}

/// The regions no published mechanism reaches, named rather than counted, so
/// expressing one is a visible single-line change.
#[test]
fn the_unreachable_regions_are_the_ones_we_know_about() {
    let conn = gsiv_codex::schema::open(codex()).expect("open");
    let mut sealed: Vec<&str> = REGIONS
        .iter()
        .filter(|(place, ..)| {
            coverage(&conn)
                .iter()
                .find(|(d, ..)| d == place)
                .is_some_and(|(_, total, out, _)| pct(*out, *total) < 5)
        })
        .map(|(p, ..)| *p)
        .collect();
    sealed.sort_unstable();
    assert_eq!(
        sealed,
        [
            "Kharam-Dzu",
            "Zul Logoth",
            "the Pinefar forests",
            "the Rift",
            "the shadow of the Sanctum",
        ]
    );
}

/// Rooms nothing can walk into. The primary number.
///
/// A room with no inbound edge is one no route can end at, and every one of
/// them is a mechanism the importer dropped and nobody has expressed yet. It
/// needs no denominator argument and no guess about the game, which is what
/// makes it the measure to drive: coverage is this plus cascade plus real
/// topology, and reading a cause off it has gone wrong repeatedly.
///
/// Measured, not hoped for. It came down by 455 in one change when the table
/// idiom was extracted, which was half of it.
#[test]
fn the_rooms_nothing_can_walk_into() {
    let conn = gsiv_codex::schema::open(codex()).expect("open");
    let stuck: i64 = conn
        .query_row(
            "SELECT count(*) FROM rooms r
              WHERE NOT EXISTS (SELECT 1 FROM edges e WHERE e.to_uid = r.uid)",
            [],
            |r| r.get(0),
        )
        .expect("count");
    // A ceiling, so the number can only come down without a deliberate edit.
    // 134 of these have no inbound edge in the mapdb either and may never be
    // fixable from this data.
    assert!(
        stuck <= 449,
        "{stuck} rooms have no way in, up from 449 -- an extraction regressed"
    );
    println!("{stuck} rooms with no inbound edge");
}

/// Not an assertion — the table, printed. Run it when writing the floors
/// above, so they come from a measurement rather than from memory.
///
///     cargo test --test reachability -- --nocapture measure
#[test]
fn measure() {
    let conn = gsiv_codex::schema::open(codex()).expect("open");
    let mut rows = coverage(&conn);
    rows.retain(|(_, total, ..)| *total >= 50);
    rows.sort_by_key(|(d, total, out, _)| {
        (
            usize::MAX - pct(*out, *total),
            usize::MAX - total,
            d.clone(),
        )
    });
    println!(
        "{:>28}  {:>6}  {:>7}  {:>7}",
        "region", "rooms", "there", "back"
    );
    for (place, total, out, back) in rows {
        println!(
            "{place:>28}  {total:>6}  {:>6}%  {:>6}%",
            pct(out, total),
            pct(back, total)
        );
    }
}
