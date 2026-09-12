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
