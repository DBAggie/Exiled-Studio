//! The synchronous application core (Phase 4 plan, decision 2): the open
//! build as an immutable state, its version, history, and file paths.
//! [`crate::App`] owns one on its writer thread; tests drive it directly.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use loom_calc::CalcResult;
use loom_domain::{
    Attribute, Attributes, Build, ConfigInput, ConfigInputKind, ConfigSet, ConfigSetId,
    ConfigValue, CustomModifierBlock, CustomModifierLine, Dashboard, DashboardComponent,
    Dashboards, ItemId, ItemSetId, ItemsEntry, League, LineResolution, Number, Ordered, SkillSetId,
    SlotId, SocketGroup, Sourced,
};
use loom_persistence::{Overwrite, SaveOptions, Store};
use loom_pipeline::{RangeEnds, Reresolved};
use loom_serialization::dto::app::v1 as api;
use loom_serialization::legacy::{pob_xml, share_code};

use crate::data::GameData;
use crate::edit;
use crate::error::{AppError, Code};
use crate::loadout;
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
    /// Unique within the session; a state moved between undo and redo
    /// gets a new one (the recovery history writes each ID once).
    id: u64,
    state: Arc<State>,
    entry: api::HistoryEntry,
}

/// An undo step for the recovery history (UX-19): its ID, what the change
/// was, and the build before it.
#[derive(Debug, Clone)]
pub struct UndoStep {
    /// Unique within the session, never reused for another state.
    pub id: u64,
    /// What the change was.
    pub entry: api::HistoryEntry,
    /// The state before the change.
    pub state: Arc<State>,
}

/// How many changes undo keeps.
pub const HISTORY_LIMIT: usize = 100;

/// The most dashboards a build keeps ([`Session::set_dashboards`]); it
/// keeps at least one, or none at all.
pub const MAX_DASHBOARDS: usize = 8;

/// The most components one dashboard holds.
pub const MAX_DASHBOARD_COMPONENTS: usize = 24;

/// The most fields one dashboard component charts.
pub const MAX_DASHBOARD_FIELDS: usize = 6;

/// The longest name of a dashboard, in characters; at least one.
pub const MAX_DASHBOARD_NAME: usize = 40;

/// The longest of every other text of a dashboard (an ID, a kind, a
/// width, a field, what a curve varies, a config input), in characters; at
/// least one.
pub const MAX_DASHBOARD_TEXT: usize = 64;

/// The application core.
#[derive(Debug)]
pub struct Session {
    store: Store,
    data: Option<Arc<GameData>>,
    data_error: Option<AppError>,
    /// The installed bundles (B-1), for the snapshot.
    installed: Vec<crate::bundles::Installed>,
    /// The quarantined bundles (Phase 5, P-e), for the snapshot.
    quarantined: Vec<crate::install::Found>,
    /// A notice about the choice of data (`APP_BUNDLE_FALLBACK`).
    data_notice: Option<AppError>,
    /// The data is loading at start ([`Self::set_loading`]).
    loading: bool,
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
    /// The last [`Change`] ID issued.
    change_id: u64,
}

/// A new build's PoB XML ([`Session::new_build`]); `{tree}` is the spec's
/// tree version attribute, or nothing. Our own template, in the shape PoB's files take.
pub(crate) const NEW_BUILD: &str = r#"<PathOfBuilding>
	<Build level="1" targetVersion="3_0" className="Scion" ascendClassName="None" mainSocketGroup="1" bandit="None" pantheonMajorGod="None" pantheonMinorGod="None"/>
	<Tree activeSpec="1">
		<Spec{tree} classId="0" ascendClassId="0" nodes=""/>
	</Tree>
	<Skills activeSkillSet="1">
		<SkillSet id="1"/>
	</Skills>
	<Items activeItemSet="1" useSecondWeaponSet="false">
		<ItemSet id="1" useSecondWeaponSet="false"/>
	</Items>
	<Config activeConfigSet="1">
		<ConfigSet id="1" title="Default"/>
	</Config>
</PathOfBuilding>
"#;

