//! Table extraction (Spike A, PR A4): picks the tables to extract, decodes
//! them, writes each as canonical JSON, and builds the manifest that ties the
//! output to its inputs (ADR-005). Stat-description files (PR A5) go through
//! the same [`Extraction`] and are parsed by [`crate::statdesc`], and so do
//! passive skill graphs (PR A6b, [`crate::psg`]). Pure: the
//! filesystem stays in `main.rs` and [`crate::install`].
//!
//! Output is deterministic by construction: tables and findings are kept in
//! `BTreeMap`s, rows keep file order, numbers are formatted without locale,
//! and `f32` values use the shortest text that reads back to the same `f32`
//! (never widened to `f64`). Keys stay exact `u64` integers. The order in
//! which tables are added does not change a byte (unit test). Nothing is
//! dropped silently (ADR-015): every planned or referenced table is in the
//! manifest with a status, and findings (row-size mismatches, keys past the
//! end of their target, non-finite floats) are counted there.
//!
//! Table JSON layout (one row per line, so diffs stay readable):
//! ```text
//! {
//! "columns": [{"array":false,"interval":false,"name":"Id","references":null,"type":"string"}],
//! "rows": [
//! ["level"],
//! ...
//! ],
//! "table": "Stats"
//! }
//! ```
//! Values: numbers and booleans as JSON; strings as JSON strings; null keys as
//! `null`; enum columns as the stored number; intervals and arrays as JSON
//! arrays; arrays of unknown type as `{"count":n,"offset":m}`; non-finite
//! floats as the strings `"NaN"`, `"Infinity"`, `"-Infinity"`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Write as _};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::dat::{RowSizeMismatch, Value, read_table};
use crate::install::{BundleHashes, INDEX_PATH, hex};
use crate::psg::{self, GraphManifest};
use crate::schema::{
    Column, ColumnKind, Game, ScalarType, Schema, TableSchema, VENDORED_COMMIT, VENDORED_SHA256,
};
use crate::statdesc::{self, StatDescManifest};

/// Tables Spike A extracts; the tables they reference are added by [`Plan::new`].
/// `PassiveSkillTrees` (PR A6b) names the graph file of each passive tree.
pub const SPIKE_TABLES: [&str; 7] = [
    "Stats",
    "Mods",
    "BaseItemTypes",
    "PassiveSkills",
    "ActiveSkills",
    "GrantedEffects",
    "PassiveSkillTrees",
];

/// Version of the manifest layout. Bump it when a field changes meaning.
/// 2: `stat_descriptions` added (PR A5).
/// 3: `passive_skill_graphs` added (PR A6b).
pub const MANIFEST_FORMAT: u32 = 3;

/// Manifest file name inside the output directory.
pub const MANIFEST_PATH: &str = "manifest.json";

/// Directory of the table files inside the output directory.
pub const TABLES_DIR: &str = "tables";

/// Language of the text read. Tables come from the game's main table
/// directory ([`table_dir`]), which holds the English text; the per-language
/// copies in its subfolders are not read, and `localized` columns are not
/// treated specially.
pub const TEXT_LANGUAGE: &str = "English";

/// Where the vendored schema comes from (`tools/extractor/schema/README.md`).
pub const SCHEMA_SOURCE: &str = "poe-tool-dev/dat-schema";

/// The game's short name in output (`poe1`, `poe2`).
#[must_use]
pub const fn game_name(game: Game) -> &'static str {
    match game {
        Game::Poe1 => "poe1",
        Game::Poe2 => "poe2",
    }
}

/// Folder of the game's English tables in the index.
#[must_use]
pub const fn table_dir(game: Game) -> &'static str {
    match game {
        Game::Poe1 => "data/",
        Game::Poe2 => "data/balance/",
    }
}

/// One table of a [`Plan`].
#[derive(Debug, Clone)]
struct Planned<'s> {
    /// Name from the schema, or as referenced when there is no schema.
    name: String,
    schema: Option<&'s TableSchema>,
    root: bool,
    /// Schema names of the planned tables with a column that refers to it.
    referenced_by: BTreeSet<String>,
}

/// The tables to extract: the roots plus every table they reach through
/// columns that reference a table, by lowercase name.
#[derive(Debug, Clone)]
pub struct Plan<'s> {
    game: Game,
    tables: BTreeMap<String, Planned<'s>>,
}

/// The column's value type, if it has one.
const fn element(kind: ColumnKind) -> Option<ScalarType> {
    match kind {
        ColumnKind::Scalar(ty) | ColumnKind::Interval(ty) | ColumnKind::Array(ty) => Some(ty),
        ColumnKind::UnknownArray => None,
    }
}

impl<'s> Plan<'s> {
    /// Plans `roots` (matched ignoring case) and, transitively, every table a
    /// planned table's column references. Enumeration references are not
    /// tables and are not followed. A table without a schema for `game` is
    /// still planned, so the manifest reports it.
    #[must_use]
    pub fn new(schema: &'s Schema, game: Game, roots: &[&str]) -> Self {
        let mut plan = Self {
            game,
            tables: BTreeMap::new(),
        };
        let mut todo = Vec::new();
        for root in roots {
            plan.visit(schema, root, &mut todo).root = true;
        }
        while let Some(key) = todo.pop() {
            let Some(from) = plan.tables.get(&key).and_then(|p| p.schema) else {
                continue;
            };
            for column in &from.columns {
                let Some(target) = &column.references else {
                    continue;
                };
                if element(column.kind) == Some(ScalarType::EnumRow) {
                    continue;
                }
                plan.visit(schema, &target.table, &mut todo)
                    .referenced_by
                    .insert(from.name.clone());
            }
        }
        plan
    }

    /// The planned entry for `name`, added (and queued in `todo`) if new.
    fn visit(
        &mut self,
        schema: &'s Schema,
        name: &str,
        todo: &mut Vec<String>,
    ) -> &mut Planned<'s> {
        let key = name.to_ascii_lowercase();
        let game = self.game;
        self.tables.entry(key).or_insert_with_key(|key| {
            todo.push(key.clone());
            let found = schema.table(game, name);
            Planned {
                name: found.map_or_else(|| name.to_owned(), |t| t.name.clone()),
                schema: found,
                root: false,
                referenced_by: BTreeSet::new(),
            }
        })
    }

    /// The game planned for.
    #[must_use]
    pub const fn game(&self) -> Game {
        self.game
    }

    /// Index paths of the planned tables that have a schema, so can be read.
    pub fn sources(&self) -> impl Iterator<Item = String> + '_ {
        self.tables
            .iter()
            .filter(|(_, p)| p.schema.is_some())
            .map(|(key, _)| format!("{}{key}.datc64", table_dir(self.game)))
    }

    /// The planned key of a table path from the index, if it is one.
    fn key_of(&self, path: &str) -> Option<String> {
        let path = path.to_ascii_lowercase();
        let stem = path
            .strip_prefix(table_dir(self.game))?
            .strip_suffix(".datc64")?;
        (!stem.contains('/') && self.tables.contains_key(stem)).then(|| stem.to_owned())
    }
}

