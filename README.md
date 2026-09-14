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

Build from the vendored mapdb. It is the source of record, and the whole
pipeline is reproducible from a clone.

```sh
codex build --from vendor/map.json \
            --out  /tmp/codex.db3 \
            --vocabulary data
```

```
rooms 28727 (skipped 7884 unmapped, 281 instanced)
edges 61664 (skipped 7720 script, 529 dangling, 5 ambiguous), facets 406066
overlays applied: 1874
```

`--from` also takes urnon's already-imported `map.db3`, which was the way in
before the mapdb importer existed. **Do not ship a build made that way.** That
store keeps only Lich rooms carrying exactly one uid, so the 281 instanced
rooms and every edge touching them are gone — 546 plain-command edges dropped
with nothing said, and the Sleeping Lady unreachable. The path stays for
comparing the two; `every_wayto_is_published_or_accounted_for` fails on it,
which is the point.

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
