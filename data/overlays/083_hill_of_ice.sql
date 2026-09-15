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
-- **Walked to, and it does not open.** On 2026-09-15 a level 100 Ranger stood
-- at the door and did everything the Ruby does, in order:
--
--   go door               -> "The stone door appears to be closed."
--   push tine             -> "...the tine set with the veil iron stone aligned
--                             with the word 'Piety'."   (any of the six counts)
--   prepare 650; cast crown  x2  -> "A soft glow surrounds the bas-relief
--                             crown for a moment, but quickly fades away."
--   look crown            -> "The crown is bathed in a brilliant aura."
--   touch crown           -> "You touch the crown.  It feels hot."
--   say Aenatumgana
--   go door               -> "The stone door appears to be closed."
--
-- Charged to a brilliant aura, and still shut. Sent as one burst the way Lich
-- sends it -- touch, say, go, no pause between -- and still shut. `open door`
-- answers "There doesn't seem to be any way to do that", so the `open-the-way`
-- remedy is wrong for this edge as well.
--
-- So the crown does *not* hold its charge between visitors, and charging it is
-- *not* sufficient. Something else gates this door and the mapdb's Ruby does
-- not know what -- a virtue that has to be chosen rather than landed on, a
-- quest flag, a group, or a script that went stale. That is a question for
-- somebody who knows the game, not something to measure blind, and until it is
-- answered nobody can walk into the Cavern of Ages.
--
-- **The edge stays published anyway**, which is a deliberate choice and not an
-- oversight. There is no second road: refusing it turns the whole cavern into
-- "no route", which tells a reader nothing. Published, a walk arrives at the
-- door and stops on the game's own words -- and "The stone door appears to be
-- closed" already matches the `way-is-closed` pattern
-- `(?:appears|seems) to be closed\.$`, so the failure is named rather than
-- mysterious. A character is one step from where they came in and out nothing
-- but the walk.
--
-- The hundred mana is beyond a walker regardless: a script cannot see the
-- game's own text and cannot know which spells the character has, so "cast
-- anything, a hundred mana's worth" is not something it can be asked to do.
-- And it could never be a prelude -- a prelude runs every time, and `push tine`
-- on an aligned crown rotates it off.
INSERT OR REPLACE INTO edge_overlays(from_uid, to_uid, command, class, time_ms, source, note) VALUES
  (4561131, 4562001, 'go door', 'walk', 12000, 'manual',
   'The door under the crown of Aenatumgana, and the only way into the Cavern of Ages. Measured 2026-09-15: closed, and Lich''s whole recovery -- push tine, 100 mana into the crown, touch crown, say Aenatumgana -- charges it to a brilliant aura without opening it. Something else gates this and the mapdb does not know what. Published because there is no second road and a named refusal beats no route at all.');

INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4561131, 4562001,
   ';e unless (move ''go door''); push tine ... cast 100 mana at ''crown'' ... touch crown; say Aenatumgana; end; move ''go door''',
   'overlay',
   'the road is `go door`; the recharge behind it is not published, and measured on 2026-09-15 the recharge does not open the door either -- see 083_hill_of_ice.sql');

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


-- 3. And the way back out, which mattered more than anything above.
--
-- `4561131 --go door--> 4562001` was published and nothing came back, so the
-- Cavern of Ages was a one-way trip: a character could walk in over the
-- mountain and the router had no route home from any room inside it. That is
-- not a missing shortcut, it is a trap, and it is the reason `travel icemule`
-- answers "no route" for anybody standing in the shrine or the Rift beyond it.
--
-- The road out is the brass ring on its chain -- pull until the stone door
-- grinds up, then walk through -- published here with the `pull-the-ring`
-- prelude. Lich empties its hands first and fills them after; nothing here can
-- say that yet, so a character with both hands full gets told by the game,
-- whose words the `hands-full` class already knows.
INSERT OR REPLACE INTO edge_overlays(from_uid, to_uid, command, class, time_ms, source, note) VALUES
  (4562001, 4561131, 'go door', 'walk', 12000, 'manual',
   'Out of the Hill of Ice: pull the brass ring until the stone door rises, then go door. Lich empties both hands before pulling and refills them after, which nothing here can express -- the hands-full failure class catches the refusal instead.');
INSERT OR REPLACE INTO edge_preludes(from_uid, to_uid, command, prelude_id) VALUES
  (4562001, 4561131, 'go door', 'pull-the-ring');

INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4562001, 4561131,
   ';e if checkleft and checkright; empty_hands; ... dothistimeout ''pull ring'' ... move ''go door''',
   'overlay',
   'published as `go door` with the `pull-the-ring` prelude; the empty_hands/fill_hands around it is not expressible -- see 083_hill_of_ice.sql');
