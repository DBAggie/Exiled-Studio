//! Application API v1 (module docs of [`super`]).
//!
//! Names are kebab-case on the wire, as the documents'. Item and line
//! indices are the domain's: an item's ID within the build, a line's index
//! among the item's source lines (the calc result's `item` sources use the
//! same pair, so a breakdown links to its line).

use serde::{Deserialize, Serialize};

/// The API version this module describes.
pub const VERSION: u32 = 1;

/// Declares DTO types with the shared derives and serde settings.
macro_rules! dto {
    ($($item:item)*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
            #[serde(deny_unknown_fields, rename_all = "kebab-case")]
            $item
        )*
    };
}

dto! {
    /// Every top-level type of the API, so one schema names them all. Never
    /// sent.
    pub struct Api {
        /// The snapshot query's answer.
        pub snapshot: Snapshot,
        /// One item.
        pub item: ItemView,
        /// A command.
        pub command: Command,
        /// A command's answer.
        pub outcome: Outcome,
        /// An event.
        pub event: Event,
        /// An error.
        pub error: ApiError,
        /// An item edit's preview.
        pub preview: ItemEditPreview,
        /// The MCP server's state.
        pub mcp: McpStatus,
        /// Recovery snapshots found at start.
        pub recovery: Vec<RecoveryEntry>,
        /// Recent builds.
        pub recent: Vec<RecentEntry>,
    }

    /// The application's state at one version. Queries return it whole;
    /// events say when it changed (ADR-004: a missed event version means
    /// query again).
    pub struct Snapshot {
        /// Increases with every change of the build (import, open, edit,
        /// undo, redo); 0 before any build.
        pub version: u64,
        /// The open build, if any.
        pub build: Option<BuildView>,
        /// The calculation of the latest version.
        pub calc: CalcStatus,
        /// Undo and redo.
        pub history: HistoryView,
        /// Whether the build changed since it was last saved or opened.
        pub dirty: bool,
        /// Where the build is saved (a build document), if it is.
        pub path: Option<String>,
        /// The file the build was imported from, if any; never written.
        pub origin: Option<String>,
        /// The game data the application calculates with.
        pub data: DataStatus,
    }

    /// The game data.
    pub struct DataStatus {
        /// The bundle folder, if one is set.
        pub bundle: Option<String>,
        /// The bundle's data version, when it loaded.
        pub data_version: Option<DataVersionView>,
        /// The passive tree version the application gives the engine for
        /// this bundle, if known.
        pub tree_version: Option<String>,
        /// Why the bundle did not load, if it did not (a stable code).
        pub error: Option<ApiError>,
    }

    /// A data version.
    pub struct DataVersionView {
        /// `poe1` or `poe2`.
        pub game: String,
        /// The bundle's data schema version.
        pub schema_version: u32,
        /// The Steam build it was extracted from, if known.
        pub steam_build: Option<u64>,
    }

    /// The open build.
    pub struct BuildView {
        /// A display name: the file's stem, else "imported build" in the
        /// UI's language (empty here).
        pub name: String,
        /// `poe1` or `poe2`.
        pub game: String,
        /// The class, if set.
        pub class: Option<String>,
        /// The ascendancy, if set.
        pub ascendancy: Option<String>,
        /// The character level, if set.
        pub level: Option<u32>,
        /// The main skill's gem names, if the build has a main group.
        pub main_skill: Option<String>,
        /// The data version the build is pinned to.
        pub data_version: Option<DataVersionView>,
        /// `complete` or `partial`: whether anything was not read.
        pub completeness: String,
        /// The import's counts.
        pub lines: LineCounts,
        /// The items, in build order.
        pub items: Vec<ItemSummary>,
        /// Lines with warnings: kept, shown, not calculated (ADR-015).
        pub warnings: Vec<WarningView>,
    }

    /// Counts of item modifier lines by status.
    pub struct LineCounts {
        /// Modifier lines.
        pub modifiers: u32,
        /// Resolved to game stats exactly.
        pub matched: u32,
        /// Resolved after an import tolerance.
        pub adapted: u32,
        /// With a warning.
        pub warnings: u32,
        /// Excluded by the item (another variant, a custom line, a
        /// section without effect).
        pub excluded: u32,
    }

    /// One item in a list.
    pub struct ItemSummary {
        /// The item's ID.
        pub id: u32,
        /// The name, if any.
        pub name: Option<String>,
        /// The base, if written.
        pub base: Option<String>,
        /// `normal`, `magic`, `rare`, `unique`, `relic`, if read.
        pub rarity: Option<String>,
        /// The slot it is in, in the active item set, if any.
        pub slot: Option<String>,
        /// Its lines with warnings.
        pub warnings: u32,
    }

    /// One item with its lines.
    pub struct ItemView {
        /// The item's ID.
        pub id: u32,
        /// The name, if any.
        pub name: Option<String>,
        /// The base, if written.
        pub base: Option<String>,
        /// The rarity, if read.
        pub rarity: Option<String>,
        /// The slot it is in, if any.
        pub slot: Option<String>,
        /// Every source line, in order.
        pub lines: Vec<LineView>,
    }

    /// One source line of an item.
    pub struct LineView {
        /// The line's index among the item's lines.
        pub index: u32,
        /// The line as written.
        pub text: String,
        /// What it is: the domain's line kind name (`modifier`, `spec`,
        /// `name`, ...).
        pub kind: String,
        /// The modifier list it belongs to (`implicit`, `explicit`, ...);
        /// modifier lines only.
        pub section: Option<String>,
        /// Its status.
        pub status: LineStatus,
        /// The warning's code (`LINE_NO_TEMPLATE`, ...), when the status is
        /// `warning`.
        pub code: Option<String>,
        /// What the line resolved to: game stats, or a typed kind.
        pub stats: Vec<StatView>,
        /// The typed kind (`granted-skill`, ...), if it resolved to one.
        pub typed: Option<String>,
        /// The tolerance it needed (`case-folded`, `split-section`).
        pub adapted: Option<String>,
        /// The numbers in the text, in order, for roll edits.
        pub numbers: Vec<NumberSpan>,
    }

    /// A number in a line's text: where it is, as character offsets.
    pub struct NumberSpan {
        /// The first character's offset.
        pub start: u32,
        /// The offset after the last character.
        pub end: u32,
        /// The number as written.
        pub text: String,
    }

    /// One stat a line resolved to.
    pub struct StatView {
        /// The game's stat ID.
        pub id: String,
        /// The value as text (`15`, `10 to 20`, `hidden`).
        pub value: String,
    }

    /// A line with a warning.
    pub struct WarningView {
        /// The item's ID.
        pub item: u32,
        /// The item's name, if any.
        pub item_name: Option<String>,
        /// The line's index.
        pub line: u32,
        /// The line as written.
        pub text: String,
        /// The stable code.
        pub code: String,
    }

    /// Undo and redo.
    pub struct HistoryView {
        /// Whether there is a change to undo.
        pub can_undo: bool,
        /// Whether there is a change to redo.
        pub can_redo: bool,
        /// What undo would revert.
        pub undo: Option<ChangeKind>,
        /// What redo would apply again.
        pub redo: Option<ChangeKind>,
    }

    /// The calculation of the latest version.
    pub struct CalcStatus {
        /// Where it is.
        pub state: CalcState,
        /// The version of the result held (`result` is of it); may be
        /// older than the snapshot's while a calculation runs.
        pub version: u64,
        /// The result's overall state (`complete`, `partial`,
        /// `unsupported`, `indeterminate`), when there is one.
        pub result_state: Option<String>,
        /// Why the latest calculation failed, if it did.
        pub error: Option<ApiError>,
    }

    /// An item edit: line operations on one item.
    pub struct ItemEdit {
        /// The item's ID.
        pub item: u32,
        /// The operations, in order; line indices are the item's before
        /// the edit.
        pub ops: Vec<LineOp>,
    }

    /// An item edit's effect, calculated by the application (never by the
    /// requester).
    pub struct ItemEditPreview {
        /// The item as edited, with its lines' new statuses.
        pub item: ItemView,
        /// Each slice field before and after.
        pub fields: Vec<FieldDelta>,
    }

    /// One field before and after an edit.
    pub struct FieldDelta {
        /// The field's name.
        pub field: String,
        /// Its value before, when computed and finite.
        pub before: Option<f64>,
        /// Its value after, when computed and finite.
        pub after: Option<f64>,
        /// Its state before.
        pub state_before: String,
        /// Its state after.
        pub state_after: String,
    }

    /// The MCP server (ADR-017).
    pub struct McpStatus {
        /// Whether it runs.
        pub running: bool,
        /// Its address, while it runs (`http://127.0.0.1:<port>/mcp`).
        pub url: Option<String>,
        /// The session's bearer token, while it runs.
        pub token: Option<String>,
        /// What clients may do.
        pub permissions: McpPermissions,
        /// Change requests, newest last.
        pub requests: Vec<ChangeRequest>,
    }

    /// What MCP clients may do. Reading is allowed while the server runs.
    pub struct McpPermissions {
        /// Clients may request changes (off by default: read-only).
        pub requests: bool,
        /// Requests apply without asking (a standing grant the user can
        /// revoke).
        pub standing_grant: bool,
    }

    /// A change an MCP client asked for.
    pub struct ChangeRequest {
        /// Its ID, unique in the session.
        pub id: u64,
        /// The client's name, as it gave it.
        pub client: String,
        /// The edit.
        pub edit: ItemEdit,
        /// The client's note, if any.
        pub note: Option<String>,
        /// Its effect, calculated when it arrived.
        pub preview: Option<ItemEditPreview>,
        /// Where it is.
        pub status: RequestStatus,
        /// Why it failed, if it did.
        pub error: Option<ApiError>,
    }

    /// A recovery snapshot (autosave) found at start.
    pub struct RecoveryEntry {
        /// Its ID (the file's stem).
        pub id: String,
        /// When it was written, in seconds since the Unix epoch.
        pub saved_at: u64,
        /// The build's display name.
        pub name: String,
        /// Where the build was saved, if it was.
        pub path: Option<String>,
        /// The file it was imported from, if any.
        pub origin: Option<String>,
    }

    /// A recent build.
    pub struct RecentEntry {
        /// Its position in the list (the UI opens a recent build by it;
        /// the application checks it, the UI never sends a path).
        pub index: u32,
        /// The build document's path.
        pub path: String,
        /// The display name.
        pub name: String,
        /// When it was last opened or saved, in seconds since the Unix
        /// epoch.
        pub at: u64,
    }

    /// An error: a stable code (ADR-014) and a message for logs. The UI
    /// shows text for the code, not the message.
    pub struct ApiError {
        /// The code of the layer that failed (`IMPORT_*`, `PERSIST_*`,
        /// `APP_*`, ...).
        pub code: String,
        /// Detail in English, for diagnostics.
        pub message: String,
    }

    /// An event (ADR-004): something committed, a job's progress, or an
    /// error.
    pub struct Event {
        /// The snapshot version after the event.
        pub version: u64,
        /// The command that caused it, if any.
        pub correlation: Option<u64>,
        /// What happened.
        pub kind: EventKind,
    }
}

