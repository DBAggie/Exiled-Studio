//! The application's errors (ADR-014): its own `APP_*` codes, and the
//! codes of the layers below unchanged.

use loom_serialization::dto::app::v1::ApiError;

/// The application's own codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// No game data is loaded.
    NoData,
    /// The bundle folder is not a valid bundle.
    Bundle,
    /// The build was made with another data version.
    DataVersion,
    /// The build is of the other game than the data.
    GameMismatch,
    /// No build is open.
    NoBuild,
    /// An edit's lines are not the ones it expects.
    StaleEdit,
    /// An edit names an item that does not exist.
    EditItem,
    /// An edit names a line that does not exist or cannot be edited so.
    EditLine,
    /// A roll outside 0 to 1, or a line without a range.
    Range,
    /// A file could not be read.
    Read,
    /// A recent build or recovery snapshot ID is unknown.
    NotFound,
    /// The export would write over the file the build was imported from.
    ExportOrigin,
    /// A change request ID is unknown, or the request is not pending.
    Request,
    /// MCP clients may not request changes (the permission is off).
    RequestsOff,
    /// Too many change requests are pending.
    RequestsFull,
    /// The build has no path to save to.
    NoPath,
    /// The application has stopped.
    Stopped,
    /// The build's stored resolutions differ from this bundle's (a
    /// notice, not an error).
    ResolutionChanged,
}

impl Code {
    /// The stable code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoData => "APP_NO_DATA",
            Self::Bundle => "APP_BUNDLE",
            Self::DataVersion => "APP_DATA_VERSION",
            Self::GameMismatch => "APP_GAME_MISMATCH",
            Self::NoBuild => "APP_NO_BUILD",
            Self::StaleEdit => "APP_STALE_EDIT",
            Self::EditItem => "APP_EDIT_ITEM",
            Self::EditLine => "APP_EDIT_LINE",
            Self::Range => "APP_RANGE",
            Self::Read => "APP_READ",
            Self::NotFound => "APP_NOT_FOUND",
            Self::ExportOrigin => "APP_EXPORT_ORIGIN",
            Self::Request => "APP_REQUEST",
            Self::RequestsOff => "APP_REQUESTS_OFF",
            Self::RequestsFull => "APP_REQUESTS_FULL",
            Self::NoPath => "APP_NO_PATH",
            Self::Stopped => "APP_STOPPED",
            Self::ResolutionChanged => "APP_RESOLUTION_CHANGED",
        }
    }
}

/// An error with its code: the application's or a lower layer's.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AppError {
    /// The stable code.
    pub code: String,
    /// Detail in English, for diagnostics.
    pub message: String,
}

impl AppError {
    /// An error with one of the application's codes.
    #[must_use]
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        Self {
            code: code.as_str().to_owned(),
            message: message.into(),
        }
    }

    /// An error with a lower layer's code.
    #[must_use]
    pub fn layer(code: &str, message: impl ToString) -> Self {
        Self {
            code: code.to_owned(),
            message: message.to_string(),
        }
    }

    /// The API DTO.
    #[must_use]
    pub fn dto(&self) -> ApiError {
        ApiError::new(self.code.clone(), self.message.clone())
    }
}

impl From<loom_pipeline::Error> for AppError {
    fn from(e: loom_pipeline::Error) -> Self {
        Self::layer(e.code, &e)
    }
}

impl From<loom_persistence::Error> for AppError {
    fn from(e: loom_persistence::Error) -> Self {
        Self::layer(e.code(), &e)
    }
}

impl From<loom_persistence::PersistError> for AppError {
    fn from(e: loom_persistence::PersistError) -> Self {
        Self::layer(e.code(), &e)
    }
}
