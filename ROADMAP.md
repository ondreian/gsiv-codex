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

1. **world** — in progress. Rooms and facets exist in `urnon`'s store today and
   move here. Room sets are unbuilt and everything else waits on them.
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

1. **Toolchain.** Is this repo Rust, and if so does it use an ORM? The
   deliverable is a SQLite file, so a consumer's choice is unaffected either
   way — but the pipeline needs migrations, and `flood` needs a recursive CTE
   that a query DSL cannot express.
2. **Sets as queries.** Eight of the nine `RoomSetDef` node types in urnon's
   digraph design are relational algebra SQLite already implements; only
   `flood` has content, and it is a recursive CTE. So a stored set is a view
   definition — as SQL text (powerful, unreviewable) or a restricted
   serialized form (reviewable, needs a compiler)?
3. **Layering.** `world` + `userland` attached together, with the runtime
   reading the union. Precedence when both define `bank`? Can userland
   *subtract*, and with what tombstone? Shared id space or separate ranges?
   What breaks when a shipped row moves under a userland reference?
4. **Distribution.** Checked-in binary, build artifact, or downloaded release?
   That decides versioning, offline behaviour, and whose problem a migration
   is.
5. **Analytics.** DuckDB reads these files; live store or exported snapshot?
   A daemon is writing the live one while a query runs.

## Non-goals

- Storage for per-character state. See "what does not", above.
- Being a runtime. This ships files; consumers route, plan and play.
- Replacing the mapdb as a *source*. The Lich mapdb stays frozen bootstrap
  input; what is published here is derived from it plus observation.
