# gsiv-codex

A unified data layer for GemStone IV: the world, its verbs, its foes and its
items, as facts that any client can read.

Lich has the knowledge but no layer — it lives in a hundred scripts that share
no vocabulary and cannot reference each other. This is that layer.

See [ROADMAP.md](ROADMAP.md) for the four domains, the order they land in, and
the shape questions still open.

## The one rule

**A fact here is true for everyone.**

| Belongs here | Does not |
| --- | --- |
| The 13 FWI teleport anchors | Whether this character holds the trinket |
| A room's facets | Where this character is standing |
| A foe's stats | What this character last killed |
| What `STOW` does | This character's aliases |

Anything true of one character at one moment belongs in the client, and reaches
a router as an offer on a request rather than a row in a table.

## Status

Nothing published yet. The first domain is **world**, whose contents currently
live in `urnon`'s `map.db3` and move here.

## Consumers

[`urnon`](https://github.com/ondreian/urnon) is the first, not the owner.
Nothing here depends on it.

## Building one

```sh
codex build --from ~/.local/share/urnon/map.db3 \
            --out  /tmp/codex.db3 \
            --vocabulary data
```

```
rooms 28446, edges 62012 (skipped 3037 script, 2 ambiguous)
locations 28192, facets from tags 374425
```

urnon reads it with no flag — `World::load_any` recognises the schema:

```sh
urnon-world --db /tmp/codex.db3 plan 4042150 4043301
4042301  walk  east
4043301  walk  go archway
2 steps, 2.6s

urnon-daemon -i gst --control-port 7331 --world-db /tmp/codex.db3
```

Verified live: `/travel bank`, `/travel town` and `/travel gemshop` all plan and
walk on a graph urnon did not write.
