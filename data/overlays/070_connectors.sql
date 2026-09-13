-- Mechanisms that are not walking.
--
-- Each of these is a road the mapdb holds as a `;e` Ruby program, which the
-- importer drops -- so the regions they reach are dark. Measured, before any
-- of this existed:
--
--   the shadow of the Sanctum     2 of 78 rooms reachable
--   the Rift                      0 of 232
--   Kharam-Dzu                    0 of 373
--   the Pinefar forests           0 of 59
--
-- A connector is the same road, written as data: where it works from, where it
-- goes, what to send, and what the game says back.

-- ---------------------------------------------------------------------------
-- The Sanctum of Scales, by periapt
-- ---------------------------------------------------------------------------
--
-- Nobody ever wrote this one, so everybody who wanted it patched their own
-- mapdb at runtime. Ported from a working Lich patch
-- (`lich-scripts/sanctum-mapdb-patch.lic`), which is the honest provenance:
-- the mechanism has been known for years and has never been writable down.
--
-- Two rooms, both tagged `sanctum` in the mapdb, and the link runs both ways:
--
--   4900340  [Fangs of the Serpent, Gateway]  the southern part of Solhaven
--   4216057  [Subterrain, Pit]                the shadow of the Sanctum
--
-- Rub the periapt and a portal appears; walk into the portal. The portal is a
-- room object rather than an exit, and it may already be there -- somebody
-- else's, which works just as well. That is the client's business; the data
-- says what to send.

INSERT INTO room_sets(name, description) VALUES
  ('sanctum-anchors', 'The two rooms a small bone periapt joins.');
INSERT INTO room_set_terms(set_name, seq, op, value) VALUES
  ('sanctum-anchors', 0, 'room', '4900340'),
  ('sanctum-anchors', 1, 'room', '4216057');

INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES
  ('periapt:sanctum', 'only', 'sanctum-anchors', 5000,
   'A small bone periapt: rub it for a viridian portal, and step through.');

INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) VALUES
  ('periapt:sanctum', 'fixed', 4900340, 0),
  ('periapt:sanctum', 'fixed', 4216057, 0);

INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect, timeout_ms) VALUES
  -- Lich's own regex accepts three replies: the portal forms, the periapt
  -- fizzles, or one is already up. Only the first is success; the third is
  -- success by somebody else's effort, and the client tells them apart by
  -- whether a portal is in the room afterwards.
  ('periapt:sanctum', 'fixed', 4900340, 0, 'rub #{item}',
   'spiraling out to form a swirling viridian portal hanging in midair', 5000),
  ('periapt:sanctum', 'fixed', 4900340, 1, 'go #{portal}', '', 3000),
  ('periapt:sanctum', 'fixed', 4216057, 0, 'rub #{item}',
   'spiraling out to form a swirling viridian portal hanging in midair', 5000),
  ('periapt:sanctum', 'fixed', 4216057, 1, 'go #{portal}', '', 3000);

-- No periapt, no road. This is Lich's `Periapt.exists? ? 5 : nil` said once
-- instead of twice per direction -- and `nil` is exactly `forbid`.
INSERT INTO conditions(id, description) VALUES
  ('has-periapt', 'the character carries a small bone periapt');
INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
  ('has-periapt', 0, 0, 'item', 'small bone periapt', 'absent', '');
INSERT INTO condition_effects(condition_id, effect, amount) VALUES
  ('has-periapt', 'forbid', 0);
INSERT INTO connector_conditions(connector_id, condition_id) VALUES
  ('periapt:sanctum', 'has-periapt');

-- ---------------------------------------------------------------------------
-- The Rift, by the sphere
-- ---------------------------------------------------------------------------
--
-- 0 of 232 rooms reachable before this. One way in, from one room:
--
--   4562036  [Breach, Cavern of Ages]   Koar's Shrine
--
-- Enter the sphere, then walk a direction into it, and which direction you
-- take decides which of the Rift's three levels you land on. Thirty seconds,
-- per the mapdb's own `timeto`.
--
-- Lich loops the direction until the game says you came apart --
--
--   "You feel every shred of yourself torn to tiny pieces and reformed..."
--
-- and then stands you up. Neither is recorded. The loop is retry, which a
-- client does by checking the room id rather than the prose, and the stand is
-- `must-be-standing` in the failure vocabulary: a postlude repairing a state
-- the recovery already repairs is a second mechanism for one problem.
--
-- Only the way IN. Every exit is one Ruby program carrying an array of 60
-- room ids -- a maze solver, not an edge -- so the Rift is a place this data
-- can get you to and not yet out of. Said plainly rather than left to be
-- discovered from a room.
--
-- # And this connector does not open the Rift on its own
--
-- Traced, because "the Rift is 0%" needed a cause rather than another guess.
-- The room this works from sits in a pocket of **seven rooms** with no
-- reachable ancestor at any depth: Koar's Shrine and the Rift reach only each
-- other. The pocket has exactly three doors, and two of them are shut:
--
--   the swim edges from the Lake of Tears and the Pool -- extracted, and
--     inside the pocket, so they join nothing to anything
--   `push tine` from Top of the World, Aenatumgana -- itself unreachable
--   **Symbol of Seeking**, from Icemule Trace and the Pinefar Trading Post
--
-- So Seeking is not merely "endgame-critical" in the abstract. It is the way
-- into the Rift, and the sphere is the second half of a journey whose first
-- half is a Voln symbol. Icemule is reachable; 2635 is one Seeking away; the
-- sphere is three rooms further on.

