//! The application layer (ADR-004; Phase 4 plan): the command, query, and
//! event API over one build, a single writer, background calculation,
//! history, autosave, and MCP change requests. Host crate: it may use the
//! filesystem, threads, and clocks; the core crates it drives may not.
//!
//! - [`Session`]: the synchronous core (state, history, files).
//! - [`App`]: the running application (writer and worker threads).
//! - [`GameData`]: the bundle the application calculates with.
//!
//! Its projections, commands, and events are the API DTOs of
//! `loom_serialization::dto::app::v1`.

mod app;
mod data;
mod edit;
mod error;
mod session;
mod view;

pub use app::{App, Config, HostCommand, MAX_PENDING_REQUESTS, Sink};
pub use data::{GameData, TREE_VERSIONS, tree_version};
pub use error::{AppError, Code};
pub use session::{HISTORY_LIMIT, Session, State, calculate, check_data, preview};
pub use view::{build_view, item_view};

pub use loom_serialization::dto::app::v1 as api;
