//! The application task (ADR-004; Phase 4 plan, decisions 2, 5, 8, 11):
//! one writer thread owns the [`Session`] and runs commands in order; a
//! worker thread calculates snapshots in the background; queries read the
//! latest published projection without waiting for either.
//!
//! - **Background calculation (ADR-003).** Each change sends the new state
//!   to the worker. A newer change cancels the running calculation through
//!   the engine's hook, and a result whose version is not the latest is
//!   discarded (stale).
//! - **Autosave (ADR-009).** While the build has unsaved changes, the
//!   writer writes a recovery snapshot at most every `autosave` interval;
//!   a save or closing the build deletes it. Stopping the application with
//!   unsaved changes keeps it, so the next start offers it.
//! - **Change requests (ADR-017).** An MCP client's item edit is a request:
//!   refused while the permission is off, applied at once under the
//!   standing grant, otherwise pending until the user approves or declines
//!   it. Applied requests go through the same edit command, so they are
//!   undoable and in the history.
//! - **Logs (ADR-014; Phase 5, P-f).** Each command runs in a `command`
//!   span with its correlation ID and ends with one event (its name, never
//!   its content; the outcome or error code; the time taken). A
//!   calculation runs in a `calc` span with the correlation of the command
//!   that asked for it. Imported and opened files are named by extension,
//!   size, and SHA-256 only ([`crate::logs`]).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use loom_calc::CalcResult;
use loom_persistence::Store;
use loom_persistence::recent::{Entry as RecentFileEntry, Recent};
use loom_persistence::recovery::{Header, Recovery, StepRef};
use loom_serialization::dto::app::v1 as api;

use crate::bundles::Library;
use crate::data::GameData;
use crate::error::{AppError, Code};
use crate::session::{self, Session, State};

/// Where events go (the host's: Tauri's emitter, a test's channel).
pub type Sink = Arc<dyn Fn(api::Event) + Send + Sync>;

/// How many change requests may wait for the user at once.
pub const MAX_PENDING_REQUESTS: usize = 16;

/// The application's configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// The application's data folder (`recovery/`, `recent.json`).
    pub data_dir: PathBuf,
    /// The bundle folder to load at start (`LOOM_BUNDLE_DIR`), if any;
    /// else the remembered choice or the newest installed bundle
    /// ([`crate::bundles`]).
    pub bundle: Option<PathBuf>,
    /// This instance's recovery ID (ASCII letters, digits, `-`).
    pub instance: String,
    /// The shortest time between two recovery snapshots.
    pub autosave: Duration,
}

impl Config {
    /// A configuration with a fresh instance ID and a 2 s autosave.
    #[must_use]
    pub fn new(data_dir: PathBuf, bundle: Option<PathBuf>) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        Self {
            data_dir,
            bundle,
            instance: format!("{}-{nanos:x}", std::process::id()),
            autosave: Duration::from_secs(2),
        }
    }
}

/// A command from the host: the API's, or one with a path the user picked
/// in a dialog (never from the webview or an addon; decision 3).
#[derive(Debug, Clone)]
pub enum HostCommand {
    /// An API command.
    Api(api::Command),
    /// Imports a PoB XML file.
    ImportFile(PathBuf),
    /// Opens a build document.
    Open(PathBuf),
    /// Saves the build at a path.
    SaveAs(PathBuf),
    /// Exports the build as PoB XML.
    ExportXml(PathBuf),
    /// Loads another bundle folder.
    SetBundle(PathBuf),
    /// Writes the diagnostic package last previewed (or, if none was, a
    /// fresh one) as a zip ([`crate::diagnostics`]).
    ExportDiagnostics(PathBuf),
}

impl HostCommand {
    /// The command's name for the logs (never its content).
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::ImportFile(_) => "import-file".to_owned(),
            Self::Open(_) => "open".to_owned(),
            Self::SaveAs(_) => "save-as".to_owned(),
            Self::ExportXml(_) => "export-xml".to_owned(),
            Self::SetBundle(_) => "set-bundle".to_owned(),
            Self::ExportDiagnostics(_) => "export-diagnostics".to_owned(),
            // Named without serializing its text.
            Self::Api(api::Command::ImportText { .. }) => "import-text".to_owned(),
            Self::Api(c) => match serde_json::to_value(c) {
                Ok(serde_json::Value::String(s)) => s,
                Ok(serde_json::Value::Object(m)) => m.keys().next().cloned().unwrap_or_default(),
                _ => "api".to_owned(),
            },
        }
    }
}

/// A file's extension, size, and SHA-256 for the logs, never its name.
fn file_digest(path: &Path) -> Option<(String, usize, String)> {
    let bytes = std::fs::read(path).ok()?;
    let (size, sha) = crate::logs::digest(&bytes);
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    Some((ext, size, sha))
}

/// What queries read: the latest published state.
#[derive(Debug, Clone)]
struct Published {
    snapshot: api::Snapshot,
    state: Option<Arc<State>>,
    data: Option<Arc<GameData>>,
    calc: Option<(u64, Arc<CalcResult>, Arc<String>)>,
    requests: Vec<api::ChangeRequest>,
    permissions: api::McpPermissions,
    endpoint: Option<(String, String)>,
    recent: Vec<api::RecentEntry>,
    recovery: Vec<api::RecoveryEntry>,
    /// The open build's generation ([`Session::generation`]).
    generation: u64,
}

type Reply = Sender<Result<api::Outcome, AppError>>;

enum Msg {
    Command {
        command: HostCommand,
        correlation: u64,
        reply: Reply,
    },
    CalcDone {
        version: u64,
        result: Result<(Arc<CalcResult>, Arc<String>), AppError>,
    },
    Request {
        request: Box<api::ChangeRequest>,
        /// The build generation its preview was made against.
        generation: u64,
        reply: Sender<Result<api::ChangeRequest, AppError>>,
    },
    Permissions(api::McpPermissions, Sender<()>),
    Endpoint(Option<(String, String)>, Sender<()>),
    Stop,
}

struct Job {
    version: u64,
    /// The command that asked for it, if any (the logs' correlation).
    correlation: Option<u64>,
    state: Arc<State>,
    data: Arc<GameData>,
}

struct Shared {
    published: RwLock<Published>,
    /// The latest version a calculation was asked for (the cancel hook).
    latest: AtomicU64,
    correlation: AtomicU64,
    requests: AtomicU64,
}

