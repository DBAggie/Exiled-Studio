//! Config inputs, as typed key-value pairs. What each key means is the
//! calculations' concern (Phase 3).

use crate::id::ConfigSetId;
use crate::opaque::{Attributes, Ordered};
use crate::value::{Number, Sourced};

/// The build's config sets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    /// The active config set, if set.
    pub active: Option<ConfigSetId>,
    /// The sets, with untyped content in place.
    pub sets: Ordered<ConfigSet>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// One config set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSet {
    /// The set's ID within the build.
    pub id: ConfigSetId,
    /// The title, if any.
    pub title: Option<String>,
    /// Inputs, with untyped content in place.
    pub inputs: Ordered<ConfigInput>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// One config input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigInput {
    /// The key.
    pub name: String,
    /// The value.
    pub value: ConfigValue,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// A config value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    /// A flag.
    Boolean(bool),
    /// A number, with its source text.
    Number(Sourced<Number>),
    /// Text.
    String(String),
}
