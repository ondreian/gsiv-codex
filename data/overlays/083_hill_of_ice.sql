-- The last two steps between the world and the Rift.
--
-- Everything from `[Breach, Cavern of Ages]` onward works: the sphere is
-- ungated, the water tunnel rides, the Lake of Tears is walkable. What is not
-- is the way *in*. The whole Cavern of Ages -- rooms 4562001 through 4562045 --
-- has no published edge entering it from anywhere in the world, and no
-- published edge to 4562020, which is where the half holding the sphere
-- begins. Two doors, and behind them the only road to the Rift that does not
-- go through Symbol of Seeking.
--
-- One of the two is published here. The other is named and left open, because
-- publishing it would need a primitive nobody has yet.

-- 1. The crown at the Top of the World.
--
--    ;e unless (move 'go door'); push_result = dothistimeout 'push tine', 10,
--      /the stone ring surrounding the crown rotates, finally stopping with the
--       tine set with the <one of six stones> aligned with the word
--       <Honor|Truth|Piety|Humility|Faith|Courage>|You grasp the top tine and
--       try to turn it, but it won't even budge./ until ...;
--      ...cast 100 mana of anything at 'crown'...; fput 'release';
--      fput 'touch crown'; fput 'say Aenatumgana'; end; move 'go door'
--
-- The road is `go door`. Everything else in that line is what you do when the
-- door does not open: rotate the ring until a virtue lines up, pour a hundred
-- mana into the crown a spell at a time, touch it, and name the place.
--
-- **Published without the puzzle, and deliberately.** The charge is a property
-- of the crown, not of the character -- whoever passed last left it as they
-- found it -- so `go door` is the move and the puzzle is a recovery. Modelling
-- it as a prelude would be worse than leaving it out: a prelude runs every
-- time, and `push tine` on a crown that is already aligned rotates it *off*
-- alignment, so the one thing an unconditional prelude reliably does here is
-- shut a door that was open.
--
-- What a character walking into a closed door gets today is a refused move and
-- a report naming the room. That is honest, it costs one command, and they are
-- one step from where they came in. Open questions, for someone who can go and
-- look: does the crown hold its charge between visitors, and what does the
-- game say when the door will not open? The second is a failure class the
-- moment somebody reads it.
--
-- The hundred mana is beyond travel in any case. A script cannot see the game's
-- own text and cannot know which spells the character has, so "cast anything,
-- a hundred mana's worth" is not something a walker can be asked to do.
INSERT OR REPLACE INTO edge_overlays(from_uid, to_uid, command, class, time_ms, source, note) VALUES
  (4561131, 4562001, 'go door', 'walk', 12000, 'manual',
   'The door under the crown of Aenatumgana. Open when the crown is charged, which is a property of the crown and not of the character. Lich recharges it -- push tine until a virtue aligns, cast 100 mana at the crown, touch crown, say Aenatumgana -- and that recovery is not published: it needs spells a walker cannot choose, and pushing the tine on an aligned crown would shut a door that was open.');

INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4561131, 4562001,
   ';e unless (move ''go door''); push tine ... cast 100 mana at ''crown'' ... touch crown; say Aenatumgana; end; move ''go door''',
   'overlay',
   'the road is `go door`; the recharge behind it is not published -- see 083_hill_of_ice.sql');

-- 2. The Stairs of Ice. Not published, and here is exactly why.
--
--    ;e clear; put 'look'; loop { line = get
--      if line =~ /a flight of (ascending|descending) steps (curving along|
--        leading straight through) the (northern|eastern|southern|western)
--        wall, <the same three groups, three more times>./
--      if $3 == 'northern' ; move 'climb steps'
--      elsif $6 == 'northern' ; move 'climb second steps'
--      elsif $9 == 'northern' ; move 'climb third steps'
--      elsif $12 == 'northern'; move 'climb fourth steps'
--      ...
--
-- Four flights, four rooms, one per wall. Which flight is *first* changes, so
-- the command that reaches a given room changes with it: you look, you find
-- which position your wall is in, and you climb that ordinal.
--
--   4562019 --northern--> 4562020   <- [Altar of the Elder], and the sphere
--   4562019 --eastern---> 4562015
--   4562019 --southern--> 4562014
--   4562019 --western---> 4562013
--
-- Nothing in this schema can say that. An `edge_overlay` is one command to one
-- room, and here the command is chosen from a reply. A `circuit` is a ring
-- ridden until an exit turns up, and this is not a ring -- you are back where
-- you started after every wrong guess, and the four wrong guesses are four
-- real rooms rather than a loop. A connector `cycle` re-sends one command and
-- compares the reply against destination titles; this sends `look` once and
-- reads an ordering out of it.
--
-- What it needs is a step action that sends a command, matches a pattern
-- against the reply, and picks one of several commands by which group matched
-- -- a sibling of `cycle`, on the same host-side footing, because reading the
-- game's own words is something only the client can do. Until then the reason
-- says so rather than implying nobody looked.
--
-- Guessing is not a substitute. Publishing four edges that each claim to reach
-- 4562020 would be four lies, three of which strand somebody in a room they
-- did not choose.
UPDATE script_edge_disposition
   SET reason = 'the four flights are ordered differently every time: `look` lists them, each is on a named wall, and the command is `climb [second|third|fourth] steps` by which position your wall came in. The walls map north->4562020 (the Altar of the Elder, and the sphere beyond it), east->4562015, south->4562014, west->4562013. Needs a step action that picks a command from a matched reply -- no overlay, circuit or cycle can express it. See 083_hill_of_ice.sql.'
 WHERE from_uid = 4562019
   AND to_uid IN (4562013, 4562014, 4562015, 4562020);
