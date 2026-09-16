-- The road to the Cavern of Ages, and so to the Rift.
--
-- The Rift is entered from `[Breach, Cavern of Ages]` through the sphere, and
-- `sphere:rift` has no conditions at all -- it was never gated on Voln.
-- Symbol of Seeking is simply the fast way; the slow way is a hundred and
-- thirty-five steps, and it was unwalkable because seven of them were
-- unpublished.
--
-- Two of those seven are here, because the extractor cannot derive either and
-- should not be taught to guess at them.

-- 1. The Water Tunnel is a circuit, outside the Rift.
--
--    ;e until checkpaths.include?('up'); fput 'swim ' + ['northwest','northeast'][rand(2)]; waitrt?; end
--
-- Swim one of two ways at random until `up` appears, then take it. That is
-- exactly the shape the Rift's wandering doors have -- ride until the way out
-- turns up -- which is worth noticing: circuits are not a Rift feature, they
-- are how this game writes "the exit is not always in the same place".
--
-- The random choice becomes alternation. A ring is ridden in order and the
-- two directions are interchangeable, so alternating covers the same ground
-- without needing randomness the walker does not have.
INSERT OR REPLACE INTO circuits(id, to_uid, description) VALUES
  ('cavern:water-tunnel:4562040', 4562039,
   'Swim between two passages until the way up appears; watch for up.');
-- **Twenty, not four.** Lich loops `until checkpaths.include?('up')` with no
-- bound at all, and it needs one: the tunnel is turbulent and a swim often
-- leaves you in the room you started in, so "one lap" is not four swims, it is
-- however many the water allows. Four gave up while still in the tunnel and
-- stranded a character in a room the graph does not know -- 4562041 and 4562042
-- are the same Lich room as 4562040 and have no edges of their own.
--
-- Twenty is a bound rather than a guess at the distance: far past what the
-- three-room tunnel needs, and still an end, because a ring whose exit never
-- appears is a ring that is not working today and swimming it forever looks
-- exactly like swimming it successfully.
INSERT OR REPLACE INTO circuit_moves(circuit_id, seq, command) VALUES
  ('cavern:water-tunnel:4562040', 0, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 1, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 2, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 3, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 4, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 5, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 6, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 7, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 8, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 9, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 10, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 11, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 12, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 13, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 14, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 15, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 16, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 17, 'swim northeast'),
  ('cavern:water-tunnel:4562040', 18, 'swim northwest'),
  ('cavern:water-tunnel:4562040', 19, 'swim northeast');
-- `up` is an obvious path rather than an object, which is what
-- `checkpaths.include?` was asking. The walker matches a watch noun against
-- the room's exits as well as its description, so this works as written.
INSERT OR REPLACE INTO circuit_exits(circuit_id, noun, command) VALUES
  ('cavern:water-tunnel:4562040', 'up', 'up');
-- **All three uids, not one.** Lich room 2642 carries uid
-- `[4562040, 4562041, 4562042]` -- one room in its map, three rooms to the
-- game -- and the importer projects a Lich room onto its *first* uid. So 4562041
-- and 4562042 exist in the world and in no table here, and a character standing
-- in one of them is nowhere as far as the router is concerned. Measured by
-- swimming it: 4562042 -> 4562041 -> 4562040 -> [Lake of Tears].
INSERT OR REPLACE INTO circuit_entries(circuit_id, room_uid, seq) VALUES
  ('cavern:water-tunnel:4562040', 4562040, 0),
  ('cavern:water-tunnel:4562040', 4562041, 0),
  ('cavern:water-tunnel:4562040', 4562042, 0);

-- 1b. The same tunnel, swum the other way.
--
--    ;e until checkpaths.include?('up'); fput 'swim ' + ['southwest','southeast'][rand(2)]; waitrt?; end
--
-- The Water Tunnel has two rings and only one was published, which made the
-- cavern a one-way trip: you could swim from the Pool out to the Lake of Tears
-- and never back. That is the whole return journey -- the Lake is the side the
-- Breach and the sphere are on, and the Pool is the side the Altar of the Elder
-- and the way home are on.
--
-- Identical in shape to the northward ring and alternating for the same reason:
-- a ring is ridden in order and the two directions are interchangeable, so
-- alternation covers the same ground without randomness the walker does not
-- have.
INSERT OR REPLACE INTO circuits(id, to_uid, description) VALUES
  ('cavern:water-tunnel:south', 4562044,
   'Swim between the two southward passages until the way up appears; watch for up.');
INSERT OR REPLACE INTO circuit_moves(circuit_id, seq, command)
SELECT 'cavern:water-tunnel:south', seq, command
  FROM circuit_moves
 WHERE circuit_id = 'cavern:water-tunnel:4562040';
UPDATE circuit_moves
   SET command = replace(command, 'north', 'south')
 WHERE circuit_id = 'cavern:water-tunnel:south';
INSERT OR REPLACE INTO circuit_exits(circuit_id, noun, command) VALUES
  ('cavern:water-tunnel:south', 'up', 'up');
INSERT OR REPLACE INTO circuit_entries(circuit_id, room_uid, seq) VALUES
  ('cavern:water-tunnel:south', 4562040, 0),
  ('cavern:water-tunnel:south', 4562041, 0),
  ('cavern:water-tunnel:south', 4562042, 0);

INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4562040, 4562044,
   'until checkpaths.include?(''up''); fput ''swim '' + [sw,se][rand(2)]',
   'connector',
   'published as circuit cavern:water-tunnel:south -- ride until the way up appears');

INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4562040, 4562039,
   'until checkpaths.include?(''up''); fput ''swim '' + [nw,ne][rand(2)]',
   'connector',
   'published as circuit cavern:water-tunnel:4562040 -- ride until the way up appears');

-- 2. The icy ledge above Aenatumgana.
--
--    ;e 8.times { ...cast Celerity if known...; dothistimeout 'search', 3,
--      /discover a narrow icy ledge!|don't find anything of interest/;
--      break if move 'go ledge' }
--
-- Search until the ledge is found, then take it. The Celerity casting is an
-- optimisation -- it makes searching faster, and the edge works without it --
-- so it is not part of what the road *is*.
--
-- The prelude already exists and says the same thing about seventy-two other
-- edges: the exit is not visible until you look for it.
INSERT OR REPLACE INTO edge_overlays(from_uid, to_uid, command, class, time_ms, source, note) VALUES
  (4561129, 4561130, 'go ledge', 'walk', 8000, 'manual',
   'Searched for, not seen: the ledge appears only after searching. Lich tries eight times and casts Celerity to hurry the search; neither is part of the road.');
INSERT OR REPLACE INTO edge_preludes(from_uid, to_uid, command, prelude_id) VALUES
  (4561129, 4561130, 'go ledge', 'search-for-the-exit');
