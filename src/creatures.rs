//! Harvesting creatures and their habitats out of lich-5's creature files.
//!
//! Each file is a Ruby hash literal, one per creature, machine-written and
//! consistently formatted. The fields this takes are the ones a bounty needs:
//! what the thing is called, what you type at it, what level it is, and which
//! rooms it comes from.
//!
//! ```text
//! name: "kobold",
//! noun: "kobold",
//! level: 1,
//! areas: [
//!   { name: "Lower Dragonsclaw",
//!     uids: [9028..9041, 372005..372014, ...] },
//! ]
//! ```
//!
//! The uids are the measurement this repository cannot make from a map:
//! somebody hunted there and wrote down the rooms.
//!
//! How it fights comes too, now that something asks: urnon prices a hunt with
//! the game's own roll, and needs the creature's AS, DS and TD to do it
//! (`schema/024_creature_combat.sql`). Messaging, treasure and the rest stay
//! behind -- a column nobody queries goes stale without anybody noticing.
//!
//! Read with `find` and `strip_prefix` rather than a Ruby parser or a regex
//! crate, which is what `lich_move` next door does and for the same reason:
//! these files are machine-written and this repository has three dependencies.
//! The harvest reports every file it could not read, so the day they stop
//! being machine-written is a line in the output rather than a silence.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// One creature: what a bounty needs, and how it fights.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Creature {
    pub name: String,
    pub noun: String,
    /// `None` where the creature has no fixed level, not where nobody
    /// measured one: the Grimswarm scale to whoever meets them.
    pub level: Option<u32>,
    /// Habitat name to the room ranges it was measured in.
    pub habitats: BTreeMap<String, Vec<(u64, u64)>>,
    /// Everything it attacks with, in lich's six lists.
    pub attacks: Vec<Attack>,
    pub defense: Defense,
    /// `(field, value)` lich wrote as prose -- `"???"`, `"250 UAF"` -- and
    /// that this left empty rather than interpret.
    pub unparsed: Vec<(String, String)>,
}

/// A measured range. A single value is `lo == hi`.
pub type Span = (i64, i64);

/// One entry of an attack list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attack {
    /// lich's list: `physical_attacks`, `bolt_spells`, ...
    pub kind: &'static str,
    pub name: String,
    /// `as` or `cs`, whichever the entry carries.
    pub roll: Option<&'static str>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Defense {
    pub max_hp: Option<i64>,
    pub asg: Option<i64>,
    /// lich writes a hide or shell as `"12N"`: sub-group 12, natural.
    pub asg_natural: bool,
    pub melee: Option<Span>,
    pub ranged: Option<Span>,
    pub bolt: Option<Span>,
    pub udf: Option<Span>,
    /// Circle abbreviation to TD.
    pub td: BTreeMap<String, Span>,
}

/// lich's attack lists, in the order it writes them.
const ATTACK_KINDS: [&str; 6] = [
    "physical_attacks",
    "bolt_spells",
    "warding_spells",
    "offensive_spells",
    "maneuvers",
    "special_abilities",
];

/// A value with its trailing comma and any `# comment` gone. A quoted value
/// is cut at its closing quote, so a `#` inside a name survives.
fn clean(value: &str) -> &str {
    let v = value.trim();
    if let Some(inner) = v.strip_prefix('"') {
        return match inner.find('"') {
            Some(end) => &v[..end + 2],
            None => v,
        };
    }
    let v = v.split(" #").next().unwrap_or(v).trim_end();
    v.strip_suffix(',').unwrap_or(v).trim_end()
}

/// `N`, `(N..N)`, `(N)`, `"N"` or `nil`. `Err` is prose, for the report.
fn span(value: &str) -> Result<Option<Span>, ()> {
    let v = clean(value);
    if v == "nil" {
        return Ok(None);
    }
    let v = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(v);
    let v = v.strip_prefix('(').and_then(|v| v.strip_suffix(')')).unwrap_or(v);
    let (lo, hi) = v.split_once("..").unwrap_or((v, v));
    match (lo.trim().parse(), hi.trim().parse()) {
        (Ok(lo), Ok(hi)) if hi >= lo => Ok(Some((lo, hi))),
        _ => Err(()),
    }
}

