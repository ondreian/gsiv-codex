//! Importing the vendored Lich mapdb.
//!
//! The mapdb is the bootstrap, and it is somebody else's work: a JSON array of
//! rooms keyed by Lich's own ids, carrying the community's accumulated
//! knowledge of the world. It is vendored rather than fetched so that a clone
//! builds offline and an old commit still builds at all.
//!
//! # Projecting into uid space
//!
//! Lich rooms are not canonical rooms. A Lich room carries a **list** of
//! GemStone uids, and the list can be empty or long:
//!
//! | uids | rooms | kept |
//! | --- | --- | --- |
//! | 0 | 7,884 | no — Lich knows a place the game does not name |
//! | 1 | 28,446 | yes |
//! | 2+ | 281 | yes, as the **first** uid |
//!
//! The 2+ case is instancing: one Lich entry covering every copy of a shared
//! room, up to fifty of them.
//!
//! Taking the first uid is **incomplete but not false**. The edge published is
//! a real edge of a real room; the other copies simply go unrepresented. The
//! first draft of this dropped them instead, on the argument that an edge out
//! of a fifty-copy room does not say which copy it leaves — which is true, and
//! cost 850 edges and **296 rooms of reachability from Icemule alone, 12.5%**.
//! Measured, that trade is the wrong way round: a conservative projection that
//! makes a tenth of the walkable world unreachable is worse than one that
//! under-represents a shared room.
//!
//! An edge survives only if both of its endpoints did.

use rusqlite::Connection;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// One room as the mapdb writes it. Every field is optional because the mapdb
/// is community data and older entries predate newer fields.
#[derive(Debug, Deserialize)]
struct MapRoom {
    id: i64,
    #[serde(default)]
    uid: Vec<i64>,
    #[serde(default)]
    title: Vec<String>,
    /// A `Value`, not a `String`, because the mapdb has `"location": false`
    /// in it. Community data, decades of it, and a type survey of every field
    /// says this is the only one that lies about its type — `timeto` values
    /// being int-or-float-or-Ruby is the other, and that is handled below.
    /// Read with `as_str`, so a `false` becomes no location rather than a
    /// failed parse of all 36,611 rooms.
    #[serde(default)]
    location: serde_json::Value,
    #[serde(default)]
    climate: Option<String>,
    #[serde(default)]
    terrain: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    /// Lich id (as a string key) to the command that goes there.
    #[serde(default)]
    wayto: HashMap<String, String>,
    /// Lich id to seconds. Sometimes Ruby instead of a number, when the cost
    /// is conditional; `serde_json::Value` rather than `f64` so one of those
    /// does not fail the whole parse.
    #[serde(default)]
    timeto: HashMap<String, serde_json::Value>,
}

/// What an import did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub rooms: usize,
    pub rooms_unmapped: usize,
    /// Rooms whose Lich entry carried several uids, counted because the
    /// projection keeps only the first and somebody should know.
    pub rooms_instanced: usize,
    pub edges: usize,
    pub edges_script: usize,
    pub edges_dangling: usize,
    pub edges_ambiguous: usize,
    pub facets: usize,
}

/// Default traversal cost when the mapdb gives Ruby instead of a number.
///
/// Matches the mapdb's own overwhelming majority, which is 0.2s for a step.
const DEFAULT_TIME_MS: i64 = 200;

