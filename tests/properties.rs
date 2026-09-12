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
    assert!(again.is_err(), "the same species twice is one fact, not two");
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

    assert_eq!(orphans, vec![999], "room 999 does not exist and nothing said so");
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

/// `{item}` is the only placeholder. Anything else is a template language
/// arriving by the back door, and the first sign of it is a `{` nobody
/// declared.
#[test]
fn commands_carry_no_placeholder_but_item() {
    let conn = open_in_memory().expect("open");
    connector_fixture(&conn);
    conn.execute_batch(
        "INSERT INTO connector_destinations(connector_id, kind, to_uid) VALUES ('c','fixed',100);
         INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command) VALUES
            ('c','fixed',100,0,'turn #{item}'),
            ('c','fixed',100,1,'say {greeting} to {npc}');",
    )
    .expect("steps");

    let offenders: i64 = conn
        .query_row(
            "SELECT count(*) FROM connector_steps
              WHERE replace(command, '{item}', '') LIKE '%{%'",
            [],
            |r| r.get(0),
        )
        .expect("count");
    assert_eq!(offenders, 1, "a placeholder nobody agreed to slipped in");
}
