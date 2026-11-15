//! Game data found for the user (UX brief note 1, B-1; start page plan,
//! PR B).
//!
//! Phase 5's installer (P-e) puts signed bundles side by side in
//! `<app data>/bundles/<game>/<steam build>-s<schema>/`. This module is the
//! development-side part of that layout, without signing: it lists what is
//! installed there (from each `bundle.json` manifest only; the full check
//! runs at load), remembers the user's choice in `settings.json`, and keeps
//! each loaded bundle so switching back does not load it again.
//!
//! **Start order:** `LOOM_BUNDLE_DIR` (development); else the remembered
//! choice (an installed bundle's ID, or a folder picked for development);
//! else the newest installed bundle (highest Steam build, then schema). A
//! remembered bundle that fails to load gives way to the newest installed
//! one, and the snapshot says so (`APP_BUNDLE_FALLBACK`).
//!
//! Nothing here copies into `bundles/`: installing is P-e's (ADR-010: an
//! unsigned pack is quarantined), so a folder the user picks stays where it
//! is, labelled unsigned in the UI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use loom_data::bundle::{MANIFEST, Manifest};
use loom_domain::DataVersion;

use crate::data::GameData;
use crate::error::{AppError, Code};

/// The folder, under the application's data folder, that holds installed
/// bundles.
pub const BUNDLES_DIR: &str = "bundles";

/// The file, under the application's data folder, that remembers the
/// choice of data.
pub const SETTINGS_FILE: &str = "settings.json";

/// A bundle installed under [`BUNDLES_DIR`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// `<game>/<steam build>-s<schema>`.
    pub id: String,
    /// Its folder.
    pub dir: PathBuf,
    /// Its data version, from the manifest.
    pub version: DataVersion,
}

/// The ID of a data version: `<game>/<steam build>-s<schema>` (`unknown`
/// for a bundle without a Steam build).
#[must_use]
pub fn bundle_id(version: &DataVersion) -> String {
    let build = version
        .steam_build
        .map_or_else(|| "unknown".to_owned(), |b| b.to_string());
    format!(
        "{}/{build}-s{}",
        version.game.name(),
        version.schema_version
    )
}

/// The data version a bundle folder's manifest names, if it reads.
fn manifest_version(dir: &Path) -> Option<DataVersion> {
    let bytes = std::fs::read(dir.join(MANIFEST)).ok()?;
    let manifest: Manifest = serde_json::from_slice(&bytes).ok()?;
    Some(DataVersion {
        game: manifest.game()?,
        schema_version: manifest.data_schema,
        steam_build: manifest.source.steam_build,
    })
}

/// The bundles installed under `<data_dir>/bundles/*/*/`, newest first
/// (highest Steam build, then schema; then by ID). A folder whose manifest
/// does not read is skipped; two folders with the same ID keep the first by
/// path.
#[must_use]
pub fn installed(data_dir: &Path) -> Vec<Installed> {
    let mut out: Vec<Installed> = Vec::new();
    let Ok(games) = std::fs::read_dir(data_dir.join(BUNDLES_DIR)) else {
        return out;
    };
    let mut dirs: Vec<PathBuf> = games
        .flatten()
        .filter(|g| g.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|g| std::fs::read_dir(g.path()).ok())
        .flat_map(|d| d.flatten())
        .filter(|b| b.file_type().is_ok_and(|t| t.is_dir()))
        .map(|b| b.path())
        .collect();
    dirs.sort();
    for dir in dirs {
        let Some(version) = manifest_version(&dir) else {
            continue;
        };
        let id = bundle_id(&version);
        if out.iter().all(|i| i.id != id) {
            out.push(Installed { id, dir, version });
        }
    }
    out.sort_by(|a, b| {
        (b.version.steam_build, b.version.schema_version)
            .cmp(&(a.version.steam_build, a.version.schema_version))
            .then_with(|| a.id.cmp(&b.id))
    });
    out
}

/// What `settings.json` remembers.
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Settings {
    /// An installed bundle's ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    /// A bundle folder picked for development (the shell's settings before
    /// B-1 used this name too, so an older file still reads).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<PathBuf>,
    /// The MCP server's last port (UX-26), tried first at its next start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_port: Option<u16>,
    /// How the application looks for its own updates (Phase 5, P-d);
    /// absent means the default, `automatic-check`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updates: Option<crate::api::UpdateMode>,
}

impl Settings {
    /// Reads the settings; the defaults when the file is missing or does
    /// not read (it is a preference, not data).
    #[must_use]
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Reads the settings, changes them, and writes them back. One update
    /// at a time: the writer thread (the bundle) and the shell (the MCP
    /// port) both update the file.
    pub fn update(path: &Path, f: impl FnOnce(&mut Self)) {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut settings = Self::load(path);
        f(&mut settings);
        settings.save(path);
    }

