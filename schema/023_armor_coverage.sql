-- World, slice 23: what an armor sub-group covers.
--
-- `INSPECT` does not name the sub-group. Measured on GST:
--
--   Your careful inspection of some spiked brigandine armor allows you to
--   conclude that it is scale armor that covers the torso, arms, legs, neck,
--   and head.
--
-- The type and the coverage, and within a type the coverage *is* the
-- sub-group: scale over the torso alone is a leather breastplate (9), over
-- torso, arms, legs and head it is brigandine (12). So the one fact that turns
-- the game's answer into a sub-group is coverage, and it was the one fact this
-- table lacked.
--
-- A column rather than a rule on the sub-group number. `asg % 4` gets every
-- type right except cloth, where robes are sub-group 2 and cover everything --
-- and a rule with an exception is a column written the hard way. Values from
-- lich-5's `ArmorStats.find_coverage`.
ALTER TABLE armor_subgroups ADD COLUMN coverage TEXT NOT NULL DEFAULT ''
  CHECK (coverage IN ('', 'torso', 'torso_and_arms', 'torso_arms_and_legs',
                      'torso_arms_legs_and_head'));
