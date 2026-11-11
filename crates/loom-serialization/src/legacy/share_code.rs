//! PoB share codes (ADR-005; Phase 2 plan, step 10, decision 16).
//!
//! A share code is a PoB build document (XML), compressed as a zlib
//! stream, in base64 with the URL-safe alphabet (`-` and `_` for `+` and
//! `/`) and `=` padding. PoB v2.67.2 writes it in
//! `Classes/ImportTab.lua` (`common.base64.encode(Deflate(xml))` with the
//! two characters swapped) and reads it the reverse way. The build goes
//! through the XML adapter ([`super::pob_xml`]) both ways, so a code
//! imports to the same build as its XML.
//!
//! **Reading is lenient where PoB is:** ASCII whitespace anywhere is
//! skipped (codes are pasted with line breaks), both alphabets are read
//! (PoB maps `-` and `_` back before decoding), and the padding may be
//! missing. Bits after the last whole byte need not be zero (`Zh==`
//! reads as `Zg==`), as common decoders allow. Bytes after the zlib stream
//! are ignored, as zlib's own one-shot reader ignores them. Everything
//! else is refused with a code.
//!
//! **Errors (`IMPORT_*`, plan decision 15):** `IMPORT_SHARE_CODE_BASE64`,
//! `IMPORT_SHARE_CODE_ZLIB`, `IMPORT_SHARE_CODE_TOO_LARGE` (the code is
//! longer than [`MAX_CODE_BYTES`] or inflates past [`MAX_XML_BYTES`], so
//! neither a long code nor a small one can make the reader allocate
//! without bound), `IMPORT_SHARE_CODE_UTF8`; then the XML adapter's own
//! codes, unchanged.

use loom_domain::Build;
use miniz_oxide::inflate::TINFLStatus;

use super::pob_xml::{self, ImportError};

/// The largest build document a code may inflate to. The 21-build corpus
/// and the public fixtures are all under 2 MiB.
pub const MAX_XML_BYTES: usize = 32 * 1024 * 1024;

/// The longest code read, whitespace included: room for the base64 of a
/// zlib stream of [`MAX_XML_BYTES`] that did not compress (stored blocks
/// add 5 bytes per 64 KiB), with margin. The PoB-made fixture of a
/// 121 KB document is a 24 KB code.
pub const MAX_CODE_BYTES: usize = MAX_XML_BYTES / 3 * 4 + 1024 * 1024;

/// The zlib level [`encode`] compresses with (the best; any level reads).
const LEVEL: u8 = 9;

/// Why a share code did not import.
#[derive(Debug, thiserror::Error)]
pub enum ShareCodeError {
    /// A character outside both base64 alphabets, or a length no base64
    /// text has.
    #[error("IMPORT_SHARE_CODE_BASE64: {0}")]
    Base64(String),
    /// The bytes are not one whole zlib stream.
    #[error("IMPORT_SHARE_CODE_ZLIB: {0}")]
    Zlib(String),
    /// The code is longer than [`MAX_CODE_BYTES`], or the document
    /// inflates past [`MAX_XML_BYTES`] (32 MiB).
    #[error("IMPORT_SHARE_CODE_TOO_LARGE: the {0} is over its limit")]
    TooLarge(&'static str),
    /// The document is not UTF-8.
    #[error("IMPORT_SHARE_CODE_UTF8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    /// The document did not import (the XML adapter's code).
    #[error(transparent)]
    Xml(#[from] ImportError),
}

impl ShareCodeError {
    /// Stable code (ADR-014); the XML adapter's own for [`Self::Xml`].
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Base64(_) => "IMPORT_SHARE_CODE_BASE64",
            Self::Zlib(_) => "IMPORT_SHARE_CODE_ZLIB",
            Self::TooLarge(_) => "IMPORT_SHARE_CODE_TOO_LARGE",
            Self::Utf8(_) => "IMPORT_SHARE_CODE_UTF8",
            Self::Xml(e) => e.code(),
        }
    }
}

/// Imports a share code: decoded to its XML, then [`pob_xml::import`].
///
/// # Errors
/// [`ShareCodeError`].
pub fn import(code: &str) -> Result<Build, ShareCodeError> {
    Ok(pob_xml::import(&decode(code)?)?)
}

/// The share code of a build: [`pob_xml::export`], then [`encode`].
#[must_use]
pub fn export(build: &Build) -> String {
    encode(&pob_xml::export(build))
}

/// The XML a share code holds.
///
/// # Errors
/// [`ShareCodeError`] except [`ShareCodeError::Xml`].
pub fn decode(code: &str) -> Result<String, ShareCodeError> {
    let compressed = base64_decode(code)?;
    let xml = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&compressed, MAX_XML_BYTES)
        .map_err(|e| match e.status {
        TINFLStatus::HasMoreOutput => ShareCodeError::TooLarge("document (32 MiB)"),
        status => ShareCodeError::Zlib(format!("{status:?}")),
    })?;
    Ok(String::from_utf8(xml)?)
}

/// The share code of PoB XML text, as PoB writes it: zlib, base64 with the
/// URL-safe alphabet, `=` padding.
#[must_use]
pub fn encode(xml: &str) -> String {
    base64_encode(&miniz_oxide::deflate::compress_to_vec_zlib(
        xml.as_bytes(),
        LEVEL,
    ))
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk.first().copied().unwrap_or(0),
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                let index = (n >> (18 - 6 * i)) & 63;
                out.push(char::from(
                    ALPHABET.get(index as usize).copied().unwrap_or(b'A'),
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The value of a base64 character in either alphabet.
const fn sextet(c: u8) -> Option<u32> {
    let v = match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'-' | b'+' => 62,
        b'_' | b'/' => 63,
        _ => return None,
    };
    Some(v as u32)
}

/// Decodes base64 in one pass, four characters at a time, straight into
/// the output (memory is about three quarters of the code's length).
fn base64_decode(code: &str) -> Result<Vec<u8>, ShareCodeError> {
    if code.len() > MAX_CODE_BYTES {
        return Err(ShareCodeError::TooLarge("code (about 44 MiB)"));
    }
    let mut out = Vec::with_capacity(code.len() / 4 * 3 + 3);
    let (mut group, mut filled) = (0_u32, 0_usize);
    let (mut characters, mut padding) = (0_usize, 0_usize);
    for (position, c) in code.bytes().enumerate() {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            padding += 1;
            continue;
        }
        if padding > 0 {
            return Err(ShareCodeError::Base64(format!(
                "data after the padding at byte {position}"
            )));
        }
        let value = sextet(c)
            .ok_or_else(|| ShareCodeError::Base64(format!("byte {position} is not base64")))?;
        group = (group << 6) | value;
        filled += 1;
        characters += 1;
        if filled == 4 {
            // The three bytes of 24 bits; the casts keep the low 8 bits on
            // purpose.
            out.extend_from_slice(&[(group >> 16) as u8, (group >> 8) as u8, group as u8]);
            (group, filled) = (0, 0);
        }
    }
    if characters == 0 {
        return Err(ShareCodeError::Base64("empty".to_owned()));
    }
    if filled == 1 || padding > 2 || (padding > 0 && !(filled + padding).is_multiple_of(4)) {
        return Err(ShareCodeError::Base64(format!(
            "{characters} characters and {padding} padding is no base64 length"
        )));
    }
    // A last group of 2 or 3 characters holds 1 or 2 bytes; its spare low
    // bits are dropped.
    match filled {
        2 => out.push((group >> 4) as u8),
        3 => out.extend_from_slice(&[(group >> 10) as u8, (group >> 2) as u8]),
        _ => {}
    }
    Ok(out)
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;
