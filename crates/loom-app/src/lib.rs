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
pub mod bundles;
mod card_values;
pub mod crash;
mod data;
pub mod diagnostics;
mod edit;
mod error;
pub mod gems;
pub mod install;
pub mod items;
pub mod loadout;
pub mod logs;
pub mod main_skill;
mod session;
pub mod tree;
pub mod trust;
mod view;
pub mod what_if;

pub use app::{App, Config, HostCommand, MAX_PENDING_REQUESTS, Sink};
pub use data::{GameData, TREE_VERSIONS, tree_version};
pub use error::{AppError, Code};
pub use session::{
    GroupPlace, HISTORY_LIMIT, Session, State, calculate, check_data, preview, preview_tree,
};
pub use view::{build_view, item_view};
pub use what_if::what_if;

pub use loom_serialization::dto::app::v1 as api;
