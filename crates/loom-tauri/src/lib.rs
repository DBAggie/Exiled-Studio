//! Desktop shell (ADR-002; Phase 4 plan, decision 10). A thin adapter: the
//! commands call `loom-app`, the application's events reach the webview on
//! one channel ([`EVENT`]), and the MCP server starts and stops here.
//!
//! **User-mediated file access.** The Rust side opens the file dialogs
//! ([`FilePicker`]); the webview never sends a path. A recent build or a
//! recovery snapshot is opened by the ID the application issued. The
//! capability file grants the webview no `dialog:*` or `fs:*` permission.
//!
//! **Dropped files** (UX brief note 4, start page B): the window's
//! drag-drop event reaches the Rust side ([`dropped`]), which keeps the
//! path and tells the webview only what kind of file it is ([`DROP_EVENT`]);
//! the webview asks about unsaved work and then calls `open_dropped`. Tauri
//! handling drops turns off HTML5 drag and drop of files in the webview on
//! Windows; nothing uses it (a later tree editor or gem reordering that
//! drags elements would need `dragDropEnabled` reconsidered).
//!
//! The command names are the committed list `ui/src/api/commands.json`,
//! which the UI's client reads and a test compares with these handlers.

pub mod release;
pub mod updates;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use loom_app::bundles::{SETTINGS_FILE, Settings};
use loom_app::{App, AppError, Config, HostCommand, api};
use loom_integrations::mcp::Server;
use loom_integrations::mcp::token::{self as mcp_token, NoStore, OsStore, TokenStore};
use tauri::{Emitter, Manager, Runtime, State};

/// The event channel the webview listens on.
pub const EVENT: &str = "loom://event";

/// The channel for a file dragged over the window or dropped on it.
pub const DROP_EVENT: &str = "loom://drop";

/// What a file is, by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DropKind {
    /// A PoB build (`.xml`): imported.
    Pob,
    /// A build document (`.json`): opened.
    Build,
    /// Anything else: refused.
    Other,
}

/// The kind of a dropped file.
#[must_use]
pub fn drop_kind(path: &std::path::Path) -> DropKind {
    match path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("xml") => DropKind::Pob,
        Some("json") => DropKind::Build,
        _ => DropKind::Other,
    }
}

/// What the webview hears on [`DROP_EVENT`]: a file is over the window,
/// left it, or was dropped (its kind and file name, never its path).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum DropNotice {
    /// A file is over the window.
    Over,
    /// It left without a drop.
    Left,
    /// It was dropped; `open_dropped` opens it.
    Dropped {
        /// What it is.
        kind: DropKind,
        /// Its file name.
        name: String,
    },
}

/// A drop on the window: keeps the first file's path for `open_dropped`
/// and tells the webview. Called by the window's drag-drop event.
pub fn dropped<R: Runtime>(handle: &tauri::AppHandle<R>, paths: &[PathBuf]) {
    let Some(path) = paths.first() else {
        let _ = handle.emit(DROP_EVENT, DropNotice::Left);
        return;
    };
    let notice = DropNotice::Dropped {
        kind: drop_kind(path),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    };
    if let Some(ctx) = handle.try_state::<Ctx>() {
        *lock_drop(&ctx) = Some(path.clone());
    }
    let _ = handle.emit(DROP_EVENT, notice);
}

/// The kinds of file the dialogs pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// A PoB build to import (`.xml`).
    OpenPob,
    /// A build document to open (`.json`).
    OpenBuild,
    /// Where to save a build document.
    SaveBuild,
    /// Where to export PoB XML.
    ExportPob,
    /// A bundle folder.
    BundleFolder,
    /// Where to write the diagnostic package (`.zip`).
    Diagnostics,
    /// A data bundle pack to install (`.loombundle`; Phase 5, P-e).
    BundlePack,
}

/// Opens the file dialogs. `None` when the user cancelled.
pub trait FilePicker: Send + Sync {
    /// Asks the user for a path; `name` is the suggested file name.
    fn pick(&self, what: Pick, name: &str) -> Option<PathBuf>;
}

