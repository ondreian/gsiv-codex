//! The Isle of Four Winds, as a connector.
//!
//! A trinket you turn. It takes you to Gardenia Commons from any of fourteen
//! town squares, and turning it again on the island takes you back to the room
//! you left — the map database remembers that in `mapdb_fwi_return_room`, which
//! is exactly what a destination of kind `origin` is for.
//!
//! # Why it matters
//!
//! Mist Harbor is where a lot of what a character needs actually lives, and
//! nothing here could reach it: every one of the fourteen roads onto the island
//! is a `;e` script edge, so the extractor dropped all fourteen and the island
//! had no published way in at all.
//!
//! # The gate is the cost function, again
//!
//! ```text
//! ;e (!UserVars.mapdb_fwi_trinket.nil? and !UserVars.mapdb_fwi_trinket.empty?)
//!      ? 15.0 : nil
//! ```
//!
//! Fifteen seconds if you have one, impassable if you do not — the same shape
//! as the portmasters and the ice. Premium is a second gate the island applies
//! on its own; the trinket is the one this connector can see.
//!
//! # Worn, not fetched
//!
//! The map database's version is mostly hand-juggling: if the trinket is stowed
//! it empties a hand, digs the trinket out, turns it, puts it back, and refills
//! the hand. None of that is expressible here — `empty_hands` is the same gap
//! that leaves sixteen climbs unpublished — and none of it is necessary if the
//! trinket is worn.
//!
//! So this publishes the worn case and only the worn case: one command, no
//! preludes, no hands. A character who keeps theirs in a backpack is not
//! offered the road rather than offered a road that aborts halfway.

use std::collections::BTreeSet;

/// Where the trinket works from, and where it lands.
pub struct Isle {
    /// Town-square uids the map database records a road from.
    pub anchors: BTreeSet<i64>,
    /// Uids on the island itself, from which turning it takes you home.
    pub island: BTreeSet<i64>,
    /// What the game calls those rooms. The island is a set of `location`
    /// facets rather than a list of uids -- fifteen rows instead of thirteen
    /// hundred, and it says "the island" rather than enumerating it.
    pub locations: BTreeSet<String>,
    /// Where the trinket lands: Gardenia Commons.
    pub arrival: i64,
}

/// The Lich room id the mechanism lands in, and the one every anchor delegates
/// to. Both are the map database's own numbers, not ours.
const ARRIVAL_LICH_ID: i64 = 3668;
const IMPLEMENTATION_LICH_ID: i64 = 7;

/// Read the roads onto the island out of the map database.
pub fn read(json: &str) -> Result<Isle, Box<dyn std::error::Error>> {
    let rooms: Vec<serde_json::Value> = serde_json::from_str(json)?;
    let uid_of =
        |r: &serde_json::Value| -> Option<i64> { r.get("uid")?.as_array()?.first()?.as_i64() };

    let mut anchors = BTreeSet::new();
    let mut island = BTreeSet::new();
    let mut locations = BTreeSet::new();
    let mut arrival = 0;

    for r in &rooms {
        let Some(uid) = uid_of(r) else { continue };
        let id = r
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(-1);
        if id == ARRIVAL_LICH_ID {
            arrival = uid;
        }
        // The island itself, by the map database's own location string. Both
        // spellings appear, and the mechanism tests for both.
        // Both of the map database's own spellings, and everything the game
        // files under them -- the private homes read as "Arborsong, inside the
        // island town of Mist Harbor" and the trinket works there too.
        if let Some(loc) = r.get("location").and_then(serde_json::Value::as_str) {
            if loc.contains("Mist Harbor") || loc.contains("Four Winds") {
                island.insert(uid);
                locations.insert(loc.to_string());
            }
        }
        // An anchor is a room that reaches the island by the trinket: either
        // it carries the implementation itself, or it delegates to the room
        // that does.
        let reaches = r
            .get("wayto")
            .and_then(serde_json::Value::as_object)
            .into_iter()
            .flatten()
            .filter(|(to, _)| to.as_str() == ARRIVAL_LICH_ID.to_string())
            .filter_map(|(_, cmd)| cmd.as_str())
            .any(|cmd| {
                cmd.contains("mapdb_fwi_trinket")
                    || cmd.contains(&format!("Map[{IMPLEMENTATION_LICH_ID}].wayto"))
            });
        if reaches {
            anchors.insert(uid);
        }
    }

    if arrival == 0 {
        return Err("the map database has no Gardenia Commons".into());
    }
    Ok(Isle {
        anchors,
        island,
        locations,
        arrival,
    })
}