/// The body between `\n<indent><key>: <open>` and the line that closes it at
/// the same indent. `None` for an absent or inline-empty (`[],`) block.
fn block<'a>(text: &'a str, indent: &str, key: &str, open: char, close: char) -> Option<&'a str> {
    let head = format!("\n{indent}{key}: {open}\n");
    let start = text.find(head.as_str())? + head.len() - 1;
    let rest = &text[start..];
    let end = rest.find(format!("\n{indent}{close}").as_str())?;
    Some(&rest[..end])
}

/// `key: value` lines at exactly `indent`, commented lines skipped.
fn fields<'a>(body: &'a str, indent: &str) -> Vec<(&'a str, &'a str)> {
    body.lines()
        .filter_map(|line| line.strip_prefix(indent))
        .filter(|line| !line.starts_with(' ') && !line.starts_with('#'))
        .filter_map(|line| line.split_once(": "))
        .collect()
}

/// The `{ ... }` entries of a list block, each as its lines.
fn entries(list: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = list;
    while let Some(open) = rest.find("\n      {") {
        rest = &rest[open + "\n      {".len()..];
        let Some(close) = rest.find("\n      }") else { break };
        out.push(&rest[..close]);
        rest = &rest[close..];
    }
    out
}

/// What a harvest found, and what it could not read.
#[derive(Debug, Default)]
pub struct Harvest {
    pub creatures: Vec<Creature>,
    /// Files that parsed to nothing usable, with the reason. Reported rather
    /// than skipped: a silent drop is how a source format changes under you.
    pub unreadable: Vec<(String, String)>,
    /// Creatures with no habitat measured at all. Not an error -- the wiki
    /// knows of them and nobody has hunted there with a notebook -- but it is
    /// the burndown list, so it comes back rather than vanishing.
    pub without_habitat: Vec<String>,
    /// `(creature, field, value)` left empty because lich wrote prose there.
    pub unparsed: Vec<(String, String, String)>,
}

/// The text after `  <key>: ` on the one line that starts with it.
///
/// Two spaces, deliberately: `name:` appears again six spaces in, inside every
/// entry of `areas`, and a search that ignored the indent would return the
/// first *area's* name as the creature's.
fn top_level<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let head = format!("  {key}: ");
    text.lines()
        .find_map(|line| line.strip_prefix(head.as_str()))
        .map(str::trim_end)
        .map(|v| v.strip_suffix(',').unwrap_or(v))
}

/// The contents of a `"..."` value, empty string included.
fn unquote(value: &str) -> Option<&str> {
    value.strip_prefix('"')?.strip_suffix('"')
}

/// The body of the `areas: [ ... ]` block.
fn areas_block(text: &str) -> Option<&str> {
    let start = text.find("\n  areas: [")? + "\n  areas: [".len();
    let rest = &text[start..];
    let end = rest.find("\n  ],")?;
    Some(&rest[..end])
}

/// `9028..9041, 372005..372014` -> the pairs, skipping anything malformed.
fn ranges_of(list: &str) -> Vec<(u64, u64)> {
    list.split(',')
        .filter_map(|piece| {
            let (lo, hi) = piece.trim().split_once("..")?;
            let lo: u64 = lo.trim().parse().ok()?;
            let hi: u64 = hi.trim().parse().ok()?;
            (hi >= lo).then_some((lo, hi))
        })
        .collect()
}

