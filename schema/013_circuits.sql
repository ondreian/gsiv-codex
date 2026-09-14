-- Exits that are somewhere in a subgraph, but not anywhere in particular.
--
-- The Rift does not have exits so much as it grows them. A door, a mirror, a
-- thread, a maw, a staircase or a fissure appears in *some* room of a ring, and
-- the way out is to walk the ring until you are standing with it. The map
-- database encodes this as a script edge, which is why all 165 of them were
-- dropped and the Rift is published as five hundred compass moves between
-- rooms with no way to leave:
--
--   ;e start_room = [ 2579, 2580, ... ];          -- rooms you can join at
--      dirs = [ 'southwest', 'east', ... ];       -- the loop, 108 moves
--      if index = start_room.index(Room.current.id);
--        until checkloot.include?('door') or checkloot.include?('mirror');
--          move dirs[index]; index += 1; index = 0 if index >= dirs.length;
--        end;
--        if checkloot.include?('door'); move 'go door';
--        elsif checkloot.include?('mirror'); move 'go mirror'; end;
--      end
--
-- This is a *search*, not a traversal, and that is why it needs tables of its
-- own rather than a row in `edges`. An edge answers "from here, this command
-- goes there". A circuit answers "from anywhere on this ring, walk until the
-- way out shows itself".
--
-- # Why the loop is longer than the entry list
--
-- 55 entry rooms, 108 moves. The list of entries is where you may *join*; the
-- loop is the whole walk, and after the first pass you are in rooms the entry
-- list never names. So the entry gives an index into the loop and the loop is
-- followed cyclically from there -- which is why `circuit_entries` carries a
-- `seq` rather than the tables being joinable on room alone.
--
-- # Why it is not a connector
--
-- A connector's steps are a sequence: send these, arrive. A circuit's moves
-- are a ring walked an unknown number of times, and the stopping condition is
-- a room object appearing. The nearest relative is a `cycle` step -- ask until
-- the answer is the one you want -- and the difference is what you repeat and
-- what you watch. Modelling one as the other would make both harder to read.

CREATE TABLE circuits (
    id          TEXT    PRIMARY KEY,   -- 'rift:doors:2635'
    -- Where the exit leads, once found. One destination: the door goes where
    -- the door goes, wherever on the ring it turned up.
    to_uid      INTEGER NOT NULL,
    description TEXT    NOT NULL DEFAULT ''
);

-- The ring, in order. `command` is a plain move.
CREATE TABLE circuit_moves (
    circuit_id TEXT    NOT NULL REFERENCES circuits(id) ON DELETE CASCADE,
    seq        INTEGER NOT NULL,
    command    TEXT    NOT NULL,
    PRIMARY KEY (circuit_id, seq)
) WITHOUT ROWID;

-- Where the ring may be joined, and at which move.
CREATE TABLE circuit_entries (
    circuit_id TEXT    NOT NULL REFERENCES circuits(id) ON DELETE CASCADE,
    room_uid   INTEGER NOT NULL,
    -- Index into `circuit_moves`. Follow the loop cyclically from here.
    seq        INTEGER NOT NULL,
    PRIMARY KEY (circuit_id, room_uid)
) WITHOUT ROWID;

-- What to watch for, and what to send when it is here.
--
-- More than one per circuit: a ring may grow a door *or* a mirror, and either
-- is the way out. Matched against the room's objects by noun -- the same
-- question `#{portal:...}` asks, which is not a coincidence: a thing that may
-- or may not be in the room is one problem with two faces.
CREATE TABLE circuit_exits (
    circuit_id TEXT NOT NULL REFERENCES circuits(id) ON DELETE CASCADE,
    noun       TEXT NOT NULL,
    command    TEXT NOT NULL,
    PRIMARY KEY (circuit_id, noun)
) WITHOUT ROWID;

CREATE INDEX idx_circuit_entries_room ON circuit_entries(room_uid);
