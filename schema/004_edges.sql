-- World, slice 4: walking.
--
-- The key is the content. urnon keeps a surrogate `edge_id` and the digraph
-- design reserves it for §6's parallel edges -- two rows agreeing on
-- (from_uid, to_uid, command) and differing only in their gate. Measured
-- against the real store, that case does not exist: all 65,053 edges are
-- distinct on those three columns, because gates are unbuilt. When they arrive
-- the gate joins the key rather than hiding behind a number.
--
-- **Script edges are not here and will not be.** 3,037 of the mapdb's edges
-- carry embedded Lich Ruby -- kilobytes of it, with newlines and tabs that
-- would break the published format outright. They are Lich implementation
-- rather than facts about the game, they are already excluded from routing
-- because Ruby is not a replayable command, and they are exactly what
-- connectors replace. What remains is 62,016 walk edges whose longest command
-- is 40 characters.

CREATE TABLE edges (
    from_uid INTEGER NOT NULL REFERENCES rooms(uid) ON DELETE CASCADE,
    to_uid   INTEGER NOT NULL REFERENCES rooms(uid) ON DELETE CASCADE,
    -- What to send. `north`, `go archway`, `climb rolaren gate`.
    command  TEXT    NOT NULL,
    -- `script` is deliberately absent from the vocabulary rather than merely
    -- unused: a row carrying Ruby cannot be written here at all.
    class    TEXT    NOT NULL DEFAULT 'walk'
             CHECK (class IN ('walk', 'transit', 'teleport')),
    -- How long the traversal takes, for the router's cost model.
    time_ms  INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (from_uid, to_uid, command)
) WITHOUT ROWID;

-- Routing asks "what leads out of here" on every node it expands.
CREATE INDEX idx_edges_from ON edges(from_uid);

-- A command issued in a room that leads to more than one room is not an edge.
-- It is a mechanism wearing an exit's clothes -- Silverwood Manor's `go door`
-- standing in four towns, an arena exit that returns you whence you came.
-- Walking one records something true once and false every time after.
--
-- 46 (from_uid, command) pairs in the mapdb name more than one destination, so
-- this is not hypothetical. Listing a pair here excludes every edge matching
-- it from the published graph; the mechanism belongs in `connectors`.
CREATE TABLE ambiguous_commands (
    from_uid INTEGER NOT NULL REFERENCES rooms(uid) ON DELETE CASCADE,
    command  TEXT    NOT NULL,
    reason   TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (from_uid, command)
) WITHOUT ROWID;
