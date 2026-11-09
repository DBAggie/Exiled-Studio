//! `.datc64` tables: a `u32` row count, fixed-width rows, an 8-byte `0xBB`
//! boundary, then a variable section that strings and arrays point into
//! (offsets count from the start of the boundary). Layout in
//! `docs/spikes/spike-a-extraction.md`, section 6.
//!
//! The file does not store its row width. [`DatFile::parse`] takes the width
//! the schema expects and uses it when the boundary is where that width puts
//! it; otherwise it finds the boundary itself and [`read_table`] reports a
//! [`RowSizeMismatch`] and decodes only the columns that fit (ADR-015: what is
//! not decoded is reported, not dropped).

use crate::reader::{Reader, Short, widen};
use crate::schema::{ColumnKind, ScalarType, TableSchema};

/// Marks the end of the rows and the start of the variable section.
pub const BOUNDARY: [u8; 8] = [0xBB; 8];

/// A null `row` or `foreignrow` key.
pub const NULL_KEY: u64 = 0xFEFE_FEFE_FEFE_FEFE;

/// One decoded value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `bool`.
    Bool(bool),
    /// `i16`.
    I16(i16),
    /// `u16`.
    U16(u16),
    /// `i32`.
    I32(i32),
    /// `u32`.
    U32(u32),
    /// `f32`.
    F32(f32),
    /// `string`.
    String(String),
    /// `row`: a row of the same table, `None` if null.
    Row(Option<u64>),
    /// `foreignrow`: a row of another table, `None` if null.
    ForeignRow(Option<u64>),
    /// `enumrow`: the stored enumerator value (see the enumeration's `indexing`).
    EnumRow(i32),
    /// Two numbers.
    Interval(Box<[Value; 2]>),
    /// Array elements.
    Array(Vec<Value>),
    /// An array of unknown element type: what the row stores about it.
    UnknownArray {
        /// Element count.
        count: u64,
        /// Offset in the variable section.
        offset: u64,
    },
}

/// The rows are not as wide as the schema says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowSizeMismatch {
    /// Width from the schema.
    pub expected: usize,
    /// Width found in the file.
    pub actual: usize,
}

/// A decoded table.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// Width of each row in the file.
    pub row_len: usize,
    /// Set when `row_len` differs from the schema's width.
    pub mismatch: Option<RowSizeMismatch>,
    /// Leading schema columns that fit in `row_len` and were decoded; the rest
    /// were not (only with a mismatch that makes rows shorter).
    pub columns_read: usize,
    /// One entry per row, each with `columns_read` values.
    pub rows: Vec<Vec<Value>>,
}

/// Decoded values plus string units allowed per input byte. Real tables
/// stay far below this (section 9); it stops a corrupt table whose rows all
/// point at one large array or string from multiplying its output.
pub const DECODE_BUDGET_PER_BYTE: usize = 8;

/// What is wrong with one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ValueError {
    /// A `bool` byte other than 0 or 1.
    #[error("bool byte {0} is not 0 or 1")]
    BadBool(u8),
    /// A string offset outside the variable section, inside the boundary,
    /// or odd (UTF-16 text starts at an even offset).
    #[error("string offset {offset} is outside the variable data or misaligned")]
    StringOutOfRange {
        /// The offset.
        offset: u64,
    },
    /// No terminator (two zero UTF-16 units) before the end of the section.
    #[error("string at offset {offset} has no terminator")]
    StringUnterminated {
        /// The offset.
        offset: u64,
    },
    /// The string is not valid UTF-16.
    #[error("string at offset {offset} is not valid UTF-16")]
    StringNotUtf16 {
        /// The offset.
        offset: u64,
    },
    /// Array elements outside the variable section.
    #[error("array of {count} elements at offset {offset} is outside the variable data")]
    ArrayOutOfRange {
        /// Element count.
        count: u64,
        /// Offset.
        offset: u64,
    },
    /// A `foreignrow` whose second word is not 0 (key set) or null (key
    /// null), as every value in both installs is (section 6). Usually means
    /// the columns are misaligned.
    #[error("foreign key {key:#x} has second word {second:#x}")]
    BadForeignKey {
        /// First word (the key).
        key: u64,
        /// Second word.
        second: u64,
    },
    /// The table decodes to more than [`DECODE_BUDGET_PER_BYTE`] values and
    /// string units per input byte.
    #[error("decoded output exceeds {DECODE_BUDGET_PER_BYTE} values per input byte")]
    OutputTooLarge,
}

