//! Properties of the schema itself.
//!
//! Two kinds of property live in this repo and this file holds the first kind
//! at its smallest scale. **Data properties** are assertions over rows — "no
//! facet names a type that does not exist" — and run against the built
//! artifact in CI. **Code properties** are `proptest` over generated inputs and
//! arrive with the set evaluator.
//!
//! What is asserted here is that the schema *refuses* things, not that it
//! accepts them. A constraint nobody has watched reject anything is a comment.

use gsiv_codex::schema::open_in_memory;
use rusqlite::Connection;

fn seed_type(conn: &Connection, ty: &str, class: &str) {
    conn.execute(
        "INSERT INTO facet_types(type, class) VALUES (?1, ?2)",
        rusqlite::params![ty, class],
    )
    .expect("facet type");
}

fn seed_room(conn: &Connection, uid: i64) {
    conn.execute(
        "INSERT INTO rooms(uid, title) VALUES (?1, ?2)",
        rusqlite::params![uid, format!("[room {uid}]")],
    )
    .expect("room");
}

/// The relation the polymorphic alternative could not have had. A facet naming
/// a room that does not exist is refused at write time.
#[test]
fn a_facet_cannot_name_a_room_that_does_not_exist() {
    let conn = open_in_memory().expect("open");
    seed_type(&conn, "bank", "poi");

    let err = conn.execute(
        "INSERT INTO room_facets(room_uid, type) VALUES (?1, ?2)",
        rusqlite::params![4042150, "bank"],
    );
    assert!(err.is_err(), "no such room, and nothing said so");
}

/// The vocabulary is closed. A typo is a foreign key violation rather than a
/// twenty-first kind of thing nobody meant to invent.
#[test]
fn a_facet_cannot_invent_its_own_type() {
    let conn = open_in_memory().expect("open");
    seed_room(&conn, 4042150);

    let err = conn.execute(
        "INSERT INTO room_facets(room_uid, type) VALUES (?1, ?2)",
        rusqlite::params![4042150, "bnak"],
    );
    assert!(err.is_err(), "bnak is not a facet type");
}

/// Class is the router's contract: `poi` is the only class it may aim at.
/// Anything outside the four is refused.
#[test]
fn a_facet_type_cannot_invent_its_own_class() {
    let conn = open_in_memory().expect("open");
    let err = conn.execute(
        "INSERT INTO facet_types(type, class) VALUES ('bank', 'landmark')",
        [],
    );
    assert!(err.is_err(), "landmark is not one of the four classes");
}

/// A room holds many herbs, so `(room, type)` cannot be the key — but the same
/// species twice in one room is a duplicate fact.
#[test]
fn a_room_holds_many_details_of_one_type_but_each_only_once() {
    let conn = open_in_memory().expect("open");
    seed_type(&conn, "herb", "resource");
    seed_room(&conn, 4042150);

    for species in ["stalk_of_bluebells", "wolifrew_lichen"] {
        conn.execute(
            "INSERT INTO room_facets(room_uid, type, detail) VALUES (?1, 'herb', ?2)",
            rusqlite::params![4042150, species],
        )
        .expect("two species in one room is ordinary");
    }
    let again = conn.execute(
        "INSERT INTO room_facets(room_uid, type, detail) VALUES (?1, 'herb', 'wolifrew_lichen')",
        rusqlite::params![4042150],
    );
    assert!(
        again.is_err(),
        "the same species twice is one fact, not two"
    );
}

/// Foreign keys are off by default in SQLite, so every guarantee above rests
/// on the pragma actually being set. Asserted directly rather than inferred
/// from the tests passing, because they would also pass if the inserts failed
/// for some other reason.
#[test]
fn foreign_keys_are_actually_enforced() {
    let conn = open_in_memory().expect("open");
    let on: i64 = conn
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .expect("pragma");
    assert_eq!(on, 1);
}

/// Deleting a room takes its facets with it. Without this a rebuild that drops
/// a room leaves orphans that no foreign key can catch, because the parent is
/// simply gone.
#[test]
fn facets_do_not_outlive_their_room() {
    let conn = open_in_memory().expect("open");
    seed_type(&conn, "bank", "poi");
    seed_room(&conn, 4042150);
    conn.execute(
        "INSERT INTO room_facets(room_uid, type) VALUES (4042150, 'bank')",
        [],
    )
    .expect("facet");

    conn.execute("DELETE FROM rooms WHERE uid = 4042150", [])
        .expect("delete");
    let left: i64 = conn
        .query_row("SELECT count(*) FROM room_facets", [], |r| r.get(0))
        .expect("count");
    assert_eq!(left, 0);
}

// --- connectors -------------------------------------------------------------
//
// `connector_destinations.to_uid` carries no foreign key, because the primary
// key is the content — `(connector_id, kind, to_uid)` — and the two kinds that
// name no room store 0 there rather than NULL. A `dest_seq` surrogate would
// have allowed the FK and would renumber every later row when a destination is
// inserted in the middle, which is the churn the bootstrap format exists to
// avoid. These are the checks that buys back.