INSERT INTO room_sets(name, description) VALUES
  ('rift-breach', 'The one room the sphere into the Rift can be entered from.');
INSERT INTO room_set_terms(set_name, seq, op, value) VALUES
  ('rift-breach', 0, 'room', '4562036');

INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES
  ('sphere:rift', 'only', 'rift-breach', 0,
   'The sphere in the Cavern of Ages. The direction you walk decides the level.');

INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) VALUES
  ('sphere:rift', 'fixed', 4566001, 30000),
  ('sphere:rift', 'fixed', 4567001, 30000),
  ('sphere:rift', 'fixed', 4568001, 30000);

INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect, timeout_ms) VALUES
  ('sphere:rift', 'fixed', 4566001, 0, 'go sphere', '', 0),
  ('sphere:rift', 'fixed', 4566001, 1, 'north',
   'You feel every shred of yourself torn to tiny pieces and reformed', 30000),
  ('sphere:rift', 'fixed', 4567001, 0, 'go sphere', '', 0),
  ('sphere:rift', 'fixed', 4567001, 1, 'west',
   'You feel every shred of yourself torn to tiny pieces and reformed', 30000),
  ('sphere:rift', 'fixed', 4568001, 0, 'go sphere', '', 0),
  ('sphere:rift', 'fixed', 4568001, 1, 'east',
   'You feel every shred of yourself torn to tiny pieces and reformed', 30000);

-- ---------------------------------------------------------------------------
-- Out of the Hinterwilds, by caravan
-- ---------------------------------------------------------------------------
--
-- 93% of the Hinterwilds was reachable and 0% could get home. The way in is
-- `climb sliver`, a plain edge the mapdb prices at 15,000 seconds -- four
-- hours, which is Lich saying "technically, please do not". The way out is a
-- caravan, and it was a `;e` so it was dropped.
--
--   7503002  the caravan stop
--     order 1  ->  4128001  the southern snowfields   (and so, Icemule)
--     order 2  ->  13205005 Ta'Illistim
--
-- Lich sets `UserVars.mapdb_hinterwilds_location` afterwards. That is its own
-- bookkeeping for routing you back later, not part of the mechanism, and it is
-- not recorded: a client that needs to know where it came from knows already.
--
-- Cost is the mapdb's own 3,600s. An hour is not a wait, it is a timetable --
-- the caravan leaves when it leaves. The router will now plan an hour's wait
-- over a four-hour climb, which is right, and both numbers are the kind of
-- thing a resource-aware cost vector should eventually say properly.

INSERT INTO room_sets(name, description) VALUES
  ('hinterwilds-caravan', 'The caravan stop in the Hinterwilds.');
INSERT INTO room_set_terms(set_name, seq, op, value) VALUES
  ('hinterwilds-caravan', 0, 'room', '7503002');

INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES
  ('caravan:hinterwilds', 'only', 'hinterwilds-caravan', 0,
   'The caravan out of the Hinterwilds. Order 1 for the snowfields, 2 for Ta''Illistim.');

INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) VALUES
  ('caravan:hinterwilds', 'fixed', 4128001, 3600000),
  ('caravan:hinterwilds', 'fixed', 13205005, 3600000);

INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect, timeout_ms) VALUES
  ('caravan:hinterwilds', 'fixed', 4128001, 0, 'inquire', '', 0),
  ('caravan:hinterwilds', 'fixed', 4128001, 1, 'order 1', '', 0),
  ('caravan:hinterwilds', 'fixed', 4128001, 2, 'order confirm',
   'The wagon comes to a halt', 3600000),
  ('caravan:hinterwilds', 'fixed', 13205005, 0, 'inquire', '', 0),
  ('caravan:hinterwilds', 'fixed', 13205005, 1, 'order 2', '', 0),
  ('caravan:hinterwilds', 'fixed', 13205005, 2, 'order confirm',
   'The wagon comes to a halt', 3600000);
