#!/usr/bin/env bash
# Harvest armor from a lich-5 checkout into the codex vocabulary.
#
#   scripts/lich-armor.sh ../lich-5
#
# What a body is wearing decides more than its defence. Scale Armor
# Proficiency makes martial stances free, but only in scale (sub-groups 9-12);
# armor hinders spells by circle; every sub-group carries an action penalty and
# a minimum roundtime. The engine needs all of that per sub-group, and the
# nouns that name each one.
#
# lich-5 keeps the table in `lib/gemstone/armaments/armor_stats.rb`: eighteen
# sub-groups, each with its type, penalties, spell hindrance by circle, the
# Armor Use training that clears each hindrance, and every noun it goes by.
# Harvested the way creatures are -- a bootstrap from lich's measurement, pinned
# to a commit in `vendor/lich_armor.source`, not a feed.
#
# **A noun does not decide the sub-group.** 38 of the 109 name more than one,
# and five name more than one *type* -- a "breastplate" is leather (scale, 9) or
# metal (plate, 17). So nouns are many-to-many here, and the game's own answer
# comes from `inspect`: "...allows you to conclude that it is <type>."
set -euo pipefail
cd "$(dirname "$0")/.."

lich="${1:?usage: scripts/lich-armor.sh <lich-5 checkout>}"
src="$lich/lib/gemstone/armaments/armor_stats.rb"
commit="$(git -C "$lich" rev-parse HEAD)"

python3 - "$src" <<'PY'
import re, sys

src = open(sys.argv[1]).read()
body = src[: src.index("def self.")]

def nums(s):
    return [None if x.strip() == "nil" else int(x) for x in s.split(",")]

FIELDS = re.compile(
    r":type\s*=>\s*:(?P<type>\w+),\s*"
    r":base_name\s*=>\s*:(?P<base>\w+),\s*"
    r":all_names\s*=>\s*\[(?P<names>.*?)\],\s*"
    r":armor_group\s*=>\s*(?P<ag>\d+),\s*"
    r":armor_sub_group\s*=>\s*(?P<asg>\d+),\s*"
    r":base_weight\s*=>\s*(?P<weight>\d+),\s*"
    r":min_rt\s*=>\s*(?P<min_rt>\d+),\s*"
    r":action_penalty\s*=>\s*(?P<ap>-?\d+),\s*"
    r":normal_cva\s*=>\s*(?P<cva>-?\d+),\s*"
    r":magical_cva\s*=>\s*(?P<mcva>-?\d+),\s*"
    r"(?:#[^\n]*\n\s*)?:hindrances\s*=>\s*\[(?P<hind>[^\]]*)\],\s*"
    r":hindrance_max\s*=>\s*(?P<hmax>\d+),\s*"
    r"(?:#[^\n]*\n\s*)?:training_reqs\s*=>\s*\[(?P<reqs>[^\]]*)\]",
    re.S,
)

# Index 0 of both arrays is the action penalty repeated, not a circle; 8, 15
# and 18 are unused (all nil in the source). Names from the file's own header.
CIRCLES = {
    1: "Minor Spiritual", 2: "Major Spiritual", 3: "Cleric Base",
    4: "Minor Elemental", 5: "Major Elemental", 6: "Ranger Base",
    7: "Sorcerer Base", 9: "Wizard Base", 10: "Bard Base",
    11: "Empath Base", 12: "Minor Mental", 13: "Major Mental",
    14: "Savant Base", 16: "Paladin Base", 17: "Arcane Spells",
    19: "Lost Arts",
}

# What each sub-group covers, from the same file's `find_coverage`. Parsed
# from the source rather than restated, so a correction upstream arrives here.
cov_src = src[src.index("def self.find_coverage"):]
cov_src = cov_src[: cov_src.index("coverage.each")]
COVERAGE = {}
for name, asgs in re.findall(r"(\w+):\s*\[([\d,\s]+)\]", cov_src):
    for n in asgs.split(","):
        COVERAGE[int(n)] = name

subgroups, nouns, hindrances = [], set(), []
for m in FIELDS.finditer(body):
    asg = int(m["asg"])
    base = m["base"].replace("_", " ")
    subgroups.append((asg, int(m["ag"]), m["type"], base, int(m["weight"]),
                      int(m["min_rt"]), int(m["ap"]), int(m["cva"]),
                      int(m["mcva"]), int(m["hmax"]), COVERAGE[asg]))
    for n in re.findall(r'"([^"]+)"', m["names"]):
        # "corslet/corselet" is two spellings of one noun.
        for spelling in n.split("/"):
            nouns.add((spelling.strip().lower(), asg))
    nouns.add((base, asg))  # the base name is a noun too, as with weapons
    hind, reqs = nums(m["hind"]), nums(m["reqs"])
    for i, circle in CIRCLES.items():
        if hind[i] is not None:
            hindrances.append((asg, circle, hind[i], reqs[i]))

if len(subgroups) != 18:
    sys.exit(f"parsed {len(subgroups)} sub-groups, expected 18; the file changed shape")

def write(path, rows, header=None):
    with open(path, "w") as f:
        if header:
            f.write("\t".join(header) + "\n")
        for r in sorted(rows):
            f.write("\t".join(str(x) for x in r) + "\n")

sg_cols = ["asg", "armor_group", "type", "base_name", "base_weight", "min_rt",
           "action_penalty", "normal_cva", "magical_cva", "hindrance_max", "coverage"]
# vendor/ keeps a header for people; data/vocabulary/ has none, since the
# column order is the schema's (docs/bootstrap-format.md).
write("vendor/lich_armor_subgroups.tsv", subgroups, sg_cols)
write("vendor/lich_armor_nouns.tsv", nouns, ["noun", "asg"])
write("vendor/lich_armor_hindrances.tsv", hindrances,
      ["asg", "circle", "hindrance", "armor_use_to_clear"])
write("data/vocabulary/081_armor_subgroups.tsv", subgroups)
write("data/vocabulary/082_armor_nouns.tsv", nouns)
write("data/vocabulary/083_armor_hindrances.tsv", hindrances)
print(f"{len(subgroups)} sub-groups, {len(nouns)} nouns, {len(hindrances)} hindrances")
PY

cat > vendor/lich_armor.source <<SRC
# Where the armor table comes from.
#
# lich-5's lib/gemstone/armaments/armor_stats.rb: eighteen armor sub-groups
# with type, penalties, spell hindrance by circle, the Armor Use training that
# clears each, and the nouns each goes by. Re-run scripts/lich-armor.sh against
# a newer checkout to pick up corrections; the TSVs are committed so a build
# never needs lich.
repo=elanthia-online/lich-5
path=lib/gemstone/armaments/armor_stats.rb
commit=$commit
fetched=$(date +%F)
SRC
