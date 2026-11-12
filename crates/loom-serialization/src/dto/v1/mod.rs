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
use serde::Deserialize;

pub use types::*;

/// The document's `format` value.
pub const FORMAT: &str = "loom-build";

/// The document version this module reads and writes.
pub const VERSION: u32 = 1;

/// The deepest array and object nesting [`to_json`] writes. `serde_json`
/// reads up to 127 levels; the margin leaves room for the readers of
/// other hosts. Only deeply nested untyped content can reach it.
pub const MAX_DEPTH: usize = 100;

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
    Version(u64),
    /// The document nests deeper than [`MAX_DEPTH`] (deeply nested untyped
    /// content): a JSON reader would refuse it, so it is not written.
    #[error("DTO_TOO_DEEP: nesting {0} over {MAX_DEPTH}")]
    TooDeep(usize),
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
            Self::TooDeep(_) => "DTO_TOO_DEEP",
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
        build
            .config
            .check_custom_lines()
            .map_err(DtoError::Domain)?;
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
            return Err(DtoError::Version(self.version.into()));
        }
        convert::build_from(self.build)
    }
}

/// The build as a v1 JSON document.
///
/// # Errors
/// [`DtoError::Item`] as [`Document::from_domain`]; [`DtoError::TooDeep`]
/// when the text would nest deeper than [`MAX_DEPTH`].
///
/// Only this output is canonical: a hand-written document that reads to
/// the same build (`1` for `1.0`, other spacing) is written back in this
/// form.
pub fn to_json(build: &d::Build) -> Result<String, DtoError> {
    let doc = Document::from_domain(build)?;
    let mut out = serde_json::to_string_pretty(&doc)?;
    let depth = nesting(&out);
    if depth > MAX_DEPTH {
        return Err(DtoError::TooDeep(depth));
    }
    out.push('\n');
    Ok(out)
}

/// The deepest array and object nesting of JSON text (brackets inside
/// strings do not count).
fn nesting(json: &str) -> usize {
    let (mut depth, mut max, mut in_string, mut escaped) = (0_usize, 0, false, false);
    for b in json.bytes() {
        if in_string {
            match b {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'[' | b'{' => {
                depth += 1;
                max = max.max(depth);
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    max
}

/// The two fields every version has, read before the rest so that a
/// document of another format or version gets its own code, whatever
/// else it holds.
#[derive(serde::Deserialize)]
struct Header {
    format: String,
    version: u64,
}

/// Reads a v1 JSON document.
///
/// # Errors
/// [`DtoError`]: `DTO_JSON` when the text is not JSON or not a v1
/// document's shape (unknown fields are refused), the others as
/// [`from_value`].
pub fn from_json(json: &str) -> Result<d::Build, DtoError> {
    from_value(serde_json::from_str(json)?)
}

/// Reads a v1 document already parsed to a JSON value (the form
/// migrations work on, Phase 2 plan, decision 14). The `format` and
/// `version` are checked first: another format gives `DTO_FORMAT`, another
/// version `DTO_VERSION`, before the rest is read.
///
/// # Errors
/// [`DtoError`] as [`Document::into_domain`], or `DTO_JSON` for a shape
/// that is not v1's.
pub fn from_value(value: serde_json::Value) -> Result<d::Build, DtoError> {
    let header = Header::deserialize(&value)?;
    if header.format != FORMAT {
        return Err(DtoError::Format(header.format));
    }
    if header.version != u64::from(VERSION) {
        return Err(DtoError::Version(header.version));
    }
    Document::deserialize(value)?.into_domain()
}
