//! Test double for `unoodle`, used by `tests/external_tool.rs` so CI can test
//! `ExternalTool` without an Oodle decoder. Not an Oodle decoder.
//!
//! Accepts the arguments `ExternalTool` passes
//! (`<in> --length <n> --header 0 --verbose --output <out>`). The input's
//! prefix selects the behaviour; the rest is the payload:
//! - `ok:`     write the payload, print `Extracted <payload len> bytes`
//! - `lie:`    write the payload, print `Extracted <--length> bytes`
//! - `silent:` write the payload, print nothing
//! - `fail:`   print the payload to stderr, exit with status 2

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("fake-unoodle: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let mut input = None;
    let mut output = None;
    let mut length = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--output") => output = args.next().map(PathBuf::from),
            Some("--length") => {
                length = args.next().and_then(|v| v.to_str()?.parse::<usize>().ok())
            }
            Some("--header") => {
                args.next();
            }
            Some("--verbose") => {}
            _ => input = Some(PathBuf::from(arg)),
        }
    }
    let input = input.ok_or("missing input")?;
    let output = output.ok_or("missing --output")?;
    let length = length.ok_or("missing --length")?;
    let bytes = std::fs::read(&input).map_err(|e| e.to_string())?;

    let (mode, payload) = ["ok:", "lie:", "silent:", "fail:"]
        .into_iter()
        .find_map(|p| bytes.strip_prefix(p.as_bytes()).map(|rest| (p, rest)))
        .ok_or("unknown mode prefix")?;
    if mode == "fail:" {
        eprintln!("{}", String::from_utf8_lossy(payload));
        return Ok(ExitCode::from(2));
    }
    std::fs::write(&output, payload).map_err(|e| e.to_string())?;
    match mode {
        "ok:" => println!("Extracted {} bytes", payload.len()),
        "lie:" => println!("Extracted {length} bytes"),
        _ => {}
    }
    Ok(ExitCode::SUCCESS)
}
