-- Conditions ported from the mapdb, faithfully.
--
-- Faithfully means: each variant as it actually reads, with its own constants,
-- even where two look like the same rule. The mapdb is crowd-sourced and
-- unupdatable in place -- a rule is pasted onto every edge it governs -- so
-- somebody improving twenty edges leaves the other hundred and fifty alone.
-- Merging on sight would bake a guess into the data.
--
-- `codex conditions` ranks the pairs that might be one rule. Narrowing happens
-- there, with somebody who knows the game, and lands as its own change.

INSERT INTO conditions(id, description) VALUES
  ('ice-slip',
   'Pause before crossing, or slip. 150 edges across six regions: Griffin''s Keen, the Icemule Trail, Ta''Vaalor, Zul Logoth, the Ice Plains, the southern snowfields.');

-- ice_mode == 'wait'
--   OR (ice_mode != 'run' AND encumbrance > 50)
--   OR (ice_mode != 'run' AND survival < 50 AND Haste inactive)
INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
  ('ice-slip', 0, 0, 'preference',  'ice_mode', 'eq',     'wait'),

  ('ice-slip', 1, 0, 'preference',  'ice_mode', 'ne',     'run'),
  ('ice-slip', 1, 1, 'encumbrance', '',         'gt',     '50'),

  ('ice-slip', 2, 0, 'preference',  'ice_mode', 'ne',     'run'),
  ('ice-slip', 2, 1, 'skill',       'survival', 'lt',     '50'),
  ('ice-slip', 2, 2, 'spell',       'Haste',    'absent', '');

-- ice_mode == 'wait' OR survival < 50 OR encumbrance >= 50
--
-- Note `>=` where the other says `>`. A boundary that moves by one between two
-- copies of a rule is exactly the artifact the shortlist hunts for, so it is
-- recorded as written rather than quietly aligned.
INSERT INTO conditions(id, description) VALUES
  ('ice-slip-resolve',
   'Pause before crossing, or slip. The Sleeping Lady variant: 21 edges on the Northern and Southern Slopes, which wait six seconds rather than four, ignore the run preference, and do not accept Haste as an excuse for low Survival.');
INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
  ('ice-slip-resolve', 0, 0, 'preference',  'ice_mode', 'eq',  'wait'),
  ('ice-slip-resolve', 1, 0, 'skill',       'survival', 'lt',  '50'),
  ('ice-slip-resolve', 2, 0, 'encumbrance', '',         'gte', '50');

-- 3500, measured rather than copied. 190 crossings on 4044101 <-> 4044102 on
-- the Icemule Trail: at Survival 0 the fall rate slides 100% -> 83% -> 33% ->
-- 17% -> 0% between 2.5 and 3.25 seconds, and thirty-two crossings at 3.25s or
-- more produced no fall. 3500 is that boundary with a quarter second on it.
--
-- The mapdb's 4000 was close, which is not what the first pass at this
-- concluded: that harness stamped arrival 1.07s late and briefly published
-- 2500, a number inside the band that still falls.
--
-- It is a roll rather than a gate, so this buys margin and not certainty, and
-- the `slipped` failure class still has to catch the ones that get through.
--
-- The `survival < 50` escape above is sound but its reason is not: 202 ranks
-- still falls one crossing in thirteen when hurried. What makes skipping right
-- is that a fall costs six seconds of roundtime, so `P(fall) x 6s` beats the
-- pause well before the slipping stops.
--
-- docs/ice-slip-measured.md in urnon has the table.
INSERT INTO condition_effects(condition_id, effect, amount) VALUES
  ('ice-slip', 'delay_ms', 3500);
-- 6000 as the mapdb writes it, not 3500. The 3500 above was measured on the
-- Icemule Trail and these are a different set of edges with their own constants
-- -- and merging on sight is exactly what the header of this file forbids.
-- Somebody should measure the Sleeping Lady the way the Icemule Trail was
-- measured; until then it waits as long as Lich waits.
INSERT INTO condition_effects(condition_id, effect, amount) VALUES
  ('ice-slip-resolve', 'delay_ms', 6000);

-- Water Walking (spell 112). Without it you swim, which is neither slower nor
-- forbidden -- it is a different command, carried on `edge_conditions.instead`
-- because the alternative differs per edge: `swim west` here, `swim north`
-- next door.
INSERT INTO conditions(id, description) VALUES
  ('no-water-walking', 'the character is not under Water Walking (spell 112)');
INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
  ('no-water-walking', 0, 0, 'spell', 'Water Walking', 'absent', '');

-- # ice-slip-resolve, and the three edges it was hiding
--
-- This condition sat commented out for a long time with a note saying nothing
-- attached it to an edge, and a condition referenced by nothing forbids nothing
-- -- which is how the Voln gate shipped offering a rank-26 road to everybody.
--
-- What attached to nothing was the *shape*. These scripts do not end in a move,
-- they end in a recovery:
--
--   result = fput 'down'
--   if result =~ /^Rushing heedlessly/
--     haste.cast if haste.known? && haste.affordable? && !haste.active?
--     fput 'stand'
--   end
--
-- Every recogniser wanted `move 'X'`, so all 21 were dropped. `captured_fput`
-- in the extractor now reads the assignment as the move, which is what the
-- assignment is for: a bare `fput 'stand'` is housekeeping, but capturing the
-- reply and testing it means the body cares what the game said, and here what
-- it says is whether you fell.
--
-- **Three of those 21 were the only walking road between Pinefar and the rest
-- of Elanthia.** The Northward Trail runs Abbey -> Sleeping Lady Mountains ->
-- Northern Slopes -> Pinefar, and it is ordinary walking the whole way except
-- at the Timberline, where 4560049, 4560050 and 4560051 are joined by these.
-- Without them the Trading Post, Mount Aenatumgana and the Cavern of Ages --
-- 288 rooms, and the entire slow road to the Rift -- hung off a 3,000 silver
-- caravan fare that a router would take for an hour when the walk is minutes.
--
-- The recovery still has no pattern of its own, and that is a known gap rather
-- than an oversight. `Rushing heedlessly` belongs in `slipped`, whose remedy --
-- wait out the roundtime, stand, wait again -- is exactly right for it. But
-- `slipped` is one of Lich's classes, `061_failure_patterns.tsv` carries no
-- provenance per row, and `the_committed_vocabulary_is_what_lich_says_today`
-- compares our Lich-derived half against `global_defs.rb` line for line. A
-- pattern of ours inside a class of theirs reads to that test as Lich having
-- changed under us, which is the one thing it exists to catch.
--
-- Fixing it properly means a `source` column on `failure_patterns` -- a
-- migration, 125 rows rewritten, and the generator taught to emit it -- so that
-- a measured pattern can sit in a Lich class and say where it came from. Worth
-- doing; not worth doing inside an unrelated change.
--
-- Nothing is broken meanwhile. An unrecognised slip is an unrecognised failure,
-- and the walker answers it the way it answers any other: it re-plans from the
-- room it is standing in, which after a fall is the room it started in, and
-- tries again. The pause above is what usually makes that moot.
