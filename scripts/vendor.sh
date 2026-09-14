#!/usr/bin/env bash
# Fetch the pinned mapdb into vendor/map.json.
#
# Not vendored into git: it is 41 MB of somebody else's data, and this repo
# should carry the pin rather than the payload. `vendor/mapdb.source` says
# which commit, and the checksum says it arrived intact.
set -euo pipefail
cd "$(dirname "$0")/.."

# shellcheck disable=SC1091
eval "$(grep -E '^(repo|path|commit|sha256)=' vendor/mapdb.source)"
out=vendor/map.json

if [ -f "$out" ] && [ "$(sha256sum "$out" | cut -d' ' -f1)" = "$sha256" ]; then
  echo "vendor/map.json already at $commit"
  exit 0
fi

url="https://raw.githubusercontent.com/$repo/$commit/$path"
echo "fetching $url"
curl -fsSL "$url" -o "$out.tmp"

got=$(sha256sum "$out.tmp" | cut -d' ' -f1)
if [ "$got" != "$sha256" ]; then
  rm -f "$out.tmp"
  echo "checksum mismatch for $commit" >&2
  echo "  expected $sha256" >&2
  echo "  got      $got" >&2
  echo "A pinned commit cannot change. Either the pin is wrong or the fetch is." >&2
  exit 1
fi
mv "$out.tmp" "$out"
echo "vendor/map.json at $commit ($(du -h "$out" | cut -f1))"