/// Errors from extraction. Each variant has a stable [`code`](Self::code).
/// A table that fails to decode is not an error: the manifest reports it.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    /// The path is not the file of a planned table that has a schema.
    #[error("{path} is not the file of a planned table")]
    Unplanned {
        /// Path as given.
        path: String,
    },
    /// The table was added before.
    #[error("{path} was added twice")]
    Duplicate {
        /// Path as given.
        path: String,
    },
    /// The path is not a stat-description file of the game.
    #[error("{path} is not a stat-description file")]
    UnplannedFile {
        /// Path as given.
        path: String,
    },
    /// The stat-description file was added before.
    #[error("{path} was added twice")]
    DuplicateFile {
        /// Path as given.
        path: String,
    },
    /// Writing JSON failed.
    #[error("cannot write JSON: {0}")]
    Json(#[from] serde_json::Error),
}

impl ExtractError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Unplanned { .. } => "EXTRACT_UNPLANNED_TABLE",
            Self::Duplicate { .. } => "EXTRACT_DUPLICATE_TABLE",
            Self::UnplannedFile { .. } => "EXTRACT_UNPLANNED_FILE",
            Self::DuplicateFile { .. } => "EXTRACT_DUPLICATE_FILE",
            Self::Json(_) => "EXTRACT_JSON",
        }
    }
}

/// Why a table or stat-description file was not decoded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableError {
    /// Stable error code.
    pub code: &'static str,
    /// Readable message.
    pub message: String,
}

/// Keys of one key column, for the range check in [`Extraction::finish`].
#[derive(Debug, Clone)]
struct Keys {
    column: String,
    /// Lowercase name of the target table.
    target: String,
    values: Vec<u64>,
}

/// A decoded and rendered table.
#[derive(Debug, Clone)]
struct Rendered {
    json: Vec<u8>,
    rows: usize,
    row_len: usize,
    mismatch: Option<RowSizeMismatch>,
    columns_read: usize,
    non_finite_floats: u64,
    keys: Vec<Keys>,
}

/// One added table.
#[derive(Debug, Clone)]
struct Added {
    source: String,
    sha256: [u8; 32],
    result: Result<Rendered, TableError>,
}

/// Collects the planned tables and the stat-description files in any order
/// and builds the output.
#[derive(Debug, Clone)]
pub struct Extraction<'s> {
    plan: Plan<'s>,
    added: BTreeMap<String, Added>,
    stat_descriptions: statdesc::Collection,
    graphs: psg::Collection,
}

impl<'s> Extraction<'s> {
    /// Starts an extraction of `plan` and of the game's stat-description
    /// files ([`statdesc::source_kind`]).
    #[must_use]
    pub const fn new(plan: Plan<'s>) -> Self {
        let game = plan.game;
        Self {
            plan,
            added: BTreeMap::new(),
            stat_descriptions: statdesc::Collection::new(game),
            graphs: psg::Collection::new(game),
        }
    }

    /// Decodes and renders the table or stat-description file at `path` (as
    /// listed in the index). A file that does not decode is recorded for the
    /// manifest, not returned as an error.
    ///
    /// # Errors
    /// [`ExtractError`] if `path` is neither a planned table with a schema
    /// nor a stat-description file, was added before, or JSON cannot be
    /// written.
    pub fn add(&mut self, path: &str, bytes: &[u8]) -> Result<(), ExtractError> {
        if self.stat_descriptions.wants(path) {
            return self.stat_descriptions.add(path, bytes);
        }
        if psg::is_graph(path) {
            return self.graphs.add(path, bytes);
        }
        let unplanned = || ExtractError::Unplanned {
            path: path.to_owned(),
        };
        let key = self.plan.key_of(path).ok_or_else(unplanned)?;
        let schema = self
            .plan
            .tables
            .get(&key)
            .and_then(|p| p.schema)
            .ok_or_else(unplanned)?;
        if self.added.contains_key(&key) {
            return Err(ExtractError::Duplicate {
                path: path.to_owned(),
            });
        }
        let result = match read_table(bytes, schema) {
            Ok(table) => Ok(render(schema, &table.rows, table.columns_read).map(
                |(json, non_finite_floats, keys)| Rendered {
                    json,
                    rows: table.rows.len(),
                    row_len: table.row_len,
                    mismatch: table.mismatch,
                    columns_read: table.columns_read,
                    non_finite_floats,
                    keys,
                },
            )?),
            Err(e) => Err(TableError {
                code: e.code(),
                message: e.to_string(),
            }),
        };
        self.added.insert(
            key,
            Added {
                source: path.to_owned(),
                sha256: Sha256::digest(bytes).into(),
                result,
            },
        );
        Ok(())
    }

