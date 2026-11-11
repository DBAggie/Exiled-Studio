//! `loom-schema-gen [--check] [<dir>]`: writes the generated schemas into
//! `<dir>` (default `schemas`), or with `--check` exits 1 when a committed
//! file differs from the generated text.

use std::path::PathBuf;
use std::process::ExitCode;

#[expect(clippy::print_stderr, reason = "a command-line tool reports to stderr")]
fn main() -> ExitCode {
    let mut check = false;
    let mut dir = PathBuf::from("schemas");
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--check" => check = true,
            "-h" | "--help" => {
                eprintln!("usage: loom-schema-gen [--check] [<dir>]");
                return ExitCode::SUCCESS;
            }
            _ => dir = PathBuf::from(arg),
        }
    }
    let mut stale = 0;
    let schemas = match loom_schema_gen::schemas() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("loom-schema-gen: {e}");
            return ExitCode::FAILURE;
        }
    };
    for (name, text) in schemas {
        let path = dir.join(name);
        if check {
            if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
                eprintln!(
                    "stale: {} (run `cargo run -p loom-schema-gen`)",
                    path.display()
                );
                stale += 1;
            }
        } else if let Err(e) =
            std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, text))
        {
            eprintln!("{}: {e}", path.display());
            return ExitCode::FAILURE;
        } else {
            eprintln!("wrote {}", path.display());
        }
    }
    if stale == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
