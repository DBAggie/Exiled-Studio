//! What a calculation reads (ADR-003; Phase 3 plan, decision 7): the
//! resolved build (borrowed, immutable), a view of the game data, and the
//! options.

use loom_data::bundle::Bundle;
use loom_domain::{Build, ConfigSetId, DataVersion, ItemSetId, SpecId};

use crate::error::CalcError;

/// The game data a calculation reads, as `ItemData` is for import. The
/// bundle implements it; the parity tool's data overlay (P3-5) is another
/// implementation, so the core keeps the game's data.
pub trait CalcData {
    /// The data's version: the build's must equal it.
    fn data_version(&self) -> DataVersion;
}

impl CalcData for Bundle {
    fn data_version(&self) -> DataVersion {
        DataVersion {
            game: self.game,
            schema_version: self.manifest.data_schema,
            steam_build: self.manifest.source.steam_build,
        }
    }
}

/// A skill picked by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillChoice {
    /// The socket group's index.
    pub group: usize,
    /// The active skill's index within the group.
    pub active: usize,
}

/// What to calculate. `None` takes the build's own choice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalcOptions {
    /// The skill (else the build's main skill).
    pub skill: Option<SkillChoice>,
    /// The passive spec (else the active one).
    pub spec: Option<SpecId>,
    /// The item set (else the active one).
    pub item_set: Option<ItemSetId>,
    /// The config set (else the active one).
    pub config_set: Option<ConfigSetId>,
}

/// A calculation's input: an immutable snapshot.
#[derive(Clone, Copy)]
pub struct CalcInput<'a> {
    /// The resolved build.
    pub build: &'a Build,
    /// The game data.
    pub data: &'a dyn CalcData,
    /// The options.
    pub options: &'a CalcOptions,
}

impl core::fmt::Debug for CalcInput<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CalcInput")
            .field("game", &self.build.game())
            .field("data", &self.data.data_version())
            .field("options", self.options)
            .finish_non_exhaustive()
    }
}

impl CalcInput<'_> {
    /// Checks that the build was made with this data (ADR-010: a data
    /// update never reinterprets a build silently) and is of its game.
    ///
    /// # Errors
    /// [`CalcError::NoDataVersion`] when the build records none,
    /// [`CalcError::DataVersionMismatch`] when it differs from the data's
    /// or the build's game is not the data's.
    pub fn check_data_version(&self) -> Result<(), CalcError> {
        let data = self.data.data_version();
        let build = self.build.data.as_ref().ok_or(CalcError::NoDataVersion)?;
        if *build != data || self.build.game() != data.game {
            return Err(CalcError::DataVersionMismatch {
                build: build.clone(),
                data,
            });
        }
        Ok(())
    }
}
