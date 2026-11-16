//! Calc result document v1 (Phase 3 plan, step 13).
//!
//! The DTOs in this module are the source of truth for the document's
//! shape: the JSON Schema (`schemas/calc-result.v1.schema.json`) is
//! generated from them by `tools/schema-gen` (the `schema` feature), and
//! the UI's TypeScript types from the schema.
//!
//! **Deterministic.** [`to_json`] gives the same bytes for equal results
//! on every platform, as build document v1 does: fields in declaration
//! order, lists in the engine's order, no maps, two-space indentation,
//! `\n` line ends, a final newline; finite numbers in their shortest
//! round-trip form, non-finite ones by their bit pattern.
//!
//! **Written from the engine, read as DTOs.** The engine's result names
//! its rules, labels, and reasons by `&'static str`, so a document does
//! not convert back to a [`loom_calc::CalcResult`]; readers (the parity
//! tool, the UI) read the [`Document`] itself ([`from_json`]).
//!
//! **Consistent.** A document is written only when the result's graph
//! holds ([`loom_calc::Graph::check`]: every recorded value is its
//! evaluation, bit for bit) and every computed field's value is its
//! node's. [`from_json`] checks the structure of what it reads
//! ([`Document::check_shape`]: child indices, arity and term per node
//! kind, field nodes and values, reasons per state), so a reader may
//! index the graph without further checks. It does not re-evaluate the
//! graph.
//!
//! **Adding to v1.** Names that grow with the engine (fields, quantities,
//! ops, terms, labels) are strings, so new ones need no change here. A
//! new variant of a closed enum (a reason, a source, a node kind) is
//! added to v1, with the schema and TypeScript regenerated, while every
//! reader is in this repository and no document is kept outside it (until
//! the Phase 3 gate). After that, any shape change is a new version with
//! a migration (Phase 2 plan, decision 14).
//!
//! **Inputs.** The document is the engine's result only. What it was
//! computed from (build, bundle, oracle pin, slice revision) is recorded
//! by whoever runs the engine (P3-5's `calc-run` summary).

mod convert;
mod types;

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;

use loom_calc::GraphError;
use serde::Deserialize;

pub use types::*;

/// The document's `format` value.
pub const FORMAT: &str = "loom-calc-result";

/// The document version this module reads and writes.
pub const VERSION: u32 = 1;

/// Why a result was not written, or a document did not read.
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
    /// The result's graph does not hold: a node names a later child, or a
    /// recorded value is not its evaluation.
    #[error("DTO_CALC_GRAPH: {0}")]
    Graph(GraphError),
    /// A field's value and node disagree: one is set without the other,
    /// the node is not in the graph, or its value is not the field's.
    #[error("DTO_CALC_FIELD: field {0}")]
    Field(String),
    /// A document read breaks the v1 structure (see
    /// [`Document::check_shape`]).
    #[error("DTO_CALC_SHAPE: {0}")]
    Shape(String),
}

impl DtoError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "DTO_JSON",
            Self::Format(_) => "DTO_FORMAT",
            Self::Version(_) => "DTO_VERSION",
            Self::Graph(_) => "DTO_CALC_GRAPH",
            Self::Field(_) => "DTO_CALC_FIELD",
            Self::Shape(_) => "DTO_CALC_SHAPE",
        }
    }
}

impl Document {
    /// The document for `result`.
    ///
    /// # Errors
    /// [`DtoError::Graph`] when the graph does not hold,
    /// [`DtoError::Field`] when a field's value is not its node's.
    pub fn from_result(result: &loom_calc::CalcResult) -> Result<Self, DtoError> {
        result.graph.check().map_err(DtoError::Graph)?;
        for (field, r) in &result.fields {
            let consistent = match (r.value, r.node) {
                (None, None) => true,
                (Some(v), Some(n)) => result
                    .graph
                    .value(n)
                    .is_some_and(|g| g.to_bits() == v.to_bits()),
                _ => false,
            };
            if !consistent {
                return Err(DtoError::Field(field.to_string()));
            }
        }
        Ok(Self {
            format: FORMAT.to_owned(),
            version: VERSION,
            result: convert::result_to(result),
        })
    }
}

