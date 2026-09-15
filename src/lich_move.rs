//! Reading Lich's `move()` into the traversal-failure vocabulary.
//!
//! Lich 5 holds twenty years of "what the game says when you do not move" as
//! 200 lines of Ruby `elsif`, with the messages packed into alternations of up
//! to twenty-six. This splits them one per row and attaches the classification
//! in [`BRANCHES`], so the knowledge lands in a table instead of in a
//! control-flow graph.
//!
//! The *splitting* is mechanical — doing it by hand is how a typo gets into
//! data whose whole job is to match the wire. The *classification* is the only
//! judgment here, and it is keyed by the source line of the branch it
//! classifies: a Lich release that moves a branch fails loudly rather than
//! silently attaching the wrong remedy to the wrong message.
//!
//! No regex crate. The Ruby being matched is three fixed shapes, and adding a
//! dependency to recognise `elsif line =~ /` would be a larger commitment than
//! the thing it parses.

use std::path::Path;

/// What one branch of `move()` means. See `schema/009_traversal_failures.sql`.
///
/// Read against the Ruby: a branch that calls `put_dir.call` is `retry` and its
/// remedies are the statements before that call; `return false` is
/// `edge-wrong`; `return nil` is `unavailable`; `return true` is `arrived`.
pub struct Branch {
    /// Source line in `lib/global_defs.rb`, which is also the precedence: Ruby
    /// evaluated the branches in this order and the first match won.
    pub line: u32,
    pub id: &'static str,
    pub disposition: &'static str,
    /// Performed in order before re-sending. `(remedy, detail)`.
    pub remedies: &'static [(&'static str, &'static str)],
    /// 0 = as often as the step's budget allows; N = at most N times.
    pub attempts: u32,
    pub note: &'static str,
}

const fn b(
    line: u32,
    id: &'static str,
    disposition: &'static str,
    remedies: &'static [(&'static str, &'static str)],
    attempts: u32,
    note: &'static str,
) -> Branch {
    Branch {
        line,
        id,
        disposition,
        remedies,
        attempts,
        note,
    }
}

