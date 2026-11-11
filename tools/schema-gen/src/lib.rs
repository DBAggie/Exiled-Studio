//! The JSON Schemas of the successor documents, generated from their DTOs
//! (ADR-005; Phase 2 plan, decision 11). The DTOs are the source of
//! truth: the committed schema files are this crate's output, and the UI's
//! TypeScript types are generated from them (`pnpm generate:schema` in
//! `ui/`). CI checks both are fresh.

use loom_serialization::dto::v1;

/// Every generated schema: its file name under `schemas/` and its text.
#[must_use]
pub fn schemas() -> Vec<(&'static str, String)> {
    vec![("build.v1.schema.json", build_v1())]
}

/// The schema of build document v1, as pretty JSON with a final newline.
#[must_use]
pub fn build_v1() -> String {
    let mut schema = schemars::schema_for!(v1::Document);
    schema.insert("title".to_owned(), "Build document v1".into());
    // `serde_json::Map` is sorted unless `preserve_order` is on; either
    // way the order is fixed by the code, so the text is deterministic.
    let mut out = serde_json::to_string_pretty(&schema).unwrap_or_default();
    out.push('\n');
    out
}
