//! Conditions, and evaluating one against a character.
//!
//! # Why a snapshot
//!
//! Every input here moves at runtime. Enhancives swap and Survival shifts fifty
//! ranks. A spell lapses. Picking up loot crosses an encumbrance threshold. An
//! injury arrives mid-route, and trying to climb with one is how a character
//! dies.
//!
//! So a condition is never evaluated *by the data* — it is evaluated against a
//! [`CharacterSnapshot`], which the caller supplies. That gives three things at
//! once:
//!
//! - **planning is deterministic**: the same snapshot yields the same plan,
//!   which is what makes a route testable at all;
//! - **hypotheticals work**: plan as a character who *would* have 60 Survival,
//!   and see whether the route changes;
//! - **real-time planning is honest**: urnon snapshots live `GameState` and
//!   plans against what is true now, rather than what was true at import.
//!
//! Lich has none of this. It cannot express the conditions, so every hard case
//! became `;e` — arbitrary Ruby run at traversal time, evaluated by luck. That
//! is why climbing while injured is a death it never warns about: there is
//! nowhere in the format to say so.
//!
//! **Evaluation belongs in the router, not in a pre-filter.** A connector can be
//! screened before planning, because "can this character use it at all" is one
//! question. An edge cannot: its condition may only add four seconds, and the
//! router has to weigh that against an alternative route. Lich's own dijkstra
//! calls the closure during relaxation for exactly this reason.

use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet};

/// What a term asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    Skill,
    Stat,
    Society,
    SocietyRank,
    Encumbrance,
    Spell,
    Item,
    Injury,
    Preference,
}

