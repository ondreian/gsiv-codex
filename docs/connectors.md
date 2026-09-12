# Expressing connectors

Script edges are not published — they are Lich Ruby, and 164 of the 3,037
would break the format on their own. But the knowledge in them is real: they
are how Lich says "sail here", "turn the trinket", "pay the urchin". Connectors
are that knowledge, expressed as data.

## What a connector is

An **offer**: from these rooms, to that room, run these commands, at this cost.
No mechanism type, no teleporter class, no urchin — dozens of items in the game
do this and differ in every particular except that shape.

Split by the repository's one rule:

| Half | Example | Home |
| --- | --- | --- |
| **Definition** | the 13 FWI anchors, the 419 urchin destinations, what to send | **here** |
| **Availability** | does this character hold the trinket, when the lease lapses | the client |

The client offers a connector only when it has established the character can
use it right now, which is why there is no gate vocabulary here and does not
need to be.

## Tables

```sql
connectors (
    id           TEXT PRIMARY KEY,      -- 'urchin-guides:icemule-trace'
    origin_mode  TEXT NOT NULL CHECK (origin_mode IN ('anywhere','only','anywhere_except')),
    origin_set   TEXT REFERENCES room_sets(name),   -- NULL iff mode = 'anywhere'
    overhead_ms  INTEGER NOT NULL DEFAULT 0,
    description  TEXT NOT NULL DEFAULT ''
);

connector_destinations (
    connector_id TEXT NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    to_uid       INTEGER NOT NULL REFERENCES rooms(uid),
    cost_ms      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (connector_id, to_uid)
);

connector_steps (
    connector_id TEXT NOT NULL,
    to_uid       INTEGER NOT NULL,
    seq          INTEGER NOT NULL,      -- ordered; a hop is several commands
    command      TEXT NOT NULL,
    expect       TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (connector_id, to_uid, seq),
    FOREIGN KEY (connector_id, to_uid)
        REFERENCES connector_destinations(connector_id, to_uid) ON DELETE CASCADE
);
```

Three tables because a hop is genuinely several commands with different failure
modes — `twist #band to 3` can fail in ways `rub #band` cannot. urnon's
`AGENTS.md` records what happened the last time this project modelled a
multi-action move as one string: `edge_steps` shipped and was dropped within
the day.

`origin_set` is a **set reference, not a room list**. That is the dependency:
connectors need room sets, so sets land first.

## `expect` is a game string here, not a client label

In urnon `Instruction::expect` is opaque — a label the client matches back to
its own `Correlation`. In the codex it is the text the **game** produces on
success:

```
urchin guide bank	You flag down a nearby urchin
```

That is a fact about GemStone, true for everyone, and exactly the kind of thing
this layer exists to stop every client rediscovering.

## One placeholder, and only one

Commands are literal, except where the literal value is per-character:

```
turn #{item}              the character's own trinket, by their own item id
twist #{item} to 3        the slot number is world data; the id is not
urchin guide bank         no placeholder; the command is the same for everyone
```

`{item}` is a named hole the client binds. It exists because the alternative is
storing a per-character item id, which the one rule forbids. It is **not** a
template language — no conditionals, no loops, no expressions. If a second
placeholder is ever needed it gets argued for on its own merits.

Destinations are written out literally rather than templated. The urchin
network is 419 rows of `urchin guide <keyword>` and that is fine: 419 lines
diff cleanly, and a template would put logic in a data file, which is the same
reason the format is TSV and not SQL `INSERT`.

## Worked example

```
# data/connectors.tsv
urchin-guides:icemule-trace	only	set:icemule-trace	0	the urchin guides, Icemule
fwi-trinket	anywhere_except	set:fwi-noteleport	3000	the Isle of Four Winds trinket

# data/connector_destinations.tsv
urchin-guides:icemule-trace	4043301	1000
urchin-guides:icemule-trace	4047001	1000
fwi-trinket	3201029	5000

# data/connector_steps.tsv
urchin-guides:icemule-trace	4043301	0	urchin guide bank	You flag down a nearby urchin
urchin-guides:icemule-trace	4047001	0	urchin guide temple	You flag down a nearby urchin
fwi-trinket	3201029	0	turn #{item}	You get the feeling
```

Sorted, one fact per line, and an automated PR adding a town's guides is a
contiguous block a reviewer can read.

## Superseding script edges is measurable

A connector records which script edges it replaces:

```sql
connector_supersedes (
    connector_id TEXT NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    from_uid     INTEGER NOT NULL,
    to_uid       INTEGER NOT NULL,
    command      TEXT NOT NULL,
    PRIMARY KEY (connector_id, from_uid, to_uid, command)
);
```

The reference is to a row in the *vendored import*, not to a published table —
script edges are never published. That gives the burn-down a number:

- 3,037 script edges exist in the mapdb.
- N are superseded by a connector.
- The rest are unhandled, and the 18,637 rooms urnon reports as "unlocked by
  migration" sit behind them.

Which makes the coverage a property test — every script edge is superseded, or
listed as known-unhandled with a reason — rather than a feeling about progress.

## Dependency order

1. **room sets** — `origin_set` references one, and `anywhere_except` is
   useless without one. Slice 2.
2. **connectors** — these tables. Slice 3.
3. **the burn-down** — `connector_supersedes` and its coverage property, once
   there are connectors to count.

`Origins::Anywhere` alone is expressible today, which is not enough to be worth
shipping before sets.