/// A SQL string literal, quotes doubled.
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// The overlay that publishes it.
pub fn to_sql(isle: &Isle) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "-- The Isle of Four Winds. Generated by `codex fwi`; do not hand-edit.\n\
         --\n\
         -- A trinket you turn. {} town squares reach Gardenia Commons with it,\n\
         -- and turning it on the island takes you back to the room you left --\n\
         -- the map database keeps that in `mapdb_fwi_return_room`, which is what\n\
         -- a destination of kind `origin` is for.\n\
         --\n\
         -- Every one of those roads is a `;e` script edge, so the extractor\n\
         -- dropped all of them and the island had no published way in at all.\n\
         --\n\
         -- The gate is the cost function, as usual:\n\
         --\n\
         --   ;e (!UserVars.mapdb_fwi_trinket.nil? and\n\
         --       !UserVars.mapdb_fwi_trinket.empty?) ? 15.0 : nil\n\
         --\n\
         -- Fifteen seconds if you have one, impassable if you do not.\n\n",
        isle.anchors.len()
    ));

    s.push_str(
        "-- Two gates. The trinket has no fixed name -- players own different\n\
         -- ones -- so it is named by a setting rather than written down here,\n\
         -- and a character who has not named theirs is not offered the road.\n\
         INSERT INTO conditions(id, description) VALUES\n  \
         ('no-fwi-trinket', 'the character has not named their Four Winds trinket'),\n  \
         ('not-premium', 'the account is not a Premium subscription');\n\
         INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES\n  \
         ('no-fwi-trinket', 0, 0, 'preference', 'fwi_trinket', 'absent', ''),\n  \
         ('not-premium', 0, 0, 'stat', 'subscription', 'ne', 'Premium');\n\
         INSERT INTO condition_effects(condition_id, effect, amount) VALUES\n  \
         ('no-fwi-trinket', 'forbid', 0),\n  \
         ('not-premium', 'forbid', 0);\n\n",
    );

    // Outbound: the town squares that reach the island.
    s.push_str(
        "INSERT INTO room_sets(name, description) VALUES\n  \
         ('fwi-anchors', 'Town squares the Four Winds trinket works from.');\n",
    );
    for (seq, uid) in isle.anchors.iter().enumerate() {
        s.push_str(&format!(
            "INSERT INTO room_set_terms(set_name, seq, op, value) VALUES \
             ('fwi-anchors', {seq}, 'room', '{uid}');\n"
        ));
    }
    // `turn my #{setting:travel/fwi_trinket}` -- worn, and only worn. The map
    // database's version empties a hand, digs the trinket out of a container,
    // turns it, puts it back and refills the hand; none of that is expressible
    // here, and none of it is needed if the trinket is worn. A character who
    // keeps theirs in a backpack is not offered the road rather than offered
    // one that aborts halfway.
    s.push_str(&format!(
        "INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES\n  \
         ('fwi:trinket:out', 'only', 'fwi-anchors', 0,\n   \
         'Turn the Four Winds trinket, from a town square to the island.');\n\
         INSERT INTO connector_conditions(connector_id, condition_id) VALUES\n  \
         ('fwi:trinket:out', 'no-fwi-trinket'),\n  \
         ('fwi:trinket:out', 'not-premium');\n\
         INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) VALUES\n  \
         ('fwi:trinket:out', 'fixed', {}, 15000);\n\
         INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect, timeout_ms)\n  \
         VALUES ('fwi:trinket:out', 'fixed', {}, 0,\n    \
         'turn my #{{setting:travel/fwi_trinket}}', 'You get the feeling', 15000);\n\n",
        isle.arrival, isle.arrival
    ));

    // Inbound: anywhere on the island, back to wherever you turned it.
    s.push_str(
        "-- The way back. `origin` rather than a room, because the trinket\n\
         -- returns you to where you came from and no row can name that in\n\
         -- advance -- which is exactly the kind the schema has for it.\n\
         --\n\
         -- The island is named by its `location` facets rather than by listing\n\
         -- thirteen hundred uids. The private homes file under their own names\n\
         -- (\"Arborsong, inside the island town of Mist Harbor\") and the\n\
         -- trinket works there too.\n\
         INSERT INTO room_sets(name, description) VALUES\n  \
         ('fwi-island', 'Everywhere the Four Winds trinket takes you home from.');\n",
    );
    for (seq, loc) in isle.locations.iter().enumerate() {
        s.push_str(&format!(
            "INSERT INTO room_set_terms(set_name, seq, op, key, value) VALUES \
             ('fwi-island', {seq}, 'facet', 'location', {});\n",
            quote(loc)
        ));
    }
    s.push_str(
        "INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES\n  \
         ('fwi:trinket:home', 'only', 'fwi-island', 0,\n   \
         'Turn the Four Winds trinket again, back to where you left.');\n\
         INSERT INTO connector_conditions(connector_id, condition_id) VALUES\n  \
         ('fwi:trinket:home', 'no-fwi-trinket'),\n  \
         ('fwi:trinket:home', 'not-premium');\n\
         INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) VALUES\n  \
         ('fwi:trinket:home', 'origin', 0, 15000);\n\
         INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect, timeout_ms)\n  \
         VALUES ('fwi:trinket:home', 'origin', 0, 0,\n    \
         'turn my #{setting:travel/fwi_trinket}', 'You get the feeling', 15000);\n",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      {"id":7,"uid":[13101017],"location":"Ta'Illistim","wayto":{
        "3668":";e worn = !GameObj[UserVars.mapdb_fwi_trinket].nil?; turn"}},
      {"id":284,"uid":[7046],"location":"the town of Wehnimer's Landing","wayto":{
        "3668":";e Map[7].wayto['3668'].call;"}},
      {"id":3668,"uid":[3201029],"location":"the Isle of Four Winds","wayto":{}},
      {"id":3672,"uid":[3201913],"location":"the Isle of Four Winds","wayto":{}},
      {"id":999,"uid":[999],"location":"somewhere else","wayto":{"3668":"north"}}
    ]"#;

    #[test]
    fn the_anchors_are_the_rooms_that_turn_the_trinket() {
        let isle = read(SAMPLE).expect("parse");
        assert_eq!(isle.arrival, 3_201_029, "Gardenia Commons");
        assert!(isle.anchors.contains(&13_101_017), "the implementation");
        assert!(isle.anchors.contains(&7046), "and everyone who delegates");
        assert!(
            !isle.anchors.contains(&999),
            "a room that merely walks north to the same id is not an anchor"
        );
    }

    /// Turning it on the island takes you home, so the island needs naming
    /// too. Both of the map database's spellings count.
    #[test]
    fn the_island_is_collected_for_the_way_back() {
        let isle = read(SAMPLE).expect("parse");
        assert!(isle.island.contains(&3_201_029));
        assert!(isle.island.contains(&3_201_913), "the bank, among the rest");
        assert!(!isle.island.contains(&7046));
    }
}