/// Read a vendored mapdb and write the tables it can support.
///
/// The vocabulary must already be loaded: tags become facets only where
/// `tag_map` says what they mean.
pub fn import(conn: &Connection, json: &str) -> Result<Stats, Box<dyn std::error::Error>> {
    let rooms: Vec<MapRoom> = serde_json::from_str(json)?;
    let mut stats = Stats::default();

    // Lich id -> the one canonical uid it stands for.
    let mut uid_of: HashMap<i64, i64> = HashMap::new();
    for room in &rooms {
        match room.uid.first() {
            None => stats.rooms_unmapped += 1,
            Some(&uid) => {
                if room.uid.len() > 1 {
                    stats.rooms_instanced += 1;
                }
                uid_of.insert(room.id, uid);
            }
        }
    }

    conn.execute_batch("BEGIN IMMEDIATE")?;

    {
        let mut room_stmt = conn.prepare(
            "INSERT OR REPLACE INTO rooms(uid, title, climate, terrain) VALUES (?1, ?2, ?3, ?4)",
        )?;
        let mut facet_stmt = conn.prepare(
            "INSERT OR IGNORE INTO room_facets(room_uid, type, detail) VALUES (?1, ?2, ?3)",
        )?;
        for room in &rooms {
            let Some(&uid) = uid_of.get(&room.id) else {
                continue;
            };
            room_stmt.execute(rusqlite::params![
                uid,
                room.title.first(),
                room.climate,
                room.terrain
            ])?;
            stats.rooms += 1;

            // What `LOCATION` reports here. A room fact; the town is the set
            // over the several answers one town gives.
            if let Some(loc) = room.location.as_str().filter(|l| !l.is_empty()) {
                facet_stmt.execute(rusqlite::params![uid, "location", loc])?;
                stats.facets += 1;
            }
        }
    }

    // Tags become facets through the curated map, so an unmapped tag becomes
    // nothing rather than a twenty-first kind of thing nobody meant to invent.
    let rules = tag_rules(conn)?;
    {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO room_facets(room_uid, type, detail) VALUES (?1, ?2, ?3)",
        )?;
        for room in &rooms {
            let Some(&uid) = uid_of.get(&room.id) else {
                continue;
            };
            for tag in &room.tags {
                if let Some((ty, detail)) = apply_rules(&rules, tag) {
                    stats.facets += stmt.execute(rusqlite::params![uid, ty, detail])?;
                }
            }
        }
    }

    // Edges, in two passes: the first finds commands that name more than one
    // destination, because such a command is a mechanism rather than an exit
    // and neither of its destinations is publishable.
    let mut seen: HashMap<(i64, String), HashSet<i64>> = HashMap::new();
    for room in &rooms {
        let Some(&from) = uid_of.get(&room.id) else {
            continue;
        };
        for (target, command) in &room.wayto {
            if is_script(command) {
                continue;
            }
            let Ok(target_id) = target.parse::<i64>() else {
                continue;
            };
            let Some(&to) = uid_of.get(&target_id) else {
                continue;
            };
            seen.entry((from, command.clone())).or_default().insert(to);
        }
    }
    let ambiguous: HashSet<(i64, String)> = seen
        .iter()
        .filter(|(_, tos)| tos.len() > 1)
        .map(|(k, _)| k.clone())
        .collect();

    {
        let mut amb_stmt = conn.prepare(
            "INSERT OR REPLACE INTO ambiguous_commands(from_uid, command, reason) VALUES (?1, ?2, ?3)",
        )?;
        for (from, command) in &ambiguous {
            amb_stmt.execute(rusqlite::params![
                from,
                command,
                "names more than one destination in the mapdb"
            ])?;
            stats.edges_ambiguous += 1;
        }

        let mut edge_stmt = conn.prepare(
            "INSERT OR IGNORE INTO edges(from_uid, to_uid, command, class, time_ms)
             VALUES (?1, ?2, ?3, 'walk', ?4)",
        )?;
        let mut unpublished = conn.prepare(
            "INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, \
             disposition, reason) VALUES (?1, ?2, ?3, 'unhandled', ?4)",
        )?;
        for room in &rooms {
            let Some(&from) = uid_of.get(&room.id) else {
                continue;
            };
            for (target, command) in &room.wayto {
                if is_script(command) {
                    stats.edges_script += 1;
                    // Ruby is somebody else's problem -- `codex extract` reads
                    // it and dispositions every one. What lands here and is
                    // *not* Ruby is a wayto holding several commands on
                    // several lines, and `edges` has one command per row. Two
                    // of those exist and both used to vanish counted as
                    // script, which is the one thing this table exists to stop.
                    if !command.trim_start().starts_with(";e") {
                        if let (Some(&from), Some(&to)) = (
                            uid_of.get(&room.id),
                            target.parse::<i64>().ok().and_then(|i| uid_of.get(&i)),
                        ) {
                            unpublished.execute(rusqlite::params![
                                from,
                                to,
                                command.replace('\n', " ; ").chars().take(90).collect::<String>(),
                                "several commands in one wayto; an edge carries one, so this needs a prelude",
                            ])?;
                        }
                    }
                    continue;
                }
                let Ok(target_id) = target.parse::<i64>() else {
                    continue;
                };
                let Some(&to) = uid_of.get(&target_id) else {
                    // Either endpoint unmapped or instanced. Rule 2.
                    stats.edges_dangling += 1;
                    continue;
                };
                if ambiguous.contains(&(from, command.clone())) {
                    continue;
                }
                let time_ms = room
                    .timeto
                    .get(target)
                    .and_then(serde_json::Value::as_f64)
                    .map(|s| (s * 1000.0).round() as i64)
                    .unwrap_or(DEFAULT_TIME_MS);
                stats.edges += edge_stmt.execute(rusqlite::params![from, to, command, time_ms])?;
            }
        }
    }

    conn.execute_batch("COMMIT")?;
    Ok(stats)
}

/// A command that is Lich Ruby rather than something to send.
///
/// These are 3,037 of the mapdb's edges and carry kilobytes of embedded code.
/// They are not facts about the game and never cross into the codex; what they
/// describe belongs in `connectors`.
fn is_script(command: &str) -> bool {
    command.trim_start().starts_with(";e") || command.contains('\n') || command.contains('\t')
}

type Rules = Vec<(String, String, String, String)>;

fn tag_rules(conn: &Connection) -> rusqlite::Result<Rules> {
    let mut stmt = conn.prepare("SELECT pattern, type, mode, detail FROM tag_map")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })?;
    rows.collect()
}