/// Errors from reading a table. Each variant has a stable [`code`](Self::code).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DatError {
    /// The file ends before the row count.
    #[error(
        "dat truncated: {} at offset {} needs {} bytes, {} left",
        .0.field, .0.offset, .0.needed, .0.available
    )]
    Truncated(Short),
    /// No boundary at a position that is a whole number of rows.
    #[error("dat has no row boundary for {rows} rows")]
    NoBoundary {
        /// Row count from the header.
        rows: u32,
    },
    /// A value could not be decoded.
    #[error("dat row {row} column {column}: {kind}")]
    Value {
        /// Row index.
        row: usize,
        /// Column position in the schema.
        column: usize,
        /// What is wrong.
        kind: ValueError,
    },
}

impl DatError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Truncated(_) => "EXTRACT_DAT_TRUNCATED",
            Self::NoBoundary { .. } => "EXTRACT_DAT_NO_BOUNDARY",
            Self::Value { kind, .. } => match kind {
                ValueError::BadBool(_) => "EXTRACT_DAT_BAD_BOOL",
                ValueError::StringOutOfRange { .. } => "EXTRACT_DAT_STRING_OUT_OF_RANGE",
                ValueError::StringUnterminated { .. } => "EXTRACT_DAT_STRING_UNTERMINATED",
                ValueError::StringNotUtf16 { .. } => "EXTRACT_DAT_STRING_NOT_UTF16",
                ValueError::ArrayOutOfRange { .. } => "EXTRACT_DAT_ARRAY_OUT_OF_RANGE",
                ValueError::BadForeignKey { .. } => "EXTRACT_DAT_BAD_FOREIGN_KEY",
                ValueError::OutputTooLarge => "EXTRACT_DAT_TOO_LARGE",
            },
        }
    }
}

/// A table split into rows and the variable section, not decoded yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatFile<'a> {
    rows: u32,
    row_len: usize,
    fixed: &'a [u8],
    variable: &'a [u8],
}

impl<'a> DatFile<'a> {
    /// Splits a `.datc64` file.
    ///
    /// If `expected_row_len` puts the boundary where it is, rows have that
    /// width. Otherwise the first boundary at a whole, non-zero number of
    /// bytes per row wins (a table with no rows must start with it). Both can
    /// in principle hit boundary bytes inside row data; the schema width and
    /// [`read_table`]'s value checks make that visible.
    ///
    /// # Errors
    /// [`DatError`] if the row count is missing or no boundary fits.
    pub fn parse(bytes: &'a [u8], expected_row_len: Option<usize>) -> Result<Self, DatError> {
        let mut r = Reader::new(bytes);
        let rows = r.u32("row count").map_err(DatError::Truncated)?;
        let body = r.rest();
        let count = widen(rows);
        let is_boundary = |at: usize| body.get(at..).is_some_and(|b| b.starts_with(&BOUNDARY));

        // Rows are at least one byte wide: zero-width rows would let a corrupt
        // count of up to `u32::MAX` rows cost memory with no input behind it.
        let expected_at = expected_row_len
            .filter(|&w| w > 0 || count == 0)
            .and_then(|w| w.checked_mul(count));
        let fixed_len = match expected_at {
            Some(at) if is_boundary(at) => at,
            _ if count == 0 => {
                if is_boundary(0) {
                    0
                } else {
                    return Err(DatError::NoBoundary { rows });
                }
            }
            _ => (count..body.len())
                .step_by(count)
                .find(|&at| is_boundary(at))
                .ok_or(DatError::NoBoundary { rows })?,
        };
        let row_len = fixed_len.checked_div(count).unwrap_or(0);
        let (fixed, variable) = body
            .split_at_checked(fixed_len)
            .ok_or(DatError::NoBoundary { rows })?;
        Ok(Self {
            rows,
            row_len,
            fixed,
            variable,
        })
    }

