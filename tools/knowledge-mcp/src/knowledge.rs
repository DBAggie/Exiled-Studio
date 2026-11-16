//! The bundle the server answers from, and its provenance (ADR-018, "Data it
//! serves").

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use loom_bundles::{DirSource, Status};
use loom_data::bundle::Bundle;
use loom_data::bundle::validate::{self, Integrity};
use loom_data::signature::TrustList;
use loom_domain::Game;
use loom_parser::resolve::Index;
use serde::Serialize;

use crate::patches;
use crate::snapshots::Snapshots;

/// Where the bundle comes from.
#[derive(Debug, Clone)]
pub enum Source {
    /// The newest installed bundle of the game under `<data_dir>/bundles/`
    /// whose status is [`Status::Trusted`].
    Installed(PathBuf),
    /// A folder chosen for development: loaded where it is, without a
    /// signature check, labelled unsigned.
    Folder(PathBuf),
}

/// Why no bundle is served.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// No installed bundle of the game is signed by a trusted key.
    #[error("no installed {0} bundle signed by a trusted key")]
    NoBundle(&'static str),
    /// The bundle does not validate (its files' hashes included).
    #[error("the bundle is not valid ({0})")]
    Invalid(String),
    /// The bundle is another game's.
    #[error("the bundle is {found}, not {wanted}")]
    Game {
        /// The game asked for.
        wanted: &'static str,
        /// The bundle's.
        found: &'static str,
    },
}

impl OpenError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NoBundle(_) => "KNOWLEDGE_NO_BUNDLE",
            Self::Invalid(_) => "KNOWLEDGE_BUNDLE_INVALID",
            Self::Game { .. } => "KNOWLEDGE_GAME",
        }
    }
}

/// What every response carries about the data it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Provenance {
    /// `poe1` or `poe2`.
    pub game: &'static str,
    /// The patch ([`patches`]); `unknown` when the table has no row.
    pub patch: String,
    /// The bundle's Steam build, if it has one.
    pub steam_build: Option<u64>,
    /// SHA-256 of `bundle.json`, lowercase hex.
    pub manifest_sha256: String,
    /// SHA-256 of the install's bundle index the data came from.
    pub index_sha256: String,
    /// Signed by a trusted key (an installed bundle); `false` for a
    /// development folder.
    pub signed: bool,
}

/// A loaded bundle with its provenance.
pub struct Loaded {
    /// The bundle (leaked: one per process).
    pub bundle: &'static Bundle,
    /// Its provenance.
    pub provenance: Provenance,
    indexes: RefCell<BTreeMap<String, Index<'static>>>,
}

impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loaded")
            .field("provenance", &self.provenance)
            .finish_non_exhaustive()
    }
}

impl Loaded {
    /// Runs `f` with the resolver index of a stat-description scope root,
    /// built on first use.
    pub(crate) fn with_index<T>(
        &self,
        root: &str,
        f: impl FnOnce(&Index<'static>) -> T,
    ) -> Option<T> {
        let mut map = self.indexes.borrow_mut();
        if !map.contains_key(root) {
            let scope = self.bundle.stat_descriptions.scope(root).ok()?;
            map.insert(root.to_owned(), Index::build_with(&scope, self.bundle));
        }
        map.get(root).map(f)
    }
}

/// The server's state: the game asked for, the bundle (or why there is
/// none), and the snapshots.
#[derive(Debug)]
pub struct Knowledge {
    /// The game the session works on (`--game`).
    pub game: Game,
    /// The bundle, or why none is served.
    pub data: Result<Loaded, OpenError>,
    /// The economy snapshots.
    pub snapshots: Snapshots,
}

/// Opens the bundle of `game` from `source`.
///
/// # Errors
/// [`OpenError`].
pub fn open(game: Game, source: &Source, trust: &TrustList) -> Result<Loaded, OpenError> {
    let (dir, signed) = match source {
        Source::Folder(dir) => (dir.clone(), false),
        Source::Installed(data_dir) => {
            let found = loom_bundles::scan(data_dir, trust)
                .into_iter()
                .find(|f| f.version.game == game && matches!(f.status, Status::Trusted { .. }))
                .ok_or(OpenError::NoBundle(game.name()))?;
            (found.dir, true)
        }
    };
    load(game, &dir, signed)
}

fn load(game: Game, dir: &Path, signed: bool) -> Result<Loaded, OpenError> {
    // Every file's hash is checked: `scan` checks sizes only (ADR-018,
    // "File integrity"). Codes only: a message could name a path.
    let bundle = validate::check(&DirSource(dir), Integrity::Hashes)
        .map_err(|report| OpenError::Invalid(report.codes().join(", ")))?;
    if bundle.game != game {
        return Err(OpenError::Game {
            wanted: game.name(),
            found: bundle.game.name(),
        });
    }
    let manifest_sha256 = loom_bundles::read_capped(
        &dir.join(loom_data::bundle::MANIFEST),
        loom_data::pack::MAX_SMALL_FILE,
    )
    .map(|b| loom_bundles::sha256_hex(&b))
    .map_err(|_| OpenError::Invalid("BUNDLE_MANIFEST".to_owned()))?;
    let steam_build = bundle.manifest.source.steam_build;
    let provenance = Provenance {
        game: game.name(),
        patch: patches::patch_of(game, steam_build),
        steam_build,
        manifest_sha256,
        index_sha256: bundle.manifest.source.index_sha256.clone(),
        signed,
    };
    Ok(Loaded {
        bundle: Box::leak(Box::new(bundle)),
        provenance,
        indexes: RefCell::new(BTreeMap::new()),
    })
}
