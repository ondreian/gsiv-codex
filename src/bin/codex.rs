//! `codex build` — produce a database from a source store.
//!
//! ```text
//! codex build --from <urnon map.db3> --out <codex.db3> [--vocabulary <dir>]
//! codex stats --db <codex.db3>
//! ```

use std::process::ExitCode;

use gsiv_codex::{ingest, schema};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };

    match args.first().map(String::as_str) {
        Some("build") => {
            let (Some(from), Some(out)) = (flag("--from"), flag("--out")) else {
                eprintln!(
                    "usage: codex build --from <map.db3> --out <codex.db3> [--vocabulary <dir>]"
                );
                return ExitCode::from(2);
            };
            match build(&from, &out, flag("--vocabulary").as_deref()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some("extract") => {
            let (Some(from), Some(out)) = (flag("--from"), flag("--out")) else {
                eprintln!(
                    "usage: codex extract --from vendor/map.json --out data/050_extracted.sql"
                );
                return ExitCode::from(2);
            };
            match extract_to(&from, &out) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some("failures") => {
            let (Some(lich), Some(out)) = (flag("--lich"), flag("--out")) else {
                eprintln!("usage: codex failures --lich <lich-5 checkout> --out data/vocabulary");
                return ExitCode::from(2);
            };
            match gsiv_codex::lich_move::regenerate(
                std::path::Path::new(&lich),
                std::path::Path::new(&out),
            ) {
                Ok((c, p, r)) => {
                    eprintln!("{c} classes, {p} patterns, {r} remedies -> {out}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some("conditions") => match flag("--db") {
            Some(db) => match conditions_report(&db, flag("--floor")) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            },
            None => {
                eprintln!("usage: codex conditions --db <codex.db3> [--floor 0.5]");
                ExitCode::from(2)
            }
        },
        Some("stats") => match flag("--db") {
            Some(db) => match stats(&db) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::from(1)
                }
            },
            None => {
                eprintln!("usage: codex stats --db <codex.db3>");
                ExitCode::from(2)
            }
        },
        _ => {
            eprintln!("usage: codex (build | extract | stats | conditions) ...");
            ExitCode::from(2)
        }
    }
}

fn build(
    from: &str,
    out: &str,
    vocabulary: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Rebuilt from scratch every time. A build that appends to whatever was
    // there before is a build whose output depends on its history, which is
    // the opposite of what a published artifact needs.
    if std::path::Path::new(out).exists() {
        std::fs::remove_file(out)?;
    }
    let conn = schema::open(out)?;

    // Two phases, because the data depends on the import in both directions.
    //
    //   vocabulary/  loads *first*: the tag map names facet types, and the
    //                projection consults the map, so both must exist before a
    //                single room is read.
    //   overlays/    loads *last*: an extracted edge asserts something over
    //                the import, and a condition tagged onto an edge cannot
    //                reference a row that does not exist yet.
    //
    // The directory is the phase. A numbering convention would have worked and
    // would have been one silent mistake away from a foreign key error nobody
    // could explain.
    let load_phase = |dir: &str| -> Result<(), Box<dyn std::error::Error>> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(());
        };
        // `.sql` is a script; `.tsv` is a table named by its file. Both are
        // ordered by the numeric prefix together, so a TSV whose foreign key
        // points at a row a `.sql` inserts can be sequenced after it.
        let mut files: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "sql" || e == "tsv"))
            .collect();
        files.sort();
        for path in &files {
            if path.extension().is_some_and(|e| e == "tsv") {
                let n = gsiv_codex::tsv::load(&conn, path)?;
                eprintln!("  {} ({n} rows)", path.display());
            } else {
                conn.execute_batch(&std::fs::read_to_string(path)?)?;
                eprintln!("  {}", path.display());
            }
        }
        Ok(())
    };

    if let Some(dir) = vocabulary {
        eprintln!("vocabulary:");
        load_phase(&format!("{dir}/vocabulary"))?;
    }

    // A `.json` is the vendored mapdb -- the real bootstrap. A `.db3` is
    // urnon's already-imported store, which was the way in before this
    // existed and stays useful for comparing the two.
    if from.ends_with(".json") {
        let s = gsiv_codex::mapdb::import(&conn, &std::fs::read_to_string(from)?)?;
        eprintln!(
            "rooms {} (skipped {} unmapped, {} instanced), edges {} (skipped {} script, \
             {} dangling, {} ambiguous), facets {}",
            s.rooms,
            s.rooms_unmapped,
            s.rooms_instanced,
            s.edges,
            s.edges_script,
            s.edges_dangling,
            s.edges_ambiguous,
            s.facets
        );
    } else {
        let s = ingest::from_urnon_store(&conn, from)?;
        eprintln!(
            "rooms {}, edges {} (skipped {} script, {} ambiguous), locations {}",
            s.rooms, s.edges, s.edges_skipped_script, s.edges_skipped_ambiguous, s.locations
        );
        let facets = ingest::project_tags(&conn, from)?;
        eprintln!("facets from tags: {facets}");
    }

    // Overlays last, and that ordering is the whole point of option C: a
    // correction applied *after* the import survives a mapdb refresh, where an
    // edit to the imported rows would be silently reverted by the next one.
    if let Some(dir) = vocabulary {
        eprintln!("overlays:");
        load_phase(&format!("{dir}/overlays"))?;
    }
    let applied = gsiv_codex::overlays::apply(&conn)?;
    if applied > 0 {
        eprintln!("overlays applied: {applied}");
    }

    // Cheap here and permanent in the artifact: every consumer's first query
    // is faster for a page-ordered file, and nobody has to remember to do it.
    conn.execute_batch("VACUUM; ANALYZE;")?;

    // Out of WAL before shipping. A published file has exactly one writer --
    // this build, which has finished -- and WAL costs every reader afterwards:
    // opening a WAL database requires creating a `-shm` file *beside* it, so
    // the directory has to be writable even when the reader only reads and the
    // database file itself is 444.
    //
    // Measured, because the failure is easy to state wrongly. A 444 file in a
    // writable directory works in either mode. In a 555 directory the WAL
    // build fails with "attempt to write a readonly database" and the rollback
    // journal build plans normally -- and a shipped artifact will sit in a
    // directory somebody has locked down.
    conn.execute_batch("PRAGMA journal_mode = DELETE;")?;
    eprintln!("built {out}");
    Ok(())
}

