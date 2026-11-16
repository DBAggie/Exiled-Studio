//! Crash records (Phase 5, P-f; ADR-014): a panic anywhere in the process
//! writes a small JSON record to `<data>/crashes/`, redacted like the logs
//! ([`crate::logs::Redactor`]). The next start lists the records, the user
//! sees them beside the recovery snapshot the application already offers,
//! and dismisses them; the diagnostic package includes them. Nothing is
//! sent anywhere.
//!
//! A panic the application catches (a calculation's, which fails that
//! calculation only) is recorded too: it is an internal error worth a
//! report even when the application keeps running.
//!
//! A formatted panic message can quote what the code was reading (a build
//! line in an `unwrap` of a parse error), and build text is never kept
//! (ADR-014). So a message made at run time ([`scrub`]) keeps its words but
//! not its quoted spans, and is cut to [`MAX_MESSAGE`] characters; a
//! literal message (`panic!("…")` without arguments) is kept as written.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::logs::Redactor;

/// The crash records folder, under the data folder.
pub const CRASH_DIR: &str = "crashes";

/// The most records kept; older ones are deleted when a new one is written.
pub const KEEP: usize = 20;

/// The most characters of a run-time panic message kept.
pub const MAX_MESSAGE: usize = 200;

/// A run-time panic message without what it quotes: each `"…"` and `'…'`
/// span becomes `"…"`, then the text is cut to [`MAX_MESSAGE`] characters.
#[must_use]
pub fn scrub(message: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in message.chars() {
        match quote {
            Some(q) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    out.push_str("…\"");
                    quote = None;
                }
            }
            None if c == '"' || c == '\'' => {
                // An apostrophe inside a word (`don't`) is not a quote.
                if c == '\'' && out.chars().next_back().is_some_and(char::is_alphanumeric) {
                    out.push(c);
                } else {
                    out.push('"');
                    quote = Some(c);
                }
            }
            None => out.push(c),
        }
    }
    if quote.is_some() {
        out.push_str("…\"");
    }
    if out.chars().count() > MAX_MESSAGE {
        out = out.chars().take(MAX_MESSAGE).collect();
        out.push('…');
    }
    out
}

/// The ID's parts (`<seconds>-<pid>-<n>`) as numbers, for ordering.
fn id_key(id: &str) -> Vec<u64> {
    id.split('-').map(|p| p.parse().unwrap_or(0)).collect()
}

/// One panic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrashRecord {
    /// Its ID, also its file name without `.json`: the time and the
    /// process (`<seconds>-<pid>-<n>`).
    pub id: String,
    /// When, in seconds since 1970 (UTC).
    pub at: u64,
    /// The application's version.
    pub version: String,
    /// The thread's name.
    pub thread: String,
    /// The panic's message, redacted.
    pub message: String,
    /// Where it panicked (`file:line:column`), redacted.
    pub location: Option<String>,
}

/// Whether `id` is a record ID this module writes (digits and `-`), so a
/// path made from it stays in the folder.
#[must_use]
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_digit() || b == b'-')
}

/// The record of a panic with `message` at `location` on `thread`.
#[must_use]
pub fn record(
    redactor: &Redactor,
    n: u64,
    thread: &str,
    message: &str,
    location: Option<&str>,
) -> CrashRecord {
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    CrashRecord {
        id: format!("{at}-{}-{n}", std::process::id()),
        at,
        version: env!("CARGO_PKG_VERSION").to_owned(),
        thread: redactor.redact(thread),
        message: redactor.redact(message),
        location: location.map(|l| redactor.redact(l)),
    }
}

/// Writes `record` to `<data_dir>/crashes/`, then deletes the oldest
/// records beyond [`KEEP`].
///
/// # Errors
/// The folder or the file cannot be written.
pub fn write(data_dir: &Path, record: &CrashRecord) -> std::io::Result<PathBuf> {
    let dir = data_dir.join(CRASH_DIR);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", record.id));
    let mut text = serde_json::to_string_pretty(record).map_err(std::io::Error::other)?;
    text.push('\n');
    std::fs::write(&path, text)?;
    let all = list(data_dir);
    for old in all.iter().skip(KEEP) {
        let _ = std::fs::remove_file(dir.join(format!("{}.json", old.id)));
    }
    Ok(path)
}

/// The records in `<data_dir>/crashes/`, newest first; files that do not
/// read as a record are skipped.
#[must_use]
pub fn list(data_dir: &Path) -> Vec<CrashRecord> {
    let Ok(entries) = std::fs::read_dir(data_dir.join(CRASH_DIR)) else {
        return Vec::new();
    };
    let mut out: Vec<CrashRecord> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| std::fs::read(e.path()).ok())
        .filter_map(|b| serde_json::from_slice::<CrashRecord>(&b).ok())
        .filter(|r| valid_id(&r.id))
        .collect();
    out.sort_by(|a, b| {
        b.at.cmp(&a.at)
            .then_with(|| id_key(&b.id).cmp(&id_key(&a.id)))
    });
    out
}

