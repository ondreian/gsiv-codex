-- The Scatter's way out is a crack, and a crack has to be widened.
--
-- `078_rift.sql` publishes the fissure rings with a bare `go fissure`, because
-- that is the `move` at the end of the mapdb's body. Everything before the move
-- was dropped, and everything before the move is the point:
--
--   until checkloot.include?('fissure'); move dirs[index]; index += 1; ... end
--   5.times { ... result = dothistimeout 'push fissure', 3, /.../
--             break if result =~ /^A wide fissure cannot be opened any farther/ }
--   move 'go fissure'
--
-- The ring is the `until` loop and was published. The pushing was not, so a
-- character rides the Scatter until a fissure appears, walks into a crack too
-- narrow to enter, and stays in the Scatter.
--
-- Fixed here rather than in the generator because `078_rift.sql` is generated
-- and hand-edits to it are reverted by the next `./scripts/generate.sh`. The
-- generator should learn to read a prelude out of a circuit body; until it
-- does, this is the correction and it survives a regeneration.
UPDATE circuit_exits
   SET prelude_id = 'widen-the-fissure'
 WHERE noun = 'fissure'
   AND circuit_id LIKE 'rift:fissure:%';