/// The running application.
pub struct App {
    tx: Sender<Msg>,
    shared: Arc<Shared>,
    threads: std::sync::Mutex<Vec<JoinHandle<()>>>,
    /// The data folder (crash records, logs, the diagnostic package).
    data_dir: PathBuf,
    /// The diagnostic package last previewed, which an export writes.
    previewed: std::sync::Mutex<Option<crate::diagnostics::Package>>,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn stopped() -> AppError {
    AppError::new(Code::Stopped, "the application has stopped")
}

impl App {
    /// Starts the application: loads the bundle, opens the recovery folder,
    /// and starts the writer and the calculation worker.
    ///
    /// # Errors
    /// The recovery folder cannot be opened (`PERSIST_*`).
    pub fn start(config: Config, sink: Sink) -> Result<Self, AppError> {
        let store = Store::new();
        let recovery = Recovery::open(&config.data_dir.join("recovery"), &config.instance)
            .map_err(|e| AppError::layer("PERSIST_RECOVERY", e))?;
        let recent = Recent::at(&config.data_dir.join("recent.json"));
        tracing::info!(
            target: "loom::app",
            version = env!("CARGO_PKG_VERSION"),
            os = std::env::consts::OS,
            "start"
        );
        let mut session = Session::new(store);
        let mut library = Library::new(&config.data_dir);
        let (data, notice) = library.start(config.bundle.as_deref());
        session.set_installed(library.installed().to_vec());
        if let Some(data) = data {
            session.set_loaded(data);
            session.set_data_notice(notice);
        }
        let idle = api::CalcStatus {
            state: api::CalcState::Idle,
            version: 0,
            result_state: None,
            error: None,
        };
        let published = Published {
            snapshot: session.snapshot(idle),
            state: None,
            data: session.data().cloned(),
            calc: None,
            requests: Vec::new(),
            permissions: api::McpPermissions {
                requests: false,
                standing_grant: false,
            },
            endpoint: None,
            recent: Vec::new(),
            recovery: Vec::new(),
            generation: 0,
        };
        let shared = Arc::new(Shared {
            published: RwLock::new(published),
            latest: AtomicU64::new(0),
            correlation: AtomicU64::new(0),
            requests: AtomicU64::new(0),
        });
        let (tx, rx) = mpsc::channel();
        let (jobs_tx, jobs_rx) = mpsc::channel::<Job>();
        let worker = {
            let tx = tx.clone();
            let shared = Arc::clone(&shared);
            std::thread::Builder::new()
                .name("loom-calc".to_owned())
                .spawn(move || worker(&jobs_rx, &tx, &shared))
                .map_err(|e| AppError::layer("APP_THREAD", e))?
        };
        let writer = Writer {
            session,
            library,
            shared: Arc::clone(&shared),
            sink,
            jobs: jobs_tx,
            recovery,
            recent,
            autosave: config.autosave,
            autosave_due: None,
            calc: CalcTrack::default(),
            requests: Vec::new(),
            permissions: api::McpPermissions {
                requests: false,
                standing_grant: false,
            },
            endpoint: None,
            recent_list: Vec::new(),
            recovery_list: Vec::new(),
            request_generation: std::collections::BTreeMap::new(),
            generation: 0,
        };
        let mut writer = writer;
        writer.refresh_lists();
        writer.publish();
        let writer = std::thread::Builder::new()
            .name("loom-app".to_owned())
            .spawn(move || writer.run(&rx))
            .map_err(|e| AppError::layer("APP_THREAD", e))?;
        Ok(Self {
            tx,
            shared,
            threads: std::sync::Mutex::new(vec![writer, worker]),
            data_dir: config.data_dir,
            previewed: std::sync::Mutex::new(None),
        })
    }

    /// The crash records, newest first ([`crate::crash`]).
    #[must_use]
    pub fn crashes(&self) -> Vec<api::CrashRecord> {
        crate::crash::list(&self.data_dir)
            .into_iter()
            .map(|r| api::CrashRecord {
                id: r.id,
                at: r.at,
                version: r.version,
                thread: r.thread,
                message: r.message,
                location: r.location,
            })
            .collect()
    }

    /// Deletes crash record `id`.
    ///
    /// # Errors
    /// `APP_NOT_FOUND` when there is no such record.
    pub fn dismiss_crash(&self, id: &str) -> Result<(), AppError> {
        if crate::crash::dismiss(&self.data_dir, id) {
            Ok(())
        } else {
            Err(AppError::new(Code::NotFound, "no such crash record"))
        }
    }

    /// The diagnostic package, for the user to preview; an export then
    /// writes exactly these files ([`crate::diagnostics`]).
    #[must_use]
    pub fn diagnostics(&self) -> Vec<api::DiagnosticFile> {
        let package = self.collect_diagnostics();
        let out = package
            .iter()
            .map(|f| api::DiagnosticFile {
                name: f.name.clone(),
                bytes: f.text.len() as u64,
                text: f.text.clone(),
            })
            .collect();
        *self
            .previewed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(package);
        out
    }

    fn collect_diagnostics(&self) -> crate::diagnostics::Package {
        let about = self.read(|p| crate::diagnostics::About {
            data_version: p
                .snapshot
                .data
                .data_version
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
            bundle: p.snapshot.data.active.clone(),
            build_open: p.snapshot.build.is_some(),
        });
        crate::diagnostics::collect(&self.data_dir, &about, &crate::logs::Redactor::from_env())
    }

    /// Writes the diagnostic package to `path`: the one last previewed
    /// (kept, so a second save or a retry after a failed write writes the
    /// same files), or a fresh one when nothing was previewed.
    fn export_diagnostics(&self, path: &Path) -> Result<api::Outcome, AppError> {
        if self.shared.latest.load(Ordering::SeqCst) == u64::MAX {
            return Err(stopped());
        }
        let previewed = self
            .previewed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let package = previewed.unwrap_or_else(|| self.collect_diagnostics());
        let bytes =
            crate::diagnostics::zip(&package).map_err(|e| AppError::layer("APP_DIAGNOSTICS", e))?;
        std::fs::write(path, bytes).map_err(|e| AppError::layer("APP_DIAGNOSTICS", e))?;
        tracing::info!(target: "loom::diagnostics", files = package.len(), "exported");
        // `Done` (the build unchanged): the shell's `nothing` means the user
        // cancelled the dialog.
        Ok(api::Outcome::Done {
            version: self.read(|p| p.snapshot.version),
        })
    }

