#!/usr/bin/env python3
"""Read Lich's `move()` and write the traversal-failure vocabulary.

    tools/split_lich_move.py ~/dev/lich-5 data/vocabulary

Lich 5 holds twenty years of "what the game says when you do not move" as 200
lines of Ruby `elsif`, with the messages packed into alternations of up to
twenty-six. This splits them one-per-row and attaches the classification below,
so the knowledge lands in a table instead of in a control-flow graph.

The *splitting* is mechanical -- doing it by hand is how a typo gets into data
whose whole job is to match the wire. The *classification* is the table at the
bottom of this file, and it is the only judgment here. Each entry is keyed by
the source line of the branch it classifies, so a Lich release that moves a
branch fails loudly rather than silently mapping the wrong remedy.

Python, in a Rust repository, deliberately: it is a generator that runs when
Lich cuts a release, its output is reviewed as a diff, and a Ruby-parsing port
would be four times the code for the same yearly run.
"""

import re
import sys
from pathlib import Path

# --- what each branch means ------------------------------------------------
#
# Keyed by source line in lib/global_defs.rb. Fields:
#   id           the class name
#   disposition  retry | edge-wrong | unavailable | arrived
#   remedies     [(remedy, detail)], performed in order before re-sending
#   attempts     0 = until the step's budget runs out; N = at most N times
#   note         why, where the branch is not self-evident
#
# Read against the Ruby: a branch that calls `put_dir.call` is `retry` and its
# remedies are the statements before that call; `return false` is `edge-wrong`;
# `return nil` is `unavailable`; `return true` is `arrived`.

