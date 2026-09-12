# World database: scoping

Input for the separate data repo. No implementation proposed here — this is
what has to be settled before there is one.

Written after measuring the current store, because the thing that prompted it
is that we have been building several systems that shadow each other without
noticing.

## 1. What exists today, measured

Room-grouping systems in the live store:

| System | Tables | Rows | Files referencing |
| --- | --- | --- | --- |
| Regions | `regions`, `region_seeds`, `region_boundaries`, `region_anti_seeds`, `room_regions` | **0** | 6 |
| Realms | `realms`, `realm_patterns`, `room_realms` | 5 / 0 / **0** | 5 |
| Zones | `zones`, `zone_lich_names` | 3 / 8 | 5 |
| Room sets | proposed by the digraph design | — | 0 |

**Three grouping systems exist in schema and code, all effectively empty, and
the design proposes a fourth to replace them.** `World.room_region` is
populated by flood at load and never read from `room_regions` at all, so the
region tables are dead storage behind a live in-memory concept.

Meanwhile the one table carrying data:

| | |
| --- | --- |
| `room_pois` | 374,411 rows |
| of which `facet='poi'` | 374,411 — the column has one value in practice |
| distinct `kind` values | 20, of which `herb` is 371,202 |

`facet` was meant to separate poi / property / route / resource. It does not,
in the data. But `World` splits that one table into **four** in-memory maps —
`pois`, `properties`, `routes`, `tags` — all of the same shape
`Uid -> Vec<(String, String)>`.

So: one table, one used discriminator, four in-memory views of it, plus three
dead grouping systems. That is the complexity being added without care.

## 2. The collapse

**POIs are one relational table and `type` is a column.** `herb`, `bank`,
`node`, `urchin_access` are rows, not tables and not facets. This is already
nearly true; what is missing is deleting the parallel structures rather than
adding a fifth.

**A room set is a query, not a new primitive.** Of the nine `RoomSetDef` node
types in the digraph design, eight are relational algebra SQLite already
implements:

| Design node | SQL |
| --- | --- |
| `rooms { uids }` | `WHERE uid IN (...)` |
| `title { pattern }` | `WHERE title REGEXP ?` |
| `has_poi`, `has_tag`, `has_property` | `WHERE type = ? AND value = ?` |
| `in_set { name }` | a view reference |
| `union` / `intersect` / `difference` | `UNION` / `INTERSECT` / `EXCEPT` |
| `flood { seeds, boundaries, anti_seeds }` | **`WITH RECURSIVE`** |

There is no functional difference between "the set of rooms with a bank" and
"the herbs in a room" — same table, same query shape, different projection.
A hand-rolled evaluator for that grammar would be a query language
reimplementing the one underneath it.

`flood` is the only node with real content, and it is a recursive CTE with a
stop condition. The four leak checks are property tests over its result.

**Open:** if sets are views, `room_sets` is a table of view definitions —
which is either SQL text (powerful, unreviewable, injectable) or a restricted
serialized form (reviewable, needs a compiler). That choice is the main
unresolved question in this section.

## 3. world + userland = runtime

Two databases, `ATTACH`ed, with the runtime reading the union.

| Layer | Holds | Written by |
| --- | --- | --- |
| **world** | canonical entities: rooms, edges, POIs, verbs, set definitions | the data repo, shipped |
| **userland** | custom verbs, custom routing, private sets, character facts | the player, locally |

Questions this raises, none of them settled:

- **Precedence.** When both define `bank`, which wins? Presumably userland,
  but "override" and "extend" are different operations and a list-valued thing
  like a set needs to say which it is.
- **Deletion.** Can userland subtract from world, or only add? Subtraction
  needs a tombstone shape.
- **Identity.** Do userland rows share the world's id space, or get their own
  range? Sharing invites collision on the next world update; separating means
  every join has to know.
- **Upgrades.** World ships a new version; userland referenced a row that
  moved. What breaks, and how loudly?
- **Read path.** `ATTACH` plus `UNION ALL` per query is uniform but pushes the
  layering into every query. A view per entity hides it in one place.

## 4. Verbs belong in the database

Baked into world, extended by userland. That is the same layering as above and
the first real consumer of it — TUI input autocomplete needs the canonical verb
list, and a player's aliases and script-defined verbs sit on top.

Nothing exists for this today. It is a new entity type, not a new system, which
is the test this scoping is trying to apply to everything.

## 5. DuckDB

Not a dependency yet. Its role is analytics over the corpus and over play
history — read-side, derived, never authoritative. It reads the same SQLite
files rather than owning storage, so nothing in the runtime path depends on it
being present.

**Open:** whether analytics reads the live store directly or an exported
snapshot. The live store is being written by a daemon while a query runs.

## 6. Repo and CI boundary

| Concern | Home |
| --- | --- |
| Entity data, set definitions, verb lists, catalogs | data repo |
| Leak checks on a `flood` | data repo CI — property-based tests |
| Schema migrations for `world` | data repo (it ships the file) |
| Evaluating/attaching at load, routing | urnon |
| Structural validation of what it loads | urnon — a parse, not a property test |

**Open:** how the world database reaches urnon. Checked-in binary, a build
artifact, or a downloaded release? That decides versioning, offline behaviour,
and whether a migration is urnon's problem at all.

## 7. What has to be settled before code

1. The entity shape — one table with a `type`, or a table per type with a
   shared view.
2. Whether a set is a stored view, and in what form.
3. The layering rules in §3: precedence, deletion, identity, upgrade.
4. How the world database is distributed and versioned.
5. Whether the three dead grouping systems are deleted before or during the
   move. They are empty; deleting them first makes the move smaller.

## 8. Recommendation

Delete before adding. The store currently carries three grouping systems with
no rows and one entity table doing all the work. Collapsing to the entity table
and expressing sets as queries over it is a smaller system than today's, not a
larger one — and it is the only version of this that does not make the shadowing
worse.
