//! Bundles2 bundle container: a header, a chunk-size table, then Oodle chunks
//! back to back. Layout in `docs/spikes/spike-a-extraction.md`, section 3.
//!
//! [`Bundle::parse`] checks every redundant header field against the others
//! and against the file length. [`Bundle::decode`] makes one decoder call for
//! the whole payload: chunks are independent Oodle blocks and the decoder
//! reads them in sequence, which the A1 install test confirmed against
//! per-chunk decoding.

use crate::decompress::{DecompressError, Decompressor};
use crate::reader::{Reader, Short, widen};

/// Bytes counted by `head size` before the chunk table (offsets 12 to 60).
pub const FIXED_HEAD_LEN: u32 = 48;

/// Parsed bundle header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleHeader {
    /// Size of the decoded data.
    pub decompressed_len: usize,
    /// Size of all chunks together.
    pub payload_len: usize,
    /// Oodle compressor id (observed 8 = Kraken, 12, 13 = Leviathan). Not
    /// needed for decoding; kept for reports.
    pub encoder: u32,
    /// Decoded size of every chunk except the last.
    pub granularity: usize,
    /// Compressed size of each chunk.
    pub chunk_sizes: Vec<u32>,
}

/// A bundle file split into its header and payload. Borrows the file bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle<'a> {
    /// The parsed header.
    pub header: BundleHeader,
    /// All chunks, back to back.
    pub payload: &'a [u8],
}

/// Errors from reading a bundle. Each variant has a stable [`code`](Self::code).
#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    /// The file ends before a field or the payload.
    #[error(
        "bundle truncated: {} at offset {} needs {} bytes, {} left",
        .0.field, .0.offset, .0.needed, .0.available
    )]
    Truncated(Short),
    /// Two header fields, or a field and the file length, disagree.
    #[error("bundle header inconsistent: {field} is {actual}, expected {expected}")]
    Inconsistent {
        /// The field that disagrees.
        field: &'static str,
        /// Value implied by the other fields.
        expected: u64,
        /// Value found.
        actual: u64,
    },
    /// Bytes follow the last chunk.
    #[error("bundle has {count} bytes after the payload")]
    TrailingBytes {
        /// Number of extra bytes.
        count: usize,
    },
    /// The decoder failed.
    #[error(transparent)]
    Decompress(#[from] DecompressError),
}

impl From<Short> for BundleError {
    fn from(short: Short) -> Self {
        Self::Truncated(short)
    }
}

impl BundleError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Truncated(_) => "EXTRACT_BUNDLE_TRUNCATED",
            Self::Inconsistent { .. } => "EXTRACT_BUNDLE_HEADER_INCONSISTENT",
            Self::TrailingBytes { .. } => "EXTRACT_BUNDLE_TRAILING_BYTES",
            Self::Decompress(inner) => inner.code(),
        }
    }
}

/// Returns an [`BundleError::Inconsistent`] unless `actual == expected`.
fn expect_eq(field: &'static str, expected: u64, actual: u64) -> Result<(), BundleError> {
    if actual == expected {
        Ok(())
    } else {
        Err(BundleError::Inconsistent {
            field,
            expected,
            actual,
        })
    }
}

impl<'a> Bundle<'a> {
    /// Parses the header of the bundle file `bytes` and checks it for
    /// consistency. Does not decode.
    ///
    /// # Errors
    /// [`BundleError::Truncated`] if the file is too short,
    /// [`BundleError::Inconsistent`] if header fields disagree, and
    /// [`BundleError::TrailingBytes`] if bytes follow the payload.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, BundleError> {
        let mut r = Reader::new(bytes);
        let decompressed_len = r.u32("decompressed size")?;
        let payload_len = r.u32("payload size")?;
        let head_len = r.u32("head size")?;
        let encoder = r.u32("encoder")?;
        let _unknown = r.u32("unknown field at 16")?;
        let decompressed_len_64 = r.u64("decompressed size (u64)")?;
        let payload_len_64 = r.u64("payload size (u64)")?;
        let chunk_count = r.u32("chunk count")?;
        let granularity = r.u32("granularity")?;
        r.take("reserved fields at 44", 16)?;

