//! `oracle-verify` against temporary oracle directories: real `git`
//! checkouts, stand-in runtime files, and a canned probe (no LuaJIT
//! needed). Platform `linux-x86_64` on every host, so the file set is the
//! same everywhere.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::*;

const PLATFORM: &str = "linux-x86_64";
const COMMITTED: &[u8] = include_bytes!("../../../oracle/manifest.json");
const GOOD_PROBE: &str = "version=LuaJIT 2.1.1784580905\nsecurity.prng=0\nsecurity.strhash=1\nsecurity.strid=1\nsecurity.mcode=1\nmodule=5\n";
const OTHER_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn run_git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=loom",
            "-c",
            "user.email=loom@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// Commits `files` into a new repository at `dir`; returns (commit, tree).
fn commit_repo(dir: &Path, files: &[(&str, &str)]) -> (String, String) {
    std::fs::create_dir_all(dir).unwrap();
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "core.autocrlf", "false"]);
    for (name, text) in files {
        let path = dir.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    run_git(dir, &["add", "-A"]);
    run_git(dir, &["commit", "-q", "-m", "fixture"]);
    (
        run_git(dir, &["rev-parse", "HEAD^{commit}"]),
        run_git(dir, &["rev-parse", "HEAD^{tree}"]),
    )
}

/// A complete oracle directory that passes, and its manifest.
struct Oracle {
    dir: PathBuf,
    manifest: serde_json::Value,
}

impl Oracle {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("loom-oracle-verify-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (pob_commit, pob_tree) = commit_repo(
            &dir.join("pob"),
            &[
                ("src/HeadlessWrapper.lua", "-- wrapper\n"),
                (".gitignore", "src/Data/*.bin\n"),
            ],
        );
        let (luajit_commit, _) = commit_repo(&dir.join("src/luajit"), &[("README", "luajit\n")]);
        let (utf8_commit, _) = commit_repo(&dir.join("src/luautf8"), &[("lutf8lib.c", "utf8\n")]);
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("bin/luajit"), "luajit binary").unwrap();
        std::fs::write(dir.join("bin/lua-utf8.so"), "utf8 module").unwrap();

        let mut manifest: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        manifest["pob"]["commit"] = pob_commit.into();
        manifest["pob"]["tree"] = pob_tree.into();
        manifest["luajit"]["commit"] = luajit_commit.clone().into();
        manifest["luautf8"]["commit"] = utf8_commit.clone().into();

        let record = serde_json::json!({
            "format": 1,
            "platform": PLATFORM,
            "luajit_commit": luajit_commit,
            "luautf8_commit": utf8_commit,
            "defines": manifest["luajit"]["defines"],
            "toolchain": "cc (test) 1.0",
            "files": {
                "bin/luajit": oracle::sha256_hex(b"luajit binary"),
                "bin/lua-utf8.so": oracle::sha256_hex(b"utf8 module"),
            },
        });
        std::fs::write(dir.join("build.json"), record.to_string()).unwrap();
        Self { dir, manifest }
    }

    fn verify_with(&self, probe: Result<&str, &str>) -> Report {
        let manifest = self.manifest.to_string();
        verify(Ok(manifest.as_bytes()), &self.dir, PLATFORM, |_, _| {
            probe.map(str::to_owned).map_err(str::to_owned)
        })
    }

    fn verify(&self) -> Report {
        self.verify_with(Ok(GOOD_PROBE))
    }

    fn edit_record(&self, edit: impl FnOnce(&mut serde_json::Value)) {
        let path = self.dir.join("build.json");
        let mut record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        edit(&mut record);
        std::fs::write(path, record.to_string()).unwrap();
    }
}

