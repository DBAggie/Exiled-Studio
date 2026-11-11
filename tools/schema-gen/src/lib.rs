//! The JSON Schemas of the successor documents, generated from their DTOs
//! (ADR-005; Phase 2 plan, decision 11). The DTOs are the source of
//! truth: the committed schema files are this crate's output, and the UI's
//! TypeScript types are generated from them (`pnpm generate:schema` in
//! `ui/`). CI checks both are fresh.

use loom_serialization::dto::v1;

/// Every generated schema: its file name under `schemas/` and its text.
///
/// # Errors
/// As [`build_v1`].
pub fn schemas() -> Result<Vec<(&'static str, String)>, serde_json::Error> {
    Ok(vec![("build.v1.schema.json", build_v1()?)])
}

/// The schema of build document v1, as pretty JSON with a final newline.
///
/// # Errors
/// When the schema does not serialize (it always does).
pub fn build_v1() -> Result<String, serde_json::Error> {
    let mut schema = schemars::schema_for!(v1::Document);
    schema.insert("title".to_owned(), "Build document v1".into());
    // Key order depends on `serde_json`'s `preserve_order` feature (sorted
    // without it), which feature unification could switch; this crate's
    // own build is what writes and checks the file, so it is stable.
    let mut out = serde_json::to_string_pretty(&schema)?;
    out.push('\n');
    Ok(out)
}
