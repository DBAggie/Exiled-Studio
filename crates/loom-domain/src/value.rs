//! Typed values that keep the text they were read from (Phase 2 plan,
//! decision 2). An exporter writes the source text back while the value is
//! unchanged, so a round-trip does not depend on how numbers or lists are
//! formatted. Changing the value drops the source text; the value is then
//! rendered fresh.

/// A number as the calculations use it (ADR-003: f64 inside).
///
/// Equality compares the bit patterns, so it is total: `NaN == NaN` when
/// the bits agree, and `0.0 != -0.0` (so a rewrite from `0` to `-0`
/// compares unequal, as its text would). A value read from text is compared
/// through its [`Sourced`] text as well.
#[derive(Debug, Clone, Copy)]
pub struct Number(pub f64);

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Number {}

/// A typed value and the text it was read from, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sourced<T> {
    value: T,
    source: Option<String>,
}

impl<T> Sourced<T> {
    /// A value read from `source`.
    #[must_use]
    pub fn read(value: T, source: impl Into<String>) -> Self {
        Self {
            value,
            source: Some(source.into()),
        }
    }

    /// A value made in the app, with no source text.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            value,
            source: None,
        }
    }

    /// The value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// The text the value was read from, while the value is unchanged.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Replaces the value and drops the source text.
    pub fn set(&mut self, value: T) {
        self.value = value;
        self.source = None;
    }
}
