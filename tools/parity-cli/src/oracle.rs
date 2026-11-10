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
/// smallest failing class is the one a report names. Exit codes are fixed
/// per class and never reused: 10 `ORACLE_ACQUISITION` to 20
/// `ORACLE_UNEXPECTED_DIFFERENCE`, in taxonomy order. The two difference
/// classes are `oracle-compare`'s; the others come from running the oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FailureClass {
    /// The pinned source is unavailable or differs from the manifest
    /// (wrong commit, dirty checkout, unreadable manifest).
    #[serde(rename = "ORACLE_ACQUISITION")]
    Acquisition,
    /// The runtime is missing, differs from its build record, or does not
    /// report the pinned version and build flags; or a run depended on
    /// something outside the pinned inputs (a user settings file, a
    /// network module).
    #[serde(rename = "ORACLE_ENVIRONMENT")]
    Environment,
    /// The adapter process or PoB's startup failed before a fixture was
    /// read.
    #[serde(rename = "ORACLE_LAUNCH")]
    Launch,
    /// A run exceeded the time limit.
    #[serde(rename = "ORACLE_TIMEOUT")]
    Timeout,
    /// The adapter's output document is missing, malformed, violates the
    /// schema, or disagrees with the exit code.
    #[serde(rename = "ORACLE_PROTOCOL")]
    Protocol,
    /// The fixture could not be loaded (unreadable, not well-formed, not a
    /// PoB build).
    #[serde(rename = "ORACLE_IMPORT")]
    Import,
    /// A well-formed PoB build made PoB raise an error while loading or
    /// calculating.
    #[serde(rename = "ORACLE_CALCULATION")]
    Calculation,
    /// PoB's output could not be written as data (a key that collides with
    /// a placeholder, a table key, a string that is not UTF-8).
    #[serde(rename = "ORACLE_NORMALIZATION")]
    Normalization,
    /// Repeated runs of one fixture had different outcomes.
    #[serde(rename = "ORACLE_NONDETERMINISM")]
    Nondeterminism,
    /// Two outputs differ, and a divergence register entry covers the
    /// difference (a known discrepancy reproduced).
    #[serde(rename = "ORACLE_EXPECTED_DIFFERENCE")]
    ExpectedDifference,
    /// Two outputs differ, and no register entry covers the difference.
    #[serde(rename = "ORACLE_UNEXPECTED_DIFFERENCE")]
    UnexpectedDifference,
}

impl FailureClass {
    /// The taxonomy name, as reports write it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Acquisition => "ORACLE_ACQUISITION",
            Self::Environment => "ORACLE_ENVIRONMENT",
            Self::Launch => "ORACLE_LAUNCH",
            Self::Timeout => "ORACLE_TIMEOUT",
            Self::Protocol => "ORACLE_PROTOCOL",
            Self::Import => "ORACLE_IMPORT",
            Self::Calculation => "ORACLE_CALCULATION",
            Self::Normalization => "ORACLE_NORMALIZATION",
            Self::Nondeterminism => "ORACLE_NONDETERMINISM",
            Self::ExpectedDifference => "ORACLE_EXPECTED_DIFFERENCE",
            Self::UnexpectedDifference => "ORACLE_UNEXPECTED_DIFFERENCE",
        }
    }

    /// The process exit code for this class.
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Acquisition => 10,
            Self::Environment => 11,
            Self::Launch => 12,
            Self::Timeout => 13,
            Self::Protocol => 14,
            Self::Import => 15,
            Self::Calculation => 16,
            Self::Normalization => 17,
            Self::Nondeterminism => 18,
            Self::ExpectedDifference => 19,
            Self::UnexpectedDifference => 20,
        }
    }

    /// Every class, in taxonomy order.
    pub const ALL: [Self; 11] = [
        Self::Acquisition,
        Self::Environment,
        Self::Launch,
        Self::Timeout,
        Self::Protocol,
        Self::Import,
        Self::Calculation,
        Self::Normalization,
        Self::Nondeterminism,
        Self::ExpectedDifference,
        Self::UnexpectedDifference,
    ];
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

/// The adapter (`tools/oracle/adapter/oracle.lua`) and what it is asked
/// to write. Versioned here, so a change to the selection is a manifest
/// change.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Adapter {
    /// Adapter protocol version.
    pub protocol: u32,
    /// Names in PoB's breakdown table (`calcsTab.calcsEnv.player.breakdown`)
    /// the adapter writes; a name a build has no breakdown for is written
    /// as `null`.
    pub breakdowns: Vec<String>,
    /// Keys whose table values the adapter writes as `{"$opaque": key}`:
    /// input objects (gem instances) whose game data is not calculation
    /// output.
    pub opaque_keys: Vec<String>,
    /// The headless shims the adapter installs (`tools/oracle/adapter/
    /// shims.lua`), by name, sorted; every output document must record
    /// exactly these.
    pub shims: Vec<String>,
    /// The zlib library the `inflate` shim loads, per platform key: a path
    /// relative to the oracle dir when it contains `/` (Windows: PoB's own
    /// `pob/runtime/zlib1.dll`, covered by the checkout's tree hash), else
    /// a library name for the system loader (Linux: `libz.so.1`).
    pub zlib: BTreeMap<String, String>,
}