/// The shell's state.
pub struct Ctx {
    app: Arc<App>,
    picker: Box<dyn FilePicker>,
    server: Mutex<Option<Server>>,
    /// The application's `settings.json` (the MCP server's last port).
    settings: PathBuf,
    /// The last dropped file, until `open_dropped` takes it.
    drop: Mutex<Option<PathBuf>>,
    /// Where the MCP token is kept (ADR-017 Amendment 1).
    tokens: Box<dyn TokenStore>,
    /// Whether the running server's token is kept between starts.
    token_kept: std::sync::atomic::AtomicBool,
}

impl Ctx {
    /// The UI's preferences file, beside `settings.json`.
    fn ui_prefs(&self) -> PathBuf {
        self.settings.with_file_name(UI_PREFS_FILE)
    }

    /// The application.
    #[must_use]
    pub fn app(&self) -> &Arc<App> {
        &self.app
    }
}

type Answer<T> = Result<T, api::ApiError>;

fn dto(e: AppError) -> api::ApiError {
    e.dto()
}

fn nothing() -> api::Outcome {
    api::Outcome::Nothing
}

/// The latest snapshot.
#[tauri::command]
fn snapshot(ctx: State<'_, Ctx>) -> api::Snapshot {
    ctx.app.snapshot()
}

/// The latest calc result document, with its version (`null` before one).
#[tauri::command]
fn calc_document(ctx: State<'_, Ctx>) -> Option<(u64, String)> {
    ctx.app
        .calc_document()
        .map(|(v, d)| (v, d.as_ref().clone()))
}

/// One item.
#[tauri::command]
fn item(ctx: State<'_, Ctx>, id: u32) -> Answer<api::ItemView> {
    ctx.app.item(id).map_err(dto)
}

/// The gems the game data offers (for a gem picker); for supports,
/// whether each fits socket group `group`.
#[tauri::command]
fn gems(ctx: State<'_, Ctx>, group: Option<u32>) -> Answer<Vec<api::GemCatalogEntry>> {
    ctx.app.gems(group).map_err(dto)
}

/// An item edit's effect (runs off the main thread).
#[tauri::command(async)]
fn preview(ctx: State<'_, Ctx>, edit: api::ItemEdit) -> Answer<api::ItemEditPreview> {
    ctx.app.preview(&edit).map_err(dto)
}

/// The loaded data's passive tree (static while the data is loaded).
#[tauri::command(async)]
fn tree_layout(ctx: State<'_, Ctx>) -> Answer<api::TreeLayout> {
    ctx.app
        .tree_layout()
        .map(|l| l.as_ref().clone())
        .map_err(dto)
}

/// The loaded data's passive tree sprites (static while the data is
/// loaded; `APP_NO_DATA` without them).
#[tauri::command(async)]
fn tree_sprites(ctx: State<'_, Ctx>) -> Answer<api::TreeSprites> {
    ctx.app
        .tree_sprites()
        .map(|s| s.as_ref().clone())
        .map_err(dto)
}

/// What clicking a node would do (runs off the main thread).
#[tauri::command(async)]
fn preview_tree(ctx: State<'_, Ctx>, spec: u32, node: u32) -> Answer<api::TreePreview> {
    ctx.app.preview_tree(spec, node).map_err(dto)
}

/// Runs an API command.
#[tauri::command(async)]
fn execute(ctx: State<'_, Ctx>, command: api::Command) -> Answer<api::Outcome> {
    ctx.app.execute(HostCommand::Api(command)).map_err(dto)
}

/// Recent builds.
#[tauri::command]
fn recent(ctx: State<'_, Ctx>) -> Vec<api::RecentEntry> {
    ctx.app.recent()
}

/// Recovery snapshots of ended sessions.
#[tauri::command]
fn recovery(ctx: State<'_, Ctx>) -> Vec<api::RecoveryEntry> {
    ctx.app.recovery()
}

/// The MCP server's state.
#[tauri::command]
fn mcp_status(ctx: State<'_, Ctx>) -> api::McpStatus {
    status(&ctx)
}

