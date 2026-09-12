# Roadmap

Where this is driving, so we can tell whether we are getting there.

## The point

A unified data layer for GemStone IV. Lich never had one: it has scripts that
each know one domain, no shared vocabulary between them, and no way for a fact
in one to reference a fact in another. A foe cannot name the rooms it lives in.
A verb cannot say what it operates on. The knowledge exists — it is spread
across a hundred scripts, a wiki, and people's heads.

Four domains, one shape:

| Domain | Entity | Has | Relates to | Effort |
| --- | --- | --- | --- | --- |
| **world** | room | facets — `bank`, `antimagic`, `herb(x)` | rooms, room sets | 3 |
| **verbs** | verb | syntax, help text | — | 2 |
| **foes** | foe | stats, capabilities | room sets | 4 |
| **items** | item | properties, effects | rooms (forageables) | 2 |

Every one of them is *entity → typed attributes → relations to other entities*.
That is what makes "unified" structural rather than aspirational, and it is
precisely the thing Lich's format cannot express.

**Same shape does not mean same table.** These are ordinary normalised
relations — `rooms`, `room_facets`, `foes`, `foe_room_sets` — with real foreign
keys. The alternative, one polymorphic attribute table keyed by
`(entity_type, entity_id)`, buys a single query shape and pays for it with the
one thing this layer exists to provide: **a polymorphic parent cannot have a
foreign key.** A foe referencing a room set is only enforceable if both are
real tables, and that reference is the thing Lich structurally cannot say.

Where a uniform "everything known about X" query is genuinely wanted, that is a
**view** over the union. Integrity in the tables, uniformity in the query.

The collapse already done in urnon is this rule applied once: room facets are
*one* table with a `type` column rather than separate tables for herbs, POIs
and properties. That is normalisation, not polymorphism.

## Order

Dependency, not preference:

1. **world** — in progress, in slices:
   1. **rooms and facets** — done. Three tables, two foreign keys.
   2. **room sets** — done, at the size the evidence justified. Terms are
      unioned; `flood`, `intersect` and `difference` are deferred because they
      have zero consumers, and each is additive when one appears.
   3. **connectors** — done. Four tables, three destination kinds, one
      placeholder. See `docs/connectors.md` and
      `docs/connectors-hard-cases.md`. Nothing is ingested yet: the shape
      exists, the rows do not.
   4. **edges and ingest** — done. `codex build --from <urnon map.db3>` emits
      a database urnon routes on: 28,446 rooms, 62,012 walk edges, 374,425
      facets. Script edges never cross; 2 statically-ambiguous commands are
      excluded and recorded.
   5. **the script-edge burn-down** — coverage as a property test rather than
      a feeling. Needs connector rows, which needs curation.
2. **verbs** — independent, cheapest, and has a waiting consumer: autocomplete
   in the urnon TUI input line, plus help menus.
3. **items** — independent. Plenty of established scripts to harvest; the work
   is encoding effects and properties, not finding the names.
4. **foes** — last, because foes reference room sets. Several existing sources
   of varying quality; a wiki crawl is likely and it is the hardest of the four.

**Verbs are not the entity model.** A verb is a taxonomy, not a row: `STOW`,
`STOW <item>`, `STOW LEFT`, plus aliases and an argument grammar per node. That
is a tree with a grammar hanging off it, and adjacency list, closure table and
materialised path each suit a different access pattern — prefix-walk for
autocomplete versus whole-subtree read for a help menu. It gets designed as a
taxonomy when we reach it rather than assumed to fall out of rooms-with-
attributes.

## What belongs here

A fact that is true for **everyone**. The 13 FWI teleport anchors, a room's
facets, a foe's stats, what `STOW` does.

## What does not

Anything true of **one character at one moment**. Whether this character holds
the trinket, when their urchin lease lapses, where they are standing. That
lives in the client and reaches the router as an offer on a request.

The line is sharp on purpose: a definition true for everyone is data to ship,
and only what differs per character has to be probed.

## Open shape questions

None of these are settled. They block writing schema, not writing prose.

1. ~~**Toolchain.**~~ **Settled.** Rust, with `rusqlite` and `proptest`, and
   **no ORM yet.**

   Rust because it has been effective for this work and because the property
   testing libraries are solid — and this project mostly cares about
   properties. Two kinds, pulling differently:

   | Kind | Example | Written as | Runs on |
   | --- | --- | --- | --- |
   | data | every `flood` declares an anti-seed; no boundary edge lacks its reverse | SQL assertions | the built artifact |
   | code | evaluating a definition twice gives the same set; `difference(X, X)` is empty | `proptest` | the evaluator |

   The data properties are the bulk and they are SQL, which is an argument
   against expressing them through a query DSL.

   No ORM *yet* because picking one now means picking before the query mix is
   known. Diesel suits relational CRUD and keeps a checked-in `schema.rs`,
   which is smoother while the shape moves; `sqlx` suits raw SQL, recursive
   CTEs and runtime-composed queries, but checks against a live database at
   compile time, which is friction for a schema still being designed.
   Revisit when hand-rolled migrations start hurting — that alone would
   justify Diesel.
2. **Sets as queries.** Eight of the nine `RoomSetDef` node types in urnon's
   digraph design are relational algebra SQLite already implements; only
   `flood` has content, and it is a recursive CTE. So a stored set is a view
   definition — as SQL text (powerful, unreviewable) or a restricted
   serialized form (reviewable, needs a compiler)?
3. **Bootstrap.** ~~Committed or rebuilt?~~ **Settled: option C.** Vendor a
   pinned `map.json`, build the tables, commit corrections as overlays applied
   after the import. Corrections are what decided it — committing the tables
   cannot keep one across a mapdb refresh, and rebuilding alone has nowhere to
   put one. See `docs/bootstrap-decision.md`.
4. **Layering.** `world` + `userland` attached together, with the runtime
   reading the union. Precedence when both define `bank`? Can userland
   *subtract*, and with what tombstone? Shared id space or separate ranges?
   What breaks when a shipped row moves under a userland reference?
5. **Distribution.** Checked-in binary, build artifact, or downloaded release?
   That decides versioning, offline behaviour, and whose problem a migration
   is.
6. **Analytics.** DuckDB reads these files; live store or exported snapshot?
   A daemon is writing the live one while a query runs.

## Non-goals

- Storage for per-character state. See "what does not", above.
- Being a runtime. This ships files; consumers route, plan and play.
- Replacing the mapdb as a *source*. The Lich mapdb stays frozen bootstrap
  input; what is published here is derived from it plus observation.