fn connector_fixture(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO rooms(uid) VALUES (100),(200);
         INSERT INTO room_sets(name) VALUES ('somewhere');
         INSERT INTO connectors(id, origin_mode, origin_set) VALUES ('c','only','somewhere');",
    )
    .expect("fixture");
}

/// What the missing foreign key would have caught.
#[test]
fn every_fixed_destination_names_a_room_that_exists() {
    let conn = open_in_memory().expect("open");
    connector_fixture(&conn);
    conn.execute_batch(
        "INSERT INTO connector_destinations(connector_id, kind, to_uid) VALUES
            ('c','fixed',100),
            ('c','fixed',999);",
    )
    .expect("destinations");

    let orphans: Vec<i64> = conn
        .prepare(
            "SELECT d.to_uid FROM connector_destinations d
              LEFT JOIN rooms r ON r.uid = d.to_uid
              WHERE d.kind = 'fixed' AND r.uid IS NULL
              ORDER BY d.to_uid",
        )
        .expect("prepare")
        .query_map([], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows");

    assert_eq!(
        orphans,
        vec![999],
        "room 999 does not exist and nothing said so"
    );
}

/// A destination nobody can reach because it has no commands is a row that
/// looks complete and does nothing. Not expressible as a constraint — SQL
/// cannot require a child — so it is a property.
#[test]
fn every_destination_has_at_least_one_step() {
    let conn = open_in_memory().expect("open");
    connector_fixture(&conn);
    conn.execute_batch(
        "INSERT INTO connector_destinations(connector_id, kind, to_uid) VALUES
            ('c','fixed',100),
            ('c','fixed',200);
         INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command)
              VALUES ('c','fixed',100,0,'go somewhere');",
    )
    .expect("destinations");

    let stepless: i64 = conn
        .query_row(
            "SELECT count(*) FROM connector_destinations d
              WHERE NOT EXISTS (SELECT 1 FROM connector_steps s
                                 WHERE s.connector_id = d.connector_id
                                   AND s.kind = d.kind AND s.to_uid = d.to_uid)",
            [],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(stepless, 1, "room 200 has no way to be reached");
}

/// Steps are sent in `seq` order, so a gap or a repeat is a hop that sends
/// commands in an order nobody chose.
#[test]
fn step_sequences_start_at_zero_and_do_not_skip() {
    let conn = open_in_memory().expect("open");
    connector_fixture(&conn);
    conn.execute_batch(
        "INSERT INTO connector_destinations(connector_id, kind, to_uid) VALUES ('c','fixed',100);
         INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command) VALUES
            ('c','fixed',100,0,'first'),
            ('c','fixed',100,2,'third, and there is no second');",
    )
    .expect("steps");

    let broken: i64 = conn
        .query_row(
            "SELECT count(*) FROM (
               SELECT connector_id, kind, to_uid
                 FROM connector_steps
                GROUP BY connector_id, kind, to_uid
               HAVING min(seq) <> 0 OR max(seq) <> count(*) - 1)",
            [],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(broken, 1, "0 then 2 is a gap");
}

/// `{item}` and `{portal}` are the placeholders. Anything else is a template
/// language arriving by the back door, and the first sign of it is a `{`
/// nobody declared.
#[test]
fn commands_carry_no_placeholder_but_item() {
    let conn = open_in_memory().expect("open");
    connector_fixture(&conn);
    conn.execute_batch(
        "INSERT INTO connector_destinations(connector_id, kind, to_uid) VALUES ('c','fixed',100);
         INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command) VALUES
            ('c','fixed',100,0,'turn #{item}'),
            ('c','fixed',100,1,'go #{portal}'),
            ('c','fixed',100,2,'say {greeting} to {npc}');",
    )
    .expect("steps");

    let offenders: i64 = conn
        .query_row(
            "SELECT count(*) FROM connector_steps
              WHERE replace(replace(command, '{item}', ''), '{portal}', '') LIKE '%{%'",
            [],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(offenders, 1, "a placeholder nobody agreed to slipped in");
}

// ---------------------------------------------------------------------------
// Traversal failures
//
// These run against the committed TSV rather than a fixture: the data *is* the
// deliverable here, and a schema that would reject a bad row proves nothing
// about the 124 rows actually checked in.
// ---------------------------------------------------------------------------

/// The committed vocabulary, loaded the way a build loads it.
fn failures() -> Connection {
    let conn = open_in_memory().expect("open");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/vocabulary");
    for name in [
        "060_failure_classes.tsv",
        "061_failure_patterns.tsv",
        "062_failure_remedies.tsv",
    ] {
        gsiv_codex::tsv::load(&conn, &dir.join(name)).expect(name);
    }
    conn
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).expect("count")
}

/// A class nothing can match is a class that does not exist.
#[test]
fn every_failure_class_has_at_least_one_pattern() {
    let conn = failures();
    let orphans = count(
        &conn,
        "SELECT count(*) FROM failure_classes c
          WHERE NOT EXISTS (SELECT 1 FROM failure_patterns p WHERE p.class_id = c.id)",
    );
    assert_eq!(orphans, 0);
}