    /// Runs a command and waits for its outcome.
    ///
    /// # Errors
    /// The command's error; `APP_STOPPED` after [`App::stop`].
    pub fn execute(&self, command: HostCommand) -> Result<api::Outcome, AppError> {
        // Written by the shell, never through the writer: it reads only the
        // data folder.
        if let HostCommand::ExportDiagnostics(path) = &command {
            return self.export_diagnostics(path);
        }
        let correlation = self.shared.correlation.fetch_add(1, Ordering::Relaxed) + 1;
        let (reply, answer) = mpsc::channel();
        self.tx
            .send(Msg::Command {
                command,
                correlation,
                reply,
            })
            .map_err(|_| stopped())?;
        answer.recv().map_err(|_| stopped())?
    }

    fn read<T>(&self, f: impl FnOnce(&Published) -> T) -> T {
        match self.shared.published.read() {
            Ok(p) => f(&p),
            Err(poisoned) => f(&poisoned.into_inner()),
        }
    }

    /// The latest snapshot.
    #[must_use]
    pub fn snapshot(&self) -> api::Snapshot {
        self.read(|p| p.snapshot.clone())
    }

    /// The calc result document of the latest finished calculation, with
    /// its version.
    #[must_use]
    pub fn calc_document(&self) -> Option<(u64, Arc<String>)> {
        self.read(|p| p.calc.as_ref().map(|(v, _, d)| (*v, Arc::clone(d))))
    }

    /// The gems the game data offers, for a gem picker; for supports,
    /// whether each fits socket group `group` of the active skill set.
    ///
    /// # Errors
    /// `APP_NO_DATA`; `APP_NO_BUILD` or `APP_SKILL` when `group` names no
    /// group.
    pub fn gems(&self, group: Option<u32>) -> Result<Vec<api::GemCatalogEntry>, AppError> {
        let (data, state) = self.read(|p| (p.data.clone(), p.state.clone()));
        let data = data.ok_or_else(|| AppError::new(Code::NoData, "no game data is loaded"))?;
        let found = match group {
            Some(g) => {
                let state =
                    state.ok_or_else(|| AppError::new(Code::NoBuild, "no build is open"))?;
                Some(crate::session::group_of(&state.build, g)?.clone())
            }
            None => None,
        };
        Ok(crate::gems::catalog(data.bundle, found.as_ref()))
    }

    /// One item of the open build.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_EDIT_ITEM`.
    pub fn item(&self, id: u32) -> Result<api::ItemView, AppError> {
        let (state, data) = self.read(|p| (p.state.clone(), p.data.clone()));
        let state = state.ok_or_else(|| AppError::new(Code::NoBuild, "no build is open"))?;
        let item = state
            .build
            .items
            .item(loom_domain::ItemId(id))
            .ok_or_else(|| AppError::new(Code::EditItem, format!("no item {id}")))?;
        Ok(crate::view::item_view(
            &state.build,
            item,
            data.as_ref().map(|d| d.data),
        ))
    }

    /// An item edit's effect, calculated on the caller's thread with the
    /// latest snapshot (never on the writer; decision 6).
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, the edit's and calculation's codes.
    pub fn preview(&self, edit: &api::ItemEdit) -> Result<api::ItemEditPreview, AppError> {
        self.preview_with_generation(edit).map(|(p, _)| p)
    }

    /// [`App::preview`], with the build generation it was made against.
    fn preview_with_generation(
        &self,
        edit: &api::ItemEdit,
    ) -> Result<(api::ItemEditPreview, u64), AppError> {
        let (state, data, calc, version, generation) = self.read(|p| {
            (
                p.state.clone(),
                p.data.clone(),
                p.calc.clone(),
                p.snapshot.version,
                p.generation,
            )
        });
        let state = state.ok_or_else(|| AppError::new(Code::NoBuild, "no build is open"))?;
        let data = data.ok_or_else(|| AppError::new(Code::NoData, "no game data is loaded"))?;
        let before = calc
            .as_ref()
            .filter(|(v, _, _)| *v == version)
            .map(|(_, r, _)| r.as_ref());
        // Abandoned when the build changes meanwhile (or the app stops).
        let shared = &self.shared;
        let cancel = || shared.latest.load(Ordering::SeqCst) != version;
        session::preview(&state, &data, edit, before, &cancel).map(|p| (p, generation))
    }

    /// The loaded data's passive tree layout (static while the data is
    /// loaded; the UI caches it by the data version).
    ///
    /// # Errors
    /// `APP_NO_DATA` when no data is loaded or it has no passive tree.
    pub fn tree_layout(&self) -> Result<Arc<api::TreeLayout>, AppError> {
        self.read(|p| p.data.as_ref().and_then(|d| d.tree_layout.clone()))
            .ok_or_else(|| AppError::new(Code::NoData, "the game data has no passive tree"))
    }

    /// The loaded data's passive tree sprites (static while the data is
    /// loaded).
    ///
    /// # Errors
    /// `APP_NO_DATA` when no data is loaded or its bundle has no sprites
    /// (the tree screen then draws its own shapes).
    pub fn tree_sprites(&self) -> Result<Arc<api::TreeSprites>, AppError> {
        // The data is taken out of the lock first: the first call reads the
        // sheets.
        self.read(|p| p.data.clone())
            .and_then(|d| d.tree_sprites())
            .ok_or_else(|| AppError::new(Code::NoData, "the game data has no tree sprites"))
    }

    /// What clicking a node of spec `spec` would do: allocate it (when it
    /// is not allocated) or deallocate it, with each field before and
    /// after; calculated on the caller's thread, never committed.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, `APP_TREE`, the calculation's codes.
    pub fn preview_tree(&self, spec: u32, node: u32) -> Result<api::TreePreview, AppError> {
        self.preview_tree_edit(spec, node, None).map(|(p, _)| p)
    }

    /// A requested tree change's effect, as [`App::preview_tree`]
    /// calculates it.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, `APP_TREE`, the calculation's codes.
    pub fn preview_tree_change(
        &self,
        change: &api::TreeChange,
    ) -> Result<api::TreePreview, AppError> {
        let (spec, edit) = crate::tree::change_edit(change);
        self.preview_tree_edit(spec, 0, Some(edit)).map(|(p, _)| p)
    }

