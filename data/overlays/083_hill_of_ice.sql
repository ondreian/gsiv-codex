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
-- **Walked to, and opened.** On 2026-09-15 a level 100 Ranger stood at the
-- door and did what the Ruby does, in order:
--
--   go door               -> "The stone door appears to be closed."
--   push tine             -> "...the veil iron stone aligned with the word
--                             'Piety'."           <- wrong pair, push again
--   push tine             -> "...the milky white stone aligned with the word
--                             'Piety'."           <- right pair
--   prepare 650; cast crown  x2  -> "A soft glow surrounds the bas-relief
--                             crown..."  (100 mana, any spells)
--   look crown            -> "The crown is bathed in a brilliant aura."
--   touch crown           -> "You touch the crown.  It feels hot."
--   say Aenatumgana
--   go door               -> [Antechamber, Hill of Ice]
--
-- **The pairing is the puzzle and it is easy to miss.** The `until` clause
-- accepts six alignments and only six: reflective glass with Honor, clear
-- crystal with Truth, milky white with Piety, dull grey with Humility,
-- flawless silver with Faith, veil iron with Courage. Each stone must meet its
-- own word. A first attempt here read the loop as "push once, then charge",
-- landed veil iron on Piety, charged the crown to a brilliant aura and found
-- the door still shut -- which looked like a second undiscovered gate and was
-- simply the ring in the wrong place.
--
-- The charge does not persist: the door was closed on arrival, and `open door`
-- answers "There doesn't seem to be any way to do that", so the `open-the-way`
-- remedy is wrong for this edge.
--
-- **Published without the recovery.** The hundred mana is beyond a walker -- a
-- script cannot see the game's own text and cannot know which spells the
-- character has -- and it could never be a prelude, because a prelude runs
-- every time and `push tine` on an aligned crown rotates it off. So `go door`
-- is the road, the recharge is a human's job, and a character who arrives at a
-- cold crown is told by the game: "The stone door appears to be closed" already
-- matches the `way-is-closed` pattern `(?:appears|seems) to be closed\.$`.
INSERT OR REPLACE INTO edge_overlays(from_uid, to_uid, command, class, time_ms, source, note) VALUES
  (4561131, 4562001, 'go door', 'walk', 12000, 'manual',
   'The door under the crown of Aenatumgana, and the only way into the Cavern of Ages. Opens when the crown is charged, which is a property of the crown and not of the character -- measured 2026-09-15: found closed, recharged by hand, walked through. The recharge is push tine until a stone meets its own word, 100 mana cast at the crown, touch crown, say Aenatumgana; it is not published because a walker cannot choose spells and pushing the tine on an aligned crown would shut a door that was open.');

INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4561131, 4562001,
   ';e unless (move ''go door''); push tine ... cast 100 mana at ''crown'' ... touch crown; say Aenatumgana; end; move ''go door''',
   'overlay',
   'the road is `go door`; the recharge behind it needs spells a walker cannot choose -- see 083_hill_of_ice.sql');

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


-- 4. The Stairs of Ice, at last.
--
-- Four flights, one per wall, listed in a different order every time. Measured
-- in the room on 2026-09-15:
--
--   climb northern steps    -> "I could not find what you were referring to."
--   climb ascending steps   -> "I could not find what you were referring to."
--   climb second steps      -> [Altar of the Elder]
--
-- The game has no name for a flight but its position, and the position moves.
-- So the command is published with the position left open, and the walker
-- fills it from the room it is standing in -- `#{ordinal:northern wall}`
-- becomes "", "second ", "third " or "fourth " by counting how many walls are
-- named before that one.
--
-- That the walls map to fixed rooms is the fact worth writing down, and it is
-- the part Lich's script encodes too:
--
--   northern -> 4562020  [Altar of the Elder], and the sphere beyond it
--   eastern  -> 4562015  [Platform of Ice]
--   southern -> 4562014  [Platform of Ice]
--   western  -> 4562013  [Platform of Ice]
--
-- Filled by the guest and not at plan time, deliberately: the order is what it
-- is when you are standing there, and a plan made ten minutes and two hundred
-- steps earlier would carry an ordinal that has since moved.
--
-- These close the loop. Before them the Cavern of Ages and the Rift beyond it
-- could be entered and never left, in either direction, and `travel icemule`
-- from inside answered "no route".
INSERT OR REPLACE INTO edge_overlays(from_uid, to_uid, command, class, time_ms, source, note) VALUES
  (4562019, 4562020, 'climb #{ordinal:northern wall} steps', 'walk', 3000, 'manual',
   'The flight on the northern wall. Which ordinal that is changes every time, so the walker counts it from the room.'),
  (4562019, 4562015, 'climb #{ordinal:eastern wall} steps', 'walk', 3000, 'manual',
   'The flight on the eastern wall.'),
  (4562019, 4562014, 'climb #{ordinal:southern wall} steps', 'walk', 3000, 'manual',
   'The flight on the southern wall.'),
  (4562019, 4562013, 'climb #{ordinal:western wall} steps', 'walk', 3000, 'manual',
   'The flight on the western wall.');

INSERT OR REPLACE INTO script_edge_disposition(from_uid, to_uid, excerpt, disposition, reason) VALUES
  (4562019, 4562020, ';e clear; put ''look''; ... if $3 == ''northern'' move ''climb steps'' elsif ...', 'overlay',
   'published as `climb #{ordinal:northern wall} steps` -- the walker reads the order out of the room'),
  (4562019, 4562015, ';e clear; put ''look''; ... if $3 == ''eastern'' ...', 'overlay',
   'published as `climb #{ordinal:eastern wall} steps`'),
  (4562019, 4562014, ';e clear; put ''look''; ... if $3 == ''southern'' ...', 'overlay',
   'published as `climb #{ordinal:southern wall} steps`'),
  (4562019, 4562013, ';e clear; put ''look''; ... if $3 == ''western'' ...', 'overlay',
   'published as `climb #{ordinal:western wall} steps`');
