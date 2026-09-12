//! Evaluating a room set.
//!
//! A set is stored as terms and resolved on demand. The whole evaluation is
//! one SQL statement, which is the point: eight of the nine node types the
//! digraph design specifies are relational algebra SQLite already implements,
//! so a hand-written evaluator would be a query language reimplementing the
//! one underneath it.

use rusqlite::Connection;
use std::collections::BTreeSet;

/// Rooms in a set, sorted.
///
/// `BTreeSet` rather than `Vec` or `HashSet`: a set is a set, and callers
/// compare and print these, so a stable order costs nothing and removes a
/// class of flaky test.
///
/// An unknown name evaluates to the empty set rather than an error. A set is a
/// question about membership, and nothing is a member of a set that does not
/// exist — but see `is_defined`, because "empty" and "not a set" are different
/// things to a caller building a connector.
pub fn evaluate(conn: &Connection, name: &str) -> rusqlite::Result<BTreeSet<i64>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT r.uid
           FROM room_set_terms t
           JOIN rooms r
           LEFT JOIN room_facets f
                  ON f.room_uid = r.uid
                 AND f.type = t.key
                 AND (t.value = '' OR f.detail = t.value)
          WHERE t.set_name = ?1
            AND ( (t.op = 'facet' AND f.room_uid IS NOT NULL)
               OR (t.op = 'room'  AND r.uid = CAST(t.value AS INTEGER)) )
          ORDER BY r.uid",
    )?;
    let rows = stmt.query_map([name], |row| row.get::<_, i64>(0))?;
    rows.collect()
}

/// Does this set exist at all?
///
/// Distinct from evaluating to nothing. A connector whose `origin_set` names a
/// set nobody defined is broken; one whose set is currently empty is merely
/// unusable today, which a new room could change.
pub fn is_defined(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM room_sets WHERE name = ?1)",
        [name],
        |row| row.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::open_in_memory;

    fn fixture() -> Connection {
        let conn = open_in_memory().expect("open");
        conn.execute_batch(
            "
            INSERT INTO facet_types(type, class) VALUES ('bank','poi'), ('herb','resource');
            INSERT INTO rooms(uid) VALUES (1),(2),(3),(4);

            -- Two rooms report one spelling, one reports the other. This is the
            -- live Icemule case in miniature.
            INSERT INTO room_facets(room_uid, type, detail) VALUES
                (1,'location','Icemule Trace'),
                (2,'location','Icemule Trace'),
                (3,'location','the town of Icemule Trace'),
                (4,'location','Wehnimer''s Landing'),
                (2,'bank',''),
                (1,'herb','stalk_of_bluebells');

            INSERT INTO room_sets(name) VALUES ('icemule');
            INSERT INTO room_set_terms(set_name, seq, op, key, value) VALUES
                ('icemule', 0, 'facet', 'location', 'Icemule Trace'),
                ('icemule', 1, 'facet', 'location', 'the town of Icemule Trace');
            ",
        )
        .expect("fixture");
        conn
    }

    /// The case this slice exists for: one town, two things the game calls it.
    #[test]
    fn a_town_is_the_union_of_what_the_game_calls_it() {
        let conn = fixture();
        assert_eq!(
            evaluate(&conn, "icemule").expect("evaluate"),
            BTreeSet::from([1, 2, 3]),
            "both spellings, and not the Landing room"
        );
    }

    /// A room the store learns later joins without anyone re-running a tool.
    /// This is why a set is a definition and not a list.
    #[test]
    fn a_new_room_joins_a_set_it_belongs_in() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO rooms(uid) VALUES (9);
             INSERT INTO room_facets(room_uid, type, detail)
                  VALUES (9,'location','Icemule Trace');",
        )
        .expect("new room");
        assert!(evaluate(&conn, "icemule").expect("evaluate").contains(&9));
    }

    /// An empty `value` means "any detail", so a set can be every room holding
    /// a herb without naming 683 species.
    #[test]
    fn an_empty_value_matches_any_detail() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO room_sets(name) VALUES ('foragable');
             INSERT INTO room_set_terms(set_name, seq, op, key, value)
                  VALUES ('foragable', 0, 'facet', 'herb', '');",
        )
        .expect("set");
        assert_eq!(
            evaluate(&conn, "foragable").expect("evaluate"),
            BTreeSet::from([1])
        );
    }

    #[test]
    fn a_room_term_names_one_room_directly() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO room_sets(name) VALUES ('hand-picked');
             INSERT INTO room_set_terms(set_name, seq, op, key, value)
                  VALUES ('hand-picked', 0, 'room', '', '4');",
        )
        .expect("set");
        assert_eq!(
            evaluate(&conn, "hand-picked").expect("evaluate"),
            BTreeSet::from([4])
        );
    }

    /// Terms are unioned, so the order they appear in cannot change the answer.
    /// The file is sorted for readable diffs, not for meaning.
    ///
    /// Tested by defining the same two terms in the opposite order rather than
    /// by renumbering in place: `seq` is half the primary key, so an update
    /// swapping two of them collides with itself.
    #[test]
    fn term_order_does_not_change_the_set() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO room_sets(name) VALUES ('icemule-reversed');
             INSERT INTO room_set_terms(set_name, seq, op, key, value) VALUES
                ('icemule-reversed', 0, 'facet', 'location', 'the town of Icemule Trace'),
                ('icemule-reversed', 1, 'facet', 'location', 'Icemule Trace');",
        )
        .expect("reversed set");
        assert_eq!(
            evaluate(&conn, "icemule-reversed").expect("evaluate"),
            evaluate(&conn, "icemule").expect("evaluate")
        );
    }

    /// Evaluating is a question, not a mutation. Asked twice, answered the
    /// same — which is what lets a connector resolve its origins per plan.
    #[test]
    fn evaluating_twice_gives_the_same_set() {
        let conn = fixture();
        assert_eq!(
            evaluate(&conn, "icemule").expect("first"),
            evaluate(&conn, "icemule").expect("second")
        );
    }

    /// "Nothing is a member" and "there is no such set" are different facts,
    /// and a connector naming a set nobody defined is broken rather than
    /// merely unusable.
    #[test]
    fn an_undefined_set_is_distinguishable_from_an_empty_one() {
        let conn = fixture();
        conn.execute("INSERT INTO room_sets(name) VALUES ('empty')", [])
            .expect("set");

        assert!(evaluate(&conn, "empty").expect("evaluate").is_empty());
        assert!(evaluate(&conn, "nonexistent").expect("evaluate").is_empty());

        assert!(is_defined(&conn, "empty").expect("defined"));
        assert!(!is_defined(&conn, "nonexistent").expect("defined"));
    }

    /// The vocabulary is closed. A term cannot invent an operation.
    #[test]
    fn a_term_cannot_invent_an_operation() {
        let conn = fixture();
        conn.execute("INSERT INTO room_sets(name) VALUES ('x')", [])
            .expect("set");
        let err = conn.execute(
            "INSERT INTO room_set_terms(set_name, seq, op) VALUES ('x', 0, 'flood')",
            [],
        );
        assert!(err.is_err(), "flood is not an op yet, and saying so is the point");
    }

    /// A term belongs to a set that exists.
    #[test]
    fn a_term_cannot_belong_to_no_set() {
        let conn = fixture();
        let err = conn.execute(
            "INSERT INTO room_set_terms(set_name, seq, op, key, value)
             VALUES ('never-defined', 0, 'facet', 'location', 'x')",
            [],
        );
        assert!(err.is_err());
    }
}
