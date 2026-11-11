//! Sequential migrations of the build document (ADR-005, ADR-009; Phase 2
//! plan, decision 14).
//!
//! A migration takes a document of version `N` to `N + 1`. It works on a
//! [`serde_json::Value`], before the document is parsed into the DTOs of
//! any version, so the DTO modules only ever read their own version.
//! [`Migrations::run`] applies the steps from the document's version to
//! the current one in order and returns a [`MigrationReport`] naming what
//! each step changed.
//!
//! Phase 2 ships only v1, so [`Migrations::shipped`] has no steps; tests
//! build their own chains with [`Migrations::new`] to exercise the
//! framework (a synthetic v0 to v1).
//!
//! **Errors (`MIGRATE_*`, plan decision 15).** The version is read first,
//! so a document newer than this program gives `MIGRATE_TOO_NEW` before
//! anything else of it is read. A document that is not a build document
//! at all (no `format`, another format, no numeric `version`) is not this
//! layer's to judge: [`crate::dto::read`] hands it to the DTO reader,
//! which gives its `DTO_*` code. No layer wraps another's code.

use serde::Serialize;
use serde_json::Value;

use crate::dto::v1;

/// The document version this program reads and writes.
pub const CURRENT: u64 = v1::VERSION as u64;

/// One step: changes a document of version `from` into one of version
/// `from + 1`, and names each change in `changes` (the report's text).
/// The framework sets the new `version`; the step must not.
///
/// # Errors
/// The step returns why it could not apply (the document is not the shape
/// its version had); [`Migrations::run`] gives it as `MIGRATE_STEP`.
pub type Step = fn(document: &mut Value, changes: &mut Vec<String>) -> Result<(), String>;

/// A migration from version `from` to `from + 1`.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// The version the step reads.
    pub from: u64,
    /// The step.
    pub apply: Step,
}

/// Why a document did not migrate.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MigrateError {
    /// The document is newer than this program reads.
    #[error("MIGRATE_TOO_NEW: version {version}, this program reads up to {current}")]
    TooNew {
        /// The document's version.
        version: u64,
        /// The newest version this program reads.
        current: u64,
    },
    /// The document is older than the oldest version a migration reads.
    #[error("MIGRATE_TOO_OLD: version {version}, the oldest readable is {oldest}")]
    TooOld {
        /// The document's version.
        version: u64,
        /// The oldest version a migration reads.
        oldest: u64,
    },
    /// A step could not apply.
    #[error("MIGRATE_STEP: version {from} to {}: {reason}", from + 1)]
    Step {
        /// The version the step reads.
        from: u64,
        /// Why it could not apply.
        reason: String,
    },
    /// The steps do not form one chain ending at the current version (a
    /// programming error, caught by [`Migrations::new`]).
    #[error("MIGRATE_CHAIN: {0}")]
    Chain(String),
}

impl MigrateError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::TooNew { .. } => "MIGRATE_TOO_NEW",
            Self::TooOld { .. } => "MIGRATE_TOO_OLD",
            Self::Step { .. } => "MIGRATE_STEP",
            Self::Chain(_) => "MIGRATE_CHAIN",
        }
    }
}

/// What a step changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepReport {
    /// The version the step read.
    pub from: u64,
    /// The version it wrote.
    pub to: u64,
    /// Each change, in the order the step made them.
    pub changes: Vec<String>,
}

/// What migrating a document did: the versions, and each step in order.
/// A document already at the current version has no steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MigrationReport {
    /// The document's version as read.
    pub from: u64,
    /// The version after migrating.
    pub to: u64,
    /// The steps applied, oldest first.
    pub steps: Vec<StepReport>,
}

