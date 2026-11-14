//! The synchronous application core (Phase 4 plan, decision 2): the open
//! build as an immutable state, its version, history, and file paths.
//! [`crate::App`] owns one on its writer thread; tests drive it directly.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use loom_calc::CalcResult;
use loom_domain::{Build, ItemId};
use loom_persistence::{Overwrite, SaveOptions, Store};
use loom_pipeline::{RangeEnds, Reresolved};
use loom_serialization::dto::app::v1 as api;
use loom_serialization::legacy::{pob_xml, share_code};

use crate::data::GameData;
use crate::edit;
use crate::error::{AppError, Code};
use crate::view;

/// The build with the range ends its calculation needs; immutable, shared
/// with calculation jobs and kept whole in the history (finding 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// The build.
    pub build: Build,
    /// The exact ends of the ranges its resolution read.
    pub ends: RangeEnds,
}

#[derive(Debug, Clone)]
struct Change {
    state: Arc<State>,
    entry: api::HistoryEntry,
}

/// How many changes undo keeps.
pub const HISTORY_LIMIT: usize = 100;

/// The application core.
#[derive(Debug)]
pub struct Session {
    store: Store,
    data: Option<Arc<GameData>>,
    data_error: Option<AppError>,
    current: Option<Arc<State>>,
    version: u64,
    undo: Vec<Change>,
    redo: Vec<Change>,
    dirty: bool,
    path: Option<PathBuf>,
    origin: Option<PathBuf>,
    notices: Vec<AppError>,
    /// Bumped when another build is opened or the build is closed.
    generation: u64,
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl Session {
    /// A session with no build and no data.
    #[must_use]
    pub fn new(store: Store) -> Self {
        Self {
            store,
            data: None,
            data_error: None,
            current: None,
            version: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            path: None,
            origin: None,
            notices: Vec::new(),
            generation: 0,
        }
    }

    /// Which build is open: it changes when another build is opened (an
    /// import, an open, a restore) or the build is closed, and never with
    /// an edit, undo, or save.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// The store.
    #[must_use]
    pub const fn store(&self) -> &Store {
        &self.store
    }

    /// Sets the game data (or why it did not load). The open build is
    /// kept; its next calculation uses the new data.
    pub fn set_data(&mut self, data: Result<GameData, AppError>) {
        match data {
            Ok(d) => {
                self.data = Some(Arc::new(d));
                self.data_error = None;
            }
            Err(e) => {
                self.data = None;
                self.data_error = Some(e);
            }
        }
        self.version += 1;
    }

    /// The game data, if loaded.
    #[must_use]
    pub fn data(&self) -> Option<&Arc<GameData>> {
        self.data.as_ref()
    }

    /// The current version.
    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// The open build's state, if a build is open.
    #[must_use]
    pub fn state(&self) -> Option<&Arc<State>> {
        self.current.as_ref()
    }

    /// Whether the build changed since it was opened or saved.
    #[must_use]
    pub const fn dirty(&self) -> bool {
        self.dirty
    }

    /// Where the build is saved, if it is.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file the build was imported from, if any.
    #[must_use]
    pub fn origin(&self) -> Option<&Path> {
        self.origin.as_deref()
    }

    /// The build's display name: the saved file's stem, else the imported
    /// file's, else empty (the UI names it).
    #[must_use]
    pub fn name(&self) -> String {
        self.path
            .as_deref()
            .or(self.origin.as_deref())
            .map(stem)
            .unwrap_or_default()
    }

    fn need_data(&self) -> Result<Arc<GameData>, AppError> {
        self.data.clone().ok_or_else(|| {
            self.data_error
                .clone()
                .unwrap_or_else(|| AppError::new(Code::NoData, "no game data is loaded"))
        })
    }

    fn need_state(&self) -> Result<Arc<State>, AppError> {
        self.current
            .clone()
            .ok_or_else(|| AppError::new(Code::NoBuild, "no build is open"))
    }

    fn replace(
        &mut self,
        state: State,
        path: Option<PathBuf>,
        origin: Option<PathBuf>,
        dirty: bool,
    ) {
        self.current = Some(Arc::new(state));
        self.undo.clear();
        self.redo.clear();
        self.path = path;
        self.origin = origin;
        self.dirty = dirty;
        self.version += 1;
        self.generation += 1;
    }

    /// Imports a PoB share code or XML text (atomic: on failure nothing
    /// changes). `origin` is the file it came from, if any.
    ///
    /// # Errors
    /// `APP_NO_DATA`, the import adapter's and the resolver's codes.
    pub fn import_text(&mut self, text: &str, origin: Option<PathBuf>) -> Result<u64, AppError> {
        let data = self.need_data()?;
        let mut build = loom_pipeline::read_pob(text)?;
        let ends = loom_pipeline::resolve(data.resolver, data.data, &mut build)?;
        self.notices.clear();
        self.replace(State { build, ends }, None, origin, true);
        Ok(self.version)
    }

    /// Imports a PoB XML file.
    ///
    /// # Errors
    /// `APP_READ`, then as [`Session::import_text`].
    pub fn import_file(&mut self, path: &Path) -> Result<u64, AppError> {
        let bytes = std::fs::read(path)
            .map_err(|e| AppError::new(Code::Read, format!("{}: {e}", path.display())))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| AppError::new(Code::Read, format!("{}: not UTF-8", path.display())))?;
        self.import_text(&text, Some(path.to_owned()))
    }