/// Declares DTO enums: externally tagged, kebab-case.
macro_rules! dto_enum {
    ($($item:item)*) => {
        $(
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
            #[serde(deny_unknown_fields, rename_all = "kebab-case")]
            $item
        )*
    };
}

dto_enum! {
    /// A line's status, as the UI names it (`docs/ui/UI-DESIGN-BRIEF.md`,
    /// terminology).
    pub enum LineStatus {
        /// Resolved to game stats or a typed kind.
        Matched,
        /// Resolved after an import tolerance.
        Adapted,
        /// Kept and shown, not calculated (ADR-015).
        Warning,
        /// Excluded by the item.
        Excluded,
        /// Continues the line before it (one description over several
        /// lines).
        Continues,
        /// Not a modifier line.
        Other,
    }

    /// A calculation's progress.
    pub enum CalcState {
        /// No build, or no data.
        Idle,
        /// A calculation of the latest version runs.
        Running,
        /// The result is of the latest version.
        Done,
        /// The latest version's calculation failed.
        Failed,
    }

    /// What a change was, for undo and redo labels.
    pub enum ChangeKind {
        /// An item edit.
        ItemEdit,
        /// An item edit an MCP client requested.
        RequestApplied,
    }

    /// One line operation of an item edit.
    pub enum LineOp {
        /// Replaces the line's text.
        Replace {
            /// The line's index.
            line: u32,
            /// The new text.
            text: String,
        },
        /// Removes the line.
        Remove {
            /// The line's index.
            line: u32,
        },
        /// Adds a modifier line after the item's last line.
        Add {
            /// The text.
            text: String,
        },
    }

    /// A change request's status.
    pub enum RequestStatus {
        /// Waiting for the user.
        Pending,
        /// Applied (approved, or under the standing grant).
        Applied,
        /// Declined by the user.
        Declined,
        /// It could not apply (the item changed, a line is gone, ...).
        Failed,
    }

    /// A command (ADR-004): an intent the application runs in order.
    pub enum Command {
        /// Imports a PoB share code or XML text.
        ImportText {
            /// The text.
            text: String,
        },
        /// Applies an item edit.
        EditItem(ItemEdit),
        /// Reverts the latest change.
        Undo,
        /// Applies the latest reverted change again.
        Redo,
        /// Saves the build where it is saved.
        Save,
        /// Opens a recent build by its index.
        OpenRecent {
            /// The index in the recent list.
            index: u32,
        },
        /// Restores a recovery snapshot.
        Restore {
            /// The snapshot's ID.
            id: String,
        },
        /// Discards a recovery snapshot.
        Discard {
            /// The snapshot's ID.
            id: String,
        },
        /// Approves a change request.
        Approve {
            /// The request's ID.
            id: u64,
        },
        /// Declines a change request.
        Decline {
            /// The request's ID.
            id: u64,
        },
        /// Closes the build.
        Close,
    }

    /// A command's answer.
    pub enum Outcome {
        /// Done; the snapshot version after it.
        Done {
            /// The version.
            version: u64,
        },
        /// Done, with text to give the user (a share code).
        Text {
            /// The version.
            version: u64,
            /// The text.
            text: String,
        },
        /// Nothing to do (undo with an empty history, a cancelled dialog).
        Nothing,
    }

    /// What an event reports.
    pub enum EventKind {
        /// The build changed (or was opened, imported, closed).
        BuildChanged,
        /// The build was saved.
        Saved {
            /// Where.
            path: String,
        },
        /// A calculation started.
        CalcStarted,
        /// A calculation finished; its result is the snapshot's.
        CalcFinished {
            /// The result's overall state.
            state: String,
        },
        /// A calculation failed.
        CalcFailed {
            /// Why.
            error: ApiError,
        },
        /// The MCP server or its requests changed.
        McpChanged,
        /// A command failed.
        Failed {
            /// Why.
            error: ApiError,
        },
    }
}

