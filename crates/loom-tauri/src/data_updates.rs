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
//! **Automatic** (maintainer 2026-11-24: game data is handled by the
//! application, not by the user). [`at_start`], in `automatic-check` mode,
//! and [`fetch_data`], when the user asks, do both steps: read the feed,
//! retrying a failed connection ([`RETRY_WAITS`]), then download and
//! install each pack this computer lacks ([`wanted`]: per game the feed's
//! newest, unless a bundle of that game as new or newer is here). The
//! state goes to the UI on [`DATA_EVENT`]; a failure is logged with its
//! code and never shown over working data.
//!
//! The downloads use a blocking HTTP client, which must not run on the
//! async runtime's workers: each command does its work on a blocking
//! thread ([`blocking`]).

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use loom_app::bundles::Settings;
use loom_app::{HostCommand, api};
use loom_data::pack::{self, Feed};
use loom_integrations::data_feed;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::{Answer, Ctx, TEST_KEYS, updates};

/// The last verified feed, for downloads, and the packs downloading now.
#[derive(Default)]
pub struct DataFeed {
    feed: Mutex<Option<Feed>>,
    downloading: Mutex<std::collections::BTreeSet<String>>,
    /// The automatic data's state, and whether a run is going.
    auto: Mutex<api::DataAutoStatus>,
    running: AtomicBool,
}

/// The automatic data's channel (the UI listens: `ui/src/api/tauri.ts`).
pub const DATA_EVENT: &str = "loom://data-auto";

/// The waits before the feed's second and third tries (a connection that
/// failed; a refused feed is not tried again).
const RETRY_WAITS: [Duration; 2] = [Duration::from_secs(2), Duration::from_secs(8)];

