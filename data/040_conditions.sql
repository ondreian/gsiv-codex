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
   'Pause before crossing, or slip. 150 edges across six regions: Griffin''s Keen, the Icemule Trail, Ta''Vaalor, Zul Logoth, the Ice Plains, the southern snowfields.'),
  ('ice-slip-resolve',
   'The Sleeping Lady variant, 21 edges. Waits 6s not 4s, ignores the run preference, and Haste does not excuse low Survival. Whether that is a harder mountain or a newer author is unresolved.');

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
INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
  ('ice-slip-resolve', 0, 0, 'preference',  'ice_mode', 'eq',  'wait'),
  ('ice-slip-resolve', 1, 0, 'skill',       'survival', 'lt',  '50'),
  ('ice-slip-resolve', 2, 0, 'encumbrance', '',         'gte', '50');

INSERT INTO condition_effects(condition_id, effect, amount) VALUES
  ('ice-slip',         'delay_ms', 4000),
  ('ice-slip-resolve', 'delay_ms', 6000);
