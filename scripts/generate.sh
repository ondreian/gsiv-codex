#!/usr/bin/env bash
# Regenerate every overlay that is derived from the mapdb.
#
# These are committed rather than built, so a reviewer reads SQL instead of
# trusting a program. The cost is that they can drift from the generator, so
# CI runs this and diffs.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --bin codex
codex=target/release/codex

"$codex" extract --from vendor/map.json --out data/overlays/050_extracted.sql
"$codex" seeking --from vendor/map.json --out data/overlays/075_seeking.sql
"$codex" ferry   --from vendor/map.json --out data/overlays/076_ferry.sql
"$codex" fwi     --from vendor/map.json --out data/overlays/077_fwi.sql
"$codex" rift    --from vendor/map.json --out data/overlays/078_rift.sql
