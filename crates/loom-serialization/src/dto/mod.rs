//! The successor JSON build document (ADR-005): versioned DTOs, kept
//! separate from the domain types (ADR-003), and the deterministic
//! conversion between them (Phase 2 plan, step 7).
//!
//! Each format version is its own module. A document is read by the
//! module of its version; migrations ([`crate::migrate`], step 9) bring an
//! older document to the current version before it is parsed.
//!
//! [`read`] and [`write`] are the entry points for any host: they read a
//! document of any supported version and write the current one.
//!
//! [`calc`] holds the calc result document (Phase 3 plan, step 13), by
//! version.

pub mod app;
pub mod calc;
pub mod v1;

use loom_domain::Build;

use crate::migrate::{self, MigrateError, MigrationReport, Migrations};

/// A build read from a document, with what migrating it did.
#[derive(Debug)]
pub struct Loaded {
    /// The build.
    pub build: Build,
    /// The migration steps applied; empty when the document was current.
    pub report: MigrationReport,
}

/// Why a document did not read. Each layer keeps its own code (plan
/// decision 15): [`ReadError::code`] is the code of the layer that failed.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// The version is not one this program reads, or a step failed.
    #[error(transparent)]
    Migrate(#[from] MigrateError),
    /// The document is malformed (not JSON, not a build document, or not
    /// its version's shape).
    #[error(transparent)]
    Dto(#[from] v1::DtoError),
}

impl ReadError {
    /// Stable code (ADR-014): the failing layer's own.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Migrate(e) => e.code(),
            Self::Dto(e) => e.code(),
        }
    }
}

/// Reads a build document of any version the shipped migrations reach.
///
/// # Errors
/// As [`read_with`].
pub fn read(json: &str) -> Result<Loaded, ReadError> {
    read_with(json, &Migrations::shipped())
}

/// Reads a build document through `migrations`: the text is parsed to a
/// JSON value, its version read first ([`migrate::document_version`]),
/// the document migrated to the current version, then parsed as v1.
///
/// # Errors
/// - `DTO_JSON` when the text is not JSON; a value that is not a build
///   document's header gives the DTO reader's code (`DTO_JSON`,
///   `DTO_FORMAT`);
/// - `MIGRATE_*` from [`Migrations::run`] (a newer document gives
///   `MIGRATE_TOO_NEW` before anything else of it is read);
/// - the other `DTO_*` codes from [`v1::from_value`] on the migrated
///   document.
pub fn read_with(json: &str, migrations: &Migrations) -> Result<Loaded, ReadError> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(v1::DtoError::from)?;
    let Some(version) = migrate::document_version(&value) else {
        // Not a build document's header: the DTO reader, which checks the
        // same header, names what is wrong.
        let build = v1::from_value(value)?;
        let report = MigrationReport {
            from: migrate::CURRENT,
            to: migrate::CURRENT,
            steps: Vec::new(),
        };
        return Ok(Loaded { build, report });
    };
    let (value, report) = migrations.run(value, version)?;
    let build = v1::from_value(value)?;
    Ok(Loaded { build, report })
}

/// The build as a document of the current version ([`v1::to_json`]).
///
/// # Errors
/// As [`v1::to_json`].
pub fn write(build: &Build) -> Result<String, v1::DtoError> {
    v1::to_json(build)
}