    /// Checks key ranges and builds the table files and the manifest.
    #[must_use]
    pub fn finish(self, inputs: Inputs) -> Output {
        let Self {
            plan,
            mut added,
            stat_descriptions,
            graphs,
        } = self;
        let rows_of: BTreeMap<String, usize> = added
            .iter()
            .filter_map(|(key, a)| Some((key.clone(), a.result.as_ref().ok()?.rows)))
            .collect();
        let name_of = |key: &str| {
            plan.tables
                .get(key)
                .map_or_else(|| key.to_owned(), |p| p.name.clone())
        };

        let mut files = BTreeMap::new();
        let mut file_entries = BTreeMap::new();
        let mut tables = BTreeMap::new();
        for (key, planned) in &plan.tables {
            let mut entry = TableEntry {
                columns: planned.schema.map(|s| s.columns.len()),
                columns_read: None,
                error: None,
                key_columns: BTreeMap::new(),
                non_finite_floats: 0,
                output: None,
                referenced_by: planned.referenced_by.clone(),
                root: planned.root,
                row_len: None,
                row_size_mismatch: None,
                rows: None,
                source: None,
                source_sha256: None,
                status: Status::NoSchema,
            };
            match (planned.schema, added.remove(key)) {
                (None, _) => {}
                (Some(_), None) => entry.status = Status::NoFile,
                (
                    Some(_),
                    Some(Added {
                        source,
                        sha256,
                        result,
                    }),
                ) => {
                    entry.source = Some(source);
                    entry.source_sha256 = Some(hex(&sha256));
                    match result {
                        Err(error) => {
                            entry.status = Status::Failed;
                            entry.error = Some(error);
                        }
                        Ok(r) => {
                            entry.status = if r.mismatch.is_some() {
                                Status::RowSizeMismatch
                            } else {
                                Status::Ok
                            };
                            entry.columns_read = Some(r.columns_read);
                            entry.non_finite_floats = r.non_finite_floats;
                            entry.row_len = Some(r.row_len);
                            entry.row_size_mismatch = r.mismatch.map(|m| Mismatch {
                                actual: m.actual,
                                expected: m.expected,
                            });
                            entry.rows = Some(r.rows);
                            for keys in r.keys {
                                let dangling = rows_of.get(&keys.target).map(|&n| {
                                    keys.values
                                        .iter()
                                        .filter(|&&v| usize::try_from(v).map_or(true, |v| v >= n))
                                        .count()
                                });
                                entry.key_columns.insert(
                                    keys.column,
                                    KeyCheck {
                                        dangling,
                                        target: name_of(&keys.target),
                                        values: keys.values.len(),
                                    },
                                );
                            }
                            let path = format!("{TABLES_DIR}/{}.json", planned.name);
                            file_entries.insert(
                                path.clone(),
                                OutputFile {
                                    rows: r.rows,
                                    sha256: hex(&Sha256::digest(&r.json)),
                                },
                            );
                            entry.output = Some(path.clone());
                            files.insert(path, r.json);
                        }
                    }
                }
            }
            tables.insert(planned.name.clone(), entry);
        }

        let (statdesc_files, stat_descriptions) = stat_descriptions.finish();
        for entry in stat_descriptions.files.values() {
            if let Some(path) = &entry.output
                && let Some(bytes) = statdesc_files.get(path)
            {
                // A stat-description file's "rows" are its descriptions.
                file_entries.insert(
                    path.clone(),
                    OutputFile {
                        rows: entry.descriptions,
                        sha256: hex(&Sha256::digest(bytes)),
                    },
                );
            }
        }
        files.extend(statdesc_files);

        let (graph_files, passive_skill_graphs) = graphs.finish();
        for entry in passive_skill_graphs.files.values() {
            if let Some(path) = &entry.output
                && let Some(bytes) = graph_files.get(path)
            {
                // A graph's "rows" are its nodes.
                file_entries.insert(
                    path.clone(),
                    OutputFile {
                        rows: entry.nodes.unwrap_or_default(),
                        sha256: hex(&Sha256::digest(bytes)),
                    },
                );
            }
        }
        files.extend(graph_files);

        let manifest = Manifest {
            bundles: inputs
                .bundles
                .iter()
                .map(|(name, hash)| (name.clone(), hex(hash)))
                .collect(),
            decoder: inputs.decoder,
            extractor: inputs.extractor,
            files: file_entries,
            format: MANIFEST_FORMAT,
            game: game_name(plan.game),
            index: IndexInput {
                path: INDEX_PATH,
                sha256: hex(&inputs.index_sha256),
            },
            passive_skill_graphs,
            schema: SchemaInput {
                commit: VENDORED_COMMIT,
                sha256: VENDORED_SHA256,
                source: SCHEMA_SOURCE,
            },
            stat_descriptions,
            steam_build_id: inputs.steam_build_id,
            table_dir: table_dir(plan.game),
            tables,
            text_language: TEXT_LANGUAGE,
        };
        Output { files, manifest }
    }
}

/// Column header of a table file. Fields are in alphabetical order, so the
/// JSON keys are sorted.
#[derive(Serialize)]
struct ColumnOut<'a> {
    array: bool,
    interval: bool,
    name: Option<&'a str>,
    references: Option<&'a str>,
    #[serde(rename = "type")]
    ty: &'static str,
}

impl<'a> ColumnOut<'a> {
    fn new(column: &'a Column) -> Self {
        let (array, interval, ty) = match column.kind {
            ColumnKind::Scalar(ty) => (false, false, ty.name()),
            ColumnKind::Interval(ty) => (false, true, ty.name()),
            ColumnKind::Array(ty) => (true, false, ty.name()),
            ColumnKind::UnknownArray => (true, false, "array"),
        };
        Self {
            array,
            interval,
            name: column.name.as_deref(),
            references: column.references.as_ref().map(|r| r.table.as_str()),
            ty,
        }
    }
}

/// Name of a column in findings: its schema name, or `#<position>` when it
/// has none; a name shared by several columns gets `#<position>` appended.
fn label(schema: &TableSchema, position: usize, column: &Column) -> String {
    match &column.name {
        Some(name)
            if schema
                .columns
                .iter()
                .filter(|c| c.name.as_ref() == Some(name))
                .count()
                == 1 =>
        {
            name.clone()
        }
        Some(name) => format!("{name}#{position}"),
        None => format!("#{position}"),
    }
}

/// Lowercase name of the table a key column's row indices point into: the
/// referenced table for `foreignrow`, the same table for `row`. `None` for
/// other columns and for keys that match a column instead of a row index.
fn key_target(schema: &TableSchema, column: &Column) -> Option<String> {
    let target = match element(column.kind)? {
        ScalarType::ForeignRow => column.references.as_ref()?,
        ScalarType::Row => match &column.references {
            Some(reference) => reference,
            None => return Some(schema.name.to_ascii_lowercase()),
        },
        _ => return None,
    };
    target
        .column
        .is_none()
        .then(|| target.table.to_ascii_lowercase())
}

/// Appends the non-null keys in `value` to `out`.
fn push_keys(value: &Value, out: &mut Vec<u64>) {
    match value {
        Value::Row(Some(key)) | Value::ForeignRow(Some(key)) => out.push(*key),
        Value::Array(items) => items.iter().for_each(|item| push_keys(item, out)),
        _ => {}
    }
}

/// Renders a table file; returns the bytes, the count of non-finite floats,
/// and the keys of each key column.
fn render(
    schema: &TableSchema,
    rows: &[Vec<Value>],
    columns_read: usize,
) -> Result<(Vec<u8>, u64, Vec<Keys>), serde_json::Error> {
    let columns: Vec<ColumnOut<'_>> = schema
        .columns
        .iter()
        .take(columns_read)
        .map(ColumnOut::new)
        .collect();
    let mut out = String::from("{\n\"columns\": ");
    out.push_str(&serde_json::to_string(&columns)?);
    out.push_str(",\n\"rows\": [");
    let mut non_finite = 0;
    for (i, row) in rows.iter().enumerate() {
        out.push_str(if i == 0 { "\n" } else { ",\n" });
        write_list(&mut out, row, &mut non_finite)?;
    }
    out.push_str(if rows.is_empty() { "],\n" } else { "\n],\n" });
    out.push_str("\"table\": ");
    out.push_str(&serde_json::to_string(&schema.name)?);
    out.push_str("\n}\n");

    let keys = schema
        .columns
        .iter()
        .enumerate()
        .take(columns_read)
        .filter_map(|(position, column)| {
            let target = key_target(schema, column)?;
            let mut values = Vec::new();
            for value in rows.iter().filter_map(|row| row.get(position)) {
                push_keys(value, &mut values);
            }
            Some(Keys {
                column: label(schema, position, column),
                target,
                values,
            })
        })
        .collect();
    Ok((out.into_bytes(), non_finite, keys))
}

/// Writes `values` as a JSON array.
fn write_list(
    out: &mut String,
    values: &[Value],
    non_finite: &mut u64,
) -> Result<(), serde_json::Error> {
    out.push('[');
    for (i, value) in values.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        write_value(out, value, non_finite)?;
    }
    out.push(']');
    Ok(())
}

/// Appends `value`'s `Display` text (locale-independent in Rust).
fn push(out: &mut String, value: impl Display) {
    // Writing to a `String` cannot fail.
    let _ = write!(out, "{value}");
}

