//! Failures that stop a calculation (ADR-014; Phase 3 plan, decision 14).
//! Per-field limits are not errors: they are field states.

use loom_domain::DataVersion;

use crate::graph::GraphError;

/// Why a calculation stopped.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CalcError {
    /// The build records no data version (ADR-010: a build names the data
    /// it was made with; import records it).
    #[error("the build records no data version")]
    NoDataVersion,
    /// The build's data version, or its game, is not the data's.
    #[error("the build's data version {build:?} is not the data's {data:?}")]
    DataVersionMismatch {
        /// The build's.
        build: DataVersion,
        /// The data's.
        data: DataVersion,
    },
    /// The build has no main skill to calculate.
    #[error("the build has no main skill")]
    NoMainSkill,
    /// A data row the calculation needs is missing.
    #[error("missing data row: {table} {key}")]
    MissingData {
        /// The table.
        table: &'static str,
        /// The row's key.
        key: String,
    },
    /// The host's cancellation hook asked the calculation to stop.
    #[error("cancelled")]
    Cancelled,
    /// The provenance graph refused a node (an engine bug).
    #[error("provenance graph: {0}")]
    Graph(#[from] GraphError),
}

impl CalcError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NoDataVersion => "CALC_NO_DATA_VERSION",
            Self::DataVersionMismatch { .. } => "CALC_DATA_VERSION",
            Self::NoMainSkill => "CALC_NO_MAIN_SKILL",
            Self::MissingData { .. } => "CALC_MISSING_DATA",
            Self::Cancelled => "CALC_CANCELLED",
            Self::Graph(_) => "CALC_GRAPH",
        }
    }
}

#[cfg(test)]
mod tests {
    use loom_domain::Game;

    use super::*;

    #[test]
    fn codes_should_be_stable_and_distinct() {
        let v = DataVersion {
            game: Game::Poe1,
            schema_version: 2,
            steam_build: None,
        };
        let all = [
            CalcError::NoDataVersion,
            CalcError::DataVersionMismatch {
                build: v.clone(),
                data: v,
            },
            CalcError::NoMainSkill,
            CalcError::MissingData {
                table: "t",
                key: "k".into(),
            },
            CalcError::Cancelled,
            CalcError::Graph(GraphError::Full),
        ];
        let mut codes: Vec<_> = all.iter().map(CalcError::code).collect();
        assert!(codes.iter().all(|c| c.starts_with("CALC_")));
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len());
        assert_eq!(CalcError::Cancelled.to_string(), "cancelled");
    }
}
