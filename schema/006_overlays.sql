-- Corrections, applied after the import.
--
-- This is the table that makes option C work. An overlay is *ours*; the
-- vendored mapdb is Lich's. Because the import runs first and overlays second,
-- refreshing `vendor/map.json` cannot silently revert a correction -- which
-- neither committing the built tables nor rebuilding from the mapdb alone can
-- manage, and which every serious use of this repository turns out to be.
--
-- An overlay that the new mapdb has made unnecessary shows up as one that no
-- longer changes anything. That is a property test, not a cleanup chore.

-- An edge we assert that the mapdb does not, or asserts differently.
--
-- The first population is the script-edge burn-down: a mapdb edge whose
-- "command" is Ruby wrapping a single real command --
-- `;e move('go doorframe');UserVars.mapdb_talondown_origin = 41` -- is a walk
-- edge wearing a costume. Reading `go doorframe` out of it is analysis, not
-- something the mapdb states, so it belongs here rather than in the importer.
CREATE TABLE edge_overlays (
    from_uid INTEGER NOT NULL,
    to_uid   INTEGER NOT NULL,
    command  TEXT    NOT NULL,
    class    TEXT    NOT NULL DEFAULT 'walk'
             CHECK (class IN ('walk', 'transit', 'teleport')),
    time_ms  INTEGER NOT NULL DEFAULT 0,
    -- Where the assertion comes from, so a reviewer can weigh it:
    -- `script-extract` is a command read out of a mapdb script edge,
    -- `observed` is somebody walking it, `manual` is a person deciding.
    source   TEXT    NOT NULL CHECK (source IN ('script-extract', 'observed', 'manual')),
    note     TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (from_uid, to_uid, command)
) WITHOUT ROWID;

-- A mapdb script edge and what became of it.
--
-- Every one is either superseded by something -- an overlay, a connector -- or
-- listed here as unhandled with a reason. The coverage property asserts there
-- is no third case, so "how much of the script-edge problem is solved" is a
-- number rather than a feeling.
CREATE TABLE script_edge_disposition (
    from_uid INTEGER NOT NULL,
    to_uid   INTEGER NOT NULL,
    -- The mapdb's Ruby, truncated. Kept so a reviewer can see what was decided
    -- about, but never published as a command: it is not one.
    excerpt  TEXT    NOT NULL DEFAULT '',
    -- `excluded` is a decision, not a gap: the edge is understood and will
    -- never be published. Silverwood Manor's door needs club membership, and
    -- membership is an invisible character flag -- there is no condition to
    -- write and no way to test for it but to walk into the door. A route
    -- planned through something you cannot observe is a route that strands
    -- somebody, so it is off, and saying so here stops a later reader
    -- "fixing" it.
    disposition TEXT NOT NULL
                CHECK (disposition IN ('overlay', 'connector', 'unhandled', 'excluded')),
    -- Required for `unhandled`: nine of these read game output and solve a
    -- riddle, and saying so is better than an empty row implying nobody looked.
    reason   TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (from_uid, to_uid),
    CHECK (disposition NOT IN ('unhandled', 'excluded') OR reason <> '')
) WITHOUT ROWID;
