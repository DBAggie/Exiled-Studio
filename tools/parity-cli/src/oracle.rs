//! Shared pieces of the PoB oracle harness (Spike C,
//! `dev/active/spike-c-oracle-harness/`): the pinned manifest
//! (`tools/oracle/manifest.json`), the build record the acquire scripts
//! write (`<oracle-dir>/build.json`), and the failure classes.
//!
//! The oracle directory (`local/oracle/`, written by
//! `tools/oracle/acquire.ps1` / `acquire.sh`) holds:
//! `pob/` (PoB checkout at the pinned tag), `src/luajit/`, `src/luautf8/`
//! (runtime sources at their pinned commits), `bin/` (the built runtime),
//! and `build.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Failure classes of the spec's taxonomy
/// (`docs/spikes/spike-c-legacy-oracle.md`, "Failure taxonomy"). Only the
/// CLI assigns a class. Variants are ordered upstream first, so the
/// smallest failing class is the one a report names. Exit codes are fixed per class and never reused:
/// 10 `ORACLE_ACQUISITION`, 11 `ORACLE_ENVIRONMENT`, and, as later
/// subcommands add them, 12 `ORACLE_LAUNCH`, 13 `ORACLE_TIMEOUT`,
/// 14 `ORACLE_PROTOCOL`, 15 `ORACLE_IMPORT`, 16 `ORACLE_CALCULATION`,
/// 17 `ORACLE_NORMALIZATION`, 18 `ORACLE_NONDETERMINISM`,
/// 19 `ORACLE_EXPECTED_DIFFERENCE`, 20 `ORACLE_UNEXPECTED_DIFFERENCE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum FailureClass {
    /// The pinned source is unavailable or differs from the manifest
    /// (wrong commit, dirty checkout, unreadable manifest).
    #[serde(rename = "ORACLE_ACQUISITION")]
    Acquisition,
    /// The runtime is missing, differs from its build record, or does not
    /// report the pinned version and build flags.
    #[serde(rename = "ORACLE_ENVIRONMENT")]
    Environment,
}

impl FailureClass {
    /// The process exit code for this class.
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Acquisition => 10,
            Self::Environment => 11,
        }
    }
}

/// `tools/oracle/manifest.json`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Manifest format; this reader knows 1.
    pub format: u32,
    /// The PoB checkout the adapter runs.
    pub pob: Pob,
    /// The LuaJIT runtime.
    pub luajit: LuaJit,
    /// The `lua-utf8` C module PoB requires.
    pub luautf8: LuaUtf8,
    /// The adapter.
    pub adapter: Adapter,
}

/// The pinned PoB source.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pob {
    /// Public repository URL (fetched by the acquire scripts only).
    pub repository: String,
    /// Release tag.
    pub tag: String,
    /// Commit of the tag.
    pub commit: String,
    /// Tree hash of the commit.
    pub tree: String,
}

/// The pinned LuaJIT source and build.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LuaJit {
    /// Public repository URL.
    pub repository: String,
    /// Source commit.
    pub commit: String,
    /// `jit.version` of a build from `commit` (embeds the commit time).
    pub version: String,
    /// Preprocessor defines the acquire scripts build with.
    pub defines: Vec<String>,
    /// Expected `jit.security(<name>)` values of the build.
    pub security: BTreeMap<String, u8>,
}

/// The pinned `luautf8` source.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LuaUtf8 {
    /// Public repository URL.
    pub repository: String,
    /// Release tag.
    pub version: String,
    /// The LuaRocks package PoB's test image installs.
    pub rock: String,
    /// Commit of the tag.
    pub commit: String,
    /// Module name PoB passes to `require`.
    pub module: String,
}

/// The adapter.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Adapter {
    /// Adapter protocol version.
    pub protocol: u32,
}