impl Drop for Oracle {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn failed(report: &Report) -> Vec<&str> {
    report
        .checks
        .iter()
        .filter(|c| !c.ok)
        .map(|c| c.name.as_str())
        .collect()
}

#[test]
fn verify_should_pass_a_complete_oracle() {
    let oracle = Oracle::new("ok");
    let report = oracle.verify();
    assert_eq!(failed(&report), Vec::<&str>::new(), "{report:#?}");
    assert_eq!(report.status, "ok");
    assert_eq!(report.class, None);
    assert_eq!(report.toolchain.as_deref(), Some("cc (test) 1.0"));
}

#[test]
fn verify_should_classify_wrong_pob_commit_as_acquisition() {
    let mut oracle = Oracle::new("pob-commit");
    oracle.manifest["pob"]["commit"] = OTHER_COMMIT.into();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(failed(&report), ["pob.commit"]);
}

#[test]
fn verify_should_classify_wrong_pob_tree_as_acquisition() {
    let mut oracle = Oracle::new("pob-tree");
    oracle.manifest["pob"]["tree"] = OTHER_COMMIT.into();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(failed(&report), ["pob.tree"]);
}

#[test]
fn verify_should_classify_missing_checkout_as_acquisition() {
    let oracle = Oracle::new("pob-missing");
    std::fs::remove_dir_all(oracle.dir.join("pob")).unwrap();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(failed(&report), ["pob.commit", "pob.tree", "pob.clean"]);
    assert_eq!(report.checks[0].actual, "error: not a git checkout");
}

#[test]
fn verify_should_classify_untracked_file_as_acquisition() {
    let oracle = Oracle::new("pob-untracked");
    std::fs::write(oracle.dir.join("pob/src/extra.lua"), "--").unwrap();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(failed(&report), ["pob.clean"]);
    assert_eq!(report.checks[2].actual, "1 entries: ?? src/extra.lua");
}

#[test]
fn verify_should_classify_ignored_file_as_acquisition() {
    // Like PoB's timeless-jewel cache: hidden by the checkout's .gitignore.
    let oracle = Oracle::new("pob-ignored");
    std::fs::create_dir_all(oracle.dir.join("pob/src/Data")).unwrap();
    std::fs::write(oracle.dir.join("pob/src/Data/jewel.bin"), "cache").unwrap();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(failed(&report), ["pob.clean"]);
    assert_eq!(report.checks[2].actual, "1 entries: !! src/Data/jewel.bin");
}

#[test]
fn verify_should_classify_modified_file_as_acquisition() {
    let oracle = Oracle::new("pob-modified");
    std::fs::write(
        oracle.dir.join("pob/src/HeadlessWrapper.lua"),
        "-- edited\n",
    )
    .unwrap();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(failed(&report), ["pob.clean"]);
}

#[test]
fn verify_should_classify_wrong_runtime_source_as_acquisition() {
    let mut oracle = Oracle::new("luajit-commit");
    oracle.manifest["luajit"]["commit"] = OTHER_COMMIT.into();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    // The build record names the old commit too.
    assert_eq!(failed(&report), ["luajit.commit", "build.luajit_commit"]);
}

#[test]
fn verify_should_classify_malformed_manifest_as_acquisition() {
    let oracle = Oracle::new("manifest");
    let report = verify(Ok(b"{ not json"), &oracle.dir, PLATFORM, |_, _| {
        Ok(GOOD_PROBE.to_owned())
    });
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(failed(&report), ["manifest"]);
}

#[test]
fn verify_should_classify_missing_luajit_as_environment() {
    let oracle = Oracle::new("luajit-missing");
    std::fs::remove_file(oracle.dir.join("bin/luajit")).unwrap();
    let report = oracle.verify_with(Err("cannot start luajit: not found"));
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["file:bin/luajit", "runtime.start"]);
}

#[test]
fn verify_should_classify_unreadable_manifest_as_acquisition() {
    let oracle = Oracle::new("manifest-unreadable");
    let report = verify(
        Err("manifest file: not found".to_owned()),
        &oracle.dir,
        PLATFORM,
        |_, _| Ok(GOOD_PROBE.to_owned()),
    );
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(failed(&report), ["manifest"]);
    assert_eq!(report.manifest_sha256, None);
}

#[test]
fn verify_should_check_extra_files_the_record_lists() {
    let oracle = Oracle::new("record-extra");
    oracle.edit_record(|r| r["files"]["bin/jit/vmdef.lua"] = "00".into());
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["file:bin/jit/vmdef.lua"]);
}

#[test]
fn verify_should_reject_build_record_with_unknown_field() {
    let oracle = Oracle::new("record-unknown");
    oracle.edit_record(|r| r["patched"] = true.into());
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert!(
        report
            .checks
            .iter()
            .any(|c| c.name == "build.platform" && c.actual.contains("unknown field"))
    );
}