    /// Row count from the header.
    #[must_use]
    pub const fn rows(&self) -> u32 {
        self.rows
    }

    /// Width of each row.
    #[must_use]
    pub const fn row_len(&self) -> usize {
        self.row_len
    }

    /// The raw bytes of row `index`, if it exists.
    #[must_use]
    pub fn row(&self, index: usize) -> Option<&'a [u8]> {
        // Checked against the count, not only the bytes: with no rows the
        // width is 0 and every empty slice would be "in range".
        if index >= widen(self.rows) {
            return None;
        }
        let start = index.checked_mul(self.row_len)?;
        self.fixed.get(start..start.checked_add(self.row_len)?)
    }

    /// The variable section, starting with the boundary.
    #[must_use]
    pub const fn variable(&self) -> &'a [u8] {
        self.variable
    }
}

/// Reads every row of a table as the schema describes it.
///
/// # Errors
/// [`DatError`] if the file cannot be split (see [`DatFile::parse`]), a
/// value is invalid, or the output would exceed [`DECODE_BUDGET_PER_BYTE`]
/// values and string units per input byte. A row-size mismatch is not an error: it is reported in
/// [`Table::mismatch`].
pub fn read_table(bytes: &[u8], schema: &TableSchema) -> Result<Table, DatError> {
    let expected = schema.row_width();
    let dat = DatFile::parse(bytes, Some(expected))?;
    let row_len = dat.row_len();
    // A table with no rows has no measurable width, so it cannot mismatch.
    let mismatch = (dat.rows() > 0 && row_len != expected).then_some(RowSizeMismatch {
        expected,
        actual: row_len,
    });

    // Leading columns that fit in a row, with their offsets.
    let mut layout = Vec::with_capacity(schema.columns.len());
    let mut offset = 0;
    for column in &schema.columns {
        let end = offset + column.kind.width();
        if end > row_len && dat.rows() > 0 {
            break;
        }
        layout.push((offset, column.kind));
        offset = end;
    }

    let mut budget = bytes.len().saturating_mul(DECODE_BUDGET_PER_BYTE);
    let rows = (0..widen(dat.rows()))
        .map(|row| {
            let bytes = dat.row(row).unwrap_or_default();
            layout
                .iter()
                .enumerate()
                .map(|(column, &(at, kind))| {
                    let field = bytes.get(at..).unwrap_or_default();
                    decode(field, kind, dat.variable(), &mut budget)
                        .map_err(|kind| DatError::Value { row, column, kind })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Table {
        row_len,
        mismatch,
        columns_read: layout.len(),
        rows,
    })
}

/// First `N` bytes of `field`. Callers pass slices at least as wide as the
/// column, so the fallback of zeros is never used.
fn bytes<const N: usize>(field: &[u8]) -> [u8; N] {
    field.first_chunk::<N>().copied().unwrap_or([0; N])
}

fn u64_at(field: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes(field.get(at..).unwrap_or_default()))
}

fn key(raw: u64) -> Option<u64> {
    (raw != NULL_KEY).then_some(raw)
}

/// Takes `n` from the decode budget.
fn spend(budget: &mut usize, n: usize) -> Result<(), ValueError> {
    *budget = budget.checked_sub(n).ok_or(ValueError::OutputTooLarge)?;
    Ok(())
}

fn decode(
    field: &[u8],
    kind: ColumnKind,
    variable: &[u8],
    budget: &mut usize,
) -> Result<Value, ValueError> {
    match kind {
        ColumnKind::Scalar(ty) => scalar(field, ty, variable, budget),
        ColumnKind::Interval(ty) => {
            let second = field.get(ty.width()..).unwrap_or_default();
            Ok(Value::Interval(Box::new([
                scalar(field, ty, variable, budget)?,
                scalar(second, ty, variable, budget)?,
            ])))
        }
        ColumnKind::Array(ty) => {
            let count = u64_at(field, 0);
            let offset = u64_at(field, 8);
            if count == 0 {
                spend(budget, 1)?;
                return Ok(Value::Array(Vec::new()));
            }
            let out_of_range = ValueError::ArrayOutOfRange { count, offset };
            // Check the whole range before allocating, so a corrupt count
            // cannot ask for more memory than the input holds.
            let start = usize::try_from(offset).map_err(|_| out_of_range)?;
            let len = usize::try_from(count)
                .ok()
                .and_then(|n| n.checked_mul(ty.width()))
                .ok_or(out_of_range)?;
            let elements = start
                .checked_add(len)
                .and_then(|end| variable.get(start..end))
                .ok_or(out_of_range)?;
            elements
                .chunks_exact(ty.width())
                .map(|element| scalar(element, ty, variable, budget))
                .collect::<Result<_, _>>()
                .map(Value::Array)
        }
        ColumnKind::UnknownArray => {
            spend(budget, 1)?;
            Ok(Value::UnknownArray {
                count: u64_at(field, 0),
                offset: u64_at(field, 8),
            })
        }
    }
}

fn scalar(
    field: &[u8],
    ty: ScalarType,
    variable: &[u8],
    budget: &mut usize,
) -> Result<Value, ValueError> {
    spend(budget, 1)?;
    Ok(match ty {
        ScalarType::Bool => match bytes::<1>(field) {
            [0] => Value::Bool(false),
            [1] => Value::Bool(true),
            [other] => return Err(ValueError::BadBool(other)),
        },
        ScalarType::I16 => Value::I16(i16::from_le_bytes(bytes(field))),
        ScalarType::U16 => Value::U16(u16::from_le_bytes(bytes(field))),
        ScalarType::I32 => Value::I32(i32::from_le_bytes(bytes(field))),
        ScalarType::U32 => Value::U32(u32::from_le_bytes(bytes(field))),
        ScalarType::F32 => Value::F32(f32::from_le_bytes(bytes(field))),
        ScalarType::EnumRow => Value::EnumRow(i32::from_le_bytes(bytes(field))),
        ScalarType::String => Value::String(string(variable, u64_at(field, 0), budget)?),
        ScalarType::Row => Value::Row(key(u64_at(field, 0))),
        ScalarType::ForeignRow => {
            let (first, second) = (u64_at(field, 0), u64_at(field, 8));
            let key = key(first);
            if second != key.map_or(NULL_KEY, |_| 0) {
                return Err(ValueError::BadForeignKey { key: first, second });
            }
            Value::ForeignRow(key)
        }
    })
}

/// Reads the UTF-16LE string at `offset`, which ends at the first pair of
/// zero units (a zero `u32`) counted from the string start. Text starts after
/// the boundary at an even offset (every string in both installs does).
fn string(variable: &[u8], offset: u64, budget: &mut usize) -> Result<String, ValueError> {
    let start = usize::try_from(offset)
        .ok()
        .filter(|&s| s >= BOUNDARY.len() && s % 2 == 0 && s <= variable.len())
        .ok_or(ValueError::StringOutOfRange { offset })?;
    let mut units = variable
        .get(start..)
        .unwrap_or_default()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&unit| u16::from_le_bytes(unit))
        .peekable();
    let mut text = Vec::new();
    while let Some(unit) = units.next() {
        if unit == 0 && units.peek() == Some(&0) {
            return String::from_utf16(&text).map_err(|_| ValueError::StringNotUtf16 { offset });
        }
        spend(budget, 1)?;
        text.push(unit);
    }
    Err(ValueError::StringUnterminated { offset })
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic buffers; a bad index should panic"
)]
mod tests {
    use super::*;
    use crate::schema::Column;

