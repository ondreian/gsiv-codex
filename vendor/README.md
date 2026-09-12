# vendor

The Lich mapdb, pinned. Somebody else's work and the bootstrap for everything
in `data/`.

**Never edited.** A correction goes in `data/` as an overlay applied after the
import, so that refreshing this file cannot silently revert it. That is the
whole reason the build has an overlay step.

## Refreshing

Replace `map.json`, rebuild, and commit both in one change that says where the
new one came from. Expect a large diff; that is what a refresh is.

## Why raw rather than gzipped

Measured, because the intuition is wrong. Git already compresses, so one
version costs the same either way — and a second version does not:

| | one version | two versions |
| --- | --- | --- |
| raw JSON | 7.0M | **9.3M** |
| gzipped | 7.0M | **14M** |

Git deltas the raw JSON against the previous version and cannot delta a gzip
stream at all. Ten refreshes is roughly 30M against 70M. The cost is 41M in a
working tree rather than 6.9M, which is the cheaper thing to spend.
