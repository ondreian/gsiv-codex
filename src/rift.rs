//! Exits that grow somewhere on a ring.
//!
//! The Rift's real navigation is 165 script edges the extractor drops, and
//! what survives is five hundred compass moves between rooms with no way to
//! leave. This reads the dropped ones.
//!
//! Each is the same shape: a list of rooms you may join at, a loop of moves,
//! and one or two objects to watch for. You walk the loop from wherever you
//! joined until the way out is standing in the room with you, then take it.
//!
//! ```text
//! ;e start_room = [ 2579, 2580, ... ];
//!    dirs = [ 'southwest', 'east', ... ];
//!    if index = start_room.index(Room.current.id);
//!      until checkloot.include?('door') or checkloot.include?('mirror');
//!        move dirs[index]; index += 1; index = 0 if index >= dirs.length;
//!      end;
//!      if checkloot.include?('door'); move 'go door';
//!      elsif checkloot.include?('mirror'); move 'go mirror'; end;
//!    end
//! ```
//!
//! Six things are sought across the Rift: `door`, `mirror`, `thread`, `maw`,
//! `staircase` and `fissure`.

use std::collections::BTreeMap;

/// One ring, and the way off it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Circuit {
    /// Rooms the ring may be joined at, each with its index into `moves`.
    pub entries: Vec<(i64, usize)>,
    /// The loop, in order.
    pub moves: Vec<String>,
    /// What to watch for, and what to send: `("door", "go door")`.
    pub exits: Vec<(String, String)>,
    /// Where the exit leads.
    pub to: i64,
}

/// Read every circuit out of the map database, keyed by a stable id.
pub fn read(json: &str) -> Result<BTreeMap<String, Circuit>, Box<dyn std::error::Error>> {
    let rooms: Vec<serde_json::Value> = serde_json::from_str(json)?;
    let uid_of: BTreeMap<i64, i64> = rooms
        .iter()
        .filter_map(|r| {
            Some((
                r.get("id")?.as_i64()?,
                r.get("uid")?.as_array()?.first()?.as_i64()?,
            ))
        })
        .collect();

    let mut out: BTreeMap<String, Circuit> = BTreeMap::new();
    for r in &rooms {
        let Some(wayto) = r.get("wayto").and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (target, command) in wayto {
            let Some(command) = command.as_str() else {
                continue;
            };
            if !command.contains("start_room") {
                continue;
            }
            let Some(to) = target.parse::<i64>().ok().and_then(|i| uid_of.get(&i)) else {
                continue;
            };
            let Some(circuit) = parse(command, &uid_of, *to) else {
                continue;
            };
            // Keyed by the ring itself as well as by where it goes: the 55
            // source rooms that encode one ring must collapse to a single row,
            // and two different rings that both end at 2635 looking for a door
            // must not. Keying on destination and quarry alone lost nine of
            // thirteen.
            let sought: Vec<&str> = circuit.exits.iter().map(|(n, _)| n.as_str()).collect();
            let joins_at = circuit.entries.first().map_or(0, |(uid, _)| *uid);
            let id = format!("rift:{}:{}:{joins_at}", sought.join("-"), to);
            out.entry(id).or_insert(circuit);
        }
    }
    Ok(out)
}

/// Pull the three lists out of one mini-script.
fn parse(command: &str, uid_of: &BTreeMap<i64, i64>, to: i64) -> Option<Circuit> {
    let rooms = between(command, "start_room = [", "]")?;
    let dirs = between(command, "dirs = [", "]")?;

    let entries: Vec<(i64, usize)> = rooms
        .split(',')
        .map(str::trim)
        .enumerate()
        .filter_map(|(i, id)| {
            // `nil` appears in a couple of the lists where a room is missing;
            // it holds the index so the rest stay aligned, and is not an entry.
            let uid = id.parse::<i64>().ok().and_then(|i| uid_of.get(&i))?;
            Some((*uid, i))
        })
        .collect();

    let moves: Vec<String> = dirs
        .split(',')
        .filter_map(|d| d.trim().strip_prefix('\'')?.strip_suffix('\''))
        .map(str::to_string)
        .collect();

    // `checkloot.include?('door')` says what to watch for; a nearby
    // `move '...'` naming it says what to send.
    //
    // The verb varies and must not be assumed. A door and a mirror are
    // entered -- `go door` -- and a thread is *climbed*. Matching on the noun
    // rather than on `go {noun}` is the difference between six circuits and
    // thirteen; the seven that were dropped were all threads.
    let sends: Vec<&str> = command
        .split("move '")
        .skip(1)
        .filter_map(|rest| rest.split('\'').next())
        .filter(|m| !m.contains("dirs["))
        .collect();

    let mut exits = Vec::new();
    for noun in command.split("checkloot.include?('").skip(1) {
        let Some(noun) = noun.split('\'').next() else {
            continue;
        };
        if exits.iter().any(|(n, _): &(String, String)| n == noun) {
            continue;
        }
        if let Some(send) = sends.iter().find(|m| m.contains(noun)) {
            exits.push((noun.to_string(), (*send).to_string()));
        }
    }

    if entries.is_empty() || moves.is_empty() || exits.is_empty() {
        return None;
    }
    Some(Circuit {
        entries,
        moves,
        exits,
        to,
    })
}

