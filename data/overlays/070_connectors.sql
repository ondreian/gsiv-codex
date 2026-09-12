-- Mechanisms that are not walking.
--
-- Each of these is a road the mapdb holds as a `;e` Ruby program, which the
-- importer drops -- so the regions they reach are dark. Measured, before any
-- of this existed:
--
--   the shadow of the Sanctum     2 of 78 rooms reachable
--   the Rift                      0 of 232
--   Kharam-Dzu                    0 of 373
--   the Pinefar forests           0 of 59
--
-- A connector is the same road, written as data: where it works from, where it
-- goes, what to send, and what the game says back.

-- ---------------------------------------------------------------------------
-- The Sanctum of Scales, by periapt
-- ---------------------------------------------------------------------------
--
-- Nobody ever wrote this one, so everybody who wanted it patched their own
-- mapdb at runtime. Ported from a working Lich patch
-- (`lich-scripts/sanctum-mapdb-patch.lic`), which is the honest provenance:
-- the mechanism has been known for years and has never been writable down.
--
-- Two rooms, both tagged `sanctum` in the mapdb, and the link runs both ways:
--
--   4900340  [Fangs of the Serpent, Gateway]  the southern part of Solhaven
--   4216057  [Subterrain, Pit]                the shadow of the Sanctum
--
-- Rub the periapt and a portal appears; walk into the portal. The portal is a
-- room object rather than an exit, and it may already be there -- somebody
-- else's, which works just as well. That is the client's business; the data
-- says what to send.

INSERT INTO room_sets(name, description) VALUES
  ('sanctum-anchors', 'The two rooms a small bone periapt joins.');
INSERT INTO room_set_terms(set_name, seq, op, value) VALUES
  ('sanctum-anchors', 0, 'room', '4900340'),
  ('sanctum-anchors', 1, 'room', '4216057');

INSERT INTO connectors(id, origin_mode, origin_set, overhead_ms, description) VALUES
  ('periapt:sanctum', 'only', 'sanctum-anchors', 5000,
   'A small bone periapt: rub it for a viridian portal, and step through.');

INSERT INTO connector_destinations(connector_id, kind, to_uid, cost_ms) VALUES
  ('periapt:sanctum', 'fixed', 4900340, 0),
  ('periapt:sanctum', 'fixed', 4216057, 0);

INSERT INTO connector_steps(connector_id, kind, to_uid, seq, command, expect, timeout_ms) VALUES
  -- Lich's own regex accepts three replies: the portal forms, the periapt
  -- fizzles, or one is already up. Only the first is success; the third is
  -- success by somebody else's effort, and the client tells them apart by
  -- whether a portal is in the room afterwards.
  ('periapt:sanctum', 'fixed', 4900340, 0, 'rub #{item}',
   'spiraling out to form a swirling viridian portal hanging in midair', 5000),
  ('periapt:sanctum', 'fixed', 4900340, 1, 'go #{portal}', '', 3000),
  ('periapt:sanctum', 'fixed', 4216057, 0, 'rub #{item}',
   'spiraling out to form a swirling viridian portal hanging in midair', 5000),
  ('periapt:sanctum', 'fixed', 4216057, 1, 'go #{portal}', '', 3000);

-- No periapt, no road. This is Lich's `Periapt.exists? ? 5 : nil` said once
-- instead of twice per direction -- and `nil` is exactly `forbid`.
INSERT INTO conditions(id, description) VALUES
  ('has-periapt', 'the character carries a small bone periapt');
INSERT INTO condition_terms(condition_id, grp, seq, subject, key, op, value) VALUES
  ('has-periapt', 0, 0, 'item', 'small bone periapt', 'absent', '');
INSERT INTO condition_effects(condition_id, effect, amount) VALUES
  ('has-periapt', 'forbid', 0);
INSERT INTO connector_conditions(connector_id, condition_id) VALUES
  ('periapt:sanctum', 'has-periapt');