    /// Builds `.datc64` bytes: rows are given raw, variable data is appended
    /// after the boundary and addressed by offsets from its start.
    struct Builder {
        variable: Vec<u8>,
    }

    impl Builder {
        fn new() -> Self {
            Self {
                variable: BOUNDARY.to_vec(),
            }
        }

        fn offset(&self) -> u64 {
            u64::try_from(self.variable.len()).unwrap()
        }

        fn string(&mut self, text: &str) -> u64 {
            let at = self.offset();
            for unit in text.encode_utf16() {
                self.variable.extend_from_slice(&unit.to_le_bytes());
            }
            self.variable.extend_from_slice(&[0; 4]);
            at
        }

        fn raw(&mut self, bytes: &[u8]) -> u64 {
            let at = self.offset();
            self.variable.extend_from_slice(bytes);
            at
        }

        fn build(&self, rows: &[Vec<u8>]) -> Vec<u8> {
            let mut out = u32::try_from(rows.len()).unwrap().to_le_bytes().to_vec();
            for row in rows {
                out.extend_from_slice(row);
            }
            out.extend_from_slice(&self.variable);
            out
        }
    }

    fn table(kinds: &[ColumnKind]) -> TableSchema {
        TableSchema {
            name: "Test".to_owned(),
            games: std::collections::BTreeSet::new(),
            columns: kinds
                .iter()
                .map(|&kind| Column {
                    name: None,
                    kind,
                    references: None,
                    localized: false,
                    unique: false,
                })
                .collect(),
            tags: Vec::new(),
        }
    }

