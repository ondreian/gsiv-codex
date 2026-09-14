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
-- INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
--   ('ice-slip-resolve', 0, 0, 'preference',  'ice_mode', 'eq',  'wait'),
--   ('ice-slip-resolve', 1, 0, 'skill',       'survival', 'lt',  '50'),
--   ('ice-slip-resolve', 2, 0, 'encumbrance', '',         'gte', '50');

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
--   ('ice-slip-resolve', 'delay_ms', 6000);

-- Water Walking (spell 112). Without it you swim, which is neither slower nor
-- forbidden -- it is a different command, carried on `edge_conditions.instead`
-- because the alternative differs per edge: `swim west` here, `swim north`
-- next door.
INSERT INTO conditions(id, description) VALUES
  ('no-water-walking', 'the character is not under Water Walking (spell 112)');
INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
  ('no-water-walking', 0, 0, 'spell', 'Water Walking', 'absent', '');

-- # ice-slip-resolve is known and not published
--
-- The Sleeping Lady's 21 ice edges wait six seconds rather than four, ignore
-- the run preference, and do not accept Haste as an excuse for low Survival.
-- That is real and worth writing down, and it is commented out rather than
-- published because **nothing attaches it to an edge**, and a condition
-- referenced by nothing forbids nothing -- which is precisely how the Voln
-- gate shipped offering a rank-26 road to everybody.
--
-- The reason it attaches to nothing is a shape the extractor does not handle.
-- These scripts do not end in a move; they end in a *recovery*:
--
--   result = fput 'down'
--   if result =~ /^Rushing heedlessly/
--     haste.cast if haste.known? && haste.affordable? && !haste.active?
--     fput 'stand'
--   end
--
-- `looks_like_movement` wants the last statement to be the move, so all 21 are
-- dropped -- and dropped *without* a `script_edge_disposition` row, which is
-- the second bug: that table exists so nothing vanishes without saying so.
--
-- Three things to do, in order:
--   1. teach the extractor the move-then-recovery shape, and make sure
--      anything it declines lands in the audit;
--   2. add "Rushing heedlessly" to the `slipped` failure class, where the
--      recovery already belongs -- the walker stands up and re-tries without
--      the edge needing to say so;
--   3. attach this condition, and uncomment it.
--
-- Until then these 21 edges are published with no pause at all, which is a
-- hazard worth stating plainly rather than papering over with a condition
-- that covers nothing.
