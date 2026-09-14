# Contributing

The deliverable is a SQLite file. Everything here exists to produce one and to
prove things about it.

```sh
./scripts/vendor.sh                      # the pinned mapdb, 41 MB, not in git
cargo run --release --bin codex -- build \
    --from vendor/map.json --out /tmp/codex.db3 --vocabulary data
CODEX=/tmp/codex.db3 cargo test --release
```

Run the build before the tests. `tests/publishable.rs` checks an artifact
against the vendored mapdb, so testing a stale `codex.db3` against a fresh
`vendor/map.json` fails for a reason that has nothing to do with your change.

## The one rule

**Never edit imported rows. Corrections go in `data/overlays/`.**

The import is mechanical and the mapdb refreshes. An edit to an imported row
is silently reverted by the next refresh; an overlay applied *after* the
import survives it. This is the whole reason the pipeline has two halves, and
`docs/bootstrap-decision.md` is the long version.

Within `data/`, `vocabulary/` is curation — decisions somebody made — and
`overlays/` is correction. Both are hand-written SQL, reviewed as SQL.

## Three kinds of contribution

### World data

New connectors, bank mappings, ferry fares, room facets. A numbered file in
`data/overlays/`, or rows added to the one that already covers your subject.

What CI will ask of it, from `tests/publishable.rs` — every one of these is
there because it shipped broken once:

- a condition must be hung on something, or it forbids nothing
- a connector destination must have steps, or nobody can execute it
- a cycling step must know what it is waiting for, or it asks twenty times
- a placeholder must name its target, or nothing can fill it
- a road must lead to a room the file describes
- a circuit needs entries, moves and exits
- **every wayto must be published, audited, ambiguous, or point at a room the
  game never names.** There is no fifth case. Dropping an edge is allowed;
  dropping it silently is not.

### Extractor code

`src/extract.rs` reads the mapdb's embedded Ruby and decides what is
publishable. Two shapes are known-unhandled and sitting there to be taken:

| shape | edges | what it is |
| --- | --- | --- |
| `move 'north' while Room.current.id == N` | 317 | the whiteout retry loop — send it again until the room changes |
| `move` then a recovery from a fall | 21 | the Sleeping Lady's ice, which ends in the recovery rather than the move |

The work queue is in the artifact itself:

```sql
SELECT from_uid, to_uid, excerpt FROM script_edge_disposition
 WHERE reason LIKE 'no recogniser matched%';
```

If your change adds edges, say how many rooms it reconnects —
`tests/reachability.rs` measures that, and it is the number that matters.

When you decline to publish something, record it. `disposition = 'unhandled'`
is a gap somebody may close; `'excluded'` is a decision, and it needs a reason
that stops a later reader "fixing" it.

### Measurements

Numbers taken in-game, like `docs/ice-slip-measured.md`. CI cannot check these,
so the write-up carries the weight:

- the edge or mechanic, by uid
- the character's relevant state — one under seventeen spells is not a result
  about characters
- how many trials, and the counts per bucket, not a summary
- **the harness's own error, measured rather than assumed.** The ice pause was
  first published at 2500 ms on a harness that stamped arrival 1.07 s late,
  which was most of the effect being claimed.
- the raw trials, as a `.tsv` beside the doc

A number in `data/` should say in a comment where it came from.

## Generated files

`data/overlays/050_extracted.sql` and the connector overlays are produced by
`./scripts/generate.sh` and committed so a reviewer reads SQL rather than
trusting a program. Do not hand-edit them — change the generator and
regenerate. CI regenerates and diffs.

## Refreshing the mapdb

Change `commit` and `sha256` in `vendor/mapdb.source`, run
`./scripts/vendor.sh` and `./scripts/generate.sh`, rebuild, and read what the
publishable suite says. It is a deliberate act with a reviewable result, which
is the point of pinning a commit rather than tracking `main`.

## How a change gets released

`main` is protected: every change arrives as a pull request, and CI's three
jobs have to be green. `scripts/protect.sh` installs the ruleset from
`.github/ruleset-main.json`, which is the state — GitHub holds a copy.

Merging to main publishes a **canary** immediately: a rolling prerelease at the
`canary` tag, built from that commit and put through the same publishable suite
a real release is. It is there to be walked before anybody promises anything.
`codex upgrade` never takes it — that asks for the newest *stable* release, and
a prerelease is not one — so taking a canary is opting in by name:

```sh
codex upgrade canary
codex rollback          # back to the stable release you were on
```

Stable releases are proposed rather than cut by hand. release-please keeps one
open pull request holding every unreleased change, with the version bump and
the changelog already written from the commit messages; merging it tags the
release and attaches the artifact. So the commit subject is the changelog
entry — `feat:` and `fix:` decide the version, and `docs:` on a measurement is
what makes the finding show up in the notes.

## Style

`cargo fmt` and `cargo clippy --all-targets -- -D warnings` are clean, and CI
keeps them that way.

Comments say *why*, and the best ones name the thing that went wrong — the
tests and the SQL here are full of them because each cost somebody an evening.