pub const BRANCHES: &[Branch] = &[
    b(
        607,
        "engaged",
        "retry",
        &[("retreat", ""), ("retreat", "")],
        0,
        "Twice, as Lich does. One retreat moves you one range band.",
    ),
    b(
        612,
        "hidden",
        "retry",
        &[("unhide", "")],
        0,
        "Some rooms will not admit someone nobody can see.",
    ),
    b(
        615,
        "wrong-door",
        "retry",
        &[("rewrite", "door>second door")],
        1,
        "Several doors, and the map named the ordinal wrong. Lich walks the \
         ordinal up one each time; the detail here is the first step of that, \
         and a client that implements the walk can keep going.",
    ),
    b(
        625,
        "no-such-exit",
        "edge-wrong",
        &[],
        0,
        "The largest branch, and the one that matters most: twenty-six ways \
         for the game to say the exit is not there. Lich returns false, which \
         its map layer takes as licence to delete the edge.",
    ),
    b(
        631,
        "not-allowed",
        "unavailable",
        &[],
        0,
        "A ticket, a guest pass, a club membership, a reputation. The exit is \
         real and this character may not use it -- Lich returns nil \
         *specifically* so the direction is not removed from the map. This is \
         the Silverwood Manor case.",
    ),
    b(
        638,
        "transient-refusal",
        "retry",
        &[("sleep", "1000"), ("wait-rt", "")],
        0,
        "Vertigo, a guard checking you in, a shimmering field. Nothing is \
         wrong; it simply did not take this time.",
    ),
    b(
        642,
        "climb-failed",
        "retry",
        &[("sleep", "1000"), ("wait-rt", ""), ("stand", "")],
        0,
        "You fell. Stand before trying again.",
    ),
    b(
        648,
        "swimming",
        "arrived",
        &[],
        0,
        "Sailor's Grief. The swim messages are progress, not failure.",
    ),
    b(
        651,
        "thread-tumble",
        "retry",
        &[
            ("sleep", "500"),
            ("wait-rt", ""),
            ("stand", ""),
            ("empty-hands", ""),
        ],
        0,
        "The silvery thread needs both hands. Lich refills them on arrival, \
         which is why `empty-hands` implies a refill.",
    ),
    b(
        661,
        "too-injured-to-climb",
        "retry",
        &[("cast", "9704")],
        1,
        "Sigil of Resolve. Lich returns nil when the character does not know \
         it, so a client without the spell should treat this as `unavailable` \
         -- the remedy is conditional on knowing 9704, which is a character \
         fact and belongs in the snapshot, not here.",
    ),
    b(
        669,
        "needs-climb",
        "retry",
        &[("rewrite", "go>climb")],
        1,
        "The map says `go`, the game wants `climb`. A correction the overlay \
         should eventually carry so nobody pays the round trip.",
    ),
    b(
        672,
        "needs-go",
        "retry",
        &[("rewrite", "climb>go")],
        1,
        "The same mistake in the other direction.",
    ),
    b(
        675,
        "cannot-drag",
        "retry",
        &[("rewrite", "climb>go")],
        1,
        "Lich also rebuilds the command as `drag <name> <target>` from the \
         earlier grab line. Only the simple repair is expressed here: dragging \
         a body is not travel, and a router that plans one is answering a \
         question nobody asked.",
    ),
    b(
        693,
        "hands-full",
        "retry",
        &[("empty-hands", "")],
        0,
        "Climbing and swimming want both hands.",
    ),
    b(
        697,
        "way-is-closed",
        "retry",
        &[("open-the-way", "")],
        1,
        "Open it once. If it is still closed it is locked, and opening it \
         again will not help.",
    ),
    b(
        708,
        "wait-n-seconds",
        "retry",
        &[("wait-rt", "")],
        0,
        "`...Wait 3 seconds.` -- the server's own roundtime refusal. The count \
         is in the message, so a client reads it there rather than sleeping a \
         constant.",
    ),
    b(
        715,
        "must-be-standing",
        "retry",
        &[("stand", ""), ("wait-rt", "")],
        0,
        "Sitting, lying, kneeling. Thirteen ways to be told to get up.",
    ),
    b(
        719,
        "still-recovering",
        "retry",
        &[("sleep", "2000")],
        0,
        "",
    ),
    b(
        722,
        "knocked-down",
        "retry",
        &[("sleep", "1000"), ("stand", "")],
        0,
        "",
    ),
    b(
        726,
        "fell-down",
        "retry",
        &[("sleep", "1000"), ("stand", "")],
        0,
        "",
    ),
    b(
        730,
        "type-ahead-refused",
        "retry",
        &[("sleep", "1000")],
        0,
        "Server backpressure: more commands in flight than the subscription \
         allows. The only failure here caused by the client rather than the \
         world, and the one a batching walker will meet first.",
    ),
    b(
        733,
        "stunned",
        "retry",
        &[("wait-stun", "")],
        0,
        "Lich also waits out stun *before* every send, so seeing this at all \
         means the stun landed between the check and the command.",
    ),
    b(
        736,
        "slipped",
        "retry",
        &[("wait-rt", ""), ("stand", ""), ("wait-rt", "")],
        0,
        "The ice. Six seconds of roundtime and you are on your back, in the \
         room you started in. This is the recovery; the four-second pause in \
         `conditions` is the avoidance, and a walk needs both -- the pause is \
         skippable at high Survival, and the recovery never is.",
    ),
    b(
        741,
        "disk-wobbled",
        "retry",
        &[],
        0,
        "Your disk did not follow. Send it again; nothing to fix.",
    ),
    b(743, "swim-failed", "retry", &[("wait-rt", "")], 0, ""),
    b(
        746,
        "item-at-your-feet",
        "retry",
        &[("stow-feet", ""), ("sleep", "1000")],
        0,
        "The game refuses to let you walk off and leave it.",
    ),
    b(
        750,
        "shocked",
        "retry",
        &[
            ("sleep", "500"),
            ("wait-stun", ""),
            ("wait-rt", ""),
            ("stand", ""),
        ],
        0,
        "A boltstone apparatus discharging into you.",
    ),
    b(
        757,
        "senses-lost",
        "retry",
        &[("await", "You regain control of your senses!")],
        0,
        "Lich polls for the line for three seconds. The text is the signal, so \
         a client waits for it rather than for a duration.",
    ),
    b(
        764,
        "pitch-dark",
        "unavailable",
        &[],
        0,
        "DIVERGENCE FROM LICH. Lich returns *true* here, which claims you \
         arrived; you did not, and every command after it is sent from the \
         wrong room. You need a light source. Recorded as unavailable, which \
         is what it is.",
    ),
];