fn between<'a>(haystack: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let at = haystack.find(open)? + open.len();
    let rest = &haystack[at..];
    Some(&rest[..rest.find(close)?])
}

/// A SQL string literal, quotes doubled.
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// The overlay that publishes them.
pub fn to_sql(circuits: &BTreeMap<String, Circuit>) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "-- The Rift's circuits. Generated by `codex rift`; do not hand-edit.\n\
         --\n\
         -- {} rings. The Rift does not have exits so much as it grows them: a\n\
         -- door, a mirror, a thread, a maw, a staircase or a fissure appears in\n\
         -- some room of a ring, and the way out is to walk the ring until you\n\
         -- are standing with it.\n\
         --\n\
         -- Every one of these is a `;e` script edge in the map database, so all\n\
         -- 165 were dropped and the Rift was published as five hundred compass\n\
         -- moves between rooms with no way to leave.\n\n",
        circuits.len()
    ));

    for (id, c) in circuits {
        let sought: Vec<&str> = c.exits.iter().map(|(n, _)| n.as_str()).collect();
        s.push_str(&format!(
            "INSERT INTO circuits(id, to_uid, description) VALUES\n  \
             ({}, {}, {});\n",
            quote(id),
            c.to,
            quote(&format!(
                "{} rooms on the ring, {} moves; watch for {}.",
                c.entries.len(),
                c.moves.len(),
                sought.join(" or ")
            ))
        ));
        for (seq, m) in c.moves.iter().enumerate() {
            s.push_str(&format!(
                "INSERT INTO circuit_moves(circuit_id, seq, command) VALUES ({}, {seq}, {});\n",
                quote(id),
                quote(m)
            ));
        }
        for (uid, seq) in &c.entries {
            s.push_str(&format!(
                "INSERT OR IGNORE INTO rooms(uid) VALUES ({uid});\n\
                 INSERT INTO circuit_entries(circuit_id, room_uid, seq) VALUES ({}, {uid}, {seq});\n",
                quote(id)
            ));
        }
        for (noun, command) in &c.exits {
            s.push_str(&format!(
                "INSERT INTO circuit_exits(circuit_id, noun, command) VALUES ({}, {}, {});\n",
                quote(id),
                quote(noun),
                quote(command)
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
      {"id":1,"uid":[100],"location":"the Rift","wayto":{
        "9":";e start_room = [ 1, 2, nil, 3 ]; dirs = [ 'east', 'west', 'north', 'south', 'up' ]; if index = start_room.index(Room.current.id); until checkloot.include?('door') or checkloot.include?('mirror'); move dirs[index]; index += 1; index = 0 if index >= dirs.length; end; if checkloot.include?('door'); move 'go door'; elsif checkloot.include?('mirror'); move 'go mirror'; end;; else; echo 'error'; end"}},
      {"id":2,"uid":[200],"wayto":{}},
      {"id":3,"uid":[300],"wayto":{}},
      {"id":9,"uid":[900],"wayto":{}}
    ]"#;

    #[test]
    fn a_circuit_reads_its_ring_and_its_way_out() {
        let found = read(SAMPLE).expect("parse");
        assert_eq!(found.len(), 1);
        let c = found.values().next().expect("one");
        assert_eq!(c.to, 900);
        assert_eq!(
            c.moves,
            vec!["east", "west", "north", "south", "up"],
            "the whole loop, which is longer than the entry list"
        );
        assert_eq!(
            c.exits,
            vec![
                ("door".to_string(), "go door".to_string()),
                ("mirror".to_string(), "go mirror".to_string()),
            ],
            "either object is the way out"
        );
    }

    /// The verb is not always `go`. A thread is climbed, and assuming the verb
    /// silently dropped seven of the thirteen circuits -- every thread in the
    /// Rift -- while the six that used `go` looked like the whole story.
    #[test]
    fn a_thread_is_climbed_rather_than_entered() {
        const THREAD: &str = r#"[
          {"id":1,"uid":[100],"wayto":{
            "9":";e start_room = [ 1 ]; dirs = [ 'east' ]; if index = start_room.index(Room.current.id); until checkloot.include?('thread'); move dirs[index]; index += 1; index = 0 if index >= dirs.length; end; move 'climb thread'; waitrt?; fput 'stand'; else; echo 'error'; end"}},
          {"id":9,"uid":[900],"wayto":{}}
        ]"#;
        let found = read(THREAD).expect("parse");
        let c = found.values().next().expect("a circuit, not a silent drop");
        assert_eq!(
            c.exits,
            vec![("thread".to_string(), "climb thread".to_string())]
        );
    }

    /// `nil` holds a place in the entry list where a room is missing. It must
    /// not become an entry, and it must not shift the indices of the rooms
    /// after it -- they are offsets into the loop.
    #[test]
    fn a_hole_in_the_entry_list_keeps_the_indices_aligned() {
        let found = read(SAMPLE).expect("parse");
        let c = found.values().next().expect("one");
        assert_eq!(
            c.entries,
            vec![(100, 0), (200, 1), (300, 3)],
            "room 3 joins at index 3, not 2"
        );
    }
}
