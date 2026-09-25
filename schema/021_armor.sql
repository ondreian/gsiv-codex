-- World, slice 21: what a body is wearing, and what that costs it.
--
-- Armor decides more than defence. Scale Armor Proficiency makes martial
-- stances free, but only in scale; armor hinders spells by circle; every
-- sub-group carries an action penalty and a minimum roundtime. A decision
-- engine choosing between a spell and a blow needs every one of those per
-- sub-group, and until this slice none of them was here.
--
-- Harvested from lich-5's `armaments/armor_stats.rb` by
-- `scripts/lich-armor.sh`, pinned in `vendor/lich_armor.source`. A bootstrap
-- from lich's measurement, the way creatures are, not a feed.
--
-- Scale is sub-groups 9-12, and the table agrees with gswiki's Armor page on
-- both the roundtime (3-6) and the action penalty (-7 to -12) -- two sources
-- that did not copy each other, landing on the same numbers.

-- One armor sub-group, the unit every penalty is keyed on.
CREATE TABLE armor_subgroups (
    -- 1-20, with 3 and 4 unused. The number the game and the wiki both use.
    asg            INTEGER PRIMARY KEY,
    -- The coarse group: 1 cloth, 2 leather, 3 scale, 4 chain, 5 plate.
    armor_group    INTEGER NOT NULL,
    -- `cloth`, `leather`, `scale`, `chain`, `plate`. What Scale Armor
    -- Proficiency asks about.
    type           TEXT    NOT NULL,
    -- `brigandine armor`. Spaced, as the game writes it.
    base_name      TEXT    NOT NULL,
    base_weight    INTEGER NOT NULL,
    -- Seconds of roundtime the armor adds at minimum, before Armor Use
    -- training reduces it.
    min_rt         INTEGER NOT NULL,
    action_penalty INTEGER NOT NULL,
    normal_cva     INTEGER NOT NULL,
    magical_cva    INTEGER NOT NULL,
    hindrance_max  INTEGER NOT NULL
);

-- The words a piece of armor goes by.
--
-- **Many-to-many, on purpose.** A noun does not decide the sub-group: 38 of the
-- 109 name more than one, and five name more than one *type* --
--
--   breastplate  leather breastplate (9, scale), metal breastplate (17, plate),
--                augmented plate (18, plate)
--   armor        every scale sub-group, 9 through 12
--
-- -- so a lookup by noun returns candidates, and the game's own answer comes
-- from `inspect`: "...allows you to conclude that it is <type>." A table keyed
-- on the noun would have had to pick, and picking is a guess wearing a primary
-- key.
CREATE TABLE armor_nouns (
    noun TEXT    NOT NULL,
    asg  INTEGER NOT NULL REFERENCES armor_subgroups(asg),
    PRIMARY KEY (noun, asg)
);

CREATE INDEX idx_armor_nouns_noun ON armor_nouns(noun);

-- How much a sub-group hinders each spell circle, and how much Armor Use
-- training clears it.
--
-- Both, because either alone misleads. Brigandine hinders Major Elemental by
-- 12 -- until the wearer has 130 ranks of Armor Use, at which point it hinders
-- nothing. A casting decision that read the hindrance without the training
-- would refuse spells a character casts freely.
CREATE TABLE armor_hindrances (
    asg                INTEGER NOT NULL REFERENCES armor_subgroups(asg),
    -- As lich and the game name it: `Major Elemental`, `Ranger Base`.
    circle             TEXT    NOT NULL,
    hindrance          INTEGER NOT NULL,
    armor_use_to_clear INTEGER NOT NULL,
    PRIMARY KEY (asg, circle)
);
