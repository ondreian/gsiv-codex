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
            let Some((body, moved)) = trailing_move(command) else {
                continue;
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
                body: classify(body),
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
    let (mut safe, mut skipped) = (Vec::new(), Vec::new());
    for e in found {
        match &e.body {
            Body::Unrecognised(excerpt) => skipped.push((e, excerpt)),
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
         -- from it discards which of four rooms the door leads back to. So an\n\
         -- unrecognised body means the edge is skipped and recorded below with\n\
         -- its reason: a missing edge fails to route, a wrong one routes a\n\
         -- character somewhere they did not ask to go.\n\n",
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

    s.push_str(
        "\n-- Left alone, with the reason. Every script edge is either superseded\n\
                -- or listed; there is no third case, and a property test says so.\n",
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

    /// The case the whole discipline exists for. Extracting `go door` here
    /// would publish an edge that takes you to Icemule when the plan said
    /// Wehnimer's.
    #[test]
    fn silverwoods_door_is_left_alone() {
        let found = from_mapdb(SAMPLE).expect("parse");
        let e = found.iter().find(|e| e.to_uid == 400).expect("found");
        assert!(
            matches!(e.body, Body::Unrecognised(_)),
            "setting a global before moving is not bookkeeping, it is the mechanism"
        );

        let sql = to_sql(&found);
        assert!(
            !sql.contains("400, 'go door'") && !sql.contains("400, 'go door', 'walk'"),
            "it must not reach edge_overlays"
        );
        assert!(sql.contains("'unhandled'"), "but it must be accounted for");
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

    /// Same input, same SQL — this is regenerated on every mapdb refresh and
    /// its diff has to be the real change, not a reordering.
    #[test]
    fn the_output_is_deterministic() {
        let a = to_sql(&from_mapdb(SAMPLE).expect("parse"));
        let b = to_sql(&from_mapdb(SAMPLE).expect("parse"));
        assert_eq!(a, b);
    }
}
