-- Which Lich tag means which facet.
--
-- Curated, not derived: the mapdb says a room is tagged `gemshop`, and that
-- this means a `gem_dealer` is somebody's decision. Keeping it as data rather
-- than code is what lets the decision be reviewed in a diff.
--
-- Two modes, because tags carry their detail two ways:
--   exact   the whole tag is the signal. `bank` -> bank.
--   prefix  the tag's remainder is the detail. `urchin guide bank` ->
--           urchin_guide(bank), `meta:teleport:fwi` -> teleport(fwi).
CREATE TABLE tag_map (
    pattern TEXT PRIMARY KEY,
    type    TEXT NOT NULL REFERENCES facet_types(type),
    mode    TEXT NOT NULL CHECK (mode IN ('exact', 'prefix')),
    -- Used by `exact`; a prefix rule derives its detail from the tag.
    detail  TEXT NOT NULL DEFAULT ''
);

CREATE INDEX idx_tag_map_mode ON tag_map(mode);