        expect_eq(
            "decompressed size (u64)",
            u64::from(decompressed_len),
            decompressed_len_64,
        )?;
        expect_eq("payload size (u64)", u64::from(payload_len), payload_len_64)?;
        expect_eq(
            "head size",
            u64::from(FIXED_HEAD_LEN) + 4 * u64::from(chunk_count),
            u64::from(head_len),
        )?;
        if granularity == 0 {
            return Err(BundleError::Inconsistent {
                field: "granularity",
                expected: 1,
                actual: 0,
            });
        }
        expect_eq(
            "chunk count",
            u64::from(decompressed_len.div_ceil(granularity)),
            u64::from(chunk_count),
        )?;

        // The chunk table is bounded by the file length before anything is allocated.
        let table = r.take("chunk table", widen(chunk_count).saturating_mul(4))?;
        let chunk_sizes: Vec<u32> = table
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        let chunk_total: u64 = chunk_sizes.iter().copied().map(u64::from).sum();
        expect_eq("chunk sizes total", u64::from(payload_len), chunk_total)?;

        let payload = r.take("payload", widen(payload_len))?;
        if !r.is_empty() {
            return Err(BundleError::TrailingBytes {
                count: r.rest().len(),
            });
        }

        Ok(Self {
            header: BundleHeader {
                decompressed_len: widen(decompressed_len),
                payload_len: widen(payload_len),
                encoder,
                granularity: widen(granularity),
                chunk_sizes,
            },
            payload,
        })
    }

    /// Decodes the payload with one decoder call.
    ///
    /// # Errors
    /// [`BundleError::Decompress`] if the decoder fails or returns the wrong length.
    pub fn decode<D>(&self, decompressor: &D) -> Result<Vec<u8>, BundleError>
    where
        D: Decompressor + ?Sized,
    {
        if self.header.decompressed_len == 0 {
            return Ok(Vec::new());
        }
        Ok(decompressor.decompress(self.payload, self.header.decompressed_len)?)
    }
}

/// Parses and decodes the bundle file `bytes`.
///
/// # Errors
/// Any [`BundleError`] from [`Bundle::parse`] or [`Bundle::decode`].
pub fn decode_bundle<D>(bytes: &[u8], decompressor: &D) -> Result<Vec<u8>, BundleError>
where
    D: Decompressor + ?Sized,
{
    Bundle::parse(bytes)?.decode(decompressor)
}

/// Builds a bundle whose chunks are stored uncompressed, for tests with the
/// [`Identity`](crate::decompress::Identity) decompressor.
#[cfg(test)]
pub(crate) fn build_identity_bundle(data: &[u8], granularity: u32) -> Vec<u8> {
    let len = u32::try_from(data.len()).expect("test data fits u32");
    let chunks: Vec<&[u8]> = data.chunks(widen(granularity)).collect();
    let count = u32::try_from(chunks.len()).expect("chunk count fits u32");
    let mut out = Vec::new();
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(FIXED_HEAD_LEN + 4 * count).to_le_bytes());
    out.extend_from_slice(&8_u32.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&u64::from(len).to_le_bytes());
    out.extend_from_slice(&u64::from(len).to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&granularity.to_le_bytes());
    out.extend_from_slice(&[0; 16]);
    for chunk in &chunks {
        let size = u32::try_from(chunk.len()).expect("chunk fits u32");
        out.extend_from_slice(&size.to_le_bytes());
    }
    out.extend_from_slice(data);
    out
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic buffers; a bad index should panic"
)]
mod tests {
    use super::*;
    use crate::decompress::Identity;

    const DATA: &[u8] = b"Bundles2 synthetic payload, not game data.";

    #[test]
    fn decode_should_return_data_of_identity_bundle() {
        let bytes = build_identity_bundle(DATA, 16);
        let bundle = Bundle::parse(&bytes).unwrap();
        assert_eq!(bundle.header.decompressed_len, DATA.len());
        assert_eq!(bundle.header.chunk_sizes, [16, 16, 10]);
        assert_eq!(bundle.header.granularity, 16);
        assert_eq!(bundle.header.encoder, 8);
        assert_eq!(bundle.decode(&Identity).unwrap(), DATA);
    }

