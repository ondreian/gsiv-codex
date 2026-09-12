-- What the game says when a move does not happen, and what to do about it.
--
-- Twenty years of Lich's `move()` are a list of messages and the response each
-- one deserves. That knowledge has never been data: it is 200 lines of Ruby
-- `elsif`, so nothing but Lich can read it, and every client that has ever
-- tried to walk somewhere has rediscovered the same sixty messages by falling
-- over in the dark.
--
-- # A known failure is not an unknown failure
--
-- The important distinction is not "did it work". It is *how* it failed, and
-- Lich's own return value already says so in three values:
--
--   true   the room changed -- you are there
--   false  recognised, and the map is wrong: the exit does not exist
--   nil    recognised, and the map is right: you personally cannot, right now
--
-- Those are different instructions to a router. `false` should quarantine the
-- edge and re-plan; `nil` should leave it alone and route around it for this
-- character; and only an unrecognised failure is a reason to give up, because
-- it is the only one nobody knows anything about. Collapsing them -- which is
-- what "walk failed, abort" does -- throws away the entire distinction and
-- makes a crowd-sourced map impossible to correct.
--
-- `disposition` is that return value, with the fourth case Lich has to infer
-- from `room_count` and this can name.
--
-- # The responses are a closed set
--
-- Twenty-eight branches, but only a dozen distinct remedies: wait out the
-- roundtime, stand up, empty your hands, open the door, unhide, retreat, stow
-- what is at your feet, say `climb` instead of `go`. A client implements the
-- dozen verbs; the data says which failure calls for which.

CREATE TABLE failure_classes (
    id          TEXT PRIMARY KEY,   -- 'slipped', 'way-is-closed', 'engaged'
    -- What this failure means for the route, not merely for the step.
    --
    --   retry        perform the remedies and send the command again
    --   edge-wrong   the map is wrong; quarantine the edge and re-plan
    --   unavailable  the map is right; this character cannot use it now
    --   arrived      not a failure at all; the move happened
    disposition TEXT NOT NULL
                CHECK (disposition IN ('retry', 'edge-wrong', 'unavailable', 'arrived')),
    -- Lich's chain is an `elsif`: the first branch whose pattern matches wins,
    -- and several patterns here overlap on purpose. `not-allowed` has
    -- "appears to be closed, perhaps you should try again later?" and
    -- `way-is-closed` has "(?:appears|seems) to be closed.$" -- one means the
    -- shop shut for the night and the other means open the door.
    --
    -- A table is a set and loses that order, so it is written down: try
    -- classes in ascending `precedence` and take the first that matches. The
    -- value is the source line the branch came from, which is the order Ruby
    -- evaluated them in, so it cannot drift from the thing it encodes.
    precedence  INTEGER NOT NULL UNIQUE,
    -- How many times this class may fire for one step before the walker stops
    -- retrying. 0 means "as often as the step's own budget allows" -- Lich
    -- bounds those by 10 seconds or 30 lines rather than by a count. A door is
    -- opened once: if it is still closed, opening it again will not help.
    attempts    INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    description TEXT NOT NULL DEFAULT '',
    -- Where this came from, so a refresh is a diff and not an archaeology
    -- project: `global_defs.rb:736` is the branch it was read out of.
    source      TEXT NOT NULL DEFAULT ''
);

-- One message per row, because that is how they are read and how they are
-- reviewed. Lich holds them as alternations of up to twenty-six, which is one
-- regex nobody can diff.
--
-- The pattern is a Ruby regex as written, kept verbatim rather than translated
-- into some portable subset: it has matched the wire for twenty years, and a
-- rewrite is a transcription error waiting to happen against text nobody can
-- re-observe on demand.
--
-- Verbatim has a price, and it is exactly one pattern. `^You are
-- already(?! as far away as you can get)` uses a negative lookahead, which
-- Ruby, PCRE and JavaScript have and Rust's `regex` crate deliberately does
-- not. A client on such an engine must read it as two tests -- matches `^You
-- are already`, does not match `^You are already as far away as you can get`.
-- A property test pins the count at one, so a second such pattern is a
-- conversation rather than a surprise.
CREATE TABLE failure_patterns (
    class_id TEXT NOT NULL REFERENCES failure_classes(id) ON DELETE CASCADE,
    pattern  TEXT NOT NULL,
    PRIMARY KEY (class_id, pattern)
) WITHOUT ROWID;

-- What to do first, in order. Same shape as `prelude_steps` because it is the
-- same kind of thing -- commands sent before the one you care about -- except
-- that a prelude is known in advance and a remedy is a reaction.
CREATE TABLE failure_remedies (
    class_id TEXT    NOT NULL REFERENCES failure_classes(id) ON DELETE CASCADE,
    seq      INTEGER NOT NULL,
    -- The closed vocabulary. A client implements these once; the data never
    -- carries a command string, because "stand" is `stand` in one game and
    -- something else in the next, and because a remedy that was free text
    -- would be a way to smuggle arbitrary commands into a data file.
    remedy   TEXT    NOT NULL CHECK (remedy IN (
                 'wait-rt',      -- wait out roundtime
                 'wait-stun',    -- wait while the stun indicator is set
                 'sleep',        -- detail: milliseconds
                 'stand',
                 'empty-hands',  -- and refill them on arrival
                 'open-the-way', -- `open` the direction this step was going
                 'unhide',
                 'retreat',
                 'stow-feet',
                 'cast',         -- detail: spell number
                 'rewrite',      -- detail: 'go>climb' -- change the command itself
                 'await'         -- detail: text to wait for before retrying
             )),
    detail   TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (class_id, seq)
) WITHOUT ROWID;

CREATE INDEX idx_failure_patterns_class ON failure_patterns(class_id);
