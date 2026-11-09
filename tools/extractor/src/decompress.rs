//! Oodle decompression behind a trait.
//!
//! Every open-source Oodle decoder found in the Spike A survey derives from
//! powzix/ooz (GPL-3.0-or-later), so none is linked into this project. The
//! maintainer installs a decoder separately and [`ExternalTool`] runs it as a
//! child process (maintainer decision 2026-11-08; `docs/spikes/spike-a-extraction.md`).
//! Container and framing code is tested with [`Identity`] on synthetic bytes.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// Decompresses one Oodle stream whose decompressed length is known up front.
///
/// Bundles2 stores the decompressed size of every bundle and chunk, so callers
/// always know `decompressed_len`. Implementations must return exactly that
/// many bytes or an error.
pub trait Decompressor {
    /// Decompresses `compressed` into exactly `decompressed_len` bytes.
    ///
    /// # Errors
    /// Returns [`DecompressError`] if the stream cannot be decoded or decodes
    /// to a different length.
    fn decompress(
        &self,
        compressed: &[u8],
        decompressed_len: usize,
    ) -> Result<Vec<u8>, DecompressError>;
}

/// Errors from a [`Decompressor`]. Each variant has a stable [`code`](Self::code).
#[derive(Debug, thiserror::Error)]
pub enum DecompressError {
    /// The decoder produced a different number of bytes than expected.
    #[error("decompressed {actual} bytes, expected {expected}")]
    LengthMismatch {
        /// Length the caller asked for.
        expected: usize,
        /// Length the decoder reported or produced.
        actual: usize,
    },
    /// The external tool could not be started.
    #[error("cannot start decompressor `{program}`: {source}")]
    Spawn {
        /// Program that was run.
        program: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The external tool exited with a failure status.
    #[error("decompressor `{program}` failed ({status}): {stderr}")]
    ToolFailed {
        /// Program that was run.
        program: PathBuf,
        /// Exit status as text.
        status: String,
        /// Captured standard error, trimmed.
        stderr: String,
    },
    /// The external tool succeeded but did not report how many bytes it decoded.
    #[error("decompressor `{program}` did not report a decoded length")]
    UnreportedLength {
        /// Program that was run.
        program: PathBuf,
    },
    /// Reading or writing the scratch files failed.
    #[error("scratch file {path}: {source}")]
    Scratch {
        /// File that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

impl DecompressError {
    /// Stable machine-readable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::LengthMismatch { .. } => "EXTRACT_DECOMPRESS_LENGTH_MISMATCH",
            Self::Spawn { .. } => "EXTRACT_DECOMPRESS_SPAWN",
            Self::ToolFailed { .. } => "EXTRACT_DECOMPRESS_TOOL_FAILED",
            Self::UnreportedLength { .. } => "EXTRACT_DECOMPRESS_UNREPORTED_LENGTH",
            Self::Scratch { .. } => "EXTRACT_DECOMPRESS_SCRATCH",
        }
    }
}

/// Returns the input unchanged. For framing tests on synthetic bytes.
#[derive(Debug, Clone, Copy, Default)]
pub struct Identity;

impl Decompressor for Identity {
    fn decompress(
        &self,
        compressed: &[u8],
        decompressed_len: usize,
    ) -> Result<Vec<u8>, DecompressError> {
        if compressed.len() == decompressed_len {
            Ok(compressed.to_vec())
        } else {
            Err(DecompressError::LengthMismatch {
                expected: decompressed_len,
                actual: compressed.len(),
            })
        }
    }
}

/// Runs a separately installed `unoodle` executable (from the `oozextract`
/// crate's `cli` feature; tested with 0.5.7) once per call.
///
/// The tool reads and writes files, so each call uses two scratch files in
/// `work_dir`, removed afterwards. The tool is run as
/// `unoodle <in> --length <n> --header 0 --output <out> --verbose`, and its
/// `Extracted <n> bytes` line is checked against the expected length, because
/// the tool always writes a buffer of the requested size.
#[derive(Debug, Clone)]
pub struct ExternalTool {
    program: PathBuf,
    work_dir: PathBuf,
}

/// Distinguishes scratch files of concurrent calls in one process.
static NEXT_SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

impl ExternalTool {
    /// Creates a decompressor that runs `program` with scratch files in `work_dir`.
    /// `work_dir` must exist.
    #[must_use]
    pub fn new(program: impl Into<PathBuf>, work_dir: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            work_dir: work_dir.into(),
        }
    }

