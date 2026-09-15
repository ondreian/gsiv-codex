-- A fifth disposition: the plan is stale, and the map is fine.
--
-- Every existing disposition answers "what does this failure mean for the
-- road". `relocated` answers a question none of them can: the road is exactly
-- as described and the character is no longer standing where the plan thinks.
--
-- The Rift moves people. Being "rifted" drops you onto another plane at the
-- same description with different exits, so the screen looks untouched and the
-- next command goes into a room whose doors have been replaced. `retry` sends
-- it again — into the new room. `edge-wrong` quarantines a road that was never
-- wrong. Neither is close.
--
-- SQLite cannot alter a CHECK, so the table is rebuilt. Rows are carried over
-- rather than re-seeded: the vocabulary is data, and a migration that quietly
-- reloads it would lose anything an overlay had corrected.
CREATE TABLE failure_classes_new (
    id          TEXT PRIMARY KEY,
    disposition TEXT NOT NULL
                CHECK (disposition IN ('retry', 'edge-wrong', 'unavailable',
                                       'arrived', 'relocated')),
    precedence  INTEGER NOT NULL UNIQUE,
    attempts    INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    description TEXT NOT NULL DEFAULT '',
    source      TEXT NOT NULL DEFAULT ''
);

INSERT INTO failure_classes_new (id, disposition, precedence, attempts, description, source)
SELECT id, disposition, precedence, attempts, description, source FROM failure_classes;

DROP TABLE failure_classes;
ALTER TABLE failure_classes_new RENAME TO failure_classes;
