//! The data feed adapter (Phase 5, P4; plan decision P-e;
//! `docs/phase-5/data-bundles.md`): fetches `data-feed.json` and its
//! signature from the URLs the shell gives (its release channel's,
//! `loom_tauri::release::Channel::data_feed`), and downloads a pack it
//! lists. Every
//! check is elsewhere: the feed's signature and sequence in
//! `loom_data::pack::verify_feed`, the pack's in `loom_app::install`; this
//! module only moves bytes, with size caps, and checks a download's size
//! and SHA-256 against the feed before anything reads it.
//!
//! Never called in strict-offline mode (the shell refuses first). HTTPS
//! through the OS's TLS (`native-tls`; ADR-011: no telemetry, no
//! identifier; GitHub sees the IP address).

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};

/// How long a small fetch (the feed, its signature) may take in all.
const FETCH_TIMEOUT: Duration = Duration::from_secs(60);
/// How long a connection may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
/// A download's time: two minutes plus a second per 64 KiB, so a link of
/// about 0.5 Mbit/s or faster finishes (the blocking client has no
/// per-read timeout; a 130 MB pack gets about 37 minutes).
fn download_timeout(size: u64) -> Duration {
    Duration::from_secs(120_u64.saturating_add(size / (64 * 1024)))
}

/// Why a fetch or download failed.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    /// The request failed or the server did not answer 200.
    #[error("{0}")]
    Http(String),
    /// More bytes than allowed.
    #[error("larger than {0} bytes")]
    TooLarge(u64),
    /// A download's size or SHA-256 is not the feed's.
    #[error("the download does not match the feed")]
    Mismatch,
    /// Writing the download failed.
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl FetchError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Http(_) => "FEED_HTTP",
            Self::TooLarge(_) => "FEED_TOO_LARGE",
            Self::Mismatch => "FEED_MISMATCH",
            Self::Io(_) => "FEED_IO",
        }
    }
}

/// The client: HTTPS only, redirects included (the unit tests serve
/// plain HTTP on loopback); a whole-request timeout for small fetches,
/// one scaled to the size for a download ([`download_timeout`]).
fn client(download: Option<u64>) -> Result<reqwest::blocking::Client, FetchError> {
    let builder = reqwest::blocking::Client::builder()
        .https_only(cfg!(not(test)))
        .connect_timeout(CONNECT_TIMEOUT)
        .user_agent(concat!("exiled-studio/", env!("CARGO_PKG_VERSION")));
    let builder = builder.timeout(download.map_or(FETCH_TIMEOUT, download_timeout));
    builder.build().map_err(|e| FetchError::Http(e.to_string()))
}

fn get(url: &str, download: Option<u64>) -> Result<reqwest::blocking::Response, FetchError> {
    let response = client(download)?
        .get(url)
        .send()
        .map_err(|e| FetchError::Http(e.to_string()))?;
    if !response.status().is_success() {
        return Err(FetchError::Http(format!("{url}: {}", response.status())));
    }
    Ok(response)
}

/// Fetches a small file whole, refusing more than `cap` bytes.
///
/// # Errors
/// [`FetchError::Http`], [`FetchError::TooLarge`].
pub fn fetch(url: &str, cap: u64) -> Result<Vec<u8>, FetchError> {
    let mut out = Vec::new();
    get(url, None)?
        .take(cap.saturating_add(1))
        .read_to_end(&mut out)
        .map_err(|e| FetchError::Http(e.to_string()))?;
    if out.len() as u64 > cap {
        return Err(FetchError::TooLarge(cap));
    }
    Ok(out)
}

/// Downloads a pack to `dest`: at most `size` bytes, then its size and
/// SHA-256 must be the feed's. A failed download leaves no file.
///
/// # Errors
/// [`FetchError`].
pub fn download(url: &str, sha256: &str, size: u64, dest: &Path) -> Result<(), FetchError> {
    let result = (|| {
        let mut body = get(url, Some(size))?.take(size.saturating_add(1));
        let mut file = std::fs::File::create(dest)?;
        let mut hasher = Sha256::new();
        let mut written: u64 = 0;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = body
                .read(&mut buf)
                .map_err(|e| FetchError::Http(e.to_string()))?;
            if n == 0 {
                break;
            }
            let chunk = buf.get(..n).unwrap_or_default();
            hasher.update(chunk);
            file.write_all(chunk)?;
            written += n as u64;
        }
        file.flush()?;
        let hash: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if written != size || hash != sha256 {
            return Err(FetchError::Mismatch);
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    /// A loopback server answering every request with `body`.
    fn serve(body: Vec<u8>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|_| unreachable!());
        let base = format!(
            "http://{}",
            listener.local_addr().unwrap_or_else(|_| unreachable!())
        );
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut reader =
                    BufReader::new(stream.try_clone().unwrap_or_else(|_| unreachable!()));
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
                    line.clear();
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        base
    }

    fn sha(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn fetch_should_refuse_more_than_its_cap() {
        let base = serve(vec![b'x'; 100]);
        assert!(fetch(&format!("{base}/f"), 100).is_ok_and(|b| b.len() == 100));
        assert!(matches!(
            fetch(&format!("{base}/f"), 99),
            Err(FetchError::TooLarge(99))
        ));
    }

    #[test]
    fn download_should_check_size_and_hash_and_leave_nothing_on_failure() {
        let body = b"a pack".to_vec();
        let base = serve(body.clone());
        let dir = std::env::temp_dir().join(format!("loom-feed-dl-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let dest = dir.join("p.loombundle");
        let url = format!("{base}/p");
        assert!(download(&url, &sha(&body), body.len() as u64, &dest).is_ok());
        assert_eq!(std::fs::read(&dest).ok(), Some(body.clone()));
        let _ = std::fs::remove_file(&dest);
        // Another hash, a smaller size (the server sends more: cut, then
        // refused), a larger size.
        for (hash, size) in [
            ("0".repeat(64), body.len() as u64),
            (sha(&body), body.len() as u64 - 1),
            (sha(&body), body.len() as u64 + 1),
        ] {
            assert!(matches!(
                download(&url, &hash, size, &dest),
                Err(FetchError::Mismatch)
            ));
            assert!(!dest.exists());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
