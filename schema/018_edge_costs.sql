-- What an edge costs besides time.
--
-- `connector_costs` has carried silver since the portmasters landed, and edges
-- have never had anywhere to put it -- so a toll gate was a road like any
-- other:
--
--   The town guard steps in front of you and says, "Excuse me, sir, there's an
--   entrance fee to use this gate.  Five silvers please."
--
-- The router already understands resources: `Cost` carries them, a plan sums
-- them, and a request declines a road it cannot pay for. Only the publishing
-- side was missing, so a character with four silver was routed through a gate
-- that wanted five and stopped at a guard.
--
-- Shaped like `connector_costs` on purpose. Both answer the same question, and
-- the resource vocabulary is open for the same reason: mana and stamina fill
-- from vitals, and anything else is whatever the world charges.
CREATE TABLE edge_costs (
    from_uid INTEGER NOT NULL,
    to_uid   INTEGER NOT NULL,
    -- Keyed on the edge's content, like `edge_conditions`, because that is its
    -- identity -- the codex publishes no surrogate.
    command  TEXT    NOT NULL,
    resource TEXT    NOT NULL,
    -- REAL, because a cost need not be whole: a share of a charge, a fraction
    -- of a stamina bar.
    amount   REAL    NOT NULL,
    PRIMARY KEY (from_uid, to_uid, command, resource)
) WITHOUT ROWID;
