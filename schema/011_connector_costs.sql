-- What a mechanism spends, besides time.
--
-- `connector_destinations.cost_ms` is time, and time was the only cost Lich's
-- `timeto` could express -- which is why every consumable became a per-user
-- custom script. The router has carried a resource vector since the beginning
-- and enforces a budget against it; there was simply no way to *publish* one.
--
-- That gap is not academic. Four of the twenty connectors here spend
-- something:
--
--   Symbol of Seeking   favor       Voln favor, earned by killing undead
--   the portmasters     silver      2,000 to the attendant on some piers
--   periapt to Sanctum  charges     charges earned by hunting
--   wildling teleport   charges     gigas artifact charges
--
-- A router that cannot see these plans the fast road every time and strands
-- whoever follows it at the far end with nothing left to come back on. That is
-- worse than a slow route, because the character is somewhere dangerous when
-- they find out.
--
-- Resource names are open, exactly as on the router's side: `mana`, `stamina`,
-- `spirit` and `silver` fill from the session automatically and anything else
-- is a counter somebody declares. Teaching the engine `favor` needs no schema
-- change, only a row.
CREATE TABLE connector_costs (
    connector_id TEXT    NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    -- Matches `connector_destinations`: a cost may be per-destination or
    -- charged once for the mechanism whatever it reaches.
    kind         TEXT    NOT NULL,
    to_uid       INTEGER NOT NULL,
    resource     TEXT    NOT NULL,
    -- REAL, because a cost need not be whole -- a share of a charge, a
    -- fraction of a stamina bar.
    amount       REAL    NOT NULL,
    PRIMARY KEY (connector_id, kind, to_uid, resource)
) WITHOUT ROWID;