/// Writes one value (format in the module docs).
fn write_value(
    out: &mut String,
    value: &Value,
    non_finite: &mut u64,
) -> Result<(), serde_json::Error> {
    match value {
        Value::Bool(v) => push(out, v),
        Value::I16(v) => push(out, v),
        Value::U16(v) => push(out, v),
        Value::I32(v) | Value::EnumRow(v) => push(out, v),
        Value::U32(v) => push(out, v),
        Value::F32(v) => write_f32(out, *v, non_finite)?,
        Value::String(v) => out.push_str(&serde_json::to_string(v)?),
        Value::Row(key) | Value::ForeignRow(key) => match key {
            Some(key) => push(out, key),
            None => out.push_str("null"),
        },
        Value::Interval(pair) => write_list(out, pair.as_slice(), non_finite)?,
        Value::Array(items) => write_list(out, items, non_finite)?,
        Value::UnknownArray { count, offset } => {
            push(
                out,
                format_args!("{{\"count\":{count},\"offset\":{offset}}}"),
            );
        }
    }
    Ok(())
}

/// Writes an `f32` as the shortest decimal that reads back to the same `f32`
/// (`serde_json` formats `f32` with `ryu` without widening). JSON has no
/// non-finite numbers, so those become strings and are counted.
fn write_f32(out: &mut String, value: f32, non_finite: &mut u64) -> Result<(), serde_json::Error> {
    if value.is_finite() {
        out.push_str(&serde_json::to_string(&value)?);
    } else {
        *non_finite += 1;
        out.push_str(if value.is_nan() {
            "\"NaN\""
        } else if value.is_sign_positive() {
            "\"Infinity\""
        } else {
            "\"-Infinity\""
        });
    }
    Ok(())
}

/// Facts about the inputs that the extraction itself cannot see.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inputs {
    /// Steam build ID of the install, if found.
    pub steam_build_id: Option<String>,
    /// SHA-256 of `_.index.bin`.
    pub index_sha256: [u8; 32],
    /// SHA-256 of each bundle read.
    pub bundles: BundleHashes,
    /// The Oodle decoder used.
    pub decoder: Decoder,
    /// The extractor build.
    pub extractor: Extractor,
}

/// The Oodle decoder used (ADR-005 Amendment 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decoder {
    /// Program file name, without its folder (no machine paths in output).
    pub program: String,
    /// What `<program> --version` printed, if it ran.
    pub version: Option<String>,
}

/// The extractor build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Extractor {
    /// Git commit of the source tree, if known.
    pub commit: Option<String>,
    /// Whether the source tree had changes (tracked or untracked), if known.
    pub dirty: Option<bool>,
    /// Crate version.
    pub version: String,
}

/// What came out: output files by path, and the manifest.
#[derive(Debug, Clone)]
pub struct Output {
    /// Table, stat-description, and graph files, by path inside the output
    /// directory.
    pub files: BTreeMap<String, Vec<u8>>,
    /// The manifest.
    pub manifest: Manifest,
}

/// Status of a planned table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Decoded with the schema's row width.
    Ok,
    /// Decoded, but the rows are not as wide as the schema says: see
    /// `row_size_mismatch` and `columns_read`. Bytes past the schema's
    /// columns in wider rows are not in the output.
    RowSizeMismatch,
    /// Did not decode: see `error`. No output file.
    Failed,
    /// Has a schema but no file in the table directory.
    NoFile,
    /// Referenced, but the schema does not describe it for this game.
    NoSchema,
}

/// The manifest: inputs, outputs, and findings. Fields of every manifest
/// struct are in alphabetical order, so the JSON keys are sorted (no
/// `serde_json::Map`, whose order depends on a feature flag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Manifest {
    /// SHA-256 of each bundle read, by name.
    pub bundles: BTreeMap<String, String>,
    /// The Oodle decoder.
    pub decoder: Decoder,
    /// The extractor build.
    pub extractor: Extractor,
    /// Every table file written, by path.
    pub files: BTreeMap<String, OutputFile>,
    /// [`MANIFEST_FORMAT`].
    pub format: u32,
    /// `poe1` or `poe2`.
    pub game: &'static str,
    /// The index read.
    pub index: IndexInput,
    /// Passive skill graph files and their counts.
    pub passive_skill_graphs: GraphManifest,
    /// The vendored schema.
    pub schema: SchemaInput,
    /// Stat-description files, their counts, and every line not parsed.
    pub stat_descriptions: StatDescManifest,
    /// Steam build ID of the install, if found.
    pub steam_build_id: Option<String>,
    /// Folder of the tables in the index.
    pub table_dir: &'static str,
    /// Every planned or referenced table, by name.
    pub tables: BTreeMap<String, TableEntry>,
    /// [`TEXT_LANGUAGE`].
    pub text_language: &'static str,
}

impl Manifest {
    /// The manifest as pretty JSON with a final newline.
    ///
    /// # Errors
    /// [`ExtractError::Json`] if serialisation fails.
    pub fn to_json(&self) -> Result<Vec<u8>, ExtractError> {
        let mut json = serde_json::to_vec_pretty(self)?;
        json.push(b'\n');
        Ok(json)
    }

    /// Problems that make the run fail: a root table that is not `ok` (not
    /// decoded, or decoded with a row-size mismatch), any table that failed
    /// to decode, any key column with keys past the end of its target, and
    /// the stat-description and graph problems ([`StatDescManifest::problems`],
    /// [`GraphManifest::problems`]).
    /// Everything else, stat-description findings included, is recorded in
    /// the manifest only.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = self.stat_descriptions.problems();
        problems.extend(self.passive_skill_graphs.problems());
        for (name, table) in &self.tables {
            if table.status == Status::Failed || (table.root && table.status != Status::Ok) {
                problems.push(format!("{name}: {:?}", table.status));
            }
            for (column, check) in &table.key_columns {
                if let Some(n @ 1..) = check.dangling {
                    problems.push(format!(
                        "{name}.{column}: {n} dangling keys into {}",
                        check.target
                    ));
                }
            }
        }
        problems
    }
}

/// One output file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OutputFile {
    /// Rows in the file.
    pub rows: usize,
    /// SHA-256 of the file.
    pub sha256: String,
}

/// The index read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IndexInput {
    /// Path inside the install.
    pub path: &'static str,
    /// SHA-256 of the file.
    pub sha256: String,
}

/// The vendored schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaInput {
    /// Upstream source commit.
    pub commit: &'static str,
    /// SHA-256 of the vendored file.
    pub sha256: &'static str,
    /// Upstream repository.
    pub source: &'static str,
}

/// A row-size mismatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Mismatch {
    /// Width found in the file.
    pub actual: usize,
    /// Width from the schema.
    pub expected: usize,
}

/// Range check of one key column.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyCheck {
    /// Keys at or past the target's row count; `None` if the target was not
    /// decoded, so the keys could not be checked.
    pub dangling: Option<usize>,
    /// Target table.
    pub target: String,
    /// Non-null keys in the column.
    pub values: usize,
}

