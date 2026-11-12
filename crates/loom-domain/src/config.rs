//! Config inputs, as typed key-value pairs. What each key means is the
//! calculations' concern (Phase 3).

use crate::error::DomainError;
use crate::id::ConfigSetId;
use crate::item::LineResolution;
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

impl Config {
    /// Checks that no custom modifier line continues another line (only
    /// item lines resolve over several lines).
    ///
    /// # Errors
    /// [`DomainError::ContinuedCustomLine`] for the first such line.
    pub fn check_custom_lines(&self) -> Result<(), DomainError> {
        let continued = |lines: &[CustomModifierLine]| {
            lines
                .iter()
                .position(|l| matches!(l.resolution, LineResolution::Continues { .. }))
        };
        for set in self.sets.typed() {
            for (block, b) in set.custom_modifiers.iter().enumerate() {
                if let Some(line) = continued(&b.lines) {
                    return Err(DomainError::ContinuedCustomLine {
                        set: set.id.0,
                        block,
                        line,
                    });
                }
            }
            for input in set.inputs.typed() {
                if let Some(line) = continued(&input.lines) {
                    return Err(DomainError::ContinuedCustomInputLine {
                        set: set.id.0,
                        input: input.name.clone(),
                        line,
                    });
                }
            }
        }
        Ok(())
    }

    /// Every custom modifier line of every set: the blocks' and the
    /// inputs'.
    pub fn custom_lines(&self) -> impl Iterator<Item = &CustomModifierLine> {
        self.sets.typed().flat_map(|s| {
            s.custom_modifiers
                .iter()
                .flat_map(|b| &b.lines)
                .chain(s.inputs.typed().flat_map(|i| &i.lines))
        })
    }
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
    /// Custom modifier blocks, after the inputs (where PoB writes them).
    /// A block anywhere else stays untyped content in `inputs`.
    pub custom_modifiers: Vec<CustomModifierBlock>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// A block of custom modifier lines, which a planner applies as if the
/// character had them when the block is enabled. What a line changes is
/// the calculations' concern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomModifierBlock {
    /// The title, if any.
    pub title: Option<String>,
    /// Whether the block applies.
    pub enabled: bool,
    /// The block's text, as read (whitespace and line breaks included).
    pub text: String,
    /// The modifier lines the import adapter read from `text` (its
    /// format's rules say which lines hold a modifier and with what text),
    /// in order, each with its resolution. They are derived from `text`,
    /// which an export writes: a change to `text` must derive them again
    /// (nothing checks that they agree).
    pub lines: Vec<CustomModifierLine>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// One modifier line of a custom modifier block. Resolution reads it as a
/// character-wide modifier (no item base).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomModifierLine {
    /// The modifier text.
    pub text: String,
    /// What the line resolved to.
    pub resolution: LineResolution,
}

/// One config input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigInput {
    /// The key.
    pub name: String,
    /// The value.
    pub value: ConfigValue,
    /// Whether the value was set or is a placeholder.
    pub kind: ConfigInputKind,
    /// The modifier lines the import adapter read from a text value its
    /// format applies as custom modifier lines (PoB's legacy `customMods`
    /// input), in order, each with its resolution; empty for any other
    /// input. Whether they apply is the calculations' concern (PoB applies
    /// them only when the set has no block with text). Derived from
    /// `value`, as a block's lines are from its text.
    pub lines: Vec<CustomModifierLine>,
    /// Attributes not typed.
    pub attributes: Attributes,
}

/// Whether a config value was set or is a placeholder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConfigInputKind {
    /// A value set for the input.
    #[default]
    Input,
    /// A placeholder: the value a planner shows for an input left unset,
    /// which it may also use for that input (PoB applies a numeric
    /// input's placeholder when the input is unset, or 0 unless the
    /// option is `countAllowZero`). What it
    /// changes is the calculations' concern.
    Placeholder,
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
