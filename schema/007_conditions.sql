-- Runtime conditions: named once, referenced by anything that has one.
--
-- Two cases forced this, and they turned out to be one case.
--
-- **The ice fields.** Crossing one is a plain walk, except you pause first or
-- you slip -- unless your Survival is high enough, or you are hasted, or you
-- are travelling light. Lich expresses that by pasting the same closure onto
-- every one of 460 ice edges:
--
--   ;e if (UserVars.mapdb_ice_mode == 'wait')
--        or ((UserVars.mapdb_ice_mode != 'run')
--            and ((XMLData.encumbrance_value > 50)
--                 or ((Skills.survival < 50) and not Spell['Haste'].active?)));
--        sleep 4; end; move 'west'
--
-- **The FWI trinket.** A connector you can only use if you have the item.
--
-- A first draft made these two systems: `connector_requires` for items and a
-- separate rule table for skills. That is the same mistake Lich makes one
-- level up -- a condition is a condition, and what carries it does not change
-- what it is. One vocabulary, referenced by edges and connectors alike.
--
-- # Nothing here is ever evaluated by this repository
--
-- Every input can change between planning and walking. Enhancives swap and
-- Survival moves fifty ranks; a spell lapses; picking up loot crosses an
-- encumbrance threshold. So a condition is **stored, never resolved**, and the
-- client evaluates it at the moment it matters -- which for an edge rule is
-- each traversal, not once per plan.
--
-- That is also why no condition mentions a *value* the codex could compute. It
-- names a subject and a comparison; the answer belongs to the character.

CREATE TABLE conditions (
    id          TEXT PRIMARY KEY,   -- 'ice-slip', 'has-fwi-trinket'
    description TEXT NOT NULL DEFAULT ''
);

-- When a condition holds, in disjunctive normal form.
--
-- Terms sharing a `grp` are ANDed; groups are ORed. Any boolean expression
-- converts to DNF, so this needs no nested tree -- and DNF is a *normal* form,
-- so one condition is always written one way, which is what a format a machine
-- writes and a human diffs requires.
--
-- The ice rule is three groups:
--   0  preference ice_mode eq wait
--   1  preference ice_mode ne run  AND  encumbrance gt 50
--   2  preference ice_mode ne run  AND  skill survival lt 50  AND  spell Haste inactive
CREATE TABLE condition_terms (
    condition_id TEXT    NOT NULL REFERENCES conditions(id) ON DELETE CASCADE,
    grp          INTEGER NOT NULL,
    seq          INTEGER NOT NULL,
    -- Closed, and closed by evidence: these are what the mapdb's 913
    -- conditional edges actually consult, plus `item` for connectors.
    --   skill         Skills.survival -- moves at runtime with enhancives
    --   stat          Stats.prof, Stats.race
    --   society       Society.status
    --   society_rank  Society.rank
    --   encumbrance   XMLData.encumbrance_value -- moves when you pick things up
    --   spell         Spell['Haste'].active?
    --   item          the character has one; worn, held or stowed alike
    --   injury        a wound by location, by severity
    --   posture       standing, kneeling, sitting, prone
    --   preference    a setting the player chose, e.g. ice_mode
    --
    -- `injury` is here on different evidence from the rest. The mapdb contains
    -- **zero** conditions mentioning wounds -- measured -- and yet climbing
    -- with an injured arm is a reliable way to die. That is not an oversight
    -- by whoever wrote the map; it is a thing the format cannot say, so the
    -- knowledge went into scripts and players' heads instead. It is exactly
    -- the kind of fact this layer exists to hold.
    subject      TEXT    NOT NULL CHECK (subject IN
                 ('skill','stat','society','society_rank','encumbrance','spell','item','injury',
                  'posture','preference')),
    key          TEXT    NOT NULL DEFAULT '',
    op           TEXT    NOT NULL CHECK (op IN
                 ('lt','lte','gt','gte','eq','ne','present','absent')),
    value        TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (condition_id, grp, seq)
) WITHOUT ROWID;