/// The facet a tag means, if the map says.
fn apply_rules<'a>(rules: &'a Rules, tag: &str) -> Option<(&'a str, String)> {
    for (pattern, ty, mode, detail) in rules {
        match mode.as_str() {
            "exact" if pattern == tag => return Some((ty, detail.clone())),
            "prefix" if tag.starts_with(pattern.as_str()) => {
                return Some((ty, tag[pattern.len()..].trim_start_matches(':').to_string()));
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::open_in_memory;

    fn codex() -> Connection {
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

    const SAMPLE: &str = r#"[
      {"id":1,"uid":[100],"title":["[Centre]"],"location":"Icemule Trace",
       "tags":["urchin guide tc"],
       "wayto":{"2":"east","9":"north",
                "4":";e fput \"x\"" },
       "timeto":{"2":0.2,"9":0.2,"4":0.2}},
      {"id":2,"uid":[200],"title":["[Road]"],"location":"Icemule Trace",
       "tags":["bank"],
       "wayto":{"1":"west","3":"go archway"},
       "timeto":{"1":0.2,"3":"; e Char.level > 5 ? 0.2 : nil"}},
      {"id":3,"uid":[300],"title":["[Bank]"],"location":"the town of Icemule Trace",
       "wayto":{"2":"out"},"timeto":{"2":0.2}},
      {"id":4,"uid":[],"title":["[No uid]"],"wayto":{}},
      {"id":5,"uid":[501,502,503],"title":["[Instanced]"],"wayto":{"1":"out"}}
    ]"#;

    /// A room with no uid is not a room. One with several is kept as its
    /// first, because dropping them cost 12.5% of reachable rooms.
    #[test]
    fn unmapped_rooms_are_dropped_and_instanced_ones_keep_their_first_uid() {
        let conn = codex();
        let s = import(&conn, SAMPLE).expect("import");
        assert_eq!(s.rooms, 4, "three plain, one instanced kept as uid[0]");
        assert_eq!(
            s.rooms_unmapped, 1,
            "Lich knows a place the game does not name"
        );
        assert_eq!(
            s.rooms_instanced, 1,
            "counted, so the under-representation is visible"
        );

        let kept: i64 = conn
            .query_row("SELECT count(*) FROM rooms WHERE uid = 501", [], |r| {
                r.get(0)
            })
            .expect("count");
        assert_eq!(kept, 1, "the first uid");
        let others: i64 = conn
            .query_row(
                "SELECT count(*) FROM rooms WHERE uid IN (502, 503)",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(others, 0, "and only the first");
    }

    /// Rule 2: an edge is only as real as its endpoints.
    #[test]
    fn an_edge_to_an_unprojected_room_is_dropped() {
        let conn = codex();
        import(&conn, SAMPLE).expect("import");
        let to_nowhere: i64 = conn
            .query_row(
                "SELECT count(*) FROM edges WHERE command = 'north'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(to_nowhere, 0, "room 9 does not exist in the mapdb at all");
    }

    #[test]
    fn ruby_never_becomes_an_edge() {
        let conn = codex();
        let s = import(&conn, SAMPLE).expect("import");
        assert_eq!(s.edges_script, 1);
        let ruby: i64 = conn
            .query_row(
                "SELECT count(*) FROM edges WHERE command LIKE ';e%'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(ruby, 0);
    }

    /// The mapdb sometimes gives Ruby where a number belongs. One of those
    /// must not fail the parse, nor produce a zero-cost edge.
    #[test]
    fn a_conditional_cost_falls_back_rather_than_failing() {
        let conn = codex();
        import(&conn, SAMPLE).expect("import");
        let ms: i64 = conn
            .query_row(
                "SELECT time_ms FROM edges WHERE command = 'go archway'",
                [],
                |r| r.get(0),
            )
            .expect("edge");
        assert_eq!(ms, DEFAULT_TIME_MS);
    }

    #[test]
    fn locations_and_mapped_tags_become_facets() {
        let conn = codex();
        import(&conn, SAMPLE).expect("import");

        let loc: String = conn
            .query_row(
                "SELECT detail FROM room_facets WHERE room_uid = 300 AND type = 'location'",
                [],
                |r| r.get(0),
            )
            .expect("location");
        assert_eq!(loc, "the town of Icemule Trace");

        let guide: String = conn
            .query_row(
                "SELECT detail FROM room_facets WHERE type = 'urchin_guide'",
                [],
                |r| r.get(0),
            )
            .expect("guide");
        assert_eq!(guide, "tc", "the prefix rule keeps the tag's remainder");
    }

    /// Every build re-runs this against the same vendored file.
    #[test]
    fn importing_twice_changes_nothing() {
        let conn = codex();
        let first = import(&conn, SAMPLE).expect("first");
        let count = |t: &str| -> i64 {
            conn.query_row(&format!("SELECT count(*) FROM {t}"), [], |r| r.get(0))
                .expect("count")
        };
        let before = (count("rooms"), count("edges"), count("room_facets"));
        let second = import(&conn, SAMPLE).expect("second");
        assert_eq!(
            (count("rooms"), count("edges"), count("room_facets")),
            before
        );
        assert_eq!(first, second, "and it reports the same thing it did");
    }
}
