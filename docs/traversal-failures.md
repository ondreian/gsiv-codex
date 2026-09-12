# Traversal failures

What the game says when a move does not happen, and what to do about it.

Extracted from Lich 5.17.1, `lib/global_defs.rb`, `move()` — 200 lines of Ruby
`elsif` holding twenty years of accumulated knowledge that no other client can
read. **29 classes, 124 patterns, 40 remedies.**

## Why this is not "the walk failed"

A walker that treats every failure alike throws away the only thing worth
knowing. Lich's own return value already makes the distinction, in three
values, and the table keeps them:

| `disposition` | Lich | Means | What a router does |
| --- | --- | --- | --- |
| `retry` | `put_dir.call` | fixable now | perform the remedies, send again |
| `edge-wrong` | `return false` | the exit is not there | quarantine the edge, re-plan |
| `unavailable` | `return nil` | real exit, not for you | keep the edge, route around it |
| `arrived` | `return true` | it worked | continue |

Nothing else is a reason to give up. Lich gives up after **10 seconds or 30
lines** with no recognised response — and only then, because an unrecognised
failure is the only one about which nothing is known.

The `edge-wrong` / `unavailable` split is the one that matters for a
crowd-sourced map. Both look identical from outside: you sent a command and did
not move. One means the data should change and the other means it should not.
Silverwood Manor's door is `unavailable` — the exit is real, and Lich returns
`nil` *specifically* so the direction is not deleted.

## Precedence is part of the data

Lich's chain is an `elsif`: first match wins. Several patterns overlap on
purpose —

```
not-allowed     appears to be closed, perhaps you should try again later\?$
way-is-closed   (?:appears|seems) to be closed\.$
```

— one means the shop shut for the night, the other means open the door. A table
is a set, so the order is written down as `precedence`, which is the source line
the branch came from. **Try classes in ascending `precedence` and take the first
that matches.**

## The remedies are a closed vocabulary

Twenty-eight branches, twelve verbs. A client implements the verbs once.

| Remedy | Detail | Notes |
| --- | --- | --- |
| `wait-rt` | — | wait out roundtime |
| `wait-stun` | — | wait while the stun indicator is set |
| `sleep` | ms | |
| `stand` | — | |
| `empty-hands` | — | **and refill them on arrival** |
| `open-the-way` | — | `open` the direction this step was going |
| `unhide` / `retreat` / `stow-feet` | — | |
| `cast` | spell number | only `9704`, Sigil of Resolve |
| `rewrite` | `go>climb` | change the command itself |
| `await` | text | wait for a line before retrying |

The data never carries a command string. `stand` is `stand` in this game and
something else in the next, and a free-text remedy would be a way to smuggle
arbitrary commands into a reviewed data file.

## What this is not

**Not preconditions.** Lich waits out roundtime and stun *before every send*,
not as a reaction. Those belong in the walker, not here — `wait-rt` appears as a
remedy only for the cases where something landed between the check and the
command.

**Not the ice pause.** `slipped` is the *recovery*: six seconds of roundtime,
on your back, in the room you started in. The four-second pause that stops it
happening is a `condition` on the edge, and the two are independent — the pause
is skippable at 50 Survival or under Haste, and the recovery never is. A walk
needs both.

## Divergences from Lich, deliberate

Two, both recorded in the rows themselves:

- **`pitch-dark`** — Lich returns `true` for `It's pitch dark and you can't see
  a thing!`, which claims you arrived. You did not, and every command after it
  is sent from the wrong room. Recorded as `unavailable`.
- **`cannot-drag`** — Lich also rebuilds the command as `drag <name> <target>`
  from an earlier grab line. Only the simple `climb`→`go` repair is expressed:
  dragging a body is not travel.

And one cost of keeping the patterns verbatim: `^You are already(?! as far away
as you can get)` uses a negative lookahead, which Ruby, PCRE and JavaScript have
and Rust's `regex` crate does not. Read it as two tests. A property test pins
the count at one, so a second is a conversation rather than a surprise.

## Refreshing it

```
tools/split_lich_move.py ~/dev/lich-5 data/vocabulary
```

The splitting is mechanical; the classification is the table at the top of that
script, keyed by source line. A Lich release that moves a branch **fails
loudly** rather than silently attaching the wrong remedy to the wrong message:

```
move() has changed; classify it again before regenerating.
  classified but no longer present: [615]
  present but unclassified:         []
```

## Open

- **`too-injured-to-climb` is conditional.** Lich's remedy is to cast Sigil of
  Resolve, and it returns `nil` when the character does not know it. Knowing a
  spell is a character fact, so the remedy needs a `condition_id` the way a
  prelude has one. Recorded as an unconditional remedy for now, which is wrong
  for a character without 9704.
- **`type-ahead-refused` is ours, not the world's.** It is the only failure
  here caused by the client — more commands in flight than the subscription
  allows — and the one a batching walker meets first. It probably belongs with
  the scheduler rather than in world data.
- **Nothing tags an edge with the failures it is prone to.** Every class is
  global today. If `slipped` only ever happens on the 132 ice edges, saying so
  turns a reaction into a prediction.
