//! Reading real commands out of the mapdb's Ruby.
//!
//! A large minority of script edges are walks in costume: some bookkeeping,
//! then `move 'west'`. Extracting the command is *analysis* rather than
//! something the mapdb states, so this emits SQL into `data/` for review
//! instead of writing rows during import. A mapdb refresh re-runs it.
//!
//! # It only extracts what it understands
//!
//! Ending in `move 'X'` is not enough. Silverwood's door reads
//!
//! ```text
//! ;e $SILVERWOOD_TOWN=:imt;move 'go door'
//! ```
//!
//! and taking `go door` from it discards the only part that matters — which
//! town you came from, and therefore which of four rooms the door leads back
//! to. So the *body* is classified, and an unrecognised body means the edge is
//! left alone and listed as unhandled with its reason. A wrong edge is worse
//! than a missing one: a missing edge fails to route, and a wrong edge routes
//! a character somewhere they did not ask to go.

use std::collections::BTreeMap;

/// What the body before the final `move` turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// Nothing at all: `;e move 'west'`.
    Bare,
    /// A recognised condition, by its id in `data/040_conditions.sql`.
    Condition(&'static str),
    /// Something this does not model. Carries a short excerpt so the
    /// disposition row can say what was skipped.
    Unrecognised(String),
    /// Understood, and deliberately never published. Carries the reason.
    ///
    /// Distinct from `Unrecognised`, which is a gap somebody may close. This
    /// is a decision: the urchin network is a connector rather than an edge,
    /// and Silverwood's door is gated on something no client can observe.
    Excluded(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    pub from_uid: i64,
    pub to_uid: i64,
    pub command: String,
    pub time_ms: i64,
    pub body: Body,
}

/// Pull the trailing `move 'X'` off a `;e` command, if there is one.
fn trailing_move(command: &str) -> Option<(&str, String)> {
    let c = command.trim();
    let at = c.rfind("move")?;
    let rest = c[at + 4..].trim_start();
    let inner = rest.strip_prefix('\'')?;
    let end = inner.find('\'')?;
    // Anything after the closing quote but a `;` means the move is not the
    // last thing that happens, and what follows it may matter.
    if !inner[end + 1..].trim().trim_matches(';').is_empty() {
        return None;
    }
    let body = c.strip_prefix(";e")?[..at - 2]
        .trim()
        .trim_matches(';')
        .trim();
    Some((body, inner[..end].to_string()))
}

/// `;e table = "red"; fput "go #{table} table" if dothistimeout("go #{table}
/// table", 25, /You ... head over to|waves.*you.*invi/)`
///
/// Half of every room the codex orphans is one of these -- 455 of 904, four
/// times the next cause and more than every connector put together. Taverns
/// and gaming halls are full of tables you `go` to, and the mapdb records the
/// way out of one as a plain move and the way in as this.
///
/// It reads as a program and it is an edge. `table` is assigned a literal in
/// every single body, the interpolation puts it straight back, and everything
/// around it -- the timeout, the alternation of replies, the `if` -- is
/// retry-and-confirm. That is the client's job and urnon's walker already does
/// it better, by checking the room id rather than the prose.
///
/// The reply text is dropped with the rest. `edge_overlays` has nowhere to put
/// it, and arriving in the room you aimed at is stronger evidence than the
/// game saying you did.
fn table_move(command: &str) -> Option<String> {
    let c = command.trim();
    // The interpolation is the shape. Without it, `table` is some other
    // variable in some other program.
    if !c.contains("go #{table} table") {
        return None;
    }
    let at = c.find("table")?;
    let rest = c[at + 5..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let inner = rest.strip_prefix('"')?;
    let end = inner.find('"')?;
    let name = &inner[..end];
    // A name with a quote or a newline in it is not a name; a name with `#{`
    // in it is another interpolation and this cannot resolve it.
    if name.is_empty() || name.contains("#{") {
        return None;
    }
    Some(format!("go {name} table"))
}

/// `;e true` is NOT a move, and must never be extracted as one.
///
/// It reads like an empty body with nothing to do, and it is the urchin
/// network: all 523 of them land in an Urchin Hideout, and every source room
/// carries `urchin guide <somewhere>` and `urchin-access`. The command is
/// empty because the *urchin* does the moving -- you flag one down and it
/// takes you. Publishing it as an edge would publish a road that sends
/// nothing and expects you to arrive somewhere else.
///
/// It is a connector, and urnon already has the shape for it -- origin room
/// straight to destination room, with no hideout in between.
///
/// This guard fires zero times today, and the reason is worth knowing: all 16
/// Urchin Hideout rooms have no `uid`, so they are among the 7,884 rooms the
/// import drops, and every one of these edges dies earlier on a destination
/// that does not exist. Lich's two-step model -- room to hideout, hideout to
/// destination -- is therefore not importable at all, which is a second reason
/// the connector model is the right one rather than merely a tidier one.
///
/// Kept anyway. It costs a string comparison, and if a later mapdb gives the
/// hideouts uids these become extractable-looking edges that send nothing.
fn is_urchin_hop(command: &str) -> bool {
    command.trim() == ";e true"
}

/// Silverwood Manor is off, deliberately and permanently.
///
/// `;e $SILVERWOOD_TOWN=:imt;move 'go door'` -- the global says which of four
/// towns the door leads back to, so taking the trailing `go door` alone routes
/// a character to whichever town the last person set. That alone would make it
/// unextractable.
///
/// It is worse than that: the door needs club membership, and membership is an
/// invisible character flag. Nothing a client can read says whether this
/// character has it, so there is no condition to write and no way to find out
/// but to walk into the door and see. A route planned through it is a route
/// that strands somebody.
///
/// So it is not a hard case awaiting a cleverer extractor. It is off. The same
/// class of problem as urchin access, and the same answer: do not infer travel
/// availability from something you cannot observe.
fn is_silverwood(command: &str) -> bool {
    command.contains("$SILVERWOOD_TOWN")
}

/// Classify the bookkeeping before the move.
fn classify(body: &str) -> Body {
    if body.is_empty() {
        return Body::Bare;
    }
    // The Sleeping Lady variant casts Sigil of Resolve and recovers from a
    // fall; checked first, because it also mentions `ice_mode`.
    if body.contains("Sigil of Resolve") {
        return Body::Condition("ice-slip-resolve");
    }
    if body.contains("mapdb_ice_mode") {
        return Body::Condition("ice-slip");
    }
    Body::Unrecognised(body.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Everything extractable from a mapdb, and everything deliberately skipped.
pub fn from_mapdb(json: &str) -> Result<Vec<Extracted>, Box<dyn std::error::Error>> {
    let rooms: Vec<serde_json::Value> = serde_json::from_str(json)?;

    // Same projection the importer uses: a Lich room stands for its first uid.
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

    let mut out = Vec::new();
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
            if !command.trim_start().starts_with(";e") {
                continue;
            }
            let Some(to) = target
                .parse::<i64>()
                .ok()
                .and_then(|i| uid_of.get(&i).copied())
            else {
                continue;
            };
            // Listed, never silently dropped: every script edge is superseded
            // or accounted for, and a decision is worth more in the record
            // than a gap.
            let excluded = if is_urchin_hop(command) {
                Some(
                    "the urchin network is a connector, not an edge: the command is empty because the urchin does the moving",
                )
            } else if is_silverwood(command) {
                Some(
                    "club membership is an invisible character flag; a route planned through it strands somebody",
                )
            } else {
                None
            };
            // A trailing `move` first, then the table idiom -- which has no
            // `move` in it at all, and would otherwise be skipped as
            // unrecognised forever.
            let (body, moved) = match excluded {
                Some(why) => (why, String::new()),
                None => match trailing_move(command) {
                    Some(found) => found,
                    None => match table_move(command) {
                        Some(moved) => ("", moved),
                        None => continue,
                    },
                },
            };
            let time_ms = r
                .get("timeto")
                .and_then(|t| t.get(target))
                .and_then(serde_json::Value::as_f64)
                .map(|s| (s * 1000.0).round() as i64)
                .unwrap_or(200);
            out.push(Extracted {
                from_uid: from,
                to_uid: to,
                command: moved,
                time_ms,
                body: match excluded {
                    Some(why) => Body::Excluded(why),
                    None => classify(body),
                },
            });
        }
    }
    out.sort_by(|a, b| (a.from_uid, a.to_uid, &a.command).cmp(&(b.from_uid, b.to_uid, &b.command)));
    Ok(out)
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Render the extractions as reviewable SQL.
pub fn to_sql(found: &[Extracted]) -> String {
    let (mut safe, mut skipped, mut excluded) = (Vec::new(), Vec::new(), Vec::new());
    for e in found {
        match &e.body {
            Body::Unrecognised(excerpt) => skipped.push((e, excerpt)),
            Body::Excluded(why) => excluded.push((e, *why)),
            _ => safe.push(e),
        }
    }

    let mut s = String::new();
    s.push_str(&format!(
        "-- Commands read out of the mapdb's Ruby. Generated by `codex extract`.\n\
         --\n\
         -- {} edges extracted, {} left alone.\n\
         --\n\
         -- Ending in `move 'X'` is not enough to extract one. Silverwood's door\n\
         -- reads `;e $SILVERWOOD_TOWN=:imt;move 'go door'`, and taking `go door`\n\
         -- from it discards which of four rooms the door leads back to -- and the\n\
         -- door needs club membership besides, which no client can observe. So an\n\
         -- unrecognised body means the edge is skipped and recorded below with its\n\
         -- reason, and an understood-but-unpublishable one is recorded as excluded:\n\
         -- a missing edge fails to route, a wrong one routes a character somewhere\n\
         -- they did not ask to go.\n\n",
        safe.len(),
        skipped.len()
    ));

    for e in &safe {
        s.push_str(&format!(
            "INSERT OR IGNORE INTO edge_overlays(from_uid, to_uid, command, class, time_ms, source, note) \
             VALUES ({}, {}, {}, 'walk', {}, 'script-extract', '');\n",
            e.from_uid,
            e.to_uid,
            quote(&e.command),
            e.time_ms
        ));
    }

    s.push_str("\n-- And what governs them.\n");
    for e in &safe {
        if let Body::Condition(id) = e.body {
            s.push_str(&format!(
                "INSERT OR IGNORE INTO edge_conditions(from_uid, to_uid, command, condition_id) \
                 VALUES ({}, {}, {}, '{id}');\n",
                e.from_uid,
                e.to_uid,
                quote(&e.command)
            ));
        }
    }

    if !excluded.is_empty() {
        s.push_str(&format!(
            "\n-- Understood and deliberately never published: {} edges. A decision\n\
             -- rather than a gap, written down so a later reader does not \"fix\" it.\n",
            excluded.len()
        ));
        for (e, why) in &excluded {
            s.push_str(&format!(
                "INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, \
                 disposition, reason) VALUES ({}, {}, '', 'excluded', {});\n",
                e.from_uid,
                e.to_uid,
                quote(why)
            ));
        }
    }

    s.push_str(
        "\n-- Left alone, with the reason. Every script edge is either superseded,\n\
                -- excluded or listed; there is no fourth case, and a property test\n\
                -- says so.\n",
    );
    for (e, excerpt) in &skipped {
        let reason = format!("ends in a move but the body is not modelled: {excerpt}");
        s.push_str(&format!(
            "INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, \
             disposition, reason) VALUES ({}, {}, {}, 'unhandled', {});\n",
            e.from_uid,
            e.to_uid,
            quote(&excerpt.chars().take(90).collect::<String>()),
            quote(&reason.chars().take(160).collect::<String>())
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      {"id":1,"uid":[100],"wayto":{
         "2":";e move 'west'",
         "3":";e if (UserVars.mapdb_ice_mode == 'wait') or ((Skills.survival < 50)); sleep 4; end; move 'southwest'",
         "4":";e $SILVERWOOD_TOWN=:imt;move 'go door'",
         "5":";e move 'east'; $go2_restart=true"
       },"timeto":{"2":0.2,"3":0.2,"4":0.2,"5":0.2}},
      {"id":2,"uid":[200],"wayto":{}},
      {"id":3,"uid":[300],"wayto":{}},
      {"id":4,"uid":[400],"wayto":{}},
      {"id":5,"uid":[500],"wayto":{}}
    ]"#;

    #[test]
    fn a_bare_move_extracts_with_no_condition() {
        let found = from_mapdb(SAMPLE).expect("parse");
        let e = found.iter().find(|e| e.to_uid == 200).expect("found");
        assert_eq!(e.command, "west");
        assert_eq!(e.body, Body::Bare);
    }

    #[test]
    fn an_ice_body_extracts_and_names_its_condition() {
        let found = from_mapdb(SAMPLE).expect("parse");
        let e = found.iter().find(|e| e.to_uid == 300).expect("found");
        assert_eq!(e.command, "southwest");
        assert_eq!(e.body, Body::Condition("ice-slip"));
    }

    /// Off permanently, not merely unrecognised: the door needs club
    /// membership, membership is an invisible character flag, and a route
    /// planned through something you cannot test for is a route that strands
    /// somebody.
    #[test]
    fn silverwood_is_excluded_by_name_not_by_accident() {
        assert!(is_silverwood(";e $SILVERWOOD_TOWN=:imt;move 'go door'"));
        assert!(!is_silverwood(";e move 'go door'"));
    }

    /// The case the whole discipline exists for, and it is now a decision
    /// rather than a gap. Extracting `go door` here would publish an edge
    /// that takes you to Icemule when the plan said Wehnimer's -- and even
    /// with the global understood, the door needs club membership, which no
    /// client can observe.
    #[test]
    fn silverwoods_door_is_excluded_and_said_so() {
        let found = from_mapdb(SAMPLE).expect("parse");
        let e = found.iter().find(|e| e.to_uid == 400).expect("found");
        assert!(
            matches!(e.body, Body::Excluded(_)),
            "a decision, not something awaiting a cleverer extractor"
        );

        let sql = to_sql(&found);
        assert!(
            !sql.lines()
                .any(|l| l.contains("edge_overlays") && l.contains("go door")),
            "it must never reach edge_overlays"
        );
        assert!(
            sql.contains("'excluded'") && sql.contains("invisible character flag"),
            "and the record must say why, or somebody will fix it"
        );
    }

    /// A `move` that is not the last statement is not a move we can take:
    /// something happens afterwards and we do not know what it was for.
    #[test]
    fn a_move_with_a_tail_is_not_extracted() {
        let found = from_mapdb(SAMPLE).expect("parse");
        assert!(
            !found.iter().any(|e| e.to_uid == 500),
            "$go2_restart after the move means the destination is not fixed"
        );
    }

    /// The single biggest cause of orphaned rooms, and it is an edge.
    #[test]
    fn a_table_is_a_go_command_with_a_program_around_it() {
        assert_eq!(
            table_move(
                r#";e table = "red"; fput "go #{table} table" if dothistimeout("go #{table} table", 25, /You (?:and your group )?head over to/)"#
            ),
            Some("go red table".to_string())
        );
        // Two words, and no space around the `=`, both of which occur.
        assert_eq!(
            table_move(
                r#";e table="Flying Tart"; fput "go #{table} table" if dothistimeout("go #{table} table", 25, /x/)"#
            ),
            Some("go Flying Tart table".to_string())
        );
    }

    /// `table` has to be *this* idiom, not any program that mentions a table.
    #[test]
    fn a_table_that_is_not_the_idiom_is_left_alone() {
        assert_eq!(table_move(";e fput 'look under table'; move 'west'"), None);
        assert_eq!(
            table_move(r##";e table = "#{colour}"; fput "go #{table} table""##),
            None,
            "an interpolated name is one this cannot resolve"
        );
    }

    /// Same input, same SQL — this is regenerated on every mapdb refresh and
    /// its diff has to be the real change, not a reordering.
    #[test]
    fn the_output_is_deterministic() {
        let a = to_sql(&from_mapdb(SAMPLE).expect("parse"));
        let b = to_sql(&from_mapdb(SAMPLE).expect("parse"));
        assert_eq!(a, b);
    }
}