/// The adapter protocol this reader knows.
pub const PROTOCOL: u32 = 2;

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
        if manifest.adapter.protocol != PROTOCOL {
            return Err(format!(
                "adapter.protocol: this reader knows {PROTOCOL}, found {}",
                manifest.adapter.protocol
            ));
        }
        for (name, list) in [
            ("adapter.breakdowns", &manifest.adapter.breakdowns),
            ("adapter.opaque_keys", &manifest.adapter.opaque_keys),
            ("adapter.shims", &manifest.adapter.shims),
        ] {
            // Passed to the adapter as `<kind>=<name>` arguments and used
            // as JSON keys: plain identifiers only, sorted, no duplicates.
            if let Some(bad) = list.iter().find(|n| !is_identifier(n)) {
                return Err(format!("{name}: {bad:?} is not a plain identifier"));
            }
            if !list.windows(2).all(|w| w[0] < w[1]) {
                return Err(format!("{name}: must be sorted, without duplicates"));
            }
        }
        for (platform, library) in &manifest.adapter.zlib {
            if runtime_files(platform).is_none() {
                return Err(format!("adapter.zlib: unknown platform {platform}"));
            }
            if !is_library_reference(library) {
                return Err(format!(
                    "adapter.zlib.{platform}: {library:?} is neither a relative path inside \
                     the oracle dir nor a plain library name"
                ));
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

/// A relative `/`-separated path without `.` or `..` parts, or a plain
/// library name (letters, digits, `.`, `-`, `_`).
fn is_library_reference(text: &str) -> bool {
    let plain = |part: &str| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
    };
    text.split('/').all(plain)
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
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
        assert_eq!(manifest.adapter.protocol, PROTOCOL);
        assert_eq!(manifest.luajit.security.get("prng"), Some(&0));
        assert_eq!(manifest.adapter.shims, ["inflate", "write_guard"]);
        // Every supported platform has a zlib library.
        for platform in ["windows-x86_64", "linux-x86_64"] {
            assert!(manifest.adapter.zlib.contains_key(platform), "{platform}");
        }
    }

    #[test]
    fn parse_should_reject_bad_zlib_references() {
        for (platform, library) in [
            ("windows-x86_64", "../outside/zlib1.dll"),
            ("windows-x86_64", "pob\\runtime\\zlib1.dll"),
            ("windows-x86_64", "/usr/lib/libz.so.1"),
            ("linux-x86_64", "libz.so.1 --x"),
            ("plan9-x86_64", "libz.so.1"),
        ] {
            let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
            value["adapter"]["zlib"][platform] = library.into();
            let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
            assert!(err.starts_with("adapter.zlib"), "{library}: {err}");
        }
    }

    #[test]
    fn parse_should_reject_unsorted_shims() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["adapter"]["shims"] = serde_json::json!(["write_guard", "inflate"]);
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("adapter.shims"), "{err}");
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
    fn parse_should_reject_unsorted_breakdowns() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["adapter"]["breakdowns"] = serde_json::json!(["Life", "Armour"]);
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("adapter.breakdowns"), "{err}");
    }

    #[test]
    fn parse_should_reject_opaque_key_with_equals_sign() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["adapter"]["opaque_keys"] = serde_json::json!(["a=b"]);
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("adapter.opaque_keys"), "{err}");
    }

    #[test]
    fn parse_should_reject_other_protocol() {
        let mut value: serde_json::Value = serde_json::from_slice(COMMITTED).unwrap();
        value["adapter"]["protocol"] = 1.into();
        let err = Manifest::parse(value.to_string().as_bytes()).unwrap_err();
        assert!(err.starts_with("adapter.protocol"), "{err}");
    }

    #[test]
    fn exit_codes_should_follow_taxonomy_order_from_10() {
        let classes = FailureClass::ALL;
        for (i, class) in classes.iter().enumerate() {
            assert_eq!(usize::from(class.exit_code()), 10 + i, "{class:?}");
            assert_eq!(
                serde_json::to_value(class).unwrap(),
                serde_json::Value::from(class.name())
            );
            let back: FailureClass = serde_json::from_value(class.name().into()).unwrap();
            assert_eq!(back, *class);
        }
        assert_eq!(FailureClass::UnexpectedDifference.exit_code(), 20);
        assert!(classes.windows(2).all(|w| w[0] < w[1]));
    }
}
