//! `oracle-verify`: checks an acquired oracle directory against the pinned
//! manifest before any run (Spike C plan, step 1). Offline: it reads
//! files, runs `git` on the local checkouts, and starts the built `luajit`
//! once to read its version and build flags; it never fetches.
//!
//! Classes: the PoB checkout or a runtime source differing from the
//! manifest (commit, tree, or any untracked, ignored, or modified file) is
//! `ORACLE_ACQUISITION`; a runtime that is missing, differs from its build
//! record, or reports another version or build flags is
//! `ORACLE_ENVIRONMENT`. When both occur, the report's class is
//! `ORACLE_ACQUISITION` (the upstream failure); every check is listed.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::oracle::{self, BuildRecord, FailureClass, Manifest};

/// How long the runtime probe may take.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// The string the probe measures with the `lua-utf8` module: 5 characters
/// in 6 bytes (`é` is two bytes in UTF-8), written as Lua byte escapes.
const PROBE_UTF8_TEXT: &str = r"h\195\169llo";
const PROBE_UTF8_LEN: &str = "5";

/// One check: what the manifest or build record expects and what was found.
#[derive(Debug, Serialize)]
pub struct Check {
    /// Stable check name, for example `pob.commit`.
    pub name: String,
    /// The class a failure of this check gets.
    pub class: FailureClass,
    /// Whether `actual` matches `expected`.
    pub ok: bool,
    /// Expected value.
    pub expected: String,
    /// Found value, or the error that prevented reading it.
    pub actual: String,
}

/// The `oracle-verify` result, printed as JSON.
#[derive(Debug, Serialize)]
pub struct Report {
    /// `ok` or `failed`.
    pub status: &'static str,
    /// Class of the failure; `None` when every check passed.
    pub class: Option<FailureClass>,
    /// Platform key checked.
    pub platform: String,
    /// SHA-256 of the manifest bytes; `None` if the file was unreadable.
    pub manifest_sha256: Option<String>,
    /// The parsed manifest (every pinned input); `None` if unreadable.
    pub manifest: Option<Manifest>,
    /// Environment variables the runtime probe receives; every other
    /// variable is cleared.
    pub environment: &'static [&'static str],
    /// Compiler of the build, from the build record.
    pub toolchain: Option<String>,
    /// Every check, in order.
    pub checks: Vec<Check>,
}

/// Variables passed to the runtime (the CRT and DLL loading on Windows
/// need `SystemRoot`).
pub const ENVIRONMENT: &[&str] = if cfg!(windows) {
    &["LUA_CPATH", "LUA_PATH", "SystemRoot"]
} else {
    &["LUA_CPATH", "LUA_PATH"]
};

#[derive(Default)]
struct Checks(Vec<Check>);

impl Checks {
    fn compare(
        &mut self,
        name: impl Into<String>,
        class: FailureClass,
        expected: &str,
        actual: Result<String, String>,
    ) {
        let (ok, actual) = match actual {
            Ok(actual) => (actual == expected, actual),
            Err(error) => (false, format!("error: {error}")),
        };
        self.0.push(Check {
            name: name.into(),
            class,
            ok,
            expected: expected.to_owned(),
            actual,
        });
    }
}

/// Verifies `oracle_dir` against the manifest (its bytes, or the error
/// reading them) for `platform`. `probe` runs the `luajit` executable at
/// the given path with a Lua chunk and returns its standard output
/// ([`run_probe`] outside tests).
pub fn verify(
    manifest_bytes: Result<&[u8], String>,
    oracle_dir: &Path,
    platform: &str,
    probe: impl Fn(&Path, &str) -> Result<String, String>,
) -> Report {
    let mut checks = Checks::default();
    let mut toolchain = None;
    let manifest_sha256 = manifest_bytes.as_ref().ok().map(|b| oracle::sha256_hex(b));
    let manifest = match manifest_bytes.and_then(Manifest::parse) {
        Err(error) => {
            checks.compare(
                "manifest",
                FailureClass::Acquisition,
                "manifest format 1",
                Err(error),
            );
            None
        }
        Ok(manifest) => {
            check_sources(&mut checks, &manifest, oracle_dir);
            let record = read_build_record(oracle_dir);
            toolchain = record.as_ref().ok().map(|r| r.toolchain.clone());
            check_runtime(&mut checks, &manifest, oracle_dir, platform, &record, probe);
            Some(manifest)
        }
    };
    let class = checks.0.iter().filter(|c| !c.ok).map(|c| c.class).min();
    Report {
        status: if class.is_none() { "ok" } else { "failed" },
        class,
        platform: platform.to_owned(),
        manifest_sha256,
        manifest,
        environment: ENVIRONMENT,
        toolchain,
        checks: checks.0,
    }
}

