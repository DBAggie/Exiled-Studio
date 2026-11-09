//! The community dat schema (`poe-tool-dev/dat-schema`, vendored in
//! `tools/extractor/schema/`): which columns each `.datc64` table has, per game.
//!
//! [`Schema::parse`] checks the whole file up front: an unknown field, column
//! type, or game bit is an error, never skipped (ADR-015), so a schema update
//! that changes the format fails loudly instead of misreading tables.

use std::collections::BTreeSet;

use serde::Deserialize;

/// The vendored schema file, embedded at compile time.
pub const VENDORED: &[u8] = include_bytes!("../schema/schema.min.json");

/// SHA-256 of [`VENDORED`], as recorded in `tools/extractor/schema/README.md`.
pub const VENDORED_SHA256: &str =
    "e8e7e4c858924655bcfd56c23b275101f0321f435f90b19f8267bf43344ea16a";

/// Source commit of [`VENDORED`] in `poe-tool-dev/dat-schema`.
pub const VENDORED_COMMIT: &str = "717d788d6ec081821c33473b2a0ec35629144826";

/// The schema format version this loader understands.
pub const SUPPORTED_VERSION: u32 = 7;

/// A game the schema describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Game {
    /// Path of Exile.
    Poe1,
    /// Path of Exile 2.
    Poe2,
}

impl Game {
    /// The game's bit in the schema's `validFor` mask.
    const fn bit(self) -> u8 {
        match self {
            Self::Poe1 => 1,
            Self::Poe2 => 2,
        }
    }
}

/// Every `validFor` bit this loader knows.
const KNOWN_GAME_BITS: u8 = 1 | 2;

/// Type of one value in a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarType {
    /// One byte, 0 or 1.
    Bool,
    /// Signed 16-bit integer.
    I16,
    /// Unsigned 16-bit integer.
    U16,
    /// Signed 32-bit integer.
    I32,
    /// Unsigned 32-bit integer.
    U32,
    /// 32-bit float.
    F32,
    /// Offset of a UTF-16LE string in the variable section.
    String,
    /// Row index in the same table.
    Row,
    /// Row index in another table, plus an unused second word.
    ForeignRow,
    /// Index into an enumeration.
    EnumRow,
}

impl ScalarType {
    /// Bytes one value takes in a row or an array.
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Bool => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 | Self::EnumRow => 4,
            Self::String | Self::Row => 8,
            Self::ForeignRow => 16,
        }
    }

    const fn is_number(self) -> bool {
        matches!(
            self,
            Self::I16 | Self::U16 | Self::I32 | Self::U32 | Self::F32
        )
    }

    fn from_schema(name: &str) -> Option<Self> {
        Some(match name {
            "bool" => Self::Bool,
            "i16" => Self::I16,
            "u16" => Self::U16,
            "i32" => Self::I32,
            "u32" => Self::U32,
            "f32" => Self::F32,
            "string" => Self::String,
            "row" => Self::Row,
            "foreignrow" => Self::ForeignRow,
            "enumrow" => Self::EnumRow,
            _ => return None,
        })
    }
}

/// Bytes an array takes in a row: element count and offset, both `u64`.
pub const ARRAY_WIDTH: usize = 16;

/// Shape of a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnKind {
    /// One value.
    Scalar(ScalarType),
    /// Two values of a number type (a range).
    Interval(ScalarType),
    /// Count and offset of values in the variable section.
    Array(ScalarType),
    /// An array whose element type the schema does not know (`array` type):
    /// count and offset are read, the elements are not.
    UnknownArray,
}

impl ColumnKind {
    /// Bytes the column takes in a row.
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Scalar(ty) => ty.width(),
            Self::Interval(ty) => 2 * ty.width(),
            Self::Array(_) | Self::UnknownArray => ARRAY_WIDTH,
        }
    }
}

/// What a key column points at.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// Table or enumeration name.
    pub table: String,
    /// Column of that table the value matches, when it is not the row index.
    #[serde(default)]
    pub column: Option<String>,
}

/// One column of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Name, if the schema knows it.
    pub name: Option<String>,
    /// Type and shape.
    pub kind: ColumnKind,
    /// Target of a key or enum column.
    pub references: Option<Reference>,
    /// The value differs per game language.
    pub localized: bool,
    /// The schema marks the column as unique.
    pub unique: bool,
}

