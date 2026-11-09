//! Library side of `loom-extractor`: format readers for the Bundles2 game data
//! (Spike A, `dev/active/spike-a-bundles2-extraction/`). Parsers work on bytes;
//! the filesystem and child processes stay at the edges (ADR-005, maintainer-run).
//!
//! Game files are untrusted input: indexing that can panic is linted, and
//! parsers return typed errors instead.

#![warn(clippy::indexing_slicing)]

pub mod bundle;
pub mod crosscheck;
pub mod dat;
pub mod decompress;
pub mod extract;
pub mod hash;
pub mod index;
pub mod install;
mod reader;
pub mod schema;
pub mod statdesc;

pub use reader::Short;