impl MigrationReport {
    /// Whether the document was already current.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

/// A chain of migrations ending at a current version.
#[derive(Debug, Clone)]
pub struct Migrations {
    steps: Vec<Migration>,
    current: u64,
}

impl Migrations {
    /// The migrations this program ships, ending at [`CURRENT`]. Phase 2
    /// ships only v1, so there are none.
    #[must_use]
    pub const fn shipped() -> Self {
        Self {
            steps: Vec::new(),
            current: CURRENT,
        }
    }

    /// A chain of `steps` ending at `current`. A chain a program loads
    /// with ends at [`CURRENT`], the version [`crate::dto::read_with`]
    /// parses; tests use other versions to exercise the framework alone.
    ///
    /// # Errors
    /// `MIGRATE_CHAIN` unless the steps, in the order given, read
    /// consecutive versions and the last one writes `current`.
    pub fn new(steps: Vec<Migration>, current: u64) -> Result<Self, MigrateError> {
        let mut expected = None;
        for step in &steps {
            if let Some(expected) = expected
                && step.from != expected
            {
                return Err(MigrateError::Chain(format!(
                    "a step reads version {}, the one before writes {expected}",
                    step.from
                )));
            }
            expected = Some(step.from.checked_add(1).ok_or_else(|| {
                MigrateError::Chain(format!("a step reads version {}", step.from))
            })?);
        }
        if let Some(last) = expected
            && last != current
        {
            return Err(MigrateError::Chain(format!(
                "the last step writes version {last}, the current one is {current}"
            )));
        }
        Ok(Self { steps, current })
    }

    /// The newest version this chain reads.
    #[must_use]
    pub const fn current(&self) -> u64 {
        self.current
    }

    /// The oldest version this chain reads.
    #[must_use]
    pub fn oldest(&self) -> u64 {
        self.steps.first().map_or(self.current, |s| s.from)
    }

    /// Migrates `document`, whose `version` is `version`, to the current
    /// version. The version is checked before any step runs. After each
    /// step the document's `version` is set to the step's target.
    ///
    /// # Errors
    /// `MIGRATE_TOO_NEW` for a version past [`Self::current`],
    /// `MIGRATE_TOO_OLD` for one before [`Self::oldest`], `MIGRATE_STEP`
    /// when a step fails (the document is then dropped, never half
    /// migrated).
    pub fn run(
        &self,
        mut document: Value,
        version: u64,
    ) -> Result<(Value, MigrationReport), MigrateError> {
        if version > self.current {
            return Err(MigrateError::TooNew {
                version,
                current: self.current,
            });
        }
        if version < self.oldest() {
            return Err(MigrateError::TooOld {
                version,
                oldest: self.oldest(),
            });
        }
        let mut report = MigrationReport {
            from: version,
            to: version,
            steps: Vec::new(),
        };
        // `new` checked the order: the steps from `version` on, in turn.
        for step in self.steps.iter().skip_while(|s| s.from < version) {
            let mut changes = Vec::new();
            (step.apply)(&mut document, &mut changes).map_err(|reason| MigrateError::Step {
                from: step.from,
                reason,
            })?;
            let to = step.from.saturating_add(1);
            let Some(object) = document.as_object_mut() else {
                return Err(MigrateError::Step {
                    from: step.from,
                    reason: "the step did not leave an object".to_owned(),
                });
            };
            object.insert("version".to_owned(), Value::from(to));
            report.steps.push(StepReport {
                from: step.from,
                to,
                changes,
            });
            report.to = to;
        }
        Ok((document, report))
    }
}

impl Default for Migrations {
    fn default() -> Self {
        Self::shipped()
    }
}

/// The version of a build document: `Some` when `document` is an object
/// whose `format` is [`v1::FORMAT`] and whose `version` is a whole number
/// that fits `u64`. Anything else is not a build document this layer can
/// migrate; the DTO reader gives its code.
#[must_use]
pub fn document_version(document: &Value) -> Option<u64> {
    let object = document.as_object()?;
    if object.get("format")?.as_str()? != v1::FORMAT {
        return None;
    }
    object.get("version")?.as_u64()
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;