    /// Writes the settings, replacing the file; a failure is ignored (the
    /// choice is only not remembered).
    pub fn save(&self, path: &Path) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = loom_persistence::Store::new().write_replacing(path, text.as_bytes());
        }
    }
}

/// The cache key of a folder: its canonical path, so two spellings of one
/// folder load it once (the path as given when it cannot be resolved).
fn cache_key(dir: &Path) -> PathBuf {
    std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_owned())
}

/// The installed bundles, the loaded ones, and the remembered choice.
#[derive(Debug)]
pub struct Library {
    data_dir: PathBuf,
    installed: Vec<Installed>,
    /// Loaded bundles by folder: [`GameData::load`] leaks its bundle, so
    /// each folder is loaded once per run.
    loaded: HashMap<PathBuf, Arc<GameData>>,
}

impl Library {
    /// The library of `data_dir`, with its installed bundles listed.
    #[must_use]
    pub fn new(data_dir: &Path) -> Self {
        Self {
            data_dir: data_dir.to_owned(),
            installed: installed(data_dir),
            loaded: HashMap::new(),
        }
    }

    /// The installed bundles, newest first.
    #[must_use]
    pub fn installed(&self) -> &[Installed] {
        &self.installed
    }

    fn settings_path(&self) -> PathBuf {
        self.data_dir.join(SETTINGS_FILE)
    }

    /// Loads a folder, or returns it if it was loaded before.
    ///
    /// # Errors
    /// `APP_BUNDLE`.
    pub fn load(&mut self, dir: &Path) -> Result<Arc<GameData>, AppError> {
        let key = cache_key(dir);
        if let Some(d) = self.loaded.get(&key) {
            return Ok(Arc::clone(d));
        }
        let data = Arc::new(GameData::load(dir)?);
        self.loaded.insert(key, Arc::clone(&data));
        Ok(data)
    }

    /// The ID of an installed bundle in `dir`, if it is one.
    #[must_use]
    pub fn id_of(&self, dir: &Path) -> Option<&str> {
        self.installed
            .iter()
            .find(|i| i.dir == dir)
            .map(|i| i.id.as_str())
    }

    /// The installed bundle with exactly this data version, if any.
    #[must_use]
    pub fn matching(&self, version: &DataVersion) -> Option<&Installed> {
        self.installed.iter().find(|i| i.version == *version)
    }

    /// The data to start with (the module's start order), and a notice
    /// when the remembered choice gave way. `None` when there is nothing to
    /// load (no folder named, nothing remembered, nothing installed).
    pub fn start(
        &mut self,
        env: Option<&Path>,
    ) -> (Option<Result<Arc<GameData>, AppError>>, Option<AppError>) {
        if let Some(dir) = env {
            return (Some(self.load(dir)), None);
        }
        let settings = Settings::load(&self.settings_path());
        let remembered = if let Some(id) = &settings.bundle_id {
            Some(self.use_id_quiet(id))
        } else {
            settings.bundle.as_deref().map(|dir| self.load(dir))
        };
        let newest = self.installed.first().map(|i| i.dir.clone());
        match remembered {
            Some(Ok(d)) => (Some(Ok(d)), None),
            Some(Err(e)) => match newest {
                Some(dir) => match self.load(&dir) {
                    Ok(d) => {
                        let notice = AppError::new(
                            Code::BundleFallback,
                            format!("the remembered game data did not load ({})", e.code),
                        );
                        (Some(Ok(d)), Some(notice))
                    }
                    // Both failed: the remembered one's error stands.
                    Err(_) => (Some(Err(e)), None),
                },
                None => (Some(Err(e)), None),
            },
            None => (newest.map(|dir| self.load(&dir)), None),
        }
    }

    fn use_id_quiet(&mut self, id: &str) -> Result<Arc<GameData>, AppError> {
        let dir = self
            .installed
            .iter()
            .find(|i| i.id == id)
            .map(|i| i.dir.clone())
            .ok_or_else(|| AppError::new(Code::NoBundle, format!("no installed bundle {id}")))?;
        self.load(&dir)
    }

    /// Loads an installed bundle by ID and remembers it.
    ///
    /// # Errors
    /// `APP_NO_BUNDLE`, `APP_BUNDLE`.
    pub fn use_id(&mut self, id: &str) -> Result<Arc<GameData>, AppError> {
        // A bundle installed since the start is found too.
        if self.installed.iter().all(|i| i.id != id) {
            self.installed = installed(&self.data_dir);
        }
        let data = self.use_id_quiet(id)?;
        Settings::update(&self.settings_path(), |s| {
            s.bundle_id = Some(id.to_owned());
            s.bundle = None;
        });
        Ok(data)
    }

