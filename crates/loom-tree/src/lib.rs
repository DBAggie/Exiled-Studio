//! The passive tree as the editor sees it (passive tree plan, T-e): the
//! graph with node kinds, positions from the bundle's layout constants,
//! and, in [`poe1`], Path of Exile 1's allocation rules and its tree URL.
//!
//! Core crate (ADR-016): no I/O; the host loads the bundle and hands the
//! tables in. The calculation does not depend on this crate; it reads the
//! spec's nodes as before (the plan's resolved passive set in
//! `loom-pipeline` is a later step).
//!
//! Built from the bundle's passive tables ([`Tree::from_tables`]). Which
//! nodes the editor models: the nodes reachable from the class starts
//! without entering an ascendancy, each regular ascendancy's nodes from its
//! start, the masteries linked to such nodes, and the jewel sockets among
//! them. The rest (bloodline and alternate ascendancies, cluster
//! jewels' template nodes, sockets inside cluster areas) is kept in a spec
//! untouched, drawn faded, and never allocated by an edit (plan, T-g).

pub mod graph;
pub mod poe1;

pub use graph::{AscendancyClass, Class, Group, Kind, Node, Tree};
