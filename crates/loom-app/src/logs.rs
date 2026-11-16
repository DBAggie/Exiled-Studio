//! Structured local logs (Phase 5, P-f; ADR-014): JSON lines in
//! `<data>/logs/`, one event per line, rotated by size, redacted before
//! they are written. Nothing is sent anywhere.
//!
//! - **Files:** `loom.log` is the current file; when the next line would
//!   take it past [`MAX_BYTES`], it becomes `loom.1.log`, the older files
//!   move up one, and the oldest beyond [`KEEP`] files is deleted.
//! - **Redaction** ([`Redactor`]): every line has the user's home folder
//!   replaced by `~` and the user name by `<user>`, in each form a JSON
//!   line can hold them (as written, with `/` separators, and with JSON's
//!   escaped `\\`), ignoring ASCII case. What the application logs never
//!   holds build text, item text, or share codes, only their sizes and
//!   SHA-256 ([`digest`]); a file is named by its extension only.
//! - **Correlation:** each command runs in a `command` span with its
//!   correlation ID, and each calculation in a `calc` span with the
//!   correlation of the command that asked for it; the JSON layer writes
//!   the current span's fields on every line, so a command's events and
//!   its calculation's share one ID.
//!
//! The host installs the subscriber once ([`Logs::subscriber`]); the
//! application only emits `tracing` events, so tests and hosts without a
//! subscriber log nothing.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use sha2::{Digest, Sha256};
use tracing_subscriber::fmt::MakeWriter;

/// The logs folder, under the data folder.
pub const LOG_DIR: &str = "logs";

/// The current file's name.
pub const CURRENT: &str = "loom.log";

/// The largest a file grows before it rotates.
pub const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// How many files are kept, the current one included.
pub const KEEP: usize = 7;

/// The name of file `n` (0: the current one).
#[must_use]
pub fn file_name(n: usize) -> String {
    if n == 0 {
        CURRENT.to_owned()
    } else {
        format!("loom.{n}.log")
    }
}

/// A text's size and SHA-256 (lowercase hex), what a log names instead of
/// the text.
#[must_use]
pub fn digest(bytes: &[u8]) -> (usize, String) {
    let sha = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    (bytes.len(), sha)
}

/// Replaces the user's home folder and name in a text (module docs).
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    /// What to find (lowercase ASCII) and what replaces it, longest first.
    needles: Vec<(String, &'static str)>,
}

impl Redactor {
    /// A redactor for `home` (replaced by `~`) and `user` (replaced by
    /// `<user>`; names shorter than three characters are left, as they
    /// would match ordinary words).
    #[must_use]
    pub fn new(home: Option<&Path>, user: Option<&str>) -> Self {
        let mut needles = Vec::new();
        if let Some(home) = home.map(|h| h.display().to_string()) {
            let home = home.trim_end_matches(['/', '\\']).to_owned();
            if !home.is_empty() {
                let slashed = home.replace('\\', "/");
                // JSON-escaped once (a path field), and twice (a path
                // `Debug`-formatted into a field).
                let escaped = home.replace('\\', "\\\\");
                let twice = home.replace('\\', "\\\\\\\\");
                for form in [home, slashed, escaped, twice] {
                    needles.push((form.to_ascii_lowercase(), "~"));
                }
            }
        }
        if let Some(user) = user.filter(|u| u.chars().count() >= 3) {
            needles.push((user.to_ascii_lowercase(), "<user>"));
        }
        needles.sort_by_key(|(n, _)| std::cmp::Reverse(n.len()));
        needles.dedup();
        Self { needles }
    }

    /// The redactor for this process's user: the home folder from
    /// `USERPROFILE` or `HOME`, the name from `USERNAME` or `USER`.
    #[must_use]
    pub fn from_env() -> Self {
        let var = |names: [&str; 2]| names.iter().find_map(std::env::var_os);
        let home = var(["USERPROFILE", "HOME"]).map(PathBuf::from);
        let user = var(["USERNAME", "USER"]).map(|u| u.to_string_lossy().into_owned());
        Self::new(home.as_deref(), user.as_deref())
    }

    /// `text` with every needle replaced.
    #[must_use]
    pub fn redact(&self, text: &str) -> String {
        let mut out = text.to_owned();
        for (needle, with) in &self.needles {
            out = replace_ignore_ascii_case(&out, needle, with);
        }
        out
    }
}

