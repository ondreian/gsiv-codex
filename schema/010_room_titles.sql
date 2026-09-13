-- The names a room answers to.
--
-- `rooms.title` is one string because almost every room has one name and a
-- column is the honest shape for that. It is not the whole truth: the map
-- database records titles as an *array*, and 303 rooms carry more than one.
-- `[Abbey Cellar]` and `[Abbey, Cellar]` are the same place, uid 4132024.
--
-- That is a curiosity until something matches on the name, and then it is a
-- bug. Symbol of Seeking offers a destination by printing its room name and
-- you confirm the one you want; a client holding one spelling asks straight
-- past its own destination, twenty times, and reports that the road does not
-- exist. The Abbey Cellar is a Seeking destination.
--
-- So the array gets a table. `rooms.title` stays as the first one, because it
-- is what a person wants to read and every display already uses it.
CREATE TABLE room_titles (
    uid   INTEGER NOT NULL REFERENCES rooms(uid) ON DELETE CASCADE,
    -- The map database's own order. First is the one on `rooms.title`.
    seq   INTEGER NOT NULL,
    title TEXT    NOT NULL,
    PRIMARY KEY (uid, seq)
) WITHOUT ROWID;

-- Matching runs the other way: a name in hand, which room is it?
CREATE INDEX idx_room_titles_title ON room_titles(title);