    /// Opens a build read with its resolutions (a saved document or a
    /// recovery snapshot). Never needs data (decision 7): with data of the
    /// build's version, a clone is resolved again to recover the range
    /// ends; if it resolves differently the stored resolutions stand.
    pub fn open_build(
        &mut self,
        build: Build,
        path: Option<PathBuf>,
        origin: Option<PathBuf>,
        dirty: bool,
    ) -> u64 {
        self.notices.clear();
        let mut ends = RangeEnds::new();
        if let Some(data) = &self.data
            && build.game() == data.version.game
            && build.data.as_ref() == Some(&data.version)
        {
            match loom_pipeline::reresolve(data.resolver, data.data, &build) {
                Ok(Reresolved::Same(e)) => ends = e,
                Ok(Reresolved::Changed) => self.notices.push(AppError::new(
                    Code::ResolutionChanged,
                    "the stored resolutions differ from this bundle's; they stand",
                )),
                // The checks above match the resolver's own.
                Err(e) => self.notices.push(e.into()),
            }
        }
        self.replace(State { build, ends }, path, origin, dirty);
        self.version
    }

    /// Opens a build document.
    ///
    /// # Errors
    /// The store's codes (`PERSIST_*`, `MIGRATE_*`, `DTO_*`).
    pub fn open(&mut self, path: &Path) -> Result<u64, AppError> {
        let loaded = self.store.load(path)?;
        Ok(self.open_build(loaded.build, Some(path.to_owned()), None, false))
    }

    /// Applies an item edit as one undoable change.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, and [`edit::apply`]'s codes.
    pub fn edit(&mut self, e: &api::ItemEdit, request: Option<u64>) -> Result<u64, AppError> {
        let state = self.need_state()?;
        let data = self.need_data()?;
        let edited = edit::apply(&state.build, &state.ends, e, &data)?;
        let entry = api::HistoryEntry {
            kind: if request.is_some() {
                api::ChangeKind::RequestApplied
            } else {
                api::ChangeKind::ItemEdit
            },
            item: e.item,
            request,
        };
        self.undo.push(Change {
            state,
            entry: entry.clone(),
        });
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.current = Some(Arc::new(State {
            build: edited.build,
            ends: edited.ends,
        }));
        self.dirty = true;
        self.version += 1;
        Ok(self.version)
    }

