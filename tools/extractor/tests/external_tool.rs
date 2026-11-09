//! `ExternalTool` against the `fake-unoodle` test double (CI, no game data).

#![expect(
    clippy::expect_used,
    reason = "test helpers: a failed check should panic with its message"
)]

use std::path::{Path, PathBuf};

use loom_extractor::decompress::{Decompressor, ExternalTool};

/// A fresh scratch directory, removed on drop (also when a test panics).
struct WorkDir(PathBuf);

impl WorkDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("loom-ext-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create work dir");
        Self(dir)
    }

    fn is_empty(&self) -> bool {
        std::fs::read_dir(&self.0)
            .expect("read work dir")
            .next()
            .is_none()
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tool(dir: &Path) -> ExternalTool {
    ExternalTool::new(env!("CARGO_BIN_EXE_fake-unoodle"), dir)
}

#[test]
fn decompress_should_return_tool_output_and_remove_scratch_files() {
    let dir = WorkDir::new("ok");
    let out = tool(&dir.0).decompress(b"ok:hello", 5).expect("decode");
    assert_eq!(out, b"hello");
    assert!(dir.is_empty(), "scratch files removed");
}

#[test]
fn decompress_should_reject_reported_length_mismatch() {
    let dir = WorkDir::new("reported");
    let err = tool(&dir.0).decompress(b"ok:hello", 6).unwrap_err();
    assert_eq!(err.code(), "EXTRACT_DECOMPRESS_LENGTH_MISMATCH");
    assert!(dir.is_empty(), "scratch files removed");
}

#[test]
fn decompress_should_reject_output_shorter_than_reported() {
    let dir = WorkDir::new("lie");
    let err = tool(&dir.0).decompress(b"lie:abc", 8).unwrap_err();
    assert_eq!(err.code(), "EXTRACT_DECOMPRESS_LENGTH_MISMATCH");
    assert!(dir.is_empty(), "scratch files removed");
}

#[test]
fn decompress_should_report_missing_length_line() {
    let dir = WorkDir::new("silent");
    let err = tool(&dir.0).decompress(b"silent:abc", 3).unwrap_err();
    assert_eq!(err.code(), "EXTRACT_DECOMPRESS_UNREPORTED_LENGTH");
    assert!(dir.is_empty(), "scratch files removed");
}

#[test]
fn decompress_should_report_tool_failure_with_stderr() {
    let dir = WorkDir::new("fail");
    let err = tool(&dir.0)
        .decompress(b"fail:corrupt block", 3)
        .unwrap_err();
    assert_eq!(err.code(), "EXTRACT_DECOMPRESS_TOOL_FAILED");
    assert!(
        err.to_string().contains("corrupt block"),
        "stderr in message: {err}"
    );
    assert!(dir.is_empty(), "scratch files removed");
}

#[test]
fn decompress_should_report_scratch_error_for_missing_work_dir() {
    let dir = WorkDir::new("missing");
    let err = tool(&dir.0.join("absent"))
        .decompress(b"ok:x", 1)
        .unwrap_err();
    assert_eq!(err.code(), "EXTRACT_DECOMPRESS_SCRATCH");
}

#[test]
fn decompress_should_not_mix_scratch_files_across_threads() {
    let dir = WorkDir::new("threads");
    let tool = tool(&dir.0);
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let tool = &tool;
                s.spawn(move || {
                    let payload = format!("payload-{i}");
                    let out = tool
                        .decompress(format!("ok:{payload}").as_bytes(), payload.len())
                        .expect("decode");
                    assert_eq!(out, payload.as_bytes());
                })
            })
            .collect();
        for h in handles {
            h.join().expect("thread");
        }
    });
    assert!(dir.is_empty(), "scratch files removed");
}