    fn u64s(words: &[u64]) -> Vec<u8> {
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }

    /// Every column kind, in this order.
    fn all_kinds() -> Vec<ColumnKind> {
        use ColumnKind::{Array, Interval, Scalar, UnknownArray};
        use ScalarType as T;
        vec![
            Scalar(T::Bool),
            Scalar(T::I16),
            Scalar(T::U16),
            Scalar(T::I32),
            Scalar(T::U32),
            Scalar(T::F32),
            Scalar(T::String),
            Scalar(T::Row),
            Scalar(T::ForeignRow),
            Scalar(T::EnumRow),
            Interval(T::I32),
            Array(T::String),
            Array(T::I32),
            Array(T::ForeignRow),
            UnknownArray,
        ]
    }

    /// Two rows over [`all_kinds`]: row 0 has values, row 1 has nulls,
    /// empty arrays, and an empty string.
    fn all_kinds_bytes() -> Vec<u8> {
        let mut b = Builder::new();
        let name = b.string("Life \u{2713}");
        let empty = b.string("");
        let first = b.string("a");
        let second = b.string("bc");
        let strings = b.raw(&u64s(&[first, second]));
        let ints = b.raw(&[7, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]);
        let keys = b.raw(&u64s(&[3, 0, NULL_KEY, NULL_KEY]));

        let mut row0 = vec![1];
        row0.extend_from_slice(&(-2_i16).to_le_bytes());
        row0.extend_from_slice(&40_000_u16.to_le_bytes());
        row0.extend_from_slice(&(-5_i32).to_le_bytes());
        row0.extend_from_slice(&4_000_000_000_u32.to_le_bytes());
        row0.extend_from_slice(&1.5_f32.to_le_bytes());
        row0.extend_from_slice(&u64s(&[name, 1, 9, 0]));
        row0.extend_from_slice(&2_i32.to_le_bytes());
        row0.extend_from_slice(&10_i32.to_le_bytes());
        row0.extend_from_slice(&20_i32.to_le_bytes());
        row0.extend_from_slice(&u64s(&[2, strings, 2, ints, 2, keys, 5, 99]));

        let mut row1 = vec![0];
        row1.extend_from_slice(&[0; 2 + 2 + 4 + 4 + 4]);
        row1.extend_from_slice(&u64s(&[empty, NULL_KEY, NULL_KEY, NULL_KEY]));
        row1.extend_from_slice(&[0; 4 + 8]);
        // Empty arrays: the offset is not checked.
        row1.extend_from_slice(&u64s(&[0, 12345, 0, u64::MAX, 0, 0, 0, 0]));
        b.build(&[row0, row1])
    }

