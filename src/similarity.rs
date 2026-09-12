//! Telling near-duplicate conditions apart from real ones.
//!
//! The mapdb is crowd-sourced over decades, and nothing in it can be updated in
//! place: a rule is pasted onto every edge it governs, so improving it means
//! editing hundreds of rows. What happens instead is that somebody writes a
//! better version on the twenty edges they were working on and leaves the other
//! hundred and fifty alone.
//!
//! The ice is exactly that. One rule waits 4s and lets Haste excuse low
//! Survival; another waits 6s, ignores the player's `run` preference, and does
//! not accept Haste. Two variants of one idea, or two genuinely different
//! hazards? Only somebody who knows the game can say.
//!
//! So this does not decide. It **shortlists**: renders each condition in a
//! canonical form you can read at a glance, and scores pairs by how much of
//! their structure they share. A human — or an agent reading the rendering —
//! confirms or rejects. The porting stays faithful; the narrowing happens
//! afterwards, on evidence.

use crate::conditions::{Condition, Op, Subject, Term};
use std::collections::BTreeSet;

/// One condition, rendered compactly enough to compare by eye.
///
/// ```text
/// delay:4000  [pref/ice_mode=wait] | [pref/ice_mode!=run & enc>50]
/// ```
///
/// Groups are sorted and so are the terms within them, so two conditions
/// written in different orders render identically. That is the whole trick:
/// the rendering is canonical, so string equality *is* structural equality.
pub fn signature(c: &Condition) -> String {
    render(c, false)
}

/// The same, with every constant replaced by `_`.
///
/// ```text
/// delay  [pref/ice_mode=_] | [pref/ice_mode!=_ & enc>_]
/// ```
///
/// Two conditions sharing a shape but not a signature differ **only in their
/// numbers** — which is the most likely form of "somebody improved one copy".
/// It is the highest-value thing on the shortlist.
pub fn shape(c: &Condition) -> String {
    render(c, true)
}

fn render(c: &Condition, elide: bool) -> String {
    let mut groups: Vec<String> = c
        .groups
        .iter()
        .map(|g| {
            let mut terms: Vec<String> = g.iter().map(|t| render_term(t, elide)).collect();
            terms.sort();
            format!("[{}]", terms.join(" & "))
        })
        .collect();
    groups.sort();

    let mut effects = Vec::new();
    if c.forbid {
        effects.push("forbid".to_string());
    }
    if c.delay_ms != 0 {
        effects.push(if elide {
            "delay".to_string()
        } else {
            format!("delay:{}", c.delay_ms)
        });
    }
    if c.add_cost_ms != 0 {
        effects.push(if elide {
            "cost".to_string()
        } else {
            format!("cost:{}", c.add_cost_ms)
        });
    }
    if effects.is_empty() {
        effects.push("(no effect)".to_string());
    }
    format!("{}  {}", effects.join("+"), groups.join(" | "))
}

fn render_term(t: &Term, elide: bool) -> String {
    let subject = match t.subject {
        Subject::Skill => "skill",
        Subject::Stat => "stat",
        Subject::Society => "soc",
        Subject::SocietyRank => "rank",
        Subject::Encumbrance => "enc",
        Subject::Spell => "spell",
        Subject::Item => "item",
        Subject::Injury => "injury",
        Subject::Posture => "posture",
        Subject::Preference => "pref",
    };
    let op = match t.op {
        Op::Lt => "<",
        Op::Lte => "<=",
        Op::Gt => ">",
        Op::Gte => ">=",
        Op::Eq => "=",
        Op::Ne => "!=",
        Op::Present => "+",
        Op::Absent => "!",
    };
    let head = if t.key.is_empty() {
        subject.to_string()
    } else {
        format!("{subject}/{}", t.key)
    };
    match t.op {
        // Presence needs no operand; printing an empty one is noise.
        Op::Present | Op::Absent => format!("{head}{op}"),
        _ if elide => format!("{head}{op}_"),
        _ => format!("{head}{op}{}", t.value),
    }
}

/// What a condition asks about: subject, key, and the *direction* of the
/// comparison, with values dropped.
///
/// `>` and `>=` collapse to one probe, and so do `<` and `<=`. That is not
/// sloppiness — it is the point. Shape 0 tests `encumbrance > 50` and shape 1
/// tests `encumbrance >= 50`, and a boundary that moves by one between two
/// copies of the same rule is exactly the crowd-sourcing artifact this is
/// hunting. Treating them as different questions would hide the pair; the
/// signature still prints the real operator, so nothing is lost to whoever
/// reads the shortlist.
fn probes(c: &Condition) -> BTreeSet<String> {
    c.groups
        .iter()
        .flatten()
        .map(|t| {
            let mut t = t.clone();
            t.value.clear();
            t.op = match t.op {
                Op::Lt | Op::Lte => Op::Lt,
                Op::Gt | Op::Gte => Op::Gt,
                other => other,
            };
            render_term(&t, true)
        })
        .collect()
}

/// How much two conditions ask about the same things, 0.0 to 1.0.
///
/// Jaccard over the probe sets: what fraction of everything either one asks
/// about, both ask about. Deliberately ignores values and grouping — a score
/// is for ranking a shortlist, and pretending it measures equivalence would
/// invite somebody to merge on it.
pub fn similarity(a: &Condition, b: &Condition) -> f64 {
    let (pa, pb) = (probes(a), probes(b));
    if pa.is_empty() && pb.is_empty() {
        return 1.0;
    }
    let shared = pa.intersection(&pb).count() as f64;
    let total = pa.union(&pb).count() as f64;
    if total == 0.0 { 0.0 } else { shared / total }
}