impl Document {
    /// Checks the structure a reader relies on: each node's children are
    /// earlier nodes; leaves have no children and no term; `override`,
    /// `min`, `max`, `difference`, `quotient` have two children, `percent-factor`, `floor`,
    /// `round-half-up`, `trunc` one, and every composite but `override` a term;
    /// a field has a value and a node together, its node is in the graph
    /// with the same value bits; `reason` is present exactly for
    /// `unsupported`, `unsupported` is not empty for `partial`,
    /// `indeterminate` is not empty exactly for `indeterminate`, and a
    /// `complete` field has no reason.
    ///
    /// # Errors
    /// [`DtoError::Shape`] naming the first node or field that breaks it.
    pub fn check_shape(&self) -> Result<(), DtoError> {
        let graph = &self.result.graph;
        for (i, n) in graph.iter().enumerate() {
            let bad = |what: &str| Err(DtoError::Shape(format!("node {i}: {what}")));
            if n.children.iter().any(|c| *c as usize >= i) {
                return bad("a child is not an earlier node");
            }
            let (arity, term) = match n.kind {
                NodeKind::Contribution { .. } | NodeKind::Constant { .. } => (Some(0), false),
                NodeKind::Override
                | NodeKind::Min
                | NodeKind::Max
                | NodeKind::Difference
                | NodeKind::Quotient
                | NodeKind::Power => (Some(2), !matches!(n.kind, NodeKind::Override)),
                NodeKind::PercentFactor
                | NodeKind::Floor
                | NodeKind::RoundHalfUp
                | NodeKind::Trunc
                | NodeKind::Ceil => (Some(1), true),
                NodeKind::Sum | NodeKind::Product | NodeKind::Any => (None, true),
            };
            if arity.is_some_and(|a| a != n.children.len()) {
                return bad("wrong number of children");
            }
            if term != n.term.is_some() {
                return bad("term present where none belongs, or missing");
            }
        }
        for f in &self.result.fields {
            let bad = |what: &str| Err(DtoError::Shape(format!("field {}: {what}", f.field)));
            match (&f.value, f.node) {
                (None, None) => {}
                (Some(v), Some(n)) => {
                    let Some(node) = graph.get(n as usize) else {
                        return bad("node not in the graph");
                    };
                    if number_bits(v) != number_bits(&node.value) {
                        return bad("value is not its node's");
                    }
                }
                _ => return bad("value and node must be set together"),
            }
            let ok = match f.state {
                StateKind::Complete => {
                    f.reason.is_none() && f.unsupported.is_empty() && f.indeterminate.is_empty()
                }
                StateKind::Partial => {
                    f.reason.is_none() && !f.unsupported.is_empty() && f.indeterminate.is_empty()
                }
                StateKind::Unsupported => {
                    f.reason.is_some()
                        && f.value.is_none()
                        && f.unsupported.is_empty()
                        && f.indeterminate.is_empty()
                }
                StateKind::Indeterminate => f.reason.is_none() && !f.indeterminate.is_empty(),
            };
            if !ok {
                return bad("reasons do not match the state");
            }
        }
        Ok(())
    }
}

/// A number's bits (a non-finite one's as written).
fn number_bits(n: &crate::dto::v1::Number) -> Option<u64> {
    match n {
        crate::dto::v1::Number::Finite(v) => Some(v.to_bits()),
        crate::dto::v1::Number::NonFinite(b) => u64::from_str_radix(&b.f64_bits, 16).ok(),
    }
}

/// The result as a v1 JSON document.
///
/// # Errors
/// As [`Document::from_result`].
pub fn to_json(result: &loom_calc::CalcResult) -> Result<String, DtoError> {
    let doc = Document::from_result(result)?;
    let mut out = serde_json::to_string_pretty(&doc)?;
    out.push('\n');
    Ok(out)
}

/// The two fields every version has, read before the rest so that a
/// document of another format or version gets its own code.
#[derive(serde::Deserialize)]
struct Header {
    format: String,
    version: u64,
}

/// Reads a v1 document. The `format` and `version` are checked first,
/// then the structure ([`Document::check_shape`]).
///
/// # Errors
/// `DTO_JSON` when the text is not JSON or not a v1 document's shape
/// (unknown fields are refused), `DTO_FORMAT` for another format,
/// `DTO_VERSION` for another version, `DTO_CALC_SHAPE` for a structure
/// a reader cannot rely on.
pub fn from_json(json: &str) -> Result<Document, DtoError> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    let header = Header::deserialize(&value)?;
    if header.format != FORMAT {
        return Err(DtoError::Format(header.format));
    }
    if header.version != u64::from(VERSION) {
        return Err(DtoError::Version(header.version));
    }
    let doc = Document::deserialize(value)?;
    doc.check_shape()?;
    Ok(doc)
}
