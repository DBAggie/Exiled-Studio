//! New game data from the project's signed data feed (Phase 5, P4; plan
//! decision P-e; `docs/phase-5/data-bundles.md`). The user asks: the shell
//! fetches the feed and its signature (`loom_integrations::data_feed`),
//! verifies both and refuses a lower sequence than the highest seen
//! (`loom_data::pack::verify_feed`, the sequence kept in `settings.json`),
//! and lists the packs. A pack the user picks is downloaded, its size and
//! SHA-256 checked against the feed, then installed like a pack from a file
//! (`loom_app::install`: its own signature, the hostile-input checks).
//!
//! Strict-offline mode refuses both, before any connection (the update
//! mode, [`crate::updates`]).
//!
//! The downloads use a blocking HTTP client, which must not run on the
//! async runtime's workers: each command does its work on a blocking
//! thread ([`blocking`]).

use std::sync::Mutex;

use loom_app::bundles::Settings;
use loom_app::{HostCommand, api};
use loom_data::pack::{self, Feed};
use loom_integrations::data_feed;
use tauri::{AppHandle, Manager, Runtime};

use crate::{Answer, Ctx, TEST_KEYS, updates};

/// The last verified feed, for downloads, and the packs downloading now.
#[derive(Default)]
pub struct DataFeed {
    feed: Mutex<Option<Feed>>,
    downloading: Mutex<std::collections::BTreeSet<String>>,
}

impl DataFeed {
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Feed>> {
        self.feed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn downloading(&self) -> std::sync::MutexGuard<'_, std::collections::BTreeSet<String>> {
        self.downloading
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Marks a pack as downloading until dropped.
struct Downloading<'f>(&'f DataFeed, String);

impl Drop for Downloading<'_> {
    fn drop(&mut self) {
        self.0.downloading().remove(&self.1);
    }
}

/// The most a feed's signature file may be.
const MAX_SIGNATURE: u64 = 64 * 1024;

fn offline(updates: &updates::Updates) -> Answer<()> {
    if updates.mode() == api::UpdateMode::StrictOffline {
        return Err(api::ApiError::new(
            "UPDATE_OFFLINE",
            "strict-offline mode: the application never connects",
        ));
    }
    Ok(())
}

fn status(ctx: &Ctx, feed: &Feed) -> api::DataFeedStatus {
    let snapshot = ctx.app().snapshot();
    let has = |id: &str| {
        snapshot.data.installed.iter().any(|i| i.id == id)
            || snapshot.data.quarantined.iter().any(|q| q.id == id)
    };
    api::DataFeedStatus {
        sequence: feed.sequence,
        offers: feed
            .bundles
            .iter()
            .map(|b| api::DataOffer {
                id: b.id.clone(),
                bytes: b.size,
                installed: has(&b.id),
            })
            .collect(),
    }
}

/// Runs `work` with the application's handle on a blocking thread.
async fn blocking<R: Runtime, T: Send + 'static>(
    handle: AppHandle<R>,
    work: impl FnOnce(&AppHandle<R>) -> Answer<T> + Send + 'static,
) -> Answer<T> {
    tauri::async_runtime::spawn_blocking(move || work(&handle))
        .await
        .map_err(|e| api::ApiError::new("FEED_IO", e.to_string()))?
}

/// Checks the data feed (the user asked).
///
/// # Errors
/// `UPDATE_OFFLINE`; `FEED_HTTP`, `FEED_TOO_LARGE` (the fetch);
/// `FEED_SIGNATURE`, `FEED_MALFORMED`, `FEED_ROLLBACK` (the feed).
#[tauri::command]
pub async fn check_data_feed<R: Runtime>(handle: AppHandle<R>) -> Answer<api::DataFeedStatus> {
    blocking(handle, check).await
}

fn check<R: Runtime>(handle: &AppHandle<R>) -> Answer<api::DataFeedStatus> {
    let ctx = handle.state::<Ctx>();
    let last = handle.state::<DataFeed>();
    offline(&handle.state::<updates::Updates>())?;
    let fetch_error = |e: data_feed::FetchError| api::ApiError::new(e.code(), e.to_string());
    let feed = data_feed::fetch(data_feed::FEED_URL, pack::MAX_SMALL_FILE).map_err(fetch_error)?;
    let signature =
        data_feed::fetch(data_feed::FEED_SIGNATURE_URL, MAX_SIGNATURE).map_err(fetch_error)?;
    let trust = loom_app::trust::trust_list(TEST_KEYS)
        .map_err(|e| api::ApiError::new("FEED_SIGNATURE", e.to_string()))?;
    let settings_path = ctx.settings.clone();
    let seen = Settings::load(&settings_path).data_feed_sequence;
    let feed = pack::verify_feed(&feed, &signature, &trust, seen)
        .map_err(|e| api::ApiError::new(e.code(), e.to_string()))?;
    Settings::update(&settings_path, |s| {
        s.data_feed_sequence = Some(s.data_feed_sequence.unwrap_or(0).max(feed.sequence));
    });
    tracing::info!(
        sequence = feed.sequence,
        packs = feed.bundles.len(),
        "data feed"
    );
    let answer = status(&ctx, &feed);
    *last.lock() = Some(feed);
    Ok(answer)
}

/// Downloads a pack the last checked feed lists and installs it.
///
/// # Errors
/// `UPDATE_OFFLINE`; `APP_NOT_FOUND` (not in the last feed); the
/// download's `FEED_*`; the install's.
#[tauri::command]
pub async fn download_data<R: Runtime>(handle: AppHandle<R>, id: String) -> Answer<api::Outcome> {
    blocking(handle, move |h| download(h, &id)).await
}

fn download<R: Runtime>(handle: &AppHandle<R>, id: &str) -> Answer<api::Outcome> {
    let ctx = handle.state::<Ctx>();
    let last = handle.state::<DataFeed>();
    offline(&handle.state::<updates::Updates>())?;
    let bundle = last
        .lock()
        .as_ref()
        .and_then(|f| f.bundles.iter().find(|b| b.id == id).cloned())
        .ok_or_else(|| api::ApiError::new("APP_NOT_FOUND", format!("{id} is not in the feed")))?;
    // One download of a pack at a time: they would share the file.
    if !last.downloading().insert(bundle.id.clone()) {
        return Err(api::ApiError::new(
            "FEED_BUSY",
            format!("{id} is already downloading"),
        ));
    }
    let _downloading = Downloading(&last, bundle.id.clone());
    let dir = ctx
        .settings
        .parent()
        .map(|d| d.join("downloads"))
        .ok_or_else(|| api::ApiError::new("FEED_IO", "no data folder"))?;
    std::fs::create_dir_all(&dir).map_err(|e| api::ApiError::new("FEED_IO", e.to_string()))?;
    // The ID's `/` makes no folder: one flat file per download.
    let dest = dir.join(format!(
        "{}.{}",
        bundle.id.replace(['/', '\\', ':'], "_"),
        pack::PACK_EXTENSION
    ));
    data_feed::download(&bundle.url, &bundle.sha256, bundle.size, &dest)
        .map_err(|e| api::ApiError::new(e.code(), e.to_string()))?;
    let result = ctx.app().execute(HostCommand::InstallBundle(dest.clone()));
    let _ = std::fs::remove_file(&dest);
    result.map_err(|e| e.dto())
}