fn stats(db: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = schema::open(db)?;
    for table in [
        "rooms",
        "edges",
        "room_facets",
        "facet_types",
        "tag_map",
        "room_sets",
        "room_set_terms",
        "connectors",
        "connector_destinations",
        "ambiguous_commands",
    ] {
        let n: i64 = conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?;
        println!("{table:24} {n:>8}");
    }
    Ok(())
}

/// Print every condition's signature, then the pairs that might be one rule.
///
/// The shortlist decides nothing. Whether two rules *should* be one is a
/// question about the game, and the answer is not in the data — so this
/// renders them side by side, compactly enough to judge, and stops.
fn conditions_report(db: &str, floor: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    use gsiv_codex::similarity;

    let floor: f64 = floor.and_then(|f| f.parse().ok()).unwrap_or(0.5);
    let conn = schema::open_any(db)?;
    let all = gsiv_codex::conditions::load_all(&conn)?;
    let list: Vec<_> = all.into_values().collect();

    println!("{} conditions\n", list.len());
    for c in &list {
        println!("  {:<24} {}", c.id, similarity::signature(c));
    }

    let candidates = similarity::shortlist(&list, floor);
    println!("\n{} pair(s) at or above {floor}:\n", candidates.len());
    for cand in &candidates {
        let mark = if cand.same_shape {
            "SAME SHAPE"
        } else {
            "          "
        };
        println!("  {mark}  {:.2}  {}  vs  {}", cand.score, cand.a, cand.b);
        for id in [&cand.a, &cand.b] {
            if let Some(c) = list.iter().find(|c| &c.id == id) {
                println!("              {:<24} {}", c.id, similarity::signature(c));
            }
        }
        println!();
    }
    Ok(())
}

/// Regenerate the extracted-command overlays from the vendored mapdb.
fn extract_to(from: &str, out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let found = gsiv_codex::extract::from_mapdb(&std::fs::read_to_string(from)?)?;
    let skipped = found
        .iter()
        .filter(|e| matches!(e.body, gsiv_codex::extract::Body::Unrecognised(_)))
        .count();
    let tagged = found
        .iter()
        .filter(|e| matches!(e.body, gsiv_codex::extract::Body::Condition(_)))
        .count();
    std::fs::write(out, gsiv_codex::extract::to_sql(&found))?;
    eprintln!(
        "{} extracted ({tagged} with a condition), {skipped} left alone -> {out}",
        found.len() - skipped
    );
    Ok(())
}
