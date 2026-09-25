//! Whether a built codex is fit to publish.
//!
//! These run against the artifact rather than against the schema. A constraint
//! says a row *cannot* be written; these say a database can be written entirely
//! within its constraints and still be wrong — a gate hung on nothing, a
//! mechanism whose commands cannot be sent, a road to a room the file has never
//! heard of.
//!
//! Every one of them is here because it happened. None is hypothetical.
//!
//! Run against whatever `CODEX` names, defaulting to the built artifact. In CI
//! that is the one the job just built; locally it is the one you are working
//! on.

use rusqlite::Connection;

/// The codex under test, or `None` when there is not one to test.
fn codex() -> Option<Connection> {
    // `CODEX` first, so CI tests the artifact the job just built. Otherwise
    // the installed one, which is what a person working on this has.
    let path = match std::env::var("CODEX") {
        Ok(p) => std::path::PathBuf::from(p),
        Err(_) => std::path::PathBuf::from(std::env::var("HOME").ok()?)
            .join(".local/share/urnon/codex.db3"),
    };
    if !path.exists() {
        eprintln!("no codex at {}; skipping", path.display());
        return None;
    }
    gsiv_codex::schema::open(&path).ok()
}

fn rows(conn: &Connection, sql: &str) -> Vec<String> {
    let Ok(mut stmt) = conn.prepare(sql) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let Ok(mut q) = stmt.query([]) else {
        return out;
    };
    while let Ok(Some(row)) = q.next() {
        out.push(row.get::<_, String>(0).unwrap_or_default());
    }
    out
}

/// A condition nobody references forbids nothing.
///
/// The Voln gate shipped this way: `voln-seeking` was defined, carried its
/// terms and its `forbid`, and was hung on no connector at all — so eight
/// Symbol of Seeking connectors offered a Voln-rank-26 road to anybody who
/// asked. It looked complete from every angle except this one.
#[test]
fn every_condition_is_hung_on_something() {
    let Some(conn) = codex() else { return };
    let orphans = rows(
        &conn,
        "SELECT c.id FROM conditions c
          WHERE NOT EXISTS (SELECT 1 FROM edge_conditions e WHERE e.condition_id = c.id)
            AND NOT EXISTS (SELECT 1 FROM connector_conditions k WHERE k.condition_id = c.id)
            AND NOT EXISTS (SELECT 1 FROM preludes p WHERE p.condition_id = c.id)
            AND NOT EXISTS (SELECT 1 FROM edge_costs w WHERE w.waived_by = c.id)
          ORDER BY c.id",
    );
    assert!(
        orphans.is_empty(),
        "conditions defined and referenced by nothing, so they forbid nothing: {orphans:?}"
    );
}

/// A mechanism with no commands cannot be executed by whoever is handed it.
#[test]
fn every_connector_destination_can_be_invoked() {
    let Some(conn) = codex() else { return };
    let silent = rows(
        &conn,
        "SELECT d.connector_id || ' -> ' || d.to_uid
           FROM connector_destinations d
          WHERE NOT EXISTS (
                  SELECT 1 FROM connector_steps s
                   WHERE s.connector_id = d.connector_id
                     AND s.kind = d.kind AND s.to_uid = d.to_uid)
          ORDER BY 1",
    );
    assert!(
        silent.is_empty(),
        "destinations with no steps, which a client cannot act on: {silent:?}"
    );
}

/// A `cycle` asks until the answer is the one it wants. Without the list of
/// answers it wants, it asks twenty times and gives up.
///
/// This is the shape Symbol of Seeking was published in before anything could
/// execute it, and the shape it briefly returned to when the accepted names
/// moved from the room to the step.
#[test]
fn every_cycling_step_knows_what_it_is_waiting_for() {
    let Some(conn) = codex() else { return };
    let deaf = rows(
        &conn,
        "SELECT s.connector_id || ' -> ' || s.to_uid || ' seq ' || s.seq
           FROM connector_steps s
          WHERE s.action = 'cycle'
            AND NOT EXISTS (
                  SELECT 1 FROM connector_step_matches m
                   WHERE m.connector_id = s.connector_id AND m.kind = s.kind
                     AND m.to_uid = s.to_uid AND m.seq = s.seq)
          ORDER BY 1",
    );
    assert!(
        deaf.is_empty(),
        "cycling steps with nothing to match against: {deaf:?}"
    );
}

