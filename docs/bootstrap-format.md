# The bootstrap format

What is checked in, in what shape, and why it diffs cleanly. The constraint
that drives all of it: **urnon opens automated pull requests against this
repo**, so a machine writes the files and a human reads the diff.

## Not all data has the same git needs

Counted from urnon's live store:

| Kind | Rows | Comes from | Committed? |
| --- | --- | --- | --- |
| **Derived** — room facets | 374,411 | built from tags + vocabulary | **no** |
| **Bootstrap** — rooms, edges, tags | 28,446 / 65,053 / 439,383 | the frozen Lich mapdb | yes, rarely changes |
| **Curated** — vocabulary, tag map, set definitions | 21 / 816 / — | humans | yes, hand-edited |
| **Contributed** — observations | tens per PR | urnon, from play | yes, the hot path |

**Derived data is never committed.** That single rule removes 374,411 lines
from the repository and the review burden with them. A facet is a function of a
tag and the tag map; checking in the output means every vocabulary change
rewrites a third of a million lines, and a reviewer cannot tell a real
correction from a re-derivation. Check in the inputs, build the output.

Only *contributed* and *curated* are edited often, and only those have to diff
beautifully.

## The format: TSV, one file per table

```
data/
  facet_types.tsv      21 rows, curated
  tag_map.tsv         816 rows, curated
  rooms.tsv        28,446 rows, bootstrap
  edges.tsv        65,053 rows, bootstrap
  room_tags.tsv   439,383 rows, bootstrap
```

One row per line, because git merges line-wise. Two contributors adding edges
in different towns produce diffs that merge without a conflict — which is the
property that makes automated PRs viable at all.

TSV over the alternatives:

| | verdict |
| --- | --- |
| **TSV** | one line per row, no quoting rules to get wrong (tabs are banned in values), smallest of the text formats |
| JSONL | same diff properties, 3–5× the bytes, and field names repeated 439,383 times |
| SQL `INSERT` | same diff properties, larger, and tempts logic into the data files |
| CSV | quoting rules that differ per writer — the one thing a canonical format cannot have |
| the `.db3` itself | binary, page-ordered, unreviewable. Never |

## Stability rules

These are what make the format canonical rather than merely textual. All are
enforced, not documented-and-hoped-for:

1. **Sorted by primary key**, always, with a fixed collation.
2. **Fixed column order**, matching the schema.
3. **LF endings, trailing newline, no trailing whitespace.**
4. **No tabs or newlines inside values** — rejected at write time rather than
   escaped, so there is one representation of every row.
5. **No timestamps in a committed row.** See below.
6. **Integers bare, no leading zeros; empty string for absent, never `NULL`.**

## Timestamps do not go upstream

`edge_observations.observed_at` is microseconds since the epoch. Contributed as
a column it would make every row globally unique, so two players observing the
same edge produce two rows that never dedupe, and the file grows without ever
converging.

The fact is *"this edge exists"*. Who saw it and when is provenance, and
provenance belongs to the pull request — its author, its commit, its date —
where git already stores it and nobody has to diff it.

## The codex has no `edge_id`

urnon's `edges.edge_id` is a surrogate, and the digraph design reserves it
deliberately: §6's parallel edges are two rows agreeing on
`(from_uid, to_uid, command)` and differing only in their gate, which any
uniqueness constraint over those columns would forbid.

Measured, that case does not exist yet. **All 65,053 edges are distinct on
`(from_uid, to_uid, command)`** — gates are unbuilt, so nothing is parallel.
When gates arrive the gate joins the key rather than being hidden behind a
surrogate.

So the published identity of an edge is its columns, and no id is written at
all. A content hash would be the same determinism made unreadable:

```
a3f8c12b	mana	20                    a hash
4042150	4042301	east	mana	20      the key
```

Only the second can be reviewed, and the first puts a hashing algorithm into
the format specification that every writer must reproduce byte-identically for
as long as the repository exists. Child tables — costs, gates, companions —
reference the columns. That is wider and it is legible, and they hold 0 rows
today, so the width is hypothetical and the legibility is not.

urnon may keep whatever surrogate it likes internally. It is an index, not a
fact, and it stops at the boundary.

## The formatter is the mechanism

`cargo run --bin codexfmt` rewrites every file into canonical form. CI runs it
and fails if anything changed — the `cargo fmt --check` pattern.

That is what makes automated PRs orderly. urnon does not need to know the
sorting rules or the escaping; it appends rows in any order, runs the
formatter, and the diff is canonical by construction. A hand edit that breaks
the order is caught by the same check.

## What an automated PR looks like

urnon walks somewhere nobody has walked, and proposes it:

```diff
--- a/data/edges.tsv
+++ b/data/edges.tsv
@@ -41203,6 +41203,7 @@
 4042150	4042229	south	walk	200
 4042150	4042301	east	walk	200
+4042301	4043301	go archway	walk	200
 4042301	4042302	east	walk	200
```

One line, in sorted position, reviewable at a glance. That is the whole goal.

## Open

- **Are bootstrap tables committed at all, or rebuilt from the mapdb?**
  `room_tags.tsv` is 439,383 lines. If the mapdb is the frozen input and it is
  archived somewhere reachable, the tags are derived too and need not be
  committed. That turns the repository from ~530k lines to ~95k. The cost is
  that building requires the mapdb rather than just the repo.
- **Partitioning.** `edges.tsv` at 65k lines is fine for git and awkward on
  GitHub's web diff. Splitting by town or uid range would help review and
  complicate the writer. Not worth doing until somebody is annoyed.
- **Schema drift.** A column added to a table invalidates every row of its
  file. Whether the formatter migrates files or the build refuses them is
  unsettled.