/// One table as the schema describes it for some games.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableSchema {
    /// Name as in the schema (`Stats`); files are named in lowercase.
    pub name: String,
    /// Games this description is valid for.
    pub games: BTreeSet<Game>,
    /// Columns in row order.
    pub columns: Vec<Column>,
    /// Schema tags (for example `item:def`).
    pub tags: Vec<String>,
}

impl TableSchema {
    /// Bytes per row the schema expects.
    #[must_use]
    pub fn row_width(&self) -> usize {
        self.columns.iter().map(|c| c.kind.width()).sum()
    }
}

/// An enumeration that `enumrow` columns refer to by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enumeration {
    /// Name.
    pub name: String,
    /// Games this description is valid for.
    pub games: BTreeSet<Game>,
    /// Value of the first enumerator (0 or 1).
    pub indexing: u32,
    /// Enumerator names in value order; `None` where unknown.
    pub enumerators: Vec<Option<String>>,
}

/// A parsed schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schema {
    /// Format version ([`SUPPORTED_VERSION`]).
    pub version: u32,
    /// Build time, Unix seconds.
    pub created_at: u64,
    /// Tables in file order.
    pub tables: Vec<TableSchema>,
    /// Enumerations in file order.
    pub enumerations: Vec<Enumeration>,
}

/// Errors from loading a schema. Each variant has a stable [`code`](Self::code).
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    /// Not valid JSON, or not the expected structure (including unknown fields).
    #[error("schema is not valid: {0}")]
    Json(#[from] serde_json::Error),
    /// A format version this loader does not know.
    #[error("schema version {found} is not supported (expected {SUPPORTED_VERSION})")]
    UnsupportedVersion {
        /// Version in the file.
        found: u32,
    },
    /// A `validFor` mask with no known game or with unknown bits.
    #[error("schema {item} `{name}`: validFor {mask} is not a known set of games")]
    UnknownGames {
        /// `table` or `enumeration`.
        item: &'static str,
        /// Its name.
        name: String,
        /// The mask.
        mask: u8,
    },
    /// A column type this loader does not know.
    #[error("schema table `{table}` column {column}: unknown type `{ty}`")]
    UnknownType {
        /// Table name.
        table: String,
        /// Column position.
        column: usize,
        /// The type string.
        ty: String,
    },
    /// A combination of type and flags that has no defined layout.
    #[error("schema table `{table}` column {column}: {reason}")]
    BadColumn {
        /// Table name.
        table: String,
        /// Column position.
        column: usize,
        /// What is wrong.
        reason: &'static str,
    },
    /// Two tables or enumerations with the same name (ignoring case) for one game.
    #[error("schema {item} `{name}` is described twice for {game:?}")]
    Duplicate {
        /// `table` or `enumeration`.
        item: &'static str,
        /// Its name.
        name: String,
        /// The game.
        game: Game,
    },
}

impl SchemaError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "EXTRACT_SCHEMA_INVALID",
            Self::UnsupportedVersion { .. } => "EXTRACT_SCHEMA_UNSUPPORTED_VERSION",
            Self::UnknownGames { .. } => "EXTRACT_SCHEMA_UNKNOWN_GAMES",
            Self::UnknownType { .. } => "EXTRACT_SCHEMA_UNKNOWN_TYPE",
            Self::BadColumn { .. } => "EXTRACT_SCHEMA_BAD_COLUMN",
            Self::Duplicate { .. } => "EXTRACT_SCHEMA_DUPLICATE",
        }
    }
}