/// A placeholder that names nothing cannot be filled.
///
/// `rub #{item}` and `go #{portal}` shipped for months. Both said a
/// substitution was wanted and neither said what to substitute, so the periapt
/// to the Sanctum was published and unexecutable the whole time.
#[test]
fn every_placeholder_names_its_target() {
    let Some(conn) = codex() else { return };
    let blind = rows(
        &conn,
        "SELECT connector_id || ': ' || command FROM connector_steps
          WHERE command LIKE '%#{item}%'
             OR command LIKE '%#{portal}%'
             OR command LIKE '%#{setting}%'
          ORDER BY 1",
    );
    assert!(
        blind.is_empty(),
        "placeholders with no target, which nothing can fill: {blind:?}"
    );
}

/// A road to a room the file has never heard of strands whoever takes it.
#[test]
fn every_road_leads_to_a_room_that_exists() {
    let Some(conn) = codex() else { return };
    let nowhere = rows(
        &conn,
        "SELECT d.connector_id || ' -> ' || d.to_uid
           FROM connector_destinations d
          WHERE d.kind = 'fixed'
            AND NOT EXISTS (SELECT 1 FROM rooms r WHERE r.uid = d.to_uid)
          UNION ALL
         SELECT 'circuit ' || c.id || ' -> ' || c.to_uid
           FROM circuits c
          WHERE NOT EXISTS (SELECT 1 FROM rooms r WHERE r.uid = c.to_uid)
          ORDER BY 1",
    );
    assert!(
        nowhere.is_empty(),
        "roads to rooms this codex does not describe: {nowhere:?}"
    );
}

/// A circuit is a ring you join and walk. Without entries there is no way on,
/// without moves there is no ring, and without an exit there is no way off.
#[test]
fn every_circuit_can_be_joined_walked_and_left() {
    let Some(conn) = codex() else { return };
    let broken = rows(
        &conn,
        "SELECT c.id || ': ' ||
                (SELECT count(*) FROM circuit_entries e WHERE e.circuit_id = c.id) || ' entries, ' ||
                (SELECT count(*) FROM circuit_moves m WHERE m.circuit_id = c.id) || ' moves, ' ||
                (SELECT count(*) FROM circuit_exits x WHERE x.circuit_id = c.id) || ' exits'
           FROM circuits c
          WHERE (SELECT count(*) FROM circuit_entries e WHERE e.circuit_id = c.id) = 0
             OR (SELECT count(*) FROM circuit_moves m WHERE m.circuit_id = c.id) = 0
             OR (SELECT count(*) FROM circuit_exits x WHERE x.circuit_id = c.id) = 0
          ORDER BY 1",
    );
    assert!(broken.is_empty(), "circuits missing a part: {broken:?}");
}

/// An entry index past the end of the ring sends a walker off the list.
#[test]
fn no_circuit_entry_points_past_its_ring() {
    let Some(conn) = codex() else { return };
    let past = rows(
        &conn,
        "SELECT e.circuit_id || ' room ' || e.room_uid || ' at ' || e.seq
           FROM circuit_entries e
          WHERE e.seq >= (SELECT count(*) FROM circuit_moves m WHERE m.circuit_id = e.circuit_id)
          ORDER BY 1",
    );
    assert!(past.is_empty(), "entries indexed past the ring: {past:?}");
}

/// A region marked transient that names no rooms changes nothing, and reads as
/// though it does — which is worse than not marking it.
#[test]
fn every_transient_region_covers_rooms() {
    let Some(conn) = codex() else { return };
    let empty = rows(
        &conn,
        "SELECT t.set_name FROM transient_exits t
          WHERE NOT EXISTS (
                  SELECT 1 FROM room_set_terms s
                    JOIN room_facets f ON f.type = s.key AND f.detail = s.value
                   WHERE s.set_name = t.set_name AND s.op = 'facet')
            AND NOT EXISTS (
                  SELECT 1 FROM room_set_terms s
                   WHERE s.set_name = t.set_name AND s.op = 'room')
          ORDER BY 1",
    );
    assert!(
        empty.is_empty(),
        "regions marked transient that cover no rooms: {empty:?}"
    );
}