/// A pair worth a human's attention.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub a: String,
    pub b: String,
    pub score: f64,
    /// True when they differ only in their constants — the strongest signal
    /// that one is a stale copy of the other.
    pub same_shape: bool,
}

/// Pairs of conditions that might be one condition, most suspicious first.
///
/// `floor` is the similarity below which a pair is not worth looking at. This
/// returns candidates and nothing else: whether two rules *should* be one is a
/// question about the game, and the answer is not in the data.
pub fn shortlist(conditions: &[Condition], floor: f64) -> Vec<Candidate> {
    let mut out = Vec::new();
    for (i, a) in conditions.iter().enumerate() {
        for b in &conditions[i + 1..] {
            let score = similarity(a, b);
            let same_shape = shape(a) == shape(b);
            if same_shape || score >= floor {
                out.push(Candidate {
                    a: a.id.clone(),
                    b: b.id.clone(),
                    score,
                    same_shape,
                });
            }
        }
    }
    // Same-shape first, then by score, then by name so the list is stable
    // enough to diff between runs.
    out.sort_by(|x, y| {
        y.same_shape
            .cmp(&x.same_shape)
            .then(y.score.total_cmp(&x.score))
            .then(x.a.cmp(&y.a))
            .then(x.b.cmp(&y.b))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(subject: Subject, key: &str, op: Op, value: &str) -> Term {
        Term {
            subject,
            key: key.into(),
            op,
            value: value.into(),
        }
    }

    /// The real pair, as the mapdb has them.
    fn ice_4s() -> Condition {
        Condition {
            id: "ice-slip".into(),
            groups: vec![
                vec![term(Subject::Preference, "ice_mode", Op::Eq, "wait")],
                vec![
                    term(Subject::Preference, "ice_mode", Op::Ne, "run"),
                    term(Subject::Encumbrance, "", Op::Gt, "50"),
                ],
                vec![
                    term(Subject::Preference, "ice_mode", Op::Ne, "run"),
                    term(Subject::Skill, "survival", Op::Lt, "50"),
                    term(Subject::Spell, "Haste", Op::Absent, ""),
                ],
            ],
            delay_ms: 4000,
            ..Condition::default()
        }
    }

    fn ice_6s() -> Condition {
        Condition {
            id: "ice-slip-resolve".into(),
            groups: vec![
                vec![term(Subject::Preference, "ice_mode", Op::Eq, "wait")],
                vec![term(Subject::Skill, "survival", Op::Lt, "50")],
                vec![term(Subject::Encumbrance, "", Op::Gte, "50")],
            ],
            delay_ms: 6000,
            ..Condition::default()
        }
    }

    #[test]
    fn a_signature_reads_at_a_glance() {
        assert_eq!(
            signature(&ice_4s()),
            "delay:4000  [enc>50 & pref/ice_mode!=run] \
             | [pref/ice_mode!=run & skill/survival<50 & spell/Haste!] | [pref/ice_mode=wait]"
        );
    }

    /// The trick the whole module rests on: order must not change the
    /// rendering, or two identical rules would look different.
    #[test]
    fn term_and_group_order_do_not_change_the_signature() {
        let mut shuffled = ice_4s();
        shuffled.groups.reverse();
        for g in &mut shuffled.groups {
            g.reverse();
        }
        assert_eq!(signature(&shuffled), signature(&ice_4s()));
    }

    /// Same structure, different numbers — the likeliest form of "somebody
    /// improved one copy and could not update the rest".
    #[test]
    fn a_copy_with_different_numbers_shares_a_shape() {
        let mut retuned = ice_4s();
        retuned.id = "ice-slip-strict".into();
        retuned.groups[2][1].value = "75".into();
        retuned.delay_ms = 6000;

        assert_ne!(signature(&retuned), signature(&ice_4s()));
        assert_eq!(shape(&retuned), shape(&ice_4s()), "structurally one rule");

        let list = shortlist(&[ice_4s(), retuned], 0.9);
        assert_eq!(list.len(), 1);
        assert!(list[0].same_shape, "and it says so");
    }

    /// The real pair is *not* the same shape — 6s ignores the `run`
    /// preference and does not accept Haste — so it scores high without
    /// claiming they are interchangeable.
    #[test]
    fn the_two_real_ice_rules_are_similar_but_not_identical() {
        let (a, b) = (ice_4s(), ice_6s());
        assert!(!same(&a, &b), "they genuinely differ");

        // 3 of 5 probes shared: both consult ice_mode, survival and
        // encumbrance; only shape 0 consults Haste, and only it distinguishes
        // `run`.
        let score = similarity(&a, &b);
        assert!((score - 0.6).abs() < 1e-9, "similar, not the same: {score}");

        let list = shortlist(&[a, b], 0.5);
        assert_eq!(list.len(), 1, "shortlisted for a human to judge");
        assert!(!list[0].same_shape);
    }

    fn same(a: &Condition, b: &Condition) -> bool {
        signature(a) == signature(b)
    }

    /// Two rules about unrelated things must not reach the shortlist, or the
    /// list stops being worth reading.
    #[test]
    fn unrelated_conditions_are_not_shortlisted() {
        let trinket = Condition {
            id: "has-fwi-trinket".into(),
            groups: vec![vec![term(Subject::Item, "fwi trinket", Op::Absent, "")]],
            forbid: true,
            ..Condition::default()
        };
        assert_eq!(shortlist(&[ice_4s(), trinket], 0.4).len(), 0);
    }

    /// Ranking must be stable, or the shortlist is noise in a diff.
    #[test]
    fn the_shortlist_is_ordered_the_same_way_twice() {
        let set = vec![ice_4s(), ice_6s()];
        assert_eq!(shortlist(&set, 0.3), shortlist(&set, 0.3));
    }
}
