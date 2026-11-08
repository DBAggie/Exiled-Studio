//! Maintainer-run game-data extractor: reads a local game install (Bundles2
//! index, `.dat` tables, stat descriptions) and writes canonical bundles,
//! recording game version, schema revision, and input hashes (ADR-005).
//! Never ships GGG art or audio (ADR-000).

use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("loom-extractor: not implemented yet (Spike A, docs/plan/ACTION-PLAN.md)");
    ExitCode::FAILURE
}
