# Do the bootstrap tables get committed?

Everything here is measured against urnon's live store and the Lich mapdb on
this machine, not estimated.

## What we are actually talking about

| Artifact | Lines | Raw | Gzipped |
| --- | --- | --- | --- |
| `map.json` — the Lich mapdb, 36,611 rooms | — | **41 MB** | 6.9 MB |
| `rooms.tsv` | 28,446 | 1.2 MB | — |
| `edges.tsv` — walk class only, see below | 62,019 | 1.7 MB | — |
| `room_tags.tsv` | 429,936 | 10.1 MB | 1.5 MB |
| **all three** | **520,401** | **12 MB** | ~2.5 MB |

## First: script edges are not ours to publish

`edges` splits cleanly:

| class | count | longest command | contains tab/newline |
| --- | --- | --- | --- |
| `walk` | 62,016 | 40 chars | **2** |
| `script` | 3,037 | thousands | **164** |

Every script edge carries embedded Lich Ruby — `;e worn = !GameObj[...]` and
three kilobytes of it in the worst case, with newlines and tabs throughout.
They are already excluded from routing by default, because Ruby is not a
command any executor can replay.

**They are Lich implementation, not GemStone facts**, so they do not belong in
a data layer about the game. Dropping them also removes every tab and newline
problem from the format at a stroke: what remains is 62,016 walk edges whose
longest command is 40 characters.

The 2 walk edges that still carry a tab or newline are data errors, and "no
command contains a control character" becomes a property test that currently
fails twice — which is the point of having one.

Script edges are what connectors replace. The 18,637 rooms urnon reports as
"unlocked by migration" are behind them, and the replacement is curated
connector definitions, not archived Ruby.

## Option A — commit the bootstrap TSVs

**For**

- The repository is self-contained: clone, build, done. No external artifact
  has to still exist in three years.
- A wrong bootstrap row can be corrected by a pull request against the row.
- `git log` on a room answers "when did this change and why".
- 12 MB and 520k lines is unremarkable for git. Clone cost is once.

**Against**

- **A mapdb refresh is an unreviewable diff.** Lich's map changes; re-importing
  rewrites tens of thousands of lines at once and no human can tell a real
  change from a re-derivation. This is the same failure as committing derived
  facets, one level up.
- **Corrections are overwritten by the next import.** A hand-fixed edge is
  indistinguishable from an imported one, so the next refresh silently reverts
  it. That is the fatal one.
- Provenance is lost: the file cannot say which rows are Lich's and which are
  ours.
- It publishes a large body of somebody else's work as though it were ours.

## Option B — rebuild from the mapdb at build time

**For**

- The repository holds only what we curate: vocabulary, corrections,
  observations. ~95k lines instead of 520k.
- Lich's data stays Lich's, and the boundary is visible.

**Against**

- Reproducibility depends on an artifact outside the repository. If the mapdb
  moves or changes, an old commit no longer builds.
- Contributors need the mapdb before they can build anything.
- There is nowhere to put a correction. This is the hole that makes B
  incomplete on its own.

## Option C — vendor a pinned mapdb, build the tables, commit corrections as overlays

**This is the recommendation.**

```
vendor/
  map.json.gz          6.9 MB, pinned, replaced deliberately and never edited
data/
  facet_types.tsv      curated vocabulary
  tag_map.tsv          curated tag -> facet type
  corrections.tsv      ours: rows that override or delete an imported one
  observations.tsv     contributed by urnon from play
```

- **Self-contained**, like A: a clone builds without the network.
- **Small hot path**, like B: the files anybody edits or reviews are the
  curated ones. The vendored blob changes only when somebody deliberately
  refreshes it, as one commit that says so.
- **Corrections survive re-import**, which neither A nor B manages. An overlay
  is applied *after* the import, so refreshing the mapdb cannot silently revert
  it — and a correction that the new mapdb has made unnecessary shows up as an
  overlay that no longer changes anything, which is a property test.
- **Provenance is structural.** Lich's rows come from `vendor/`; ours are in
  `data/`. Nobody has to ask which is which.

**Against**

- A 6.9 MB binary blob in git history, which is permanent. Each refresh adds
  another ~7 MB and gzip does not delta-compress, so ten refreshes is ~70 MB.
  Mitigations: refresh rarely, or store it uncompressed so git can delta it —
  41 MB raw but packs far better across versions. Worth measuring before
  choosing.
- The build has a step that A does not: import, then overlay.

## The correction question is what decides it

Option A cannot keep a correction across a refresh. Option B has nowhere to put
one. Every serious use of this repository — urnon observing that an edge is
wrong, a human fixing a mis-tagged room — is a correction, so the option that
handles corrections properly is the one to take.

## Open within C

1. **Vendor compressed or raw?** Gzipped is 6.9 MB and opaque to git's delta
   compression; raw is 41 MB and deltas well across refreshes. Measure two real
   mapdb versions before deciding.
2. **Are tags overlay-able too,** or only rooms and edges? A mis-tagged room is
   a common correction and tags are the largest table.
3. **Does `vendor/` belong in git at all, or in a release artifact** pinned by
   checksum? That keeps history small and trades away the offline clone.
