//! `dto-hashes` (Phase 2 plan, step 7): the SHA-256 of the v1 build
//! document of every PoB build in some folders, for CI to compare between
//! Windows and Linux. Each document is also written twice and read back,
//! so a run checks determinism and the round-trip on its own platform.

use std::path::Path;

use loom_serialization::dto::v1::{from_json, to_json};
use loom_serialization::legacy::pob_xml::import;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// One build's result.
#[derive(Serialize)]
pub struct Document {
    /// The folder as given, `/`, and the file name.
    pub name: String,
    /// The document's SHA-256 (lowercase hex), when it was written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// The document's size in bytes, when it was written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<usize>,
    /// The import's error code, for a file that is not a build.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub import_error: Option<&'static str>,
}

/// The report.
#[derive(Serialize)]
pub struct Report {
    /// Every `*.xml` file, by name.
    pub documents: Vec<Document>,
    /// Builds whose document differs on a second run or does not read back
    /// to the same build.
    pub failures: Vec<String>,
}

/// Runs over every `*.xml` file directly in each folder, in name order.
///
/// # Errors
/// A folder or file that cannot be read.
pub fn run(dirs: &[String]) -> Result<Report, String> {
    let mut report = Report {
        documents: Vec::new(),
        failures: Vec::new(),
    };
    for dir in dirs {
        let mut files = Vec::new();
        for e in std::fs::read_dir(dir).map_err(|e| format!("{dir}: {e}"))? {
            let p = e.map_err(|e| format!("{dir}: {e}"))?.path();
            if p.extension().is_some_and(|x| x == "xml") {
                files.push(p);
            }
        }
        files.sort();
        for path in files {
            let file = path.file_name().map(|n| n.to_string_lossy().into_owned());
            let name = format!(
                "{}/{}",
                dir.trim_end_matches(['/', '\\']),
                file.unwrap_or_default()
            );
            report
                .documents
                .push(document(&path, name, &mut report.failures)?);
        }
    }
    Ok(report)
}

fn document(path: &Path, name: String, failures: &mut Vec<String>) -> Result<Document, String> {
    let xml = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let build = match import(&xml) {
        Ok(b) => b,
        Err(e) => {
            return Ok(Document {
                name,
                sha256: None,
                bytes: None,
                import_error: Some(e.code()),
            });
        }
    };
    let json = to_json(&build).map_err(|e| format!("{name}: {e}"))?;
    if to_json(&build).ok().as_deref() != Some(json.as_str()) {
        failures.push(format!("{name}: a second run differs"));
    }
    match from_json(&json) {
        Ok(b) if b == build => {}
        Ok(_) => failures.push(format!("{name}: the build read back differs")),
        Err(e) => failures.push(format!("{name}: {e}")),
    }
    let digest = Sha256::digest(json.as_bytes());
    Ok(Document {
        name,
        sha256: Some(digest.iter().map(|b| format!("{b:02x}")).collect()),
        bytes: Some(json.len()),
        import_error: None,
    })
}