impl Subject {
    fn parse(s: &str) -> Option<Subject> {
        Some(match s {
            "skill" => Subject::Skill,
            "stat" => Subject::Stat,
            "society" => Subject::Society,
            "society_rank" => Subject::SocietyRank,
            "encumbrance" => Subject::Encumbrance,
            "spell" => Subject::Spell,
            "item" => Subject::Item,
            "injury" => Subject::Injury,
            "preference" => Subject::Preference,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Ne,
    Present,
    Absent,
}

impl Op {
    fn parse(s: &str) -> Option<Op> {
        Some(match s {
            "lt" => Op::Lt,
            "lte" => Op::Lte,
            "gt" => Op::Gt,
            "gte" => Op::Gte,
            "eq" => Op::Eq,
            "ne" => Op::Ne,
            "present" => Op::Present,
            "absent" => Op::Absent,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    pub subject: Subject,
    pub key: String,
    pub op: Op,
    pub value: String,
}

/// A named condition in disjunctive normal form: groups are ORed, and the
/// terms within a group are ANDed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Condition {
    pub id: String,
    pub groups: Vec<Vec<Term>>,
    pub delay_ms: i64,
    pub add_cost_ms: i64,
    pub forbid: bool,
}

/// Everything a condition can ask about a character, at one moment.
///
/// Deliberately plain data. It is built by whoever has the live state, passed
/// by value into planning, and never reaches back for more — so a plan cannot
/// silently depend on something that changed halfway through computing it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterSnapshot {
    /// `survival` → ranks. Enhancives are already counted: this is what the
    /// character has *now*, not what their training says.
    pub skills: BTreeMap<String, i64>,
    /// `prof` → `Ranger`, `race` → `Halfling`.
    pub stats: BTreeMap<String, String>,
    pub society: Option<String>,
    pub society_rank: i64,
    pub encumbrance: i64,
    /// Spells active right now, by the name or number the condition uses.
    pub spells: BTreeSet<String>,
    /// Items the character has — worn, held or stowed alike. A condition says
    /// *what*, never *where*.
    pub items: BTreeSet<String>,
    /// Injuries by location: `left_arm` → severity. Absent means unhurt.
    pub injuries: BTreeMap<String, i64>,
    /// Settings the player chose, e.g. `ice_mode` → `run`.
    pub preferences: BTreeMap<String, String>,
}

/// What a condition does to a traversal when it holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Time spent standing still before the command, on purpose.
    pub delay_ms: i64,
    /// Time the traversal itself costs beyond its base.
    pub add_cost_ms: i64,
    /// The traversal is not available to this character. Lich writes this as a
    /// `nil` cost; naming it keeps the meaning visible.
    pub forbidden: bool,
}

impl Outcome {
    /// Everything a plan should add for one traversal.
    pub fn total_ms(&self) -> i64 {
        self.delay_ms + self.add_cost_ms
    }
}

/// Does the condition hold for this character?
pub fn holds(condition: &Condition, who: &CharacterSnapshot) -> bool {
    // No groups is no condition. A condition row with no terms holds for
    // nobody rather than everybody: an empty rule is unfinished curation, and
    // silently applying it to every character is the worse failure.
    condition
        .groups
        .iter()
        .any(|group| group.iter().all(|t| term_holds(t, who)))
}

/// What this condition does to a traversal, for this character.
pub fn evaluate(condition: &Condition, who: &CharacterSnapshot) -> Outcome {
    if !holds(condition, who) {
        return Outcome::default();
    }
    Outcome {
        delay_ms: condition.delay_ms,
        add_cost_ms: condition.add_cost_ms,
        forbidden: condition.forbid,
    }
}

fn term_holds(t: &Term, who: &CharacterSnapshot) -> bool {
    match t.subject {
        Subject::Skill => cmp_num(who.skills.get(&t.key).copied().unwrap_or(0), t),
        Subject::Encumbrance => cmp_num(who.encumbrance, t),
        Subject::SocietyRank => cmp_num(who.society_rank, t),
        Subject::Injury => cmp_num(who.injuries.get(&t.key).copied().unwrap_or(0), t),
        Subject::Stat => cmp_str(who.stats.get(&t.key).map(String::as_str), t),
        Subject::Society => cmp_str(who.society.as_deref(), t),
        Subject::Preference => cmp_str(who.preferences.get(&t.key).map(String::as_str), t),
        Subject::Spell => cmp_member(who.spells.contains(&t.key), t),
        Subject::Item => cmp_member(who.items.contains(&t.key), t),
    }
}

fn cmp_num(have: i64, t: &Term) -> bool {
    let want: i64 = t.value.parse().unwrap_or(0);
    match t.op {
        Op::Lt => have < want,
        Op::Lte => have <= want,
        Op::Gt => have > want,
        Op::Gte => have >= want,
        Op::Eq => have == want,
        Op::Ne => have != want,
        // An injury or a skill is "present" when it is anything but zero.
        Op::Present => have != 0,
        Op::Absent => have == 0,
    }
}

fn cmp_str(have: Option<&str>, t: &Term) -> bool {
    match t.op {
        Op::Eq => have == Some(t.value.as_str()),
        Op::Ne => have != Some(t.value.as_str()),
        Op::Present => have.is_some_and(|h| !h.is_empty()),
        Op::Absent => have.is_none_or(str::is_empty),
        // Ordering a profession is meaningless, and guessing an answer is
        // worse than refusing one.
        _ => false,
    }
}

fn cmp_member(have: bool, t: &Term) -> bool {
    match t.op {
        Op::Present | Op::Eq => have,
        Op::Absent | Op::Ne => !have,
        _ => false,
    }
}

/// Every condition, by id.
pub fn load_all(conn: &Connection) -> rusqlite::Result<BTreeMap<String, Condition>> {
    let mut out: BTreeMap<String, Condition> = BTreeMap::new();
    {
        let mut stmt = conn.prepare("SELECT id FROM conditions ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for id in rows {
            let id = id?;
            out.insert(id.clone(), Condition { id, ..Condition::default() });
        }
    }
    {
        let mut stmt = conn.prepare(
            "SELECT condition_id, grp, subject, key, op, value
               FROM condition_terms ORDER BY condition_id, grp, seq",
        )?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let grp: usize = row.get::<_, i64>(1)?.max(0) as usize;
            let (Some(subject), Some(op)) = (
                Subject::parse(&row.get::<_, String>(2)?),
                Op::parse(&row.get::<_, String>(4)?),
            ) else {
                // A term this build does not understand. Skipped rather than
                // guessed at, and the CHECK constraints make it unreachable
                // from a database this build wrote.
                continue;
            };
            if let Some(c) = out.get_mut(&id) {
                while c.groups.len() <= grp {
                    c.groups.push(Vec::new());
                }
                c.groups[grp].push(Term {
                    subject,
                    key: row.get(3)?,
                    op,
                    value: row.get(5)?,
                });
            }
        }
    }
    {
        let mut stmt = conn.prepare("SELECT condition_id, effect, amount FROM condition_effects")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let effect: String = row.get(1)?;
            let amount: i64 = row.get(2)?;
            if let Some(c) = out.get_mut(&id) {
                match effect.as_str() {
                    "delay_ms" => c.delay_ms = amount,
                    "add_cost_ms" => c.add_cost_ms = amount,
                    "forbid" => c.forbid = true,
                    _ => {}
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::open_in_memory;

    /// The ice fields, as they actually read, and the FWI trinket.
    fn fixture() -> Connection {
        let conn = open_in_memory().expect("open");
        conn.execute_batch(
            "
            INSERT INTO conditions(id, description) VALUES
              ('ice-slip', 'pause before crossing ice or you slip'),
              ('no-climb-injured', 'climbing with an injured limb kills'),
              ('has-fwi-trinket', 'the character has a trinket');

            -- ice_mode = wait
            --   OR (ice_mode != run AND encumbrance > 50)
            --   OR (ice_mode != run AND survival < 50 AND Haste inactive)
            INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
              ('ice-slip', 0, 0, 'preference', 'ice_mode', 'eq', 'wait'),

              ('ice-slip', 1, 0, 'preference', 'ice_mode', 'ne', 'run'),
              ('ice-slip', 1, 1, 'encumbrance', '', 'gt', '50'),

              ('ice-slip', 2, 0, 'preference', 'ice_mode', 'ne', 'run'),
              ('ice-slip', 2, 1, 'skill', 'survival', 'lt', '50'),
              ('ice-slip', 2, 2, 'spell', 'Haste', 'absent', ''),

              ('no-climb-injured', 0, 0, 'injury', 'left_arm', 'gte', '2'),
              ('no-climb-injured', 1, 0, 'injury', 'right_arm', 'gte', '2'),

              ('has-fwi-trinket', 0, 0, 'item', 'fwi trinket', 'absent', '');

            INSERT INTO condition_effects(condition_id, effect, amount) VALUES
              ('ice-slip', 'delay_ms', 4000),
              ('no-climb-injured', 'forbid', 0),
              ('has-fwi-trinket', 'forbid', 0);
            ",
        )
        .expect("fixture");
        conn
    }

    fn ranger(survival: i64) -> CharacterSnapshot {
        CharacterSnapshot {
            skills: BTreeMap::from([("survival".into(), survival)]),
            stats: BTreeMap::from([("prof".into(), "Ranger".into())]),
            encumbrance: 10,
            ..CharacterSnapshot::default()
        }
    }

    /// The case that started this: a skilled, unencumbered character walks the
    /// ice at full speed, and a green one waits four seconds a room.
    #[test]
    fn the_ice_costs_time_only_for_those_who_would_slip() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let ice = &all["ice-slip"];

        assert_eq!(evaluate(ice, &ranger(80)).total_ms(), 0, "80 survival, no pause");
        assert_eq!(evaluate(ice, &ranger(20)).total_ms(), 4000, "20 survival, pause");
    }

    /// Haste rescues the unskilled, which is the third clause of the rule and
    /// the reason a flat cost could not express it.
    #[test]
    fn haste_gets_you_across_regardless_of_survival() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let mut hasted = ranger(20);
        hasted.spells.insert("Haste".into());

        assert_eq!(evaluate(&all["ice-slip"], &hasted).total_ms(), 0);
    }

    /// Encumbrance is its own clause: a loaded character slips however skilled.
    #[test]
    fn carrying_too_much_slips_whatever_your_survival() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let mut loaded = ranger(80);
        loaded.encumbrance = 70;

        assert_eq!(evaluate(&all["ice-slip"], &loaded).total_ms(), 4000);
    }

    /// A preference the player set overrides the lot, which is what
    /// `mapdb_ice_mode` is for — and why `preference` is in the vocabulary
    /// rather than being something a client applies afterwards.
    #[test]
    fn a_players_preference_participates_in_the_same_evaluation() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let ice = &all["ice-slip"];

        let mut runner = ranger(20);
        runner.preferences.insert("ice_mode".into(), "run".into());
        assert_eq!(evaluate(ice, &runner).total_ms(), 0, "told to run, so it runs");

        let mut waiter = ranger(80);
        waiter.preferences.insert("ice_mode".into(), "wait".into());
        assert_eq!(evaluate(ice, &waiter).total_ms(), 4000, "told to wait, so it waits");
    }

    /// The thing Lich has nowhere to say. An injured arm forbids a climb
    /// rather than costing more, and the mapdb contains zero conditions
    /// mentioning wounds — measured, not assumed.
    #[test]
    fn climbing_with_an_injured_arm_is_forbidden_not_expensive() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let climb = &all["no-climb-injured"];

        assert!(!evaluate(climb, &ranger(80)).forbidden, "unhurt, go ahead");

        let mut hurt = ranger(80);
        hurt.injuries.insert("right_arm".into(), 3);
        assert!(evaluate(climb, &hurt).forbidden, "a rank 3 arm is not climbing");
    }

