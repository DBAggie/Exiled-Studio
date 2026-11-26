//! The command table and the argument readers of `main.rs`.

use std::collections::BTreeSet;
use std::path::Path;

use crate::build_round_trip::Format;
use crate::{COMMANDS, SurveyArgs, USAGE, command, format_of, options, round_trip_inputs};

fn args(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

/// The subcommands the usage text names, in its order.
fn usage_names() -> Vec<&'static str> {
    let mut names = Vec::new();
    for line in USAGE.lines() {
        if let Some(rest) = line.strip_prefix("  loom-parity-cli ")
            && let Some(name) = rest.split_whitespace().next()
            && names.last() != Some(&name)
        {
            names.push(name);
        }
    }
    names
}

#[test]
fn the_table_holds_each_usage_command_once_in_usage_order() {
    let table: Vec<&str> = COMMANDS.iter().map(|(name, _)| *name).collect();
    assert_eq!(table, usage_names());
    let distinct: BTreeSet<&str> = table.iter().copied().collect();
    assert_eq!(distinct.len(), table.len());
}

#[test]
fn a_command_is_found_by_its_whole_name_only() {
    for (name, _) in COMMANDS {
        assert!(command(name).is_some(), "{name}");
    }
    for bad in ["", "bundle-", "Bundle", "calc", "--out", "tree parity"] {
        assert!(command(bad).is_none(), "{bad:?}");
    }
}

#[test]
fn options_reads_values_flags_and_positional_arguments() {
    let (positional, values, flags) = options(
        &args("a --out o b --anonymous --bundle x"),
        &["--out", "--bundle"],
        &["--anonymous"],
    )
    .unwrap();
    assert_eq!(positional, ["a", "b"]);
    assert_eq!(values.get("--out").map(String::as_str), Some("o"));
    assert_eq!(values.get("--bundle").map(String::as_str), Some("x"));
    assert!(flags.contains("--anonymous"));
    // An option's value may look like an option.
    let (_, values, _) = options(&args("--out --bundle"), &["--out", "--bundle"], &[]).unwrap();
    assert_eq!(values.get("--out").map(String::as_str), Some("--bundle"));
}

#[test]
fn options_refuses_unknown_repeated_and_valueless_options() {
    let names = ["--out", "--rules"];
    for bad in [
        "a --other b",
        "--out a --out b",
        "a --out",
        "--anonymous --anonymous",
        "--scope x",
    ] {
        assert!(
            options(&args(bad), &names, &["--anonymous"]).is_none(),
            "{bad}"
        );
    }
}

#[test]
fn survey_args_reads_every_option() {
    let got = crate::survey_args(&args(
        "builds --bundle b --anonymous one.xml --scope s --data-rules r --out o --anonymous",
    ));
    assert_eq!(
        got,
        Some(SurveyArgs {
            inputs: vec!["builds".to_owned(), "one.xml".to_owned()],
            bundle: Some("b".to_owned()),
            scope: Some("s".to_owned()),
            data_rules: Some("r".to_owned()),
            out: Some("o".to_owned()),
            anonymous: true,
        })
    );
}

#[test]
fn survey_args_refuses_incomplete_repeated_and_unknown_options() {
    for bad in [
        "",
        "--bundle b --scope s --data-rules r --out o",
        "in --scope s --data-rules r --out o",
        "in --bundle b --data-rules r --out o",
        "in --bundle b --scope s --out o",
        "in --bundle b --scope s --data-rules r",
        "in --bundle b --scope s --data-rules r --out",
        "in --bundle b --bundle c --scope s --data-rules r --out o",
        "in --bundle b --scope s --data-rules r --out o --other",
    ] {
        assert_eq!(crate::survey_args(&args(bad)), None, "{bad}");
    }
}

#[test]
fn a_format_is_read_from_the_extension_in_any_case() {
    assert_eq!(format_of(Path::new("a/b.xml")), Some(Format::PobXml));
    assert_eq!(format_of(Path::new("b.XML")), Some(Format::PobXml));
    assert_eq!(format_of(Path::new("b.Json")), Some(Format::BuildDocument));
    assert_eq!(format_of(Path::new("b.txt")), None);
    assert_eq!(format_of(Path::new("xml")), None);
}

#[test]
fn inputs_are_a_folders_xml_files_or_the_named_file() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join("build-round-trip");
    let folder = dir.to_string_lossy().into_owned();
    // A folder: its *.xml only, shown under the folder as given.
    let inputs = round_trip_inputs(&[format!("{folder}/")]).unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].file, "synthetic-poe1.xml");
    assert_eq!(inputs[0].format, Format::PobXml);
    assert_eq!(
        inputs[0].path,
        format!("{}/synthetic-poe1.xml", folder.replace('\\', "/"))
    );
    assert!(!inputs[0].bytes.is_empty());
    // A file: either format, shown as given.
    let json = dir.join("synthetic-poe2.json");
    let given = json.to_string_lossy().into_owned();
    let inputs = round_trip_inputs(&[given.clone(), folder.clone()]).unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0].file, "synthetic-poe2.json");
    assert_eq!(inputs[0].format, Format::BuildDocument);
    assert_eq!(inputs[0].path, given.replace('\\', "/"));
    assert_eq!(inputs[1].file, "synthetic-poe1.xml");
    // Another extension, or a file that is not there.
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let error = round_trip_inputs(&[manifest.to_string_lossy().into_owned()]).unwrap_err();
    assert!(error.ends_with("not *.xml or *.json"), "{error}");
    let missing = dir.join("missing.xml");
    assert!(round_trip_inputs(&[missing.to_string_lossy().into_owned()]).is_err());
}
