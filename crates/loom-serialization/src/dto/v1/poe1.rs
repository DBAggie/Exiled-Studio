//! Path of Exile only DTOs (ADR-001): the PoE1 game-specific choices.

use serde::{Deserialize, Serialize};

/// Path of Exile choices, as the game data names them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Poe1Choices {
    /// The bandit choice, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandit: Option<String>,
    /// The major pantheon god, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pantheon_major: Option<String>,
    /// The minor pantheon god, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pantheon_minor: Option<String>,
}

impl From<&loom_domain::poe1::Poe1Choices> for Poe1Choices {
    fn from(c: &loom_domain::poe1::Poe1Choices) -> Self {
        Self {
            bandit: c.bandit.clone(),
            pantheon_major: c.pantheon_major.clone(),
            pantheon_minor: c.pantheon_minor.clone(),
        }
    }
}

impl From<Poe1Choices> for loom_domain::poe1::Poe1Choices {
    fn from(c: Poe1Choices) -> Self {
        Self {
            bandit: c.bandit,
            pantheon_major: c.pantheon_major,
            pantheon_minor: c.pantheon_minor,
        }
    }
}
