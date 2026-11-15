//! Application updates (Phase 5, P3; plan decision P-d;
//! `docs/phase-5/updates.md`). `tauri-plugin-updater` fetches the feed
//! (`latest.json` on the GitHub release) and checks the update package's
//! signature with the update key; this module adds what the plan asks of
//! it:
//!
//! - **Modes** ([`api::UpdateMode`], kept in `settings.json`):
//!   `automatic-check` checks once at start, `manual` only when the user
//!   asks, `strict-offline` never connects.
//! - **Per artifact:** the NSIS setup, the MSI, and the AppImage install an
//!   update; the `.deb` and the portable zip only link to the release, and
//!   [`installable`] refuses them before anything is downloaded.
//! - **No downgrade:** only a version greater than the running one is
//!   offered ([`newer`]); an equal or lower one is no update.
//! - **The user approves every install:** nothing installs without the
//!   `install_update` command, which the UI sends from its prompt.
//!
//! This is the one network path outside `loom-integrations` (ADR-010,
//! ADR-011; AGENTS.md). The check reveals the IP address and the running
//! version to GitHub (README, "Privacy").

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use loom_app::api;
use loom_app::bundles::Settings;
use semver::Version;
use tauri::{Emitter, Manager, Runtime, Url};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::release::Artifact;

/// The channel a finished check reports on (an [`api::UpdateStatus`]).
pub const UPDATE_EVENT: &str = "loom://updates";

/// The project's releases.
pub const RELEASES: &str = "https://github.com/DBAggie/Exiled-Studio/releases";

/// The update key's public half (`release/updater.pub`, minisign, as
/// `tauri signer generate` writes it). Empty until the maintainer makes the
/// first key (`docs/release/signing.md`); a build without one cannot check.
const UPDATE_KEY: &str = include_str!("../../../release/updater.pub");

/// How long a check or download may wait for the server.
const TIMEOUT: Duration = Duration::from_secs(30);

/// Marks a builder that registered the updater plugin (`run` does; tests
/// that build the shell without it do not, and every check then answers
/// `UPDATE_UNAVAILABLE`).
#[derive(Debug, Clone, Copy)]
pub struct Plugin;

/// Whether `remote` is an update to `current`: strictly greater. An equal
/// or lower version (a downgrade, or a replayed old feed) is never offered.
#[must_use]
pub fn newer(current: &Version, remote: &Version) -> bool {
    remote > current
}

/// Why an update cannot be installed: an error with its code.
fn refuse(code: &str, message: &str) -> api::ApiError {
    api::ApiError::new(code, message)
}

/// Whether this package may install the offered update in this mode.
///
/// # Errors
/// `UPDATE_OFFLINE` in strict-offline mode; `UPDATE_NOTIFY_ONLY` for a
/// package that only links to the release (`.deb`, portable zip).
pub fn installable(artifact: Artifact, mode: api::UpdateMode) -> Result<(), api::ApiError> {
    if mode == api::UpdateMode::StrictOffline {
        return Err(refuse(
            "UPDATE_OFFLINE",
            "strict-offline mode: the application never connects",
        ));
    }
    if !artifact.self_updates() {
        return Err(refuse(
            "UPDATE_NOTIFY_ONLY",
            "this package does not install updates; download the new version from its release page",
        ));
    }
    Ok(())
}

/// The update check, with the downgrade refusal. `endpoints` replaces the
/// configured feed (tests use a loopback one).
///
/// # Errors
/// The plugin's: no endpoint answered, the feed did not parse, no entry for
/// this platform.
pub async fn check<R: Runtime>(
    handle: &tauri::AppHandle<R>,
    pubkey: &str,
    endpoints: Option<Vec<Url>>,
) -> Result<Option<Update>, tauri_plugin_updater::Error> {
    // On Windows the installer replaces the process: stop the application
    // first, as at any exit, so recovery keeps the unsaved changes.
    let exiting = handle.clone();
    let mut builder = handle
        .updater_builder()
        .pubkey(pubkey)
        .timeout(TIMEOUT)
        .version_comparator(|current, release| newer(&current, &release.version))
        .on_before_exit(move || {
            crate::shutdown(&exiting);
            exiting.cleanup_before_exit();
        });
    if let Some(endpoints) = endpoints {
        builder = builder.endpoints(endpoints)?;
    }
    builder.build()?.check().await
}

/// The offer the UI shows for an update.
#[must_use]
pub fn offer(update: &Update) -> api::UpdateOffer {
    api::UpdateOffer {
        version: update.version.clone(),
        notes: update.body.clone(),
        date: update
            .raw_json
            .get("pub_date")
            .and_then(|d| d.as_str())
            .map(str::to_owned),
        url: format!("{RELEASES}/tag/v{}", update.version),
    }
}

#[derive(Default)]
struct Inner {
    mode: api::UpdateMode,
    checking: bool,
    checked: bool,
    update: Option<Update>,
    error: Option<api::ApiError>,
}

/// The shell's update state.
pub struct Updates {
    plugin: bool,
    artifact: Artifact,
    settings: PathBuf,
    inner: Mutex<Inner>,
}

