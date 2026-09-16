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

use std::collections::{BTreeMap, HashMap};

/// What the body before the final `move` turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    /// Nothing at all: `;e move 'west'`.
    Bare,
    /// A recognised condition, by its id in `data/040_conditions.sql`.
    Condition(&'static str),
    /// The condition picks the command: without Water Walking you swim.
    /// Carries what to send instead.
    Instead(&'static str, String),
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
    /// Things to do before sending `command`, by prelude id. Usually empty.
    pub preludes: Vec<&'static str>,
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

/// `result = fput 'down'` -- the move, with its reply kept so the body can look
/// at what happened.
///
/// The Sleeping Lady's icy slopes are written this way and nothing else in the
/// mapdb is. `classify` has recognised the body as `ice-slip-resolve` since the
/// ice work -- Sigil of Resolve, `mapdb_ice_mode`, Survival under 50, the
/// six-second pause -- but no recogniser found a *command* in it, so `no_move`
/// forced the whole thing to `Unrecognised` and three edges were dropped with a
/// condition already written for them.
///
/// Three edges, and they were the only walking road between Pinefar and the
/// rest of Elanthia. Without them the Trading Post, Mount Aenatumgana and the
/// Cavern of Ages -- 288 rooms, and the whole slow road to the Rift -- hung off
/// a 3,000 silver caravan fare.
///
/// **The assignment is the tell.** A bare `fput 'stand'` is housekeeping and
/// there are hundreds of those; capturing the reply and testing it means the
/// body cares what the game said, and in this family what it says is whether
/// you slipped. So this wants exactly one captured `fput` and takes that as the
/// move.
///
/// The `$go2_restart = true` after it is not refused here, and that is not a
/// loosening of the rule below. There it means "the destination is not fixed",
/// a fact about the road. Here it sits inside `if result =~ /^Rushing
/// heedlessly/` and means "you fell over, start again" -- which is a failure
/// class the vocabulary already carries and the walker already answers by
/// standing up and re-planning.
fn captured_fput(command: &str) -> Option<String> {
    let mut found = None;
    for raw in command.split(['\n', ';']) {
        let st = raw.trim();
        let Some((lhs, rhs)) = st.split_once('=') else {
            continue;
        };
        // `result = fput 'down'`, and nothing cleverer: a plain identifier on
        // the left, a quoted literal on the right.
        if lhs.trim().is_empty()
            || !lhs
                .trim()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }
        let Some(arg) = rhs.trim().strip_prefix("fput ") else {
            continue;
        };
        let arg = arg.trim();
        let Some(inner) = arg
            .strip_prefix('\'')
            .and_then(|i| i.split_once('\''))
            .map(|(c, _)| c)
        else {
            continue;
        };
        // Two captured sends are two decisions and this reads neither.
        if found.replace(inner.to_string()).is_some() {
            return None;
        }
    }
    found
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

/// `if checkspell(112) then move 'west' else move 'swim west' end`
///
/// Water Walking. With it you walk; without it you swim, and the crossing is
/// neither slower nor forbidden -- the condition picks the *command*. Returns
/// `(walking, swimming)`.
///
/// Both spellings the mapdb uses: `checkspell(112)` and `Spell[112].active?`.
fn water_walk(command: &str) -> Option<(String, String)> {
    let c = command.trim().strip_prefix(";e")?.trim();
    if !(c.contains("checkspell(112)") || c.contains("Spell[112]")) {
        return None;
    }
    // Two moves, in order: the one with the spell, then the one without.
    let mut moves = Vec::new();
    let mut rest = c;
    while let Some(at) = rest.find("move ") {
        let after = &rest[at + 5..];
        let mut taken = None;
        for q in ['\'', '"'] {
            if let Some(inner) = after.strip_prefix(q)
                && let Some(end) = inner.find(q)
            {
                taken = Some((inner[..end].to_string(), end + 1));
            }
        }
        let (m, used) = taken?;
        moves.push(m);
        rest = &after[used..];
    }
    let [walking, swimming] = moves.as_slice() else {
        return None;
    };
    // The order is the whole meaning: the `then` branch is the one that has
    // the spell. A body whose second move is not a swim is some other
    // conditional and this cannot read it.
    if !swimming.starts_with("swim ") {
        return None;
    }
    Some((walking.clone(), swimming.clone()))
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

/// A `;e` body read as a list of statements, when every one of them is
/// something this understands.
///
/// Generalises three cases the extractor was failing separately:
///
/// ```text
///   ;e move 'west'; waitrt?                        a benign tail
///   ;e fput 'kneel'; move 'go burrow'; fput 'stand' a prelude and a postlude
///   ;e multifput 'search', 'go alleyway'            a prelude, and no `move`
/// ```
///
/// All three are one move with known housekeeping around it, and the
/// housekeeping is either something the client already does or something the
/// data can name. Anything with a statement this does not recognise returns
/// `None` and stays unhandled -- the bar is that every statement is
/// understood, not that one of them is.
///
/// # `fput 'stand'` afterwards is dropped on purpose
///
/// The kneel edges stand you back up, and this does not record that. It does
/// not need to: leaving a character kneeling makes the *next* move answer
/// "You will have to stand up first", which is `must-be-standing` in the
/// failure vocabulary, whose remedy is to stand and send it again. A postlude
/// that repairs a state the recovery already repairs is a second mechanism
/// for one problem.
fn statements(body: &str) -> Option<(Vec<&'static str>, String)> {
    let body = body.trim().strip_prefix(";e")?.trim();
    let mut preludes = Vec::new();
    let mut moved: Option<String> = None;
    // `direction="west"` and friends. The mapdb assigns a literal and then
    // interpolates it straight back into the command; keeping the binding lets
    // the command be read as what it is.
    let mut bound: Vec<(String, String)> = Vec::new();

    // Ruby ends a statement with a semicolon *or* a newline, and the mapdb uses
    // both -- often in the same file. Splitting on `;` alone made
    //
    //     ;e fput 'search'
    //     move 'go snowy path'
    //
    // a single unreadable statement, so the whole body fell through to
    // `trailing_move`, which found the move but handed back `fput 'search'` as a
    // body nothing could classify. The Northern Slopes is what that cost: the
    // trail between Pinefar and Icemule is written this way, and without it
    // Pinefar is an island of 161 rooms whose only link to the rest of Elanthia
    // is a caravan fare.
    //
    // A newline inside a quoted regex splits wrongly, which is harmless: the
    // halves are unreadable, `statements` returns None, and the body falls
    // through exactly as it does today.
    for raw in body.split([';', '\n']) {
        let st = raw.trim().trim_end_matches(';').trim();
        if st.is_empty() {
            continue;
        }
        // `unless (move 'go door')` -- the move, with "and if that failed"
        // hanging off it. What follows is the waiting, which is the client's
        // business.
        let st = st.strip_prefix("unless ").unwrap_or(st);
        let st = st
            .strip_prefix('(')
            .and_then(|inner| inner.strip_suffix(')'))
            .unwrap_or(st);
        // `move 'south' while Room.current.id == 2925` -- send it again while
        // you are still standing where you started.
        //
        // The same idea as `start=Room.current.id` below and dropped for the
        // same reason: the walker already retries by room id, and does it
        // better, because it checks the id it was actually given rather than
        // one pasted into the map. On the Northward Trail's snow plains a
        // single `south` does not always move you, and this is the mapdb
        // saying so.
        //
        // Only this exact guard, and only on a `move`. Any other trailing
        // condition changes what happens and the body stays unhandled.
        let st = match st.split_once(" while Room.current.id") {
            Some((head, _)) if head.trim_start().starts_with("move ") => head.trim_end(),
            _ => st,
        };
        // The move itself, single- or double-quoted.
        if let Some(found) = quoted_after(st, "move") {
            // Two *different* moves is not one edge. The same move twice is:
            //
            //   ;e unless (move 'go door'); echo 'Waiting for the door...';
            //      waitfor 'You hear a soft click'; move 'go door'
            //
            // which is one road, tried, waited on, and tried again. The
            // waiting is the client's business -- and the failure vocabulary
            // already knows `way-is-closed`.
            if let Some(first) = moved.replace(found.clone())
                && first != found
            {
                return None;
            }
            continue;
        }
        // `multifput 'search', 'go alleyway'` -- a prelude and a move, in one
        // statement and with no `move` keyword anywhere.
        if let Some(rest) = st.strip_prefix("multifput ") {
            let parts: Vec<&str> = rest.split(',').map(str::trim).collect();
            let [first, second] = parts.as_slice() else {
                return None;
            };
            // `search` or `search wall` -- the same prelude either way. The
            // noun says where to look and the walker looks there; what matters
            // to the route is that the exit is not visible until you do.
            if !is_search(unquote(first)?) {
                return None;
            }
            preludes.push("search-for-the-exit");
            if moved.replace(unquote(second)?.to_string()).is_some() {
                return None;
            }
            continue;
        }
        if let Some((found, guard)) = fput_call(st) {
            match found.as_str() {
                // A trailing `unless kneeling? or (Stats.race =~ /Dwarf|
                // Halfling|Gnome/)` is allowed here and nowhere else: it is
                // the prelude's own condition, already written down as
                // `needs-to-kneel`, and re-reading it from the Ruby would be
                // two sources for one rule. On any other command a guard
                // changes what happens and the body stays unhandled.
                "kneel" => preludes.push("kneel-to-fit"),
                other if guard.is_empty() && is_search(other) => {
                    preludes.push("search-for-the-exit")
                }
                // Housekeeping the walker does anyway.
                "stand" if guard.is_empty() => {}
                // `fput 'go narrow door'` is a move, said the long way. Only
                // when it *looks* like one: `fput` sends any command at all,
                // and `fput 'pull lever'` is a lever, not a road. Guessing
                // there would publish an edge whose command moves nobody.
                other if guard.is_empty() && looks_like_movement(other) => {
                    if moved.replace(other.to_string()).is_some() {
                        return None;
                    }
                }
                _ => return None,
            }
            continue;
        }
        // `direction="west"` -- a literal bound for one interpolation.
        if let Some((name, value)) = binding(st) {
            bound.push((name, value));
            continue;
        }
        // `start=Room.current.id` is the loop's own bookkeeping: it remembers
        // where you were so it can retry until you are somewhere else. The
        // walker retries by checking the room id, which is the same idea.
        if st.ends_with("=Room.current.id") || st.ends_with("= Room.current.id") {
            continue;
        }
        // `dothistimeout "pedal #{direction}", 2, /pedal/ while ...` -- the
        // move, wearing a retry loop. 753 of the harbour's edges are this, and
        // `pedal` is a movement verb exactly as `go` and `climb` are.
        if let Some(cmd) = dothistimeout_command(st, &bound) {
            if moved.replace(cmd).is_some() {
                return None;
            }
            continue;
        }
        // `UserVars.mapdb_talondown_origin = nil` -- Lich clearing a
        // breadcrumb it left itself. The move still happens and the command is
        // still the command, so the assignment is read past rather than
        // refused.
        //
        // **Reading past it is not the same as it meaning nothing.** That
        // breadcrumb is how Lich knows which of Talondown's nine exits to take,
        // and taking this rule as proof the edge is ordinary published nine
        // wormholes out of one arena. What stops that is the fan-out check in
        // `to_sql`, which refuses any command naming more than one destination
        // no matter which recogniser read it -- because the tell is the shape
        // of the result, not the shape of the Ruby.
        //
        // Deliberately *not* the same as `$go2_restart = true`, which is
        // refused below and must stay refused: that one says the destination
        // is not fixed, which is a fact about the road rather than
        // housekeeping.
        if st.starts_with("UserVars.") && st.contains('=') {
            continue;
        }
        // Waiting is what the walker does before every send regardless.
        if st == "waitrt?"
            || st == "true"
            || st.starts_with("sleep ")
            || st.starts_with("pause ")
            // Waiting for the game to say something, and Lich talking to its
            // own console. Both are the client's business.
            || st.starts_with("waitfor ")
            || st.starts_with("echo ")
        {
            continue;
        }
        return None;
    }
    moved.map(|m| (preludes, m))
}

/// `search`, or `search` at something.
///
/// The Pool's crevice is only there once you `search wall`, which is the same
/// fact about the room as a bare `search` -- the exit is not visible until you
/// look. Taking only the bare form left that edge unpublished and the Cavern of
/// Ages a step further away.
fn is_search(command: &str) -> bool {
    let c = command.trim();
    c == "search" || c.starts_with("search ")
}

/// Is this command a way out of a room, rather than something you do in one?
///
/// The published vocabulary of movement: the two verbs the mapdb uses for
/// exits, the two the harbour and the swamp use, and the bare directions.
/// Anything else `fput` sends is an action -- pulling a lever, opening a
/// chest -- and an edge whose command is one of those moves nobody.
fn looks_like_movement(command: &str) -> bool {
    const VERBS: &[&str] = &["go ", "climb ", "swim ", "pedal ", "push ", "jump "];
    const DIRECTIONS: &[&str] = &[
        "north",
        "south",
        "east",
        "west",
        "northeast",
        "northwest",
        "southeast",
        "southwest",
        "up",
        "down",
        "out",
        "in",
    ];
    let c = command.trim();
    VERBS.iter().any(|v| c.starts_with(v)) || DIRECTIONS.contains(&c)
}

/// `direction="west"` -> `("direction", "west")`.
fn binding(statement: &str) -> Option<(String, String)> {
    let (name, rest) = statement.split_once('=')?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some((name.to_string(), unquote(rest)?.to_string()))
}

/// The command out of `dothistimeout "...", 2, /re/ while ...`.
///
/// The timeout and the pattern are error handling and the `while` is a retry;
/// both are the client's job. What is left is one command, once any `#{var}`
/// in it is replaced by the literal the body bound.
fn dothistimeout_command(statement: &str, bound: &[(String, String)]) -> Option<String> {
    let rest = statement.strip_prefix("dothistimeout")?.trim_start();
    let inner = {
        let mut found = None;
        for q in ['\'', '"'] {
            if let Some(after) = rest.strip_prefix(q)
                && let Some(end) = after.find(q)
            {
                found = Some(after[..end].to_string());
                break;
            }
        }
        found?
    };
    let mut command = inner;
    for (name, value) in bound {
        command = command.replace(&format!("#{{{name}}}"), value);
    }
    // An interpolation this body never bound is one nothing here can resolve.
    if command.contains("#{") || command.trim().is_empty() {
        return None;
    }
    Some(command)
}

/// `fput 'x'`, and whatever guard trails it.
///
/// Returns `(command, guard)` where the guard is whatever followed the closing
/// quote -- empty when the statement is nothing but the call. Only the caller
/// knows whether a guard is tolerable, and for almost every command it is not.
fn fput_call(statement: &str) -> Option<(String, String)> {
    let rest = statement.strip_prefix("fput ")?;
    let rest = rest.trim();
    for q in ['\'', '"'] {
        if let Some(inner) = rest.strip_prefix(q)
            && let Some(end) = inner.find(q)
        {
            return Some((
                inner[..end].to_string(),
                inner[end + 1..].trim().to_string(),
            ));
        }
    }
    None
}

/// The contents of the first quoted string after `keyword`, when the statement
/// is nothing but that call.
fn quoted_after(statement: &str, keyword: &str) -> Option<String> {
    let rest = statement.strip_prefix(keyword)?;
    // `move'x'` never occurs; requiring the space stops `movement` matching.
    let rest = rest.strip_prefix(' ').or_else(|| rest.strip_prefix('('))?;
    let inner = unquote(rest.trim().trim_end_matches(')').trim())?;
    Some(inner.to_string())
}

fn unquote(s: &str) -> Option<&str> {
    let s = s.trim();
    for q in ['\'', '"'] {
        if let Some(inner) = s.strip_prefix(q)
            && let Some(end) = inner.find(q)
        {
            // Trailing text means this statement does more than the call.
            if inner[end + 1..].trim().is_empty() {
                return Some(&inner[..end]);
            }
        }
    }
    None
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
            let mut preludes = Vec::new();
            let mut swim_instead: Option<String> = None;
            let (body, moved) = match excluded {
                Some(why) => (why, String::new()),
                // `statements` first, because it is the stricter test: it
                // demands every statement be understood, and it is the only
                // one that yields preludes. `trailing_move` merely wants a
                // `move` at the end, so it would win on
                // `fput 'kneel' unless ...; move 'southeast'` and hand back a
                // body `classify` cannot read -- extracting nothing and
                // tagging nothing, which is what it did.
                // Checked before the rest: the body has two moves in it, and
                // any recogniser that takes the first would publish the
                // walking command for a character who has to swim.
                None if water_walk(command).is_some() => {
                    let Some((walking, swimming)) = water_walk(command) else {
                        continue;
                    };
                    swim_instead = Some(swimming);
                    ("", walking)
                }
                None => match statements(command) {
                    Some((found, moved)) => {
                        preludes = found;
                        ("", moved)
                    }
                    // Bodies with real logic in them: the ice conditions,
                    // which `classify` reads and `statements` rightly refuses.
                    None => match trailing_move(command) {
                        Some(found) => found,
                        None => match table_move(command) {
                            Some(moved) => ("", moved),
                            // The ice bodies, whose move is a captured `fput`.
                            // The body goes on to `classify`, which has known
                            // this family all along and only ever lacked a
                            // command to hang it on.
                            None if captured_fput(command).is_some() => {
                                match captured_fput(command) {
                                    Some(moved) => (command, moved),
                                    None => (command, String::new()),
                                }
                            }
                            // Nothing recognised it. Handing the whole body to
                            // `classify` makes it `Unrecognised`, which earns a
                            // disposition row -- where `continue` here earned
                            // nothing, and 5,158 script edges left no trace of
                            // having been looked at. The header above this loop
                            // claims every script edge is superseded, excluded
                            // or listed; this is what makes that true.
                            None => (command, String::new()),
                        },
                    },
                },
            };
            let time_ms = r
                .get("timeto")
                .and_then(|t| t.get(target))
                .and_then(serde_json::Value::as_f64)
                .map(|s| (s * 1000.0).round() as i64)
                .unwrap_or(200);
            // Read before `moved` is handed to the struct, because the body
            // below depends on whether a command came out of the parse.
            let no_move = moved.is_empty();
            out.push(Extracted {
                from_uid: from,
                to_uid: to,
                command: moved,
                time_ms,
                body: match (excluded, swim_instead) {
                    (Some(why), _) => Body::Excluded(why),
                    (None, Some(swim)) => Body::Instead("no-water-walking", swim),
                    // No move came out of it, so there is nothing to hang a
                    // condition on. `classify` would still recognise the ice
                    // body and return a condition for an edge whose command is
                    // empty -- which is a foreign key failure at build time,
                    // and would be a road nobody can walk if it were not.
                    (None, None) if no_move => {
                        Body::Unrecognised(body.split_whitespace().collect::<Vec<_>>().join(" "))
                    }
                    (None, None) => classify(body),
                },
                preludes,
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
    let (mut safe, mut skipped, mut excluded): (
        Vec<&Extracted>,
        Vec<_>,
        Vec<(&Extracted, String)>,
    ) = (Vec::new(), Vec::new(), Vec::new());
    for e in found {
        match &e.body {
            Body::Unrecognised(excerpt) => skipped.push((e, excerpt)),
            Body::Excluded(why) => excluded.push((e, (*why).to_string())),
            Body::Instead(..) => safe.push(e),
            _ => safe.push(e),
        }
    }

    // One command, several destinations: the map is describing a mechanism and
    // not an exit, and none of the destinations is publishable.
    //
    // `mapdb::load` already refuses these, but it looks at the raw wayto and
    // skips anything scripted -- so a command that *becomes* ambiguous once the
    // Ruby around it is read has never been checked by anything. Talondown
    // Arena is what that costs. Nine rooms across four towns reach it, and
    // every one of them writes down where you came from:
    //
    //   ;e move('go doorframe');UserVars.mapdb_talondown_origin = 417;
    //   ;e move('go exit passage');UserVars.mapdb_talondown_origin = nil;
    //
    // The arena's exit passage returns you to whichever of the nine you came in
    // by, which Lich knows because it left itself the breadcrumb. Read as nine
    // separate roads it is a wormhole from the Landing to Icemule, Solhaven,
    // Ta'Vaalor and back -- and a character sent through it walks in, walks
    // straight back out where it started, and does it again. That is journey
    // history, the same thing `caravan:hinterwilds` publishes two roads on, and
    // `condition_terms` cannot express it.
    //
    // So the rule belongs here, downstream of every recogniser, rather than
    // beside the one that let this through. Whatever the next extractor rule
    // learns to read, it cannot turn one command into several roads.
    let mut fan: HashMap<(i64, &str), usize> = HashMap::new();
    for e in &safe {
        *fan.entry((e.from_uid, e.command.as_str())).or_default() += 1;
    }
    let (kept, wormholes): (Vec<&Extracted>, Vec<&Extracted>) = safe.into_iter().partition(|e| {
        fan.get(&(e.from_uid, e.command.as_str()))
            .copied()
            .unwrap_or(0)
            <= 1
    });
    for e in wormholes {
        let n = fan
            .get(&(e.from_uid, e.command.as_str()))
            .copied()
            .unwrap_or(0);
        excluded.push((
            e,
            format!(
                "one command, {n} destinations from room {}: {:?} is a mechanism rather than an \
                 exit, and which room it reaches depends on something the map does not record",
                e.from_uid, e.command
            ),
        ));
    }
    let safe = kept;

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

    s.push_str(
        "\n-- Things to do before the move. `kneel-to-fit` and\n\
         -- `search-for-the-exit` were defined and tagged on nothing at all\n\
         -- until the extractor learned to read a body as a list of\n\
         -- statements rather than as a trailing `move`.\n",
    );
    for e in &safe {
        for prelude in &e.preludes {
            s.push_str(&format!(
                "INSERT OR IGNORE INTO edge_preludes(from_uid, to_uid, command, prelude_id) \
                 VALUES ({}, {}, {}, '{prelude}');\n",
                e.from_uid,
                e.to_uid,
                quote(&e.command)
            ));
        }
    }

    s.push_str(
        "\n-- And where a condition picks the command rather than the cost:\n\
         -- without Water Walking you swim across, and the crossing is neither\n\
         -- slower nor forbidden.\n",
    );
    for e in &safe {
        if let Body::Instead(condition, swim) = &e.body {
            s.push_str(&format!(
                "INSERT OR IGNORE INTO edge_conditions(from_uid, to_uid, command, \
                 condition_id, instead) VALUES ({}, {}, {}, '{condition}', {});\n",
                e.from_uid,
                e.to_uid,
                quote(&e.command),
                quote(swim)
            ));
        }
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
        // Two shapes reach here and the reason has to tell them apart: a body
        // that ends in a move this cannot read, and a body no recogniser
        // matched at all. The second has no command to publish.
        let reason = if e.command.is_empty() {
            format!("no recogniser matched: {excerpt}")
        } else {
            format!("ends in a move but the body is not modelled: {excerpt}")
        };
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
    ///
    /// Listed rather than dropped. It used to be neither — a bare `continue`
    /// that left no command and no record, which is how 5,158 script edges
    /// came to have been looked at and leave no trace of it.
    #[test]
    fn a_move_with_a_tail_is_listed_but_carries_no_command() {
        let found = from_mapdb(SAMPLE).expect("parse");
        let e = found
            .iter()
            .find(|e| e.to_uid == 500)
            .expect("looked at, so recorded");
        assert!(
            e.command.is_empty(),
            "$go2_restart after the move means the destination is not fixed"
        );
        assert!(
            matches!(e.body, Body::Unrecognised(_)),
            "and it must say so, or somebody will assume nobody looked"
        );
    }

    /// The three shapes one statement-list parser replaced.
    #[test]
    fn a_body_of_understood_statements_is_a_move_with_housekeeping() {
        // A benign tail: the walker waits out roundtime before every send.
        assert_eq!(
            statements(";e move 'west'; waitrt?"),
            Some((vec![], "west".to_string()))
        );
        // A prelude and a postlude. The postlude is dropped: leaving a
        // character kneeling makes the next move say "You will have to stand
        // up first", which the failure vocabulary already recovers from.
        assert_eq!(
            statements(";e fput 'kneel'; move 'go burrow'; fput 'stand'"),
            Some((vec!["kneel-to-fit"], "go burrow".to_string()))
        );
        // A prelude and a move in one statement, with no `move` keyword.
        assert_eq!(
            statements(";e multifput 'search', 'go alleyway'"),
            Some((vec!["search-for-the-exit"], "go alleyway".to_string()))
        );
        // Double quotes, which the old recogniser never handled.
        assert_eq!(
            statements(r#";e fput "search";move "go trapdoor""#),
            Some((vec!["search-for-the-exit"], "go trapdoor".to_string()))
        );
    }

    /// The condition picks the command, not the cost. With Water Walking you
    /// walk; without it you swim, and the crossing is neither slower nor
    /// forbidden.
    #[test]
    fn water_walking_chooses_between_two_commands() {
        assert_eq!(
            water_walk(";e if checkspell(112) then move 'west' else move 'swim west' end; waitrt?"),
            Some(("west".to_string(), "swim west".to_string()))
        );
        assert_eq!(
            water_walk(";e if Spell[112].active?; move 'east';else; move 'swim east';end"),
            Some(("east".to_string(), "swim east".to_string()))
        );
    }

    /// The order carries the meaning, and a second branch that is not a swim
    /// is some other conditional this cannot read.
    #[test]
    fn a_conditional_that_is_not_the_swim_pair_is_refused() {
        assert_eq!(
            water_walk(";e if checkspell(112) then move 'west' end"),
            None
        );
        assert_eq!(
            water_walk(";e if checkspell(112) then move 'west' else move 'north' end"),
            None,
            "two plain moves is a choice this does not understand"
        );
        assert_eq!(water_walk(";e move 'west'"), None);
    }

    /// `fput` sends any command. Only the ones that are ways out of a room
    /// count as the move.
    #[test]
    fn an_fput_is_a_move_only_when_it_looks_like_one() {
        assert_eq!(
            statements(r#";e fput "search";fput "go narrow door""#),
            Some((vec!["search-for-the-exit"], "go narrow door".to_string()))
        );
        assert_eq!(
            statements(";e fput 'pull lever'; move 'north'"),
            None,
            "a lever is not a road, and an edge that pulls one moves nobody"
        );
    }

    /// 753 of the harbour's edges, and `pedal` is a movement verb exactly as
    /// `go` and `climb` are.
    #[test]
    fn a_bound_literal_is_interpolated_back_into_the_command() {
        assert_eq!(
            statements(
                r#";e direction="west";start=Room.current.id; dothistimeout "pedal #{direction}", 2, /pedal/ while Room.current.id == start"#
            ),
            Some((vec![], "pedal west".to_string()))
        );
        // A literal command needs no binding at all.
        assert_eq!(
            statements(";e dothistimeout 'push southeast',5,/you push/i;waitrt?"),
            Some((vec![], "push southeast".to_string()))
        );
    }

    /// An interpolation nothing bound is one this cannot resolve, and guessing
    /// would publish a command with a `#{` in it.
    #[test]
    fn an_unbound_interpolation_is_refused() {
        assert_eq!(
            statements(r#";e dothistimeout "pedal #{direction}", 2, /pedal/"#),
            None
        );
    }

    /// The guard on `kneel` is the prelude's own condition, already written
    /// down as `needs-to-kneel`. Re-reading it from the Ruby would be two
    /// sources for one rule.
    #[test]
    fn a_kneel_may_carry_its_guard_and_nothing_else_may() {
        assert_eq!(
            statements(
                ";e fput 'kneel' unless kneeling? or (Stats.race =~ /Dwarf|Halfling|Gnome/); move 'southeast'"
            ),
            Some((vec!["kneel-to-fit"], "southeast".to_string()))
        );
        assert_eq!(
            statements(";e fput 'stand' unless standing?; move 'north'"),
            None,
            "a guard on anything else changes what happens"
        );
    }

    /// The bar is that EVERY statement is understood, not that one is. One
    /// unrecognised call and the edge stays unhandled, because the thing it
    /// does may be the mechanism.
    #[test]
    fn one_unknown_statement_rejects_the_whole_body() {
        assert_eq!(statements(";e fput 'pull lever'; move 'north'"), None);
        assert_eq!(statements(";e move 'north'; $go2_restart=true"), None);
        assert_eq!(
            statements(";e move 'north'; move 'south'"),
            None,
            "two different moves is not one edge"
        );
        assert_eq!(
            statements(
                ";e unless (move 'go door'); waitfor 'You hear a soft click'; move 'go door'"
            ),
            Some((vec![], "go door".to_string())),
            "the same move twice is one road, tried and tried again"
        );
        assert_eq!(statements(";e waitrt?"), None, "housekeeping and no move");
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
