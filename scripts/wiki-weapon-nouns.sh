#!/usr/bin/env bash
# Fetch the approved weapon nouns from gswiki into vendor/wiki_weapon_nouns.tsv.
#
# What a weapon *is* cannot be read off the item. The game hands you "a
# vultite longsword" and nothing in it says Edged -- but `riposte` needs an
# edged weapon and `clobber` needs a blunt one, so anything deciding what a
# character can do has to get from the noun to the skill.
#
# Simutronics maintains the mapping as the approved-alteration list: every
# noun a weapon of a given base may legitimately be called. Eight sections,
# one per weapon skill, each a table of `Base | Name`. The base noun is itself
# valid, so it is emitted alongside its alternates rather than only as their
# parent.
#
# Same shape as scripts/wiki-levels.sh and for the same reasons: a wiki has no
# commit to pin, so `vendor/wiki_weapon_nouns.source` records the fetch, and
# the TSV is committed so a build never needs the network and a change to the
# wiki arrives as a reviewable diff.
set -euo pipefail
cd "$(dirname "$0")/.."

page="List_of_alternative_weapon_names"
api="https://gswiki.play.net/api.php"
out=vendor/wiki_weapon_nouns.tsv

curl -fsSL --max-time 120 \
  "$api?action=parse&page=$page&prop=text&formatversion=2&format=json" \
  -o vendor/.wiki_weapon_nouns.json

python3 - "$out" <<'PY'
import html as htmllib
import json
import re
import sys

doc = json.load(open("vendor/.wiki_weapon_nouns.json"))["parse"]["text"]

# The section heading is the authority on the skill, not the cell prefix: the
# cells say "Edged Weapon - " and "Edged weapon - " in the same table.
SKILLS = {
    "Blunt_Weapons": "Blunt Weapons",
    "Brawling_Weapons": "Brawling",
    "Edged_Weapons": "Edged Weapons",
    "Polearm_Weapons": "Polearm Weapons",
    "Ranged_weapons": "Ranged Weapons",
    "Two-handed_Weapons": "Two-Handed Weapons",
    "Thrown_Weapons": "Thrown Weapons",
    "Runestaves": "Runestaff",
}

def clean(s):
    s = re.sub(r"<[^>]+>", "", s)
    return htmllib.unescape(s).strip()

# Split on the headings so each table is attributed to the section above it.
parts = re.split(r'<h3><span class="mw-headline" id="([^"]+)"', doc)
rows = {}
for i in range(1, len(parts), 2):
    skill = SKILLS.get(parts[i])
    if skill is None:
        continue
    body = parts[i + 1]
    # Stop at the next heading of any level, so a section cannot swallow the
    # table after it.
    body = re.split(r"<h[23]>", body)[0]
    for cells in re.findall(r"<tr>\s*<td>(.*?)</td>\s*<td>(.*?)</td>", body, re.S):
        base_cell, name = (clean(c) for c in cells)
        # "Edged weapon -  broadsword" -> "broadsword". Split on the spaced
        # separator, not on any hyphen: "Two-handed weapon - battle axe"
        # splits on the first hyphen into "handed weapon -  battle axe".
        base = re.split(r"\s+-\s+", base_cell)[-1].strip().lower()
        name = name.lower()
        if not base or not name:
            continue
        # The base noun is a real noun, not just a heading for its alternates.
        rows.setdefault((base, base, skill), None)
        rows.setdefault((name, base, skill), None)

if len(rows) < 400:
    sys.exit(f"only {len(rows)} nouns parsed; the page shape probably changed")

out = sys.argv[1]
with open(out, "w") as f:
    f.write("noun\tbase\tskill\n")
    for noun, base, skill in sorted(rows):
        f.write(f"{noun}\t{base}\t{skill}\n")

# And the loadable copy. `data/vocabulary/*.tsv` has no header -- the column
# order is the schema's, and a header would be a second declaration of it that
# can disagree with the first (docs/bootstrap-format.md). Written here rather
# than by a `codex` subcommand, because a subcommand for "the same file
# without its first line" is a subcommand nobody should have to find.
vocab = "data/vocabulary/080_weapon_nouns.tsv"
with open(vocab, "w") as f:
    for noun, base, skill in sorted(rows):
        f.write(f"{noun}\t{base}\t{skill}\n")
print(f"{len(rows)} nouns -> {out}, {vocab}")
PY

rm -f vendor/.wiki_weapon_nouns.json