-- What follows when it holds.
--
-- `delay_ms` is a pause *before* the command, which a plain cost cannot say:
-- the time is spent standing still on purpose, not walking slowly.
--
-- `forbid` is how a gate is written, and it is deliberately the same mechanism.
-- Lich's dijkstra reads a `nil` cost as "this edge does not exist for you", so
-- impassable is only infinite cost. Naming it keeps that honest instead of
-- hiding it in a magic number.
-- # What an effect cannot yet say: it cost you health
--
-- Measured, on 2026-09-13: a level 100 character walked Wehnimer's Landing to
-- Moonsedge, 168 steps, and arrived at 54% health having fought nothing. The
-- route crosses the Ice Plains and the Crawling Shore, and the passes
-- themselves do the damage --
--
--   "your teeth are chattering so hard you can be heard a mile away"
--   "All the climbing up and down over the icy rocks of this pass has
--    exhausted you -- the bitter cold has taken its toll"
--   "... and hits for 102 points of damage!"
--
-- `delay_ms`, `add_cost_ms` and `forbid` cannot express that. The route is not
-- slower and it is not forbidden; it costs something the cost vector does not
-- have. A router that cannot see it will plan the same crossing again on a
-- character at 54%, and again at 8%.
--
-- The same gap `008_preludes.sql` records for `give attendant 2000`: a toll is
-- a resource cost written as a prelude because there is nowhere else to put
-- it. Two instances now, and the second one can kill somebody.
--
CREATE TABLE condition_effects (
    condition_id TEXT NOT NULL REFERENCES conditions(id) ON DELETE CASCADE,
    effect       TEXT NOT NULL CHECK (effect IN ('delay_ms', 'add_cost_ms', 'forbid')),
    amount       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (condition_id, effect)
) WITHOUT ROWID;

-- Edges pointing at one condition, rather than a copy of it per edge.
--
-- **No foreign key to `edges`, deliberately.** An extracted edge does not exist
-- until overlays are applied, which is after this data loads -- so a key here
-- would impose an ordering the build would have to grow a third phase to
-- satisfy. A row naming an edge that is not there is inert rather than wrong,
-- which is the same call `connector_destinations.to_uid` makes, and the same
-- property test covers it: every row here names a real edge.
-- # When a condition picks the command
--
-- `condition_effects` says a traversal is slower, dearer or impossible. It
-- cannot say the third thing the mapdb actually does:
--
--   ;e if checkspell(112) then move 'west' else move 'swim west' end
--
-- Water Walking (spell 112) does not make the crossing slower or forbidden. It
-- changes what you send. Without it you swim.
--
-- That cannot live on the condition, because a condition is shared -- the
-- whole point of `ice-slip` is that one rule covers 132 edges -- and the
-- alternative command differs per edge: `swim west` here, `swim north` next
-- door. So it lives on the link, which is the only per-edge, per-condition
-- place there is.
--
-- The same shape as the `rewrite` remedy in `009_traversal_failures.sql`,
-- which repairs `go` to `climb` after the game complains. This is the
-- predictive half of it: knowing in advance rather than being told.
--
-- # When a condition picks the command
--
-- `condition_effects` says a traversal is slower, dearer or impossible. It
-- cannot say the third thing the mapdb actually does:
--
--   ;e if checkspell(112) then move 'west' else move 'swim west' end
--
-- Water Walking (spell 112) does not make the crossing slower or forbidden. It
-- changes what you send: without it, you swim.
--
-- That cannot live on the condition, because a condition is shared -- the
-- whole point of `ice-slip` is that one rule covers 132 edges -- while the
-- alternative command differs per edge: `swim west` here, `swim north` next
-- door. So it lives on the link, which is the only per-edge, per-condition
-- place there is.
--
-- The same shape as the `rewrite` remedy in `009_traversal_failures.sql`,
-- which repairs `go` to `climb` once the game has complained. This is the
-- predictive half: knowing in advance instead of being told.
--
CREATE TABLE edge_conditions (
    from_uid     INTEGER NOT NULL,
    to_uid       INTEGER NOT NULL,
    command      TEXT    NOT NULL,
    condition_id TEXT    NOT NULL REFERENCES conditions(id) ON DELETE CASCADE,
    -- Send this instead of `command` while the condition holds -- see the note
    -- above. Empty in the usual case, where a condition costs time or forbids
    -- the road rather than changing the way you take it.
    instead      TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (from_uid, to_uid, command, condition_id)
) WITHOUT ROWID;

-- And the same vocabulary for a connector, replacing `connector_requires`.
-- "You must hold the trinket" is `item present`, negated by a `forbid` effect
-- when it does not hold.
CREATE TABLE connector_conditions (
    connector_id TEXT NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    condition_id TEXT NOT NULL REFERENCES conditions(id) ON DELETE CASCADE,
    PRIMARY KEY (connector_id, condition_id)
) WITHOUT ROWID;

DROP TABLE connector_requires;

CREATE INDEX idx_edge_conditions_condition ON edge_conditions(condition_id);
