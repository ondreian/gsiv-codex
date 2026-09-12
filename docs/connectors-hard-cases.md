# The hard script edges

`{item}` covers the easy half. This is what the other half actually contains,
counted rather than guessed.

## Measured

| | count | of 3,037 |
| --- | --- | --- |
| script edges | 3,037 | — |
| single line, no Ruby control flow | 428 | 14% |
| carries `$go2_restart` | 636 | 21% |
| juggles hands or containers | 136 | 4.5% |
| invokes `go2` recursively | **4** | 0.1% |
| reads game output to decide | **9** | 0.3% |

The tail that is genuinely a program is **13 edges**. Everything else is a
sequence with waits, preconditions, and sometimes a destination Lich could not
name.

## What Lich's constructs actually mean

Each of these is a workaround for something the mapdb format cannot say. Named
properly, most stop being code.

### `$go2_restart = true` — "I do not know where you land"

636 edges. Lich sets a global and lets the pathfinder start over. It is not
control flow; it is a **confession that the destination is not fixed**.

As data that is a destination *kind*: `replan`. The hop happens, the client
reads the room it arrived in, and routes on from there. No global, no restart —
the router simply plans again from a room it now knows.

### `UserVars.mapdb_duskruin_origin = 9652` — "remember the way back"

The outbound hop records where it started so the return hop can undo it. That
is a **round trip**, and it is why the FWI trinket is worth +1,374 rather than
the +11,468 an anchor-to-anchor reading suggested.

As data: destination kind `origin`. The client already knows where it came
from; it does not need a variable in the map to tell it.

### `waitfor 'A crew member escorts you off the ship.'`

Already modelled. That string is `expect` on the step — a game fact, publishable
as-is.

### `2.times{fput "event transport duskruin"}` and `multifput a, a`

The same command twice, because the game wants it twice. Two steps with the
same text, or a repeat count. Not logic.

### `dothistimeout "get my trinket", 5, /pattern/`

Send, and wait up to n seconds for one of several replies. A step with an
`expect` and a timeout — the branching in Lich is error handling, which is the
client's job, not the map's.

### Hands and containers — 136 edges

The FWI trinket's Ruby opens with `worn = !GameObj[...].nil?` and then does
nothing at all if the answer is yes — you simply `turn` it. The hand and
container dance is the *fallback* for a trinket carried in a container: empty a
hand, find it, get it, remember where it came from, turn it, put it back,
refill hands.

So the requirement is **not** that the trinket is in hand. It is that the
character **has** one. Worn is the common case and needs no preparation
whatever; held needs none either; in a container needs the dance.

**None of that is a fact about the world.** The fact is *this connector needs
that item*. Where the character keeps it, and what it takes to make it usable,
is the client's own business — urnon already has a container store and hand
tracking, and it knows whether the thing is worn. As data it is a
**precondition** from a closed vocabulary, and the cleanup afterwards is the
client restoring its own state, which it is far better placed to do than a map
is.

### `force_start_script 'go2', [room]` — 4 edges

An edge whose traversal is "first go somewhere else". Lich needs recursion here
because its edges are opaque: nothing can see inside one to plan through it.

A router that understands connectors does not need this at all. "Reach room X,
then use connector Y" is an ordinary plan — composition is what a router is.
These four stop being special the moment the mechanism is data.

### Reading output to decide — 9 edges

`put "touch mural"`, then read lines, match a riddle, answer it. This is a
program and will not be data. It should not be.

## The design that follows

Destination kind, closed:

```sql
connector_destinations (
    connector_id TEXT NOT NULL,
    kind         TEXT NOT NULL CHECK (kind IN ('fixed','replan','origin')),
    to_uid       INTEGER REFERENCES rooms(uid),   -- NULL unless kind = 'fixed'
    cost_ms      INTEGER NOT NULL DEFAULT 0,
    ...
);
```

- `fixed` — a known room. The urchins, the Chronomage rooms.
- `replan` — you land somewhere; read it and route on. What `$go2_restart` meant.
- `origin` — back where this connector brought you from. Round trips.

Preconditions, closed vocabulary, client-satisfied:

```sql
connector_requires (
    connector_id TEXT NOT NULL,
    requirement  TEXT NOT NULL,   -- 'has-item', ...
    detail       TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (connector_id, requirement, detail)
);
```

`has-item` says what the connector needs, never where it must be — a worn
trinket and one in a backpack satisfy the same requirement at different cost to
the client. The vocabulary starts at exactly what the 136 edges need and grows
only on evidence. A requirement the client cannot satisfy means the connector is not
offered — which is the same rule as the lease, and needs no new machinery.

Steps gain a timeout, since `dothistimeout` carries one:

```
connector_steps (connector_id, dest_seq, seq, command, expect, timeout_ms)
```

## What stays unhandled, on purpose

Nine edges that read game output and decide. `connector_supersedes` lists them
as known-unhandled with a reason, and the coverage property asserts that every
script edge is either superseded or listed — never silently missing.

Nine is a number worth stating plainly: it is 0.3% of the script edges, and
pretending a puzzle is data would be how the other 99.7% get compromised to
accommodate it.
