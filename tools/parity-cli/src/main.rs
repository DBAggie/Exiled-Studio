//! Oracle and corpus harnesses, maintainer-run. The only workspace code
//! besides the extractor that reads disk: core crates take bytes
//! (ADR-016), and every extract- or corpus-backed check is a subcommand
//! here (Spike B plan, `dev/active/spike-b-text-stat-resolution/`).
//!
//! The PoB oracle harness (drives legacy `HeadlessWrapper.lua` on pinned
//! LuaJIT at a pinned public PoB tag, ADR-006) comes with Spike C.

mod load_statdesc;

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage:
  loom-parity-cli load-statdesc <extract-dir>
      Load every stat-description file of a `loom-extractor extract` output
      through loom-data; print counts as JSON. Exit 1 if a check fails.";

fn load_statdesc(extract: &Path) -> Result<bool, String> {
    let manifest_path = extract.join("manifest.json");
    let manifest =
        std::fs::read(&manifest_path).map_err(|e| format!("{}: {e}", manifest_path.display()))?;
    let summary = load_statdesc::run(&manifest, |relative| {
        let path = extract.join(relative);
        std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    })?;
    let json = serde_json::to_string_pretty(&summary).map_err(|e| e.to_string())?;
    println!("{json}");
    for problem in &summary.problems {
        eprintln!("problem: {problem}");
    }
    Ok(summary.problems.is_empty())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [cmd, extract] if cmd == "load-statdesc" => load_statdesc(Path::new(extract)),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("loom-parity-cli: {message}");
            ExitCode::FAILURE
        }
    }
}