/// Sets what MCP clients may do.
#[tauri::command]
fn set_permissions(ctx: State<'_, Ctx>, permissions: api::McpPermissions) -> api::McpStatus {
    ctx.app.set_permissions(permissions);
    status(&ctx)
}

fn lock(ctx: &Ctx) -> std::sync::MutexGuard<'_, Option<Server>> {
    match ctx.server.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn lock_drop(ctx: &Ctx) -> std::sync::MutexGuard<'_, Option<PathBuf>> {
    match ctx.drop.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Opens the last dropped file: a PoB build is imported, a build document
/// opened; `nothing` when none is waiting.
///
/// # Errors
/// `UI_DROP` for another kind of file; else the command's.
#[tauri::command(async)]
fn open_dropped(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    let Some(path) = lock_drop(&ctx).take() else {
        return Ok(nothing());
    };
    let command = match drop_kind(&path) {
        DropKind::Pob => HostCommand::ImportFile(path),
        DropKind::Build => HostCommand::Open(path),
        DropKind::Other => {
            return Err(api::ApiError::new(
                "UI_DROP",
                "not a build file (.xml or .json)",
            ));
        }
    };
    ctx.app.execute(command).map_err(dto)
}

/// Starts the MCP server (no-op when it runs), on its last port when that
/// is free (UX-26: a client's configured URL keeps working), and
/// remembers the port.
#[tauri::command]
fn mcp_start(ctx: State<'_, Ctx>) -> Answer<api::McpStatus> {
    let mut server = lock(&ctx);
    if server.is_none() {
        let last = Settings::load(&ctx.settings).mcp_port;
        let (token, kept) = mcp_token::kept_token(&*ctx.tokens)
            .map_err(|e| api::ApiError::new("MCP_START", e.to_string()))?;
        let started = Server::start_with(Arc::clone(&ctx.app), last, Some(token))
            .map_err(|e| api::ApiError::new("MCP_START", e.to_string()))?;
        ctx.token_kept
            .store(kept, std::sync::atomic::Ordering::SeqCst);
        let port = started.addr().port();
        if last != Some(port) {
            Settings::update(&ctx.settings, |s| s.mcp_port = Some(port));
        }
        *server = Some(started);
    }
    drop(server);
    Ok(status(&ctx))
}

/// Stops the MCP server (a kept token stays for the next start).
#[tauri::command]
fn mcp_stop(ctx: State<'_, Ctx>) -> api::McpStatus {
    let server = lock(&ctx).take();
    drop(server);
    status(&ctx)
}

/// Makes a new MCP token and keeps it (ADR-017 Amendment 1, Rotate). A
/// running server restarts on its port with it at once, so clients with
/// the old token are refused.
///
/// # Errors
/// `MCP_START` when the random source or the restart fails.
#[tauri::command]
fn mcp_rotate(ctx: State<'_, Ctx>) -> Answer<api::McpStatus> {
    let start = |e: std::io::Error| api::ApiError::new("MCP_START", e.to_string());
    let (token, kept) = mcp_token::rotate(&*ctx.tokens).map_err(start)?;
    let mut server = lock(&ctx);
    if let Some(mut running) = server.take() {
        let port = running.addr().port();
        running.stop();
        let restarted =
            Server::start_with(Arc::clone(&ctx.app), Some(port), Some(token)).map_err(start)?;
        ctx.token_kept
            .store(kept, std::sync::atomic::Ordering::SeqCst);
        *server = Some(restarted);
    }
    drop(server);
    Ok(status(&ctx))
}

/// The application's MCP status with where the token is kept.
fn status(ctx: &Ctx) -> api::McpStatus {
    let mut status = ctx.app.mcp_status();
    status.token_kept = status.running && ctx.token_kept.load(std::sync::atomic::Ordering::SeqCst);
    status
}

fn with_pick(
    ctx: &Ctx,
    what: Pick,
    name: &str,
    f: impl FnOnce(PathBuf) -> HostCommand,
) -> Answer<api::Outcome> {
    match ctx.picker.pick(what, name) {
        None => Ok(nothing()),
        Some(path) => ctx.app.execute(f(path)).map_err(dto),
    }
}

