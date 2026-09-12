-- World, slice 3: travel that is not walking.
--
-- The mapdb can describe walking and nothing else. Sailing, a teleport
-- trinket, the urchin network, a ferry -- Lich encodes all of it as embedded
-- Ruby on a fake edge, which is why 3,037 of them carry `;e` and 164 contain
-- newlines. The knowledge is real; the container was wrong.
--
-- A connector is an **offer**: from these rooms, to that room, run these
-- commands. No mechanism type, no teleporter class, no urchin -- dozens of
-- items differ in every particular except that shape.

CREATE TABLE connectors (
    id          TEXT PRIMARY KEY,
    -- Where it can be used from. `anywhere` is a worn item with no
    -- restriction; `only` is a pier or a town's urchins; `anywhere_except` is
    -- the 35 rooms tagged meta:noteleport:fwi.
    origin_mode TEXT NOT NULL CHECK (origin_mode IN ('anywhere', 'only', 'anywhere_except')),
    -- A set, never a room list. That is the whole reason sets landed first.
    origin_set  TEXT REFERENCES room_sets(name),
    -- Paid once per use whatever the destination: drawing the item, the
    -- roundtime of the gesture.
    overhead_ms INTEGER NOT NULL DEFAULT 0,
    description TEXT    NOT NULL DEFAULT '',
    -- A set is required by two modes and meaningless to the third. Stated as a
    -- constraint because "anywhere, except this set" is a sentence somebody
    -- will write by accident.
    CHECK ((origin_mode = 'anywhere') = (origin_set IS NULL))
);

-- Where a connector can put you.
--
-- Three kinds, closed, and two of them exist because Lich had no word for
-- what it was doing:
--
--   fixed   a known room. The urchins, the Chronomage rooms, the FWI anchors.
--   replan  you land somewhere this cannot name. 636 script edges set
--           `$go2_restart = true`, which is not control flow -- it is a
--           confession that the destination is not fixed. Take the hop, read
--           the room, route on.
--   origin  back where this connector brought you from. Lich writes
--           `UserVars.mapdb_duskruin_origin = 9652` on the way out so the
--           return hop can undo it; a client already knows where it was.
--
-- **`to_uid` is 0, not NULL, for the two kinds that do not name a room**, and
-- the key is the content rather than a sequence number. A `dest_seq` surrogate
-- would be smaller to write and would renumber every later row when a
-- destination is inserted in the middle -- which is exactly the churn the
-- bootstrap format exists to avoid. The cost is that `to_uid` cannot carry a
-- foreign key to `rooms`, so "every fixed destination names a room that
-- exists" is a property test instead. See tests/properties.rs.
CREATE TABLE connector_destinations (
    connector_id TEXT    NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    kind         TEXT    NOT NULL CHECK (kind IN ('fixed', 'replan', 'origin')),
    to_uid       INTEGER NOT NULL DEFAULT 0,
    cost_ms      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (connector_id, kind, to_uid),
    -- A room iff the kind names one. `replan` and `origin` are therefore
    -- singletons per connector, which falls out of the key rather than needing
    -- its own rule: both are (id, kind, 0).
    CHECK ((kind = 'fixed') = (to_uid <> 0))
) WITHOUT ROWID;

-- What to send. Ordered, because a hop is genuinely several commands with
-- different failure modes: `twist #band to 3` can fail in ways `rub #band`
-- cannot. urnon tried modelling a multi-action move as one string once;
-- `edge_steps` shipped and was dropped within the day.
CREATE TABLE connector_steps (
    connector_id TEXT    NOT NULL,
    kind         TEXT    NOT NULL,
    to_uid       INTEGER NOT NULL,
    seq          INTEGER NOT NULL,
    -- Literal, except for `{item}` -- the character's own item id, which the
    -- one rule forbids storing. Not a template language: no conditionals, no
    -- expressions. A second placeholder gets argued for on its own merits.
    command      TEXT    NOT NULL,
    -- What the game says on success, verbatim: "You flag down a nearby
    -- urchin". A game fact, not a client label, and exactly the kind of thing
    -- this layer exists to stop every client rediscovering.
    expect       TEXT    NOT NULL DEFAULT '',
    -- 0 means the client's default. Lich's `dothistimeout "...", 5, /re/`
    -- carries one; the branching around it is error handling, which is the
    -- client's job.
    timeout_ms   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (connector_id, kind, to_uid, seq),
    FOREIGN KEY (connector_id, kind, to_uid)
        REFERENCES connector_destinations(connector_id, kind, to_uid) ON DELETE CASCADE
) WITHOUT ROWID;

-- What the character must have for this to work at all.
--
-- `has-item` says *what*, never *where*. The FWI Ruby opens with
-- `worn = !GameObj[...].nil?` and does nothing further when the answer is yes;
-- the empty-a-hand, find it, put it back dance is the fallback for one in a
-- container. A worn trinket and one in a backpack satisfy the same
-- requirement at different cost to the client, and the client's container
-- store already knows which.
--
-- Not a gate. Whether the character *may* use a mechanism is settled before a
-- connector is ever offered; this is state to arrange, not permission to seek.
CREATE TABLE connector_requires (
    connector_id TEXT NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    requirement  TEXT NOT NULL CHECK (requirement IN ('has-item')),
    detail       TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (connector_id, requirement, detail)
) WITHOUT ROWID;

CREATE INDEX idx_connector_destinations_to ON connector_destinations(to_uid);