/// Every `{ name: "...", uids: [...] }` in an areas block, in order.
fn area_entries(block: &str) -> Vec<(String, Vec<(u64, u64)>)> {
    let mut out = Vec::new();
    let mut rest = block;
    while let Some(at) = rest.find("name: \"") {
        rest = &rest[at + "name: \"".len()..];
        let Some(close) = rest.find('"') else { break };
        let name = rest[..close].trim().to_string();
        rest = &rest[close + 1..];

        // The uids belong to this entry only if they arrive before the next
        // one does; an area with no measured rooms has `uids: []` and is
        // skipped by `ranges_of` returning nothing.
        let next_name = rest.find("name: \"").unwrap_or(rest.len());
        let ranges = match rest[..next_name].find("uids: [") {
            Some(u) => {
                let after = &rest[u + "uids: [".len()..next_name];
                match after.find(']') {
                    Some(end) => ranges_of(&after[..end]),
                    None => Vec::new(),
                }
            }
            None => Vec::new(),
        };
        if !name.is_empty() && !ranges.is_empty() {
            out.push((name, ranges));
        }
    }
    out
}

/// Read one creature file.
pub fn parse_one(text: &str) -> Result<Creature, String> {
    let name = top_level(text, "name")
        .and_then(unquote)
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| "no name".to_string())?
        .to_string();

    // An empty `noun` in the source means "same as the name", which the
    // template says in as many words. Resolved here so nothing downstream has
    // to know the convention.
    let noun = top_level(text, "noun")
        .and_then(unquote)
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map_or_else(|| name.clone(), str::to_string);

    let level = top_level(text, "level").and_then(|v| v.trim().parse().ok());

    let mut habitats: BTreeMap<String, Vec<(u64, u64)>> = BTreeMap::new();
    if let Some(block) = areas_block(text) {
        for (area, ranges) in area_entries(block) {
            habitats.entry(area).or_default().extend(ranges);
        }
    }
    for ranges in habitats.values_mut() {
        ranges.sort_unstable();
        ranges.dedup();
    }

    let mut unparsed = Vec::new();
    let mut read = |field: &str, value: &str| match span(value) {
        Ok(v) => v,
        Err(()) => {
            unparsed.push((field.to_string(), clean(value).to_string()));
            None
        }
    };

    let mut attacks = Vec::new();
    if let Some(body) = block(text, "  ", "attack_attributes", '{', '}') {
        for kind in ATTACK_KINDS {
            let Some(list) = block(body, "    ", kind, '[', ']') else {
                continue;
            };
            for entry in entries(list) {
                let fs = fields(entry, "        ");
                let Some(name) = fs
                    .iter()
                    .find(|(k, _)| *k == "name")
                    .and_then(|(_, v)| unquote(clean(v)))
                else {
                    continue;
                };
                let rolled = fs.iter().find_map(|(k, v)| match *k {
                    "as" => Some(("as", *v)),
                    "cs" => Some(("cs", *v)),
                    _ => None,
                });
                let (roll, span) = match rolled {
                    Some((roll, v)) => (Some(roll), read(&format!("{kind}/{name}/{roll}"), v)),
                    None => (None, None),
                };
                attacks.push(Attack {
                    kind,
                    name: name.to_string(),
                    roll,
                    span,
                });
            }
        }
    }

    let mut defense = Defense {
        max_hp: top_level(text, "max_hp").and_then(|v| read("max_hp", v)).map(|(lo, _)| lo),
        ..Defense::default()
    };
    if let Some(body) = block(text, "  ", "defense_attributes", '{', '}') {
        for (key, value) in fields(body, "    ") {
            match key {
                "asg" => {
                    let v = clean(value);
                    let natural = v.trim_matches('"').ends_with('N');
                    let v = v.replacen("N\"", "\"", 1);
                    defense.asg = read(key, &v).map(|(lo, _)| lo);
                    defense.asg_natural = natural && defense.asg.is_some();
                }
                "melee" => defense.melee = read(key, value),
                "ranged" => defense.ranged = read(key, value),
                "bolt" => defense.bolt = read(key, value),
                "udf" => defense.udf = read(key, value),
                td if td.ends_with("_td") => {
                    if let Some(span) = read(key, value) {
                        defense.td.insert(td.trim_end_matches("_td").to_string(), span);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(Creature {
        name,
        noun,
        level,
        habitats,
        attacks,
        defense,
        unparsed,
    })
}

/// Base levels read off gswiki, keyed for matching against creature names.
///
/// `vendor/wiki_levels.tsv`, one `name\tlevel` per line. Names are matched
/// loosely -- case, hyphens and apostrophes differ between the two sources for
/// the same creature, and `black-winged daggerbeak` is not a different animal
/// from `Black-Winged Daggerbeak`.
pub fn read_levels(path: &Path) -> Result<BTreeMap<String, u32>, Box<dyn std::error::Error>> {
    let mut out = BTreeMap::new();
    for line in std::fs::read_to_string(path)?.lines() {
        if let Some((name, level)) = line.split_once('\t')
            && let Ok(level) = level.trim().parse()
        {
            out.insert(match_key(name), level);
        }
    }
    Ok(out)
}

/// What two spellings of one creature have in common.
fn match_key(name: &str) -> String {
    name.to_lowercase()
        .replace('\u{2019}', "'")
        .chars()
        .map(|c| if c == '-' { ' ' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// What the wiki said that lich-5 did not, and where the two disagree.
#[derive(Debug, Default)]
pub struct Levelling {
    /// Creatures that had no level and now have one.
    pub filled: Vec<(String, u32)>,
    /// `(creature, lich, wiki)` where the two differ. Lich's value is kept.
    ///
    /// It is the measurement: somebody fought the thing and wrote down what it
    /// was. The wiki is a reckoning of the same creature by people who mostly
    /// did the same, and the eight it disagrees about differ by one or two --
    /// which is drift, not a dispute worth arbitrating twice.
    ///
    /// Reported rather than silenced, because a re-run that quietly took a
    /// different answer than the last one should say so.
    pub disagreed: Vec<(String, u32, u32)>,
    /// Ours that the wiki has never heard of.
    pub unknown_to_wiki: Vec<String>,
}

/// Fill in the levels lich-5 did not have.
///
/// Fills only. Where lich has a level, lich keeps it: this runs once to
/// bootstrap a table that did not exist, and after it the codex is the record
/// -- so the question is not "which source is right today" but "what did the
/// measurement say", and only one of the two sources is a measurement.
pub fn apply_levels(harvest: &mut Harvest, wiki: &BTreeMap<String, u32>) -> Levelling {
    let mut out = Levelling::default();
    for creature in &mut harvest.creatures {
        let Some(&wiki_level) = wiki.get(&match_key(&creature.name)) else {
            out.unknown_to_wiki.push(creature.name.clone());
            continue;
        };
        match creature.level {
            None => {
                creature.level = Some(wiki_level);
                out.filled.push((creature.name.clone(), wiki_level));
            }
            Some(ours) if ours != wiki_level => {
                out.disagreed
                    .push((creature.name.clone(), ours, wiki_level));
            }
            Some(_) => {}
        }
    }
    out
}

/// Read every `*.rb` in a lich-5 `lib/gemstone/creatures` directory.
pub fn harvest(dir: &Path) -> Result<Harvest, Box<dyn std::error::Error>> {
    let mut out = Harvest::default();
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "rb"))
        // The template is the format's documentation, not a creature.
        .filter(|p| p.file_stem().is_some_and(|s| s != "_creature_template"))
        .collect();
    paths.sort();

    for path in paths {
        let who = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = std::fs::read_to_string(&path)?;
        match parse_one(&text) {
            Ok(creature) => {
                for (field, value) in &creature.unparsed {
                    out.unparsed
                        .push((creature.name.clone(), field.clone(), value.clone()));
                }
                if creature.habitats.is_empty() {
                    out.without_habitat.push(creature.name.clone());
                }
                out.creatures.push(creature);
            }
            Err(why) => out.unreadable.push((who, why)),
        }
    }
    out.creatures.sort_by(|a, b| a.name.cmp(&b.name));
    out.without_habitat.sort();
    Ok(out)
}

/// The three TSVs, in the order the loader applies them: a child table never
/// arrives before the parent its foreign key needs.
pub fn to_tsv(harvest: &Harvest) -> BTreeMap<&'static str, String> {
    let mut creatures = String::new();
    for c in &harvest.creatures {
        let level = c.level.map_or_else(|| r"\N".to_string(), |l| l.to_string());
        let _ = writeln!(creatures, "{}\t{}\t{}", c.name, c.noun, level);
    }

    // Habitats are collected across creatures: two creatures naming the same
    // area are naming one place.
    let mut named: BTreeMap<&str, ()> = BTreeMap::new();
    let mut rooms = String::new();
    for c in &harvest.creatures {
        for (habitat, ranges) in &c.habitats {
            named.insert(habitat.as_str(), ());
            // Merged per creature per habitat, not across creatures: two
            // measurements of the same creature in the same area are one
            // stretch of rooms, and two creatures are not.
            let mut sorted = ranges.clone();
            sorted.sort_unstable();
            for (lo, hi) in merge(&sorted) {
                let _ = writeln!(rooms, "{}\t{habitat}\t{lo}\t{hi}", c.name);
            }
        }
    }

    let mut habitats = String::new();
    for name in named.keys() {
        let _ = writeln!(habitats, "{name}\t");
    }

    let cell = |v: Option<i64>| v.map_or_else(|| r"\N".to_string(), |v| v.to_string());
    let lo = |s: Option<Span>| cell(s.map(|(lo, _)| lo));
    let hi = |s: Option<Span>| cell(s.map(|(_, hi)| hi));

    // Primary-key order: creature, then kind by name, then position.
    let mut attack_rows: Vec<(&str, &str, usize, String)> = Vec::new();
    let mut defenses = String::new();
    let mut tds = String::new();
    for c in &harvest.creatures {
        let mut ords: BTreeMap<&str, usize> = BTreeMap::new();
        for a in &c.attacks {
            let ord = ords.entry(a.kind).or_default();
            let roll = a.roll.unwrap_or(r"\N");
            attack_rows.push((
                &c.name,
                a.kind,
                *ord,
                format!("{}\t{roll}\t{}\t{}", a.name, lo(a.span), hi(a.span)),
            ));
            *ord += 1;
        }
        let d = &c.defense;
        let _ = writeln!(
            defenses,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            c.name,
            cell(d.max_hp),
            cell(d.asg),
            u8::from(d.asg_natural),
            lo(d.melee),
            hi(d.melee),
            lo(d.ranged),
            hi(d.ranged),
            lo(d.bolt),
            hi(d.bolt),
            lo(d.udf),
            hi(d.udf),
        );
        for (circle, (lo, hi)) in &d.td {
            let _ = writeln!(tds, "{}\t{circle}\t{lo}\t{hi}", c.name);
        }
    }
    attack_rows.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
    let mut attacks = String::new();
    for (creature, kind, ord, rest) in attack_rows {
        let _ = writeln!(attacks, "{creature}\t{kind}\t{ord}\t{rest}");
    }

    BTreeMap::from([
        ("070_creatures.tsv", creatures),
        ("071_habitats.tsv", habitats),
        ("072_creature_rooms.tsv", rooms),
        ("073_creature_attacks.tsv", attacks),
        ("074_creature_defenses.tsv", defenses),
        ("075_creature_tds.tsv", tds),
    ])
}

/// Fold overlapping and touching ranges together.
///
/// Two creatures in one area rarely walked the same rooms, so the union
/// arrives as fragments: `9028..9041` from one and `9030..9052` from another
/// are one corridor. Left apart they would be two rows that overlap, and a
/// count of rooms would be wrong by the overlap.
fn merge(sorted: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut out: Vec<(u64, u64)> = Vec::new();
    for &(lo, hi) in sorted {
        match out.last_mut() {
            // `lo <= hi_prev + 1` folds touching ranges too: 100..110 and
            // 111..120 are one run of rooms and reading them as two is an
            // artefact of who measured which half.
            Some(prev) if lo <= prev.1.saturating_add(1) => prev.1 = prev.1.max(hi),
            _ => out.push((lo, hi)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const KOBOLD: &str = r#"{
  schema_version: 3,
  name: "kobold",
  noun: "kobold",
  level: 1,
  areas: [
    {
      name: "Briar Thicket",
      uids: [14013001..14013018]
    },
    {
      name: "Lower Dragonsclaw",
      uids: [9028..9041, 372005..372014]
    }
  ],
  max_hp: 40
}"#;

    #[test]
    fn reads_the_fields_a_bounty_needs() {
        let c = parse_one(KOBOLD).expect("parses");
        assert_eq!(c.name, "kobold");
        assert_eq!(c.noun, "kobold");
        assert_eq!(c.level, Some(1));
        assert_eq!(c.habitats.len(), 2);
        assert_eq!(
            c.habitats["Lower Dragonsclaw"],
            vec![(9028, 9041), (372005, 372014)]
        );
    }

    const TROLL: &str = r#"{
  name: "ice troll",
  noun: "troll",
  level: 29,
  areas: [],
  max_hp: 235,
  attack_attributes: {
    physical_attacks: [
      {
        name: "Sword",
        as: 228
      },
      {
        name: "Freezing ball of pure cold",
        as: (179..185)
      },
      {
        name: "Lunge",
        as: "???"
      }
    ],
    bolt_spells: [],
    warding_spells: [
      # { name: "Frenzy (216)", cs: (438..450), effect: "anger" }
      {
        name: "Vertigo (1219)",
        cs: 448
      }
    ],
    maneuvers: [
      {
        name: "Pounce"
      }
    ],
  },
  defense_attributes: {
    asg: "16N",
    immunities: [],
    melee: (152..280),
    bolt: (134..179),
    wiz_td: nil,
    cle_td: 105,
    mje_td: (104..109),
  },
  messaging: {
    attacks: {
      attack: [
        "An ice troll swings {weapon} at you!"
      ]
    }
  }
}"#;

    #[test]
    fn reads_how_it_fights_ranges_as_ranges() {
        let c = parse_one(TROLL).expect("parses");
        let got: Vec<(&str, &str, Option<&str>, Option<Span>)> = c
            .attacks
            .iter()
            .map(|a| (a.kind, a.name.as_str(), a.roll, a.span))
            .collect();
        assert_eq!(
            got,
            vec![
                ("physical_attacks", "Sword", Some("as"), Some((228, 228))),
                ("physical_attacks", "Freezing ball of pure cold", Some("as"), Some((179, 185))),
                ("physical_attacks", "Lunge", Some("as"), None),
                ("warding_spells", "Vertigo (1219)", Some("cs"), Some((448, 448))),
                ("maneuvers", "Pounce", None, None),
            ]
        );
        assert_eq!(c.defense.max_hp, Some(235));
        assert_eq!(c.defense.asg, Some(16));
        assert!(c.defense.asg_natural);
        assert_eq!(c.defense.melee, Some((152, 280)));
        assert_eq!(c.defense.ranged, None);
        assert_eq!(
            c.defense.td.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>(),
            vec![("cle", (105, 105)), ("mje", (104, 109))]
        );
        // Prose is reported, never guessed at.
        assert_eq!(
            c.unparsed,
            vec![("physical_attacks/Lunge/as".to_string(), "\"???\"".to_string())]
        );
    }

    #[test]
    fn a_trailing_comment_does_not_hide_a_value() {
        let text = "{\n  name: \"x\",\n  max_hp: nil,                  # base / typical max HP\n}";
        let c = parse_one(text).expect("parses");
        assert_eq!(c.defense.max_hp, None);
        assert!(c.unparsed.is_empty());
    }

    /// The Grimswarm have no level, and the file says so with `nil`. That is a
    /// different fact from a level nobody measured, and `0` would read as one.
    #[test]
    fn a_creature_without_a_level_has_none_rather_than_zero() {
        let text = r#"{
  name: "grimswarm orc",
  noun: "orc",
  level: nil,
  areas: [
  ],
}"#;
        let c = parse_one(text).expect("parses");
        assert_eq!(c.level, None);
        assert!(c.habitats.is_empty());
    }

    /// The template says an empty `noun` means "same as the name". Resolved at
    /// harvest so nothing downstream has to know that.
    #[test]
    fn an_empty_noun_falls_back_to_the_name() {
        let text = r#"{
  name: "wild hound",
  noun: "",
  level: 5,
  areas: [
  ],
}"#;
        assert_eq!(parse_one(text).expect("parses").noun, "wild hound");
    }

    #[test]
    fn a_file_with_no_name_is_reported_rather_than_skipped() {
        assert!(parse_one("{\n  level: 3,\n}").is_err());
    }

    /// The indent is the contract. `name:` appears again inside every entry of
    /// `areas`, so a reader that ignored it would call this creature "Briar
    /// Thicket" -- which is the bug this test exists to keep out, and the
    /// reason these fixtures are laid out like the real files.
    #[test]
    fn an_area_name_is_not_mistaken_for_the_creature_name() {
        let c = parse_one(KOBOLD).expect("parses");
        assert_eq!(c.name, "kobold");
    }

    #[test]
    fn touching_and_overlapping_ranges_become_one() {
        assert_eq!(merge(&[(1, 5), (6, 9)]), vec![(1, 9)]);
        assert_eq!(merge(&[(1, 5), (3, 9)]), vec![(1, 9)]);
        assert_eq!(merge(&[(1, 5), (8, 9)]), vec![(1, 5), (8, 9)]);
        assert_eq!(merge(&[(1, 5), (1, 5)]), vec![(1, 5)]);
    }

    /// Two creatures naming one area describe one place, and the rooms are the
    /// union -- neither of them walked all of it.
    #[test]
    fn creatures_sharing_an_area_share_its_rooms() {
        let harvest = Harvest {
            creatures: vec![
                Creature {
                    name: "kobold".into(),
                    noun: "kobold".into(),
                    level: Some(1),
                    habitats: BTreeMap::from([("Old Mine Road".into(), vec![(20002, 20018)])]),
                    ..Default::default()
                },
                Creature {
                    name: "rolton".into(),
                    noun: "rolton".into(),
                    level: Some(2),
                    habitats: BTreeMap::from([("Old Mine Road".into(), vec![(20019, 20030)])]),
                    ..Default::default()
                },
            ],
            ..Harvest::default()
        };
        let tsv = to_tsv(&harvest);
        // One habitat, named once.
        assert_eq!(tsv["071_habitats.tsv"], "Old Mine Road\t\n");
        // Two creatures, each keeping the rooms it was measured in. Merging
        // these into one range would answer "where are the kobolds" with the
        // rolton's half as well.
        assert_eq!(
            tsv["072_creature_rooms.tsv"],
            "kobold\tOld Mine Road\t20002\t20018\nrolton\tOld Mine Road\t20019\t20030\n"
        );
    }

    /// The two sources spell one creature differently, and neither is wrong.
    #[test]
    fn a_name_matches_across_the_two_spellings() {
        assert_eq!(
            match_key("Black-Winged Daggerbeak"),
            "black winged daggerbeak"
        );
        assert_eq!(
            match_key("black-winged daggerbeak"),
            "black winged daggerbeak"
        );
        assert_eq!(
            match_key("cook\u{2019}s assistant"),
            match_key("cook's assistant")
        );
        assert_eq!(match_key("  ki-lin  "), "ki lin");
    }

    /// A level the wiki knows fills a gap; where it disagrees, lich's stands.
    /// Lich's files are a measurement and the wiki is a reckoning, and this
    /// runs once to bootstrap the table rather than to reconcile two feeds
    /// forever.
    #[test]
    fn the_wiki_fills_gaps_and_reports_disagreements() {
        let mut harvest = Harvest {
            creatures: vec![
                Creature {
                    name: "ki-lin".into(),
                    noun: "ki-lin".into(),
                    level: None,
                    habitats: BTreeMap::new(),
                    ..Default::default()
                },
                Creature {
                    name: "direbear".into(),
                    noun: "direbear".into(),
                    level: Some(65),
                    habitats: BTreeMap::new(),
                    ..Default::default()
                },
                Creature {
                    name: "Grimswarm".into(),
                    noun: "Grimswarm".into(),
                    level: None,
                    habitats: BTreeMap::new(),
                    ..Default::default()
                },
            ],
            ..Harvest::default()
        };
        let wiki = BTreeMap::from([("ki lin".to_string(), 28), ("direbear".to_string(), 64)]);
        let out = apply_levels(&mut harvest, &wiki);

        assert_eq!(out.filled, vec![("ki-lin".to_string(), 28)]);
        assert_eq!(harvest.creatures[0].level, Some(28));

        assert_eq!(out.disagreed, vec![("direbear".to_string(), 65, 64)]);
        assert_eq!(harvest.creatures[1].level, Some(65), "ours is left alone");

        // The Grimswarm scale, so neither source has a level and that is the
        // answer rather than a gap.
        assert_eq!(out.unknown_to_wiki, vec!["Grimswarm".to_string()]);
        assert_eq!(harvest.creatures[2].level, None);
    }

    /// The committed vocabulary loads into the committed schema.
    ///
    /// Column order is the contract between a TSV and its table, and nothing
    /// else states it: there is no header row, on purpose. So the check is to
    /// load the real files and ask a real question of them -- a kobold's
    /// habitat, which is the exact shape a bounty asks.
    #[test]
    fn the_committed_creatures_load_and_answer_a_bounty() {
        let conn = crate::schema::open_in_memory().expect("open");
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/vocabulary");
        for name in [
            "070_creatures.tsv",
            "071_habitats.tsv",
            "072_creature_rooms.tsv",
        ] {
            crate::tsv::load(&conn, &dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        }

        // "kill 12 kobolds in Lower Dragonsclaw" -> the rooms to walk to.
        let rooms: Vec<(i64, i64)> = conn
            .prepare(
                "SELECT lo, hi FROM creature_rooms
                  WHERE creature = ?1 AND habitat = ?2
                  ORDER BY lo",
            )
            .expect("prepare")
            .query_map(("kobold", "Lower Dragonsclaw"), |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows");
        assert!(
            rooms.contains(&(9028, 9041)),
            "the kobold's own measured stretch, not the whole area: {rooms:?}"
        );

        // The Grimswarm have no level, and it has to survive the round trip as
        // NULL rather than as the two characters the TSV spells it with.
        let level: Option<i64> = conn
            .query_row(
                "SELECT level FROM creatures WHERE name = ?1",
                ["Grimswarm"],
                |row| row.get(0),
            )
            .expect("grimswarm");
        assert_eq!(level, None);
    }

    #[test]
    fn a_level_that_is_absent_is_written_as_the_null_cell() {
        let harvest = Harvest {
            creatures: vec![Creature {
                name: "grimswarm orc".into(),
                noun: "orc".into(),
                level: None,
                habitats: BTreeMap::new(),
                ..Default::default()
            }],
            ..Harvest::default()
        };
        assert_eq!(
            to_tsv(&harvest)["070_creatures.tsv"],
            "grimswarm orc\torc\t\\N\n"
        );
    }
}
