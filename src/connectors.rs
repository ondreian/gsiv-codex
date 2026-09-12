//! Reading connectors out of the codex.
//!
//! This is the shape a client receives. It is deliberately not urnon's
//! `Connector` type: that one carries a `Lease`, which is per-character and has
//! no business here. A client takes what this returns, decides whether the
//! character can use it right now, and only then offers it to a router.

use rusqlite::Connection;

/// Where a connector can be used from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origins {
    Anywhere,
    Only(String),
    AnywhereExcept(String),
}

/// Where a hop puts you.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationKind {
    /// A known room.
    Fixed(i64),
    /// Somewhere this cannot name. Read the room on arrival and route on.
    Replan,
    /// Back where this connector brought you from.
    Origin,
}

/// One command of a hop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub command: String,
    /// What the game prints on success. Empty when nothing reliable is known.
    pub expect: String,
    /// 0 means the client's own default.
    pub timeout_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub kind: DestinationKind,
    pub cost_ms: i64,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connector {
    pub id: String,
    pub origins: Origins,
    pub overhead_ms: i64,
    /// What the character must have. Empty for a mechanism anyone can use.
    pub requires: Vec<(String, String)>,
    pub destinations: Vec<Destination>,
}

/// Every connector, with its destinations and steps.
///
/// Read whole rather than by id: there are tens of these, a client wants all of
/// them to decide which apply, and three queries beats N+1 round trips through
/// a table that fits in a cache line's worth of pages.
pub fn load_all(conn: &Connection) -> rusqlite::Result<Vec<Connector>> {
    let mut connectors: Vec<Connector> = Vec::new();

    let mut stmt = conn.prepare(
        "SELECT id, origin_mode, COALESCE(origin_set, ''), overhead_ms
           FROM connectors ORDER BY id",
    )?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let mode: String = row.get(1)?;
        let set: String = row.get(2)?;
        Ok(Connector {
            id,
            origins: match mode.as_str() {
                "only" => Origins::Only(set),
                "anywhere_except" => Origins::AnywhereExcept(set),
                _ => Origins::Anywhere,
            },
            overhead_ms: row.get(3)?,
            requires: Vec::new(),
            destinations: Vec::new(),
        })
    })?;
    for c in rows {
        connectors.push(c?);
    }

    for c in &mut connectors {
        let mut stmt = conn.prepare(
            "SELECT kind, to_uid, cost_ms FROM connector_destinations
              WHERE connector_id = ?1 ORDER BY kind, to_uid",
        )?;
        let dests = stmt.query_map([&c.id], |row| {
            let kind: String = row.get(0)?;
            let to_uid: i64 = row.get(1)?;
            Ok(Destination {
                kind: match kind.as_str() {
                    "fixed" => DestinationKind::Fixed(to_uid),
                    "origin" => DestinationKind::Origin,
                    _ => DestinationKind::Replan,
                },
                cost_ms: row.get(2)?,
                steps: Vec::new(),
            })
        })?;
        for d in dests {
            c.destinations.push(d?);
        }

        for d in &mut c.destinations {
            let (kind, to_uid) = match d.kind {
                DestinationKind::Fixed(uid) => ("fixed", uid),
                DestinationKind::Replan => ("replan", 0),
                DestinationKind::Origin => ("origin", 0),
            };
            let mut stmt = conn.prepare(
                "SELECT command, expect, timeout_ms FROM connector_steps
                  WHERE connector_id = ?1 AND kind = ?2 AND to_uid = ?3
                  ORDER BY seq",
            )?;
            let steps = stmt.query_map(rusqlite::params![&c.id, kind, to_uid], |row| {
                Ok(Step {
                    command: row.get(0)?,
                    expect: row.get(1)?,
                    timeout_ms: row.get(2)?,
                })
            })?;
            for s in steps {
                d.steps.push(s?);
            }
        }

        let mut stmt = conn.prepare(
            "SELECT requirement, detail FROM connector_requires
              WHERE connector_id = ?1 ORDER BY requirement, detail",
        )?;
        let reqs = stmt.query_map([&c.id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        for r in reqs {
            c.requires.push(r?);
        }
    }

    Ok(connectors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::open_in_memory;

    /// The two real mechanisms, as they would actually be written: a town's
    /// urchin guides, and the FWI trinket's round trip.
    fn fixture() -> Connection {
        let conn = open_in_memory().expect("open");
        conn.execute_batch(
            "
            INSERT INTO rooms(uid) VALUES (4042150),(4043301),(4047001),(3201029);
            INSERT INTO room_sets(name) VALUES ('icemule'),('fwi-noteleport');

            INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES
                ('urchin-guides:icemule', 'only', 'icemule', 0, 'the urchin guides'),
                ('fwi-trinket', 'anywhere_except', 'fwi-noteleport', 3000, 'the trinket');

            INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) VALUES
                ('urchin-guides:icemule', 'fixed', 4043301, 1000),
                ('urchin-guides:icemule', 'fixed', 4047001, 1000),
                ('fwi-trinket', 'fixed', 3201029, 5000),
                ('fwi-trinket', 'origin', 0, 5000);

            INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect) VALUES
                ('urchin-guides:icemule','fixed',4043301,0,'urchin guide bank','You flag down a nearby urchin'),
                ('urchin-guides:icemule','fixed',4047001,0,'urchin guide temple','You flag down a nearby urchin'),
                ('fwi-trinket','fixed',3201029,0,'turn #{item}','You get the feeling'),
                ('fwi-trinket','origin',0,0,'turn #{item}','You get the feeling');

            INSERT INTO connector_requires(connector_id, requirement, detail) VALUES
                ('fwi-trinket', 'has-item', 'fwi trinket');
            ",
        )
        .expect("fixture");
        conn
    }

    #[test]
    fn a_towns_guides_load_with_their_commands() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let urchins = all.iter().find(|c| c.id.starts_with("urchin")).expect("found");

        assert_eq!(urchins.origins, Origins::Only("icemule".into()));
        assert_eq!(urchins.destinations.len(), 2);
        assert_eq!(
            urchins.destinations[0].steps[0].command,
            "urchin guide bank"
        );
        assert_eq!(
            urchins.destinations[0].steps[0].expect,
            "You flag down a nearby urchin",
            "the game's own words, not a client label"
        );
        assert!(urchins.requires.is_empty(), "the lease is not stored here");
    }

    /// The round trip, which is why the trinket is worth +1,374 and not
    /// +11,468: it goes somewhere and brings you back.
    #[test]
    fn a_round_trip_has_a_fixed_leg_and_an_origin_leg() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let fwi = all.iter().find(|c| c.id == "fwi-trinket").expect("found");

        let kinds: Vec<_> = fwi.destinations.iter().map(|d| d.kind).collect();
        assert!(kinds.contains(&DestinationKind::Fixed(3201029)));
        assert!(kinds.contains(&DestinationKind::Origin));
        assert_eq!(
            fwi.requires,
            vec![("has-item".to_string(), "fwi trinket".to_string())],
            "what, never where -- worn and stowed satisfy it alike"
        );
    }

    /// `anywhere` and a set are mutually exclusive: "anywhere, except this
    /// set" is a sentence somebody writes by accident.
    #[test]
    fn anywhere_cannot_also_name_a_set() {
        let conn = fixture();
        let err = conn.execute(
            "INSERT INTO connectors(id, origin_mode, origin_set)
             VALUES ('bad', 'anywhere', 'icemule')",
            [],
        );
        assert!(err.is_err());

        let err = conn.execute(
            "INSERT INTO connectors(id, origin_mode) VALUES ('alsobad', 'only')",
            [],
        );
        assert!(err.is_err(), "`only` what?");
    }

    /// The set has to exist. A connector whose origins name nothing is broken,
    /// not merely unusable.
    #[test]
    fn origins_cannot_name_an_undefined_set() {
        let conn = fixture();
        let err = conn.execute(
            "INSERT INTO connectors(id, origin_mode, origin_set)
             VALUES ('bad', 'only', 'no-such-set')",
            [],
        );
        assert!(err.is_err());
    }

    /// `replan` and `origin` do not name a room and `fixed` must. The key
    /// makes the first two singletons per connector as a side effect.
    #[test]
    fn only_a_fixed_destination_names_a_room() {
        let conn = fixture();

        let err = conn.execute(
            "INSERT INTO connector_destinations(connector_id, kind, to_uid)
             VALUES ('fwi-trinket', 'replan', 4042150)",
            [],
        );
        assert!(err.is_err(), "replan means it could not be named");

        let err = conn.execute(
            "INSERT INTO connector_destinations(connector_id, kind, to_uid)
             VALUES ('fwi-trinket', 'fixed', 0)",
            [],
        );
        assert!(err.is_err(), "fixed to where?");

        let err = conn.execute(
            "INSERT INTO connector_destinations(connector_id, kind, to_uid)
             VALUES ('fwi-trinket', 'origin', 0)",
            [],
        );
        assert!(err.is_err(), "one origin leg per connector, by the key");
    }

    /// Steps die with their destination, and destinations with their
    /// connector. Otherwise a retired mechanism leaves commands behind that
    /// nothing can reach and nothing can find.
    #[test]
    fn nothing_outlives_the_connector_it_belongs_to() {
        let conn = fixture();
        conn.execute("DELETE FROM connectors WHERE id = 'fwi-trinket'", [])
            .expect("delete");

        for table in ["connector_destinations", "connector_steps", "connector_requires"] {
            let left: i64 = conn
                .query_row(
                    &format!("SELECT count(*) FROM {table} WHERE connector_id = 'fwi-trinket'"),
                    [],
                    |r| r.get(0),
                )
                .expect("count");
            assert_eq!(left, 0, "{table} kept an orphan");
        }
    }

    /// Steps come back in the order they are sent, which is the only thing
    /// `seq` is for.
    #[test]
    fn steps_keep_their_order() {
        let conn = fixture();
        conn.execute_batch(
            "INSERT INTO connectors(id, origin_mode) VALUES ('band', 'anywhere');
             INSERT INTO connector_destinations(connector_id, kind, to_uid) VALUES ('band','fixed',4042150);
             INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command) VALUES
                ('band','fixed',4042150,1,'rub #{item}'),
                ('band','fixed',4042150,0,'twist #{item} to 3');",
        )
        .expect("band");

        let all = load_all(&conn).expect("load");
        let band = all.iter().find(|c| c.id == "band").expect("found");
        let commands: Vec<&str> = band.destinations[0]
            .steps
            .iter()
            .map(|s| s.command.as_str())
            .collect();
        assert_eq!(commands, vec!["twist #{item} to 3", "rub #{item}"]);
    }
}
