-- A thirteenth remedy: pay someone.
--
-- Wehnimer's Landing's west gate stops a non-citizen coming in: "Excuse me,
-- sir, there's an entrance fee to use this gate.  Five silvers please." The
-- fee is already priced (018 edge_costs), so a router knows the road costs
-- five silver and can check the purse -- but nothing paid it, and a walker
-- that only knew how to wait and retry sent `go gate` into the guard until it
-- gave up (GST, 2026-09-28). What the guard wants is `give guard 5`
-- (Benjamin, 2026-09-28).
--
-- `give-silver` keeps the vocabulary closed: its detail is `payee:amount`, a
-- lowercase noun and a whole number, and the client builds the command. Data
-- never carries the command string.
--
-- SQLite cannot alter a CHECK, so the table is rebuilt; rows are carried over,
-- as 016 did for dispositions.
CREATE TABLE failure_remedies_new (
    class_id TEXT    NOT NULL REFERENCES failure_classes(id) ON DELETE CASCADE,
    seq      INTEGER NOT NULL,
    remedy   TEXT    NOT NULL CHECK (remedy IN (
                 'wait-rt', 'wait-stun', 'sleep', 'stand', 'empty-hands',
                 'open-the-way', 'unhide', 'retreat', 'stow-feet', 'cast',
                 'rewrite', 'await',
                 'give-silver'   -- detail: 'payee:amount' -- `give guard 5`
             )),
    detail   TEXT    NOT NULL DEFAULT '',
    PRIMARY KEY (class_id, seq)
) WITHOUT ROWID;

INSERT INTO failure_remedies_new (class_id, seq, remedy, detail)
SELECT class_id, seq, remedy, detail FROM failure_remedies;

DROP TABLE failure_remedies;
ALTER TABLE failure_remedies_new RENAME TO failure_remedies;
