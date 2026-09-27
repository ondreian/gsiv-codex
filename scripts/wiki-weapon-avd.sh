#!/usr/bin/env bash
# Fetch attack-versus-defense from gswiki into vendor/wiki_*.tsv.
#
# Every swing and bolt adds an AvD to its roll -- `AS - DS + AvD + d100` --
# and the AvD is fixed by what is attacking and what the defender wears. The
# wiki publishes it per weapon skill page, one `{{Weapon table <weapon>}}`
# template per weapon, each carrying a `{{Weapon table simple|...}}` row:
#
#   name | DF cloth leather scale chain plate | RT | min RT | damage type
#        | STR/DU | 17 AvDs, for armor sub-groups 1 and 5-20
#
# Bolt spells have their own table on `Bolt spell`, the same 17 columns, with
# `V` where the AvD varies with the spell cast.
#
# Same shape as scripts/wiki-weapon-nouns.sh: a wiki has no commit to pin, so
# `vendor/wiki_weapon_avd.source` records the fetch, and the TSVs are
# committed so a build never needs the network and a change to the wiki
# arrives as a reviewable diff.
set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
import datetime, json, re, sys, time, urllib.parse, urllib.request

API = "https://gswiki.play.net/api.php"
SKILLS = {
    "Edged_Weapons": "Edged Weapons",
    "Blunt_Weapons": "Blunt Weapons",
    "Two-Handed_Weapons": "Two-Handed Weapons",
    "Polearm_Weapons": "Polearm Weapons",
    "Brawling_weapons": "Brawling",
    "Ranged_Weapons": "Ranged Weapons",
    "Thrown_Weapons": "Thrown Weapons",
}
ASGS = [1] + list(range(5, 21))

def raw(titles):
    """Wikitext of up to 50 pages, by title."""
    q = urllib.parse.urlencode({
        "action": "query", "prop": "revisions", "rvprop": "content",
        "rvslots": "main", "format": "json", "formatversion": "2",
        "redirects": "1", "titles": "|".join(titles),
    })
    with urllib.request.urlopen(f"{API}?{q}", timeout=120) as r:
        pages = json.load(r)["query"]["pages"]
    time.sleep(1)  # be polite to a community wiki
    out = {}
    for p in pages:
        if "revisions" in p:
            out[p["title"]] = p["revisions"][0]["slots"]["main"]["content"]
    return out

def plain(cell):
    """`[[Slash critical table|Slash]]` -> `Slash`; bold and whitespace gone."""
    cell = re.sub(r"\[\[(?:[^|\]]*\|)?([^\]]*)\]\]", r"\1", cell)
    return cell.replace("'''", "").strip()

def number(cell):
    cell = plain(cell)
    try:
        return str(int(cell))
    except ValueError:
        return r"\N"

def template_calls(text):
    """Every `{{Weapon table simple|...}}` or `{{Weapon table entry|...}}` in
    `text`: its kind and its top-level arguments, links kept whole."""
    return [call_args(text, m) for m in
            re.finditer(r"\{\{\s*Weapon table (simple|entry)\s*\|", text, re.I)]

def call_args(text, m):
    i, depth, args, buf = m.end(), 1, [], ""
    while i < len(text) and depth:
        two = text[i:i + 2]
        if two in ("{{", "[["):
            depth += two == "{{"
            buf += two
            i += 2
            continue
        if two == "}}" and depth == 1:
            break
        if two in ("}}", "]]"):
            depth -= two == "}}"
            buf += two
            i += 2
            continue
        c = text[i]
        if c == "|" and depth == 1 and buf.count("[[") == buf.count("]]"):
            args.append(buf.replace("\n", "").strip())
            buf = ""
        else:
            buf += c
        i += 1
    args.append(buf.replace("\n", "").strip())
    kind = m.group(1).lower()
    return kind, [a for a in args if a != ""] if kind == "entry" else args

pages = raw(list(SKILLS))
skill_of = {}
templates = []
for title, text in pages.items():
    skill = SKILLS[title.replace(" ", "_")] if title.replace(" ", "_") in SKILLS else None
    if skill is None:
        sys.exit(f"unexpected page {title!r}")
    for name in re.findall(r"\{\{\s*[Ww]eapon table ([^}|]+?)\s*\}\}", text):
        if name.lower() in ("start", "end"):
            continue
        t = f"Template:Weapon table {name}"
        if t not in templates:
            templates.append(t)
        skill_of.setdefault(t, []).append(skill)

