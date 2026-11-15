//! The diagnostic package (Phase 5, P-f; ADR-014): what a user can choose
//! to send with a problem report, previewed first and written only where
//! the user picks. Nothing is sent anywhere.
//!
//! The package is a list of text files ([`Package`]): `about.json` (the
//! application's version, the platform, the loaded data's version and
//! bundle ID, never a path), each log file (`logs/loom.log`, ...), and each
//! crash record (`crashes/<id>.json`). Every file is redacted again
//! ([`Redactor`]) when the package is made. The preview shows exactly
//! these files, and the zip ([`zip`]) holds exactly them: the same names
//! and bytes, stored without compression.

use std::path::Path;

use serde_json::json;

use crate::crash::{self, CRASH_DIR};
use crate::logs::{self, LOG_DIR, Redactor};

/// One file of the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// Its name in the package (`/`-separated).
    pub name: String,
    /// Its text, redacted.
    pub text: String,
}

/// The package's files, in order.
pub type Package = Vec<File>;

/// The largest text a log file contributes: its last bytes (a whole log is
/// at most [`logs::MAX_BYTES`]; this keeps the package and its preview
/// small).
pub const MAX_FILE_BYTES: usize = 1024 * 1024;

/// The facts `about.json` names, from the application.
#[derive(Debug, Clone, Default)]
pub struct About {
    /// The loaded data's version (game, data schema, Steam build), if any.
    pub data_version: Option<serde_json::Value>,
    /// The loaded bundle's ID, when it is an installed one (never its
    /// folder).
    pub bundle: Option<String>,
    /// Whether a build is open.
    pub build_open: bool,
}

/// The package for the data folder `data_dir`.
#[must_use]
pub fn collect(data_dir: &Path, about: &About, redactor: &Redactor) -> Package {
    let mut out = Vec::new();
    let about = json!({
        "application": "Exiled Studio",
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "data_version": about.data_version,
        "bundle": about.bundle,
        "build_open": about.build_open,
    });
    let mut text = serde_json::to_string_pretty(&about).unwrap_or_default();
    text.push('\n');
    out.push(File {
        name: "about.json".to_owned(),
        text: redactor.redact(&text),
    });
    for path in logs::files_in(&data_dir.join(LOG_DIR), logs::KEEP) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        // The last whole lines within the limit: the cut moves to the
        // next line's start, so no line (or character) is split.
        let tail = bytes
            .len()
            .checked_sub(MAX_FILE_BYTES)
            .map_or(bytes.as_slice(), |cut| {
                let rest = bytes.get(cut..).unwrap_or_default();
                rest.iter()
                    .position(|&b| b == b'\n')
                    .and_then(|n| rest.get(n + 1..))
                    .unwrap_or_default()
            });
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        out.push(File {
            name: format!("{LOG_DIR}/{name}"),
            text: redactor.redact(&String::from_utf8_lossy(tail)),
        });
    }
    for r in crash::list(data_dir) {
        let mut text = serde_json::to_string_pretty(&r).unwrap_or_default();
        text.push('\n');
        out.push(File {
            name: format!("{CRASH_DIR}/{}.json", r.id),
            text: redactor.redact(&text),
        });
    }
    out
}