/// Imports a PoB XML file the user picks.
#[tauri::command(async)]
fn import_file(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    with_pick(&ctx, Pick::OpenPob, "", HostCommand::ImportFile)
}

/// Opens a build document the user picks.
#[tauri::command(async)]
fn open_file(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    with_pick(&ctx, Pick::OpenBuild, "", HostCommand::Open)
}

fn suggested(ctx: &Ctx, ext: &str) -> String {
    let name = ctx
        .app
        .snapshot()
        .build
        .map(|b| b.name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "build".to_owned());
    format!("{name}.{ext}")
}

/// Saves the build where the user picks.
#[tauri::command(async)]
fn save_as(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    let name = suggested(&ctx, "json");
    with_pick(&ctx, Pick::SaveBuild, &name, HostCommand::SaveAs)
}

/// Exports the build as PoB XML where the user picks.
#[tauri::command(async)]
fn export_xml(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    let name = suggested(&ctx, "xml");
    with_pick(&ctx, Pick::ExportPob, &name, HostCommand::ExportXml)
}

/// Loads a bundle folder the user picks (development: an unsigned folder,
/// not installed); the application remembers it when it loads.
#[tauri::command(async)]
fn choose_bundle(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    with_pick(&ctx, Pick::BundleFolder, "", HostCommand::SetBundle)
}

/// Installs a data bundle pack the user picks (Phase 5, P-e): signed by a
/// trusted key it loads; unsigned or by an unknown key it is quarantined
/// until the user accepts it.
#[tauri::command(async)]
fn install_bundle(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    with_pick(&ctx, Pick::BundlePack, "", HostCommand::InstallBundle)
}

/// The first screen has rendered. In a smoke run (`--smoke <file>`,
/// [`release::Smoke`]) the build info is written and the application quits;
/// otherwise nothing happens.
#[tauri::command]
fn app_ready<R: Runtime>(handle: tauri::AppHandle<R>) {
    if let Some(smoke) = handle.try_state::<release::Smoke>() {
        smoke.write(true);
        handle.exit(0);
    }
}

/// Application updates: the mode, this package, the last check (Phase 5,
/// P-d; [`updates`]).
#[tauri::command]
fn update_status(updates: State<'_, updates::Updates>) -> api::UpdateStatus {
    updates.status()
}

/// Sets how the application looks for its updates.
#[tauri::command]
fn set_update_mode(
    updates: State<'_, updates::Updates>,
    mode: api::UpdateMode,
) -> api::UpdateStatus {
    updates.set_mode(mode);
    updates.status()
}

/// Checks for an update now (the user asked).
#[tauri::command]
async fn check_updates<R: Runtime>(
    handle: tauri::AppHandle<R>,
    updates: State<'_, updates::Updates>,
) -> Answer<api::UpdateStatus> {
    updates.check(&handle).await
}

/// Downloads, verifies, and installs the offered update, then restarts:
/// the user approved it in the UI's prompt. Refused, before anything is
/// downloaded, for a package that only notifies, in strict-offline mode,
/// and without an offer.
#[tauri::command]
async fn install_update<R: Runtime>(
    handle: tauri::AppHandle<R>,
    updates: State<'_, updates::Updates>,
) -> Answer<()> {
    let update = updates.to_install()?;
    tracing::info!(version = %update.version, "installing an update");
    let result = update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| api::ApiError::new("UPDATE_INSTALL", e.to_string()));
    if let Err(e) = result {
        updates.failed(e.clone());
        return Err(e);
    }
    shutdown(&handle);
    handle.restart()
}

/// Stops the application (a change made since the last autosave is kept
/// for recovery) and the MCP server; before the process exits, also when
/// an update's installer replaces it.
pub(crate) fn shutdown<R: Runtime>(handle: &tauri::AppHandle<R>) {
    if let Some(ctx) = handle.try_state::<Ctx>() {
        drop(lock(&ctx).take());
        ctx.app.stop();
    }
}