    #[test]
    fn decode_should_handle_empty_bundle() {
        let bytes = build_identity_bundle(b"", 16);
        assert_eq!(decode_bundle(&bytes, &Identity).unwrap(), b"");
    }

    #[test]
    fn decode_should_handle_exact_multiple_of_granularity() {
        let bytes = build_identity_bundle(&DATA[..32], 16);
        let bundle = Bundle::parse(&bytes).unwrap();
        assert_eq!(bundle.header.chunk_sizes, [16, 16]);
    }

    /// Overwrites the `u32` at `offset` in a copy of `bytes`.
    fn with_u32(bytes: &[u8], offset: usize, value: u32) -> Vec<u8> {
        let mut out = bytes.to_vec();
        out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        out
    }

    fn parse_code(bytes: &[u8]) -> &'static str {
        Bundle::parse(bytes).unwrap_err().code()
    }

    #[test]
    fn parse_should_reject_inconsistent_header_fields() {
        let good = build_identity_bundle(DATA, 16);
        let inconsistent = "EXTRACT_BUNDLE_HEADER_INCONSISTENT";
        // decompressed size (u32) disagrees with its u64 copy
        assert_eq!(parse_code(&with_u32(&good, 0, 41)), inconsistent);
        // payload size disagrees with its u64 copy
        assert_eq!(parse_code(&with_u32(&good, 4, 41)), inconsistent);
        // head size
        assert_eq!(parse_code(&with_u32(&good, 8, 64)), inconsistent);
        // u64 decompressed size
        assert_eq!(parse_code(&with_u32(&good, 20, 41)), inconsistent);
        // granularity zero, and granularity that implies another chunk count
        assert_eq!(parse_code(&with_u32(&good, 40, 0)), inconsistent);
        assert_eq!(parse_code(&with_u32(&good, 40, 32)), inconsistent);
        // a chunk size that breaks the total
        assert_eq!(parse_code(&with_u32(&good, 60, 15)), inconsistent);
    }

    #[test]
    fn parse_should_reject_trailing_bytes() {
        let mut bytes = build_identity_bundle(DATA, 16);
        bytes.push(0);
        assert_eq!(parse_code(&bytes), "EXTRACT_BUNDLE_TRAILING_BYTES");
    }

    #[test]
    fn parse_should_reject_every_truncation() {
        let bytes = build_identity_bundle(DATA, 16);
        for len in 0..bytes.len() {
            let code = parse_code(&bytes[..len]);
            assert_eq!(code, "EXTRACT_BUNDLE_TRUNCATED", "truncated to {len}");
        }
    }

    #[test]
    fn parse_should_not_panic_on_any_single_byte_corruption() {
        let bytes = build_identity_bundle(DATA, 16);
        for offset in 0..bytes.len() {
            for value in [0x00, 0x01, 0x7f, 0x80, 0xff, bytes[offset].wrapping_add(1)] {
                let mut corrupt = bytes.clone();
                corrupt[offset] = value;
                if let Ok(bundle) = Bundle::parse(&corrupt) {
                    // Accepted input must still decode to the declared length.
                    let decoded = bundle.decode(&Identity).unwrap();
                    assert_eq!(decoded.len(), bundle.header.decompressed_len);
                }
            }
        }
    }

    #[test]
    fn parse_should_reject_huge_chunk_count_without_allocating() {
        // Consistent header claiming 2^28 chunks, but no table bytes.
        let count: u32 = 1 << 28;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&count.to_le_bytes()); // decompressed size, granularity 1
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.extend_from_slice(&(FIXED_HEAD_LEN + 4 * count).to_le_bytes());
        bytes.extend_from_slice(&[8, 0, 0, 0, 1, 0, 0, 0]);
        bytes.extend_from_slice(&u64::from(count).to_le_bytes());
        bytes.extend_from_slice(&0_u64.to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
        bytes.extend_from_slice(&1_u32.to_le_bytes());
        bytes.extend_from_slice(&[0; 16]);
        assert_eq!(parse_code(&bytes), "EXTRACT_BUNDLE_TRUNCATED");
    }
}