fn check_sources(checks: &mut Checks, manifest: &Manifest, oracle_dir: &Path) {
    let pob = oracle_dir.join("pob");
    let acquisition = FailureClass::Acquisition;
    checks.compare(
        "pob.commit",
        acquisition,
        &manifest.pob.commit,
        git(&pob, &["rev-parse", "HEAD^{commit}"]),
    );
    checks.compare(
        "pob.tree",
        acquisition,
        &manifest.pob.tree,
        git(&pob, &["rev-parse", "HEAD^{tree}"]),
    );
    // Ignored files count: PoB writes caches its own .gitignore hides
    // (src/Data/TimelessJewelData/*.bin).
    let status = git(
        &pob,
        &[
            "status",
            "--porcelain=v1",
            "--ignored",
            "--untracked-files=all",
        ],
    )
    .map(|text| describe_status(&text));
    checks.compare("pob.clean", acquisition, "clean", status);
    for (name, dir, commit) in [
        (
            "luajit.commit",
            oracle_dir.join("src").join("luajit"),
            &manifest.luajit.commit,
        ),
        (
            "luautf8.commit",
            oracle_dir.join("src").join("luautf8"),
            &manifest.luautf8.commit,
        ),
    ] {
        checks.compare(
            name,
            acquisition,
            commit,
            git(&dir, &["rev-parse", "HEAD^{commit}"]),
        );
    }
}

/// `clean`, or the number of status entries and the first few.
fn describe_status(porcelain: &str) -> String {
    let entries: Vec<&str> = porcelain.lines().filter(|l| !l.is_empty()).collect();
    if entries.is_empty() {
        return "clean".to_owned();
    }
    let shown: Vec<&str> = entries.iter().take(5).copied().collect();
    format!("{} entries: {}", entries.len(), shown.join("; "))
}

fn read_build_record(oracle_dir: &Path) -> Result<BuildRecord, String> {
    let bytes = std::fs::read(oracle_dir.join("build.json"))
        .map_err(|e| format!("build.json: {}", e.kind()))?;
    let record: BuildRecord =
        serde_json::from_slice(&bytes).map_err(|e| format!("build.json: {e}"))?;
    if record.format != 1 {
        return Err(format!("build.json: unsupported format {}", record.format));
    }
    Ok(record)
}

fn check_runtime(
    checks: &mut Checks,
    manifest: &Manifest,
    oracle_dir: &Path,
    platform: &str,
    record: &Result<BuildRecord, String>,
    probe: impl Fn(&Path, &str) -> Result<String, String>,
) {
    let environment = FailureClass::Environment;
    let Some(required) = oracle::runtime_files(platform) else {
        checks.compare(
            "platform",
            environment,
            "windows-x86_64 or linux-x86_64",
            Ok(platform.to_owned()),
        );
        return;
    };
    let field = |get: fn(&BuildRecord) -> String| record.as_ref().map(get).map_err(Clone::clone);
    checks.compare(
        "build.platform",
        environment,
        platform,
        field(|r| r.platform.clone()),
    );
    checks.compare(
        "build.luajit_commit",
        environment,
        &manifest.luajit.commit,
        field(|r| r.luajit_commit.clone()),
    );
    checks.compare(
        "build.luautf8_commit",
        environment,
        &manifest.luautf8.commit,
        field(|r| r.luautf8_commit.clone()),
    );
    checks.compare(
        "build.defines",
        environment,
        &manifest.luajit.defines.join(" "),
        field(|r| r.defines.join(" ")),
    );

    // Every required file, and anything else the record lists.
    let mut files: Vec<&str> = required.to_vec();
    if let Ok(record) = record {
        files.extend(
            record
                .files
                .keys()
                .map(String::as_str)
                .filter(|f| !required.contains(f)),
        );
    }
    for file in files {
        let expected = match record {
            Ok(record) => record
                .files
                .get(file)
                .cloned()
                .unwrap_or_else(|| "a hash in build.json".to_owned()),
            Err(_) => "a hash in build.json".to_owned(),
        };
        let actual = std::fs::read(oracle_dir.join(file))
            .map(|bytes| oracle::sha256_hex(&bytes))
            .map_err(|e| e.kind().to_string());
        checks.compare(format!("file:{file}"), environment, &expected, actual);
    }

    let luajit = oracle_dir.join(required[0]);
    match probe(&luajit, &probe_script(manifest)) {
        Err(error) => checks.compare("runtime.start", environment, "exit 0", Err(error)),
        Ok(output) => {
            let values: BTreeMap<&str, &str> = output
                .lines()
                .filter_map(|line| line.split_once('='))
                .collect();
            let value = |key: &str| {
                values
                    .get(key)
                    .map(|v| (*v).to_owned())
                    .ok_or_else(|| format!("{key} not reported"))
            };
            checks.compare(
                "runtime.version",
                environment,
                &manifest.luajit.version,
                value("version"),
            );
            for (flag, expected) in &manifest.luajit.security {
                let key = format!("security.{flag}");
                checks.compare(
                    format!("runtime.{key}"),
                    environment,
                    &expected.to_string(),
                    value(&key),
                );
            }
            checks.compare(
                "runtime.module",
                environment,
                PROBE_UTF8_LEN,
                value("module"),
            );
        }
    }
}

