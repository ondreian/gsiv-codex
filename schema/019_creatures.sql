-- World, slice 19: what lives where.
--
-- A bounty says "kill 12 kobolds in Lower Dragonsclaw" and every word of that
-- has to become something a client can act on. The creature is a noun to
-- attack; the area is a set of rooms to walk to. Neither was expressible here
-- before, so a parsed bounty was a sentence rather than a destination.
--
-- Harvested from lich-5's creature files, which carry the measurement this
-- repository cannot make from a map: somebody hunted there and wrote down the
-- room ids. 584 of its 627 creatures carry uids, across 127 areas and 9,680
-- rooms, and 94.7% of those rooms are ones this database already knows.
--
-- **Spawn, not encounter.** `creature_habitats` says where a creature *comes
-- from*, which is the question a bounty asks. It deliberately does not say
-- what you might meet while standing there: the enormous rift crawler spawns
-- in 23 rooms of one plane and teleports across the whole Rift, and a table
-- that recorded both as the same fact would send a bounty hunter to the wrong
-- twelve rooms. What you might meet is a question for a hunting engine, from
-- these facts plus its own policy.

-- One creature, by the name the game gives it.
CREATE TABLE creatures (
    -- The display name, lowercase as the game writes it: `kobold`,
    -- `greater ghoul`. Unique across the source's 627 files.
    name  TEXT PRIMARY KEY,
    -- What you type at it, when that differs from the name. A `magna vereri`
    -- answers to `vereri`.
    noun  TEXT NOT NULL,
    -- NULL where the creature has none rather than where nobody measured it.
    -- The Grimswarm scale to whoever meets them, which is thirty of these.
    level INTEGER
);

-- A named place creatures come from. The name is the game's own, which is why
-- it can be joined to a bounty: both halves are quoting the same source.
CREATE TABLE habitats (
    name        TEXT PRIMARY KEY,
    description TEXT NOT NULL DEFAULT ''
);

-- Where a creature was measured, as ranges.
--
-- One table, not two. An earlier draft had `habitat_rooms` for the area's
-- extent and `creature_habitats` for membership, and unioning every creature's
-- ranges into the habitat threw away the thing a bounty actually wants: the
-- kobold was measured in 9028..9041, and the area runs 9008..9071 because
-- something else was measured in the rest of it. "Kill twelve kobolds in Lower
-- Dragonsclaw" is answered by the first and only approximated by the second.
--
-- Both other questions are still here, as queries rather than tables: what
-- lives in an area is `DISTINCT creature WHERE habitat = ?`, and the area's
-- extent is its rooms unioned, which is a `min`/`max` away. Storing a
-- derivable thing next to what it derives from is how the two disagree.
--
-- Ranges rather than one row per room because that is how the measurement was
-- taken and how it reads: 9,680 rooms are a few hundred ranges, and a
-- contributor correcting a boundary edits one line rather than finding forty.
-- `lo = hi` for a single room.
--
-- Not a `room_sets` definition, though the shape is close. Those are evaluated
-- lazily so a room the mapdb learns later joins automatically, which is right
-- for "the rooms the game calls Icemule Trace" and wrong for a measured range
-- nobody has walked past. When a habitat needs to nest inside another -- the
-- Hinterwilds gardens inside the Hinterwilds -- that is the moment to make
-- these sets, and a migration then is cheaper than an abstraction now.
CREATE TABLE creature_rooms (
    creature TEXT    NOT NULL REFERENCES creatures(name) ON DELETE CASCADE,
    habitat  TEXT    NOT NULL REFERENCES habitats(name) ON DELETE CASCADE,
    lo       INTEGER NOT NULL,
    hi       INTEGER NOT NULL,
    PRIMARY KEY (creature, habitat, lo),
    CHECK (hi >= lo)
);

-- The three directions anything asks this data.
--
-- "A bounty named this creature and this area -- where do I go" is the primary
-- key. "I am standing here -- what lives in this area" is the first index, and
-- "which room is this" the second.
CREATE INDEX idx_creature_rooms_habitat ON creature_rooms(habitat);
CREATE INDEX idx_creature_rooms_span ON creature_rooms(lo, hi);
