//! The portmasters, as connectors.
//!
//! Nine piers and sixty-two routes, which is most of how the western coast
//! joins up. River's Rest could be walked to and not left; Kraken's Fall,
//! Seareach, Frostmain and the Isle of Ornath could be reached at all only by
//! ferry, and the ferry was a `;e` so it was dropped.
//!
//! # The cost carries its own gate
//!
//! The mapdb prices these with a `StringProc`, not a number:
//!
//! ```text
//! ;e UserVars.mapdb_use_portmasters == true ? 1200 : nil
//! ```
//!
//! Twenty minutes if the player has opted in, and `nil` -- impassable -- if
//! they have not. That is the same shape as `ice_mode`: a *preference*, a
//! thing the player chose, sitting in the same vocabulary as skills and
//! spells. Nobody wants a router silently putting them on a twenty-minute boat
//! because it shaved thirty seconds off a walk, and nobody should have to
//! discover that by finding themselves at sea.
//!
//! So `use-portmasters` is published as a condition that forbids, and the
//! twenty minutes is published as the cost. A client whose character has not
//! set the preference is never offered a ferry at all.
//!
//! # Why nine connectors
//!
//! The same reason Symbol of Seeking gets eight: the mapdb records pairs. Nine
//! piers over nine destinations is eighty-one, and there are sixty-two. The
//! nineteen that do not exist are mostly a pier to itself, but not all of them
//! -- the Isle of Ornath reaches three places and is reached from three, and
//! they are not the same three. One connector per pier keeps what is recorded
//! and invents nothing.

use std::collections::BTreeMap;

/// One numbered sailing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sailing {
    /// The `N` in `ask portmaster about travel N`.
    pub number: u32,
    pub to: i64,
    /// The game's own words on arrival, out of Lich's `waitfor`.
    pub expect: String,
    /// The fare, in silver, from the pier's `silver-cost:<room>:<amount>` tag.
    ///
    /// Zero where the map database records none, which is not the same as
    /// free -- it is unrecorded, and the safe reading of an unrecorded price
    /// is no price rather than an invented one.
    pub fare: i64,
}

/// `pier uid -> sailings`.
pub fn routes(json: &str) -> Result<BTreeMap<i64, Vec<Sailing>>, Box<dyn std::error::Error>> {
    let rooms: Vec<serde_json::Value> = serde_json::from_str(json)?;

    let mut uid_of: BTreeMap<i64, i64> = BTreeMap::new();
    for r in &rooms {
        let (Some(id), Some(uid)) = (
            r.get("id").and_then(serde_json::Value::as_i64),
            r.get("uid")
                .and_then(serde_json::Value::as_array)
                .and_then(|a| a.first())
                .and_then(serde_json::Value::as_i64),
        ) else {
            continue;
        };
        uid_of.insert(id, uid);
    }

    let mut out: BTreeMap<i64, Vec<Sailing>> = BTreeMap::new();
    for r in &rooms {
        let Some(from) = r
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .and_then(|i| uid_of.get(&i).copied())
        else {
            continue;
        };
        let Some(wayto) = r.get("wayto").and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (target, command) in wayto {
            let Some(command) = command.as_str() else {
                continue;
            };
            if !command.contains("portmaster") {
                continue;
            }
            let Some(to) = target
                .parse::<i64>()
                .ok()
                .and_then(|i| uid_of.get(&i).copied())
            else {
                continue;
            };
            let (Some(number), expect) = (travel_number(command), waitfor_text(command)) else {
                continue;
            };
            // The fare is a tag on the pier naming the destination, not part
            // of the command: `silver-cost:10838:25000`. Nothing else in the
            // map database says what a sailing costs.
            let fare = fare_to(r, target);
            let routes = out.entry(from).or_default();
            if !routes.iter().any(|s| s.number == number) {
                routes.push(Sailing {
                    number,
                    to,
                    expect,
                    fare,
                });
            }
        }
    }
    for routes in out.values_mut() {
        routes.sort_unstable_by_key(|s| s.number);
    }
    Ok(out)
}

