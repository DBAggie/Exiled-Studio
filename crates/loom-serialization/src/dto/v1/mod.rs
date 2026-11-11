//! Build document v1 (Phase 2 plan, step 7; decision 11).
//!
//! The DTOs in this module are the source of truth for the document's
//! shape: the JSON Schema (`schemas/build.v1.schema.json`) is generated
//! from them by `tools/schema-gen` (the `schema` feature), and the UI's
//! TypeScript types from the schema.
//!
//! **Deterministic.** [`to_json`] gives the same bytes for equal builds on
//! every platform: fields in declaration order, lists in model order, no
//! maps, two-space indentation, `\n` line ends, and a final newline.
//! Finite numbers are written in their shortest round-trip form and read
//! back to the same bits (`serde_json`'s `float_roundtrip`); non-finite
//! ones by their bit pattern. `from_json(to_json(b)) == b` for every
//! build whose item links hold.
//!
//! **Complete.** The whole [`loom_domain::Build`] is kept: opaque content
//! in place, typed values with their source text, item lines with their
//! resolutions (including `adapted`, `continues`, and the reasons of
//! unresolved lines), `Item::text_spacing`, and the data version.

mod convert;
pub mod poe1;
mod types;

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;

use loom_domain::{self as d, DomainError};

pub use types::*;

/// The document's `format` value.
pub const FORMAT: &str = "loom-build";

/// The document version this module reads and writes.
pub const VERSION: u32 = 1;

/// Why a document did not convert.
#[derive(Debug, thiserror::Error)]
pub enum DtoError {
    /// Not JSON, or not a v1 document's shape.
    #[error("DTO_JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// The `format` is not [`FORMAT`].
    #[error("DTO_FORMAT: format {0:?}")]
    Format(String),
    /// The `version` is not [`VERSION`].
    #[error("DTO_VERSION: version {0}")]
    Version(u32),
    /// A domain invariant does not hold (game-specific choices of another
    /// game).
    #[error("DTO_DOMAIN: {0}")]
    Domain(DomainError),
    /// An item's continuation links do not hold.
    #[error("DTO_ITEM_LINKS: item {item}: {error}")]
    Item {
        /// The item's ID.
        item: u32,
        /// The broken link.
        error: DomainError,
    },
    /// An index does not fit this platform's `usize`.
    #[error("DTO_OUT_OF_RANGE: {0}")]
    OutOfRange(u64),
    /// A non-finite number's bits are not 16 lowercase hex digits of a
    /// non-finite value.
    #[error("DTO_NUMBER: f64_bits {0:?}")]
    Number(String),
}

impl DtoError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "DTO_JSON",
            Self::Format(_) => "DTO_FORMAT",
            Self::Version(_) => "DTO_VERSION",
            Self::Domain(_) => "DTO_DOMAIN",
            Self::Item { .. } => "DTO_ITEM_LINKS",
            Self::OutOfRange(_) => "DTO_OUT_OF_RANGE",
            Self::Number(_) => "DTO_NUMBER",
        }
    }
}

impl Document {
    /// The document for `build`.
    ///
    /// # Errors
    /// [`DtoError::Item`] when an item's continuation links do not hold
    /// ([`loom_domain::ItemBody::check_links`]): such a document would not
    /// read back, so it is not written.
    pub fn from_domain(build: &d::Build) -> Result<Self, DtoError> {
        for item in build.items.items() {
            item.body.check_links().map_err(|error| DtoError::Item {
                item: item.id.0,
                error,
            })?;
        }
        Ok(Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            build: convert::build_to(build),
        })
    }

    /// The build.
    ///
    /// # Errors
    /// [`DtoError`] when the format or version is not this module's, or
    /// the content breaks a domain invariant.
    pub fn into_domain(self) -> Result<d::Build, DtoError> {
        if self.format != FORMAT {
            return Err(DtoError::Format(self.format));
        }
        if self.version != VERSION {
            return Err(DtoError::Version(self.version));
        }
        convert::build_from(self.build)
    }
}

/// The build as a v1 JSON document.
///
/// # Errors
/// [`DtoError::Item`] as [`Document::from_domain`].
pub fn to_json(build: &d::Build) -> Result<String, DtoError> {
    let doc = Document::from_domain(build)?;
    let mut out = serde_json::to_string_pretty(&doc)?;
    out.push('\n');
    Ok(out)
}

/// Reads a v1 JSON document.
///
/// # Errors
/// [`DtoError`]: `DTO_JSON` when the text is not a v1 document's shape
/// (unknown fields are refused), the others as
/// [`Document::into_domain`].
pub fn from_json(json: &str) -> Result<d::Build, DtoError> {
    serde_json::from_str::<Document>(json)?.into_domain()
}