    #[test]
    fn read_table_should_decode_every_column_type() {
        let schema = table(&all_kinds());
        let table = read_table(&all_kinds_bytes(), &schema).unwrap();
        assert_eq!(table.row_len, schema.row_width());
        assert_eq!(
            table.row_len,
            1 + 2 + 2 + 4 + 4 + 4 + 8 + 8 + 16 + 4 + 8 + 4 * 16
        );
        assert_eq!(table.mismatch, None);
        assert_eq!(table.columns_read, 15);
        assert_eq!(
            table.rows[0],
            [
                Value::Bool(true),
                Value::I16(-2),
                Value::U16(40_000),
                Value::I32(-5),
                Value::U32(4_000_000_000),
                Value::F32(1.5),
                Value::String("Life \u{2713}".to_owned()),
                Value::Row(Some(1)),
                Value::ForeignRow(Some(9)),
                Value::EnumRow(2),
                Value::Interval(Box::new([Value::I32(10), Value::I32(20)])),
                Value::Array(vec![
                    Value::String("a".to_owned()),
                    Value::String("bc".to_owned())
                ]),
                Value::Array(vec![Value::I32(7), Value::I32(-1)]),
                Value::Array(vec![Value::ForeignRow(Some(3)), Value::ForeignRow(None)]),
                Value::UnknownArray {
                    count: 5,
                    offset: 99
                },
            ]
        );
    }

    #[test]
    fn read_table_should_decode_nulls_and_empty_values() {
        let table = read_table(&all_kinds_bytes(), &table(&all_kinds())).unwrap();
        let row = &table.rows[1];
        assert_eq!(row[0], Value::Bool(false));
        assert_eq!(row[6], Value::String(String::new()));
        assert_eq!(row[7], Value::Row(None));
        assert_eq!(row[8], Value::ForeignRow(None));
        for array in &row[11..14] {
            assert_eq!(*array, Value::Array(Vec::new()));
        }
        assert_eq!(
            row[14],
            Value::UnknownArray {
                count: 0,
                offset: 0
            }
        );
    }

    #[test]
    fn read_table_should_report_wider_rows_and_read_all_columns() {
        let i32_col = ColumnKind::Scalar(ScalarType::I32);
        // Rows are 6 bytes; the schema knows the first 4.
        let bytes = Builder::new().build(&[vec![1, 0, 0, 0, 0xAA, 0xAA], vec![2, 0, 0, 0, 0, 0]]);
        let table = read_table(&bytes, &table(&[i32_col])).unwrap();
        assert_eq!(
            table.mismatch,
            Some(RowSizeMismatch {
                expected: 4,
                actual: 6
            })
        );
        assert_eq!(table.columns_read, 1);
        assert_eq!(table.rows, [[Value::I32(1)], [Value::I32(2)]]);
    }

    #[test]
    fn read_table_should_report_narrower_rows_and_read_columns_that_fit() {
        let u16_col = ColumnKind::Scalar(ScalarType::U16);
        let i32_col = ColumnKind::Scalar(ScalarType::I32);
        // Rows are 4 bytes; the schema expects 2 + 4 + 2.
        let bytes = Builder::new().build(&[vec![1, 0, 2, 0]]);
        let table = read_table(&bytes, &table(&[u16_col, i32_col, u16_col])).unwrap();
        assert_eq!(
            table.mismatch,
            Some(RowSizeMismatch {
                expected: 8,
                actual: 4
            })
        );
        assert_eq!(table.columns_read, 1);
        assert_eq!(table.rows, [[Value::U16(1)]]);
    }