    /// Reverts the latest change; `None` when there is none.
    pub fn undo(&mut self) -> Option<u64> {
        let change = self.undo.pop()?;
        let current = self.current.replace(change.state)?;
        self.redo.push(Change {
            state: current,
            entry: change.entry,
        });
        self.dirty = true;
        self.version += 1;
        Some(self.version)
    }

    /// Applies the latest reverted change again; `None` when there is none.
    pub fn redo(&mut self) -> Option<u64> {
        let change = self.redo.pop()?;
        let current = self.current.replace(change.state)?;
        self.undo.push(Change {
            state: current,
            entry: change.entry,
        });
        self.dirty = true;
        self.version += 1;
        Some(self.version)
    }

    /// Saves the build where it is saved.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_PATH`, the store's codes.
    pub fn save(&mut self) -> Result<PathBuf, AppError> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| AppError::new(Code::NoPath, "the build has not been saved yet"))?;
        self.save_as(&path)
    }

    /// Saves the build as a build document at `path` (a build document
    /// there is replaced; any other file is not; never the imported file).
    ///
    /// # Errors
    /// `APP_NO_BUILD`, the store's codes.
    pub fn save_as(&mut self, path: &Path) -> Result<PathBuf, AppError> {
        let state = self.need_state()?;
        self.store.save(
            &state.build,
            path,
            &SaveOptions {
                overwrite: Overwrite::BuildDocument,
                imported_from: self.origin.as_deref(),
            },
        )?;
        // The build is unchanged: no new version, so no recalculation.
        self.path = Some(path.to_owned());
        self.dirty = false;
        Ok(path.to_owned())
    }

    /// Writes the build as PoB XML to `path` (replacing a file there the
    /// user chose, never the file the build was imported from).
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_EXPORT_ORIGIN`, the store's write codes.
    pub fn export_xml(&self, path: &Path) -> Result<(), AppError> {
        let state = self.need_state()?;
        if let Some(origin) = &self.origin
            && same_file(origin, path)
        {
            return Err(AppError::new(
                Code::ExportOrigin,
                "the export would write over the imported file",
            ));
        }
        let xml = pob_xml::export(&state.build);
        self.store.write_replacing(path, xml.as_bytes())?;
        Ok(())
    }

    /// The build's PoB share code.
    ///
    /// # Errors
    /// `APP_NO_BUILD`.
    pub fn share_code(&self) -> Result<String, AppError> {
        Ok(share_code::export(&self.need_state()?.build))
    }

    /// Closes the build.
    pub fn close(&mut self) {
        self.current = None;
        self.undo.clear();
        self.redo.clear();
        self.path = None;
        self.origin = None;
        self.dirty = false;
        self.notices.clear();
        self.version += 1;
        self.generation += 1;
    }

    /// What a calculation of the current version needs, or why it cannot
    /// run (decision 7: a stable code, never a crash).
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, `APP_GAME_MISMATCH`, `APP_DATA_VERSION`.
    pub fn calc_input(&self) -> Result<(Arc<State>, Arc<GameData>), AppError> {
        let state = self.need_state()?;
        let data = self.need_data()?;
        check_data(&state.build, &data)?;
        Ok((state, data))
    }

    /// One item's view.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_EDIT_ITEM`.
    pub fn item(&self, id: u32) -> Result<api::ItemView, AppError> {
        let state = self.need_state()?;
        let item = state
            .build
            .items
            .item(ItemId(id))
            .ok_or_else(|| AppError::new(Code::EditItem, format!("no item {id}")))?;
        Ok(view::item_view(&state.build, item))
    }

    /// The snapshot DTO, with the calculation's status.
    #[must_use]
    pub fn snapshot(&self, calc: api::CalcStatus) -> api::Snapshot {
        let entry = |c: Option<&Change>| c.map(|c| c.entry.clone());
        api::Snapshot {
            version: self.version,
            build: self
                .current
                .as_ref()
                .map(|s| view::build_view(&s.build, self.name())),
            calc,
            history: api::HistoryView {
                can_undo: !self.undo.is_empty(),
                can_redo: !self.redo.is_empty(),
                undo: entry(self.undo.last()),
                redo: entry(self.redo.last()),
            },
            dirty: self.dirty,
            path: self.path.as_ref().map(|p| p.display().to_string()),
            origin: self.origin.as_ref().map(|p| p.display().to_string()),
            data: api::DataStatus {
                bundle: self.data.as_ref().map(|d| d.dir.display().to_string()),
                data_version: self.data.as_ref().map(|d| view::data_version(&d.version)),
                tree_version: self
                    .data
                    .as_ref()
                    .and_then(|d| d.tree_version.as_ref())
                    .map(|t| t.as_str().to_owned()),
                error: self.data_error.as_ref().map(AppError::dto),
            },
            notices: self.notices.iter().map(AppError::dto).collect(),
        }
    }
}