/// A file's name without its extension (a build's display name).
pub(crate) fn stem(path: &Path) -> String {
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
            installed: Vec::new(),
            quarantined: Vec::new(),
            data_notice: None,
            loading: false,
            current: None,
            version: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
            path: None,
            origin: None,
            notices: Vec::new(),
            generation: 0,
            change_id: 0,
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
        self.set_loaded(data.map(Arc::new));
    }

    /// Sets game data that may be shared (a loaded bundle the application
    /// keeps, [`crate::bundles::Library`]), or why it did not load. Clears
    /// the data notice.
    pub fn set_loaded(&mut self, data: Result<Arc<GameData>, AppError>) {
        self.data_notice = None;
        match data {
            Ok(d) => {
                self.data = Some(d);
                self.data_error = None;
            }
            Err(e) => {
                self.data = None;
                self.data_error = Some(e);
            }
        }
        self.version += 1;
    }

    /// Sets the installed and quarantined bundles the snapshot lists.
    pub fn set_installed(
        &mut self,
        installed: Vec<crate::bundles::Installed>,
        quarantined: Vec<crate::install::Found>,
    ) {
        self.installed = installed;
        self.quarantined = quarantined;
    }

    /// Marks the data as loading at start, or no longer loading.
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    /// Sets a notice about the choice of data (after [`Self::set_loaded`]).
    pub fn set_data_notice(&mut self, notice: Option<AppError>) {
        self.data_notice = notice;
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
        let state = import_state(text, &data)?;
        self.notices.clear();
        self.replace(state, None, origin, true);
        Ok(self.version)
    }

    /// Starts a new PoE1 build as PoB starts one (UX brief note 3, B-3):
    /// Scion, level 1, one empty passive spec on the data's tree version,
    /// one skill set, one item set, one config set. It is read from a fixed
    /// PoB template through the import path, so it saves and exports like
    /// any build. Not dirty until edited; no path (UX-15 names it).
    ///
    /// # Errors
    /// `APP_NO_DATA`; `APP_DATA_VERSION` when the data is not PoE1.
    pub fn new_build(&mut self) -> Result<u64, AppError> {
        let data = self.need_data()?;
        if data.version.game != loom_domain::Game::Poe1 {
            return Err(AppError::new(
                Code::DataVersion,
                "a new build needs PoE1 data",
            ));
        }
        // An unknown tree version is left out (PoB reads its latest); the
        // calculation reports it (`unchecked-tree-version`).
        let tree = data
            .tree_version
            .as_ref()
            .map(|t| format!(r#" treeVersion="{}""#, t.as_str()))
            .unwrap_or_default();
        let text = NEW_BUILD.replace("{tree}", &tree);
        let mut build = loom_pipeline::read_pob(&text)?;
        let ends = loom_pipeline::resolve(data.resolver, data.data, &mut build)?;
        self.notices.clear();
        self.replace(State { build, ends }, None, None, false);
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
        let (state, notice) = opened_state(build, self.data.as_deref());
        self.notices.extend(notice);
        self.replace(state, path, origin, dirty);
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
        Ok(self.commit(
            state,
            State {
                build: edited.build,
                ends: edited.ends,
            },
            entry,
        ))
    }

    /// Adds an item from a game copy or PoB's item text, as one undoable
    /// change; with `slot`, puts it there in the active item set.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, and [`crate::items::add`]'s codes.
    pub fn add_item(&mut self, text: &str, slot: Option<&str>) -> Result<u64, AppError> {
        let state = self.need_state()?;
        let data = self.need_data()?;
        let added = crate::items::add(&state.build, &state.ends, text, slot, &data)?;
        let entry = api::HistoryEntry {
            kind: api::ChangeKind::ItemAdded,
            item: added.item.0,
            request: None,
        };
        Ok(self.commit(
            state,
            State {
                build: added.build,
                ends: added.ends,
            },
            entry,
        ))
    }

    /// Makes item set `id` the active one, as one undoable change (the
    /// UX walkthrough's UX-02: the user picks the set the calculation
    /// uses). The same set again changes nothing.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_ITEM_SET` when the build has no set `id`.
    pub fn use_item_set(&mut self, id: u32) -> Result<Option<u64>, AppError> {
        let state = self.need_state()?;
        let set = ItemSetId(id);
        if !state.build.items.sets().any(|s| s.id == set) {
            return Err(AppError::new(Code::ItemSet, format!("no item set {id}")));
        }
        if loom_calc::poe1::collect::active_item_set(&state.build).map(|s| s.id) == Some(set) {
            return Ok(None);
        }
        let mut build = state.build.clone();
        build.items.active_set = Some(set);
        let next = State {
            build,
            ends: state.ends.clone(),
        };
        let entry = api::HistoryEntry {
            kind: api::ChangeKind::ItemSet,
            item: id,
            request: None,
        };
        Ok(Some(self.commit(state, next, entry)))
    }

    /// Changes the build with `change`, as one undoable change; `None` when
    /// it left the build as it was.
    fn change_build(
        &mut self,
        kind: api::ChangeKind,
        item: u32,
        change: impl FnOnce(&mut Build) -> Result<(), AppError>,
    ) -> Result<Option<u64>, AppError> {
        self.change_build_for(kind, item, None, change)
    }

    /// [`Session::change_build`], recording the change request it applies.
    fn change_build_for(
        &mut self,
        kind: api::ChangeKind,
        item: u32,
        request: Option<u64>,
        change: impl FnOnce(&mut Build) -> Result<(), AppError>,
    ) -> Result<Option<u64>, AppError> {
        let state = self.need_state()?;
        let mut build = state.build.clone();
        change(&mut build)?;
        if build == state.build {
            return Ok(None);
        }
        let next = State {
            build,
            ends: state.ends.clone(),
        };
        let entry = api::HistoryEntry {
            kind,
            item,
            request,
        };
        Ok(Some(self.commit(state, next, entry)))
    }

    /// Makes a stage (PoB's loadout) active: its passive spec and the sets
    /// linked to it ([`loadout`]).
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_LOADOUT` when no spec has ID `spec`.
    pub fn use_loadout(&mut self, spec: u32) -> Result<Option<u64>, AppError> {
        let main = self.main_skill_default()?;
        self.change_build(api::ChangeKind::Loadout, spec, |build| {
            let l = stage(build, spec)?;
            let before = build.skills.active;
            loadout::apply(build, &l);
            if build.skills.active != before {
                main(build);
            }
            Ok(())
        })
    }

    /// After a skill set switch, sets the build's main skill
    /// ([`crate::main_skill::choose_after_switch`]: the open build's main
    /// skill again when the new set has it) with the loaded data and the
    /// open build's range ends; does nothing without data.
    fn main_skill_default(&self) -> Result<impl Fn(&mut Build) + use<>, AppError> {
        let state = self.need_state()?;
        let ends = state.ends.clone();
        let data = self.data.clone();
        let previous = data
            .as_ref()
            .map(|d| crate::main_skill::current_skills(&state.build, d))
            .unwrap_or_default();
        Ok(move |build: &mut Build| {
            if let Some(data) = &data {
                main_after_switch(build, &ends, data, &previous);
            }
        })
    }

    /// Makes skill set `id` the active one.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_SKILL` when the build has no set `id`.
    pub fn use_skill_set(&mut self, id: u32) -> Result<Option<u64>, AppError> {
        let main = self.main_skill_default()?;
        self.change_build(api::ChangeKind::SkillSet, id, |build| {
            if !build.skills.sets.typed().any(|s| s.id.0 == id) {
                return Err(AppError::new(Code::Skill, format!("no skill set {id}")));
            }
            if build.skills.active != Some(SkillSetId(id)) {
                build.skills.active = Some(SkillSetId(id));
                main(build);
            }
            Ok(())
        })
    }

    /// Changes a gem of the active skill set: its level, quality, or
    /// whether it is enabled.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_SKILL` when there is no such group or gem, or a
    /// level is 0.
    pub fn edit_gem(
        &mut self,
        group: u32,
        gem: u32,
        level: Option<u32>,
        quality: Option<i32>,
        enabled: Option<bool>,
    ) -> Result<Option<u64>, AppError> {
        self.change_build(api::ChangeKind::SkillEdit, group, |build| {
            set_gem(build, group, gem, level, quality, enabled)
        })
    }

    /// Changes a socket group of the active skill set: whether it is
    /// enabled, its slot and label, or makes it the main skill group.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_SKILL` when there is no such group.
    pub fn edit_socket_group(
        &mut self,
        group: u32,
        enabled: Option<bool>,
        main: Option<bool>,
        main_gem: Option<u32>,
        place: GroupPlace,
    ) -> Result<Option<u64>, AppError> {
        let bundle = self.data.as_ref().map(|d| d.bundle);
        self.change_build(api::ChangeKind::SkillEdit, group, |build| {
            let g = socket_group(build, group)?;
            if let Some(enabled) = enabled {
                g.enabled = enabled;
            }
            if let Some(slot) = place.slot {
                g.slot = (!slot.is_empty()).then(|| SlotId::new(slot));
            }
            if let Some(label) = place.label {
                g.label = (!label.is_empty()).then_some(label);
            }
            if let Some(gem) = main_gem {
                // Its place among the group's active skills (a gem the data
                // does not know counts as active, as the view counts).
                let support = |x: &loom_domain::Gem| {
                    bundle
                        .and_then(|b| loom_calc::poe1::skill::gem_effects(x, b))
                        .and_then(|e| e.first().map(|f| f.support))
                        == Some(true)
                };
                let gems: Vec<_> = g.gems.typed().collect();
                if gems.get(gem as usize).is_none_or(|x| support(x)) {
                    return Err(AppError::new(
                        Code::Skill,
                        format!("gem {gem} is not an active skill"),
                    ));
                }
                let position = gems
                    .iter()
                    .take(gem as usize)
                    .filter(|x| !support(x))
                    .count();
                let value = u32::try_from(position + 1).unwrap_or(1);
                if g.main_active_skill.as_ref().map(|m| *m.value()) != Some(value) {
                    match &mut g.main_active_skill {
                        Some(m) => m.set(value),
                        None => g.main_active_skill = Some(Sourced::new(value)),
                    }
                }
            }
            let index = group + 1;
            if main == Some(true)
                && build
                    .character
                    .main_socket_group
                    .as_ref()
                    .map(|m| *m.value())
                    != Some(index)
            {
                match &mut build.character.main_socket_group {
                    Some(m) => m.set(index),
                    None => build.character.main_socket_group = Some(Sourced::new(index)),
                }
            }
            Ok(())
        })
    }

    /// Adds the gem of gem effect `gem` (a catalog entry's ID) to a socket
    /// group of the active skill set, at `at` (the end when absent). The
    /// group's main skill stays the same gem.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, `APP_SKILL` when there is no such
    /// group or the data has no such gem.
    pub fn add_gem(
        &mut self,
        group: u32,
        gem: &str,
        at: Option<u32>,
    ) -> Result<Option<u64>, AppError> {
        let data = self.need_data()?;
        let bundle = data.bundle;
        let new = crate::gems::new_gem(bundle, gem)
            .ok_or_else(|| AppError::new(Code::Skill, format!("no gem {gem} in the data")))?;
        self.change_build(api::ChangeKind::SkillEdit, group, |build| {
            let g = socket_group(build, group)?;
            let count = g.gems.typed().count();
            let at = at.map_or(count, |a| (a as usize).min(count));
            let main = main_gem_index(g, bundle);
            g.gems.insert_typed(at, new);
            set_main_gem(g, bundle, main.map(|m| if at <= m { m + 1 } else { m }));
            Ok(())
        })
    }

    /// Removes a gem from a socket group of the active skill set. The
    /// group's main skill stays the same gem; when that gem is removed, it
    /// becomes the group's first active skill.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_SKILL` when there is no such group or gem.
    pub fn remove_gem(&mut self, group: u32, gem: u32) -> Result<Option<u64>, AppError> {
        let bundle = self.data.as_ref().map(|d| d.bundle);
        self.change_build(api::ChangeKind::SkillEdit, group, |build| {
            let g = socket_group(build, group)?;
            let at = gem as usize;
            let main = bundle.and_then(|b| main_gem_index(g, b));
            g.gems
                .remove_typed(at)
                .ok_or_else(|| AppError::new(Code::Skill, format!("no gem {gem}")))?;
            if let Some(b) = bundle {
                let main = main.and_then(|m| match m.cmp(&at) {
                    std::cmp::Ordering::Less => Some(m),
                    std::cmp::Ordering::Equal => None,
                    std::cmp::Ordering::Greater => Some(m - 1),
                });
                set_main_gem(g, b, main);
            }
            Ok(())
        })
    }

    /// Adds an empty socket group at the end of the active skill set.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_SKILL` when the build has no skill set.
    pub fn add_socket_group(
        &mut self,
        slot: Option<String>,
        label: Option<String>,
    ) -> Result<Option<u64>, AppError> {
        let state = self.need_state()?;
        let active = loadout::active_skill_set(&state.build);
        let index = state
            .build
            .skills
            .sets
            .typed()
            .find(|s| Some(s.id) == active)
            .map_or(0, |s| s.groups.typed().count());
        self.change_build(
            api::ChangeKind::SkillEdit,
            u32::try_from(index).unwrap_or(u32::MAX),
            |build| {
                let set = build
                    .skills
                    .sets
                    .typed_mut()
                    .find(|s| Some(s.id) == active)
                    .ok_or_else(|| AppError::new(Code::Skill, "the build has no skill set"))?;
                set.groups.push(SocketGroup {
                    label: label.filter(|l| !l.is_empty()),
                    enabled: true,
                    slot: slot.filter(|s| !s.is_empty()).map(SlotId::new),
                    main_active_skill: None,
                    gems: Ordered::new(),
                    attributes: Vec::new(),
                });
                Ok(())
            },
        )
    }

    /// Removes a socket group of the active skill set. The main skill
    /// group stays the same group; when that group is removed, the group
    /// now at its place (else the last) is the main one.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_SKILL` when there is no such group.
    pub fn remove_socket_group(&mut self, group: u32) -> Result<Option<u64>, AppError> {
        self.change_build(api::ChangeKind::SkillEdit, group, |build| {
            let active = loadout::active_skill_set(build);
            let set = build
                .skills
                .sets
                .typed_mut()
                .find(|s| Some(s.id) == active)
                .ok_or_else(|| AppError::new(Code::Skill, "the build has no skill set"))?;
            set.groups
                .remove_typed(group as usize)
                .ok_or_else(|| AppError::new(Code::Skill, format!("no socket group {group}")))?;
            let left = u32::try_from(set.groups.typed().count()).unwrap_or(u32::MAX);
            if let Some(m) = &mut build.character.main_socket_group {
                let main = *m.value();
                let next = if main > group + 1 {
                    main - 1
                } else {
                    main.min(left.max(1))
                };
                if next != main {
                    m.set(next);
                }
            }
            Ok(())
        })
    }

    /// Switches the active item set to its first or second weapon set (PoB's
    /// `useSecondWeaponSet`).
    ///
    /// # Errors
    /// `APP_NO_BUILD`; `APP_ITEM_SET` when the build has no item set.
    pub fn use_weapon_set(&mut self, second: bool) -> Result<Option<u64>, AppError> {
        let state = self.need_state()?;
        let Some(id) = loom_calc::poe1::collect::active_item_set(&state.build).map(|s| s.id) else {
            return Err(AppError::new(Code::ItemSet, "the build has no item set"));
        };
        self.change_build(api::ChangeKind::WeaponSet, id.0, |build| {
            let set = build
                .items
                .entries
                .typed_mut()
                .find_map(|e| match e {
                    ItemsEntry::Set(s) if s.id == id => Some(s),
                    _ => None,
                })
                .ok_or_else(|| AppError::new(Code::ItemSet, "the item set is gone"))?;
            let value = if second { "true" } else { "false" };
            match set
                .attributes
                .iter_mut()
                .find(|a| a.name == crate::view::SECOND_WEAPON_SET)
            {
                Some(a) => value.clone_into(&mut a.value),
                // Absent reads as the first set.
                None if second => set
                    .attributes
                    .push(Attribute::new(crate::view::SECOND_WEAPON_SET, value)),
                None => {}
            }
            Ok(())
        })
    }

    /// A passive tree edit of spec `spec` (passive tree plan, T5a), as one
    /// undoable change.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA` (no tree), `APP_TREE`.
    pub fn edit_tree(
        &mut self,
        spec: u32,
        edit: crate::tree::Edit,
    ) -> Result<Option<u64>, AppError> {
        self.edit_tree_for(spec, edit, None)
    }

    /// [`Session::edit_tree`] for change request `request`, when it is one
    /// (history kind `tree-request-applied`).
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA` (no tree), `APP_TREE`.
    pub fn edit_tree_for(
        &mut self,
        spec: u32,
        edit: crate::tree::Edit,
        request: Option<u64>,
    ) -> Result<Option<u64>, AppError> {
        let tree = self.tree()?;
        let version = self.data.as_ref().and_then(|d| d.tree_version.clone());
        let kind = if request.is_some() {
            api::ChangeKind::TreeRequestApplied
        } else {
            api::ChangeKind::Tree
        };
        self.change_build_for(kind, spec, request, |build| {
            let outcome = crate::tree::run(build, spec, &tree, version.as_ref(), edit)?;
            crate::tree::write_spec(build, spec, &tree, &outcome.spec)
        })
    }

    /// Sockets a jewel in a tree socket of spec `spec`
    /// (`crate::tree::socket_jewel`), as one undoable change.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA` (no tree), `APP_TREE`.
    pub fn socket_jewel(
        &mut self,
        spec: u32,
        node: u32,
        item: Option<u32>,
    ) -> Result<Option<u64>, AppError> {
        let tree = self.tree()?;
        let data = self.data.clone();
        self.change_build(api::ChangeKind::Tree, spec, |build| {
            crate::tree::socket_jewel(
                build,
                spec,
                &tree,
                data.as_ref().map(|d| d.bundle),
                node,
                item,
            )
        })
    }

    /// Converts spec `spec` to the loaded data's tree version
    /// (`crate::tree::convert_spec`; passive tree plan, T5b), as one
    /// undoable change.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA` (no tree), `APP_TREE`.
    pub fn convert_tree_version(&mut self, spec: u32) -> Result<Option<u64>, AppError> {
        let tree = self.tree()?;
        let version = self.data.as_ref().and_then(|d| d.tree_version.clone());
        self.change_build(api::ChangeKind::TreeSpecs, spec, |build| {
            crate::tree::convert_spec(build, spec, &tree, version.as_ref()).map(|_| ())
        })
    }

    /// Adds a passive spec (`crate::tree::new_spec`), as one undoable
    /// change.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA` (no tree), `APP_TREE`.
    pub fn new_spec(&mut self, title: &str, copy: Option<u32>) -> Result<Option<u64>, AppError> {
        let tree = self.tree()?;
        let version = self.data.as_ref().and_then(|d| d.tree_version.clone());
        let item = crate::tree::spec_count(&self.need_state()?.build) + 1;
        self.change_build(api::ChangeKind::TreeSpecs, item, |build| {
            crate::tree::new_spec(build, &tree, version.as_ref(), title, copy).map(|_| ())
        })
    }

    /// Renames passive spec `spec`, as one undoable change.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_TREE`.
    pub fn rename_spec(&mut self, spec: u32, title: &str) -> Result<Option<u64>, AppError> {
        self.change_build(api::ChangeKind::TreeSpecs, spec, |build| {
            crate::tree::rename_spec(build, spec, title)
        })
    }

    /// Deletes passive spec `spec`, as one undoable change.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_TREE`.
    pub fn delete_spec(&mut self, spec: u32) -> Result<Option<u64>, AppError> {
        self.change_build(api::ChangeKind::TreeSpecs, spec, |build| {
            crate::tree::delete_spec(build, spec)
        })
    }

    /// The loaded data's passive tree.
    ///
    /// # Errors
    /// `APP_NO_DATA` when no data is loaded or it has no passive tree.
    pub fn tree(&self) -> Result<Arc<loom_tree::Tree>, AppError> {
        self.data
            .as_ref()
            .and_then(|d| d.tree.clone())
            .ok_or_else(|| AppError::new(Code::NoData, "the game data has no passive tree"))
    }

    /// Sets the league the build is planned for.
    ///
    /// # Errors
    /// `APP_NO_BUILD`.
    pub fn set_league(&mut self, league: &api::League) -> Result<Option<u64>, AppError> {
        let league = match league {
            api::League::Challenge => League::Challenge,
            api::League::Standard => League::Standard,
        };
        self.change_build(api::ChangeKind::League, 0, |build| {
            build.league = Some(league);
            Ok(())
        })
    }

    /// Sets the build's dashboards (the layout its author chose to show
    /// it with; never numbers), or clears them, as one undoable change.
    /// The same dashboards again change nothing.
    ///
    /// # Errors
    /// `APP_DASHBOARDS`, with nothing changed, for dashboards outside the
    /// limits ([`MAX_DASHBOARDS`] and the limits beside it), with a
    /// repeated ID, or whose `active` is none of them; `APP_NO_BUILD`.
    pub fn set_dashboards(
        &mut self,
        dashboards: Option<api::Dashboards>,
    ) -> Result<Option<u64>, AppError> {
        let dashboards = dashboards.map(checked_dashboards).transpose()?;
        self.change_build(api::ChangeKind::Dashboards, 0, |build| {
            build.dashboards = dashboards;
            Ok(())
        })
    }

    /// Makes config set `id` the active one.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_CONFIG` when the build has no set `id`.
    pub fn use_config_set(&mut self, id: u32) -> Result<Option<u64>, AppError> {
        self.change_build(api::ChangeKind::Config, id, |build| {
            if !build.config.sets.typed().any(|s| s.id.0 == id) {
                return Err(AppError::new(Code::Config, format!("no config set {id}")));
            }
            build.config.active = Some(ConfigSetId(id));
            Ok(())
        })
    }

    /// Sets a config input of the active config set: its value, which a
    /// placeholder of that name becomes; a new input goes after the
    /// others. A build with no config set gets one (ID 1).
    ///
    /// # Errors
    /// `APP_NO_BUILD`; `APP_CONFIG` for an empty name, PoB's legacy
    /// `customMods` (its text is lines, edited as a custom modifier block),
    /// or a number that is not finite.
    pub fn set_config_input(
        &mut self,
        name: &str,
        value: &api::ConfigValueView,
    ) -> Result<Option<u64>, AppError> {
        if name.trim().is_empty() || name == LEGACY_CUSTOM_MODS {
            return Err(AppError::new(
                Code::Config,
                format!("the config input {name:?} cannot be set here"),
            ));
        }
        let value = match value {
            api::ConfigValueView::Boolean(b) => ConfigValue::Boolean(*b),
            api::ConfigValueView::Number(n) if n.is_finite() => {
                ConfigValue::Number(Sourced::new(Number(*n)))
            }
            api::ConfigValueView::Number(_) => {
                return Err(AppError::new(
                    Code::Config,
                    "a config number must be finite",
                ));
            }
            api::ConfigValueView::Text(t) => ConfigValue::String(t.clone()),
        };
        self.change_build(api::ChangeKind::Config, 0, |build| {
            write_config_input(build, name, value);
            Ok(())
        })
    }

    /// Removes a config input of the active config set.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_CONFIG` when the set has no input `name`.
    pub fn remove_config_input(&mut self, name: &str) -> Result<Option<u64>, AppError> {
        self.change_build(api::ChangeKind::Config, 0, |build| {
            let set = active_config(build)?;
            let at = set
                .inputs
                .typed()
                .position(|i| i.name == name)
                .ok_or_else(|| AppError::new(Code::Config, format!("no config input {name}")))?;
            set.inputs.remove_typed(at);
            Ok(())
        })
    }

    /// Changes a custom modifier block of the active config set: whether
    /// it applies, or its text, whose lines are read again (PoB's rule,
    /// [`pob_xml::custom_modifier_lines`]) and resolved as an import
    /// resolves them.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_CONFIG` when there is no such block; with a
    /// text, `APP_NO_DATA` and the resolver's codes.
    pub fn edit_custom_block(
        &mut self,
        block: u32,
        enabled: Option<bool>,
        text: Option<&str>,
    ) -> Result<Option<u64>, AppError> {
        let data = if text.is_some() {
            Some(self.need_data()?)
        } else {
            None
        };
        self.change_build(api::ChangeKind::Config, block, |build| {
            let b = active_config(build)?
                .custom_modifiers
                .get_mut(block as usize)
                .ok_or_else(|| AppError::new(Code::Config, format!("no custom block {block}")))?;
            if let Some(enabled) = enabled {
                b.enabled = enabled;
            }
            if let Some(text) = text.filter(|t| *t != b.text) {
                b.text = text.to_owned();
                b.lines = custom_lines(text);
            }
            if let Some(data) = &data {
                resolve_pending(build, data)?;
            }
            Ok(())
        })
    }

    /// Adds a custom modifier block, enabled, with `text`, after the
    /// others of the active config set (one is made when the build has
    /// none).
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_NO_DATA`, and the resolver's codes.
    pub fn add_custom_block(&mut self, text: &str) -> Result<Option<u64>, AppError> {
        let data = self.need_data()?;
        self.change_build(api::ChangeKind::Config, 0, |build| {
            active_config_or_new(build)
                .custom_modifiers
                .push(CustomModifierBlock {
                    title: None,
                    enabled: true,
                    text: text.to_owned(),
                    lines: custom_lines(text),
                    attributes: Attributes::default(),
                });
            resolve_pending(build, &data)
        })
    }

    /// Removes a custom modifier block of the active config set.
    ///
    /// # Errors
    /// `APP_NO_BUILD`, `APP_CONFIG` when there is no such block.
    pub fn remove_custom_block(&mut self, block: u32) -> Result<Option<u64>, AppError> {
        self.change_build(api::ChangeKind::Config, block, |build| {
            let blocks = &mut active_config(build)?.custom_modifiers;
            if block as usize >= blocks.len() {
                return Err(AppError::new(
                    Code::Config,
                    format!("no custom block {block}"),
                ));
            }
            blocks.remove(block as usize);
            Ok(())
        })
    }

    fn next_change_id(&mut self) -> u64 {
        self.change_id += 1;
        self.change_id
    }

    /// The undo steps, oldest first (for the recovery history).
    #[must_use]
    pub fn undo_steps(&self) -> Vec<UndoStep> {
        self.undo
            .iter()
            .map(|c| UndoStep {
                id: c.id,
                entry: c.entry.clone(),
                state: Arc::clone(&c.state),
            })
            .collect()
    }

    /// Gives a build just opened from a recovery snapshot its undo history
    /// back (UX-19): `steps` oldest first, each the build before a change.
    /// A step is resolved again for its range ends as [`Self::open_build`]
    /// does; its notices are not shown (they are the open build's to say).
    /// Nothing happens without an open build.
    pub fn restore_history(&mut self, steps: Vec<(api::HistoryEntry, Build)>) {
        if self.current.is_none() {
            return;
        }
        let keep = steps.len().saturating_sub(HISTORY_LIMIT);
        let mut undo = Vec::new();
        for (entry, build) in steps.into_iter().skip(keep) {
            let ends = self.ends_of(&build).unwrap_or_default();
            let id = self.next_change_id();
            undo.push(Change {
                id,
                state: Arc::new(State { build, ends }),
                entry,
            });
        }
        // Same build, same version: only what undo can reach changed.
        self.undo = undo;
        self.redo.clear();
    }

    /// The range ends of a build resolved again with the loaded data, when
    /// the data is the build's and it resolves the same.
    fn ends_of(&self, build: &Build) -> Option<RangeEnds> {
        match reresolved(build, self.data.as_deref()?)? {
            Ok(Reresolved::Same(e)) => Some(e),
            _ => None,
        }
    }

    /// Makes `next` current, keeping `previous` for undo.
    fn commit(&mut self, previous: Arc<State>, next: State, entry: api::HistoryEntry) -> u64 {
        let id = self.next_change_id();
        self.undo.push(Change {
            id,
            state: previous,
            entry,
        });
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.current = Some(Arc::new(next));
        self.dirty = true;
        self.version += 1;
        self.version
    }

    /// Reverts the latest change; `None` when there is none.
    pub fn undo(&mut self) -> Option<u64> {
        let change = self.undo.pop()?;
        let current = self.current.replace(change.state)?;
        let id = self.next_change_id();
        self.redo.push(Change {
            id,
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
        let id = self.next_change_id();
        self.undo.push(Change {
            id,
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
        Ok(view::item_view_of(&state, item, self.data.as_deref()))
    }

    /// The snapshot DTO, with the calculation's status.
    #[must_use]
    pub fn snapshot(&self, calc: api::CalcStatus) -> api::Snapshot {
        let entry = |c: Option<&Change>| c.map(|c| c.entry.clone());
        api::Snapshot {
            version: self.version,
            build: self.current.as_ref().map(|s| {
                let mut v =
                    view::build_view(&s.build, self.name(), self.data.as_ref().map(|d| d.bundle));
                v.tree = self.data.as_ref().and_then(|d| {
                    crate::tree::view(
                        &s.build,
                        d.tree.as_ref()?,
                        d.tree_version.as_ref(),
                        Some(d.bundle),
                    )
                });
                v
            }),
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
                installed: self
                    .installed
                    .iter()
                    .map(|i| api::InstalledBundle {
                        id: i.id.clone(),
                        data_version: view::data_version(&i.version),
                        tree_version: crate::data::tree_version(&i.version)
                            .map(|t| t.as_str().to_owned()),
                        signed_by: match &i.status {
                            crate::install::Status::Trusted { key_id } => Some(key_id.clone()),
                            _ => None,
                        },
                        accepted: matches!(i.status, crate::install::Status::Accepted(_)),
                    })
                    .collect(),
                active: self.data.as_ref().and_then(|d| {
                    self.installed
                        .iter()
                        .find(|i| i.dir == d.dir)
                        .map(|i| i.id.clone())
                }),
                notice: self.data_notice.as_ref().map(AppError::dto),
                pinned: self.pinned_bundle(),
                quarantined: self.quarantined.iter().map(quarantined_view).collect(),
                loading: self.loading,
            },
            notices: self.notices.iter().map(AppError::dto).collect(),
        }
    }
}

/// A quarantined bundle's view.
fn quarantined_view(q: &crate::install::Found) -> api::QuarantinedBundle {
    use crate::install::{Quarantine, Status};
    let reason = match q.status {
        Status::Quarantined(r) | Status::Accepted(r) => r,
        Status::Trusted { .. } => Quarantine::Damaged,
    };
    api::QuarantinedBundle {
        id: q.id.clone(),
        data_version: view::data_version(&q.version),
        reason: reason.name().to_owned(),
        acceptable: reason.acceptable(),
        manifest_sha256: q.manifest_sha256.clone(),
        key_id: q.key_id.clone(),
        files: u32::try_from(q.files).unwrap_or(u32::MAX),
        bytes: q.bytes,
    }
}

impl Session {
    /// The installed bundle the open build was made with, when the loaded
    /// data is another one (ADR-010: offered, never switched silently).
    fn pinned_bundle(&self) -> Option<String> {
        let pinned = self.current.as_ref()?.build.data.as_ref()?;
        if self.data.as_ref().is_some_and(|d| d.version == *pinned) {
            return None;
        }
        self.installed
            .iter()
            .find(|i| i.version == *pinned)
            .map(|i| i.id.clone())
    }
}

/// Socket group `group` (an index) of the active skill set.
/// Where a socket group goes: its slot and label (each: `None` keeps it,
/// empty clears it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupPlace {
    /// The equipment slot.
    pub slot: Option<String>,
    /// The label.
    pub label: Option<String>,
}

/// Whether the data reads gem `x` as a support (a gem it does not know is
/// not one).
fn is_support(x: &loom_domain::Gem, bundle: &loom_data::bundle::Bundle) -> bool {
    loom_calc::poe1::skill::gem_effects(x, bundle).and_then(|e| e.first().map(|f| f.support))
        == Some(true)
}

/// The index (among all its gems) of the gem a group's `mainActiveSkill`
/// names (the n-th gem that is not a support), if it names one.
fn main_gem_index(g: &SocketGroup, bundle: &loom_data::bundle::Bundle) -> Option<usize> {
    let n = usize::try_from(*g.main_active_skill.as_ref()?.value()).ok()?;
    g.gems
        .typed()
        .enumerate()
        .filter(|(_, x)| !is_support(x, bundle))
        .nth(n.checked_sub(1)?)
        .map(|(i, _)| i)
}

/// Points a group's `mainActiveSkill` (when it has one) at gem `main` (by
/// index among all its gems), or at its first active skill when `None`.
fn set_main_gem(g: &mut SocketGroup, bundle: &loom_data::bundle::Bundle, main: Option<usize>) {
    let value = main.map_or(1, |m| {
        let before = g
            .gems
            .typed()
            .take(m)
            .filter(|x| !is_support(x, bundle))
            .count();
        u32::try_from(before + 1).unwrap_or(1)
    });
    if let Some(m) = &mut g.main_active_skill
        && *m.value() != value
    {
        m.set(value);
    }
}

/// The build's stage (PoB's loadout) of passive spec `spec`.
///
/// # Errors
/// `APP_LOADOUT` when no spec has ID `spec`.
pub(crate) fn stage(build: &Build, spec: u32) -> Result<loadout::Loadout, AppError> {
    loadout::loadouts(build)
        .into_iter()
        .find(|l| l.spec.0 == spec)
        .ok_or_else(|| AppError::new(Code::Loadout, format!("no stage {spec}")))
}

/// A PoB share code or XML text read and resolved with the data, as an
/// import makes it ([`Session::import_text`]; a file compared without
/// opening it is read the same way).
///
/// # Errors
/// The import adapter's and the resolver's codes.
pub(crate) fn import_state(text: &str, data: &GameData) -> Result<State, AppError> {
    let mut build = loom_pipeline::read_pob(text)?;
    let ends = loom_pipeline::resolve(data.resolver, data.data, &mut build)?;
    // A spec PoB numbered before its cluster hash fix takes the
    // current numbers, as PoB loads it (passive tree plan, T-i).
    if let Some(tree) = &data.tree {
        loom_pipeline::migrate_cluster_hashes(&mut build, tree, data.tree_version.as_ref());
    }
    let mut state = State { build, ends };
    // Exiled Studio's own main skill, not PoB's number (Q-UX-1).
    crate::main_skill::set_default(&mut state, data);
    Ok(state)
}

/// A build resolved again with the data, when the data is the build's
/// (its game and data version); `None` when it is not.
fn reresolved(build: &Build, data: &GameData) -> Option<Result<Reresolved, loom_pipeline::Error>> {
    (build.game() == data.version.game && build.data.as_ref() == Some(&data.version))
        .then(|| loom_pipeline::reresolve(data.resolver, data.data, build))
}

/// The state of a build read with its resolutions (a saved document, a
/// recovery snapshot), as [`Session::open_build`] opens it: with data of
/// the build's version, a clone is resolved again to recover the range
/// ends; when it resolves differently the stored resolutions stand, and
/// the notice says so.
pub(crate) fn opened_state(build: Build, data: Option<&GameData>) -> (State, Option<AppError>) {
    let mut ends = RangeEnds::new();
    let mut notice = None;
    match data.and_then(|d| reresolved(&build, d)) {
        Some(Ok(Reresolved::Same(e))) => ends = e,
        Some(Ok(Reresolved::Changed)) => {
            notice = Some(AppError::new(
                Code::ResolutionChanged,
                "the stored resolutions differ from this bundle's; they stand",
            ));
        }
        // The checks match the resolver's own.
        Some(Err(e)) => notice = Some(e.into()),
        None => {}
    }
    (State { build, ends }, notice)
}

/// Socket group `group` of the active skill set.
pub(crate) fn group_of(build: &Build, group: u32) -> Result<&SocketGroup, AppError> {
    let active = loadout::active_skill_set(build);
    build
        .skills
        .sets
        .typed()
        .find(|s| Some(s.id) == active)
        .and_then(|s| s.groups.typed().nth(group as usize))
        .ok_or_else(|| AppError::new(Code::Skill, format!("no socket group {group}")))
}

fn socket_group(build: &mut Build, group: u32) -> Result<&mut SocketGroup, AppError> {
    let active = loadout::active_skill_set(build);
    build
        .skills
        .sets
        .typed_mut()
        .find(|s| Some(s.id) == active)
        .and_then(|s| s.groups.typed_mut().nth(group as usize))
        .ok_or_else(|| AppError::new(Code::Skill, format!("no socket group {group}")))
}

/// Changes gem `gem` of socket group `group` of the active skill set
/// ([`Session::edit_gem`]; a what-if point writes a level the same way).
///
/// # Errors
/// `APP_SKILL` when there is no such group or gem, or a level is 0.
pub(crate) fn set_gem(
    build: &mut Build,
    group: u32,
    gem: u32,
    level: Option<u32>,
    quality: Option<i32>,
    enabled: Option<bool>,
) -> Result<(), AppError> {
    let g = socket_group(build, group)?
        .gems
        .typed_mut()
        .nth(gem as usize)
        .ok_or_else(|| AppError::new(Code::Skill, format!("no gem {gem}")))?;
    if let Some(level) = level.filter(|l| *l != *g.level.value()) {
        if level == 0 {
            return Err(AppError::new(Code::Skill, "a gem's level is at least 1"));
        }
        g.level.set(level);
    }
    if let Some(quality) = quality.filter(|q| *q != *g.quality.value()) {
        g.quality.set(quality);
    }
    if let Some(enabled) = enabled {
        g.enabled = enabled;
    }
    Ok(())
}

/// Sets the build's main skill after its active skill set changed
/// ([`crate::main_skill::choose_after_switch`]); `previous` is the main
/// skill before the switch ([`crate::main_skill::current_skills`]).
pub(crate) fn main_after_switch(
    build: &mut Build,
    ends: &RangeEnds,
    data: &GameData,
    previous: &std::collections::BTreeSet<String>,
) {
    let state = State {
        build: build.clone(),
        ends: ends.clone(),
    };
    if let Some(choice) = crate::main_skill::choose_after_switch(&state, data, previous) {
        crate::main_skill::apply(build, choice);
    }
}

/// Writes input `name` of the active config set
/// ([`Session::set_config_input`]; a what-if point writes a number the
/// same way): a placeholder of that name becomes an input; a new input
/// goes after the others. A build with no config set gets one (ID 1).
pub(crate) fn write_config_input(build: &mut Build, name: &str, value: ConfigValue) {
    let set = active_config_or_new(build);
    if let Some(input) = set.inputs.typed_mut().find(|i| i.name == name) {
        // The same number keeps its source text (an export writes it as
        // read).
        let same = match (&input.value, &value) {
            (ConfigValue::Number(a), ConfigValue::Number(b)) => a.value() == b.value(),
            (a, b) => a == b,
        };
        if !same {
            input.value = value;
        }
        input.kind = ConfigInputKind::Input;
    } else {
        set.inputs.push(ConfigInput {
            name: name.to_owned(),
            value,
            kind: ConfigInputKind::Input,
            lines: Vec::new(),
            attributes: Attributes::default(),
        });
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
    let (tree, version) = loom_pipeline::pick_tree(
        &state.build,
        data.tree.as_deref(),
        data.tree_version.clone(),
        &data.trees,
    );
    // The Timeless jewels' results the build needs, read on demand (P6-10a).
    let results = tree.map_or_else(Default::default, |t| {
        data.timeless_results(&loom_pipeline::timeless::needed(&state.build, t))
    });
    let timeless = data.bundle.poe1.as_ref().and_then(|p| {
        Some(loom_pipeline::timeless::TimelessData {
            timeless: p.timeless.as_ref()?,
            luts: p.timeless_luts.as_ref()?,
            results: results.iter().map(|(t, b)| (*t, b.as_slice())).collect(),
        })
    });
    let options = loom_pipeline::options_with(
        &state.build,
        data.data,
        &state.ends,
        version,
        tree,
        timeless.as_ref(),
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
        .map(|i| view::item_view_of(&after_state, i, Some(data)))
        .ok_or_else(|| AppError::new(Code::EditItem, format!("no item {}", e.item)))?;
    Ok(api::ItemEditPreview { item, fields })
}

/// A tree edit's effect, calculated off the writer thread as an item
/// edit's: the path allocated, the nodes removed, each field before and
/// after (empty when the build does not calculate with the data).
///
/// # Errors
/// `APP_NO_DATA` (no tree), `APP_TREE`; the calculation's.
pub fn preview_tree(
    state: &State,
    data: &GameData,
    spec: u32,
    edit: crate::tree::Edit,
    before: Option<&CalcResult>,
    cancel: &dyn Fn() -> bool,
) -> Result<api::TreePreview, AppError> {
    preview_tree_with(state, data, spec, edit, before, true, cancel)
}

/// [`preview_tree`], with the fields only when `fields` (without, no
/// calculation runs: the path and the nodes removed alone).
///
/// # Errors
/// As [`preview_tree`].
pub fn preview_tree_with(
    state: &State,
    data: &GameData,
    spec: u32,
    edit: crate::tree::Edit,
    before: Option<&CalcResult>,
    fields: bool,
    cancel: &dyn Fn() -> bool,
) -> Result<api::TreePreview, AppError> {
    let tree = data
        .tree
        .as_ref()
        .ok_or_else(|| AppError::new(Code::NoData, "the game data has no passive tree"))?;
    let outcome = crate::tree::run(&state.build, spec, tree, data.tree_version.as_ref(), edit)?;
    let node = match edit {
        crate::tree::Edit::Allocate(n)
        | crate::tree::Edit::Deallocate(n)
        | crate::tree::Edit::Mastery { node: n, .. } => n,
        _ => 0,
    };
    let wanted = fields;
    let mut fields = Vec::new();
    if wanted && check_data(&state.build, data).is_ok() {
        let mut build = state.build.clone();
        crate::tree::write_spec(&mut build, spec, tree, &outcome.spec)?;
        let computed;
        let before = match before {
            Some(b) => b,
            None => {
                computed = calculate(state, data, cancel)?;
                &computed
            }
        };
        let after_state = State {
            build,
            ends: state.ends.clone(),
        };
        let after = calculate(&after_state, data, cancel)?;
        fields = view::fields(before)
            .into_iter()
            .zip(view::fields(&after))
            .map(|((field, b, sb), (_, a, sa))| api::FieldDelta {
                field,
                before: b,
                after: a,
                state_before: sb,
                state_after: sa,
            })
            .collect();
    }
    Ok(api::TreePreview {
        node,
        path: outcome.path,
        removed: outcome.removed,
        fields,
    })
}

/// Whether two paths name the same file (by canonical path when both
/// exist, else by text).
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// PoB's legacy custom modifier input: its text is lines, which the page
/// does not edit as an input.
const LEGACY_CUSTOM_MODS: &str = "customMods";

/// The active config set (the first when none is named).
fn active_config(build: &mut Build) -> Result<&mut ConfigSet, AppError> {
    let active = loadout::active_config_set(build);
    build
        .config
        .sets
        .typed_mut()
        .find(|s| Some(s.id) == active)
        .ok_or_else(|| AppError::new(Code::Config, "the build has no config set"))
}

/// The active config set, made (ID 1, active) when the build has none.
fn active_config_or_new(build: &mut Build) -> &mut ConfigSet {
    if loadout::active_config_set(build).is_none() {
        build.config.sets.push(ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs: Ordered::default(),
            custom_modifiers: Vec::new(),
            attributes: Attributes::default(),
        });
        build.config.active = Some(ConfigSetId(1));
    }
    let active = loadout::active_config_set(build);
    build
        .config
        .sets
        .typed_mut()
        .find(|s| Some(s.id) == active)
        .unwrap_or_else(|| unreachable!("a config set was just made"))
}

/// A custom modifier block's lines read from `text`, pending resolution.
fn custom_lines(text: &str) -> Vec<CustomModifierLine> {
    pob_xml::custom_modifier_lines(text)
        .map(|text| CustomModifierLine {
            text,
            resolution: LineResolution::Pending,
        })
        .collect()
}

/// Resolves the build's pending lines (a changed block's), as an import
/// does. The items' lines are resolved already, so their range ends stand.
fn resolve_pending(build: &mut Build, data: &GameData) -> Result<(), AppError> {
    loom_pipeline::resolve(data.resolver, data.data, build)?;
    Ok(())
}

/// Whether `text` has 1 to `max` characters.
fn within(text: &str, max: usize) -> bool {
    !text.is_empty() && text.chars().count() <= max
}

/// One text of a dashboard, checked against [`MAX_DASHBOARD_TEXT`].
fn dashboard_text(what: &str, text: &str) -> Result<(), AppError> {
    if within(text, MAX_DASHBOARD_TEXT) {
        Ok(())
    } else {
        Err(AppError::new(
            Code::Dashboards,
            format!("a dashboard's {what} must have 1 to {MAX_DASHBOARD_TEXT} characters"),
        ))
    }
}

/// The dashboards a command sent as the build keeps them, once every rule
/// of [`Session::set_dashboards`] holds.
fn checked_dashboards(dashboards: api::Dashboards) -> Result<Dashboards, AppError> {
    let refuse = |message: String| AppError::new(Code::Dashboards, message);
    let count = dashboards.list.len();
    if count == 0 || count > MAX_DASHBOARDS {
        return Err(refuse(format!(
            "{count} dashboards; a build keeps 1 to {MAX_DASHBOARDS}"
        )));
    }
    let mut ids = BTreeSet::new();
    let mut component_ids = BTreeSet::new();
    for dashboard in &dashboards.list {
        dashboard_text("ID", &dashboard.id)?;
        if !ids.insert(dashboard.id.as_str()) {
            return Err(refuse(format!(
                "two dashboards have the ID {}",
                dashboard.id
            )));
        }
        if !within(&dashboard.name, MAX_DASHBOARD_NAME) {
            return Err(refuse(format!(
                "a dashboard's name must have 1 to {MAX_DASHBOARD_NAME} characters"
            )));
        }
        if dashboard.components.len() > MAX_DASHBOARD_COMPONENTS {
            return Err(refuse(format!(
                "a dashboard of {} components; at most {MAX_DASHBOARD_COMPONENTS}",
                dashboard.components.len()
            )));
        }
        for component in &dashboard.components {
            dashboard_text("component ID", &component.id)?;
            if !component_ids.insert(component.id.as_str()) {
                return Err(refuse(format!(
                    "two dashboard components have the ID {}",
                    component.id
                )));
            }
            dashboard_text("component kind", &component.kind)?;
            dashboard_text("component width", &component.width)?;
            if component.fields.len() > MAX_DASHBOARD_FIELDS {
                return Err(refuse(format!(
                    "a dashboard component of {} fields; at most {MAX_DASHBOARD_FIELDS}",
                    component.fields.len()
                )));
            }
            let texts = [
                ("field", &component.field),
                ("varied value", &component.vary),
                ("config input", &component.input),
            ];
            for (what, text) in texts {
                if let Some(text) = text {
                    dashboard_text(what, text)?;
                }
            }
            for field in &component.fields {
                dashboard_text("field", field)?;
            }
        }
    }
    if !ids.contains(dashboards.active.as_str()) {
        return Err(refuse(
            "the active dashboard is none of the build's dashboards".to_owned(),
        ));
    }
    Ok(Dashboards {
        active: dashboards.active,
        list: dashboards
            .list
            .into_iter()
            .map(|d| Dashboard {
                id: d.id,
                name: d.name,
                components: d
                    .components
                    .into_iter()
                    .map(|c| DashboardComponent {
                        id: c.id,
                        kind: c.kind,
                        width: c.width,
                        field: c.field,
                        fields: c.fields,
                        vary: c.vary,
                        input: c.input,
                        target: c.target,
                        new_row: c.new_row,
                    })
                    .collect(),
            })
            .collect(),
    })
}