/// The file as stored. Every field is listed so an unknown one is an error.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawSchema {
    version: u32,
    created_at: u64,
    tables: Vec<RawTable>,
    enumerations: Vec<RawEnumeration>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawTable {
    valid_for: u8,
    name: String,
    columns: Vec<RawColumn>,
    tags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawColumn {
    name: Option<String>,
    // Read so the field is known; documentation only.
    #[serde(rename = "description")]
    _description: Option<String>,
    array: bool,
    #[serde(rename = "type")]
    ty: String,
    unique: bool,
    localized: bool,
    references: Option<Reference>,
    // Version and file-path annotations; not needed to read rows.
    #[serde(rename = "until")]
    _until: Option<String>,
    #[serde(rename = "file")]
    _file: Option<String>,
    #[serde(rename = "files")]
    _files: Option<Vec<String>>,
    interval: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawEnumeration {
    valid_for: u8,
    name: String,
    indexing: u32,
    enumerators: Vec<Option<String>>,
}

fn games(item: &'static str, name: &str, mask: u8) -> Result<BTreeSet<Game>, SchemaError> {
    if mask == 0 || mask & !KNOWN_GAME_BITS != 0 {
        return Err(SchemaError::UnknownGames {
            item,
            name: name.to_owned(),
            mask,
        });
    }
    Ok([Game::Poe1, Game::Poe2]
        .into_iter()
        .filter(|g| mask & g.bit() != 0)
        .collect())
}

fn column(table: &str, position: usize, raw: RawColumn) -> Result<Column, SchemaError> {
    let bad = |reason| SchemaError::BadColumn {
        table: table.to_owned(),
        column: position,
        reason,
    };
    let kind = if raw.ty == "array" {
        if !raw.array || raw.interval {
            return Err(bad("type `array` must be an array and not an interval"));
        }
        ColumnKind::UnknownArray
    } else {
        let ty = ScalarType::from_schema(&raw.ty).ok_or_else(|| SchemaError::UnknownType {
            table: table.to_owned(),
            column: position,
            ty: raw.ty.clone(),
        })?;
        match (raw.array, raw.interval) {
            (false, false) => ColumnKind::Scalar(ty),
            (true, false) => ColumnKind::Array(ty),
            (false, true) if ty.is_number() => ColumnKind::Interval(ty),
            (false, true) => return Err(bad("only number columns can be intervals")),
            (true, true) => return Err(bad("a column cannot be both array and interval")),
        }
    };
    Ok(Column {
        name: raw.name,
        kind,
        references: raw.references,
        localized: raw.localized,
        unique: raw.unique,
    })
}

/// Fails if two items share a lowercase name for one game.
fn check_unique<'a>(
    item: &'static str,
    entries: impl Iterator<Item = (&'a str, &'a BTreeSet<Game>)>,
) -> Result<(), SchemaError> {
    let mut seen = BTreeSet::new();
    for (name, games) in entries {
        for &game in games {
            if !seen.insert((game, name.to_ascii_lowercase())) {
                return Err(SchemaError::Duplicate {
                    item,
                    name: name.to_owned(),
                    game,
                });
            }
        }
    }
    Ok(())
}

impl Schema {
    /// Parses and checks a schema file.
    ///
    /// # Errors
    /// [`SchemaError`] if the JSON is invalid or has unknown fields, the version
    /// is not [`SUPPORTED_VERSION`], or a table, column, or enumeration is not
    /// understood.
    pub fn parse(json: &[u8]) -> Result<Self, SchemaError> {
        let raw: RawSchema = serde_json::from_slice(json)?;
        if raw.version != SUPPORTED_VERSION {
            return Err(SchemaError::UnsupportedVersion { found: raw.version });
        }
        let tables = raw
            .tables
            .into_iter()
            .map(|t| {
                let games = games("table", &t.name, t.valid_for)?;
                let columns = t
                    .columns
                    .into_iter()
                    .enumerate()
                    .map(|(i, c)| column(&t.name, i, c))
                    .collect::<Result<_, _>>()?;
                Ok(TableSchema {
                    name: t.name,
                    games,
                    columns,
                    tags: t.tags,
                })
            })
            .collect::<Result<Vec<_>, SchemaError>>()?;
        let enumerations = raw
            .enumerations
            .into_iter()
            .map(|e| {
                Ok(Enumeration {
                    games: games("enumeration", &e.name, e.valid_for)?,
                    name: e.name,
                    indexing: e.indexing,
                    enumerators: e.enumerators,
                })
            })
            .collect::<Result<Vec<_>, SchemaError>>()?;
        check_unique("table", tables.iter().map(|t| (t.name.as_str(), &t.games)))?;
        check_unique(
            "enumeration",
            enumerations.iter().map(|e| (e.name.as_str(), &e.games)),
        )?;
        Ok(Self {
            version: raw.version,
            created_at: raw.created_at,
            tables,
            enumerations,
        })
    }

    /// Parses [`VENDORED`].
    ///
    /// # Errors
    /// [`SchemaError`] as for [`parse`](Self::parse); a unit test keeps the
    /// vendored file valid.
    pub fn vendored() -> Result<Self, SchemaError> {
        Self::parse(VENDORED)
    }

    /// Tables valid for `game`, in file order.
    pub fn tables_for(&self, game: Game) -> impl Iterator<Item = &TableSchema> {
        self.tables.iter().filter(move |t| t.games.contains(&game))
    }

    /// The table valid for `game` whose name matches `name` ignoring ASCII
    /// case (file stems are lowercase: `stats` finds `Stats`).
    #[must_use]
    pub fn table(&self, game: Game, name: &str) -> Option<&TableSchema> {
        self.tables_for(game)
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }

    /// The enumeration valid for `game` named `name` (exact case, as columns
    /// refer to it).
    #[must_use]
    pub fn enumeration(&self, game: Game, name: &str) -> Option<&Enumeration> {
        self.enumerations
            .iter()
            .find(|e| e.games.contains(&game) && e.name == name)
    }
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic buffers; a bad index should panic"
)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::install::hex;

    /// A minimal schema: one table per game, one shared enumeration.
    const MINIMAL: &str = r#"{
      "version": 7, "createdAt": 1,
      "tables": [
        {"validFor": 1, "name": "Stats", "tags": [], "columns": [
          {"name": "Id", "description": null, "array": false, "type": "string",
           "unique": true, "localized": false, "references": null, "until": null,
           "file": null, "files": null, "interval": false},
          {"name": null, "description": null, "array": false, "type": "i32",
           "unique": false, "localized": false, "references": null, "until": null,
           "file": null, "files": null, "interval": true},
          {"name": "Keys", "description": null, "array": true, "type": "foreignrow",
           "unique": false, "localized": false, "references": {"table": "Mods"},
           "until": null, "file": null, "files": null, "interval": false}
        ]},
        {"validFor": 2, "name": "Stats", "tags": ["x"], "columns": [
          {"name": "Unknown", "description": null, "array": true, "type": "array",
           "unique": false, "localized": false, "references": null, "until": null,
           "file": null, "files": null, "interval": false}
        ]}
      ],
      "enumerations": [
        {"validFor": 3, "name": "Kind", "indexing": 1, "enumerators": ["A", null]}
      ]
    }"#;

    fn with(column: &str) -> String {
        format!(
            r#"{{"version": 7, "createdAt": 1, "enumerations": [], "tables": [
              {{"validFor": 1, "name": "T", "tags": [], "columns": [{column}]}}]}}"#
        )
    }

    fn col(ty: &str, array: bool, interval: bool) -> String {
        format!(
            r#"{{"name": null, "description": null, "array": {array}, "type": "{ty}",
               "unique": false, "localized": false, "references": null, "until": null,
               "file": null, "files": null, "interval": {interval}}}"#
        )
    }

    #[test]
    fn parse_should_load_minimal_schema() {
        let schema = Schema::parse(MINIMAL.as_bytes()).unwrap();
        assert_eq!(schema.version, 7);
        let poe1 = schema.table(Game::Poe1, "stats").unwrap();
        assert_eq!(poe1.columns.len(), 3);
        assert_eq!(poe1.columns[0].name.as_deref(), Some("Id"));
        assert!(poe1.columns[0].unique);
        assert_eq!(poe1.columns[1].kind, ColumnKind::Interval(ScalarType::I32));
        assert_eq!(
            poe1.columns[2].kind,
            ColumnKind::Array(ScalarType::ForeignRow)
        );
        assert_eq!(
            poe1.columns[2].references,
            Some(Reference {
                table: "Mods".to_owned(),
                column: None
            })
        );
        assert_eq!(poe1.row_width(), 8 + 8 + 16);
        let poe2 = schema.table(Game::Poe2, "STATS").unwrap();
        assert_eq!(poe2.columns[0].kind, ColumnKind::UnknownArray);
        assert_eq!(poe2.row_width(), 16);
        assert_eq!(schema.tables_for(Game::Poe1).count(), 1);
        let kind = schema.enumeration(Game::Poe2, "Kind").unwrap();
        assert_eq!(kind.indexing, 1);
        assert_eq!(kind.enumerators, [Some("A".to_owned()), None]);
        assert!(schema.enumeration(Game::Poe1, "kind").is_none());
    }

    #[test]
    fn widths_should_match_datc64_layout() {
        use ScalarType as T;
        let widths = [
            (T::Bool, 1),
            (T::I16, 2),
            (T::U16, 2),
            (T::I32, 4),
            (T::U32, 4),
            (T::F32, 4),
            (T::EnumRow, 4),
            (T::String, 8),
            (T::Row, 8),
            (T::ForeignRow, 16),
        ];
        for (ty, width) in widths {
            assert_eq!(ty.width(), width, "{ty:?}");
            assert_eq!(ColumnKind::Array(ty).width(), 16);
        }
        assert_eq!(ColumnKind::Interval(T::I32).width(), 8);
        assert_eq!(ColumnKind::UnknownArray.width(), 16);
    }

    #[test]
    fn parse_should_reject_bad_schemas() {
        let cases = [
            ("not json".to_owned(), "EXTRACT_SCHEMA_INVALID"),
            (
                MINIMAL.replace("\"version\": 7", "\"version\": 8"),
                "EXTRACT_SCHEMA_UNSUPPORTED_VERSION",
            ),
            (
                MINIMAL.replace("\"createdAt\": 1", "\"createdAt\": 1, \"extra\": 0"),
                "EXTRACT_SCHEMA_INVALID",
            ),
            (
                MINIMAL.replace("\"validFor\": 2", "\"validFor\": 4"),
                "EXTRACT_SCHEMA_UNKNOWN_GAMES",
            ),
            (
                MINIMAL.replace("\"validFor\": 2", "\"validFor\": 0"),
                "EXTRACT_SCHEMA_UNKNOWN_GAMES",
            ),
            (
                MINIMAL.replace("\"validFor\": 3", "\"validFor\": 7"),
                "EXTRACT_SCHEMA_UNKNOWN_GAMES",
            ),
            (
                MINIMAL.replace("\"validFor\": 2", "\"validFor\": 3"),
                "EXTRACT_SCHEMA_DUPLICATE",
            ),
            (
                with(&col("u64", false, false)),
                "EXTRACT_SCHEMA_UNKNOWN_TYPE",
            ),
            (
                with(&col("array", false, false)),
                "EXTRACT_SCHEMA_BAD_COLUMN",
            ),
            (
                with(&col("string", false, true)),
                "EXTRACT_SCHEMA_BAD_COLUMN",
            ),
            (with(&col("i32", true, true)), "EXTRACT_SCHEMA_BAD_COLUMN"),
            (
                with(&col("i32", false, false).replace("\"until\"", "\"since\"")),
                "EXTRACT_SCHEMA_INVALID",
            ),
        ];
        for (json, code) in cases {
            let err = Schema::parse(json.as_bytes()).unwrap_err();
            assert_eq!(err.code(), code, "{err}");
        }
    }

    #[test]
    fn parse_should_reject_duplicate_enumerations_and_case_variants() {
        let json = MINIMAL.replace(
            r#""enumerators": ["A", null]}"#,
            r#""enumerators": []}, {"validFor": 2, "name": "Kind", "indexing": 0, "enumerators": []}"#,
        );
        assert_eq!(
            Schema::parse(json.as_bytes()).unwrap_err().code(),
            "EXTRACT_SCHEMA_DUPLICATE"
        );
        let json = MINIMAL.replace(
            r#""validFor": 2, "name": "Stats""#,
            r#""validFor": 1, "name": "STATS""#,
        );
        assert_eq!(
            Schema::parse(json.as_bytes()).unwrap_err().code(),
            "EXTRACT_SCHEMA_DUPLICATE"
        );
    }

    #[test]
    fn vendored_schema_should_match_recorded_hash() {
        assert_eq!(hex(&Sha256::digest(VENDORED)), VENDORED_SHA256);
    }

    #[test]
    fn vendored_schema_should_load_for_both_games() {
        let schema = Schema::vendored().unwrap();
        assert_eq!(schema.version, SUPPORTED_VERSION);
        for game in [Game::Poe1, Game::Poe2] {
            let stats = schema.table(game, "stats").unwrap();
            assert_eq!(stats.columns[0].name.as_deref(), Some("Id"));
            assert_eq!(
                stats.columns[0].kind,
                ColumnKind::Scalar(ScalarType::String)
            );
            assert!(schema.tables_for(game).count() > 900, "{game:?}");
        }
    }
}
