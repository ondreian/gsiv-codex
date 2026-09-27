-- World, slice 26: what a critical hit does.
--
-- A hit that lands rolls a critical on its damage type's table, at a body
-- location, at a rank, and the entry says what happened: extra damage, stun,
-- a knockdown, a severed limb, a wound, or death. A body dies by these or by
-- running out of hit points, and which one it risks depends on the body --
-- "warriors in plate armor with redux are rarely going to be critted to
-- death, wizards in robes are very likely to die from crits" (Benjamin) -- so
-- the hunt-risk model needs every entry.
--
-- Harvested from lich-5's lib/gemstone/critranks by `scripts/lich-crits.sh`,
-- pinned in `vendor/lich_crits.source`. Every flag is 0/1, and NULL where
-- lich did not say; `stunned` is NULL where lich wrote 999, its "unknown".

CREATE TABLE crit_ranks (
    -- The damage type's table: `slash`, `fire`, `ucs-punch`, ...
    type                 TEXT    NOT NULL,
    -- `head`, `left arm`, `nerves`, ... lowercase.
    location             TEXT    NOT NULL,
    -- 0 (a graze) upward; most tables stop at 9, some UCS ones at 11.
    rank                 INTEGER NOT NULL,
    -- Hit points taken on top of the hit's own damage.
    damage               INTEGER NOT NULL,
    -- `PRONE`, `KNEELING` or `SITTING` when the crit knocks the body there.
    position             TEXT,
    fatal                INTEGER CHECK (fatal IN (0, 1)),
    -- Rounds stunned.
    stunned              INTEGER,
    amputated            INTEGER CHECK (amputated IN (0, 1)),
    crippled             INTEGER CHECK (crippled IN (0, 1)),
    sleeping             INTEGER CHECK (sleeping IN (0, 1)),
    dazed                INTEGER CHECK (dazed IN (0, 1)),
    limb_favored         INTEGER CHECK (limb_favored IN (0, 1)),
    -- Seconds of roundtime added.
    roundtime            INTEGER,
    silenced             INTEGER CHECK (silenced IN (0, 1)),
    slowed               INTEGER CHECK (slowed IN (0, 1)),
    -- The wound it leaves at the location, and a second one elsewhere.
    wound_rank           INTEGER NOT NULL,
    secondary_location   TEXT,
    secondary_wound_rank INTEGER,
    -- The start of the message the game prints, as lich matches it -- how a
    -- crit taken is recognised in the log.
    message              TEXT    NOT NULL,
    PRIMARY KEY (type, location, rank)
);