#[test]
fn describe_status_should_count_all_and_show_five() {
    let porcelain = (1..=7)
        .map(|i| format!("?? f{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        describe_status(&porcelain),
        "7 entries: ?? f1; ?? f2; ?? f3; ?? f4; ?? f5"
    );
    assert_eq!(describe_status(""), "clean");
}

#[test]
fn run_probe_should_report_missing_runtime() {
    let missing = std::env::temp_dir().join(format!(
        "loom-oracle-verify-no-runtime-{}/bin/luajit",
        std::process::id()
    ));
    let err = run_probe(&missing, "print(1)").unwrap_err();
    assert!(err.starts_with("cannot start luajit"), "{err}");
}

#[test]
fn verify_should_classify_changed_luajit_as_environment() {
    let oracle = Oracle::new("luajit-changed");
    std::fs::write(oracle.dir.join("bin/luajit"), "another luajit").unwrap();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["file:bin/luajit"]);
}

#[test]
fn verify_should_classify_missing_module_as_environment() {
    let oracle = Oracle::new("module-missing");
    std::fs::remove_file(oracle.dir.join("bin/lua-utf8.so")).unwrap();
    let report = oracle.verify_with(Err("exit code: 1: module 'lua-utf8' not found"));
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["file:bin/lua-utf8.so", "runtime.start"]);
}

#[test]
fn verify_should_classify_missing_build_record_as_environment() {
    let oracle = Oracle::new("record-missing");
    std::fs::remove_file(oracle.dir.join("build.json")).unwrap();
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(
        failed(&report),
        [
            "build.platform",
            "build.luajit_commit",
            "build.luautf8_commit",
            "build.defines",
            "file:bin/luajit",
            "file:bin/lua-utf8.so",
        ]
    );
}

#[test]
fn verify_should_classify_build_with_other_defines_as_environment() {
    let oracle = Oracle::new("record-defines");
    oracle.edit_record(|r| r["defines"] = serde_json::json!([]));
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["build.defines"]);
}

#[test]
fn verify_should_classify_build_for_other_platform_as_environment() {
    let oracle = Oracle::new("record-platform");
    oracle.edit_record(|r| r["platform"] = "windows-x86_64".into());
    let report = oracle.verify();
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["build.platform"]);
}

#[test]
fn verify_should_classify_other_luajit_version_as_environment() {
    let oracle = Oracle::new("probe-version");
    let probe = GOOD_PROBE.replace("2.1.1784580905", "2.1.1720049189");
    let report = oracle.verify_with(Ok(&probe));
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["runtime.version"]);
}

#[test]
fn verify_should_classify_randomized_string_hashing_as_environment() {
    let oracle = Oracle::new("probe-prng");
    let probe = GOOD_PROBE.replace("prng=0", "prng=1");
    let report = oracle.verify_with(Ok(&probe));
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["runtime.security.prng"]);
}

#[test]
fn verify_should_classify_broken_module_as_environment() {
    let oracle = Oracle::new("probe-module");
    let probe = GOOD_PROBE.replace("module=5", "module=6");
    let report = oracle.verify_with(Ok(&probe));
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["runtime.module"]);
}

#[test]
fn verify_should_classify_unsupported_platform_as_environment() {
    let oracle = Oracle::new("platform");
    let manifest = oracle.manifest.to_string();
    let report = verify(
        Ok(manifest.as_bytes()),
        &oracle.dir,
        "macos-aarch64",
        |_, _| Ok(GOOD_PROBE.to_owned()),
    );
    assert_eq!(report.class, Some(FailureClass::Environment));
    assert_eq!(failed(&report), ["platform"]);
}

#[test]
fn verify_should_report_acquisition_when_both_classes_fail() {
    let oracle = Oracle::new("both");
    std::fs::write(oracle.dir.join("pob/src/extra.lua"), "--").unwrap();
    std::fs::remove_file(oracle.dir.join("bin/luajit")).unwrap();
    let report = oracle.verify_with(Err("cannot start luajit: not found"));
    assert_eq!(report.class, Some(FailureClass::Acquisition));
    assert_eq!(
        failed(&report),
        ["pob.clean", "file:bin/luajit", "runtime.start"]
    );
}

#[test]
fn probe_script_should_load_the_manifest_module() {
    let manifest = Manifest::parse(COMMITTED).unwrap();
    let script = probe_script(&manifest);
    assert!(
        script.starts_with("local u = require(\"lua-utf8\")"),
        "{script}"
    );
    assert!(script.contains(r#"u.len("h\195\169llo")"#), "{script}");
}
