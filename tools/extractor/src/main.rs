//! Maintainer-run game-data extractor: reads a local game install (Bundles2
//! index, `.dat` tables, stat descriptions) and writes canonical bundles,
//! recording game version, schema revision, and input hashes (ADR-005).
//! Never ships GGG art or audio (ADR-000).
//!
//! Subcommands so far (Spike A, `dev/active/spike-a-bundles2-extraction/`):
//! - `list-files <install> [--decoder <program>] [--sample <n> | --all]`:
//!   reads `Bundles2/_.index.bin`, rebuilds every path, matches paths to file
//!   records, and prints counts, the index hash, every unmatched entry, and
//!   the first `n` files (default 10) or all of them.

#![warn(clippy::indexing_slicing)]

use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use loom_extractor::decompress::ExternalTool;
use loom_extractor::index::{Index, Listing};
use loom_extractor::install::{INDEX_PATH, InstallIndex, hex, read_index};

const USAGE: &str =
    "usage: loom-extractor list-files <install> [--decoder <program>] [--sample <n> | --all]

  <install>            game folder that contains Bundles2/
  --decoder <program>  Oodle decoder to run (default: unoodle on PATH;
                       see docs/spikes/spike-a-extraction.md, section 2)
  --sample <n>         print the first n files by path (default 10)
  --all                print every file";

/// Parsed `list-files` arguments.
#[derive(Debug, PartialEq, Eq)]
struct ListFiles {
    install: PathBuf,
    decoder: PathBuf,
    /// `None` prints every file.
    sample: Option<usize>,
}

fn parse_args(args: &[String]) -> Result<ListFiles, String> {
    let mut args = args.iter();
    match args.next().map(String::as_str) {
        Some("list-files") => {}
        Some(other) => return Err(format!("unknown subcommand `{other}`")),
        None => return Err("missing subcommand".to_owned()),
    }
    let mut install = None;
    let mut decoder = PathBuf::from("unoodle");
    let mut sample = Some(10);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--decoder" => {
                decoder = args.next().ok_or("--decoder needs a program")?.into();
            }
            "--sample" => {
                let n = args.next().ok_or("--sample needs a number")?;
                sample = Some(n.parse().map_err(|_| format!("bad --sample `{n}`"))?);
            }
            "--all" => sample = None,
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
            path if install.is_none() => install = Some(PathBuf::from(path)),
            extra => return Err(format!("unexpected argument `{extra}`")),
        }
    }
    Ok(ListFiles {
        install: install.ok_or("missing <install>")?,
        decoder,
        sample,
    })
}

/// Scratch directory for the decoder's files, removed on drop.
struct ScratchDir(PathBuf);

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn bundle_name(index: &Index, bundle: u32) -> &str {
    usize::try_from(bundle)
        .ok()
        .and_then(|i| index.bundles.get(i))
        .map_or("?", |b| b.name.as_str())
}

fn print_listing(
    out: &mut impl Write,
    cmd: &ListFiles,
    install: &InstallIndex,
    listing: &Listing,
) -> std::io::Result<()> {
    let index = &install.index;
    writeln!(out, "index                {INDEX_PATH}")?;
    writeln!(out, "sha256               {}", hex(&install.sha256))?;
    writeln!(
        out,
        "size                 {} bytes, decoded {} bytes",
        install.file_len, install.decoded_len
    )?;
    writeln!(out, "decoder              {}", cmd.decoder.display())?;
    writeln!(out, "bundles              {}", index.bundles.len())?;
    writeln!(out, "file records         {}", index.files.len())?;
    writeln!(out, "path reps            {}", index.path_reps.len())?;
    writeln!(out, "path data            {} bytes", index.path_data.len())?;
    writeln!(out, "listed files         {}", listing.files.len())?;
    writeln!(
        out,
        "paths without record {}",
        listing.paths_without_record.len()
    )?;
    for path in &listing.paths_without_record {
        writeln!(out, "  {path}")?;
    }
    writeln!(
        out,
        "records without path {}",
        listing.records_without_path.len()
    )?;
    for rec in &listing.records_without_path {
        writeln!(
            out,
            "  hash {:016x}  {}  offset {}  size {}",
            rec.path_hash,
            bundle_name(index, rec.bundle),
            rec.offset,
            rec.size
        )?;
    }
    writeln!(
        out,
        "dirs without path    {}",
        listing.path_reps_without_directory.len()
    )?;
    for rep in &listing.path_reps_without_directory {
        writeln!(out, "  hash {:016x}  size {}", rep.hash, rep.size)?;
    }
    let shown = cmd.sample.unwrap_or(listing.files.len());
    writeln!(out, "files (path, bundle, offset, size), first {shown}:")?;
    for file in listing.files.iter().take(shown) {
        writeln!(
            out,
            "  {}\t{}\t{}\t{}",
            file.path,
            bundle_name(index, file.record.bundle),
            file.record.offset,
            file.record.size
        )?;
    }
    out.flush()
}

fn run(cmd: &ListFiles) -> Result<(), String> {
    let scratch =
        ScratchDir(std::env::temp_dir().join(format!("loom-extractor-{}", std::process::id())));
    // `create_dir` fails if the directory exists, so a pre-made one is never used.
    std::fs::create_dir(&scratch.0)
        .map_err(|e| format!("cannot create {}: {e}", scratch.0.display()))?;
    let decoder = ExternalTool::new(&cmd.decoder, &scratch.0);

    let install =
        read_index(&cmd.install, &decoder).map_err(|e| format!("error[{}]: {e}", e.code()))?;
    let listing = install
        .index
        .list()
        .map_err(|e| format!("error[{}]: {e}", e.code()))?;

    let mut out = BufWriter::new(std::io::stdout().lock());
    print_listing(&mut out, cmd, &install, &listing)
        .map_err(|e| format!("cannot write output: {e}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = match parse_args(&args) {
        Ok(cmd) => cmd,
        Err(message) => {
            eprintln!("loom-extractor: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(&cmd) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("loom-extractor: {message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_args_should_use_defaults() {
        let cmd = parse_args(&args(&["list-files", "D:/game"])).unwrap();
        assert_eq!(
            cmd,
            ListFiles {
                install: PathBuf::from("D:/game"),
                decoder: PathBuf::from("unoodle"),
                sample: Some(10),
            }
        );
    }

    #[test]
    fn parse_args_should_read_options() {
        let cmd = parse_args(&args(&[
            "list-files",
            "--decoder",
            "x/unoodle",
            "D:/game",
            "--all",
        ]))
        .unwrap();
        assert_eq!(cmd.decoder, PathBuf::from("x/unoodle"));
        assert_eq!(cmd.sample, None);
        let cmd = parse_args(&args(&["list-files", "D:/game", "--sample", "3"])).unwrap();
        assert_eq!(cmd.sample, Some(3));
    }

    #[test]
    fn parse_args_should_reject_bad_input() {
        for bad in [
            &[][..],
            &["extract"][..],
            &["list-files"][..],
            &["list-files", "a", "b"][..],
            &["list-files", "a", "--sample", "x"][..],
            &["list-files", "a", "--decoder"][..],
            &["list-files", "a", "--bogus"][..],
        ] {
            assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
        }
    }
}
