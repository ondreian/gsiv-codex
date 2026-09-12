-- World, slice 2: named groups of rooms.
--
-- A set is stored as a **definition**, never as a resolved list, and evaluated
-- when asked. A room the mapdb learns later inside a boundary joins
-- automatically; a snapshot would need a tool re-run to notice.
--
-- The town problem this exists for: standing in Icemule Trace, `LOCATION`
-- answers "Icemule Trace". One step north into the Sorcerer Guild it answers
-- "the town of Icemule Trace". Verified live. Both are true, neither is the
-- town, and normalising the strings would erase a distinction the game makes.
-- So "Icemule Trace, the place a person means" is a set over both.

CREATE TABLE room_sets (
    name        TEXT PRIMARY KEY,
    description TEXT NOT NULL DEFAULT ''
);

-- One term of a definition. Terms of a set are **unioned**.
--
-- Union only, for now, and the reason is evidence rather than taste: the
-- digraph design specifies nine node types -- flood, rooms, title, has_poi,
-- has_property, has_tag, in_set, union, intersect, difference -- and today
-- exactly two things want a set at all. Town composition is a union of two
-- location facets. A connector's `anywhere_except` takes one set and negates
-- it at the *connector*, not inside the definition.
--
-- intersect, difference and flood have zero consumers. They are additive when
-- one appears: a later migration adds an `op` to the CHECK, or a parent column
-- for nesting. Building the algebra first would be building a query language
-- nobody has asked a question in.
CREATE TABLE room_set_terms (
    set_name TEXT    NOT NULL REFERENCES room_sets(name) ON DELETE CASCADE,
    -- Ordering is for stable output and readable diffs, not precedence: union
    -- is commutative, so a reordered file is the same set.
    seq      INTEGER NOT NULL,
    -- `facet` selects rooms carrying a facet; `room` names one room directly.
    op       TEXT    NOT NULL CHECK (op IN ('facet', 'room')),
    -- For `facet`: the facet type, and the detail to match. An empty `value`
    -- matches any detail, so `herb` with no value is every room with a herb.
    -- For `room`: key is empty and value is the uid.
    key      TEXT    NOT NULL DEFAULT '',
    value    TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (set_name, seq)
);

CREATE INDEX idx_room_set_terms_op ON room_set_terms(op, key, value);

-- Where the game says you are.
--
-- A room fact, not a town: `LOCATION` is something the game answers, and one
-- town answers several ways. The grouping is a `room_sets` row over these.
-- Class `property` because it is true *of* the room and is not somewhere to
-- travel to -- routing to "Icemule Trace" means routing to a set, not to a
-- facet named after one.
INSERT INTO facet_types(type, class, description) VALUES
    ('location', 'property', 'what the game''s LOCATION verb reports here');
