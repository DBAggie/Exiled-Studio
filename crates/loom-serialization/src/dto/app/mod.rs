//! The application API's DTOs (Phase 4 plan, decision 9): the projections
//! queries return, the commands the UI and addons send, and the events the
//! application reports (ADR-004). They are versioned like the documents:
//! `schemas/app-api.v1.schema.json` is generated from them and the UI's
//! types from the schema (ADR-002, generated bindings). A shape change
//! that breaks a reader is a new version.
//!
//! They are plain data. The host (`loom-app`) projects the domain onto
//! them; nothing here reads a build.

pub mod v1;
