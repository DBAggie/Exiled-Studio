//! `exiled-studio-knowledge-mcp`: the knowledge MCP server over stdio
//! (ADR-018). The MCP client starts it; it exits when stdin closes.
//!
//! ```text
//! exiled-studio-knowledge-mcp [--game poe1|poe2] [--data-dir <dir>] [--bundle <dir>] [--snapshots <dir>]
//! exiled-studio-knowledge-mcp --version
//! ```
//! - `--data-dir`: the app's data folder (default: `LOOM_DATA_DIR`, else the
//!   portable package's `data/` beside this program, else the app's data
//!   folder for this user: the stable app's, or the experimental app's when
//!   only that one has bundles). The newest installed bundle of the game
//!   signed by a trusted key is served.
//! - `--bundle`: a bundle folder for development, served unsigned.
//! - `--snapshots`: the knowledge pack's economy snapshots folder.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use loom_domain::Game;
use loom_knowledge_mcp::knowledge::{self, Knowledge, Source};
use loom_knowledge_mcp::rpc;
use loom_knowledge_mcp::snapshots::Snapshots;

/// The app's identifiers, stable first (`crates/loom-tauri/tauri.conf.json`,
/// `tauri.experimental.conf.json`).
const APP_IDS: [&str; 2] = [
    "io.github.dbaggie.exiledstudio",
    "io.github.dbaggie.exiledstudio.experimental",
];

struct Args {
    game: Game,
    data_dir: Option<PathBuf>,
    bundle: Option<PathBuf>,
    snapshots: Option<PathBuf>,
}

fn parse(mut it: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut a = Args {
        game: Game::Poe1,
        data_dir: None,
        bundle: None,
        snapshots: None,
    };
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--game" => {
                a.game = match value()?.as_str() {
                    "poe1" => Game::Poe1,
                    "poe2" => Game::Poe2,
                    g => return Err(format!("unknown game {g:?}")),
                }
            }
            "--data-dir" => a.data_dir = Some(PathBuf::from(value()?)),
            "--bundle" => a.bundle = Some(PathBuf::from(value()?)),
            "--snapshots" => a.snapshots = Some(PathBuf::from(value()?)),
            _ => return Err(format!("unknown argument {flag:?}")),
        }
    }
    Ok(a)
}

/// The app's data folder, as the shell picks it (`loom-tauri`).
fn default_data_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("LOOM_DATA_DIR") {
        return Some(PathBuf::from(d));
    }
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_owned();
    if exe_dir.join("portable").is_file() {
        return Some(exe_dir.join("data"));
    }
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    let base = base?;
    let dirs = APP_IDS.map(|id| base.join(id));
    // The first with installed bundles; the stable one when neither has.
    dirs.iter()
        .find(|d| d.join(loom_bundles::BUNDLES_DIR).is_dir())
        .or(dirs.first())
        .cloned()
}

fn main() -> ExitCode {
    if std::env::args().skip(1).any(|a| a == "--version") {
        println!("exiled-studio-knowledge-mcp {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(m) => {
            eprintln!("{m}");
            return ExitCode::from(2);
        }
    };
    let source = match (args.bundle, args.data_dir.or_else(default_data_dir)) {
        (Some(b), _) => Source::Folder(b),
        (None, Some(d)) => Source::Installed(d),
        (None, None) => Source::Installed(PathBuf::from(".loom")),
    };
    let trust = loom_bundles::trust_list(false).unwrap_or_else(|e| {
        eprintln!(
            "knowledge-mcp: the compiled trust list is unreadable ({e}); no bundle is trusted"
        );
        loom_data::signature::TrustList::default()
    });
    let data = knowledge::open(args.game, &source, &trust);
    match &data {
        Ok(d) => eprintln!(
            "knowledge-mcp: {} patch {} (signed: {})",
            d.provenance.game, d.provenance.patch, d.provenance.signed
        ),
        Err(e) => eprintln!("knowledge-mcp: {}: {e}", e.code()),
    }
    let snapshots = args
        .snapshots
        .as_deref()
        .map(Snapshots::read)
        .unwrap_or_default();
    eprintln!(
        "knowledge-mcp: {} snapshot(s), {} skipped",
        snapshots.items.len(),
        snapshots.skipped.len()
    );
    let k = Knowledge {
        game: args.game,
        data,
        snapshots,
    };
    let mut input = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut buf = Vec::new();
    while let Ok(Some(fits)) = rpc::read_line(&mut input, &mut buf) {
        let out = if fits {
            // Invalid UTF-8 becomes U+FFFD and then a JSON parse error,
            // not the end of the server.
            rpc::handle(&k, String::from_utf8_lossy(&buf).trim_end_matches('\r'))
        } else {
            Some(rpc::too_large())
        };
        if let Some(out) = out
            && writeln!(stdout, "{out}")
                .and_then(|()| stdout.flush())
                .is_err()
        {
            break;
        }
    }
    ExitCode::SUCCESS
}
