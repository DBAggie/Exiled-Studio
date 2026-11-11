//! The game a build or data bundle belongs to (ADR-001), and the choices
//! only one game has.

use crate::poe1::Poe1Choices;

/// A game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Game {
    /// Path of Exile.
    Poe1,
    /// Path of Exile 2.
    Poe2,
}

impl Game {
    /// Stable name (`poe1`, `poe2`), as bundles and reports write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Poe1 => "poe1",
            Self::Poe2 => "poe2",
        }
    }
}

/// Choices that exist in one game only. Shared types never hold them
/// (ADR-001); a [`crate::Build`] keeps the variant of its own game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameSpecific {
    /// Path of Exile choices.
    Poe1(Poe1Choices),
    /// Path of Exile 2 choices (none typed yet).
    Poe2(Poe2Choices),
}

impl GameSpecific {
    /// The empty choices of `game`.
    #[must_use]
    pub fn empty(game: Game) -> Self {
        match game {
            Game::Poe1 => Self::Poe1(Poe1Choices::default()),
            Game::Poe2 => Self::Poe2(Poe2Choices::default()),
        }
    }

    /// The game these choices belong to.
    #[must_use]
    pub const fn game(&self) -> Game {
        match self {
            Self::Poe1(_) => Game::Poe1,
            Self::Poe2(_) => Game::Poe2,
        }
    }
}

/// Path of Exile 2 choices. None is typed yet; the type exists so that a
/// PoE2 build has its own variant.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Poe2Choices {}

/// The data bundle a build was made with (ADR-010): a data update never
/// reinterprets a build silently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataVersion {
    /// The bundle's game.
    pub game: Game,
    /// The bundle's data schema version.
    pub schema_version: u32,
    /// The Steam build the bundle was extracted from, if known.
    pub steam_build: Option<u64>,
}