/// Deletes record `id`; `false` when the ID is not valid or there is no
/// such record.
#[must_use]
pub fn dismiss(data_dir: &Path, id: &str) -> bool {
    valid_id(id)
        && std::fs::remove_file(data_dir.join(CRASH_DIR).join(format!("{id}.json"))).is_ok()
}

std::thread_local! {
    /// Set while this thread runs the hook: a panic inside the hook (in the
    /// logs' writer, say) is not recorded again.
    static IN_HOOK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Installs the panic hook: each panic writes a record, then an `ERROR`
/// event (which the logs keep), then runs the hook that was there before.
/// The record is written first, so a log writer that cannot proceed does
/// not cost the record.
pub fn install(data_dir: PathBuf, redactor: Redactor) {
    let previous = std::panic::take_hook();
    let count = std::sync::atomic::AtomicU64::new(0);
    std::panic::set_hook(Box::new(move |info| {
        if IN_HOOK.with(|f| f.replace(true)) {
            previous(info);
            return;
        }
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| scrub(s)))
            .unwrap_or_else(|| "a panic without a message".to_owned());
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        let thread = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_owned();
        let n = count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let r = record(&redactor, n, &thread, &message, location.as_deref());
        let _ = write(&data_dir, &r);
        tracing::error!(target: "loom::crash", id = %r.id, thread = %r.thread, location = ?r.location, "panic");
        IN_HOOK.with(|f| f.set(false));
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_should_be_written_listed_newest_first_and_dismissed() {
        let dir = std::env::temp_dir().join(format!("loom-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let redactor = Redactor::new(Some(Path::new("/home/tester")), Some("tester"));
        let mut a = record(
            &redactor,
            0,
            "main",
            "boom in /home/tester/x",
            Some("/home/tester/src/a.rs:1:2"),
        );
        a.at = 10;
        a.id = "10-1-0".to_owned();
        let mut b = record(&redactor, 1, "loom-calc", "second", None);
        b.at = 20;
        b.id = "20-1-1".to_owned();
        write(&dir, &a).unwrap();
        write(&dir, &b).unwrap();
        let listed = list(&dir);
        assert_eq!(
            listed.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["20-1-1", "10-1-0"]
        );
        assert_eq!(listed[1].message, "boom in ~/x");
        assert_eq!(listed[1].location.as_deref(), Some("~/src/a.rs:1:2"));
        assert!(!dismiss(&dir, "../x"));
        // Ties in time order by the ID's numbers, not its text.
        let mut c = b.clone();
        c.id = "20-1-10".to_owned();
        let mut d = b.clone();
        d.id = "20-1-9".to_owned();
        write(&dir, &c).unwrap();
        write(&dir, &d).unwrap();
        let ids: Vec<String> = list(&dir).into_iter().map(|r| r.id).collect();
        assert_eq!(ids, ["20-1-10", "20-1-9", "20-1-1", "10-1-0"]);
        assert!(dismiss(&dir, "20-1-10") && dismiss(&dir, "20-1-9"));
        assert!(dismiss(&dir, "10-1-0"));
        assert_eq!(list(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scrub_should_drop_quoted_spans_and_cut_long_messages() {
        assert_eq!(
            scrub(r#"called `Result::unwrap()` on an `Err` value: Parse("Ring: \"x\" y")"#),
            "called `Result::unwrap()` on an `Err` value: Parse(\"…\")"
        );
        assert_eq!(scrub("don't read 'secret'"), "don't read \"…\"");
        assert_eq!(scrub("open \"unterminated"), "open \"…\"");
        let long = scrub(&"x".repeat(500));
        assert_eq!(long.chars().count(), MAX_MESSAGE + 1);
    }

    #[test]
    fn the_hook_should_write_a_record_for_a_caught_panic() {
        let dir = std::env::temp_dir().join(format!("loom-crash-hook-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        install(dir.clone(), Redactor::default());
        let caught = std::thread::Builder::new()
            .name("crash-test".to_owned())
            .spawn(|| {
                let a = std::panic::catch_unwind(|| panic!("deliberate test panic")).is_err();
                let line = "Ring: Loom Test Band";
                let b = std::panic::catch_unwind(|| panic!("cannot read \"{line}\" at 3")).is_err();
                a && b
            })
            .unwrap()
            .join()
            .unwrap();
        // Other tests' hooks are not affected: the previous hook runs too.
        let _ = std::panic::take_hook();
        assert!(caught);
        let records = list(&dir);
        let mine: Vec<&CrashRecord> = records
            .iter()
            .filter(|r| r.thread == "crash-test")
            .collect();
        assert_eq!(mine.len(), 2, "{records:?}");
        let literal = mine
            .iter()
            .find(|r| r.message == "deliberate test panic")
            .unwrap();
        assert!(literal.location.as_deref().unwrap().contains("crash.rs"));
        // A formatted message keeps its words, not the build text it quotes.
        assert!(
            mine.iter().any(|r| r.message == "cannot read \"…\" at 3"),
            "{mine:?}"
        );
        assert!(!records.iter().any(|r| r.message.contains("Loom Test Band")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