/// The Lua chunk the probe runs: loads the `lua-utf8` module and prints
/// `key=value` lines (version, security flags, a module call).
fn probe_script(manifest: &Manifest) -> String {
    let flags = oracle::SECURITY_FLAGS
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "local u = require(\"{module}\") \
         io.write(\"version=\", jit.version, \"\\n\") \
         for _, k in ipairs({{{flags}}}) do io.write(\"security.\", k, \"=\", jit.security(k), \"\\n\") end \
         io.write(\"module=\", u.len(\"{PROBE_UTF8_TEXT}\"), \"\\n\")",
        module = manifest.luautf8.module,
    )
}

/// Runs `git` in `dir`, which must be a checkout of its own (a missing
/// checkout must not fall through to an enclosing repository). Every
/// inherited `GIT_*` variable is removed (`GIT_DIR`, `GIT_WORK_TREE`,
/// `GIT_INDEX_FILE` set by a hook or a shell would point `git` at another
/// repository), and the system config is skipped.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    if !dir.join(".git").exists() {
        return Err("not a git checkout".to_owned());
    }
    let mut command = Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    let output = command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("git: {}", e.kind()))?;
    if !output.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned())
}

/// Runs `luajit -e <script>` with a cleared environment plus
/// [`ENVIRONMENT`]: `LUA_PATH` empty, `LUA_CPATH` the executable's
/// directory only. Standard output on exit 0.
pub fn run_probe(luajit: &Path, script: &str) -> Result<String, String> {
    // Absolute, because the child runs in `bin/`, not in our directory.
    let luajit = std::path::absolute(luajit).map_err(|e| e.to_string())?;
    let bin = luajit
        .parent()
        .ok_or_else(|| "luajit has no parent directory".to_owned())?;
    let suffix = if cfg!(windows) { "dll" } else { "so" };
    let mut command = Command::new(&luajit);
    command
        .env_clear()
        .env("LUA_PATH", "")
        .env("LUA_CPATH", bin.join(format!("?.{suffix}")))
        .current_dir(bin)
        .arg("-e")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if cfg!(windows)
        && let Some(root) = std::env::var_os("SystemRoot")
    {
        command.env("SystemRoot", root);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot start luajit: {}", e.kind()))?;
    // Both pipes are drained on their own threads while we wait, so a
    // child that writes a lot cannot block on a full pipe.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > PROBE_TIMEOUT {
            // Best effort: the child may have exited meanwhile.
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("no exit within {}s", PROBE_TIMEOUT.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout.join().unwrap_or_default();
    let stderr = stderr.join().unwrap_or_default();
    if !status.success() {
        return Err(format!("{status}: {}", stderr.trim()));
    }
    Ok(stdout)
}

/// Reads a child's pipe to the end on a thread (lossy UTF-8).
fn drain(pipe: Option<impl std::io::Read + Send + 'static>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            // A read error keeps what was read so far.
            let _ = pipe.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

#[cfg(test)]
mod tests;
