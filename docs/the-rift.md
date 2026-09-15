# The Rift moves you

Most places, where you are is a function of what you sent. The Rift is not most
places, and a walker built on that assumption misbehaves there in a way nothing
on screen explains.

## Being rifted

> Suddenly you feel sick and queasy, and your world seems blurry and
> indistinct.  You fight an intense vertigo for a moment before the sensation
> leaves you.

You do not change place. You change **plane** — the same description, a
different layout, different exits. Plane 1 rifts to 2, 2 to 3, and 3 back to 2.
Documented at [Research:The Rift (planes)][planes]; the trigger is described
only as "unknown mechanics during normal hunting", so there is no rate to
quote. It is frequent enough that a long walk meets one.

[planes]: https://gswiki.play.net/Research:The_Rift_(planes)

**This is the dangerous shape of relocation, not the obvious one.** A teleport
that dropped you somewhere else would be visible: a new room, a new
description, an obviously broken plan. A rift leaves the screen looking
untouched. Two hundred and two of the Rift's two hundred and thirty-two rooms
are titled `[The Rift]`, so title tells you nothing, and the description is by
definition the same one.

What does change is the room uid, because the planes are separate blocks:

```
4566001 – 4569023    188 rooms
4570001 – 4571030     44 rooms
```

So it is invisible to a reader and unmistakable to `<nav rm=>`. The message is
the fast signal; the uid is the one that cannot be missed in a busy feed.

## Why the existing dispositions are all wrong for it

| | what it would do | why it is wrong |
| --- | --- | --- |
| `retry` | send the command again | into the new room, whose doors have been replaced |
| `edge-wrong` | quarantine the edge, re-plan | the road was never wrong, and quarantine is for the session |
| `unavailable` | leave the edge, refuse the route | the route is fine; the position is not |

Hence `relocated`, added in schema 16: **the plan is stale and the map is
fine**. Re-plan from where you now are, and quarantine nothing.

An engine that does not know the word refuses to guess at it rather than
picking the closest, so an old client sees the vertigo line as unrecognised —
exactly what it does today. The behaviour arrives with the engine; nothing
breaks while waiting.

## Not a temporal rift

A separate mechanic with separate messaging: [Research:Temporal rift][temporal]
is triggered by Familiar Gate (930), glyph traps and chronomage punishment,
displaces roughly 5% per movement, and announces itself with *"Time and space
fold in upon itself..."*. It is the one with a documented rate, which makes it
easy to quote about the wrong thing.

[temporal]: https://gswiki.play.net/Research:Temporal_rift

## What a walker still owes

Publishing the word does not make anybody act on it. On the urnon side:

- **Send one command at a time where exits move.** A batch assumes position
  follows from commands. After a rift the queued remainder fires into a room
  chosen for you, and those commands are real — they move you further from the
  plan. A `cycle` step already runs alone for a neighbouring reason.
- **Compare position per step, not per batch.** A rift is not a failure and
  nothing is refused, so the only other evidence is a uid that is not the one
  the plan named.
- **Report no timing for a surprised step.** `report_step` attributes elapsed
  time assuming the prefix worked, and after a rift those numbers are fiction
  feeding a cost model that a walk is the only thing measuring.
