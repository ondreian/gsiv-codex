-- World, slice 24: how a creature fights, as lich-5 measured it.
--
-- A body deciding where to hunt prices its risk with the game's own arithmetic
-- -- a swing lands when `AS - DS + AvD + d100 > 100`, a warding spell when CS
-- beats TD -- so it needs the numbers on both sides. These are the creature's.
--
-- Harvested from lich-5's creature files alongside `creatures` itself, by
-- `codex creatures`. **Ranges stay ranges.** `melee: (152..280)` is not a
-- sloppy 216: the width is how uncertain the measurement is, and it is the
-- prior urnon narrows with every `AS: +N vs DS: +N` line the game prints. A
-- single value is `lo = hi`.
--
-- What lich wrote as prose -- `"???"`, `"250 UAF"`, `"(lunge) 180-200"` -- is
-- left NULL and reported by the harvest, never interpreted here.

-- Everything a creature attacks with, in the six lists lich keeps.
--
-- `kind` is lich's list name verbatim. Nothing downstream should depend on
-- lich filing `Jab` under warding rather than offensive spells; the roll
-- column says which arithmetic applies. Maneuvers and special abilities carry
-- no roll -- they resolve through SMRv2, which urnon learns rather than
-- simulates -- so they are names only.
--
-- `name` verbatim, too: the roll line does not say which attack it was, the
-- line before it does ("An ice troll swings a sword at you!"), and that is
-- the join a posterior will need.
CREATE TABLE creature_attacks (
    creature TEXT    NOT NULL REFERENCES creatures(name) ON DELETE CASCADE,
    kind     TEXT    NOT NULL CHECK (kind IN ('physical_attacks', 'bolt_spells',
                         'warding_spells', 'offensive_spells', 'maneuvers',
                         'special_abilities')),
    -- Position in lich's list: two attacks can share a name.
    ord      INTEGER NOT NULL,
    name     TEXT    NOT NULL,
    -- `as` for a swing or bolt, `cs` for a spell against TD, NULL for none.
    roll     TEXT    CHECK (roll IN ('as', 'cs')),
    lo       INTEGER,
    hi       INTEGER,
    PRIMARY KEY (creature, kind, ord),
    CHECK ((lo IS NULL) = (hi IS NULL)),
    CHECK (hi >= lo)
);

-- What a creature defends with. DS by the kind of attack against it, since a
-- troll that shrugs off a sword may not shrug off a bolt.
CREATE TABLE creature_defenses (
    creature  TEXT PRIMARY KEY REFERENCES creatures(name) ON DELETE CASCADE,
    max_hp    INTEGER,
    -- The armor sub-group it wears, joinable to `armor_subgroups`.
    asg       INTEGER,
    -- 1 for a hide, shell or scales -- lich's `"12N"` -- rather than armor
    -- it put on.
    asg_natural INTEGER NOT NULL DEFAULT 0 CHECK (asg_natural IN (0, 1)),
    melee_lo  INTEGER, melee_hi  INTEGER,
    ranged_lo INTEGER, ranged_hi INTEGER,
    bolt_lo   INTEGER, bolt_hi   INTEGER,
    -- Unarmed defense factor, what a UAC blow is rolled against.
    udf_lo    INTEGER, udf_hi    INTEGER
);

-- Target defense, by the circle of the spell cast at it: a creature resists
-- a wizard's bolt and a cleric's differently.
CREATE TABLE creature_tds (
    creature TEXT    NOT NULL REFERENCES creatures(name) ON DELETE CASCADE,
    -- lich's abbreviation: `bar`, `cle`, `emp`, `pal`, `ran`, `sor`, `wiz`,
    -- `mje`, `mne`, `mjs`, `mns`, `mnm`.
    circle   TEXT    NOT NULL,
    lo       INTEGER NOT NULL,
    hi       INTEGER NOT NULL,
    PRIMARY KEY (creature, circle),
    CHECK (hi >= lo)
);