impl Manifest {
    /// Parses and checks a manifest.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if manifest.format != 1 {
            return Err(format!("unsupported manifest format {}", manifest.format));
        }
        for (name, hash) in [
            ("pob.commit", &manifest.pob.commit),
            ("pob.tree", &manifest.pob.tree),
            ("luajit.commit", &manifest.luajit.commit),
            ("luautf8.commit", &manifest.luautf8.commit),
        ] {
            if !is_object_id(hash) {
                return Err(format!("{name}: not a 40-digit lowercase hex hash"));
            }
        }
        let module = &manifest.luautf8.module;
        if module.is_empty()
            || !module
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return Err(format!(
                "luautf8.module: {module:?} is not a plain module name"
            ));
        }
        // Every flag the runtime probe reads, and no other, so dropping a
        // flag cannot drop its check. PRNG must be fixed: a random seed
        // changes string hashing, and with it `pairs` order, per run.
        let flags: Vec<&str> = manifest
            .luajit
            .security
            .keys()
            .map(String::as_str)
            .collect();
        if flags != SECURITY_FLAGS {
            return Err(format!(
                "luajit.security: expected the flags {SECURITY_FLAGS:?}, found {flags:?}"
            ));
        }
        if manifest.luajit.security.get("prng") != Some(&0) {
            return Err("luajit.security.prng: must be 0 (fixed string-hash seed)".to_owned());
        }
        Ok(manifest)
    }
}

/// The `jit.security` flags of LuaJIT 2.1 (`LJ_SECURITY_MODESTRING` in
/// `src/lj_arch.h`), sorted.
pub const SECURITY_FLAGS: [&str; 4] = ["mcode", "prng", "strhash", "strid"];

/// `<oracle-dir>/build.json`, written by the acquire scripts after a build.
/// A build's hashes depend on the toolchain and, on Windows, on the build
/// path (MSVC embeds the PDB path), so they are recorded per machine here
/// rather than pinned in the manifest. The record is self-attested: its
/// hashes show that the runtime has not changed since the acquire script
/// wrote them, not that it was built from the pinned source. The runtime
/// probe in `oracle-verify` (version with the commit time, build flags)
/// is the check that ties the binary to the pin.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRecord {
    /// Record format; this reader knows 1.
    pub format: u32,
    /// Platform key the build is for (see [`platform`]).
    pub platform: String,
    /// LuaJIT commit built.
    pub luajit_commit: String,
    /// `luautf8` commit built.
    pub luautf8_commit: String,
    /// Defines the build used.
    pub defines: Vec<String>,
    /// Compiler version line.
    pub toolchain: String,
    /// SHA-256 per built file, keyed by path relative to the oracle dir.
    pub files: BTreeMap<String, String>,
}

/// The platform key of this process, as the acquire scripts write it:
/// `<os>-<arch>` (`windows-x86_64`, `linux-x86_64`).
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// The runtime files a build must provide on `platform`, relative to the
/// oracle dir: the `luajit` executable first. `None` for an unsupported
/// platform.
pub fn runtime_files(platform: &str) -> Option<&'static [&'static str]> {
    match platform {
        "windows-x86_64" => Some(&["bin/luajit.exe", "bin/lua51.dll", "bin/lua-utf8.dll"]),
        "linux-x86_64" => Some(&["bin/luajit", "bin/lua-utf8.so"]),
        _ => None,
    }
}

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn is_object_id(text: &str) -> bool {
    text.len() == 40 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED: &[u8] = include_bytes!("../../oracle/manifest.json");

    #[test]
    fn committed_manifest_should_parse() {
        let manifest = Manifest::parse(COMMITTED).unwrap();
        assert_eq!(manifest.pob.tag, "v2.67.2");
        assert_eq!(manifest.adapter.protocol, 1);
        assert_eq!(manifest.luajit.security.get("prng"), Some(&0));
    }

    #[test]
    fn parse_should_reject_unknown_field() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["pob"]["branch"] = "dev".into();
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.contains("unknown field"), "{err}");
    }

    #[test]
    fn parse_should_reject_short_commit() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["luajit"]["commit"] = "2460b3ff".into();
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("luajit.commit"), "{err}");
    }

    #[test]
    fn parse_should_reject_module_name_with_quote() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["luautf8"]["module"] = "lua\")os.exit(1)--".into();
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("luautf8.module"), "{err}");
    }

    #[test]
    fn parse_should_reject_missing_prng_flag() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["luajit"]["security"]
            .as_object_mut()
            .unwrap()
            .remove("prng");
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("luajit.security:"), "{err}");
    }

    #[test]
    fn parse_should_reject_unknown_security_flag() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["luajit"]["security"]["other"] = 1.into();
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("luajit.security:"), "{err}");
    }

    #[test]
    fn parse_should_reject_random_prng_seed() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["luajit"]["security"]["prng"] = 1.into();
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("luajit.security.prng"), "{err}");
    }

    #[test]
    fn exit_codes_should_be_distinct_from_generic_failure() {
        assert_eq!(FailureClass::Acquisition.exit_code(), 10);
        assert_eq!(FailureClass::Environment.exit_code(), 11);
    }
}