/// The crash records, newest first (Phase 5, P-f).
#[tauri::command]
fn crashes(ctx: State<'_, Ctx>) -> Vec<api::CrashRecord> {
    ctx.app.crashes()
}

/// Deletes a crash record the user has seen.
#[tauri::command]
fn dismiss_crash(ctx: State<'_, Ctx>, id: String) -> Answer<()> {
    ctx.app.dismiss_crash(&id).map_err(dto)
}

/// The diagnostic package for the user to preview (Phase 5, P-f).
#[tauri::command(async)]
fn diagnostics(ctx: State<'_, Ctx>) -> Vec<api::DiagnosticFile> {
    ctx.app.diagnostics()
}

/// Writes the previewed diagnostic package where the user picks.
#[tauri::command(async)]
fn export_diagnostics(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    with_pick(
        &ctx,
        Pick::Diagnostics,
        "exiled-studio-diagnostics.zip",
        HostCommand::ExportDiagnostics,
    )
}

/// The UI's own preferences (B-5: the user's dashboards), an opaque JSON
/// document the shell keeps in `<data>/ui-prefs.json`; the application
/// never reads it.
const UI_PREFS_FILE: &str = "ui-prefs.json";

/// The largest preferences document kept.
pub const MAX_UI_PREFS: usize = 64 * 1024;

/// The UI's preferences, if any were saved (`null` otherwise, or when the
/// file does not read: a preference, not data).
#[tauri::command]
fn ui_prefs(ctx: State<'_, Ctx>) -> Option<String> {
    let path = ctx.ui_prefs();
    // A file grown past the limit by hand is not read.
    let len = std::fs::metadata(&path).ok()?.len();
    if usize::try_from(len).map_or(true, |n| n > MAX_UI_PREFS) {
        return None;
    }
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .map(|_| text)
}

/// Saves the UI's preferences.
///
/// # Errors
/// `UI_PREFS` when the text is not JSON or is larger than
/// [`MAX_UI_PREFS`]; `PERSIST_*` when it cannot be written.
#[tauri::command]
fn set_ui_prefs(ctx: State<'_, Ctx>, text: String) -> Answer<()> {
    if text.len() > MAX_UI_PREFS || serde_json::from_str::<serde_json::Value>(&text).is_err() {
        return Err(api::ApiError::new(
            "UI_PREFS",
            "not a JSON document of at most 64 KiB",
        ));
    }
    let path = ctx.ui_prefs();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    loom_persistence::Store::new()
        .write_replacing(&path, text.as_bytes())
        .map_err(|e| api::ApiError::new(e.code(), e.to_string()))
}

/// Every command name, as `ui/src/api/commands.json` lists them.
pub const COMMANDS: [&str; 34] = [
    "snapshot",
    "calc_document",
    "item",
    "gems",
    "preview",
    "execute",
    "recent",
    "recovery",
    "mcp_status",
    "set_permissions",
    "mcp_start",
    "mcp_stop",
    "mcp_rotate",
    "import_file",
    "open_file",
    "save_as",
    "export_xml",
    "choose_bundle",
    "open_dropped",
    "ui_prefs",
    "set_ui_prefs",
    "tree_layout",
    "tree_sprites",
    "preview_tree",
    "crashes",
    "dismiss_crash",
    "diagnostics",
    "export_diagnostics",
    "app_ready",
    "update_status",
    "set_update_mode",
    "check_updates",
    "install_update",
    "install_bundle",
];

/// Where the application keeps its data and which bundle it loads.
#[derive(Debug, Clone)]
pub struct Paths {
    /// The data folder.
    pub data: PathBuf,
    /// The bundle folder named by `LOOM_BUNDLE_DIR`, if set (else the
    /// application takes the remembered one or the newest installed one,
    /// `loom_app::bundles`).
    pub bundle: Option<PathBuf>,
    /// Whether the MCP token is kept between starts in the OS's protected
    /// store (ADR-017 Amendment 1); tests pass `false`, so they never
    /// touch the user's credentials.
    pub keep_token: bool,
    /// The folder of data packs the installer ships (Phase 5, P-e), if
    /// any: each one not installed yet is installed at start.
    pub shipped: Option<PathBuf>,
}