    /// Loads a folder the user picked and, when it loads, remembers it.
    ///
    /// # Errors
    /// `APP_BUNDLE`.
    pub fn use_dir(&mut self, dir: &Path) -> Result<Arc<GameData>, AppError> {
        // A picked folder is read again: a development folder may have been
        // rebuilt since it was loaded (the old copy stays leaked; a pick is
        // rare and explicit).
        self.loaded.remove(&cache_key(dir));
        let data = self.load(dir)?;
        let id = self.id_of(dir).map(str::to_owned);
        Settings::update(&self.settings_path(), |s| {
            s.bundle = id.is_none().then(|| dir.to_owned());
            s.bundle_id = id;
        });
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_domain::Game;

    /// A fresh folder under the system's temp folder, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn tempdir(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("loom-bundles-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn manifest(dir: &Path, game: &str, schema: u32, steam: Option<u64>) {
        std::fs::create_dir_all(dir).unwrap();
        let m = serde_json::json!({
            "format": 2,
            "data_schema": schema,
            "game": game,
            "source": {
                "steam_build": steam,
                "index_sha256": "",
                "extractor_commit": "",
                "decoder": "",
                "dat_schema": ""
            },
            "files": [],
            "signatures": []
        });
        std::fs::write(dir.join(MANIFEST), m.to_string()).unwrap();
    }

    #[test]
    fn installed_should_list_bundles_newest_first() {
        let tmp = tempdir("list");
        let root = tmp.path().join(BUNDLES_DIR);
        manifest(&root.join("poe1/100-s3"), "poe1", 3, Some(100));
        manifest(&root.join("poe1/200-s3"), "poe1", 3, Some(200));
        manifest(&root.join("poe1/200-s4"), "poe1", 4, Some(200));
        // Unreadable manifest: skipped.
        std::fs::create_dir_all(root.join("poe1/broken")).unwrap();
        std::fs::write(root.join("poe1/broken").join(MANIFEST), "{").unwrap();
        // A duplicate ID under another folder name: the first by path stays.
        manifest(&root.join("poe1/zz-copy"), "poe1", 3, Some(100));
        let list = installed(tmp.path());
        let ids: Vec<&str> = list.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["poe1/200-s4", "poe1/200-s3", "poe1/100-s3"]);
        assert_eq!(list[2].dir, root.join("poe1/100-s3"));
        assert_eq!(list[0].version.game, Game::Poe1);
    }

    #[test]
    fn installed_should_be_empty_without_a_bundles_folder() {
        let tmp = tempdir("empty");
        assert!(installed(tmp.path()).is_empty());
    }

    #[test]
    fn bundle_id_should_name_game_build_and_schema() {
        let v = DataVersion {
            game: Game::Poe1,
            schema_version: 4,
            steam_build: Some(26_530_219),
        };
        assert_eq!(bundle_id(&v), "poe1/26530219-s4");
        let none = DataVersion {
            steam_build: None,
            ..v
        };
        assert_eq!(bundle_id(&none), "poe1/unknown-s4");
    }

    #[test]
    fn settings_should_read_the_shells_older_file() {
        let tmp = tempdir("settings");
        let path = tmp.path().join(SETTINGS_FILE);
        std::fs::write(&path, r#"{"bundle":"D:/data/bundle"}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.bundle, Some(PathBuf::from("D:/data/bundle")));
        assert_eq!(s.bundle_id, None);
        Settings::update(&path, |s| s.mcp_port = Some(4321));
        Settings::update(&path, |s| {
            s.bundle_id = Some("poe1/1-s4".to_owned());
            s.bundle = None;
        });
        assert_eq!(
            std::fs::read_to_string(&path).unwrap().trim(),
            "{\n  \"bundle-id\": \"poe1/1-s4\",\n  \"mcp-port\": 4321\n}",
            "an update keeps the other settings"
        );
    }

    #[test]
    fn start_should_take_nothing_when_nothing_is_there() {
        let tmp = tempdir("nothing");
        let mut lib = Library::new(tmp.path());
        let (data, notice) = lib.start(None);
        assert!(data.is_none());
        assert!(notice.is_none());
    }

    #[test]
    fn use_id_should_refuse_an_unknown_id() {
        let tmp = tempdir("unknown");
        let mut lib = Library::new(tmp.path());
        let e = lib.use_id("poe1/1-s4").unwrap_err();
        assert_eq!(e.code, "APP_NO_BUNDLE");
        assert!(!tmp.path().join(SETTINGS_FILE).exists());
    }
}
