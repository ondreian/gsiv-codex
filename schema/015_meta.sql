-- What this file is, written inside the file.
--
-- A published codex travels on its own: downloaded, stored beside other
-- versions, rolled back to. Whoever holds one needs to answer "which is this,
-- and can my engine read it" without a sidecar, a filename convention, or a
-- lookup against a server that may not be reachable.
--
-- Keys written by `codex build`:
--
--   version         the codex release, semver. `0.3.1`
--   schema_version  `PRAGMA user_version` at build time, so a reader can tell
--                   a schema it does not know from a schema it does
--   urnon_min       the oldest engine that can read this, inclusive
--   urnon_max       the first engine that cannot, exclusive. Empty means no
--                   known ceiling
--   built_at        RFC 3339, UTC
--   source          what it was built from
--   notes           what changed, for a human choosing between two of these
--
-- # Why a range and not a single number
--
-- Because both directions happen. A codex using a table an old engine never
-- heard of needs a floor; an engine that drops support for a retired column
-- needs a ceiling. `urnon_max` empty is the normal case and says "nothing has
-- broken this yet" rather than "anything works".
CREATE TABLE codex_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;
