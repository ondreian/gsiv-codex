-- Things you do before you move.
--
-- `if` and `unless` preludes are a permanent category, not a long tail. Before
-- a single `move`, the mapdb variously searches for the exit, kneels to fit
-- through, pulls a rope, empties its hands, pays an attendant 2000 silvers, or
-- waits for your disk to catch up:
--
--   fput 'search'                                                    61 edges
--   fput 'kneel' unless kneeling? or (Stats.race =~ /Dwarf|Halfling|Gnome/)   37
--   fput 'pull rope' / 'push rope' / 'open trapdoor'                  15
--   40.times { sleep 0.1; break if ... disk }                          6
--   empty_hands                                                        4
--   cast Celerity if known && affordable && !active                    3
--   fput 'give attendant 2000'                                         3
--
-- A prelude is neither a delay nor a cost nor a refusal, so `condition_effects`
-- cannot hold one. It is a command, and it is named and referenced for the same
-- reason a condition is: `search` before a move appears on 61 edges, and 61
-- copies of a rule is the thing this schema exists to stop.

CREATE TABLE preludes (
    id           TEXT PRIMARY KEY,   -- 'search-for-the-exit', 'kneel-to-fit'
    description  TEXT NOT NULL DEFAULT '',
    -- When to do it. NULL means always -- `search` is not conditional on
    -- anything, you simply cannot see the exit until you look.
    --
    -- One condition gates the whole prelude, which covers every shape the
    -- mapdb actually has. A prelude whose steps need different conditions is
    -- two preludes.
    condition_id TEXT REFERENCES conditions(id) ON DELETE CASCADE
);

-- What to send, in order. Same shape as `connector_steps`, because it is the
-- same thing: commands with a way to tell whether they worked.
CREATE TABLE prelude_steps (
    prelude_id TEXT    NOT NULL REFERENCES preludes(id) ON DELETE CASCADE,
    seq        INTEGER NOT NULL,
    command    TEXT    NOT NULL,
    -- What the game says on success, verbatim, when there is something
    -- reliable to match. Empty when the command just needs sending.
    expect     TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (prelude_id, seq)
) WITHOUT ROWID;

-- Which edges carry it. No key to `edges`, for the reason `edge_conditions`
-- gives: an extracted edge does not exist until overlays are applied, and a
-- row naming an absent edge is inert rather than wrong. The property test
-- covers it.
CREATE TABLE edge_preludes (
    from_uid   INTEGER NOT NULL,
    to_uid     INTEGER NOT NULL,
    command    TEXT    NOT NULL,
    prelude_id TEXT    NOT NULL REFERENCES preludes(id) ON DELETE CASCADE,
    PRIMARY KEY (from_uid, to_uid, command, prelude_id)
) WITHOUT ROWID;

CREATE INDEX idx_edge_preludes_prelude ON edge_preludes(prelude_id);

-- `posture`, so "kneel unless already kneeling" can be said. Character state
-- the client already tracks, like the rest of the vocabulary.
--
-- Note what is *not* here: `fput 'give attendant 2000'` is a toll, and a toll
-- is a resource cost rather than a prelude. It is written as a prelude for now
-- because the cost vector does not exist yet, and that is worth remembering
-- when it does -- a router that cannot see the 2000 silvers will happily plan
-- a route the character cannot afford.

-- # The climbs, and why they are not published
--
-- 16 of the rooms with no way in are behind a body shaped like this:
--
--   ;e Spell[9704].cast if known? and !active? and affordable?;
--      empty_hands;
--      fput 'stance offensive' if Skills.climbing < 20;
--      move 'climb boulder';
--      waitrt?; fput 'stance defensive'; fill_hands
--
-- The road is `climb boulder`. Everything around it is preparation, and three
-- quarters of it this schema can already say: `stance offensive` gated on
-- `skill climbing lt 20` is a prelude with a condition, exactly like
-- `kneel-to-fit`.
--
-- `empty_hands` is the one that stops it. It is a prelude no client here can
-- perform -- the WIT exposes no hands, which is already why `empty-hands` is
-- one of the three remedies a guest declines in `009_traversal_failures.sql`.
-- Publishing the edge without it would route a character into a climb that
-- aborts, and route them there *instead* of around, which is worse than
-- leaving the road unlisted.
--
-- `fill_hands` afterwards is the other half: the character is meant to be
-- carrying things again when they arrive. A prelude vocabulary that can empty
-- hands must be able to fill them, or it leaves people standing in the snow
-- holding nothing.
--
-- So this wants two things, in order: hands on the guest's state surface, and
-- a prelude pair that takes and restores. Then 16 rooms and every other climb
-- in the game become expressible together.
