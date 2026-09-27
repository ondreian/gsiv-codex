#!/usr/bin/env bash
# Harvest the critical tables from a lich-5 checkout into the codex vocabulary.
#
#   scripts/lich-crits.sh ../lich-5
#
# A hit that lands past the roll rolls a critical on its damage type's table,
# at a location, at a rank -- and that entry says what the hit did: extra
# damage, stun rounds, knocked prone, a limb severed, a wound, or death. How a
# body dies depends on which of those it risks (Benjamin: "warriors in plate
# armor with redux are rarely going to be critted to death, wizards in robes
# are very likely to die from crits"), so the hunt-risk model needs every
# entry, not a summary.
#
# lich-5 keeps them in lib/gemstone/critranks/*_critical_table.rb, one Ruby
# hash per damage type, each entry with the message the game prints -- kept
# here too, since it is how a crit taken is recognised in the log. Harvested
# the way armor and creatures are: a bootstrap pinned to a commit in
# vendor/lich_crits.source, not a feed.
set -euo pipefail
cd "$(dirname "$0")/.."

lich="${1:?usage: scripts/lich-crits.sh <lich-5 checkout>}"
dir="$lich/lib/gemstone/critranks"
commit="$(git -C "$lich" rev-parse HEAD)"

python3 - "$dir" <<'PY'
import glob, os, re, sys

ENTRY = re.compile(
    r'\{\s*(?P<body>:(?:type|location)\s*=>.*?)'
    r':regex\s*=>\s*/(?P<re>(?:\\.|[^/\\])*)/[a-z]*\s*\}', re.S)
SECONDARY = re.compile(
    r':secondary_wound\s*=>\s*(?:nil|\{\s*:location\s*=>\s*"(?P<loc>[^"]+)",'
    r'\s*:wound_rank\s*=>\s*(?P<rank>\d+)\s*\})')

# Where on a body a crit lands, as lich keys the tables.
LOCATIONS = {"head", "neck", "chest", "abdomen", "back", "left_arm", "right_arm",
             "left_hand", "right_hand", "left_leg", "right_leg", "left_eye",
             "right_eye", "nerves", "unspecified"}

rows, problems, total = [], [], 0
for path in sorted(glob.glob(os.path.join(sys.argv[1], "*_critical_table.rb"))):
    text = open(path).read()
    total += len(re.findall(r":type\s*=>", text))
    # The location keys the entries sit under -- `:right_arm =>` on a line
    # of its own. Each entry names its location too; where one forgot to,
    # the key it sits under says where it belongs.
    outer = [(k.start(), k.group(1).replace("_", " "))
             for k in re.finditer(r"\n\s*:([a-z_]+)\s*=>\s*\n", text)
             if k.group(1) in LOCATIONS]
    for m in ENTRY.finditer(text):
        body = m.group("body")
        def raw(key):
            f = re.search(rf':{key}\s*=>\s*("[^"]*"|[^,\n}}]+)', body)
            return f.group(1).strip() if f else None
        def text_(key):
            v = raw(key)
            return None if v in (None, "nil") else v.strip('"')
        def num(key):
            v = raw(key)
            return int(v) if v is not None and re.fullmatch(r"-?\d+", v) else None
        enclosing = next((loc for at, loc in reversed(outer) if at < m.start()), None)
        location = text_("location")
        where = f"{text_('type')}/{location or enclosing}/{text_('rank')}"
        if location is None:
            problems.append(f"{where}: no location field; took the key it sits under")
            location = enclosing
        elif enclosing and location.lower() != enclosing:
            problems.append(f"{where}: says {location!r}, sits under {enclosing!r}")
        def flag(key):
            v = raw(key)
            if v is None:
                return None
            if v in ("true", "false"):
                return int(v == "true")
            problems.append(f"{where}: {key} = {v}")
            return None
        sec = SECONDARY.search(body)
        stunned = num("stunned")
        rows.append([
            text_("type").lower(),
            location.lower(),
            num("rank"),
            num("damage"),
            text_("position"),
            flag("fatal"),
            # 999 is lich's "unknown", not ninety-nine rounds of stun.
            None if stunned == 999 else stunned,
            flag("amputated"),
            flag("crippled"),
            flag("sleeping"),
            flag("dazed"),
            flag("limb_favored"),
            num("roundtime"),
            flag("silenced"),
            flag("slowed"),
            num("wound_rank"),
            sec.group("loc") if sec and sec.group("loc") else None,
            int(sec.group("rank")) if sec and sec.group("rank") else None,
            m.group("re").replace("\t", " "),
        ])

if len(rows) != total:
    problems.append(f"read {len(rows)} entries of {total}")
keys = [(r[0], r[1], r[2]) for r in rows]
if len(set(keys)) != len(keys):
    problems.append("duplicate (type, location, rank)")

def cell(v):
    return r"\N" if v is None else str(v)

rows.sort(key=lambda r: (r[0], r[1], r[2]))
cols = ["type", "location", "rank", "damage", "position", "fatal", "stunned",
        "amputated", "crippled", "sleeping", "dazed", "limb_favored", "roundtime",
        "silenced", "slowed", "wound_rank", "secondary_location",
        "secondary_wound_rank", "message"]
for path, header in (("vendor/lich_crit_ranks.tsv", True),
                     ("data/vocabulary/087_crit_ranks.tsv", False)):
    with open(path, "w") as f:
        if header:
            f.write("\t".join(cols) + "\n")
        for r in rows:
            f.write("\t".join(cell(v) for v in r) + "\n")
print(f"{len(rows)} crit entries across {len({r[0] for r in rows})} tables, "
      f"{sum(1 for r in rows if r[5])} fatal", file=sys.stderr)
for p in problems:
    print(f"unread: {p}", file=sys.stderr)
PY

cat > vendor/lich_crits.source <<SRC
# Where the critical tables come from.
#
# lich-5's lib/gemstone/critranks/*_critical_table.rb: every damage type's
# table, by location and rank, with what each crit does and the message the
# game prints. Re-run scripts/lich-crits.sh against a newer checkout to pick
# up corrections; the TSVs are committed so a build never needs lich.
repo=elanthia-online/lich-5
path=lib/gemstone/critranks
commit=$commit
fetched=$(date +%F)
SRC