/// The `silver-cost:<destination>:<amount>` tag on a pier, if it carries one.
fn fare_to(room: &serde_json::Value, target: &str) -> i64 {
    let prefix = format!("silver-cost:{target}:");
    room.get("tags")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .find_map(|t| t.strip_prefix(&prefix))
        .and_then(|amount| amount.parse().ok())
        .unwrap_or(0)
}

/// The `N` in `ask portmaster about travel N`.
fn travel_number(command: &str) -> Option<u32> {
    let at = command.find("travel ")?;
    command[at + 7..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()
}

/// The game's own words on arrival, out of Lich's `waitfor`.
fn waitfor_text(command: &str) -> String {
    let Some(at) = command.find("waitfor ") else {
        return String::new();
    };
    let rest = &command[at + 8..];
    for q in ['\'', '"'] {
        if let Some(inner) = rest.strip_prefix(q)
            && let Some(end) = inner.find(q)
        {
            return inner[..end].to_string();
        }
    }
    String::new()
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// The overlay that publishes them.
pub fn to_sql(routes: &BTreeMap<i64, Vec<Sailing>>) -> String {
    let total: usize = routes.values().map(Vec::len).sum();
    let mut s = String::new();
    s.push_str(&format!(
        "-- The portmasters. Generated by `codex ferry`; do not hand-edit.\n\
         --\n\
         -- {} piers, {total} routes. One connector per pier: the mapdb records\n\
         -- pairs, and nine piers over nine destinations would be eighty-one.\n\
         --\n\
         -- Twenty minutes, and only if the player asked for ferries. The mapdb\n\
         -- prices these with a StringProc rather than a number --\n\
         --\n\
         --   ;e UserVars.mapdb_use_portmasters == true ? 1200 : nil\n\
         --\n\
         -- and `nil` means impassable. That is the same shape as `ice_mode`: a\n\
         -- preference, a thing the player chose. Nobody wants a router quietly\n\
         -- putting them on a twenty-minute boat to save thirty seconds of\n\
         -- walking, and nobody should find that out at sea.\n\n",
        routes.len(),
    ));

    s.push_str(
        "INSERT INTO conditions(id, description) VALUES\n  \
         ('no-portmasters', 'the player has not opted in to ferry travel');\n\
         INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES\n  \
         ('no-portmasters', 0, 0, 'preference', 'use_portmasters', 'ne', 'true');\n\
         INSERT INTO condition_effects(condition_id, effect, amount) VALUES\n  \
         ('no-portmasters', 'forbid', 0);\n\n",
    );

    for (pier, dests) in routes {
        let id = format!("portmaster:{pier}");
        s.push_str(&format!(
            "INSERT INTO room_sets(name, description) VALUES\n  \
             ('pier:{pier}', 'A pier with a portmaster.');\n\
             INSERT INTO room_set_terms(set_name, seq, op, value) VALUES\n  \
             ('pier:{pier}', 0, 'room', '{pier}');\n\
             INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES\n  \
             ('{id}', 'only', 'pier:{pier}', 0, 'The portmaster, to {} places.');\n\
             INSERT INTO connector_conditions(connector_id, condition_id) VALUES\n  \
             ('{id}', 'no-portmasters');\n",
            dests.len()
        ));
        for sail in dests {
            let to = sail.to;
            s.push_str(&format!(
                "INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) \
                 VALUES ('{id}', 'fixed', {to}, 1200000);\n"
            ));
            // The fare. Twelve to thirty-five thousand silver is not pocket
            // change, and a router that cannot see it puts a character on a
            // boat they cannot pay for -- which they discover at the pier,
            // having walked there instead of somewhere useful.
            if sail.fare > 0 {
                s.push_str(&format!(
                    "INSERT INTO connector_costs(connector_id, kind, to_uid, resource, amount) \
                     VALUES ('{id}', 'fixed', {to}, 'silver', {});\n",
                    sail.fare
                ));
            }
        }
        for sail in dests {
            let (number, to, expect) = (sail.number, sail.to, &sail.expect);
            // Lich sends it twice through `multifput`. Once is the ask and
            // once is the confirmation the portmaster wants; both go.
            s.push_str(&format!(
                "INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect, \
                 timeout_ms) VALUES\n  \
                 ('{id}', 'fixed', {to}, 0, 'ask portmaster about travel {number}', '', 0),\n  \
                 ('{id}', 'fixed', {to}, 1, 'ask portmaster about travel {number}', {}, 1200000);\n",
                quote(expect)
            ));
        }
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      {"id":1,"uid":[100],"tags":["silver-cost:2:25000"],"wayto":{
        "2":";e multifput 'ask portmaster about travel 3','ask portmaster about travel 3';waitfor 'A crew member escorts you off the ship.'",
        "3":"north"}},
      {"id":2,"uid":[200],"wayto":{}},
      {"id":3,"uid":[300],"wayto":{}}
    ]"#;

    #[test]
    fn a_pier_carries_its_numbered_routes() {
        let found = routes(SAMPLE).expect("parse");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[&100],
            vec![Sailing {
                number: 3,
                to: 200,
                expect: "A crew member escorts you off the ship.".to_string(),
                fare: 25_000,
            }],
            "the number, the destination, the game's own words, and the fare"
        );
    }

    /// The gate is the cost. `nil` in the mapdb's `timeto` means impassable,
    /// and a preference nobody set is exactly that.
    #[test]
    fn a_ferry_nobody_asked_for_is_forbidden_rather_than_slow() {
        let sql = to_sql(&routes(SAMPLE).expect("parse"));
        assert!(sql.contains("'preference', 'use_portmasters', 'ne', 'true'"));
        assert!(sql.contains("'no-portmasters', 'forbid'"));
        assert!(sql.contains("'portmaster:100', 'no-portmasters'"));
        assert!(
            sql.contains("1200000"),
            "twenty minutes, when it is allowed"
        );
    }

    /// Twelve to thirty-five thousand silver is not pocket change, and the
    /// map database has recorded it all along on a tag nobody read. A router
    /// that cannot see the fare puts a character on a boat they cannot pay
    /// for, which they discover at the pier.
    #[test]
    fn the_fare_is_published_with_the_sailing() {
        let sql = to_sql(&routes(SAMPLE).expect("parse"));
        assert!(sql.contains(
            "INSERT INTO connector_costs(connector_id, kind, to_uid, resource, amount) \
             VALUES ('portmaster:100', 'fixed', 200, 'silver', 25000);"
        ));
    }

    /// An unrecorded fare is unrecorded, not free. Publishing a zero would
    /// state a price the map database never gave.
    #[test]
    fn a_sailing_with_no_recorded_fare_carries_no_price() {
        const NO_TAG: &str = r#"[
          {"id":1,"uid":[100],"wayto":{
            "2":";e multifput 'ask portmaster about travel 3','ask portmaster about travel 3';waitfor 'off the ship.'"}},
          {"id":2,"uid":[200],"wayto":{}}
        ]"#;
        let found = routes(NO_TAG).expect("parse");
        assert_eq!(found[&100][0].fare, 0);
        let sql = to_sql(&found);
        assert!(!sql.contains("connector_costs"));
    }

    /// Sent twice, as Lich sends it, and the arrival text is the game's.
    #[test]
    fn the_ask_goes_twice_and_the_expect_is_the_games_words() {
        let sql = to_sql(&routes(SAMPLE).expect("parse"));
        assert_eq!(
            sql.matches("ask portmaster about travel 3").count(),
            2,
            "once to ask, once to confirm"
        );
        assert!(sql.contains("'A crew member escorts you off the ship.'"));
    }
}