    /// The program this decompressor runs.
    #[must_use]
    pub fn program(&self) -> &Path {
        &self.program
    }
}

impl Decompressor for ExternalTool {
    fn decompress(
        &self,
        compressed: &[u8],
        decompressed_len: usize,
    ) -> Result<Vec<u8>, DecompressError> {
        let id = NEXT_SCRATCH_ID.fetch_add(1, Ordering::Relaxed);
        let stem = format!("loom-oodle-{}-{id}", std::process::id());
        let scratch = ScratchFiles {
            input: self.work_dir.join(format!("{stem}.in")),
            output: self.work_dir.join(format!("{stem}.out")),
        };
        std::fs::write(&scratch.input, compressed).map_err(|source| DecompressError::Scratch {
            path: scratch.input.clone(),
            source,
        })?;

        let output = Command::new(&self.program)
            .arg(&scratch.input)
            .arg("--length")
            .arg(decompressed_len.to_string())
            .args(["--header", "0", "--verbose", "--output"])
            .arg(&scratch.output)
            .output()
            .map_err(|source| DecompressError::Spawn {
                program: self.program.clone(),
                source,
            })?;
        if !output.status.success() {
            return Err(DecompressError::ToolFailed {
                program: self.program.clone(),
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        let reported =
            parse_extracted_len(&String::from_utf8_lossy(&output.stdout)).ok_or_else(|| {
                DecompressError::UnreportedLength {
                    program: self.program.clone(),
                }
            })?;
        if reported != decompressed_len {
            return Err(DecompressError::LengthMismatch {
                expected: decompressed_len,
                actual: reported,
            });
        }
        let bytes = std::fs::read(&scratch.output).map_err(|source| DecompressError::Scratch {
            path: scratch.output.clone(),
            source,
        })?;
        if bytes.len() == decompressed_len {
            Ok(bytes)
        } else {
            Err(DecompressError::LengthMismatch {
                expected: decompressed_len,
                actual: bytes.len(),
            })
        }
    }
}

/// Removes the scratch files when a call ends, on success or error.
struct ScratchFiles {
    input: PathBuf,
    output: PathBuf,
}

impl Drop for ScratchFiles {
    fn drop(&mut self) {
        // Best effort: a missing file (tool never wrote it) is not an error.
        let _ = std::fs::remove_file(&self.input);
        let _ = std::fs::remove_file(&self.output);
    }
}

/// Reads `n` from the tool's `Extracted <n> bytes` progress line.
fn parse_extracted_len(stdout: &str) -> Option<usize> {
    stdout.lines().find_map(|line| {
        line.trim()
            .strip_prefix("Extracted ")?
            .strip_suffix(" bytes")?
            .parse()
            .ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_should_return_input_when_length_matches() {
        let out = Identity.decompress(b"abc", 3).unwrap();
        assert_eq!(out, b"abc");
    }

    #[test]
    fn identity_should_report_length_mismatch() {
        let err = Identity.decompress(b"abc", 4).unwrap_err();
        assert_eq!(err.code(), "EXTRACT_DECOMPRESS_LENGTH_MISMATCH");
    }

    #[test]
    fn parse_extracted_len_should_read_tool_progress_line() {
        let stdout = "Read 52638 bytes\nExpected output: 262144 bytes\nExtracted 262144 bytes\nWriting to x\n";
        assert_eq!(parse_extracted_len(stdout), Some(262_144));
    }

    #[test]
    fn parse_extracted_len_should_return_none_without_progress_line() {
        assert_eq!(parse_extracted_len("Read 10 bytes\n"), None);
        assert_eq!(parse_extracted_len("Extracted many bytes\n"), None);
    }

    #[test]
    fn external_tool_should_report_spawn_error_for_missing_program() {
        let tool = ExternalTool::new("loom-no-such-decompressor-tool", std::env::temp_dir());
        let err = tool.decompress(b"\x8c\x0c", 8).unwrap_err();
        assert_eq!(err.code(), "EXTRACT_DECOMPRESS_SPAWN");
    }
}
