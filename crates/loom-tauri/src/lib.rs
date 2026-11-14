//! Desktop shell (ADR-002; Phase 4 plan, decision 10). A thin adapter: the
//! commands call `loom-app`, the application's events reach the webview on
//! one channel ([`EVENT`]), and the MCP server starts and stops here.
//!
//! **User-mediated file access.** The Rust side opens the file dialogs
//! ([`FilePicker`]); the webview never sends a path. A recent build or a
//! recovery snapshot is opened by the ID the application issued. The
//! capability file grants the webview no `dialog:*` or `fs:*` permission.
//!
//! The command names are the committed list `ui/src/api/commands.json`,
//! which the UI's client reads and a test compares with these handlers.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use loom_app::{App, AppError, Config, HostCommand, api};
use loom_integrations::mcp::Server;
use tauri::{Emitter, Manager, Runtime, State};

/// The event channel the webview listens on.
pub const EVENT: &str = "loom://event";

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
    settings: PathBuf,
}

impl Ctx {
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

/// An item edit's effect (runs off the main thread).
#[tauri::command(async)]
fn preview(ctx: State<'_, Ctx>, edit: api::ItemEdit) -> Answer<api::ItemEditPreview> {
    ctx.app.preview(&edit).map_err(dto)
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
    ctx.app.mcp_status()
}

/// Sets what MCP clients may do.
#[tauri::command]
fn set_permissions(ctx: State<'_, Ctx>, permissions: api::McpPermissions) -> api::McpStatus {
    ctx.app.set_permissions(permissions);
    ctx.app.mcp_status()
}

fn lock(ctx: &Ctx) -> std::sync::MutexGuard<'_, Option<Server>> {
    match ctx.server.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Starts the MCP server (no-op when it runs).
#[tauri::command]
fn mcp_start(ctx: State<'_, Ctx>) -> Answer<api::McpStatus> {
    let mut server = lock(&ctx);
    if server.is_none() {
        *server = Some(
            Server::start(Arc::clone(&ctx.app))
                .map_err(|e| api::ApiError::new("MCP_START", e.to_string()))?,
        );
    }
    drop(server);
    Ok(ctx.app.mcp_status())
}

/// Stops the MCP server.
#[tauri::command]
fn mcp_stop(ctx: State<'_, Ctx>) -> api::McpStatus {
    let server = lock(&ctx).take();
    drop(server);
    ctx.app.mcp_status()
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

/// Loads a bundle folder the user picks, and remembers it.
#[tauri::command(async)]
fn choose_bundle(ctx: State<'_, Ctx>) -> Answer<api::Outcome> {
    let Some(dir) = ctx.picker.pick(Pick::BundleFolder, "") else {
        return Ok(nothing());
    };
    let outcome = ctx
        .app
        .execute(HostCommand::SetBundle(dir.clone()))
        .map_err(dto)?;
    if ctx.app.snapshot().data.error.is_none() {
        save_settings(&ctx.settings, &Settings { bundle: Some(dir) });
    }
    Ok(outcome)
}

/// The shell's own settings (the bundle folder).
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Settings {
    bundle: Option<PathBuf>,
}

fn load_settings(path: &std::path::Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_settings(path: &std::path::Path, settings: &Settings) {
    if let Ok(text) = serde_json::to_string_pretty(settings) {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let store = loom_persistence::Store::new();
        let _ = store.write_replacing(path, text.as_bytes());
    }
}

/// Every command name, as `ui/src/api/commands.json` lists them.
pub const COMMANDS: [&str; 16] = [
    "snapshot",
    "calc_document",
    "item",
    "preview",
    "execute",
    "recent",
    "recovery",
    "mcp_status",
    "set_permissions",
    "mcp_start",
    "mcp_stop",
    "import_file",
    "open_file",
    "save_as",
    "export_xml",
    "choose_bundle",
];

/// Where the application keeps its data and which bundle it loads.
#[derive(Debug, Clone)]
pub struct Paths {
    /// The data folder.
    pub data: PathBuf,
    /// The bundle folder named by `LOOM_BUNDLE_DIR`, if set (else the
    /// remembered one).
    pub bundle: Option<PathBuf>,
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
            preview,
            execute,
            recent,
            recovery,
            mcp_status,
            set_permissions,
            mcp_start,
            mcp_stop,
            import_file,
            open_file,
            save_as,
            export_xml,
            choose_bundle,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            let paths = paths(&handle);
            let settings = paths.data.join("settings.json");
            let bundle = paths.bundle.or_else(|| load_settings(&settings).bundle);
            let emitter = handle.clone();
            let sink: loom_app::Sink = Arc::new(move |event: api::Event| {
                let _ = emitter.emit(EVENT, event);
            });
            let application = App::start(Config::new(paths.data, bundle), sink)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            app.manage(Ctx {
                app: Arc::new(application),
                picker: picker(&handle),
                server: Mutex::new(None),
                settings,
            });
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
                .set_title("Choose the game data folder (the folder that holds bundle.json)")
                .blocking_pick_folder(),
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
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    let builder = build(
        builder,
        |handle| Paths {
            data: std::env::var_os("LOOM_DATA_DIR")
                .map(PathBuf::from)
                .or_else(|| handle.path().app_data_dir().ok())
                .unwrap_or_else(|| PathBuf::from(".loom")),
            bundle: std::env::var_os("LOOM_BUNDLE_DIR").map(PathBuf::from),
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
        if let tauri::RunEvent::Exit = event
            && let Some(ctx) = handle.try_state::<Ctx>()
        {
            drop(lock(&ctx).take());
            ctx.app.stop();
        }
    });
    Ok(())
}
