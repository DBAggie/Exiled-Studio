//! The numbers of another build file, to compare the open build with.
//!
//! The file is read as the application reads a file it opens: a saved
//! build document through the store (as `Session::open`), or a PoB XML
//! file through the import (as `Session::import_file`, with the cluster
//! hash migration and the default main skill). Content whose first byte
//! that is not whitespace is `{` is a build document; anything else is
//! read as PoB XML, so a file that is neither gives its reader's error.
//!
//! It is then calculated with the loaded data on the caller's thread,
//! never on the writer, as a what-if point is ([`crate::what_if`]).
//! Nothing is opened: the open build, its version and history, and the
//! recent list stay as they are, and no build needs to be open.

use std::path::Path;

use loom_persistence::Store;
use loom_serialization::dto::app::v1 as api;

use crate::data::GameData;
use crate::error::{AppError, Code};
use crate::session;

/// The computed fields of the build file at `path`
/// ([`api::BuildFields`]), calculated with `data`, polling `cancel`.
///
/// # Errors
/// `APP_READ` when the file does not read (or, a PoB file, is not
/// UTF-8); the store's codes for a build document (`PERSIST_*`,
/// `MIGRATE_*`, `DTO_*`) or the import adapter's and the resolver's for
/// PoB XML; `APP_GAME_MISMATCH`, `APP_DATA_VERSION` when the data cannot
/// calculate the build; the calculation's codes (`CALC_CANCELLED` when
/// cancelled).
pub fn fields_of_file(
    path: &Path,
    data: &GameData,
    cancel: &dyn Fn() -> bool,
) -> Result<api::BuildFields, AppError> {
    let bytes = std::fs::read(path)
        .map_err(|e| AppError::new(Code::Read, format!("{}: {e}", path.display())))?;
    let document = bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{');
    let state = if document {
        // The store's own reading (its encoding checks and migrations).
        let loaded = Store::new().load(path)?;
        session::opened_state(loaded.build, Some(data)).0
    } else {
        let text = String::from_utf8(bytes)
            .map_err(|_| AppError::new(Code::Read, format!("{}: not UTF-8", path.display())))?;
        session::import_state(&text, data)?
    };
    session::check_data(&state.build, data)?;
    let result = session::calculate(&state, data, cancel)?;
    Ok(api::BuildFields {
        name: session::stem(path),
        fields: crate::what_if::point_fields(&result),
    })
}