    /// The preview of `edit` on spec `spec`, or, without one, of clicking
    /// `node`; with the build generation it was made against.
    fn preview_tree_edit(
        &self,
        spec: u32,
        node: u32,
        edit: Option<crate::tree::Edit>,
    ) -> Result<(api::TreePreview, u64), AppError> {
        let (state, data, calc, version, generation) = self.read(|p| {
            (
                p.state.clone(),
                p.data.clone(),
                p.calc.clone(),
                p.snapshot.version,
                p.generation,
            )
        });
        let state = state.ok_or_else(|| AppError::new(Code::NoBuild, "no build is open"))?;
        let data = data.ok_or_else(|| AppError::new(Code::NoData, "no game data is loaded"))?;
        let tree = data
            .tree
            .clone()
            .ok_or_else(|| AppError::new(Code::NoData, "the game data has no passive tree"))?;
        let edit = if let Some(e) = edit {
            e
        } else {
            let allocated = crate::tree::rules_spec(
                &state.build,
                crate::tree::spec_of(&state.build, spec)?,
                &tree,
            )
            .nodes
            .contains(&node);
            if allocated {
                crate::tree::Edit::Deallocate(node)
            } else {
                crate::tree::Edit::Allocate(node)
            }
        };
        let before = calc
            .as_ref()
            .filter(|(v, _, _)| *v == version)
            .map(|(_, r, _)| r.as_ref());
        let shared = &self.shared;
        let cancel = || shared.latest.load(Ordering::SeqCst) != version;
        session::preview_tree(&state, &data, spec, edit, before, &cancel).map(|p| (p, generation))
    }

    /// The recent builds.
    #[must_use]
    pub fn recent(&self) -> Vec<api::RecentEntry> {
        self.read(|p| p.recent.clone())
    }

    /// Recovery snapshots of ended sessions.
    #[must_use]
    pub fn recovery(&self) -> Vec<api::RecoveryEntry> {
        self.read(|p| p.recovery.clone())
    }

    /// The MCP server's state as the application knows it.
    #[must_use]
    pub fn mcp_status(&self) -> api::McpStatus {
        self.read(|p| api::McpStatus {
            running: p.endpoint.is_some(),
            url: p.endpoint.as_ref().map(|(u, _)| u.clone()),
            token: p.endpoint.as_ref().map(|(_, t)| t.clone()),
            // The shell knows where the token is kept and says so.
            token_kept: false,
            permissions: p.permissions.clone(),
            requests: p.requests.clone(),
        })
    }

    /// One change request.
    #[must_use]
    pub fn request(&self, id: u64) -> Option<api::ChangeRequest> {
        self.read(|p| p.requests.iter().find(|r| r.id == id).cloned())
    }

    /// Sets what MCP clients may do; returns once it applies.
    pub fn set_permissions(&self, permissions: api::McpPermissions) {
        let (done, wait) = mpsc::channel();
        if self.tx.send(Msg::Permissions(permissions, done)).is_ok() {
            let _ = wait.recv();
        }
    }

    /// Records the MCP server's address and token while it runs; `None`
    /// when it stops, which also ends the standing grant (decision 11).
    pub fn set_endpoint(&self, endpoint: Option<(String, String)>) {
        let (done, wait) = mpsc::channel();
        if self.tx.send(Msg::Endpoint(endpoint, done)).is_ok() {
            let _ = wait.recv();
        }
    }

    /// An MCP client asks for an item edit (decision 11). Its effect is
    /// calculated here, on the caller's thread.
    ///
    /// # Errors
    /// `APP_REQUESTS_OFF`, `APP_REQUESTS_FULL`, the preview's codes.
    pub fn request_change(
        &self,
        client: &str,
        edit: api::ItemEdit,
        note: Option<String>,
    ) -> Result<api::ChangeRequest, AppError> {
        self.admit()?;
        let (preview, generation) = self.preview_with_generation(&edit)?;
        let mut request = self.new_request(client, note);
        request.edit = Some(edit);
        request.preview = Some(preview);
        self.submit(request, generation)
    }

    /// An MCP client asks for a passive tree change (passive tree plan,
    /// A-b), as [`App::request_change`] asks for an item edit.
    ///
    /// # Errors
    /// `APP_REQUESTS_OFF`, `APP_REQUESTS_FULL`, the preview's codes
    /// (`APP_TREE` for a change the editor refuses).
    pub fn request_tree_change(
        &self,
        client: &str,
        change: api::TreeChange,
        note: Option<String>,
    ) -> Result<api::ChangeRequest, AppError> {
        self.admit()?;
        let (spec, edit) = crate::tree::change_edit(&change);
        let (preview, generation) = self.preview_tree_edit(spec, 0, Some(edit))?;
        let mut request = self.new_request(client, note);
        request.tree = Some(change);
        request.tree_preview = Some(preview);
        self.submit(request, generation)
    }

    /// A pending request with no change yet.
    fn new_request(&self, client: &str, note: Option<String>) -> api::ChangeRequest {
        api::ChangeRequest {
            id: self.shared.requests.fetch_add(1, Ordering::Relaxed) + 1,
            client: client.chars().take(64).collect(),
            edit: None,
            note: note.map(|n| n.chars().take(500).collect()),
            preview: None,
            status: api::RequestStatus::Pending,
            error: None,
            tree: None,
            tree_preview: None,
        }
    }

    /// Hands a request to the writer, which checks it again.
    fn submit(
        &self,
        request: api::ChangeRequest,
        generation: u64,
    ) -> Result<api::ChangeRequest, AppError> {
        let (reply, answer) = mpsc::channel();
        self.tx
            .send(Msg::Request {
                request: Box::new(request),
                generation,
                reply,
            })
            .map_err(|_| stopped())?;
        answer.recv().map_err(|_| stopped())?
    }

    /// Whether clients may request changes now (checked again on the
    /// writer).
    fn admit(&self) -> Result<(), AppError> {
        let (allowed, pending) = self.read(|p| {
            (
                p.permissions.requests,
                p.requests
                    .iter()
                    .filter(|r| r.status == api::RequestStatus::Pending)
                    .count(),
            )
        });
        if !allowed {
            return Err(AppError::new(
                Code::RequestsOff,
                "change requests are off in the application",
            ));
        }
        if pending >= MAX_PENDING_REQUESTS {
            return Err(AppError::new(
                Code::RequestsFull,
                "too many change requests are waiting",
            ));
        }
        Ok(())
    }

