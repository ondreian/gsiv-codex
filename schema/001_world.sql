-- World, slice 1: rooms and what they are.
--
-- Three tables and two foreign keys, which is the whole argument. A facet
-- could have been stored polymorphically -- one attribute table keyed by
-- (entity_type, entity_id), serving rooms and foes and items alike -- and then
-- neither key below could exist, because a polymorphic parent cannot be
-- referenced. Relations are the thing this layer is for.

-- The vocabulary. A facet's type has to be declared before a room can carry
-- it, so a typo is a foreign key violation rather than a new kind of thing.
CREATE TABLE facet_types (
    -- `type` rather than `kind`: it is the word for this. Rust callers spell
    -- it `r#type`, which is a small and local cost.
    type        TEXT PRIMARY KEY,
    -- How the facet relates to travel. Load-bearing, not descriptive:
    --   poi       somewhere a command works -- a bank teller, a forge. The
    --             only class a router may aim at.
    --   property  true *of* the room: antimagic, nomagic.
    --   route     an origin marker for a travel mechanism. Names hundreds of
    --             rooms you are most likely already standing in, so aiming at
    --             one is meaningless.
    --   resource  something gatherable here.
    class       TEXT NOT NULL CHECK (class IN ('poi', 'property', 'route', 'resource')),
    description TEXT NOT NULL DEFAULT ''
);

-- A room, by its canonical GemStone id -- the `<nav rm='...'>` attribute.
--
-- No location column. What the game calls a place is `LOCATION`'s answer, and
-- one town gives several: standing in Icemule Trace reports "Icemule Trace",
-- and one step north into the Sorcerer Guild reports "the town of Icemule
-- Trace". Both are true and neither is the town, so a room's place is a set
-- membership rather than a string on the row. Sets arrive in slice 2.
CREATE TABLE rooms (
    uid     INTEGER PRIMARY KEY,
    title   TEXT,
    climate TEXT,
    terrain TEXT
);

-- What a room is. One row per fact.
--
-- `detail` qualifies the type where the type alone does not say enough: a
-- herb's species, a guild's profession. Empty rather than NULL so the primary
-- key stays simple -- a room may hold many herbs, so (room, type) alone will
-- not do.
CREATE TABLE room_facets (
    room_uid INTEGER NOT NULL REFERENCES rooms(uid) ON DELETE CASCADE,
    type     TEXT    NOT NULL REFERENCES facet_types(type),
    detail   TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (room_uid, type, detail)
) WITHOUT ROWID;

-- Answering "where is the nearest bank" means scanning by type, not by room.
CREATE INDEX idx_room_facets_type ON room_facets(type, detail);