    /// An item is what, never where. The snapshot carries possession and the
    /// client decides whether that means worn, held or dug out of a backpack.
    #[test]
    fn an_item_condition_asks_only_whether_you_have_one() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let trinket = &all["has-fwi-trinket"];

        assert!(evaluate(trinket, &ranger(50)).forbidden, "no trinket, no trip");

        let mut owner = ranger(50);
        owner.items.insert("fwi trinket".into());
        assert!(!evaluate(trinket, &owner).forbidden);
    }

    /// Determinism is what makes a plan testable: same snapshot, same answer,
    /// however many times it is asked.
    #[test]
    fn the_same_snapshot_always_evaluates_the_same_way() {
        let conn = fixture();
        let all = load_all(&conn).expect("load");
        let who = ranger(20);
        let first = evaluate(&all["ice-slip"], &who);
        for _ in 0..5 {
            assert_eq!(evaluate(&all["ice-slip"], &who), first);
        }
    }

    /// A condition nobody finished holds for nobody. Applying an empty rule to
    /// everybody is the worse of the two failures.
    #[test]
    fn a_condition_with_no_terms_holds_for_nobody() {
        let empty = Condition { id: "unfinished".into(), ..Condition::default() };
        assert!(!holds(&empty, &ranger(50)));
    }
}
