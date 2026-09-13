-- Preludes ported from the mapdb. See schema/008_preludes.sql.

INSERT INTO conditions(id, description) VALUES
  ('needs-to-kneel',
   'Not already kneeling, and too tall to fit through without. Dwarves, Halflings and Gnomes never need to.');

-- posture != kneeling AND race != each of the small races.
--
-- Three `ne` terms rather than a `not_in` operator: DNF expresses it exactly as
-- written, and a new operator earns its place when something cannot be said
-- without one.
INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
  ('needs-to-kneel', 0, 0, 'posture', '',     'ne', 'kneeling'),
  ('needs-to-kneel', 0, 1, 'stat',    'race', 'ne', 'Dwarf'),
  ('needs-to-kneel', 0, 2, 'stat',    'race', 'ne', 'Halfling'),
  ('needs-to-kneel', 0, 3, 'stat',    'race', 'ne', 'Gnome');

INSERT INTO preludes(id, description, condition_id) VALUES
  ('search-for-the-exit',
   'The exit is not visible until you look for it. 72 edges, measured; unconditional, because looking always works.',
   NULL),
  ('kneel-to-fit',
   'A low passage. 39 edges, measured.',
   'needs-to-kneel');

INSERT INTO prelude_steps(prelude_id, seq, command, expect) VALUES
  ('search-for-the-exit', 0, 'search', ''),
  ('kneel-to-fit',        0, 'kneel',  'You kneel');
