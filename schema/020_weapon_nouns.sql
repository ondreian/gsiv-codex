-- World, slice 20: what kind of weapon that is.
--
-- Nothing in an item says what it is. The game hands you "a vultite
-- longsword" and the noun is all there is -- but `riposte` needs an edged
-- weapon and `clobber` needs a blunt one, so anything deciding what a
-- character can do right now has to get from the noun to the skill, and
-- until this table there was no way to.
--
-- Simutronics maintains the mapping as the approved-alteration list: every
-- noun a weapon of a given base may legitimately be called, in eight tables,
-- one per weapon skill. That makes it authoritative rather than folklore --
-- it is the list the alteration desk enforces.
--
-- **Nouns, not items.** This says what a *word* means, not what any
-- particular weapon is. An item's display is matched against these nouns by
-- whoever holds the item; a magical longsword called "Widowmaker" is not in
-- here and never will be, and that is a lookup miss rather than a gap in the
-- data.
--
-- `[metal] disc` and its kind are kept verbatim from the source. They will
-- never match an item display, because the real noun substitutes the bracket,
-- but dropping them would quietly lose the base they document.
-- **A noun can mean two weapons.** Keyed on the pair rather than the noun,
-- because two of the 432 are genuinely ambiguous and flattening them would
-- make a lookup confidently wrong:
--
--   palache -> broadsword, scimitar        both Edged Weapons
--   staff   -> quarterstaff, runestaff     Two-Handed Weapons vs Runestaff
--
-- The first costs nothing -- the skill is the same either way. The second is
-- the noun a warrior and a wizard both hold, and it decides whether the thing
-- in the hand permits `sweep` or a spell. A caller resolves it with what the
-- character is trained in, or by looking at the item; it cannot be resolved
-- here, and a row that picked one would be a guess wearing a primary key.
CREATE TABLE weapon_nouns (
    -- The word as it appears in an item's display, lowercase: `katzbalger`.
    noun  TEXT NOT NULL,
    -- What it is a name for: `broadsword`. A base is its own noun too, so
    -- `broadsword -> broadsword` is a row rather than an implied one.
    base  TEXT NOT NULL,
    -- The skill it trains and the techniques it permits, as the game spells
    -- it: `Edged Weapons`, `Blunt Weapons`, `Brawling`, `Runestaff`, ...
    skill TEXT NOT NULL,
    PRIMARY KEY (noun, base)
);

-- "What is this thing in my hand" reads by noun and may get two answers.
CREATE INDEX idx_weapon_nouns_noun ON weapon_nouns(noun);

-- "What can I do with what is in my hand" reads by skill, not by noun.
CREATE INDEX idx_weapon_nouns_skill ON weapon_nouns(skill);