/// Whether the data can calculate the build.
///
/// # Errors
/// `APP_GAME_MISMATCH`, `APP_DATA_VERSION`.
pub fn check_data(build: &Build, data: &GameData) -> Result<(), AppError> {
    if build.game() != data.version.game {
        return Err(AppError::new(
            Code::GameMismatch,
            format!(
                "the build is for {} and the data for {}",
                build.game().name(),
                data.version.game.name()
            ),
        ));
    }
    if let Some(v) = &build.data
        && *v != data.version
    {
        return Err(AppError::new(
            Code::DataVersion,
            format!(
                "the build was made with data {:?} and the loaded data is {:?}",
                v.steam_build, data.version.steam_build
            ),
        ));
    }
    Ok(())
}

/// Calculates a state with the data, polling `cancel`.
///
/// # Errors
/// The pipeline's codes (`CALC_*`).
pub fn calculate(
    state: &State,
    data: &GameData,
    cancel: &dyn Fn() -> bool,
) -> Result<CalcResult, AppError> {
    let options = loom_pipeline::options(
        &state.build,
        data.data,
        &state.ends,
        data.tree_version.clone(),
    );
    Ok(loom_pipeline::calculate(
        &state.build,
        data.bundle,
        &options,
        cancel,
    )?)
}

/// An item edit's effect, calculated off the writer thread (decision 6):
/// the edited item and each field before and after. `before` is the
/// current result when it is of `state`, else it is calculated.
///
/// # Errors
/// [`edit::apply`]'s codes; the calculation's.
pub fn preview(
    state: &State,
    data: &GameData,
    e: &api::ItemEdit,
    before: Option<&CalcResult>,
    cancel: &dyn Fn() -> bool,
) -> Result<api::ItemEditPreview, AppError> {
    check_data(&state.build, data)?;
    let edited = edit::apply(&state.build, &state.ends, e, data)?;
    let computed;
    let before = match before {
        Some(b) => b,
        None => {
            computed = calculate(state, data, cancel)?;
            &computed
        }
    };
    let after_state = State {
        build: edited.build,
        ends: edited.ends,
    };
    let after = calculate(&after_state, data, cancel)?;
    let after_fields = view::fields(&after);
    let fields = view::fields(before)
        .into_iter()
        .zip(after_fields)
        .map(|((field, b, sb), (_, a, sa))| api::FieldDelta {
            field,
            before: b,
            after: a,
            state_before: sb,
            state_after: sa,
        })
        .collect();
    let item = after_state
        .build
        .items
        .item(ItemId(e.item))
        .map(|i| view::item_view(&after_state.build, i))
        .ok_or_else(|| AppError::new(Code::EditItem, format!("no item {}", e.item)))?;
    Ok(api::ItemEditPreview { item, fields })
}

/// Whether two paths name the same file (by canonical path when both
/// exist, else by text).
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}