BRANCHES = {
    607: dict(
        id="engaged",
        disposition="retry",
        remedies=[("retreat", ""), ("retreat", "")],
        note="Twice, as Lich does. One retreat moves you one range band.",
    ),
    612: dict(
        id="hidden",
        disposition="retry",
        remedies=[("unhide", "")],
        note="Some rooms will not admit someone nobody can see.",
    ),
    615: dict(
        id="wrong-door",
        disposition="retry",
        remedies=[("rewrite", "door>second door")],
        attempts=1,
        note="Several doors, and the map named the ordinal wrong. Lich walks "
        "the ordinal up one each time; the detail here is the first step of "
        "that, and a client that implements the walk can keep going.",
    ),
    625: dict(
        id="no-such-exit",
        disposition="edge-wrong",
        note="The largest branch, and the one that matters most: twenty-six "
        "ways for the game to say the exit is not there. Lich returns false, "
        "which its map layer takes as licence to delete the edge.",
    ),
    631: dict(
        id="not-allowed",
        disposition="unavailable",
        note="A ticket, a guest pass, a club membership, a reputation. The "
        "exit is real and this character may not use it -- Lich returns nil "
        "*specifically* so the direction is not removed from the map. This is "
        "the Silverwood Manor case.",
    ),
    638: dict(
        id="transient-refusal",
        disposition="retry",
        remedies=[("sleep", "1000"), ("wait-rt", "")],
        note="Vertigo, a guard checking you in, a shimmering field. Nothing is "
        "wrong; it simply did not take this time.",
    ),
    642: dict(
        id="climb-failed",
        disposition="retry",
        remedies=[("sleep", "1000"), ("wait-rt", ""), ("stand", "")],
        note="You fell. Stand before trying again.",
    ),
    648: dict(
        id="swimming",
        disposition="arrived",
        note="Sailor's Grief. The swim messages are progress, not failure.",
    ),
    651: dict(
        id="thread-tumble",
        disposition="retry",
        remedies=[("sleep", "500"), ("wait-rt", ""), ("stand", ""), ("empty-hands", "")],
        note="The silvery thread needs both hands. Lich refills them on "
        "arrival, which is why `empty-hands` implies a refill.",
    ),
    661: dict(
        id="too-injured-to-climb",
        disposition="retry",
        remedies=[("cast", "9704")],
        attempts=1,
        note="Sigil of Resolve. Lich returns nil when the character does not "
        "know it, so a client without the spell should treat this as "
        "`unavailable` -- the remedy is conditional on knowing 9704, which is "
        "a character fact and belongs in the snapshot, not here.",
    ),
    669: dict(
        id="needs-climb",
        disposition="retry",
        remedies=[("rewrite", "go>climb")],
        attempts=1,
        note="The map says `go`, the game wants `climb`. A correction the "
        "overlay should eventually carry so nobody pays the round trip.",
    ),
    672: dict(
        id="needs-go",
        disposition="retry",
        remedies=[("rewrite", "climb>go")],
        attempts=1,
        note="The same mistake in the other direction.",
    ),
    675: dict(
        id="cannot-drag",
        disposition="retry",
        remedies=[("rewrite", "climb>go")],
        attempts=1,
        note="Lich also rebuilds the command as `drag <name> <target>` from "
        "the earlier grab line. Only the simple repair is expressed here: "
        "dragging a body is not travel, and a router that plans one is "
        "answering a question nobody asked.",
    ),
    693: dict(
        id="hands-full",
        disposition="retry",
        remedies=[("empty-hands", "")],
        note="Climbing and swimming want both hands.",
    ),
    697: dict(
        id="way-is-closed",
        disposition="retry",
        remedies=[("open-the-way", "")],
        attempts=1,
        note="Open it once. If it is still closed it is locked, and opening "
        "it again will not help.",
    ),
    708: dict(
        id="wait-n-seconds",
        disposition="retry",
        remedies=[("wait-rt", "")],
        note="`...Wait 3 seconds.` -- the server's own roundtime refusal. The "
        "count is in the message, so a client reads it there rather than "
        "sleeping a constant.",
    ),
    715: dict(
        id="must-be-standing",
        disposition="retry",
        remedies=[("stand", ""), ("wait-rt", "")],
        note="Sitting, lying, kneeling. Thirteen ways to be told to get up.",
    ),
    719: dict(
        id="still-recovering",
        disposition="retry",
        remedies=[("sleep", "2000")],
    ),
    722: dict(
        id="knocked-down",
        disposition="retry",
        remedies=[("sleep", "1000"), ("stand", "")],
    ),
    726: dict(
        id="fell-down",
        disposition="retry",
        remedies=[("sleep", "1000"), ("stand", "")],
    ),
    730: dict(
        id="type-ahead-refused",
        disposition="retry",
        remedies=[("sleep", "1000")],
        note="Server backpressure: more commands in flight than the "
        "subscription allows. The only failure here caused by the client "
        "rather than the world, and the one a batching walker will meet first.",
    ),
    733: dict(
        id="stunned",
        disposition="retry",
        remedies=[("wait-stun", "")],
        note="Lich also waits out stun *before* every send, so seeing this at "
        "all means the stun landed between the check and the command.",
    ),
    736: dict(
        id="slipped",
        disposition="retry",
        remedies=[("wait-rt", ""), ("stand", ""), ("wait-rt", "")],
        note="The ice. Six seconds of roundtime and you are on your back, in "
        "the room you started in. This is the recovery; the four-second pause "
        "in `conditions` is the avoidance, and a walk needs both -- the pause "
        "is skippable at high Survival, and the recovery never is.",
    ),
    741: dict(
        id="disk-wobbled",
        disposition="retry",
        remedies=[],
        note="Your disk did not follow. Send it again; nothing to fix.",
    ),
    743: dict(
        id="swim-failed",
        disposition="retry",
        remedies=[("wait-rt", "")],
    ),
    746: dict(
        id="item-at-your-feet",
        disposition="retry",
        remedies=[("stow-feet", ""), ("sleep", "1000")],
        note="The game refuses to let you walk off and leave it.",
    ),
    750: dict(
        id="shocked",
        disposition="retry",
        remedies=[("sleep", "500"), ("wait-stun", ""), ("wait-rt", ""), ("stand", "")],
        note="A boltstone apparatus discharging into you.",
    ),
    757: dict(
        id="senses-lost",
        disposition="retry",
        remedies=[("await", "You regain control of your senses!")],
        note="Lich polls for the line for three seconds. The text is the "
        "signal, so a client waits for it rather than for a duration.",
    ),
    764: dict(
        id="pitch-dark",
        disposition="unavailable",
        note="DIVERGENCE FROM LICH. Lich returns *true* here, which claims you "
        "arrived; you did not, and every command after it is sent from the "
        "wrong room. You need a light source. Recorded as unavailable, which "
        "is what it is.",
    ),
}