/// Two classes claiming the same message would make the answer depend on which
/// row came back first. Overlap is resolved by `precedence`; an exact
/// duplicate is resolved by nothing.
#[test]
fn no_message_is_claimed_by_two_classes() {
    let conn = failures();
    let shared = count(
        &conn,
        "SELECT count(*) FROM (SELECT pattern FROM failure_patterns
                                GROUP BY pattern HAVING count(DISTINCT class_id) > 1)",
    );
    assert_eq!(shared, 0, "two classes claim one message verbatim");
}

/// The order Lich evaluated its branches in, which a set would otherwise lose.
/// `UNIQUE` enforces distinctness; this says the column is populated with
/// something meaningful rather than defaulted to a pile of zeroes.
#[test]
fn precedence_orders_every_class() {
    let conn = failures();
    let classes = count(&conn, "SELECT count(*) FROM failure_classes");
    let ordered = count(
        &conn,
        "SELECT count(DISTINCT precedence) FROM failure_classes",
    );
    assert_eq!(classes, ordered);
    assert_eq!(
        count(
            &conn,
            "SELECT count(*) FROM failure_classes WHERE precedence <= 0"
        ),
        0
    );
}

/// `sleep` is milliseconds and `cast` is a spell number. A remedy whose detail
/// does not parse is one a client will discover at the worst moment.
#[test]
fn a_remedy_that_takes_a_number_carries_one() {
    let conn = failures();
    let bad = count(
        &conn,
        "SELECT count(*) FROM failure_remedies
          WHERE remedy IN ('sleep', 'cast')
            AND (detail = '' OR CAST(detail AS INTEGER) <= 0)",
    );
    assert_eq!(bad, 0);
}

/// And the ones that take no argument carry none, so nobody starts encoding
/// meaning into a field the vocabulary says is empty.
#[test]
fn a_remedy_that_takes_no_argument_carries_none() {
    let conn = failures();
    let bad = count(
        &conn,
        "SELECT count(*) FROM failure_remedies
          WHERE remedy IN ('wait-rt','wait-stun','stand','empty-hands',
                           'open-the-way','unhide','retreat','stow-feet')
            AND detail <> ''",
    );
    assert_eq!(bad, 0);
}

/// Remedies are performed in order, so a gap means one silently never runs.
#[test]
fn remedies_are_numbered_without_gaps() {
    let conn = failures();
    let broken = count(
        &conn,
        "SELECT count(*) FROM (
           SELECT class_id FROM failure_remedies GROUP BY class_id
            HAVING min(seq) <> 0 OR max(seq) <> count(*) - 1)",
    );
    assert_eq!(broken, 0);
}

/// The size of the thing, asserted so a regeneration that quietly drops half
/// the vocabulary is a failing test rather than a smaller file.
///
/// Lich 5.17.1, `lib/global_defs.rb`, `move()`.
#[test]
fn the_whole_of_lichs_move_is_here() {
    let conn = failures();
    // Lich's half, counted by where the rows say they came from. The
    // vocabulary is Lich's plus what we have measured -- `rifted` came from
    // gswiki, and more will -- so a total would grow every time we learn
    // something and stop guarding the thing it was written to guard: that
    // Lich's own branches are all still here.
    let lich = "source LIKE 'global_defs.rb:%'";
    assert_eq!(
        count(
            &conn,
            &format!("SELECT count(*) FROM failure_classes WHERE {lich}")
        ),
        29
    );
    assert_eq!(
        count(
            &conn,
            &format!(
                "SELECT count(*) FROM failure_patterns WHERE class_id IN
                   (SELECT id FROM failure_classes WHERE {lich})"
            )
        ),
        124
    );
    assert_eq!(count(&conn, "SELECT count(*) FROM failure_remedies"), 40);

    // One of each of the three verdicts that are not `retry`, because each is
    // a different instruction to a router and losing one is invisible.
    for (disposition, n) in [("edge-wrong", 1), ("unavailable", 2), ("arrived", 1)] {
        let got = count(
            &conn,
            &format!("SELECT count(*) FROM failure_classes WHERE disposition = '{disposition}'"),
        );
        assert_eq!(got, n, "{disposition}");
    }
}

/// The patterns are Ruby regexes. Exactly one uses a construct Rust's `regex`
/// crate does not have, and the schema documents how to read it; a second one
/// appearing should be a conversation, not a silent `unwrap` in a client.
#[test]
fn exactly_one_pattern_needs_a_lookahead() {
    let conn = failures();
    let mut stmt = conn
        .prepare("SELECT pattern FROM failure_patterns")
        .expect("prepare");
    let patterns: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .expect("query")
        .map(|p| p.expect("pattern"))
        .collect();

    let exotic: Vec<&String> = patterns
        .iter()
        .filter(|p| p.contains("(?!") || p.contains("(?<"))
        .collect();
    assert_eq!(
        exotic.len(),
        1,
        "patterns needing a lookahead-capable engine: {exotic:?}"
    );
    assert!(exotic[0].starts_with("^You are already"));
}
