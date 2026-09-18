//! Building the codex: schema in, database out.
//!
//! The deliverable of this repo is a SQLite file. Everything here exists to
//! produce one and to prove things about it.

pub mod conditions;
pub mod connectors;
pub mod creatures;
pub mod extract;
pub mod ferry;
pub mod fwi;
pub mod ingest;
pub mod lich_move;
pub mod mapdb;
pub mod overlays;
pub mod rift;
pub mod schema;
pub mod seeking;
pub mod sets;
pub mod similarity;
pub mod tsv;

/// The engine versions that can read a codex this crate builds.
///
/// Declared here rather than guessed by a reader, because only this side knows
/// what it used: a build that starts writing a new table has a new floor, and
/// the release that introduces it should say so in the same commit.
///
/// `URNON_MAX` empty means no known ceiling — the normal case, and a weaker
/// claim than "anything works". It says nothing has broken this yet.
// `relocated` is a disposition older engines cannot read. They refuse to guess
// at one rather than act on it, so an old client sees the vertigo line as
// unrecognised -- which is what it does today. Nothing breaks; the behaviour
// simply does not arrive until the engine has it.
pub const URNON_MIN: &str = "0.1.0";
pub const URNON_MAX: &str = "";

/// This codex release, from the crate version. Tag a release `v{VERSION}`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