/// Whether this build trusts the public test data key: debug and `e2e`
/// builds only (`docs/release/signing.md`, "The test key"). A released
/// build is neither; its smoke run checks so.
pub const TEST_KEYS: bool = cfg!(debug_assertions) || cfg!(feature = "e2e");

/// Installs the shipped packs not installed yet (a first start, or an
/// update that ships newer data). A pack that fails is logged; the
/// application starts without it.
fn install_shipped(app: &App, dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut packs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e == loom_app::install::PACK_EXTENSION)
        })
        .collect();
    packs.sort();
    for pack in packs {
        if let Err(e) = app.execute(HostCommand::InstallBundle(pack.clone())) {
            tracing::warn!(code = %e.code, pack = %pack.display(), "shipped bundle not installed");
        }
    }
}

/// Adds the commands and starts the application in `setup`.
pub fn build<R: Runtime>(
    builder: tauri::Builder<R>,
    paths: impl FnOnce(&tauri::AppHandle<R>) -> Paths + Send + 'static,
    picker: impl FnOnce(&tauri::AppHandle<R>) -> Box<dyn FilePicker> + Send + 'static,
) -> tauri::Builder<R> {
    builder
        .invoke_handler(tauri::generate_handler![
            snapshot,
            calc_document,
            item,
            gems,
            preview,
            execute,
            recent,
            recovery,
            mcp_status,
            set_permissions,
            mcp_start,
            mcp_stop,
            mcp_rotate,
            import_file,
            open_file,
            save_as,
            export_xml,
            choose_bundle,
            open_dropped,
            ui_prefs,
            set_ui_prefs,
            tree_layout,
            tree_sprites,
            preview_tree,
            crashes,
            dismiss_crash,
            diagnostics,
            export_diagnostics,
            app_ready,
            update_status,
            set_update_mode,
            check_updates,
            install_update,
            install_bundle,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::DragDrop(drag) = event {
                let handle = window.app_handle();
                match drag {
                    tauri::DragDropEvent::Enter { .. } => {
                        let _ = handle.emit(DROP_EVENT, DropNotice::Over);
                    }
                    tauri::DragDropEvent::Leave => {
                        let _ = handle.emit(DROP_EVENT, DropNotice::Left);
                    }
                    tauri::DragDropEvent::Drop { paths, .. } => dropped(handle, paths),
                    _ => {}
                }
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let paths = paths(&handle);
            // Logs and crash records first, so the start is recorded
            // (Phase 5, P-f). A log folder that cannot be made leaves the
            // application without logs, never without a start.
            if let Ok(logs) =
                loom_app::logs::Logs::open(&paths.data, loom_app::logs::Redactor::from_env())
            {
                let _ = tracing::subscriber::set_global_default(logs.subscriber());
            }
            loom_app::crash::install(paths.data.clone(), loom_app::logs::Redactor::from_env());
            let settings = paths.data.join(SETTINGS_FILE);
            let settings_for_updates = settings.clone();
            let emitter = handle.clone();
            let sink: loom_app::Sink = Arc::new(move |event: api::Event| {
                let _ = emitter.emit(EVENT, event);
            });
            let application = App::start(
                Config::new(paths.data, paths.bundle).with_test_keys(TEST_KEYS),
                sink,
            )
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            if let Some(shipped) = &paths.shipped {
                install_shipped(&application, shipped);
            }
            app.manage(Ctx {
                app: Arc::new(application),
                picker: picker(&handle),
                server: Mutex::new(None),
                settings,
                drop: Mutex::new(None),
                tokens: if paths.keep_token {
                    Box::new(OsStore::new())
                } else {
                    Box::new(NoStore)
                },
                token_kept: std::sync::atomic::AtomicBool::new(false),
            });
            let plugin = handle.try_state::<updates::Plugin>().is_some();
            app.manage(updates::Updates::new(plugin, settings_for_updates));
            updates::at_start(&handle);
            Ok(())
        })
}

/// The real dialogs (`tauri-plugin-dialog`, called from Rust).
struct Dialogs<R: Runtime>(tauri::AppHandle<R>);

impl<R: Runtime> FilePicker for Dialogs<R> {
    fn pick(&self, what: Pick, name: &str) -> Option<PathBuf> {
        use tauri_plugin_dialog::DialogExt;
        let dialog = self.0.dialog().file();
        let path = match what {
            Pick::OpenPob => dialog
                .add_filter("Path of Building build", &["xml"])
                .blocking_pick_file(),
            Pick::OpenBuild => dialog
                .add_filter("Exiled Studio build", &["json"])
                .blocking_pick_file(),
            Pick::SaveBuild => dialog
                .add_filter("Exiled Studio build", &["json"])
                .set_file_name(name)
                .blocking_save_file(),
            Pick::ExportPob => dialog
                .add_filter("Path of Building build", &["xml"])
                .set_file_name(name)
                .blocking_save_file(),
            Pick::BundleFolder => dialog
                .set_title(
                    "Choose an unsigned game data folder for development (it holds bundle.json)",
                )
                .blocking_pick_folder(),
            Pick::Diagnostics => dialog
                .add_filter("Diagnostic package", &["zip"])
                .set_file_name(name)
                .blocking_save_file(),
            Pick::BundlePack => dialog
                .add_filter("Exiled Studio game data", &["loombundle"])
                .blocking_pick_file(),
        };
        path.and_then(|p| p.into_path().ok())
    }
}

/// The end-to-end test picker (feature `e2e` only): fixed files in the
/// folder `LOOM_E2E_DIR` names, so WebDriver runs need no native dialog.
#[cfg(feature = "e2e")]
struct E2ePicker(PathBuf);

#[cfg(feature = "e2e")]
impl FilePicker for E2ePicker {
    fn pick(&self, what: Pick, _name: &str) -> Option<PathBuf> {
        let file = match what {
            Pick::OpenPob => "import.xml",
            Pick::OpenBuild | Pick::SaveBuild => "saved.json",
            Pick::ExportPob => "export.xml",
            Pick::Diagnostics => "diagnostics.zip",
            Pick::BundlePack => "bundle.loombundle",
            Pick::BundleFolder => return None,
        };
        Some(self.0.join(file))
    }
}

/// Runs the desktop application.
///
/// # Errors
/// Tauri's.
pub fn run() -> tauri::Result<()> {
    let smoke = release::Smoke::from_args(std::env::args_os());
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::Plugin);
    if let Some(smoke) = smoke {
        smoke.watchdog();
        builder = builder.manage(smoke);
    }
    let builder = build(
        builder,
        |handle| Paths {
            // LOOM_DATA_DIR (development and tests), else the portable
            // package's folder, else the user's application data folder.
            data: std::env::var_os("LOOM_DATA_DIR")
                .map(PathBuf::from)
                .or_else(release::portable_data)
                .or_else(|| handle.path().app_data_dir().ok())
                .unwrap_or_else(|| PathBuf::from(".loom")),
            bundle: std::env::var_os("LOOM_BUNDLE_DIR").map(PathBuf::from),
            // End-to-end runs keep nothing in the user's credentials.
            keep_token: !cfg!(feature = "e2e"),
            // The installer's resources (`bundles/`), when it ships data.
            shipped: handle.path().resource_dir().ok().map(|d| d.join("bundles")),
        },
        |handle| {
            #[cfg(feature = "e2e")]
            if let Some(dir) = std::env::var_os("LOOM_E2E_DIR") {
                let _ = handle;
                return Box::new(E2ePicker(PathBuf::from(dir)));
            }
            Box::new(Dialogs(handle.clone()))
        },
    );
    let app = builder.build(tauri::generate_context!())?;
    app.run(|handle, event| {
        // Tauri's exit does not drop managed state: stop the application
        // here, so a change made since the last autosave is kept for
        // recovery and the MCP server stops.
        if let tauri::RunEvent::Exit = event {
            shutdown(handle);
        }
    });
    Ok(())
}