impl ApiError {
    /// An error with this code and message.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_should_read_its_kebab_case_wire_form() {
        let json = r#"{"edit-item":{"item":1,"ops":[{"replace":{"line":3,"text":"+1 to x"}},"undo-is-not-an-op"]}}"#;
        assert!(serde_json::from_str::<Command>(json).is_err());
        let json = r#"{"edit-item":{"item":1,"ops":[{"replace":{"line":3,"text":"+1 to x"}},{"remove":{"line":4}},{"add":{"text":"y"}}]}}"#;
        let command: Command = serde_json::from_str(json).unwrap();
        assert_eq!(
            command,
            Command::EditItem(ItemEdit {
                item: 1,
                ops: vec![
                    LineOp::Replace {
                        line: 3,
                        text: "+1 to x".to_owned()
                    },
                    LineOp::Remove { line: 4 },
                    LineOp::Add {
                        text: "y".to_owned()
                    },
                ],
            })
        );
        assert_eq!(serde_json::to_string(&Command::Undo).unwrap(), r#""undo""#);
    }

    #[test]
    fn event_should_write_kebab_case_fields() {
        let event = Event {
            version: 3,
            correlation: Some(7),
            kind: EventKind::CalcFinished {
                state: "partial".to_owned(),
            },
        };
        assert_eq!(
            serde_json::to_string(&event).unwrap(),
            r#"{"version":3,"correlation":7,"kind":{"calc-finished":{"state":"partial"}}}"#
        );
    }
}
