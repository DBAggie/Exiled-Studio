//! Path of Exile only choices (ADR-001). Nothing outside this module and
//! [`crate::GameSpecific::Poe1`] names them; a test scans the crate for it.

/// Choices only Path of Exile has. Values are game-data names, kept as
/// written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Poe1Choices {
    /// The bandit choice (`None` in the data's own sense is a value too,
    /// written as the data writes it); `None` when the build has none.
    pub bandit: Option<String>,
    /// The major pantheon god.
    pub pantheon_major: Option<String>,
    /// The minor pantheon god.
    pub pantheon_minor: Option<String>,
}