    #[test]
    fn read_table_should_measure_rows_when_schema_has_no_columns() {
        let bytes = Builder::new().build(&[vec![1, 2, 3], vec![4, 5, 6]]);
        let table = read_table(&bytes, &table(&[])).unwrap();
        assert_eq!(
            table.mismatch,
            Some(RowSizeMismatch {
                expected: 0,
                actual: 3
            })
        );
        assert_eq!(table.columns_read, 0);
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn read_table_should_accept_empty_table_of_any_schema() {
        let bytes = Builder::new().build(&[]);
        let read = read_table(&bytes, &table(&all_kinds())).unwrap();
        assert_eq!(read.mismatch, None);
        assert!(read.rows.is_empty());
        assert_eq!(read.columns_read, 15);
        assert_eq!(DatFile::parse(&bytes, None).unwrap().row(0), None);
        let mut no_boundary = bytes.clone();
        no_boundary.insert(4, 0);
        assert_eq!(
            read_table(&no_boundary, &table(&[])).unwrap_err().code(),
            "EXTRACT_DAT_NO_BOUNDARY"
        );
    }

    #[test]
    fn parse_should_prefer_schema_width_over_boundary_bytes_in_rows() {
        // One 16-byte row with boundary bytes at offset 1.
        let mut row = vec![0];
        row.extend_from_slice(&BOUNDARY);
        row.extend_from_slice(&[0; 7]);
        let bytes = Builder::new().build(&[row]);
        assert_eq!(DatFile::parse(&bytes, Some(16)).unwrap().row_len(), 16);
        // Without the schema width the first aligned boundary wins.
        assert_eq!(DatFile::parse(&bytes, None).unwrap().row_len(), 1);
    }

    #[test]
    fn parse_should_fall_back_to_first_aligned_boundary() {
        let bytes = Builder::new().build(&[vec![1, 2, 3], vec![4, 5, 6]]);
        let dat = DatFile::parse(&bytes, Some(4)).unwrap();
        assert_eq!(dat.rows(), 2);
        assert_eq!(dat.row_len(), 3);
        assert_eq!(dat.row(1), Some(&[4, 5, 6][..]));
        assert_eq!(dat.row(2), None);
        assert!(dat.variable().starts_with(&BOUNDARY));
    }

    #[test]
    fn parse_should_reject_bad_framing() {
        assert_eq!(
            DatFile::parse(&[1, 0], None).unwrap_err().code(),
            "EXTRACT_DAT_TRUNCATED"
        );
        // Rows but the boundary right after the count: rows cannot be 0 wide.
        let mut zero_width = 3_u32.to_le_bytes().to_vec();
        zero_width.extend_from_slice(&BOUNDARY);
        for expected in [None, Some(0)] {
            assert_eq!(
                DatFile::parse(&zero_width, expected).unwrap_err().code(),
                "EXTRACT_DAT_NO_BOUNDARY"
            );
        }
        let mut short_boundary = 1_u32.to_le_bytes().to_vec();
        short_boundary.extend_from_slice(&[0xBB; 7]);
        assert_eq!(
            DatFile::parse(&short_boundary, None).unwrap_err().code(),
            "EXTRACT_DAT_NO_BOUNDARY"
        );
    }

    fn error_code(kinds: &[ColumnKind], row: Vec<u8>, b: &Builder) -> &'static str {
        read_table(&b.build(&[row]), &table(kinds))
            .unwrap_err()
            .code()
    }

    #[test]
    fn read_table_should_reject_invalid_values() {
        use ColumnKind::{Array, Scalar};
        use ScalarType as T;
        let b = Builder::new();
        assert_eq!(
            error_code(&[Scalar(T::Bool)], vec![2], &b),
            "EXTRACT_DAT_BAD_BOOL"
        );
        assert_eq!(
            error_code(&[Scalar(T::String)], u64s(&[b.offset() + 1]), &b),
            "EXTRACT_DAT_STRING_OUT_OF_RANGE"
        );
        let mut unterminated = Builder::new();
        let at = unterminated.raw(&[b'a', 0, 0, 0, b'b']);
        assert_eq!(
            error_code(&[Scalar(T::String)], u64s(&[at]), &unterminated),
            "EXTRACT_DAT_STRING_UNTERMINATED"
        );
        let mut surrogate = Builder::new();
        let at = surrogate.raw(&[0x00, 0xD8, 0, 0, 0, 0]);
        assert_eq!(
            error_code(&[Scalar(T::String)], u64s(&[at]), &surrogate),
            "EXTRACT_DAT_STRING_NOT_UTF16"
        );
        for words in [[u64::MAX, 0], [2, u64::MAX], [1, 8], [u64::MAX / 2, 8]] {
            assert_eq!(
                error_code(&[Array(T::I32)], u64s(&words), &b),
                "EXTRACT_DAT_ARRAY_OUT_OF_RANGE",
                "{words:?}"
            );
        }
        for words in [[1, 1], [NULL_KEY, 0], [0, NULL_KEY]] {
            assert_eq!(
                error_code(&[Scalar(T::ForeignRow)], u64s(&words), &b),
                "EXTRACT_DAT_BAD_FOREIGN_KEY",
                "{words:?}"
            );
        }
    }