/// Whether `c` continues a word.
fn wordy(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `text` with each occurrence of `needle` (lowercase ASCII where it is
/// ASCII) replaced, comparing ASCII letters without case. Where the needle
/// starts or ends with a word character, a match must start or end at a
/// word boundary, so a user name `bob` leaves `bobsled` alone.
fn replace_ignore_ascii_case(text: &str, needle: &str, with: &str) -> String {
    let (Some(first), Some(last)) = (needle.chars().next(), needle.chars().next_back()) else {
        return text.to_owned();
    };
    let lower = text.to_ascii_lowercase();
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut at = 0;
    while let Some(found) = lower.get(at..).and_then(|rest| rest.find(needle)) {
        let start = at + found;
        let end = start + needle.len();
        let before = lower.get(..start).and_then(|s| s.chars().next_back());
        let after = lower.get(end..).and_then(|s| s.chars().next());
        let bounded = !(wordy(first) && before.is_some_and(wordy))
            && !(wordy(last) && after.is_some_and(wordy));
        if bounded {
            out.push_str(text.get(copied..start).unwrap_or_default());
            out.push_str(with);
            copied = end;
            at = end;
        } else {
            at = start + first.len_utf8();
        }
    }
    out.push_str(text.get(copied..).unwrap_or_default());
    out
}

/// The log files: the current one, rotated by size (module docs).
#[derive(Debug)]
pub struct RotatingFile {
    dir: PathBuf,
    max_bytes: u64,
    keep: usize,
    file: Option<File>,
    size: u64,
}

impl RotatingFile {
    /// Opens (or creates) `dir` and its current file for appending.
    ///
    /// # Errors
    /// The folder or the file cannot be made.
    pub fn open(dir: &Path, max_bytes: u64, keep: usize) -> io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        let mut out = Self {
            dir: dir.to_owned(),
            max_bytes,
            keep: keep.max(1),
            file: None,
            size: 0,
        };
        out.reopen()?;
        Ok(out)
    }

    fn reopen(&mut self) -> io::Result<()> {
        let path = self.dir.join(CURRENT);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        self.size = file.metadata()?.len();
        self.file = Some(file);
        Ok(())
    }

    /// Moves every file up one name, deleting the oldest beyond `keep`,
    /// and starts an empty current file.
    fn rotate(&mut self) -> io::Result<()> {
        self.file = None;
        let oldest = self.dir.join(file_name(self.keep - 1));
        if oldest.exists() {
            std::fs::remove_file(&oldest)?;
        }
        for n in (0..self.keep - 1).rev() {
            let from = self.dir.join(file_name(n));
            if from.exists() {
                std::fs::rename(&from, self.dir.join(file_name(n + 1)))?;
            }
        }
        self.reopen()
    }

    /// Appends one line (with its newline), rotating first when it would
    /// take the current file past the limit.
    ///
    /// # Errors
    /// The write or the rotation fails.
    pub fn write_line(&mut self, line: &[u8]) -> io::Result<()> {
        let len = line.len() as u64;
        // A rotation that fails (a file held open elsewhere) keeps the line
        // in the current file; the next write tries again.
        if self.size > 0 && self.size + len > self.max_bytes && self.rotate().is_err() {
            self.reopen()?;
        }
        let file = match &mut self.file {
            Some(f) => f,
            None => {
                self.reopen()?;
                self.file
                    .as_mut()
                    .ok_or_else(|| io::Error::other("the log file is not open"))?
            }
        };
        file.write_all(line)?;
        self.size += len;
        Ok(())
    }

    /// The files that exist, newest first.
    #[must_use]
    pub fn files(&self) -> Vec<PathBuf> {
        files_in(&self.dir, self.keep)
    }
}

/// The log files in `dir`, newest first, up to `keep`.
#[must_use]
pub fn files_in(dir: &Path, keep: usize) -> Vec<PathBuf> {
    (0..keep)
        .map(|n| dir.join(file_name(n)))
        .filter(|p| p.is_file())
        .collect()
}

/// The logs: the files and the redactor, shared by the subscriber's
/// writers.
#[derive(Debug, Clone)]
pub struct Logs {
    file: Arc<Mutex<RotatingFile>>,
    redactor: Arc<Redactor>,
}

impl Logs {
    /// Opens `<data_dir>/logs/` with the usual limits.
    ///
    /// # Errors
    /// The folder or the file cannot be made.
    pub fn open(data_dir: &Path, redactor: Redactor) -> io::Result<Self> {
        Self::with_limits(&data_dir.join(LOG_DIR), redactor, MAX_BYTES, KEEP)
    }

    /// Opens `dir` with other limits (tests).
    ///
    /// # Errors
    /// The folder or the file cannot be made.
    pub fn with_limits(
        dir: &Path,
        redactor: Redactor,
        max_bytes: u64,
        keep: usize,
    ) -> io::Result<Self> {
        Ok(Self {
            file: Arc::new(Mutex::new(RotatingFile::open(dir, max_bytes, keep)?)),
            redactor: Arc::new(redactor),
        })
    }

    /// The JSON-lines subscriber writing to these logs: the application's
    /// own events (targets under `loom`, never a library's) at `INFO` and
    /// above, each with its target, level, time, and the current span's
    /// fields (the correlation).
    #[must_use]
    pub fn subscriber(&self) -> impl tracing::Subscriber + Send + Sync + use<> {
        use tracing_subscriber::layer::SubscriberExt as _;
        let ours = tracing_subscriber::filter::Targets::new()
            .with_target("loom", tracing::Level::INFO)
            .with_target("loom_app", tracing::Level::INFO);
        tracing_subscriber::fmt()
            .json()
            .with_current_span(true)
            .with_span_list(false)
            .flatten_event(true)
            .with_target(true)
            .with_ansi(false)
            .with_max_level(tracing::Level::INFO)
            .with_writer(self.clone())
            .finish()
            .with(ours)
    }

    /// The files that exist, newest first.
    #[must_use]
    pub fn files(&self) -> Vec<PathBuf> {
        self.file
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .files()
    }
}

/// One event's bytes, written as one redacted line when dropped.
#[derive(Debug)]
pub struct Line {
    buf: Vec<u8>,
    logs: Logs,
}

impl Write for Line {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for Line {
    fn drop(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(&self.buf);
        let mut line = self.logs.redactor.redact(&text);
        if !line.ends_with('\n') {
            line.push('\n');
        }
        // A log that cannot be written is dropped, never the application.
        let _ = self
            .logs
            .file
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .write_line(line.as_bytes());
    }
}

impl<'a> MakeWriter<'a> for Logs {
    type Writer = Line;

    fn make_writer(&'a self) -> Line {
        Line {
            buf: Vec::new(),
            logs: self.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("loom-logs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn redaction_should_replace_the_home_folder_in_every_form_and_the_user_name() {
        let r = Redactor::new(Some(Path::new(r"C:\Users\Alice")), Some("alice"));
        let line = r#"{"path":"C:\\Users\\Alice\\builds\\a.xml","p2":"c:/users/alice/x","who":"ALICE","raw":"C:\Users\Alice\y"}"#;
        let out = r.redact(line);
        assert_eq!(
            out,
            r#"{"path":"~\\builds\\a.xml","p2":"~/x","who":"<user>","raw":"~\y"}"#
        );
        // Short names are left alone.
        assert_eq!(Redactor::new(None, Some("al")).redact("total"), "total");
        // A path `Debug`-formatted into a JSON field, escaped twice.
        let twice = r#"{"p":"\"C:\\\\Users\\\\Alice\\\\b\""}"#;
        assert_eq!(r.redact(twice), r#"{"p":"\"~\\\\b\""}"#);
        // The user name only as a word.
        let bob = Redactor::new(None, Some("bob"));
        assert_eq!(
            bob.redact("bob bobsled kabob Bob."),
            "<user> bobsled kabob <user>."
        );
    }

    #[test]
    fn rotation_should_keep_the_newest_files_up_to_the_limit() {
        let dir = temp("rotation");
        let mut f = RotatingFile::open(&dir, 20, 3).unwrap();
        for i in 0..10 {
            f.write_line(format!("line {i:02} ......\n").as_bytes())
                .unwrap();
        }
        let files = f.files();
        assert_eq!(files.len(), 3, "{files:?}");
        let read = |n: usize| std::fs::read_to_string(dir.join(file_name(n))).unwrap();
        assert_eq!(read(0), "line 09 ......\n");
        assert_eq!(read(1), "line 08 ......\n");
        assert_eq!(read(2), "line 07 ......\n");
        assert!(!dir.join(file_name(3)).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn events_should_be_json_lines_with_their_span_and_redacted() {
        let dir = temp("events");
        let home = std::env::temp_dir().join("HomeOfTester");
        let logs = Logs::with_limits(
            &dir,
            Redactor::new(Some(&home), Some("tester")),
            MAX_BYTES,
            KEEP,
        )
        .unwrap();
        tracing::subscriber::with_default(logs.subscriber(), || {
            let span = tracing::info_span!("command", correlation = 7_u64);
            let _in = span.enter();
            tracing::info!(
                command = "import-text",
                path = %home.join("a.xml").display(),
                "done"
            );
            tracing::debug!("not written");
        });
        let text = std::fs::read_to_string(dir.join(CURRENT)).unwrap();
        let lines: Vec<serde_json::Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 1, "{text}");
        let line = &lines[0];
        assert_eq!(line["command"], "import-text");
        assert_eq!(line["span"]["correlation"], 7);
        assert_eq!(line["level"], "INFO");
        assert!(line["path"].as_str().unwrap().starts_with('~'), "{line}");
        assert!(
            !text.to_ascii_lowercase().contains("homeoftester"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn digest_should_give_size_and_sha256() {
        let (n, sha) = digest(b"abc");
        assert_eq!(n, 3);
        assert_eq!(
            sha,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