weapons, skills, avd = [], set(), []
bad = []
seen_weapons = set()

def take(kind, args, skill_list, where):
    if kind == "simple":
        name = plain(args[0]).lower()
        dfs, rt, min_rt, dtype, strdu = args[1:6], args[6], args[7], args[8], args[9]
        vals = args[10:27]
    else:
        named = {}
        for a in args:
            if "=" in a:
                k, v = a.split("=", 1)
                named[k.strip()] = v.strip()
        name = plain(named.get("weapon", "")).lower()
        dfs = [named.get(f"DF{g}", "") for g in ("cloth", "leather", "scale", "chain", "plate")]
        rt, min_rt = named.get("RT", ""), named.get("MinRT", "")
        dtype, strdu = named.get("DamageType", ""), named.get("STDUR", "")
        vals = [named.get(f"AvD{g}", "") for g in ASGS]
    if not name or len(vals) != 17:
        bad.append(where)
        return
    for skill in skill_list:
        skills.add((name, skill))
    if name in seen_weapons:
        return
    seen_weapons.add(name)
    weapons.append([name] + [plain(d) or r"\N" for d in dfs]
                   + [number(rt), number(min_rt), plain(dtype), plain(strdu)])
    for asg, v in zip(ASGS, vals):
        avd.append((name, "weapon", 0, asg, number(v)))

# Rows written inline on a skill page.
for title, text in pages.items():
    for kind, args in template_calls(text):
        take(kind, args, [SKILLS[title.replace(" ", "_")]], title)

# Rows in their own templates.
for i in range(0, len(templates), 50):
    for title, text in raw(templates[i:i + 50]).items():
        calls = template_calls(text)
        if not calls:
            bad.append(title)
        for kind, args in calls:
            take(kind, args, skill_of.get(title, []), title)

# Bolt spells: the table under "== AvDs ==".
bolt = raw(["Bolt spell"])["Bolt spell"]
section = bolt[bolt.index("== AvDs =="):]
section = section[: re.search(r"\n\s*\|\}", section).start()]
seen = {}
for row in section.split("|- align=center")[2:]:
    cells = [c.strip() for c in row.split("\n") if c.strip().startswith("|")]
    cells = [c[1:].strip() for c in cells]
    if not cells:
        continue
    label = re.sub(r"\*+", "", plain(cells[0])).strip()
    # Separator columns between armor groups are empty cells.
    values = [c for c in cells[1:] if c != ""]
    if len(values) != 17:
        bad.append(f"Bolt spell: {label}")
        continue
    variant = seen.get(label, 0)
    seen[label] = variant + 1
    for asg, v in zip(ASGS, values):
        avd.append((label, "bolt", variant, asg, number(v)))

def write(path, rows):
    with open(path, "w") as f:
        for r in sorted(rows, key=lambda r: [str(x) for x in r]):
            f.write("\t".join(str(x) for x in r) + "\n")

write("vendor/wiki_weapons.tsv", weapons)
write("vendor/wiki_weapon_skills.tsv", skills)
avd.sort(key=lambda r: (r[0], r[1], r[2], r[3]))
with open("vendor/wiki_attack_avd.tsv", "w") as f:
    for r in avd:
        f.write("\t".join(str(x) for x in r) + "\n")
# The codex's copies, loaded by file name (`084_weapons.tsv` -> `weapons`).
# Headerless like vendor/, since the column order is the schema's.
import shutil
shutil.copy("vendor/wiki_weapons.tsv", "data/vocabulary/084_weapons.tsv")
shutil.copy("vendor/wiki_weapon_skills.tsv", "data/vocabulary/085_weapon_skills.tsv")
shutil.copy("vendor/wiki_attack_avd.tsv", "data/vocabulary/086_attack_avd.tsv")
with open("vendor/wiki_weapon_avd.source", "w") as f:
    f.write(f"gswiki weapon skill pages + Weapon table templates + Bolt spell, "
            f"fetched {datetime.date.today().isoformat()}\n")
print(f"{len(weapons)} weapons, {len(skills)} weapon-skill pairs, "
      f"{len({(r[0], r[1], r[2]) for r in avd})} attacks x 17 armor groups", file=sys.stderr)
for b in bad:
    print(f"unread: {b}", file=sys.stderr)
PY
