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

-- The brass ring in the Antechamber of the Hill of Ice.
--
--    ;e if checkleft and checkright; empty_hands; ...; end
--       result = dothistimeout 'pull ring', 8,
--         /the stone door begins to rise ...|it will not budge an inch|you manage
--          to pull it downward some before fatigue forces you to stop/
--       until result =~ /the stone door begins to rise|will not budge an inch/
--       waitrt?; fill_hands if need_fill_hands; move 'go door'
--
-- One prelude and one move, and the only way out of the Cavern of Ages. Pull
-- the ring until the door grinds up, then walk through it. The pull is a retry
-- rather than a single send -- "you manage to pull it downward some before
-- fatigue forces you to stop" means pull again -- and that retry is the
-- client's job, which is why it is one step here.
--
-- Lich empties its hands first and fills them after. That is a real
-- requirement, and it is not written down: the codex has no verb for it and the
-- walker has no hands API, so a character carrying something in both hands will
-- be told so by the game rather than by us. The `hands-full` failure class
-- already catches that line, which is the honest half of the answer.
INSERT INTO preludes(id, description, condition_id) VALUES
  ('pull-the-ring',
   'The stone door of the Hill of Ice is raised by a brass ring on a chain, and stays shut until it is. Pull until it grinds open; fatigue stops you part-way and you pull again.',
   NULL);
INSERT INTO prelude_steps(prelude_id, seq, command, expect) VALUES
  ('pull-the-ring', 0, 'pull ring', 'the stone door begins to rise');
