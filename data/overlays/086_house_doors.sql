-- Chartered houses are members-only.
--
-- A non-member walking in is stopped at the door -- "The doorman stops you.
-- 'You're not a member of this house, Norhaak!'" -- and until this overlay
-- nothing said so: the door was a plain walk, and the house's own lockers
-- looked like public ones, so `/travel locker` from nearby routed straight
-- into Sylvanfair.
--
-- Derived rather than listed. Every room inside a house carries a `house`
-- facet (tag_map), so a door is any edge from a room outside the house into a
-- room inside it -- whatever class it is, which also catches a portal or a
-- window. Moving within a house, and leaving it, are untouched.
--
-- The condition holds for anyone whose `house` stat is not this house, and
-- holding forbids the road. A character never probed has no `house` stat, and
-- `ne` against nothing holds, so the door is shut until PROFILE FULL says
-- otherwise -- the safe way round: the other way strands a non-member at a
-- door they cannot open.

-- The doors first, as a view the rest reads from: a house with no edge leading
-- into it from outside gets no condition at all. Argent Aspis is one -- sixty
-- tagged rooms and nothing in the mapdb walks in -- and a condition hung on
-- nothing is what `every_condition_is_hung_on_something` refuses to publish.
CREATE TEMP VIEW house_doors AS
SELECT e.from_uid, e.to_uid, e.command, inside.detail AS house,
       'not-house-member:' || lower(replace(inside.detail, ' ', '-')) AS condition_id
  FROM edges e
  JOIN room_facets inside
    ON inside.room_uid = e.to_uid AND inside.type = 'house'
 WHERE NOT EXISTS (
       SELECT 1 FROM room_facets f
        WHERE f.room_uid = e.from_uid AND f.type = 'house'
          AND f.detail = inside.detail);

INSERT OR IGNORE INTO conditions(id, description)
SELECT DISTINCT condition_id, 'the character is not a member of ' || house
  FROM house_doors;

INSERT OR IGNORE INTO condition_terms(condition_id, grp, seq, subject, key, op, value)
SELECT DISTINCT condition_id, 0, 0, 'stat', 'house', 'ne', house
  FROM house_doors;

INSERT OR IGNORE INTO condition_effects(condition_id, effect, amount)
SELECT DISTINCT condition_id, 'forbid', 0
  FROM house_doors;

INSERT OR IGNORE INTO edge_conditions(from_uid, to_uid, command, condition_id)
SELECT from_uid, to_uid, command, condition_id
  FROM house_doors;

DROP VIEW house_doors;

-- And if a plan made before this was published walks into the door anyway,
-- the refusal is named rather than waited out. Here rather than in
-- 061_failure_patterns.tsv, which is regenerated from lich's move().
INSERT OR IGNORE INTO failure_patterns(class_id, pattern) VALUES
  ('not-allowed', '^The \w+ stops you\.\s+"You''re not a member of this house, [A-Za-z]+!');
