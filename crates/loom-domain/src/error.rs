//! Domain errors, with stable codes (ADR-014).

use core::fmt;

use crate::game::Game;

/// A domain invariant was broken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// Game-specific choices of another game than the build's.
    GameMismatch {
        /// The build's game.
        build: Game,
        /// The choices' game.
        choices: Game,
    },
    /// A split-section link is broken: the line at this index of an item's
    /// lines continues no valid first line, or is a first line not
    /// continued exactly once.
    BrokenContinuation {
        /// Index of the line.
        line: usize,
    },
}

impl DomainError {
    /// Stable code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::GameMismatch { .. } => "DOMAIN_GAME_MISMATCH",
            Self::BrokenContinuation { .. } => "DOMAIN_BROKEN_CONTINUATION",
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GameMismatch { build, choices } => write!(
                f,
                "{}: a {} build cannot hold {} choices",
                self.code(),
                build.name(),
                choices.name()
            ),
            Self::BrokenContinuation { line } => {
                write!(f, "{}: item line {line}", self.code())
            }
        }
    }
}

impl core::error::Error for DomainError {}
