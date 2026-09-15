-- Kalivan's caravans.
--
--   ;e multifput 'inquire','order N','order confirm';
--      waitfor 'The wagon comes to a halt', ...
--
-- Ask what is going, name one, confirm, and wait an hour. Four of these are
-- fixed roads and are published; two are not, and say why below.
--
-- # Not gated, unlike the ferries
--
-- The portmasters carry a `no-portmasters` condition because the *mapdb*
-- prices them with a preference -- `UserVars.mapdb_use_portmasters == true ?
-- 1200 : nil` -- and nil means impassable. The caravans are priced with a
-- plain 3600. The mapdb does not make them a choice, so neither does this.
--
-- An hour is its own deterrent. The router is multi-objective and will not
-- spend one to save a walk; where a caravan wins it is because it is the only
-- road, which for the Pinefar Trading Post -- and so for the Cavern of Ages,
-- and so for the Rift -- it is.

INSERT INTO room_sets(name, description) VALUES
  ('caravan:abbey', 'The caravan master in the Abbey stables.'),
  ('caravan:illistim', 'Seethe Naedal, Transportation.');
INSERT INTO room_set_terms(set_name, seq, op, value) VALUES
  ('caravan:abbey', 0, 'room', '4132017'),
  ('caravan:illistim', 0, 'room', '13205201');

INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES
  ('caravan:abbey', 'only', 'caravan:abbey', 0,
   'Kalivan''s caravan from the Abbey, to three places. An hour on a wagon.'),
  ('caravan:illistim', 'only', 'caravan:illistim', 0,
   'The caravan from Ta''Illistim to the Hinterwilds. An hour on a wagon.');

-- The ordinal is the menu position and is fixed at each stand, which is why
-- these can be written down at all. The Sanctum's caravans read theirs back
-- from `inquire` because they are not; see the exclusions.
INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) VALUES
  ('caravan:abbey',    'fixed', 9004,    3600000),
  ('caravan:abbey',    'fixed', 4564001, 3600000),
  ('caravan:abbey',    'fixed', 7503001, 3600000),
  ('caravan:illistim', 'fixed', 7503001, 3600000);

-- `order confirm` is the step that commits, so it is the one that waits for
-- arrival. The two before it are the conversation.
INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect, timeout_ms) VALUES
  ('caravan:abbey', 'fixed', 9004, 0, 'inquire', '', 0),
  ('caravan:abbey', 'fixed', 9004, 1, 'order 1', '', 0),
  ('caravan:abbey', 'fixed', 9004, 2, 'order confirm', 'The wagon comes to a halt', 3600000),

  ('caravan:abbey', 'fixed', 4564001, 0, 'inquire', '', 0),
  ('caravan:abbey', 'fixed', 4564001, 1, 'order 2', '', 0),
  ('caravan:abbey', 'fixed', 4564001, 2, 'order confirm', 'The wagon comes to a halt', 3600000),

  ('caravan:abbey', 'fixed', 7503001, 0, 'inquire', '', 0),
  ('caravan:abbey', 'fixed', 7503001, 1, 'order 3', '', 0),
  ('caravan:abbey', 'fixed', 7503001, 2, 'order confirm', 'The wagon comes to a halt', 3600000),

  ('caravan:illistim', 'fixed', 7503001, 0, 'inquire', '', 0),
  ('caravan:illistim', 'fixed', 7503001, 1, 'order 1', '', 0),
  ('caravan:illistim', 'fixed', 7503001, 2, 'order confirm', 'the caravan comes to a stop', 3600000);

INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4132017, 9004,    'multifput ''inquire'',''order 1'',''order confirm''', 'connector', 'published as caravan:abbey'),
  (4132017, 4564001, 'multifput ''inquire'',''order 2'',''order confirm''', 'connector', 'published as caravan:abbey'),
  (4132017, 7503001, 'multifput ''inquire'',''order 3'',''order confirm''', 'connector', 'published as caravan:abbey'),
  (13205201, 7503001,'multifput ''inquire'',''order 1'',''order confirm''', 'connector', 'published as caravan:illistim');

-- # The two out of the Hinterwilds are already published, and over-permissively
--
-- `070_connectors.sql` publishes `caravan:hinterwilds` to both the snowfields
-- and Ta'Illistim, unconditionally, at an hour each. The mapdb prices them
--
--   UserVars.mapdb_hinterwilds_location == 'IM' ? 240 : nil
--
-- -- four minutes if you arrived from Icemule, and *impassable* otherwise. You
-- may ride back only to the place you came from, and the outbound trip is what
-- recorded which.
--
-- So the published pair offers a road that does not exist half the time, and
-- prices the half that does at fifteen times its cost. It is left as it is
-- rather than quietly narrowed, because the fix is not a smaller road: the
-- condition is *journey history*, and `condition_terms` can ask about
-- preferences, skills, spells and encumbrance -- not about where somebody was
-- an hour ago. A client that remembers its own outbound trip could answer it;
-- this file cannot, and pretending otherwise by dropping the destinations
-- would lose a road that works.
--
-- Written down here because it is the kind of wrong that looks right: the
-- connector loads, the suite passes, and the strand happens in the Hinterwilds
-- an hour from anywhere.

-- # The Sanctum's caravans read their own menu
--
--   res = dothistimeout "inquire", 5, /(\d)\) Wehnimer's Landing/
--   ... dothistimeout "order #{choice}" ...
--
-- The ordinal is not fixed there, so the script asks and reads the number back
-- out of the answer. That is the same shape as the Character Manager's skill
-- menu and wants the same treatment -- a step that matches rather than sends.
-- Left unhandled: it is a mechanism worth having, not a line worth guessing.
INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4216902, 9004,    'dothistimeout "inquire", 5, /(\d)\) Wehnimer''s Landing/', 'unhandled', 'reads its ordinal back from the menu; needs a match step'),
  (4216902, 4744007, 'dothistimeout "inquire", 5, /(\d)\) Vornavis/',            'unhandled', 'reads its ordinal back from the menu; needs a match step'),
  (4216903, 4216101, 'dothistimeout "inquire", 5, /(\d)\) the Sea of Fire/',     'unhandled', 'reads its ordinal back from the menu; needs a match step'),
  (4216901, 4216101, 'dothistimeout "inquire", 5, /(\d)\) the Sea of Fire/',     'unhandled', 'reads its ordinal back from the menu; needs a match step');
