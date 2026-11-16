//! Steam build to patch (ADR-018, "Patch label"): bundles name their Steam
//! build, not a patch number. One row per extracted build, set by the
//! maintainer at each data run (`patches.json` beside this crate).

use loom_domain::Game;
use serde::Deserialize;

const TABLE: &str = include_str!("../patches.json");

/// One row of the table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PatchRow {
    /// `poe1` or `poe2`.
    pub game: String,
    /// The Steam build.
    pub steam_build: u64,
    /// The patch, e.g. `3.30.0`.
    pub patch: String,
    /// The challenge league of that patch.
    pub league: String,
    /// Who set the row, and when.
    pub set_by: String,
}

/// The table.
///
/// # Errors
/// The compiled-in table does not parse (a test guards it).
pub fn table() -> Result<Vec<PatchRow>, serde_json::Error> {
    serde_json::from_str(TABLE)
}

/// The patch of a game's Steam build: `unknown` when the build is not in the
/// table or the bundle has none.
#[must_use]
pub fn patch_of(game: Game, steam_build: Option<u64>) -> String {
    let Some(build) = steam_build else {
        return UNKNOWN.to_owned();
    };
    table()
        .unwrap_or_default()
        .into_iter()
        .find(|r| r.game == game.name() && r.steam_build == build)
        .map_or_else(|| UNKNOWN.to_owned(), |r| r.patch)
}

/// The patch label of a build the table does not know.
pub const UNKNOWN: &str = "unknown";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_should_parse_and_name_the_3_30_build() {
        assert!(table().is_ok_and(|t| !t.is_empty()));
        assert_eq!(patch_of(Game::Poe1, Some(26_530_219)), "3.30.0");
    }

    #[test]
    fn an_unknown_build_or_game_should_be_unknown() {
        assert_eq!(patch_of(Game::Poe1, Some(1)), UNKNOWN);
        assert_eq!(patch_of(Game::Poe2, Some(26_530_219)), UNKNOWN);
        assert_eq!(patch_of(Game::Poe1, None), UNKNOWN);
    }
}
