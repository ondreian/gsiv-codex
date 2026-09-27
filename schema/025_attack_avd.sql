-- World, slice 25: what a weapon does against what armor.
--
-- Every swing and bolt adds an AvD to its roll -- `AS - DS + AvD + d100`,
-- landing past 100 -- fixed by what is attacking and the armor sub-group it
-- lands on. A body pricing its own blows against a creature's ASG needs it,
-- and until this slice the only AvDs anywhere were the ones the game printed
-- after the fact.
--
-- Harvested from gswiki by `scripts/wiki-weapon-avd.sh`, fetch recorded in
-- `vendor/wiki_weapon_avd.source`: each weapon skill page's weapon table, and
-- `Bolt spell`'s. Checked against the corpus before it shipped: a closed fist
-- against sub-groups 6 and 12 and a short sword against 20 print exactly the
-- AvD these rows give.
--
-- The published columns are sub-groups 1 and 5-20. Sub-group 2 (robes) has
-- no column of its own and is left absent rather than guessed.

-- A weapon, as the wiki's table describes it.
CREATE TABLE weapons (
    -- Lowercase, as the table names it: `bastard sword`, `closed fist`,
    -- `bastard sword, two-handed`.
    name        TEXT PRIMARY KEY,
    -- Damage factor against each armor group.
    df_cloth    REAL,
    df_leather  REAL,
    df_scale    REAL,
    df_chain    REAL,
    df_plate    REAL,
    -- Base roundtime and the floor training cannot go under, in seconds.
    rt          INTEGER,
    min_rt      INTEGER,
    -- `Slash/Puncture`: which critical tables its hits roll on.
    damage_type TEXT NOT NULL,
    -- Strength/durability, as printed (`75/200`, or `N/A` for a fist).
    str_du      TEXT NOT NULL
);

-- Which skill trains a weapon. Many-to-many: a katar is edged and brawling.
CREATE TABLE weapon_skills (
    weapon TEXT NOT NULL REFERENCES weapons(name) ON DELETE CASCADE,
    skill  TEXT NOT NULL,
    PRIMARY KEY (weapon, skill)
);

-- The AvD itself: one row per attack per armor sub-group.
--
-- `kind` says which table: a `weapon` (named as in `weapons`) or a `bolt`
-- spell (named as the wiki does, `Major Cold (907)`). `variant` tells apart
-- rows the wiki gives one name -- Telekinesis has five, by the object thrown
-- -- in the wiki's order. `avd` is NULL where the wiki writes `V`: Cone of
-- Elements takes the AvD of whatever element it casts.
CREATE TABLE attack_avd (
    attack  TEXT    NOT NULL,
    kind    TEXT    NOT NULL CHECK (kind IN ('weapon', 'bolt')),
    variant INTEGER NOT NULL,
    asg     INTEGER NOT NULL REFERENCES armor_subgroups(asg),
    avd     INTEGER,
    PRIMARY KEY (attack, kind, variant, asg)
);