def split_alternation(rx):
    """Top-level `|` only -- never inside (), [], or an escape."""
    out, depth, in_class, i, cur = [], 0, False, 0, ""
    while i < len(rx):
        c = rx[i]
        if c == "\\":
            cur += rx[i : i + 2]
            i += 2
            continue
        if c == "[":
            in_class = True
        elif c == "]":
            in_class = False
        elif not in_class and c == "(":
            depth += 1
        elif not in_class and c == ")":
            depth -= 1
        if c == "|" and depth == 0 and not in_class:
            out.append(cur)
            cur = ""
        else:
            cur += c
        i += 1
    out.append(cur)
    return [a for a in (x.strip() for x in out) if a]


def branches_of(path):
    """(source line, patterns) for every `elsif line =~ //` in move()."""
    text = path.read_text()
    start = text.index("def move(dir = 'none'")
    end = text.index("\ndef watchhealth", start)
    base = text[:start].count("\n") + 1
    found = []
    for i, line in enumerate(text[start:end].splitlines()):
        m = re.match(r"\s*(?:els)?if line =~ /(.*)/\s*$", line)
        if m:
            found.append((base + i, split_alternation(m.group(1))))
            continue
        # One branch guards the message with a second test -- `(line =~ /.../)
        # and (dir =~ /door/)`. The guard is not recorded: its remedy rewrites
        # `door`, which is inert on a command that has no `door` in it, so the
        # branch guards itself.
        m = re.match(r"\s*(?:els)?if \(line =~ /(.*)/\) and \(", line)
        if m:
            found.append((base + i, split_alternation(m.group(1))))
            continue
        m = re.match(r"\s*(?:els)?if line == (['\"])(.*)\1\s*$", line)
        if m:
            found.append((base + i, [re.escape(m.group(2))]))
    return found


def tsv(path, rows):
    """Canonical: sorted, tab-separated, LF, trailing newline, no tabs in values."""
    for row in rows:
        for cell in row:
            if "\t" in str(cell) or "\n" in str(cell):
                raise SystemExit(f"value contains a tab or newline: {cell!r}")
    body = "".join("\t".join(str(c) for c in row) + "\n" for row in sorted(rows))
    path.write_text(body, newline="\n")
    return len(rows)


def main():
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    lich, out = Path(sys.argv[1]), Path(sys.argv[2])
    defs = lich / "lib" / "global_defs.rb"
    if not defs.exists():
        raise SystemExit(f"no {defs} -- pass the root of a lich-5 checkout")

    found = branches_of(defs)
    seen = {line for line, _ in found}
    if seen != set(BRANCHES):
        missing = sorted(set(BRANCHES) - seen)
        extra = sorted(seen - set(BRANCHES))
        raise SystemExit(
            "move() has changed; classify it again before regenerating.\n"
            f"  classified but no longer present: {missing}\n"
            f"  present but unclassified:         {extra}"
        )

    classes, patterns, remedies = [], [], []
    for line, pats in found:
        b = BRANCHES[line]
        classes.append(
            (
                b["id"],
                b["disposition"],
                # Precedence is the source line: Ruby evaluated the branches in
                # that order, and the first match won. A table is a set, so the
                # order has to be written down or it is lost.
                line,
                b.get("attempts", 0),
                b.get("note", ""),
                f"global_defs.rb:{line}",
            )
        )
        patterns.extend((b["id"], p) for p in pats)
        for seq, (remedy, detail) in enumerate(b.get("remedies", [])):
            remedies.append((b["id"], seq, remedy, detail))

    # Numeric prefixes order the load: a child table must not arrive
    # before the parent its foreign key needs.
    n = tsv(out / "060_failure_classes.tsv", classes)
    p = tsv(out / "061_failure_patterns.tsv", patterns)
    r = tsv(out / "062_failure_remedies.tsv", remedies)
    print(f"{n} classes, {p} patterns, {r} remedies -> {out}")


if __name__ == "__main__":
    main()
