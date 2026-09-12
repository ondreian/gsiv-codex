//! Building the codex: schema in, database out.
//!
//! The deliverable of this repo is a SQLite file. Everything here exists to
//! produce one and to prove things about it.

pub mod conditions;
pub mod connectors;
pub mod extract;
pub mod ingest;
pub mod lich_move;
pub mod mapdb;
pub mod overlays;
pub mod schema;
pub mod sets;
pub mod similarity;
pub mod tsv;
