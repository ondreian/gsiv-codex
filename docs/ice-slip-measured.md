# The ice pause, measured

`ice-slip` gates 132 edges across five regions. The mapdb pauses four seconds
before crossing one and this repo copied that number without checking it. This
is the check.

Edge under test: 4044101 ↔ 4044102, *[Snowy Plains, The River]* on the Icemule
Trail, east/west, both directions carrying `ice-slip`. Norhaak, 17 active
spells, unencumbered. 190 crossings at Survival 0, 49 and 202.

## What the game does

Two messages, and only one of them is a failure:

| Message | Effect |
| --- | --- |
| `…you slip on a patch of ice but catch yourself before…` | none — the room description follows immediately, you moved |
| `…you slip on a patch of ice and flail uselessly as you land on your rear!` | 6s roundtime, prone, **still in the room you left** |

Both open with *"Running heedlessly through the icy terrain,"* which is the
whole mechanic in four words: the roll only happens when you move too soon
after arriving.

The `slipped` failure class matches the fall and not the catch. That is
correct — the catch costs nothing.

## The numbers

Gap is time between arrival in the room and the move, randomised per trial.
The harness stamps arrival **1.07s late** — measured directly, and stable to
five milliseconds across trials, because it stamps after a `push` and a `tail`
that each hold the daemon's control connection for 350ms. Every gap below is
the recorded figure plus that offset.

| Survival | true gap | n | fell | caught | clean |
| --- | --- | --- | --- | --- | --- |
| 0 | < 2.5s | 11 | 100% | — | — |
| 0 | 2.5–2.75s | 6 | 83% | — | 17% |
| 0 | 2.75–3.0s | 6 | 33% | — | 67% |
| 0 | 3.0–3.25s | 6 | 17% | — | 83% |
| 0 | ≥ 3.25s | 32 | **0%** | — | 100% |
| 49 | < 2.5s | 33 | 18% | 39% | 42% |
| 49 | ≥ 2.5s | 31 | **0%** | 6% | 94% |
| 202 | < 2.0s | 12 | 8% | 42% | 50% |
| 202 | ≥ 2.0s | 27 | **0%** | 52% | 48% |

Four things fall out.

**The pause a character owes is roughly 3.25s, 2.5s, 2.0s at Survival 0, 49,
202.** Four seconds was never far off — it is the worst case with about
three-quarters of a second of margin. `delay_ms` is now 3500.

**It is a roll, not a gate.** At Survival 0 the fall rate slides 100 → 83 → 33
→ 17 → 0 across three-quarters of a second. Something is rolled against the
gap and Survival sits on the other side of the roll.

**The catch message is Survival.** Zero catches in 80 trials at Survival 0.
Fifty-two percent of *safe* crossings at 202. That line is not flavour — it is
the near-miss, and a character who never sees it has no margin at all.

**The `survival < 50` escape is right, for the wrong reason.** It reads as "at
50 ranks you stop slipping", and that is false: 202 still falls one crossing in
thirteen when hurried. It survives on arithmetic instead. A fall costs six
seconds of roundtime, so skipping is worth it whenever `P(fall) × 6s` beats the
pause — 18% × 6s ≈ 1.1s against 3.5s at Survival 49, and half a second at 202.
Skipping wins comfortably at both. The term is a good policy; only its stated
reason needs correcting.

## `ice-slip-resolve` could not be tested

Not for the reason first written here. The claim was that the 21 dropped edges
are the only way north and that Pinefar is one-way. That is wrong, and the
arithmetic says so: restoring those 21 edges reconnects **nothing**.

What is true is that three separate classes of dropped edge overlap on the
route, and the region needs all three:

| restored to the published graph | edges | rooms reachable from the Icemule Trail |
| --- | --- | --- |
| (nothing — as published) | — | 10,047 |
| `move`-while-still-here, the whiteout retry loop | 140 | 10,078 |
| `ice-slip-resolve`, move-then-recovery | 21 | 10,047 |
| plain commands dropped anyway | 512 | 10,116 |
| all three | 673 | 10,193 |

4560050 becomes reachable only with all three. The Sleeping Lady is not cut
off by the ice edges; it is cut off by whichever of the three you leave out.

### The third row is the bug worth fixing first

**546 plain-command edges are dropped with no `script_edge_disposition` row.**
Not scripts the extractor declined to read — `go opening`, `east`, `go rocks`,
commands with nothing to interpret. That table exists so nothing vanishes
without saying so, and 546 edges vanished silently. Thirty more are in the
audit and correctly so.

### The other two shapes

**The whiteout retry loop**, 317 in the mapdb and 140 not otherwise published:

```ruby
;e fput 'north'; move 'north' while Room.current.id == 2925
```

Send the direction, repeat until the room actually changes. The Snow Plains
are a whiteout and a move there sometimes does not take. This needs no new
mechanism — it is `Action::Cycle` with the room id as the condition, which
already exists for Symbol of Seeking.

**Move-then-recovery**, the 21 ice edges, exactly as `040_conditions.sql`
describes: the script ends in a recovery rather than a move, so
`looks_like_movement` declines it. Worth fixing for the pause it carries, but
it unblocks no route on its own.

## Three harness errors worth not repeating

**`push` and `tail` hold the control connection open** — the daemon streams the
feed down it — so `timeout N nc` costs N, not "up to N". A `timeout 5` paced
the first run of this experiment at five seconds a move, and it reported forty
crossings with no slips. There is no mechanic that survives a five-second gap.
The same hold is where the 1.07s stamping bias comes from.

**A fall does not move you.** Six seconds of roundtime, then you stand, still
in the room you started in — so the next attempt has six seconds of gap no
matter what the experiment asked for. At "gap 0" the outcomes alternated
fell/crossed *perfectly*, twenty times, and the honest rate was double what was
reported. Perfect alternation is the tell.

**Measure the instrument before believing the reading.** The first write-up of
this file said "two seconds, not four" and briefly published a pause inside the
band that still falls. The bias was assumed to be "a few hundred milliseconds"
and was 1.07s, most of the effect being claimed. Measuring it took thirty
seconds.

Raw trials: `docs/ice-slip-trials.tsv`, gaps as recorded — add 1.07s.
