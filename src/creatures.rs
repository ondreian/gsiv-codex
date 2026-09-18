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
//! somebody hunted there and wrote down the rooms. The rest of each file --
//! attack tables, defences, arrival and death messaging -- is left alone,
//! because nothing has asked a question of it yet and a column nobody queries
//! is a column that goes stale without anybody noticing.
//!
//! Read with `find` and `strip_prefix` rather than a Ruby parser or a regex
//! crate, which is what `lich_move` next door does and for the same reason:
//! these files are machine-written and this repository has three dependencies.
//! The harvest reports every file it could not read, so the day they stop
//! being machine-written is a line in the output rather than a silence.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// One creature, as much of it as a bounty needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Creature {
    pub name: String,
    pub noun: String,
    /// `None` where the creature has no fixed level, not where nobody
    /// measured one: the Grimswarm scale to whoever meets them.
    pub level: Option<u32>,
    /// Habitat name to the room ranges it was measured in.
    pub habitats: BTreeMap<String, Vec<(u64, u64)>>,
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

    Ok(Creature {
        name,
        noun,
        level,
        habitats,
    })
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

    BTreeMap::from([
        ("070_creatures.tsv", creatures),
        ("071_habitats.tsv", habitats),
        ("072_creature_rooms.tsv", rooms),
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
                },
                Creature {
                    name: "rolton".into(),
                    noun: "rolton".into(),
                    level: Some(2),
                    habitats: BTreeMap::from([("Old Mine Road".into(), vec![(20019, 20030)])]),
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
            }],
            ..Harvest::default()
        };
        assert_eq!(
            to_tsv(&harvest)["070_creatures.tsv"],
            "grimswarm orc\torc\t\\N\n"
        );
    }
}
