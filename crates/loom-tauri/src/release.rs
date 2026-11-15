//! What a packaged application knows about itself (Phase 5, P1;
//! `docs/phase-5/packaging.md`): its build info, the smoke run the
//! `package` and `release` workflows start it with, and the portable
//! package's data folder.
//!
//! **Smoke run.** `--smoke <file>` starts the application as usual; when
//! the first screen has rendered (the UI calls `app_ready`), it writes
//! [`BuildInfo`] with `ready: true` to `<file>` and quits with exit code 0.
//! If the screen does not come up within [`SMOKE_TIMEOUT`], it writes
//! `ready: false` and quits with exit code 2. The workflows read the file:
//! a GUI-subsystem executable on Windows has no console to print to.
//!
//! **Portable package.** The portable zip holds the executable and a
//! [`PORTABLE_MARKER`] file beside it; with the marker there, the
//! application keeps its data in `data/` beside the executable instead of
//! the user's application data folder, so the folder can be moved or
//! deleted as a whole.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a smoke run waits for the first screen.
pub const SMOKE_TIMEOUT: Duration = Duration::from_secs(90);

/// The file beside the executable that makes it portable.
pub const PORTABLE_MARKER: &str = "portable";

/// The data folder of a portable package, beside the executable.
pub const PORTABLE_DATA: &str = "data";

/// The kind of package the running executable came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Artifact {
    /// The NSIS installer (Windows, per user).
    Nsis,
    /// The MSI installer (Windows).
    Msi,
    /// The AppImage (Linux).
    AppImage,
    /// The Debian package (Linux).
    Deb,
    /// The portable zip, or a development build: no installer.
    Plain,
}

impl Artifact {
    /// The running executable's package, as the Tauri bundler marked it.
    #[must_use]
    pub fn current() -> Self {
        use tauri::utils::config::BundleType;
        match tauri::utils::platform::bundle_type() {
            Some(BundleType::Nsis) => Self::Nsis,
            Some(BundleType::Msi) => Self::Msi,
            Some(BundleType::AppImage) => Self::AppImage,
            Some(BundleType::Deb) => Self::Deb,
            _ => Self::Plain,
        }
    }

    /// Whether the updater may install a new version over this package
    /// itself (P-d: NSIS, MSI, AppImage); the others only notify.
    #[must_use]
    pub fn self_updates(self) -> bool {
        matches!(self, Self::Nsis | Self::Msi | Self::AppImage)
    }
}

/// The build's own facts, written by a smoke run and shown nowhere else.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BuildInfo {
    /// Whether the first screen came up.
    pub ready: bool,
    /// The application version (the workspace version).
    pub version: &'static str,
    /// Whether the end-to-end test feature is in (never in a release).
    pub e2e: bool,
    /// Whether this is a debug build.
    pub debug: bool,
    /// The package it runs from.
    pub artifact: Artifact,
    /// Whether it keeps its data beside the executable.
    pub portable: bool,
}

impl BuildInfo {
    /// This build's facts.
    #[must_use]
    pub fn current(ready: bool) -> Self {
        Self {
            ready,
            version: env!("CARGO_PKG_VERSION"),
            e2e: cfg!(feature = "e2e"),
            debug: cfg!(debug_assertions),
            artifact: Artifact::current(),
            portable: portable_data().is_some(),
        }
    }
}

/// A smoke run's output file (managed state; absent in a normal run).
#[derive(Debug, Clone)]
pub struct Smoke(pub PathBuf);

impl Smoke {
    /// The output file `--smoke <file>` names, if given.
    #[must_use]
    pub fn from_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<Self> {
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            if arg == "--smoke" {
                return args.next().map(|f| Self(PathBuf::from(f)));
            }
        }
        None
    }

    /// Writes the build info with `ready`; a failure to write is reported
    /// on stderr (the workflow then fails on the missing file).
    pub fn write(&self, ready: bool) {
        let info = BuildInfo::current(ready);
        let text = serde_json::to_string_pretty(&info).unwrap_or_default();
        if let Err(e) = std::fs::write(&self.0, text) {
            eprintln!("smoke: cannot write {}: {e}", self.0.display());
        }
    }

    /// Quits the process with exit code 2 after [`SMOKE_TIMEOUT`] unless
    /// the first screen came up first (which exits the process itself).
    pub fn watchdog(&self) {
        let smoke = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(SMOKE_TIMEOUT);
            smoke.write(false);
            std::process::exit(2);
        });
    }
}

/// The data folder of a portable package: `data/` beside the executable
/// when the [`PORTABLE_MARKER`] is there.
#[must_use]
pub fn portable_data() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    portable_data_beside(exe.parent()?)
}

/// [`portable_data`] for the executable's folder `dir`.
#[must_use]
pub fn portable_data_beside(dir: &Path) -> Option<PathBuf> {
    dir.join(PORTABLE_MARKER)
        .is_file()
        .then(|| dir.join(PORTABLE_DATA))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_smoke_file_should_be_the_argument_after_the_flag() {
        let args = ["loom", "--smoke", "out.json"].map(std::ffi::OsString::from);
        assert_eq!(
            Smoke::from_args(args).map(|s| s.0),
            Some(PathBuf::from("out.json"))
        );
        let none = ["loom", "--other"].map(std::ffi::OsString::from);
        assert!(Smoke::from_args(none).is_none());
        let missing = ["loom", "--smoke"].map(std::ffi::OsString::from);
        assert!(Smoke::from_args(missing).is_none());
    }

    #[test]
    fn the_portable_marker_should_put_the_data_beside_the_executable() {
        let dir = std::env::temp_dir().join(format!("loom-portable-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(portable_data_beside(&dir), None);
        std::fs::write(dir.join(PORTABLE_MARKER), "").unwrap();
        assert_eq!(portable_data_beside(&dir), Some(dir.join(PORTABLE_DATA)));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_development_build_should_be_plain_and_say_so() {
        let info = BuildInfo::current(true);
        assert_eq!(info.artifact, Artifact::Plain);
        assert!(!Artifact::Plain.self_updates());
        assert!(!Artifact::Deb.self_updates());
        assert!(Artifact::Nsis.self_updates() && Artifact::Msi.self_updates());
        assert!(Artifact::AppImage.self_updates());
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }
}
