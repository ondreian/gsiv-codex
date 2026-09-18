#!/usr/bin/env bash
# Fetch base creature levels from gswiki into vendor/wiki_levels.tsv.
#
# The wiki is the game's own reckoning of what level a creature is, and it is
# maintained by people who play it. lich-5's files carry a level too and the
# two agree 569 times out of 577, which is the reason to keep both: where they
# differ, somebody should look rather than a build silently picking one.
#
# Not pinned the way vendor/mapdb.source pins a commit, because a wiki has no
# commit to pin. `vendor/wiki_levels.source` records when this was run and what
# it asked for, and the TSV is committed so a build never needs the network.
set -euo pipefail
cd "$(dirname "$0")/.."

page="Category:Creatures_by_Level"
api="https://gswiki.play.net/api.php"
out=vendor/wiki_levels.tsv

# `action=parse` rather than `action=raw`: the page is a table of
# `{{Level N creatures}}` transclusions, and raw wikitext is 120 template
# names rather than 587 creatures.
curl -fsSL --max-time 120 \
  "$api?action=parse&page=$page&prop=text&formatversion=2&format=json" \
  -o vendor/.wiki_levels.json

python3 - "$out" <<'PY'
import json, re, sys

html = json.load(open("vendor/.wiki_levels.json"))["parse"]["text"]
# Each cell opens with a link to Category:Level_N_Creatures and is followed by
# the list of creatures at that level, up to the end of the cell.
cells = re.split(r'<a href="/Category:Level_(\d+)_Creatures"', html)
levels = {}
for i in range(1, len(cells), 2):
    level = int(cells[i])
    body = cells[i + 1].split("</td>")[0]
    for name in re.findall(r'<a href="/[^"]+" title="[^"]*">([^<]+)</a>', body):
        name = name.strip()
        if name and not name.startswith("Level"):
            levels.setdefault(name, level)

with open(sys.argv[1], "w") as f:
    for name in sorted(levels):
        f.write(f"{name}\t{levels[name]}\n")
print(f"{len(levels)} creatures", file=sys.stderr)
PY

rm -f vendor/.wiki_levels.json
echo "wrote $out"
