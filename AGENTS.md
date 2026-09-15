# Working on this repo

The deliverable is one SQLite file. Everything here produces one, or proves
something about one.

Read `CONTRIBUTING.md` for what a change should be. This is how to make one
without losing an afternoon to something the repo already knows.

## The loop

```sh
./scripts/vendor.sh                       # the pinned mapdb, 41 MB, not in git
cargo run --release --bin codex -- build \
    --from vendor/map.json --out /tmp/codex.db3 --vocabulary data
CODEX=/tmp/codex.db3 cargo test --release
```

**Build before you test, and pass `CODEX`.** `tests/publishable.rs` checks an
*artifact* against the vendored mapdb. Without `CODEX` it falls back to
whatever is installed at `~/.local/share/urnon/codex.db3`, which is likely a
different build, and it will fail for reasons that have nothing to do with your
change. This has cost time more than once.

Timings, so nothing looks hung: the artifact build is about ninety seconds,
`tests/reachability.rs` about the same, everything else is instant.

## What the tests are

`tests/publishable.rs` is the contract, and **every check in it is there
because it shipped broken once**. A schema constraint says a row cannot be
written; these say a database can be written entirely within its constraints
and still be wrong — a gate hung on nothing, a mechanism whose commands cannot
be sent, a road to a room the file has never heard of.

If you are adding a kind of thing, add the check that says it is wired up.

## Rules that are not style

**Never edit imported rows.** The import is mechanical and the mapdb refreshes;
an edit to an imported row is silently reverted by the next refresh. Corrections
go in `data/overlays/` and are applied afterwards.

**Never hand-edit a generated file.** `data/overlays/050_extracted.sql` and the
connector overlays come from `./scripts/generate.sh`. CI regenerates and diffs;
a hand-edit fails with the diff as its message.

**Nothing vanishes without saying so.** If the extractor declines an edge, it
gets a `script_edge_disposition` row. `unhandled` is a gap somebody may close;
`excluded` is a decision and needs a reason that stops a later reader "fixing"
it. A bare `continue` once left 5,158 edges with no trace of having been looked
at.

## The backlog is in the artifact

Not in a document that goes stale:

```sql
-- what the extractor declined, by shape
SELECT substr(reason, 1, 40), count(*)
  FROM script_edge_disposition
 WHERE disposition = 'unhandled'
 GROUP BY 1 ORDER BY 2 DESC;
```

The largest group is usually one pattern. Teaching the extractor one shape
moves thousands of edges.

## How to tell if it worked

`tests/reachability.rs` counts rooms reachable from a starting room. That is
the number a data change is really about — not how many rows landed. Say it in
the pull request: *"reconnects N rooms"* beats *"adds 673 edges"*.

## Traps, each one paid for

- **SQLite cannot `ALTER` a `CHECK`.** Widening one means rebuilding the table
  and carrying the rows over. Re-seeding instead would lose whatever an overlay
  had corrected. See `schema/016_relocated.sql`.
- **A refresh of the mapdb is a real diff.** The pin is a commit for that
  reason. Change it deliberately, regenerate, and read what the suite says.
- **Grep the corpus with `-a`.** Session logs are binary-framed; without it
  grep prints nothing and you conclude the thing never happens. It cost a whole
  experiment.
- **`urnon_min` is a promise.** Publishing a value an older engine cannot read
  is fine — engines refuse to guess at an unknown disposition — but bumping the
  floor strands every client below it.

## When the game is the source

Some facts are only knowable by playing. Those get measured and written down
with the numbers: `docs/ice-slip-measured.md` and `docs/the-rift.md` are the
shape to copy — what was measured, how many trials, and **what the instrument's
own error was**. A number without its method is not evidence.

Do not invent in-game message text. Take it from a log, or from gswiki with the
page linked.
