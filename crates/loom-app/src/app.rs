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

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use loom_calc::CalcResult;
use loom_persistence::Store;
use loom_persistence::recent::{Entry as RecentFileEntry, Recent};
use loom_persistence::recovery::{Header, Recovery};
use loom_serialization::dto::app::v1 as api;

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
    /// The bundle folder to load at start, if any.
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
        reply: Sender<Result<api::ChangeRequest, AppError>>,
    },
    Permissions(api::McpPermissions, Sender<()>),
    Endpoint(Option<(String, String)>, Sender<()>),
    Stop,
}

struct Job {
    version: u64,
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
    writer: Option<JoinHandle<()>>,
    worker: Option<JoinHandle<()>>,
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
        let mut session = Session::new(store);
        if let Some(dir) = &config.bundle {
            session.set_data(GameData::load(dir));
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
        };
        writer.publish();
        let writer = std::thread::Builder::new()
            .name("loom-app".to_owned())
            .spawn(move || writer.run(&rx))
            .map_err(|e| AppError::layer("APP_THREAD", e))?;
        Ok(Self {
            tx,
            shared,
            writer: Some(writer),
            worker: Some(worker),
        })
    }

    /// Runs a command and waits for its outcome.
    ///
    /// # Errors
    /// The command's error; `APP_STOPPED` after [`App::stop`].
    pub fn execute(&self, command: HostCommand) -> Result<api::Outcome, AppError> {
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

    /// One item of the open build.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_EDIT_ITEM`.
    pub fn item(&self, id: u32) -> Result<api::ItemView, AppError> {
        let state = self
            .read(|p| p.state.clone())
            .ok_or_else(|| AppError::new(Code::NoBuild, "no build is open"))?;
        let item = state
            .build
            .items
            .item(loom_domain::ItemId(id))
            .ok_or_else(|| AppError::new(Code::EditItem, format!("no item {id}")))?;
        Ok(crate::view::item_view(&state.build, item))
    }

    /// An item edit's effect, calculated on the caller's thread with the
    /// latest snapshot (never on the writer; decision 6).
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, the edit's and calculation's codes.
    pub fn preview(&self, edit: &api::ItemEdit) -> Result<api::ItemEditPreview, AppError> {
        let (state, data, calc) = self.read(|p| (p.state.clone(), p.data.clone(), p.calc.clone()));
        let state = state.ok_or_else(|| AppError::new(Code::NoBuild, "no build is open"))?;
        let data = data.ok_or_else(|| AppError::new(Code::NoData, "no game data is loaded"))?;
        let version = self.read(|p| p.snapshot.version);
        let before = calc
            .as_ref()
            .filter(|(v, _, _)| *v == version)
            .map(|(_, r, _)| r.as_ref());
        session::preview(&state, &data, edit, before, &|| false)
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
        let preview = self.preview(&edit)?;
        let request = api::ChangeRequest {
            id: self.shared.requests.fetch_add(1, Ordering::Relaxed) + 1,
            client: client.chars().take(64).collect(),
            edit,
            note: note.map(|n| n.chars().take(500).collect()),
            preview: Some(preview),
            status: api::RequestStatus::Pending,
            error: None,
        };
        let (reply, answer) = mpsc::channel();
        self.tx
            .send(Msg::Request {
                request: Box::new(request),
                reply,
            })
            .map_err(|_| stopped())?;
        answer.recv().map_err(|_| stopped())?
    }

    /// Stops the threads (keeping a recovery snapshot of unsaved changes).
    pub fn stop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(w) = self.writer.take() {
            let _ = w.join();
        }
        if let Some(w) = self.worker.take() {
            let _ = w.join();
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
        let result = session::calculate(&job.state, &job.data, &cancel).and_then(|r| {
            let doc = loom_pipeline::document(&r)?;
            Ok((Arc::new(r), Arc::new(doc)))
        });
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
                    let before = self.session.version();
                    let result = self.command(command);
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
                Msg::Request { request, reply } => {
                    let result = self.request(*request);
                    let _ = reply.send(result);
                }
                Msg::Permissions(p, done) => {
                    self.permissions = p;
                    self.publish();
                    let _ = done.send(());
                    self.emit(None, api::EventKind::McpChanged);
                }
                Msg::Endpoint(e, done) => {
                    if e.is_none() {
                        self.permissions.standing_grant = false;
                    }
                    self.endpoint = e;
                    self.publish();
                    let _ = done.send(());
                    self.emit(None, api::EventKind::McpChanged);
                }
                Msg::Stop => break,
            }
        }
        // Unsaved changes stay recoverable; otherwise nothing is left.
        if self.session.state().is_some() && self.session.dirty() {
            self.write_recovery();
        } else {
            self.recovery.clear();
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
            recent: self
                .recent
                .entries(self.session.store())
                .into_iter()
                .map(|e| api::RecentEntry {
                    id: e.id(),
                    path: e.path,
                    name: e.name,
                    at: e.at,
                })
                .collect(),
            recovery: self.recovery_list(),
        };
        match self.shared.published.write() {
            Ok(mut p) => *p = published,
            Err(poisoned) => *poisoned.into_inner() = published,
        }
    }

    fn recovery_list(&self) -> Vec<api::RecoveryEntry> {
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
                            state,
                            data,
                        })
                        .is_ok();
                }
                Err(e) => self.calc.error = Some(e),
            }
        } else {
            self.calc.done = None;
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
            // A failed autosave is retried with the next change.
            let _ = self
                .recovery
                .write(self.session.store(), &header, &state.build);
        }
    }

    fn touch_recent(&self, path: &Path) {
        let _ = self.recent.touch(
            self.session.store(),
            RecentFileEntry {
                path: path.display().to_string(),
                name: self.session.name(),
                at: now(),
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
                self.session.set_data(GameData::load(&dir));
                Ok(done(self.session.version()))
            }
            HostCommand::Api(c) => self.api(c),
        }
    }

    fn after_save(&mut self, path: &Path) {
        self.recovery.clear();
        self.autosave_due = None;
        self.touch_recent(path);
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
            api::Command::EditItem(e) => self.session.edit(&e, None).map(done),
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
                        Ok(done(v))
                    }
                    Err(e) => {
                        if e.code == "PERSIST_READ" {
                            let _ = self.recent.remove(&store, &id);
                            self.publish();
                        }
                        Err(e)
                    }
                }
            }
            api::Command::Restore { id } => {
                if !self.recovery_list().iter().any(|r| r.id == id) {
                    return Err(AppError::new(Code::NotFound, "no such recovery snapshot"));
                }
                let (header, build) = self
                    .recovery
                    .read(self.session.store(), &id)
                    .map_err(|e| AppError::layer(e.code(), &e))?;
                let v = self.session.open_build(
                    build,
                    header.path.map(PathBuf::from),
                    header.origin.map(PathBuf::from),
                    true,
                );
                let _ = self.recovery.discard(&id);
                Ok(done(v))
            }
            api::Command::Discard { id } => {
                self.recovery.discard(&id)?;
                self.publish();
                Ok(api::Outcome::Nothing)
            }
            api::Command::Approve { id } => {
                let index = self.pending(id)?;
                let edit = self.requests.get(index).map(|r| r.edit.clone());
                let result = match edit {
                    Some(e) => self.session.edit(&e, Some(id)),
                    None => Err(AppError::new(Code::Request, "no such request")),
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

    fn request(&mut self, request: api::ChangeRequest) -> Result<api::ChangeRequest, AppError> {
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
        self.requests.push(request);
        let index = self.requests.len() - 1;
        if self.permissions.standing_grant {
            let edit = self.requests.get(index).map(|r| r.edit.clone());
            let before = self.session.version();
            let result = match edit {
                Some(e) => self.session.edit(&e, Some(id)),
                None => Err(AppError::new(Code::Request, "no such request")),
            };
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
