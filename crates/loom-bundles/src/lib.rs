//! Installed data bundles on disk, read only (ADR-018; moved out of
//! `loom-app` so the application and the knowledge MCP share one trust
//! rule).
//!
//! - [`status_of`] and [`scan`]: each installed folder's signature checked
//!   against the trust list and its files' sizes against the manifest.
//! - [`verify_full`]: every file's SHA-256 against the manifest.
//! - [`trust_list`]: the data keys compiled into the binary.
//! - [`DirSource`]: a bundle folder as a [`loom_data::bundle::BundleSource`].
//!
//! Nothing here writes, removes, or renames a file. Installing, accepting,
//! and sweeping temporary folders stay in `loom-app` (`install`).

mod source;
mod status;
mod trust;

pub use source::DirSource;
pub use status::{
    ACCEPTED_FILE, AcceptedRecord, BUNDLES_DIR, Found, Quarantine, Status, VerifyError, bundle_id,
    read_capped, scan, sha256_hex, status_of, verify_full, version_of,
};
pub use trust::{trust_list, verify_manifest};