    #[test]
    fn read_table_should_reject_every_truncation() {
        let schema = table(&all_kinds());
        let bytes = all_kinds_bytes();
        for len in 0..bytes.len() {
            assert!(
                read_table(&bytes[..len], &schema).is_err(),
                "prefix of {len} bytes"
            );
        }
    }

    #[test]
    fn read_table_should_not_panic_on_single_byte_corruption() {
        let schema = table(&all_kinds());
        let bytes = all_kinds_bytes();
        let mut corrupt = bytes.clone();
        for at in 0..bytes.len() {
            for value in [0x00, 0x01, 0x7F, 0x80, 0xBB, 0xFE, 0xFF] {
                corrupt[at] = value;
                // Ok or Err are both fine; a panic or a huge allocation is not.
                let _ = read_table(&corrupt, &schema);
                let _ = DatFile::parse(&corrupt, None);
            }
            corrupt[at] = bytes[at];
        }
    }

    #[test]
    fn read_table_should_reject_string_offsets_in_boundary_or_odd() {
        use ColumnKind::Scalar;
        use ScalarType as T;
        let mut b = Builder::new();
        b.string("ab");
        for offset in [0, 4, 7, 9] {
            assert_eq!(
                error_code(&[Scalar(T::String)], u64s(&[offset]), &b),
                "EXTRACT_DAT_STRING_OUT_OF_RANGE",
                "{offset}"
            );
        }
    }

    #[test]
    fn read_table_should_decode_float_intervals() {
        let mut row = 1.5_f32.to_le_bytes().to_vec();
        row.extend_from_slice(&(-0.25_f32).to_le_bytes());
        let bytes = Builder::new().build(&[row]);
        let kind = ColumnKind::Interval(ScalarType::F32);
        let table = read_table(&bytes, &table(&[kind])).unwrap();
        assert_eq!(
            table.rows,
            [[Value::Interval(Box::new([
                Value::F32(1.5),
                Value::F32(-0.25)
            ]))]]
        );
    }

    #[test]
    fn read_table_should_locate_bad_array_element() {
        let mut b = Builder::new();
        let keys = b.raw(&u64s(&[1, 0, 2, 7]));
        // Row 0: an empty array. Row 1: two keys, the second one bad.
        let mut row0 = vec![0];
        row0.extend_from_slice(&u64s(&[0, 0]));
        let mut row1 = vec![1];
        row1.extend_from_slice(&u64s(&[2, keys]));
        let kinds = [
            ColumnKind::Scalar(ScalarType::Bool),
            ColumnKind::Array(ScalarType::ForeignRow),
        ];
        let err = read_table(&b.build(&[row0, row1]), &table(&kinds)).unwrap_err();
        assert_eq!(
            err,
            DatError::Value {
                row: 1,
                column: 1,
                kind: ValueError::BadForeignKey { key: 2, second: 7 }
            }
        );
        assert_eq!(
            err.to_string(),
            "dat row 1 column 1: foreign key 0x2 has second word 0x7"
        );
    }

    #[test]
    fn read_table_should_cap_output_of_rows_sharing_one_array() {
        let mut b = Builder::new();
        let shared = b.raw(&[0; 1000]);
        let row = u64s(&[1000, shared]);
        let rows = vec![row; 100];
        let kinds = [ColumnKind::Array(ScalarType::Bool)];
        let bytes = b.build(&rows);
        let err = read_table(&bytes, &table(&kinds)).unwrap_err();
        assert_eq!(err.code(), "EXTRACT_DAT_TOO_LARGE");
        // A few rows sharing it stay within the budget.
        let bytes = b.build(&rows[..2]);
        assert_eq!(read_table(&bytes, &table(&kinds)).unwrap().rows.len(), 2);
    }
}