    /// Stops the threads, cancelling a running calculation, and keeps a
    /// recovery snapshot of unsaved changes. Idempotent; the host calls it
    /// on exit.
    pub fn stop(&self) {
        self.shared.latest.store(u64::MAX, Ordering::SeqCst);
        let _ = self.tx.send(Msg::Stop);
        let threads = match self.threads.lock() {
            Ok(mut t) => std::mem::take(&mut *t),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        };
        for t in threads {
            let _ = t.join();
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The worker: calculates the latest job; older jobs are skipped and a
/// running one is cancelled by a newer version.
fn worker(jobs: &Receiver<Job>, tx: &Sender<Msg>, shared: &Shared) {
    while let Ok(mut job) = jobs.recv() {
        while let Ok(newer) = jobs.try_recv() {
            job = newer;
        }
        let version = job.version;
        let cancel = || shared.latest.load(Ordering::SeqCst) != version;
        if cancel() {
            continue;
        }
        let span = tracing::info_span!("calc", correlation = job.correlation, version);
        let _in = span.enter();
        let started = Instant::now();
        let run = || {
            session::calculate(&job.state, &job.data, &cancel).and_then(|r| {
                let doc = loom_pipeline::document(&r)?;
                Ok((Arc::new(r), Arc::new(doc)))
            })
        };
        // A panic in the engine fails this calculation, not the worker.
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|_| {
                Err(AppError::layer(
                    "CALC_PANIC",
                    "the calculation stopped unexpectedly",
                ))
            });
        let ms = started.elapsed().as_millis();
        match &result {
            Ok((r, _)) => {
                tracing::info!(target: "loom::calc", state = r.state.name(), ms, "calculated");
            }
            Err(e) if e.code == "CALC_CANCELLED" => {
                tracing::info!(target: "loom::calc", ms, "cancelled");
            }
            Err(e) => tracing::warn!(target: "loom::calc", error = %e.code, ms, "failed"),
        }
        if tx.send(Msg::CalcDone { version, result }).is_err() {
            return;
        }
    }
}

#[derive(Default)]
struct CalcTrack {
    /// The version the latest calculation is for.
    asked: u64,
    /// The latest finished result.
    done: Option<(u64, Arc<CalcResult>, Arc<String>)>,
    /// Why the latest calculation could not run or failed.
    error: Option<AppError>,
}

struct Writer {
    session: Session,
    /// The installed and loaded bundles (B-1).
    library: Library,
    shared: Arc<Shared>,
    sink: Sink,
    jobs: Sender<Job>,
    recovery: Recovery,
    recent: Recent,
    autosave: Duration,
    autosave_due: Option<Instant>,
    calc: CalcTrack,
    requests: Vec<api::ChangeRequest>,
    permissions: api::McpPermissions,
    endpoint: Option<(String, String)>,
    /// The recent list and recovery snapshots, read when they can change
    /// (not on every publish).
    recent_list: Vec<api::RecentEntry>,
    recovery_list: Vec<api::RecoveryEntry>,
    /// The build generation each request's preview was made against.
    request_generation: std::collections::BTreeMap<u64, u64>,
    /// The generation the pending requests were last checked against.
    generation: u64,
}

impl Writer {
    fn run(mut self, rx: &Receiver<Msg>) {
        loop {
            let msg = match self.autosave_due {
                Some(due) => match rx.recv_timeout(due.saturating_duration_since(Instant::now())) {
                    Ok(m) => m,
                    Err(RecvTimeoutError::Timeout) => {
                        self.write_recovery();
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                },
                None => match rx.recv() {
                    Ok(m) => m,
                    Err(_) => break,
                },
            };
            match msg {
                Msg::Command {
                    command,
                    correlation,
                    reply,
                } => {
                    let span = tracing::info_span!("command", correlation);
                    let _in = span.enter();
                    let started = Instant::now();
                    let name = command.name();
                    let file = match &command {
                        HostCommand::ImportFile(p) | HostCommand::Open(p) => file_digest(p),
                        _ => None,
                    };
                    let text = match &command {
                        HostCommand::Api(api::Command::ImportText { text }) => {
                            Some(crate::logs::digest(text.as_bytes()))
                        }
                        _ => None,
                    };
                    let before = self.session.version();
                    let result = self.command(command);
                    let ms = started.elapsed().as_millis();
                    let (ext, size, sha) = file
                        .map(|(e, n, s)| (Some(e), Some(n), Some(s)))
                        .or_else(|| text.map(|(n, s)| (None, Some(n), Some(s))))
                        .unwrap_or_default();
                    match &result {
                        Ok(_) => tracing::info!(
                            target: "loom::command",
                            command = %name,
                            outcome = "ok",
                            version = self.session.version(),
                            ext,
                            size,
                            sha256 = sha,
                            ms,
                        ),
                        Err(e) => tracing::warn!(
                            target: "loom::command",
                            command = %name,
                            error = %e.code,
                            ext,
                            size,
                            sha256 = sha,
                            ms,
                        ),
                    }
                    match &result {
                        Ok(_) => {
                            if self.session.version() != before {
                                self.changed(Some(correlation));
                            }
                        }
                        Err(e) => {
                            self.emit(Some(correlation), api::EventKind::Failed { error: e.dto() })
                        }
                    }
                    let _ = reply.send(result);
                }
                Msg::CalcDone { version, result } => self.calc_done(version, result),
                Msg::Request {
                    request,
                    generation,
                    reply,
                } => {
                    let result = self.request(*request, generation);
                    let _ = reply.send(result);
                }
                Msg::Permissions(mut p, done) => {
                    // The standing grant lives only while the server runs.
                    if self.endpoint.is_none() {
                        p.standing_grant = false;
                    }
                    self.permissions = p;
                    self.publish();
                    let _ = done.send(());
                    self.emit(None, api::EventKind::McpChanged);
                }
                Msg::Endpoint(e, done) => {
                    // A start or a stop ends any standing grant.
                    self.permissions.standing_grant = false;
                    self.endpoint = e;
                    self.publish();
                    let _ = done.send(());
                    self.emit(None, api::EventKind::McpChanged);
                }
                Msg::Stop => break,
            }
        }
        // Unsaved changes stay recoverable; otherwise nothing is left (the
        // lock file goes too).
        if self.session.state().is_some() && self.session.dirty() {
            self.write_recovery();
        } else {
            self.recovery.clear();
            self.recovery.release();
        }
    }

    fn emit(&self, correlation: Option<u64>, kind: api::EventKind) {
        (self.sink)(api::Event {
            version: self.session.version(),
            correlation,
            kind,
        });
    }

    fn calc_status(&self) -> api::CalcStatus {
        let done_version = self.calc.done.as_ref().map_or(0, |(v, _, _)| *v);
        let result_state = self
            .calc
            .done
            .as_ref()
            .map(|(_, r, _)| r.state.name().to_owned());
        let state = if self.session.state().is_none() {
            api::CalcState::Idle
        } else if self.calc.error.is_some() {
            api::CalcState::Failed
        } else if done_version == self.session.version() {
            api::CalcState::Done
        } else {
            api::CalcState::Running
        };
        api::CalcStatus {
            state,
            version: done_version,
            result_state,
            error: self.calc.error.as_ref().map(AppError::dto),
        }
    }

    fn publish(&self) {
        let snapshot = self.session.snapshot(self.calc_status());
        let published = Published {
            snapshot,
            state: self.session.state().cloned(),
            data: self.session.data().cloned(),
            calc: self.calc.done.clone(),
            requests: self.requests.clone(),
            permissions: self.permissions.clone(),
            endpoint: self.endpoint.clone(),
            recent: self.recent_list.clone(),
            recovery: self.recovery_list.clone(),
            generation: self.session.generation(),
        };
        match self.shared.published.write() {
            Ok(mut p) => *p = published,
            Err(poisoned) => *poisoned.into_inner() = published,
        }
    }

    /// Reads the recent list and the recovery snapshots again.
    fn refresh_lists(&mut self) {
        self.recent_list = self
            .recent
            .entries(self.session.store())
            .into_iter()
            .map(|e| {
                let s = e.summary.clone().unwrap_or_default();
                api::RecentEntry {
                    id: e.id(),
                    path: e.path,
                    name: e.name,
                    at: e.at,
                    class: s.class,
                    ascendancy: s.ascendancy,
                    level: s.level,
                    main_skill: s.skill,
                }
            })
            .collect();
        self.recovery_list = self.read_recovery();
    }

    fn read_recovery(&self) -> Vec<api::RecoveryEntry> {
        self.recovery
            .list(self.session.store())
            .into_iter()
            .map(|f| api::RecoveryEntry {
                id: f.id,
                saved_at: f.header.saved_at,
                name: f.header.name,
                path: f.header.path,
                origin: f.header.origin,
            })
            .collect()
    }

    /// After a change: a new calculation, the projection, the event, and
    /// autosave.
    fn changed(&mut self, correlation: Option<u64>) {
        let version = self.session.version();
        self.shared.latest.store(version, Ordering::SeqCst);
        self.calc.asked = version;
        self.calc.error = None;
        let mut started = false;
        if self.session.state().is_some() {
            match self.session.calc_input() {
                Ok((state, data)) => {
                    started = self
                        .jobs
                        .send(Job {
                            version,
                            correlation,
                            state,
                            data,
                        })
                        .is_ok();
                    if !started {
                        self.calc.error = Some(stopped());
                    }
                }
                Err(e) => self.calc.error = Some(e),
            }
        } else {
            self.calc.done = None;
        }
        // Another build (or none): pending requests were made for the old
        // one, so they fail as stale.
        if self.session.generation() != self.generation {
            self.generation = self.session.generation();
            for r in &mut self.requests {
                if r.status == api::RequestStatus::Pending {
                    r.status = api::RequestStatus::Failed;
                    r.error =
                        Some(AppError::new(Code::StaleEdit, "another build was opened").dto());
                }
            }
        }
        if self.session.dirty() && self.session.state().is_some() {
            self.autosave_due
                .get_or_insert_with(|| Instant::now() + self.autosave);
        } else {
            self.autosave_due = None;
        }
        self.publish();
        self.emit(correlation, api::EventKind::BuildChanged);
        if started {
            self.emit(None, api::EventKind::CalcStarted);
        } else if let Some(e) = &self.calc.error {
            self.emit(None, api::EventKind::CalcFailed { error: e.dto() });
        }
    }

    fn calc_done(
        &mut self,
        version: u64,
        result: Result<(Arc<CalcResult>, Arc<String>), AppError>,
    ) {
        if version != self.session.version() {
            // Stale: a newer version is being calculated.
            return;
        }
        let kind = match result {
            Ok((r, doc)) => {
                let state = r.state.name().to_owned();
                self.calc.done = Some((version, r, doc));
                self.calc.error = None;
                api::EventKind::CalcFinished { state }
            }
            Err(e) => {
                let error = e.dto();
                self.calc.error = Some(e);
                api::EventKind::CalcFailed { error }
            }
        };
        self.publish();
        self.emit(None, kind);
    }

    fn write_recovery(&mut self) {
        self.autosave_due = None;
        if let Some(state) = self.session.state() {
            let header = Header {
                saved_at: now(),
                name: self.session.name(),
                path: self.session.path().map(|p| p.display().to_string()),
                origin: self.session.origin().map(|p| p.display().to_string()),
            };
            // A failed autosave is retried with the next change. The
            // history first: a snapshot is never newer than its history.
            let steps = self.session.undo_steps();
            let entries: Vec<serde_json::Value> = steps
                .iter()
                .map(|s| serde_json::to_value(&s.entry).unwrap_or(serde_json::Value::Null))
                .collect();
            let refs: Vec<StepRef<'_>> = steps
                .iter()
                .zip(&entries)
                .map(|(s, entry)| StepRef {
                    id: s.id,
                    entry,
                    build: &s.state.build,
                })
                .collect();
            let history = self.recovery.write_history(self.session.store(), &refs);
            let snapshot = self
                .recovery
                .write(self.session.store(), &header, &state.build);
            match (history, snapshot) {
                (Ok(_), Ok(_)) => {
                    tracing::info!(target: "loom::recovery", steps = refs.len(), "written");
                }
                _ => {
                    tracing::warn!(target: "loom::recovery", "not written; retried with the next change")
                }
            }
        }
    }

    fn touch_recent(&self, path: &Path) {
        let _ = self.recent.touch(
            self.session.store(),
            RecentFileEntry {
                path: path.display().to_string(),
                name: self.session.name(),
                at: now(),
                summary: self.session.state().map(|s| {
                    crate::view::recent_summary(&s.build, self.session.data().map(|d| d.bundle))
                }),
            },
        );
    }

    fn command(&mut self, command: HostCommand) -> Result<api::Outcome, AppError> {
        let done = |v: u64| api::Outcome::Done { version: v };
        match command {
            HostCommand::ImportFile(path) => self.session.import_file(&path).map(done),
            HostCommand::Open(path) => {
                let v = self.session.open(&path)?;
                self.recovery.clear();
                self.touch_recent(&path);
                self.refresh_lists();
                Ok(done(v))
            }
            HostCommand::SaveAs(path) => {
                self.session.save_as(&path)?;
                self.after_save(&path);
                Ok(done(self.session.version()))
            }
            HostCommand::ExportXml(path) => {
                self.session.export_xml(&path)?;
                Ok(done(self.session.version()))
            }
            HostCommand::SetBundle(dir) => {
                let data = self.library.use_dir(&dir);
                self.session.set_loaded(data);
                Ok(done(self.session.version()))
            }
            // App::execute writes it without the writer.
            HostCommand::ExportDiagnostics(_) => Ok(done(self.session.version())),
            HostCommand::Api(c) => self.api(c),
        }
    }

    fn after_save(&mut self, path: &Path) {
        self.recovery.clear();
        self.autosave_due = None;
        self.touch_recent(path);
        self.refresh_lists();
        // A save changes no build, so no recalculation: the projection
        // (clean, its path) and the event only.
        self.publish();
        self.emit(
            None,
            api::EventKind::Saved {
                path: path.display().to_string(),
            },
        );
    }

    fn api(&mut self, command: api::Command) -> Result<api::Outcome, AppError> {
        let done = |v: u64| api::Outcome::Done { version: v };
        match command {
            api::Command::ImportText { text } => self.session.import_text(&text, None).map(done),
            api::Command::NewBuild => self.session.new_build().map(done),
            api::Command::UseBundle { id } => {
                // An unknown ID or a bundle that does not load leaves the
                // loaded data as it was.
                let data = self.library.use_id(&id)?;
                self.session
                    .set_installed(self.library.installed().to_vec());
                self.session.set_loaded(Ok(data));
                Ok(done(self.session.version()))
            }
            api::Command::EditItem(e) => self.session.edit(&e, None).map(done),
            api::Command::UseItemSet { id } => Ok(self
                .session
                .use_item_set(id)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::UseLoadout { spec } => Ok(self
                .session
                .use_loadout(spec)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::UseSkillSet { id } => Ok(self
                .session
                .use_skill_set(id)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::EditGem {
                group,
                gem,
                level,
                quality,
                enabled,
            } => Ok(self
                .session
                .edit_gem(group, gem, level, quality, enabled)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::EditSocketGroup {
                group,
                enabled,
                main,
                main_gem,
                slot,
                label,
            } => Ok(self
                .session
                .edit_socket_group(
                    group,
                    enabled,
                    main,
                    main_gem,
                    crate::GroupPlace { slot, label },
                )?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::AddGem { group, gem, at } => Ok(self
                .session
                .add_gem(group, &gem, at)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::RemoveGem { group, gem } => Ok(self
                .session
                .remove_gem(group, gem)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::AddSocketGroup { slot, label } => Ok(self
                .session
                .add_socket_group(slot, label)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::RemoveSocketGroup { group } => Ok(self
                .session
                .remove_socket_group(group)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::UseWeaponSet { second } => Ok(self
                .session
                .use_weapon_set(second)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::SetLeague { league } => Ok(self
                .session
                .set_league(&league)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::UseConfigSet { id } => Ok(self
                .session
                .use_config_set(id)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::SetConfigInput { name, value } => Ok(self
                .session
                .set_config_input(&name, &value)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::RemoveConfigInput { name } => Ok(self
                .session
                .remove_config_input(&name)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::EditCustomBlock {
                block,
                enabled,
                text,
            } => Ok(self
                .session
                .edit_custom_block(block, enabled, text.as_deref())?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::AddCustomBlock { text } => Ok(self
                .session
                .add_custom_block(&text)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::RemoveCustomBlock { block } => Ok(self
                .session
                .remove_custom_block(block)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::AllocateNode { spec, node } => Ok(self
                .session
                .edit_tree(spec, crate::tree::Edit::Allocate(node))?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::DeallocateNode { spec, node } => Ok(self
                .session
                .edit_tree(spec, crate::tree::Edit::Deallocate(node))?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::SetMasteryEffect { spec, node, effect } => Ok(self
                .session
                .edit_tree(spec, crate::tree::Edit::Mastery { node, effect })?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::SelectClass { spec, class } => Ok(self
                .session
                .edit_tree(spec, crate::tree::Edit::Class(class))?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::SelectAscendancy { spec, ascendancy } => Ok(self
                .session
                .edit_tree(spec, crate::tree::Edit::Ascendancy(ascendancy))?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::ResetTree { spec } => Ok(self
                .session
                .edit_tree(spec, crate::tree::Edit::Reset)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::SocketJewel { spec, node, item } => Ok(self
                .session
                .socket_jewel(spec, node, item)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::NewSpec { title, copy } => Ok(self
                .session
                .new_spec(&title, copy)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::RenameSpec { spec, title } => Ok(self
                .session
                .rename_spec(spec, &title)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::DeleteSpec { spec } => Ok(self
                .session
                .delete_spec(spec)?
                .map_or(api::Outcome::Nothing, done)),
            api::Command::Undo => Ok(self.session.undo().map_or(api::Outcome::Nothing, done)),
            api::Command::Redo => Ok(self.session.redo().map_or(api::Outcome::Nothing, done)),
            api::Command::Save => {
                let path = self.session.save()?;
                self.after_save(&path);
                Ok(done(self.session.version()))
            }
            api::Command::OpenRecent { id } => {
                let store = self.session.store().clone();
                let entry = self
                    .recent
                    .find(&store, &id)
                    .ok_or_else(|| AppError::new(Code::NotFound, "no such recent build"))?;
                let path = PathBuf::from(&entry.path);
                match self.session.open(&path) {
                    Ok(v) => {
                        self.recovery.clear();
                        self.touch_recent(&path);
                        self.refresh_lists();
                        Ok(done(v))
                    }
                    Err(e) => {
                        if e.code == "PERSIST_READ" {
                            let _ = self.recent.remove(&store, &id);
                            self.refresh_lists();
                            self.publish();
                        }
                        Err(e)
                    }
                }
            }
            api::Command::Restore { id } => {
                if !self.read_recovery().iter().any(|r| r.id == id) {
                    return Err(AppError::new(Code::NotFound, "no such recovery snapshot"));
                }
                let (header, build) = self
                    .recovery
                    .read(self.session.store(), &id)
                    .map_err(|e| AppError::layer(e.code(), &e))?;
                let history = self.recovery.read_history(self.session.store(), &id);
                let v = self.session.open_build(
                    build,
                    header.path.map(PathBuf::from),
                    header.origin.map(PathBuf::from),
                    true,
                );
                // The undo history comes back too (UX-19): the newest steps
                // whose change still reads.
                let mut steps: Vec<_> = history
                    .into_iter()
                    .rev()
                    .map_while(|s| {
                        serde_json::from_value::<api::HistoryEntry>(s.entry)
                            .ok()
                            .map(|e| (e, s.build))
                    })
                    .collect();
                steps.reverse();
                self.session.restore_history(steps);
                let _ = self.recovery.discard(&id);
                self.refresh_lists();
                Ok(done(v))
            }
            api::Command::ForgetRecent { id } => {
                let store = self.session.store().clone();
                if self.recent.find(&store, &id).is_none() {
                    return Err(AppError::new(Code::NotFound, "no such recent build"));
                }
                self.recent.remove(&store, &id)?;
                self.refresh_lists();
                self.publish();
                Ok(api::Outcome::Nothing)
            }
            api::Command::Discard { id } => {
                self.recovery.discard(&id)?;
                self.refresh_lists();
                self.publish();
                Ok(api::Outcome::Nothing)
            }
            api::Command::Approve { id } => {
                let index = self.pending(id)?;
                let result = if self.request_generation.get(&id) == Some(&self.session.generation())
                {
                    self.apply_request(index)
                } else {
                    Err(AppError::new(
                        Code::StaleEdit,
                        "the request is for another build",
                    ))
                };
                self.settle(index, &result);
                self.publish();
                self.emit(None, api::EventKind::McpChanged);
                result.map(done)
            }
            api::Command::Decline { id } => {
                let index = self.pending(id)?;
                if let Some(r) = self.requests.get_mut(index) {
                    r.status = api::RequestStatus::Declined;
                }
                self.publish();
                self.emit(None, api::EventKind::McpChanged);
                Ok(api::Outcome::Nothing)
            }
            api::Command::ExportShareCode => Ok(api::Outcome::Text {
                version: self.session.version(),
                text: self.session.share_code()?,
            }),
            api::Command::Close => {
                self.session.close();
                self.recovery.clear();
                self.refresh_lists();
                Ok(done(self.session.version()))
            }
        }
    }

    fn pending(&self, id: u64) -> Result<usize, AppError> {
        self.requests
            .iter()
            .position(|r| r.id == id && r.status == api::RequestStatus::Pending)
            .ok_or_else(|| AppError::new(Code::Request, format!("no pending request {id}")))
    }

    /// Applies request `index`: its item edit or its tree change. A tree
    /// change that changes nothing is applied at the current version.
    fn apply_request(&mut self, index: usize) -> Result<u64, AppError> {
        let request = self
            .requests
            .get(index)
            .cloned()
            .ok_or_else(|| AppError::new(Code::Request, "no such request"))?;
        match (request.edit, request.tree) {
            (Some(e), _) => self.session.edit(&e, Some(request.id)),
            (None, Some(change)) => {
                let (spec, edit) = crate::tree::change_edit(&change);
                let applied = self.session.edit_tree_for(spec, edit, Some(request.id))?;
                Ok(applied.unwrap_or_else(|| self.session.version()))
            }
            (None, None) => Err(AppError::new(Code::Request, "the request has no change")),
        }
    }

    fn settle(&mut self, index: usize, result: &Result<u64, AppError>) {
        if let Some(r) = self.requests.get_mut(index) {
            match result {
                Ok(_) => r.status = api::RequestStatus::Applied,
                Err(e) => {
                    r.status = api::RequestStatus::Failed;
                    r.error = Some(e.dto());
                }
            }
        }
    }

    fn request(
        &mut self,
        request: api::ChangeRequest,
        generation: u64,
    ) -> Result<api::ChangeRequest, AppError> {
        if generation != self.session.generation() {
            return Err(AppError::new(
                Code::StaleEdit,
                "another build was opened meanwhile",
            ));
        }
        if !self.permissions.requests {
            return Err(AppError::new(
                Code::RequestsOff,
                "change requests are off in the application",
            ));
        }
        let pending = self
            .requests
            .iter()
            .filter(|r| r.status == api::RequestStatus::Pending)
            .count();
        if pending >= MAX_PENDING_REQUESTS {
            return Err(AppError::new(
                Code::RequestsFull,
                "too many change requests are waiting",
            ));
        }
        let id = request.id;
        self.request_generation.insert(id, generation);
        self.requests.push(request);
        let index = self.requests.len() - 1;
        if self.permissions.standing_grant {
            let before = self.session.version();
            let result = self.apply_request(index);
            self.settle(index, &result);
            if self.session.version() != before {
                self.changed(None);
            }
        }
        // Keep the list bounded: settled requests beyond 100 are dropped.
        while self.requests.len() > 100 {
            if let Some(i) = self
                .requests
                .iter()
                .position(|r| r.status != api::RequestStatus::Pending)
            {
                self.requests.remove(i);
            } else {
                break;
            }
        }
        self.publish();
        self.emit(None, api::EventKind::McpChanged);
        self.requests
            .iter()
            .find(|r| r.id == id)
            .cloned()
            .ok_or_else(|| AppError::new(Code::Request, "the request was dropped"))
    }
}
