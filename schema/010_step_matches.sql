-- What a cycling step is waiting to hear.
--
-- A `cycle` step does not take a destination: it advances a mechanism, reads
-- what the mechanism offered, and stops when the offer is one it wants. These
-- are the offers it wants.
--
-- Several strings per step, because "the one you want" is rarely one spelling:
--
--   Symbol of Seeking offers a *room name*, and a room may answer to more than
--   one -- `[Abbey Cellar]` and `[Abbey, Cellar]` are uid 4132024, a Seeking
--   destination. A client holding one spelling asks straight past its own
--   destination twenty times and reports the road does not exist.
--
--   The crown's tine offers a *stone*: `push tine` until "the stone ring
--   surrounding the crown rotates, finally stopping with the tine set with the
--   <stone> aligned". Nothing to do with rooms at all.
--
-- # Why here and not on the room
--
-- Because the values are not always about rooms. An earlier draft of this put
-- a room's alternate titles in a `room_titles` table, which was world data and
-- true -- and then the tine arrived wanting stone names, and the mural wanting
-- omens, and the table answered none of it.
--
-- What every cycle actually needs is "the strings this step accepts", which is
-- a property of the step. For Seeking the generator resolves those from the
-- map database's title array at generation time, so they are complete by
-- construction rather than complete if somebody remembered to publish the
-- room. For the tine they are stone names, and the same column holds them.
--
-- `connector_steps.capture` says *how* to read the offer -- a pattern, or
-- nothing for "read the room name the game printed". This says what counts as
-- the right one.
CREATE TABLE connector_step_matches (
    connector_id TEXT    NOT NULL,
    kind         TEXT    NOT NULL,
    to_uid       INTEGER NOT NULL,
    seq          INTEGER NOT NULL,
    -- One accepted offer, verbatim as the game says it.
    value        TEXT    NOT NULL,
    PRIMARY KEY (connector_id, kind, to_uid, seq, value),
    FOREIGN KEY (connector_id, kind, to_uid, seq)
        REFERENCES connector_steps(connector_id, kind, to_uid, seq) ON DELETE CASCADE
) WITHOUT ROWID;
