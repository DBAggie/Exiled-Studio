//! The knowledge pack's economy snapshots (ADR-018, "Data it serves"):
//! `<dir>/<league-slug>/<date>.json`, written by the pack's refresh job.
//! Unsigned, untrusted input: each file is size-capped and its header
//! checked; a file that fails is skipped and listed, never served.

use std::path::Path;

use serde::Serialize;
use serde_json::Value;

/// The largest snapshot file read.
pub const MAX_FILE: u64 = 32 * 1024 * 1024;

/// The most snapshot files read.
pub const MAX_FILES: usize = 400;

/// The most bytes read over every snapshot file: the files are untrusted
/// and held parsed in memory, so the per-file cap alone is not a bound.
pub const MAX_TOTAL: u64 = 128 * 1024 * 1024;

const VERIFICATION: [&str; 4] = [
    "unverified",
    "engine-checked",
    "in-game-checked",
    "maintainer-approved",
];

/// A snapshot's header (the pack's knowledge header).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Header {
    /// `poe1` or `poe2`.
    pub game: String,
    /// The patch it was taken on.
    pub patch: String,
    /// The league's name.
    pub league: String,
    /// `YYYY-MM-DD`.
    pub as_of: String,
    /// How far it has been checked.
    pub verification: String,
    /// `current` or `historical`.
    pub status: String,
}

/// One snapshot file.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// The file, relative to the snapshots folder (`/`-separated).
    pub file: String,
    /// Its header.
    pub header: Header,
    /// poe.ninja's league ID, when the file names one.
    pub league_id: Option<String>,
    /// When it was fetched, when the file says.
    pub fetched_utc: Option<String>,
    /// The file's content.
    pub body: Value,
}

/// A file that was not read, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Skipped {
    /// The file, relative to the snapshots folder.
    pub file: String,
    /// Why.
    pub reason: String,
}

/// Every snapshot read, and every file skipped.
#[derive(Debug, Clone, Default)]
pub struct Snapshots {
    /// Snapshots, newest `as_of` first.
    pub items: Vec<Snapshot>,
    /// Files skipped.
    pub skipped: Vec<Skipped>,
}

impl Snapshots {
    /// Reads every `*.json` under `dir` (two levels: `<league>/<date>.json`).
    #[must_use]
    pub fn read(dir: &Path) -> Self {
        let mut out = Self::default();
        let mut files = Vec::new();
        for league in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            if !league.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            for f in std::fs::read_dir(league.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                let path = f.path();
                if f.file_type().is_ok_and(|t| t.is_file())
                    && path.extension().is_some_and(|e| e == "json")
                {
                    files.push(path);
                }
            }
        }
        // Newest first by name (`<league>/<date>.json`), so the caps skip
        // the oldest files.
        files.sort_unstable_by(|a, b| b.cmp(a));
        let mut total: u64 = 0;
        for path in files {
            let rel = path.strip_prefix(dir).map_or_else(
                |_| String::new(),
                |p| p.to_string_lossy().replace('\\', "/"),
            );
            if out.items.len() + out.skipped.len() >= MAX_FILES {
                out.skipped.push(Skipped {
                    file: rel,
                    reason: format!("more than {MAX_FILES} files"),
                });
                continue;
            }
            let budget = MAX_TOTAL.saturating_sub(total);
            match read_one(&path, budget) {
                Ok((header, body, len)) => {
                    total = total.saturating_add(len);
                    out.items.push(Snapshot {
                        file: rel,
                        league_id: body
                            .get("league_id")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        fetched_utc: body
                            .get("fetched_utc")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        header,
                        body,
                    });
                }
                Err(reason) => out.skipped.push(Skipped { file: rel, reason }),
            }
        }
        out.items.sort_by(|a, b| {
            b.header
                .as_of
                .cmp(&a.header.as_of)
                .then_with(|| a.file.cmp(&b.file))
        });
        out
    }

    /// The newest snapshot of `game` for `league` (its name or poe.ninja ID,
    /// any case); without a league, the newest `current` one of the game.
    #[must_use]
    pub fn newest(&self, game: &str, league: Option<&str>) -> Option<&Snapshot> {
        self.items.iter().find(|s| {
            s.header.game == game
                && match league {
                    Some(l) => {
                        s.header.league.eq_ignore_ascii_case(l)
                            || s.league_id
                                .as_deref()
                                .is_some_and(|id| id.eq_ignore_ascii_case(l))
                    }
                    None => s.header.status == "current",
                }
        })
    }
}

/// Reads one file of at most `budget` bytes (and [`MAX_FILE`]); returns its
/// length too.
fn read_one(path: &Path, budget: u64) -> Result<(Header, Value, u64), String> {
    let cap = MAX_FILE.min(budget);
    let bytes = loom_bundles::read_capped(path, cap).map_err(|_| {
        if cap < MAX_FILE {
            format!("unreadable, or past the {MAX_TOTAL}-byte total for every snapshot")
        } else {
            format!("unreadable or larger than {MAX_FILE} bytes")
        }
    })?;
    let body: Value = serde_json::from_slice(&bytes).map_err(|e| format!("not JSON: {e}"))?;
    let header = header(body.get("header").ok_or("no header")?)?;
    Ok((header, body, bytes.len() as u64))
}

/// Checks a header object against the pack's header schema (the fields the
/// server relies on).
///
/// # Errors
/// The first field that is missing or not allowed.
pub fn header(h: &Value) -> Result<Header, String> {
    let text = |k: &str| -> Result<String, String> {
        h.get(k)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("header: `{k}` missing"))
    };
    let game = text("game")?;
    if game != "poe1" && game != "poe2" {
        return Err(format!("header: unknown game {game:?}"));
    }
    let as_of = text("as_of")?;
    let date_ok = as_of.len() == 10
        && as_of.bytes().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }
        });
    if !date_ok {
        return Err("header: `as_of` is not YYYY-MM-DD".to_owned());
    }
    let verification = text("verification")?;
    if !VERIFICATION.contains(&verification.as_str()) {
        return Err(format!("header: unknown verification {verification:?}"));
    }
    if !h
        .get("sources")
        .and_then(Value::as_array)
        .is_some_and(|s| !s.is_empty())
    {
        return Err("header: `sources` missing".to_owned());
    }
    let status = h
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("current")
        .to_owned();
    if status != "current" && status != "historical" {
        return Err(format!("header: unknown status {status:?}"));
    }
    Ok(Header {
        game,
        patch: text("patch")?,
        league: text("league")?,
        as_of,
        verification,
        status,
    })
}
