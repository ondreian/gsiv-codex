# vendor

The Lich mapdb, pinned but not committed. Somebody else's work, and the
bootstrap for everything in `data/`.

`mapdb.source` names a commit in
[FarFigNewGut/lich-mapdb-room](https://github.com/FarFigNewGut/lich-mapdb-room)
and the SHA-256 of the file at it. `../scripts/vendor.sh` fetches it to
`map.json`, which `.gitignore` keeps out of git.

**Never edited.** A correction goes in `data/` as an overlay applied after the
import, so that refreshing this file cannot silently revert it. That is the
whole reason the build has an overlay step.

## Why a commit and not `main`

Upstream updates on a cron job, so `main` moves under any build that tracks
it — and a mapdb refresh arrives as an unreviewable diff in 41 MB of JSON.
Pinning a commit makes a refresh a deliberate act: change two lines, run
`scripts/vendor.sh` and `scripts/generate.sh`, rebuild, and let the
publishable suite tell you what moved. The first time that was done it caught
nine dropped edges in the first minute.

## Why it is fetched rather than committed

It was committed once, and the reasoning was sound as far as it went: git
deltas raw JSON against the previous version and cannot delta a gzip stream at
all, so raw cost 9.3M for two versions against gzipped 14M. What that
arithmetic left out is that the cheapest version of somebody else's 41 MB file
is the one you do not carry. The repo clones in 3.8M now, and the pin is a
stronger claim about provenance than a copy is.