/// The package as a zip file: each file stored (no compression), in order,
/// with a fixed time (1980-01-01), so the same package gives the same
/// bytes.
///
/// # Errors
/// A file, its name, or the whole is too large for a zip without the
/// 64-bit extension, or there are more than 65,535 files; [`collect`]
/// stays far below each, so the zip always holds the whole preview.
pub fn zip(package: &[File]) -> std::io::Result<Vec<u8>> {
    let too_large = || std::io::Error::other("the diagnostic package is too large for a zip");
    let mut out = Vec::new();
    let mut central = Vec::new();
    let mut count: u16 = 0;
    for f in package {
        let name = f.name.as_bytes();
        let data = f.text.as_bytes();
        let (Ok(name_len), Ok(size), Ok(offset)) = (
            u16::try_from(name.len()),
            u32::try_from(data.len()),
            u32::try_from(out.len()),
        ) else {
            return Err(too_large());
        };
        let crc = crc32fast::hash(data);
        // The local header: signature, version 2.0, flags (bit 11: UTF-8
        // names), stored, time 0, date 1980-01-01, CRC, sizes, name length,
        // no extra field.
        out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        out.extend_from_slice(&20_u16.to_le_bytes());
        out.extend_from_slice(&0x0800_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(&0x0021_u16.to_le_bytes());
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0_u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        // Its central directory entry.
        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        central.extend_from_slice(&20_u16.to_le_bytes());
        central.extend_from_slice(&20_u16.to_le_bytes());
        central.extend_from_slice(&0x0800_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0_u16.to_le_bytes());
        central.extend_from_slice(&0x0021_u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&name_len.to_le_bytes());
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
        count = count.checked_add(1).ok_or_else(too_large)?;
    }
    let start = u32::try_from(out.len()).map_err(|_| too_large())?;
    let size = u32::try_from(central.len()).map_err(|_| too_large())?;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    Ok(out)
}

/// Reads a zip that [`zip`] wrote back into its files (tests, and the
/// check that the written package equals the preview): the central
/// directory's entries, each from its local header. `None` for anything
/// else.
#[must_use]
pub fn unzip(bytes: &[u8]) -> Option<Package> {
    let u16_at = |at: usize| -> Option<usize> {
        Some(usize::from(u16::from_le_bytes(
            bytes.get(at..at + 2)?.try_into().ok()?,
        )))
    };
    let u32_at = |at: usize| -> Option<usize> {
        usize::try_from(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?)).ok()
    };
    let end = bytes.len().checked_sub(22)?;
    if u32_at(end)? != 0x0605_4b50 {
        return None;
    }
    let count = u16_at(end + 10)?;
    let mut at = u32_at(end + 16)?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if u32_at(at)? != 0x0201_4b50 {
            return None;
        }
        let size = u32_at(at + 20)?;
        let name_len = u16_at(at + 28)?;
        let offset = u32_at(at + 42)?;
        let name = String::from_utf8(bytes.get(at + 46..at + 46 + name_len)?.to_vec()).ok()?;
        let local_name = u16_at(offset + 26)?;
        let local_extra = u16_at(offset + 28)?;
        let data_at = offset + 30 + local_name + local_extra;
        let data = bytes.get(data_at..data_at + size)?;
        if crc32fast::hash(data) != u32::try_from(u32_at(offset + 14)?).ok()? {
            return None;
        }
        out.push(File {
            name,
            text: String::from_utf8(data.to_vec()).ok()?,
        });
        at += 46 + name_len;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zip_should_hold_exactly_the_preview() {
        let dir = std::env::temp_dir().join(format!("loom-diag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let home = Path::new("/home/tester");
        let redactor = Redactor::new(Some(home), Some("tester"));
        std::fs::create_dir_all(dir.join(LOG_DIR)).unwrap();
        std::fs::write(
            dir.join(LOG_DIR).join(logs::CURRENT),
            "{\"msg\":\"opened /home/tester/build.xml\"}\n",
        )
        .unwrap();
        std::fs::write(dir.join(LOG_DIR).join(logs::file_name(1)), "older\n").unwrap();
        let r = crash::record(&redactor, 0, "main", "boom", None);
        crash::write(&dir, &r).unwrap();
        let about = About {
            data_version: Some(json!({"game": "poe1"})),
            bundle: Some("poe1-1-s4".to_owned()),
            build_open: true,
        };
        let package = collect(&dir, &about, &redactor);
        let names: Vec<&str> = package.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "about.json".to_owned(),
                "logs/loom.log".to_owned(),
                "logs/loom.1.log".to_owned(),
                format!("crashes/{}.json", r.id),
            ]
        );
        assert_eq!(package[1].text, "{\"msg\":\"opened ~/build.xml\"}\n");
        assert!(package[0].text.contains("poe1-1-s4"));
        let bytes = zip(&package).unwrap();
        assert_eq!(unzip(&bytes), Some(package.clone()));
        assert_eq!(
            zip(&package).unwrap(),
            bytes,
            "the same package gives the same bytes"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unzip_should_refuse_what_is_not_a_package() {
        assert_eq!(unzip(b"not a zip"), None);
        assert_eq!(unzip(&zip(&[]).unwrap()), Some(Vec::new()));
    }
}
