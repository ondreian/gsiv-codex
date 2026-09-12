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
    --   preference    a setting the player chose, e.g. ice_mode
    --
    -- `injury` is here on different evidence from the rest. The mapdb contains
    -- **zero** conditions mentioning wounds -- measured -- and yet climbing
    -- with an injured arm is a reliable way to die. That is not an oversight
    -- by whoever wrote the map; it is a thing the format cannot say, so the
    -- knowledge went into scripts and players' heads instead. It is exactly
    -- the kind of fact this layer exists to hold.
    subject      TEXT    NOT NULL CHECK (subject IN
                 ('skill','stat','society','society_rank','encumbrance','spell','item','injury','preference')),
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
CREATE TABLE condition_effects (
    condition_id TEXT NOT NULL REFERENCES conditions(id) ON DELETE CASCADE,
    effect       TEXT NOT NULL CHECK (effect IN ('delay_ms', 'add_cost_ms', 'forbid')),
    amount       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (condition_id, effect)
) WITHOUT ROWID;

-- 460 edges pointing at one condition, rather than 460 copies of it.
CREATE TABLE edge_conditions (
    from_uid     INTEGER NOT NULL,
    to_uid       INTEGER NOT NULL,
    command      TEXT    NOT NULL,
    condition_id TEXT    NOT NULL REFERENCES conditions(id) ON DELETE CASCADE,
    PRIMARY KEY (from_uid, to_uid, command, condition_id),
    FOREIGN KEY (from_uid, to_uid, command) REFERENCES edges(from_uid, to_uid, command)
        ON DELETE CASCADE
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
