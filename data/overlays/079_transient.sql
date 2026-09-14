-- Regions whose exits move.
--
-- The Rift's exits are not placed, they are grown: a door, a mirror, a thread,
-- a maw, a staircase or a fissure appears somewhere on a ring and you walk
-- until you are standing with it. `circuits` publishes the rings; this says
-- that a refusal there means "not yet" rather than "not there".
--
-- The Hinterwilds is here for a smaller reason and the same one: the slivers
-- teleport around. Walking to Brindlestoat's Moneylender on 2026-09-13 got
-- 167 steps and then `no-such-exit on "climb sliver"`, and the honest reading
-- is that the sliver had moved rather than that the road was imaginary.
INSERT INTO room_sets(name, description) VALUES
  ('region:rift', 'The Rift, where the exits wander.'),
  ('region:hinterwilds', 'The Hinterwilds, where the slivers teleport.');

-- Named exactly, not matched. `LIKE '%Rift%'` also catches *D*rift Cottage in
-- Kraken's Fall and Spind*rift* Sanctuary in Icemule -- twelve rooms in two
-- towns, told their exits wander. Every refusal in them would then read as
-- "not yet" forever, and the router would keep walking into a wall it had been
-- instructed to be patient with.
INSERT INTO room_set_terms(set_name, seq, op, key, value) VALUES
  ('region:rift', 0, 'facet', 'location', 'the Rift'),
  ('region:hinterwilds', 0, 'facet', 'location', 'the Hinterwilds');

INSERT INTO transient_exits(set_name, reason) VALUES
  ('region:rift',
   'Doors, mirrors, threads, maws, staircases and fissures wander the rings; see `circuits`.'),
  ('region:hinterwilds',
   'The slivers teleport. Observed 2026-09-13: no-such-exit on "climb sliver" at [Abbey, Teleportation Chamber].');
