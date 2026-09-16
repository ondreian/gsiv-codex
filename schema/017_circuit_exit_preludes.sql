-- What to do to a way off before you take it.
--
-- A circuit's exit was one command, which is right for a door you walk through
-- and wrong for a fissure. The Scatter's is a crack you have to widen first:
--
--   5.times { ... result = dothistimeout 'push fissure', 3, /.../
--             break if result =~ /^A wide fissure cannot be opened any farther/ }
--   move 'go fissure'
--
-- That push loop was dropped when the circuit was published and the exit
-- shipped as a bare `go fissure`, which walks a character up to a crack too
-- narrow to enter and leaves them in the Scatter.
--
-- It reuses `preludes`, the same table edges hang `search-for-the-exit` and
-- `kneel-to-fit` on, because it is the same idea: a thing to do before the
-- move, not part of the move.
--
-- SQLite cannot add a column with a foreign key, so the table is rebuilt.
ALTER TABLE circuit_exits RENAME TO circuit_exits_old;

CREATE TABLE circuit_exits (
    circuit_id TEXT NOT NULL REFERENCES circuits(id) ON DELETE CASCADE,
    noun       TEXT NOT NULL,
    command    TEXT NOT NULL,
    -- Run before `command`, in order. NULL for the ordinary case, which is
    -- most of them: a door needs nothing done to it first.
    prelude_id TEXT REFERENCES preludes(id) ON DELETE SET NULL,
    PRIMARY KEY (circuit_id, noun)
) WITHOUT ROWID;

INSERT INTO circuit_exits(circuit_id, noun, command)
SELECT circuit_id, noun, command FROM circuit_exits_old;

DROP TABLE circuit_exits_old;