/// A published artifact says what it is. Without this a downloaded file is a
/// filename and a hope.
#[test]
fn the_artifact_says_what_it_is() {
    let Some(conn) = codex() else { return };
    for key in ["version", "schema_version", "urnon_min", "built_at"] {
        let value: String = conn
            .query_row("SELECT value FROM codex_meta WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .unwrap_or_default();
        assert!(!value.is_empty(), "codex_meta is missing {key}");
    }

    // The schema version in the stamp must be the schema version of the file.
    // They drift when a migration lands and the stamp is written from a
    // constant somebody forgot to bump.
    let stamped: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM codex_meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .expect("a stamped schema version");
    let actual: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .expect("the file's own");
    assert_eq!(stamped, actual, "the stamp disagrees with the file");
}

/// Nothing vanishes without saying so.
///
/// Every wayto in the vendored mapdb must end up somewhere a reader can find:
/// published as an edge, recorded in the audit, recorded as an ambiguous
/// command, or pointing at a Lich room the game never names. There is no fifth
/// case, and until this test existed there were 5,158 of one — script edges
/// the extractor declined with a bare `continue`, leaving no trace of having
/// been looked at, plus 546 plain edges lost to building from urnon's store
/// instead of from the mapdb.
#[test]
fn every_wayto_is_published_or_accounted_for() {
    let Some(conn) = codex() else { return };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/map.json");
    let Ok(json) = std::fs::read_to_string(&path) else {
        eprintln!("no vendored mapdb at {}; skipping", path.display());
        return;
    };
    let Ok(rooms) = serde_json::from_str::<Vec<serde_json::Value>>(&json) else {
        eprintln!("mapdb did not parse; skipping");
        return;
    };

    // The same projection the importer uses: a Lich room stands for its first
    // uid, and a room with no uid at all is a place the game does not name.
    let mut uid_of = std::collections::HashMap::new();
    for r in &rooms {
        if let (Some(id), Some(uid)) = (
            r.get("id").and_then(serde_json::Value::as_i64),
            r.get("uid")
                .and_then(serde_json::Value::as_array)
                .and_then(|a| a.first())
                .and_then(serde_json::Value::as_i64),
        ) {
            uid_of.insert(id, uid);
        }
    }

    let set = |sql: &str| -> std::collections::HashSet<(i64, String)> {
        let Ok(mut stmt) = conn.prepare(sql) else {
            return Default::default();
        };
        let mut out = std::collections::HashSet::new();
        if let Ok(mut q) = stmt.query([]) {
            while let Ok(Some(row)) = q.next() {
                if let (Ok(a), Ok(b)) = (row.get(0), row.get::<_, String>(1)) {
                    out.insert((a, b));
                }
            }
        }
        out
    };
    let published = set("SELECT from_uid, CAST(to_uid AS TEXT) FROM edges");
    let audited = set("SELECT from_uid, CAST(to_uid AS TEXT) FROM script_edge_disposition");
    let ambiguous = set("SELECT from_uid, command FROM ambiguous_commands");
    if published.is_empty() {
        eprintln!("codex has no edges; skipping");
        return;
    }

    let mut silent = Vec::new();
    for r in &rooms {
        let Some(id) = r.get("id").and_then(serde_json::Value::as_i64) else {
            continue;
        };
        let Some(wayto) = r.get("wayto").and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (target, command) in wayto {
            let (Some(&from), Some(&to)) = (
                uid_of.get(&id),
                target.parse::<i64>().ok().and_then(|i| uid_of.get(&i)),
            ) else {
                continue; // an endpoint the game never names
            };
            let key = (from, to.to_string());
            if published.contains(&key) || audited.contains(&key) {
                continue;
            }
            let Some(command) = command.as_str() else {
                continue;
            };
            if ambiguous.contains(&(from, command.to_string())) {
                continue;
            }
            silent.push(format!(
                "{from} -> {to}: {}",
                command
                    .replace('\n', " ; ")
                    .chars()
                    .take(70)
                    .collect::<String>()
            ));
        }
    }
    silent.sort();
    assert!(
        silent.is_empty(),
        "{} wayto dropped with nothing published and nothing said: {:?}",
        silent.len(),
        &silent[..silent.len().min(8)]
    );
}