/// One planned or referenced table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TableEntry {
    /// Columns in the schema, if there is one.
    pub columns: Option<usize>,
    /// Leading columns decoded (fewer than `columns` only for narrower rows).
    pub columns_read: Option<usize>,
    /// Why the table did not decode.
    pub error: Option<TableError>,
    /// Range check of each key column decoded, by column name.
    pub key_columns: BTreeMap<String, KeyCheck>,
    /// `f32` values that are NaN or infinite (written as strings).
    pub non_finite_floats: u64,
    /// Output file path.
    pub output: Option<String>,
    /// Planned tables that reference this one.
    pub referenced_by: BTreeSet<String>,
    /// One of the requested tables (not only referenced).
    pub root: bool,
    /// Row width in the file.
    pub row_len: Option<usize>,
    /// Set when the row width differs from the schema.
    pub row_size_mismatch: Option<Mismatch>,
    /// Rows decoded.
    pub rows: Option<usize>,
    /// Path of the table in the index.
    pub source: Option<String>,
    /// SHA-256 of the table file as read from its (decoded) bundle.
    pub source_sha256: Option<String>,
    /// Outcome.
    pub status: Status,
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;
    use crate::dat::{BOUNDARY, NULL_KEY};

    /// Root -> Child (scalar and array keys), Root -> Absent (schema, no
    /// file), Child -> Missing (no schema), Root -> enum Kind (not a table).
    const SCHEMA: &str = r#"{
      "version": 7, "createdAt": 1,
      "tables": [
        {"validFor": 3, "name": "Root", "tags": [], "columns": [
          {"name": "Id", "description": null, "array": false, "type": "string",
           "unique": true, "localized": false, "references": null, "until": null,
           "file": null, "files": null, "interval": false},
          {"name": "Child", "description": null, "array": false, "type": "foreignrow",
           "unique": false, "localized": false, "references": {"table": "Child"},
           "until": null, "file": null, "files": null, "interval": false},
          {"name": "Children", "description": null, "array": true, "type": "foreignrow",
           "unique": false, "localized": false, "references": {"table": "Child"},
           "until": null, "file": null, "files": null, "interval": false},
          {"name": null, "description": null, "array": false, "type": "foreignrow",
           "unique": false, "localized": false, "references": {"table": "Absent"},
           "until": null, "file": null, "files": null, "interval": false},
          {"name": "Weight", "description": null, "array": false, "type": "f32",
           "unique": false, "localized": false, "references": null, "until": null,
           "file": null, "files": null, "interval": false},
          {"name": "Kind", "description": null, "array": false, "type": "enumrow",
           "unique": false, "localized": false, "references": {"table": "Kind"},
           "until": null, "file": null, "files": null, "interval": false}
        ]},
        {"validFor": 1, "name": "Child", "tags": [], "columns": [
          {"name": "Parent", "description": null, "array": false, "type": "foreignrow",
           "unique": false, "localized": false, "references": {"table": "Missing"},
           "until": null, "file": null, "files": null, "interval": false}
        ]},
        {"validFor": 3, "name": "Absent", "tags": [], "columns": []},
        {"validFor": 3, "name": "Unrelated", "tags": [], "columns": []}
      ],
      "enumerations": [
        {"validFor": 3, "name": "Kind", "indexing": 0, "enumerators": ["A"]}
      ]
    }"#;

    fn schema() -> Schema {
        Schema::parse(SCHEMA.as_bytes()).unwrap()
    }

    fn u64s(words: &[u64]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// `.datc64` bytes from raw rows and the variable section after the boundary.
    fn dat(rows: &[Vec<u8>], variable: &[u8]) -> Vec<u8> {
        let mut out = u32::try_from(rows.len()).unwrap().to_le_bytes().to_vec();
        for row in rows {
            out.extend_from_slice(row);
        }
        out.extend_from_slice(&BOUNDARY);
        out.extend_from_slice(variable);
        out
    }

    /// UTF-16LE text plus terminator.
    fn utf16(text: &str) -> Vec<u8> {
        let mut out: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        out.extend_from_slice(&[0; 4]);
        out
    }

    /// Two Root rows. Row 0: Child 1, Children [0, 7], Absent 0, weight 0.1,
    /// kind 0. Row 1: null keys, empty array, weight NaN, kind 0.
    fn root_dat() -> Vec<u8> {
        // Variable section, offsets from the boundary start (8).
        let mut variable = utf16("a\"\u{e9}"); // at 8, 10 bytes
        let second = 8 + u64::try_from(variable.len()).unwrap();
        variable.extend(utf16("")); // at 18
        let keys = 8 + u64::try_from(variable.len()).unwrap();
        variable.extend(u64s(&[0, 0, 7, 0]));

        let mut row0 = u64s(&[8, 1, 0, 2, keys, 0, 0]);
        row0.extend(0.1_f32.to_le_bytes());
        row0.extend(0_i32.to_le_bytes());
        let mut row1 = u64s(&[second, NULL_KEY, NULL_KEY, 0, 0, NULL_KEY, NULL_KEY]);
        row1.extend(f32::NAN.to_le_bytes());
        row1.extend(0_i32.to_le_bytes());
        dat(&[row0, row1], &variable)
    }

    /// Two Child rows with `Missing` keys 0 and 1.
    fn child_dat() -> Vec<u8> {
        dat(&[u64s(&[0, 0]), u64s(&[1, 0])], &[])
    }

    fn inputs() -> Inputs {
        Inputs {
            steam_build_id: Some("123".to_owned()),
            index_sha256: [0xab; 32],
            bundles: [("Folders/b".to_owned(), [1; 32])].into_iter().collect(),
            decoder: Decoder {
                program: "unoodle".to_owned(),
                version: Some("oozextract 0.5.7".to_owned()),
            },
            extractor: Extractor {
                commit: Some("c0ffee".to_owned()),
                dirty: Some(false),
                version: "0.0.0".to_owned(),
            },
        }
    }

    const STAT_PATH: &str = "metadata/statdescriptions/stat_descriptions.txt";

    /// A stat-description file (UTF-16LE with BOM): one description with two
    /// English variants and a French list, then a line that is not parsed.
    fn stat_file() -> Vec<u8> {
        let text = "\u{feff}description\r\n\t1 level\r\n\t2\r\n\
                    \t\t1|# \"Level {0}\" reminderstring ReminderTextLevel\r\n\
                    \t\t#|-1 \"Minus {0}\" negate 1\r\n\
                    \tlang \"French\"\r\n\t1\r\n\t\t# \"Niveau {0}\"\r\n\
                    \r\nbogus line\r\n";
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    const GRAPH_PATH: &str = "metadata/passiveskillgraph.psg";

    /// An empty PoE1 passive skill graph: version, kind, no orbits, no
    /// roots, no groups.
    const GRAPH_FILE: [u8; 11] = [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    /// Extracts `tables` plus [`stat_file`] and [`GRAPH_FILE`], so the run
    /// has the stat descriptions and the graph it needs.
    fn extract(schema: &Schema, tables: &[(&str, &[u8])]) -> Output {
        let mut extraction = Extraction::new(Plan::new(schema, Game::Poe1, &["root"]));
        for (path, bytes) in tables {
            extraction.add(path, bytes).unwrap();
        }
        extraction.add(STAT_PATH, &stat_file()).unwrap();
        extraction.add(GRAPH_PATH, &GRAPH_FILE).unwrap();
        extraction.finish(inputs())
    }

    #[test]
    fn plan_should_follow_table_references_transitively() {
        let schema = schema();
        let plan = Plan::new(&schema, Game::Poe1, &["ROOT"]);
        let names: Vec<_> = plan.tables.values().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Absent", "Child", "Missing", "Root"]);
        assert!(plan.tables["root"].root);
        assert!(!plan.tables["child"].root);
        assert_eq!(
            plan.tables["child"].referenced_by,
            BTreeSet::from(["Root".to_owned()])
        );
        assert!(plan.tables["missing"].schema.is_none());
        assert_eq!(
            plan.sources().collect::<Vec<_>>(),
            [
                "data/absent.datc64",
                "data/child.datc64",
                "data/root.datc64"
            ]
        );

        // Child has no PoE2 schema, so Missing is not reached.
        let plan = Plan::new(&schema, Game::Poe2, &["Root"]);
        let names: Vec<_> = plan.tables.values().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Absent", "Child", "Root"]);
        assert_eq!(
            plan.sources().next().as_deref(),
            Some("data/balance/absent.datc64")
        );
    }

    #[test]
    fn extract_should_render_canonical_table_json() {
        let schema = schema();
        let out = extract(
            &schema,
            &[
                ("data/root.datc64", &root_dat()),
                ("Data/Child.datc64", &child_dat()),
            ],
        );
        let root = String::from_utf8(out.files["tables/Root.json"].clone()).unwrap();
        let expected = concat!(
            "{\n\"columns\": [",
            r#"{"array":false,"interval":false,"name":"Id","references":null,"type":"string"},"#,
            r#"{"array":false,"interval":false,"name":"Child","references":"Child","type":"foreignrow"},"#,
            r#"{"array":true,"interval":false,"name":"Children","references":"Child","type":"foreignrow"},"#,
            r#"{"array":false,"interval":false,"name":null,"references":"Absent","type":"foreignrow"},"#,
            r#"{"array":false,"interval":false,"name":"Weight","references":null,"type":"f32"},"#,
            r#"{"array":false,"interval":false,"name":"Kind","references":"Kind","type":"enumrow"}"#,
            "],\n\"rows\": [\n",
            "[\"a\\\"\u{e9}\",1,[0,7],0,0.1,0],\n",
            "[\"\",null,[],null,\"NaN\",0]\n",
            "],\n\"table\": \"Root\"\n}\n",
        );
        assert_eq!(root, expected);
        let child = String::from_utf8(out.files["tables/Child.json"].clone()).unwrap();
        assert!(child.ends_with("\"rows\": [\n[0],\n[1]\n],\n\"table\": \"Child\"\n}\n"));
    }

    #[test]
    fn write_value_should_keep_numbers_exact() {
        let mut out = String::new();
        let mut non_finite = 0;
        for value in [
            Value::F32(0.1),
            Value::F32(-0.0),
            Value::F32(1.0),
            Value::F32(1e-7),
            Value::F32(f32::MAX),
            Value::F32(f32::INFINITY),
            Value::F32(f32::NEG_INFINITY),
            Value::ForeignRow(Some(u64::MAX - 1)),
            Value::ForeignRow(Some((1 << 53) + 1)),
            Value::I16(i16::MIN),
            Value::U32(u32::MAX),
            Value::Bool(true),
            Value::Interval(Box::new([Value::I32(-1), Value::I32(2)])),
            Value::UnknownArray {
                count: 3,
                offset: 16,
            },
        ] {
            write_value(&mut out, &value, &mut non_finite).unwrap();
            out.push(' ');
        }
        assert_eq!(
            out,
            "0.1 -0.0 1.0 1e-7 3.4028235e+38 \"Infinity\" \"-Infinity\" \
             18446744073709551614 9007199254740993 -32768 4294967295 true [-1,2] \
             {\"count\":3,\"offset\":16} "
        );
        assert_eq!(non_finite, 2);
    }

    #[test]
    fn extract_should_not_depend_on_add_order() {
        let schema = schema();
        let (root, child, absent) = (root_dat(), child_dat(), dat(&[], &[]));
        // A second stat-description file, which includes the first.
        let other: Vec<u8> = "include \"Metadata/StatDescriptions/stat_descriptions.txt\"\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let files: [(&str, &[u8]); 4] = [
            ("data/root.datc64", &root),
            ("data/child.datc64", &child),
            ("data/absent.datc64", &absent),
            ("metadata/statdescriptions/b/other.txt", &other),
        ];
        // Every rotation, forwards and backwards.
        let mut orders = Vec::new();
        for start in 0..4 {
            let forward: [usize; 4] = std::array::from_fn(|i| (start + i) % 4);
            let mut backward = forward;
            backward.reverse();
            orders.extend([forward, backward]);
        }
        let outputs: Vec<_> = orders
            .iter()
            .map(|order| {
                let out = extract(&schema, &order.map(|i| files[i]));
                (out.files, out.manifest.to_json().unwrap())
            })
            .collect();
        assert_eq!(outputs[0].0.len(), 6);
        let manifest = String::from_utf8(outputs[0].1.clone()).unwrap();
        assert!(manifest.contains("\"stat_descriptions/b/other.txt.json\""));
        assert!(!manifest.contains("missing-include"));
        for other in &outputs[1..] {
            assert_eq!(other, &outputs[0]);
        }
    }

    #[test]
    fn key_target_should_follow_row_and_foreignrow_columns() {
        let table = schema().tables[0].clone();
        let column = |kind, references: Option<(&str, Option<&str>)>| Column {
            name: None,
            kind,
            references: references.map(|(table, column)| crate::schema::Reference {
                table: table.to_owned(),
                column: column.map(str::to_owned),
            }),
            localized: false,
            unique: false,
        };
        use ColumnKind::{Array, Interval, Scalar};
        use ScalarType as T;
        let cases = [
            (column(Scalar(T::Row), None), Some("root")),
            (column(Scalar(T::Row), Some(("Other", None))), Some("other")),
            (column(Array(T::Row), None), Some("root")),
            (
                column(Scalar(T::ForeignRow), Some(("Child", None))),
                Some("child"),
            ),
            (column(Scalar(T::ForeignRow), None), None),
            (
                column(Scalar(T::ForeignRow), Some(("Child", Some("Id")))),
                None,
            ),
            (column(Scalar(T::String), Some(("Child", Some("Id")))), None),
            (column(Scalar(T::EnumRow), Some(("Kind", None))), None),
            (column(Interval(T::I32), None), None),
            (column(ColumnKind::UnknownArray, None), None),
        ];
        for (column, expected) in cases {
            assert_eq!(
                key_target(&table, &column).as_deref(),
                expected,
                "{column:?}"
            );
        }
    }

    #[test]
    fn finish_should_count_dangling_self_keys_and_huge_keys() {
        // Child with a `row` column (self keys) and three rows: 1 and 2 are in
        // range; u64::MAX - 1 is not (and does not fit a usize on 32-bit).
        let json = SCHEMA
            .replace(
                r#""references": {"table": "Missing"}"#,
                r#""references": null"#,
            )
            .replace(
                r#"{"name": "Parent", "description": null, "array": false, "type": "foreignrow""#,
                r#"{"name": "Parent", "description": null, "array": false, "type": "row""#,
            );
        let schema = Schema::parse(json.as_bytes()).unwrap();
        let child = dat(&[u64s(&[1]), u64s(&[2]), u64s(&[u64::MAX - 1])], &[]);
        let out = extract(
            &schema,
            &[
                ("data/root.datc64", &root_dat()),
                ("data/child.datc64", &child),
            ],
        );
        let check = &out.manifest.tables["Child"].key_columns["Parent"];
        assert_eq!(
            check,
            &KeyCheck {
                dangling: Some(1),
                target: "Child".to_owned(),
                values: 3
            }
        );
        // Root's Children key 7 is past Child's three rows as well.
        assert_eq!(
            out.manifest.problems(),
            [
                "Child.Parent: 1 dangling keys into Child",
                "Root.Children: 1 dangling keys into Child",
            ]
        );
    }

    #[test]
    fn write_value_should_count_non_finite_floats_in_intervals_and_arrays() {
        let mut out = String::new();
        let mut non_finite = 0;
        let value = Value::Array(vec![
            Value::Interval(Box::new([Value::F32(f32::NAN), Value::F32(1.5)])),
            Value::F32(f32::INFINITY),
        ]);
        write_value(&mut out, &value, &mut non_finite).unwrap();
        assert_eq!(out, "[[\"NaN\",1.5],\"Infinity\"]");
        assert_eq!(non_finite, 2);
    }

    #[test]
    fn manifest_should_match_golden_with_sorted_keys() {
        let schema = schema();
        let out = extract(
            &schema,
            &[
                ("data/root.datc64", &root_dat()),
                ("data/child.datc64", &child_dat()),
            ],
        );
        let json = String::from_utf8(out.manifest.to_json().unwrap()).unwrap();
        let root_sha = hex(&Sha256::digest(&out.files["tables/Root.json"]));
        let child_sha = hex(&Sha256::digest(&out.files["tables/Child.json"]));
        let sd_sha = hex(&Sha256::digest(
            &out.files["stat_descriptions/stat_descriptions.txt.json"],
        ));
        let graph_json = &out.files["passive_skill_graphs/metadata/passiveskillgraph.psg.json"];
        assert_eq!(
            graph_json.as_slice(),
            b"{
\"groups\": [],
\"kind\": 0,
\"orbit_slots\": [],
\"roots\": [],
\"version\": 3
}
"
        );
        let expected = GOLDEN
            .replace("{graph_sha}", &hex(&Sha256::digest(graph_json)))
            .replace("{graph_src}", &hex(&Sha256::digest(GRAPH_FILE)))
            .replace("{root_sha}", &root_sha)
            .replace("{child_sha}", &child_sha)
            .replace("{sd_sha}", &sd_sha)
            .replace("{sd_src}", &hex(&Sha256::digest(stat_file())))
            .replace("{root_src}", &hex(&Sha256::digest(root_dat())))
            .replace("{child_src}", &hex(&Sha256::digest(child_dat())))
            .replace("{schema_commit}", VENDORED_COMMIT)
            .replace("{schema_sha}", VENDORED_SHA256);
        assert_eq!(json, expected);
    }

    /// The manifest of [`manifest_should_match_golden_with_sorted_keys`]:
    /// keys sorted at every level; Child's keys point into Missing (no
    /// schema) so cannot be checked; Root's Children has one key (7) past
    /// Child's two rows.
    const GOLDEN: &str = r##"{
  "bundles": {
    "Folders/b": "0101010101010101010101010101010101010101010101010101010101010101"
  },
  "decoder": {
    "program": "unoodle",
    "version": "oozextract 0.5.7"
  },
  "extractor": {
    "commit": "c0ffee",
    "dirty": false,
    "version": "0.0.0"
  },
  "files": {
    "passive_skill_graphs/metadata/passiveskillgraph.psg.json": {
      "rows": 0,
      "sha256": "{graph_sha}"
    },
    "stat_descriptions/stat_descriptions.txt.json": {
      "rows": 1,
      "sha256": "{sd_sha}"
    },
    "tables/Child.json": {
      "rows": 2,
      "sha256": "{child_sha}"
    },
    "tables/Root.json": {
      "rows": 2,
      "sha256": "{root_sha}"
    }
  },
  "format": 3,
  "game": "poe1",
  "index": {
    "path": "Bundles2/_.index.bin",
    "sha256": "abababababababababababababababababababababababababababababababab"
  },
  "passive_skill_graphs": {
    "dir": "passive_skill_graphs",
    "files": {
      "metadata/passiveskillgraph.psg": {
        "connections": 0,
        "dangling_connections": 0,
        "duplicate_nodes": 0,
        "error": null,
        "groups": 0,
        "kind": 0,
        "nodes": 0,
        "output": "passive_skill_graphs/metadata/passiveskillgraph.psg.json",
        "roots": 0,
        "source": "metadata/passiveskillgraph.psg",
        "source_sha256": "{graph_src}",
        "status": "ok"
      }
    }
  },
  "schema": {
    "commit": "{schema_commit}",
    "sha256": "{schema_sha}",
    "source": "poe-tool-dev/dat-schema"
  },
  "stat_descriptions": {
    "dir": "metadata/statdescriptions/",
    "files": {
      "metadata/statdescriptions/stat_descriptions.txt": {
        "descriptions": 1,
        "english_variants": 2,
        "error": null,
        "findings": 1,
        "includes": [],
        "kind": "descriptions",
        "languages": {
          "French": {
            "descriptions": 1,
            "variants": 1
          }
        },
        "no_description": 0,
        "output": "stat_descriptions/stat_descriptions.txt.json",
        "source": "metadata/statdescriptions/stat_descriptions.txt",
        "source_sha256": "{sd_src}",
        "status": "ok"
      }
    },
    "findings": [
      {
        "file": "metadata/statdescriptions/stat_descriptions.txt",
        "kind": "unknown-line",
        "line": 10,
        "skipped": 1,
        "text": "bogus line"
      }
    ],
    "findings_by_kind": {
      "unknown-line": 1
    },
    "handlers": {
      "negate": 1,
      "reminderstring": 1
    },
    "variant_flags": {}
  },
  "steam_build_id": "123",
  "table_dir": "data/",
  "tables": {
    "Absent": {
      "columns": 0,
      "columns_read": null,
      "error": null,
      "key_columns": {},
      "non_finite_floats": 0,
      "output": null,
      "referenced_by": [
        "Root"
      ],
      "root": false,
      "row_len": null,
      "row_size_mismatch": null,
      "rows": null,
      "source": null,
      "source_sha256": null,
      "status": "no-file"
    },
    "Child": {
      "columns": 1,
      "columns_read": 1,
      "error": null,
      "key_columns": {
        "Parent": {
          "dangling": null,
          "target": "Missing",
          "values": 2
        }
      },
      "non_finite_floats": 0,
      "output": "tables/Child.json",
      "referenced_by": [
        "Root"
      ],
      "root": false,
      "row_len": 16,
      "row_size_mismatch": null,
      "rows": 2,
      "source": "data/child.datc64",
      "source_sha256": "{child_src}",
      "status": "ok"
    },
    "Missing": {
      "columns": null,
      "columns_read": null,
      "error": null,
      "key_columns": {},
      "non_finite_floats": 0,
      "output": null,
      "referenced_by": [
        "Child"
      ],
      "root": false,
      "row_len": null,
      "row_size_mismatch": null,
      "rows": null,
      "source": null,
      "source_sha256": null,
      "status": "no-schema"
    },
    "Root": {
      "columns": 6,
      "columns_read": 6,
      "error": null,
      "key_columns": {
        "#3": {
          "dangling": null,
          "target": "Absent",
          "values": 1
        },
        "Child": {
          "dangling": 0,
          "target": "Child",
          "values": 1
        },
        "Children": {
          "dangling": 1,
          "target": "Child",
          "values": 2
        }
      },
      "non_finite_floats": 1,
      "output": "tables/Root.json",
      "referenced_by": [],
      "root": true,
      "row_len": 64,
      "row_size_mismatch": null,
      "rows": 2,
      "source": "data/root.datc64",
      "source_sha256": "{root_src}",
      "status": "ok"
    }
  },
  "text_language": "English"
}
"##;

    #[test]
    fn extract_should_report_failed_and_mismatched_tables() {
        let schema = schema();
        // Child rows 24 bytes wide instead of 16; Root's string offset is bad.
        let wide_child = dat(&[u64s(&[0, 0, 9]), u64s(&[1, 0, 9])], &[]);
        let mut bad_root = root_dat();
        bad_root[4] = 0xFF;
        let out = extract(
            &schema,
            &[
                ("data/root.datc64", &bad_root),
                ("data/child.datc64", &wide_child),
            ],
        );
        let root = &out.manifest.tables["Root"];
        assert_eq!(root.status, Status::Failed);
        assert_eq!(
            root.error.as_ref().map(|e| e.code),
            Some("EXTRACT_DAT_STRING_OUT_OF_RANGE")
        );
        assert_eq!(root.output, None);
        let child = &out.manifest.tables["Child"];
        assert_eq!(child.status, Status::RowSizeMismatch);
        assert_eq!(
            child.row_size_mismatch,
            Some(Mismatch {
                actual: 24,
                expected: 16
            })
        );
        assert_eq!(child.columns_read, Some(1));
        assert_eq!(
            out.files.keys().collect::<Vec<_>>(),
            [
                "passive_skill_graphs/metadata/passiveskillgraph.psg.json",
                "stat_descriptions/stat_descriptions.txt.json",
                "tables/Child.json"
            ]
        );
        assert_eq!(out.manifest.problems(), ["Root: Failed"]);
    }

    #[test]
    fn problems_should_flag_missing_root() {
        let schema = schema();
        let out = extract(&schema, &[("data/child.datc64", &child_dat())]);
        assert_eq!(out.manifest.problems(), ["Root: NoFile"]);
        let out = extract(
            &schema,
            &[
                ("data/root.datc64", &root_dat()),
                ("data/child.datc64", &child_dat()),
            ],
        );
        assert_eq!(
            out.manifest.problems(),
            ["Root.Children: 1 dangling keys into Child"]
        );

        // A root with a row-size mismatch is a problem; a clean root is not.
        let wide_child = dat(&[u64s(&[0, 0, 9])], &[]);
        for (bytes, expected) in [
            (wide_child, vec!["Child: RowSizeMismatch"]),
            (child_dat(), vec![]),
        ] {
            let mut extraction = Extraction::new(Plan::new(&schema, Game::Poe1, &["Child"]));
            extraction.add("data/child.datc64", &bytes).unwrap();
            extraction.add(STAT_PATH, &stat_file()).unwrap();
            extraction.add(GRAPH_PATH, &GRAPH_FILE).unwrap();
            let problems = extraction.finish(inputs()).manifest.problems();
            // Child's keys point into Missing, which has no schema: unchecked.
            assert_eq!(problems, expected);
        }
    }

    #[test]
    fn problems_should_flag_missing_and_undecodable_stat_descriptions() {
        let schema = schema();
        let run = |files: &[(&str, &[u8])]| {
            let mut extraction = Extraction::new(Plan::new(&schema, Game::Poe1, &["Child"]));
            extraction.add("data/child.datc64", &child_dat()).unwrap();
            extraction.add(GRAPH_PATH, &GRAPH_FILE).unwrap();
            for (path, bytes) in files {
                extraction.add(path, bytes).unwrap();
            }
            extraction.finish(inputs())
        };
        // A filter file alone is not enough; findings are not problems.
        let filters = "metadata/statdescriptions/skillpopup_stat_filters.txt";
        assert_eq!(
            run(&[(filters, b"")]).manifest.problems(),
            ["no stat-description files in metadata/statdescriptions/"]
        );
        let out = run(&[(STAT_PATH, &stat_file()), (filters, b"")]);
        assert_eq!(out.manifest.problems(), Vec::<String>::new());
        assert_eq!(out.manifest.stat_descriptions.findings.len(), 1);
        // An odd byte count is not UTF-16.
        let out = run(&[(STAT_PATH, b"abc")]);
        assert_eq!(out.manifest.problems(), [format!("{STAT_PATH}: Failed")]);
        let entry = &out.manifest.stat_descriptions.files[STAT_PATH];
        assert_eq!(
            entry.error.as_ref().map(|e| e.code),
            Some("EXTRACT_STATDESC_ODD_LENGTH")
        );
        assert_eq!(entry.output, None);
        assert!(
            !out.files
                .keys()
                .any(|k| k.starts_with("stat_descriptions/"))
        );
    }

    #[test]
    fn add_should_reject_unplanned_and_duplicate_paths() {
        let schema = schema();
        let mut extraction = Extraction::new(Plan::new(&schema, Game::Poe1, &["Root"]));
        for path in [
            "data/unrelated.datc64",
            "data/missing.datc64",
            "data/french/root.datc64",
            "data/root.dat",
            "other/root.datc64",
        ] {
            let err = extraction.add(path, &[]).unwrap_err();
            assert_eq!(err.code(), "EXTRACT_UNPLANNED_TABLE", "{path}");
        }
        extraction.add("data/child.datc64", &child_dat()).unwrap();
        let err = extraction
            .add("data/CHILD.datc64", &child_dat())
            .unwrap_err();
        assert_eq!(err.code(), "EXTRACT_DUPLICATE_TABLE");
    }

    #[test]
    fn label_should_disambiguate_shared_and_missing_names() {
        let mut schema = schema().tables[0].clone();
        assert_eq!(label(&schema, 0, &schema.columns[0]), "Id");
        assert_eq!(label(&schema, 3, &schema.columns[3]), "#3");
        schema.columns[1].name = Some("Id".to_owned());
        assert_eq!(label(&schema, 1, &schema.columns[1]), "Id#1");
    }
}