/// How long the start waits for the installed data to load before it
/// looks for more.
const LOAD_WAIT: Duration = Duration::from_secs(120);

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

    /// The automatic data's state.
    #[must_use]
    pub fn auto(&self) -> api::DataAutoStatus {
        self.auto
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

/// A bundle ID read: its game, Steam build, and data schema
/// (`<game>/<steam build>-s<schema>`).
fn parse_id(id: &str) -> Option<(&str, u64, u32)> {
    let (game, rest) = id.split_once('/')?;
    let (build, schema) = rest.split_once("-s")?;
    Some((game, build.parse().ok()?, schema.parse().ok()?))
}

/// The packs of `feed` (IDs, newest first) to get when `have` is on this
/// computer (loaded or not): per game the feed's newest, unless a bundle
/// of that game with the same or a newer Steam build and schema is here.
/// An ID that does not read is never wanted.
fn wanted(feed: &[String], have: &[String]) -> Vec<String> {
    let have: Vec<(&str, u64, u32)> = have.iter().filter_map(|id| parse_id(id)).collect();
    let mut games: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for id in feed {
        let Some((game, build, schema)) = parse_id(id) else {
            continue;
        };
        if games.contains(&game) {
            continue;
        }
        games.push(game);
        let here = have
            .iter()
            .any(|(g, b, s)| *g == game && (*b, *s) >= (build, schema));
        if !here {
            out.push(id.clone());
        }
    }
    out
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
    let (feed_url, signature_url) = crate::release::Channel::CURRENT.data_feed();
    let feed = data_feed::fetch(feed_url, pack::MAX_SMALL_FILE).map_err(fetch_error)?;
    let signature = data_feed::fetch(signature_url, MAX_SIGNATURE).map_err(fetch_error)?;
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

/// Records the automatic data's state and tells the UI.
fn set_auto<R: Runtime>(handle: &AppHandle<R>, status: api::DataAutoStatus) {
    let feed = handle.state::<DataFeed>();
    *feed
        .auto
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = status.clone();
    let _ = handle.emit(DATA_EVENT, status);
}

/// [`check`], tried again after each of [`RETRY_WAITS`] when the
/// connection failed (`FEED_HTTP`); each failure is logged with its code.
fn check_retrying<R: Runtime>(handle: &AppHandle<R>) -> Answer<api::DataFeedStatus> {
    let mut waits = RETRY_WAITS.iter();
    loop {
        match check(handle) {
            Ok(status) => return Ok(status),
            Err(e) => {
                tracing::warn!(code = %e.code, reason = %e.message, "data feed check failed");
                match waits.next() {
                    Some(wait) if e.code == "FEED_HTTP" => std::thread::sleep(*wait),
                    _ => return Err(e),
                }
            }
        }
    }
}

/// One run: the feed, then each wanted pack. One at a time
/// (`FEED_BUSY` otherwise).
fn fetch<R: Runtime>(handle: &AppHandle<R>) -> api::DataAutoStatus {
    let feed = handle.state::<DataFeed>();
    if feed.running.swap(true, Ordering::SeqCst) {
        return feed.auto();
    }
    let state = |state| api::DataAutoStatus {
        state,
        id: None,
        error: None,
    };
    set_auto(handle, state(api::DataAutoState::Checking));
    let result = (|| -> Answer<()> {
        let status = check_retrying(handle)?;
        let ctx = handle.state::<Ctx>();
        let snapshot = ctx.app().snapshot();
        let have: Vec<String> = snapshot
            .data
            .installed
            .iter()
            .map(|i| i.id.clone())
            .chain(snapshot.data.quarantined.iter().map(|q| q.id.clone()))
            .collect();
        let offered: Vec<String> = status.offers.iter().map(|o| o.id.clone()).collect();
        for id in wanted(&offered, &have) {
            set_auto(
                handle,
                api::DataAutoStatus {
                    state: api::DataAutoState::Downloading,
                    id: Some(id.clone()),
                    error: None,
                },
            );
            download(handle, &id)?;
            tracing::info!(pack = %id, "game data installed from the feed");
        }
        Ok(())
    })();
    let status = match result {
        Ok(()) => state(api::DataAutoState::Done),
        Err(e) => {
            tracing::warn!(code = %e.code, reason = %e.message, "automatic game data failed");
            api::DataAutoStatus {
                state: api::DataAutoState::Failed,
                id: None,
                error: Some(e),
            }
        }
    };
    feed.running.store(false, Ordering::SeqCst);
    set_auto(handle, status.clone());
    status
}

/// The automatic data's state.
#[tauri::command]
pub fn data_auto_status(feed: tauri::State<'_, DataFeed>) -> api::DataAutoStatus {
    feed.auto()
}

/// Gets the game data this computer lacks (the user asked: the start
/// page's one action). The answer is the state it ended in; a failure is
/// in it, never an error of the command.
#[tauri::command]
pub async fn fetch_data<R: Runtime>(handle: AppHandle<R>) -> Answer<api::DataAutoStatus> {
    blocking(handle, |h| Ok(fetch(h))).await
}

/// The automatic data at start, in `automatic-check` mode: in the
/// background, after the installed data has loaded. Not in a smoke run,
/// an `e2e` build, a test (no updater plugin), or with `LOOM_DATA_DIR`
/// (development and tests), which never connect on their own.
pub fn at_start<R: Runtime>(handle: &AppHandle<R>) {
    let Some(updates) = handle.try_state::<updates::Updates>() else {
        return;
    };
    if updates.mode() != api::UpdateMode::AutomaticCheck
        || cfg!(feature = "e2e")
        || handle.try_state::<updates::Plugin>().is_none()
        || handle.try_state::<crate::release::Smoke>().is_some()
        || std::env::var_os("LOOM_DATA_DIR").is_some()
    {
        return;
    }
    let handle = handle.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // What is installed is known once the start's load is done.
        let started = std::time::Instant::now();
        while handle.state::<Ctx>().app().snapshot().data.loading && started.elapsed() < LOAD_WAIT {
            std::thread::sleep(Duration::from_millis(250));
        }
        fetch(&handle);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parse_id_should_read_game_build_and_schema() {
        assert_eq!(parse_id("poe1/26530219-s4"), Some(("poe1", 26_530_219, 4)));
        assert_eq!(parse_id("poe1/26530219"), None);
        assert_eq!(parse_id("26530219-s4"), None);
        assert_eq!(parse_id("poe1/x-s4"), None);
    }

    #[test]
    fn wanted_should_be_the_newest_pack_of_a_game_this_computer_lacks() {
        let feed = ids(&["poe1/200-s5", "poe1/100-s4", "poe2/50-s5"]);
        // Nothing here: each game's newest, never an older pack.
        assert_eq!(wanted(&feed, &[]), ids(&["poe1/200-s5", "poe2/50-s5"]));
        // The newest is here, or something newer is: nothing for that game.
        assert_eq!(wanted(&feed, &ids(&["poe1/200-s5"])), ids(&["poe2/50-s5"]));
        assert_eq!(
            wanted(&feed, &ids(&["poe1/300-s5", "poe2/50-s5"])),
            Vec::<String>::new()
        );
        // Older data here: the newer pack is wanted.
        assert_eq!(
            wanted(&feed, &ids(&["poe1/100-s4", "poe2/50-s5"])),
            ids(&["poe1/200-s5"])
        );
        // The same build with an older schema is older.
        assert_eq!(
            wanted(&ids(&["poe1/200-s5"]), &ids(&["poe1/200-s4"])),
            ids(&["poe1/200-s5"])
        );
        // An ID that does not read is skipped, on either side.
        assert_eq!(
            wanted(&ids(&["odd", "poe1/200-s5"]), &ids(&["also odd"])),
            ids(&["poe1/200-s5"])
        );
    }
}
