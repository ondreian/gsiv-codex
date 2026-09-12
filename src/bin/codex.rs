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
                eprintln!("usage: codex build --from <map.db3> --out <codex.db3> [--vocabulary <dir>]");
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
            eprintln!("usage: codex (build | stats) ...");
            ExitCode::from(2)
        }
    }
}

fn build(from: &str, out: &str, vocabulary: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // Rebuilt from scratch every time. A build that appends to whatever was
    // there before is a build whose output depends on its history, which is
    // the opposite of what a published artifact needs.
    if std::path::Path::new(out).exists() {
        std::fs::remove_file(out)?;
    }
    let conn = schema::open(out)?;

    // Vocabulary before projection: a tag map row references a facet type, and
    // the projection consults the map. Loaded from `.sql` files so the
    // decisions stay reviewable as data rather than compiled in.
    if let Some(dir) = vocabulary {
        let mut files: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "sql"))
            .collect();
        files.sort();
        for path in &files {
            conn.execute_batch(&std::fs::read_to_string(path)?)?;
            eprintln!("vocabulary: {}", path.display());
        }
    }

    let s = ingest::from_urnon_store(&conn, from)?;
    eprintln!(
        "rooms {}, edges {} (skipped {} script, {} ambiguous), locations {}",
        s.rooms, s.edges, s.edges_skipped_script, s.edges_skipped_ambiguous, s.locations
    );

    let facets = ingest::project_tags(&conn, from)?;
    eprintln!("facets from tags: {facets}");

    // Cheap here and permanent in the artifact: every consumer's first query
    // is faster for a page-ordered file, and nobody has to remember to do it.
    conn.execute_batch("VACUUM; ANALYZE;")?;
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