/// Split a Ruby alternation on top-level `|` — never inside `()`, `[]`, or an
/// escape.
fn split_alternation(rx: &str) -> Vec<String> {
    let (mut out, mut cur) = (Vec::new(), String::new());
    let (mut depth, mut in_class) = (0i32, false);
    let mut chars = rx.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            cur.push(c);
            if let Some(next) = chars.next() {
                cur.push(next);
            }
            continue;
        }
        match c {
            '[' => in_class = true,
            ']' => in_class = false,
            '(' if !in_class => depth += 1,
            ')' if !in_class => depth -= 1,
            _ => {}
        }
        if c == '|' && depth == 0 && !in_class {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Escape a Ruby literal so it reads as a regex matching exactly itself.
fn escape(literal: &str) -> String {
    let mut out = String::with_capacity(literal.len() + 4);
    for c in literal.chars() {
        if "\\^$.|?*+()[]{}".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The patterns of one branch, by source line, in source order.
fn branches_of(source: &str) -> Vec<(u32, Vec<String>)> {
    let Some(start) = source.find("def move(dir = 'none'") else {
        return Vec::new();
    };
    let end = source[start..]
        .find("\ndef watchhealth")
        .map_or(source.len(), |e| start + e);
    let base = source[..start].matches('\n').count() + 1;

    let mut found = Vec::new();
    for (i, raw) in source[start..end].lines().enumerate() {
        let line = base as u32 + i as u32;
        let t = raw.trim_start();
        let t = t.strip_prefix("elsif ").or_else(|| t.strip_prefix("if "));
        let Some(t) = t else { continue };

        // `line =~ /.../`
        if let Some(rest) = t.strip_prefix("line =~ /") {
            if let Some(rx) = rest.strip_suffix('/') {
                found.push((line, split_alternation(rx)));
            }
            continue;
        }
        // One branch guards the message with a second test --
        // `(line =~ /.../) and (dir =~ /door/)`. The guard is not recorded:
        // its remedy rewrites `door`, which is inert on a command with no
        // `door` in it, so the branch guards itself.
        if let Some(rest) = t.strip_prefix("(line =~ /") {
            if let Some(cut) = rest.find("/) and (") {
                found.push((line, split_alternation(&rest[..cut])));
            }
            continue;
        }
        // `line == '...'`
        for quote in ['\'', '"'] {
            let head = format!("line == {quote}");
            if let Some(rest) = t.strip_prefix(&head)
                && let Some(lit) = rest.strip_suffix(quote)
            {
                found.push((line, vec![escape(lit)]));
            }
        }
    }
    found
}

/// The three files, as canonical TSV: `(file name, contents)`.
///
/// Sorted, tab-separated, LF, trailing newline. Numeric prefixes order the
/// load so a child table never arrives before the parent its foreign key
/// needs.
pub fn render(source: &str) -> Result<Vec<(&'static str, String)>, String> {
    let found = branches_of(source);

    let seen: Vec<u32> = found.iter().map(|(l, _)| *l).collect();
    let known: Vec<u32> = BRANCHES.iter().map(|b| b.line).collect();
    if seen != known {
        let missing: Vec<u32> = known
            .iter()
            .copied()
            .filter(|l| !seen.contains(l))
            .collect();
        let extra: Vec<u32> = seen
            .iter()
            .copied()
            .filter(|l| !known.contains(l))
            .collect();
        return Err(format!(
            "move() has changed; classify it again before regenerating.\n  \
             classified but no longer present: {missing:?}\n  \
             present but unclassified:         {extra:?}"
        ));
    }

    let (mut classes, mut patterns, mut remedies) = (Vec::new(), Vec::new(), Vec::new());
    for (branch, (line, pats)) in BRANCHES.iter().zip(&found) {
        classes.push(format!(
            "{}\t{}\t{}\t{}\t{}\tglobal_defs.rb:{line}",
            branch.id, branch.disposition, line, branch.attempts, branch.note
        ));
        for p in pats {
            patterns.push(format!("{}\t{p}", branch.id));
        }
        for (seq, (remedy, detail)) in branch.remedies.iter().enumerate() {
            remedies.push(format!("{}\t{seq}\t{remedy}\t{detail}", branch.id));
        }
    }

    let mut out = Vec::new();
    for (name, mut rows) in [
        ("060_failure_classes.tsv", classes),
        ("061_failure_patterns.tsv", patterns),
        ("062_failure_remedies.tsv", remedies),
    ] {
        for row in &rows {
            if row.contains('\n') {
                return Err(format!("{name}: a value contains a newline"));
            }
        }
        rows.sort();
        out.push((name, rows.join("\n") + "\n"));
    }
    Ok(out)
}

/// Regenerate the vocabulary from a Lich checkout into `out_dir`.
pub fn regenerate(lich: &Path, out_dir: &Path) -> Result<(usize, usize, usize), String> {
    let defs = lich.join("lib").join("global_defs.rb");
    let source = std::fs::read_to_string(&defs).map_err(|e| {
        format!(
            "{}: {e} -- pass the root of a lich-5 checkout",
            defs.display()
        )
    })?;
    let files = render(&source)?;
    let mut counts = [0usize; 3];
    for (i, (name, body)) in files.iter().enumerate() {
        std::fs::write(out_dir.join(name), body)
            .map_err(|e| format!("{}: {e}", out_dir.join(name).display()))?;
        counts[i] = body.lines().count();
    }
    Ok((counts[0], counts[1], counts[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real thing, when a checkout is present. Skipped otherwise, because
    /// a developer without Lich should still be able to run the suite.
    fn lich_source() -> Option<String> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .join("lich-5/lib/global_defs.rb");
        std::fs::read_to_string(p).ok()
    }

    #[test]
    fn alternation_splits_only_at_the_top_level() {
        assert_eq!(
            split_alternation(r"^You can't (?:go|swim)\.|^Where are you\?"),
            vec![r"^You can't (?:go|swim)\.", r"^Where are you\?"]
        );
    }

    /// A `|` inside a character class is a literal pipe, not a branch.
    #[test]
    fn a_pipe_in_a_character_class_is_not_a_split() {
        assert_eq!(split_alternation(r"^[a|b]c").len(), 1);
    }

    /// And an escaped one is a literal too -- the case that would silently
    /// halve a pattern into two that match nothing.
    #[test]
    fn an_escaped_pipe_is_not_a_split() {
        assert_eq!(split_alternation(r"a\|b").len(), 1);
    }

    /// A space is literal in a regex, so escaping one is noise in a file
    /// whose whole purpose is to be read.
    #[test]
    fn a_literal_branch_becomes_a_pattern_matching_itself() {
        assert_eq!(escape("You are still stunned."), r"You are still stunned\.");
        assert_eq!(escape("a+b (c)"), r"a\+b \(c\)");
    }

    /// The guard that makes this safe to re-run: a Lich release that moves a
    /// branch must fail rather than attach the wrong remedy to the wrong
    /// message.
    #[test]
    fn a_changed_move_is_refused_rather_than_misclassified() {
        let err = render("def move(dir = 'none')\n  nothing\nend\n").expect_err("no branches");
        assert!(err.contains("classify it again"), "{err}");
    }

    #[test]
    fn the_committed_vocabulary_is_what_lich_says_today() {
        /// Whether a row came out of Lich rather than out of a measurement.
        fn is_lich(source: &str) -> bool {
            source.starts_with("global_defs.rb:")
        }

        let Some(source) = lich_source() else {
            eprintln!("no lich-5 checkout alongside this repo; skipping");
            return;
        };
        let files = render(&source).expect("render");
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/vocabulary");

        // The committed vocabulary is Lich's plus ours. `rifted` was measured
        // from gswiki rather than read out of `global_defs.rb`, and there will
        // be more like it -- the game is the source for things Lich never
        // encoded. So this compares the Lich-derived half and leaves the rest
        // alone, which is still the point: catching the day Lich changes a
        // branch under us.
        //
        // Both files are keyed by class id in the first column, and only the
        // classes file records where a row came from, so the ids are gathered
        // there once and used for both.
        let classes =
            std::fs::read_to_string(dir.join("060_failure_classes.tsv")).expect("classes");
        let measured: Vec<&str> = classes
            .lines()
            .filter(|l| !l.split('\t').next_back().is_some_and(is_lich))
            .filter_map(|l| l.split('\t').next())
            .collect();

        for (name, body) in files {
            let committed = std::fs::read_to_string(dir.join(name)).expect("committed file");
            let lich_half: String = committed
                .lines()
                .filter(|l| !l.split('\t').next().is_some_and(|c| measured.contains(&c)))
                .map(|l| format!("{l}\n"))
                .collect();
            assert_eq!(body, lich_half, "{name} is stale; run `codex failures`");
        }
    }
}