impl Updates {
    /// The state, with the mode read from `settings`.
    #[must_use]
    pub fn new(plugin: bool, settings: PathBuf) -> Self {
        let mode = Settings::load(&settings).updates.unwrap_or_default();
        Self {
            plugin,
            artifact: Artifact::current(),
            settings,
            inner: Mutex::new(Inner {
                mode,
                ..Inner::default()
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Whether this build can check: the plugin is registered and the
    /// build carries an update key.
    #[must_use]
    pub fn available(&self) -> bool {
        self.plugin && !UPDATE_KEY.trim().is_empty()
    }

    /// The mode.
    #[must_use]
    pub fn mode(&self) -> api::UpdateMode {
        self.lock().mode
    }

    /// The status the UI shows.
    #[must_use]
    pub fn status(&self) -> api::UpdateStatus {
        let inner = self.lock();
        api::UpdateStatus {
            mode: inner.mode,
            current: env!("CARGO_PKG_VERSION").to_owned(),
            artifact: self.artifact.name().to_owned(),
            installs: self.artifact.self_updates(),
            available: self.available(),
            checking: inner.checking,
            offer: inner.update.as_ref().map(offer),
            checked: inner.checked,
            error: inner.error.clone(),
        }
    }

    /// Sets the mode and remembers it. Strict-offline forgets an offer.
    pub fn set_mode(&self, mode: api::UpdateMode) {
        {
            let mut inner = self.lock();
            inner.mode = mode;
            if mode == api::UpdateMode::StrictOffline {
                inner.update = None;
                inner.error = None;
            }
        }
        Settings::update(&self.settings, |s| s.updates = Some(mode));
    }

    /// Checks the feed and keeps the answer.
    ///
    /// # Errors
    /// `UPDATE_OFFLINE` in strict-offline mode, `UPDATE_UNAVAILABLE` when
    /// this build cannot check, `UPDATE_BUSY` while a check runs,
    /// `UPDATE_CHECK` when the check failed.
    pub async fn check<R: Runtime>(
        &self,
        handle: &tauri::AppHandle<R>,
    ) -> Result<api::UpdateStatus, api::ApiError> {
        {
            let mut inner = self.lock();
            if inner.mode == api::UpdateMode::StrictOffline {
                return Err(refuse(
                    "UPDATE_OFFLINE",
                    "strict-offline mode: the application never connects",
                ));
            }
            if !self.available() {
                return Err(refuse(
                    "UPDATE_UNAVAILABLE",
                    "this build has no update key, so it cannot check for updates",
                ));
            }
            if inner.checking {
                return Err(refuse("UPDATE_BUSY", "a check is already running"));
            }
            inner.checking = true;
        }
        let result = check(handle, UPDATE_KEY.trim(), None).await;
        let mut inner = self.lock();
        inner.checking = false;
        inner.checked = true;
        match result {
            Ok(update) => {
                tracing::info!(
                    offer = update.as_ref().map(|u| u.version.as_str()),
                    "update check"
                );
                inner.update = update;
                inner.error = None;
            }
            Err(e) => {
                tracing::warn!(error = %e, "update check failed");
                inner.error = Some(refuse("UPDATE_CHECK", &e.to_string()));
            }
        }
        drop(inner);
        Ok(self.status())
    }

    /// The offered update, if this package may install it now.
    ///
    /// # Errors
    /// Those of [`installable`]; `UPDATE_NONE` without an offer.
    pub fn to_install(&self) -> Result<Update, api::ApiError> {
        let inner = self.lock();
        installable(self.artifact, inner.mode)?;
        inner
            .update
            .clone()
            .ok_or_else(|| refuse("UPDATE_NONE", "no update was offered"))
    }

    /// Records a failed install.
    pub fn failed(&self, error: api::ApiError) {
        self.lock().error = Some(error);
    }
}

/// The check at start, in `automatic-check` mode: in the background, its
/// answer on [`UPDATE_EVENT`]. Not in a smoke run or an `e2e` build, which
/// never connect.
pub fn at_start<R: Runtime>(handle: &tauri::AppHandle<R>) {
    let Some(updates) = handle.try_state::<Updates>() else {
        return;
    };
    if updates.mode() != api::UpdateMode::AutomaticCheck
        || !updates.available()
        || cfg!(feature = "e2e")
        || handle.try_state::<crate::release::Smoke>().is_some()
    {
        return;
    }
    let handle = handle.clone();
    tauri::async_runtime::spawn(async move {
        let Some(updates) = handle.try_state::<Updates>() else {
            return;
        };
        if let Ok(status) = updates.check(&handle).await {
            let _ = handle.emit(UPDATE_EVENT, status);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use api::UpdateMode::{AutomaticCheck, Manual, StrictOffline};

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap_or_else(|_| Version::new(0, 0, 0))
    }

    #[test]
    fn only_a_greater_version_is_an_update() {
        assert!(newer(&v("0.1.0"), &v("0.2.0")));
        assert!(newer(&v("0.1.0"), &v("0.1.1")));
        assert!(!newer(&v("0.1.0"), &v("0.1.0")), "equal");
        assert!(!newer(&v("0.2.0"), &v("0.1.9")), "downgrade");
        assert!(
            !newer(&v("0.2.0"), &v("0.2.0-rc.1")),
            "a pre-release of the running version"
        );
        assert!(newer(&v("0.2.0-rc.1"), &v("0.2.0")));
    }

    #[test]
    fn the_deb_and_the_portable_zip_never_install() {
        for mode in [AutomaticCheck, Manual] {
            for artifact in [Artifact::Nsis, Artifact::Msi, Artifact::AppImage] {
                assert!(installable(artifact, mode).is_ok(), "{artifact:?}");
            }
            for artifact in [Artifact::Deb, Artifact::Plain] {
                let e = installable(artifact, mode).err().map(|e| e.code);
                assert_eq!(e.as_deref(), Some("UPDATE_NOTIFY_ONLY"), "{artifact:?}");
            }
        }
        for artifact in [Artifact::Nsis, Artifact::Deb] {
            let e = installable(artifact, StrictOffline).err().map(|e| e.code);
            assert_eq!(e.as_deref(), Some("UPDATE_OFFLINE"));
        }
    }
}
