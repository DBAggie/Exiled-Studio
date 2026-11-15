//! `tree-layout`: adds the passive tree's layout constants
//! (`loom_data::bundle::tree_layout`) to a bundle, from GGG's passive tree
//! export at the commit pinned in `tools/tree/manifest.json` (ADR-005
//! Amendment 2). Maintainer-run: the export is fetched by
//! `tools/tree/acquire.{ps1,sh}` into `local/tree-export/`.
//!
//! Checks, each refusing the run:
//! - the export's checkout is the pinned commit (its `.git/HEAD`);
//! - its `data.json` `skillsPerOrbit` equals the bundle's orbit slots, and
//!   it has one `orbitRadii` entry per orbit;
//! - its README's table of the 16-slot orbits' angles equals the rule's
//!   (`tree_layout::orbit_angles(16)`);
//! - the bundle validates before, and the written one after.
//!
//! The output is a copy of the bundle with `data/passive_tree_layout.json`
//! added and the manifest's file list updated (nothing else changes), in a
//! new folder.

use std::path::Path;

use loom_data::bundle::tree_layout::{
    ANGLE_RULE, ExportFile, TREE_LAYOUT, TreeExport, TreeLayout, orbit_angles,
};
use loom_data::bundle::{Bundle, FileEntry, MANIFEST, Manifest, MemorySource};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct Pin {
    export: Pinned,
}

#[derive(Deserialize)]
struct Pinned {
    repository: String,
    tag: String,
    commit: String,
}

/// What the run did.
#[derive(Serialize)]
pub struct Summary {
    tag: String,
    commit: String,
    orbit_radii: Vec<f64>,
    orbit_slots: Vec<i64>,
    layout_sha256: String,
    manifest_sha256: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The README's 16-slot angle table: the rows `| index | angle |` after
/// the line naming the angles of orbits 2 and 3.
fn readme_angles(readme: &str) -> Option<Vec<f64>> {
    let start = readme.find("The angle for positions in these orbits")?;
    let mut out = Vec::new();
    for line in readme[start..].lines().skip(1) {
        let cells: Vec<&str> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().trim_matches('*').trim())
            .collect();
        match cells.as_slice() {
            [index, angle] if index.parse::<usize>().is_ok() => {
                out.push(angle.parse::<f64>().ok()?);
            }
            _ if !out.is_empty() => break,
            _ => {}
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Runs the command; `Ok(false)` never (every problem is an error).
pub fn run(pin: &Path, export: &Path, bundle: &Path, out: &Path) -> Result<Summary, String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    let pin: Pin = serde_json::from_slice(&read(pin)?).map_err(|e| format!("pin: {e}"))?;
    let pin = pin.export;
    let head = String::from_utf8_lossy(&read(&export.join(".git").join("HEAD"))?)
        .trim()
        .to_owned();
    if head != pin.commit {
        return Err(format!(
            "the export is at {head}, the pin is {} ({}): run tools/tree/acquire",
            pin.commit, pin.tag
        ));
    }
    let data_bytes = read(&export.join("data.json"))?;
    let readme_bytes = read(&export.join("README.md"))?;
    let data: Value = serde_json::from_slice(&data_bytes).map_err(|e| format!("data.json: {e}"))?;
    let constants = &data["constants"];
    let radii: Vec<f64> = serde_json::from_value(constants["orbitRadii"].clone())
        .map_err(|e| format!("data.json constants.orbitRadii: {e}"))?;
    let slots: Vec<i64> = serde_json::from_value(constants["skillsPerOrbit"].clone())
        .map_err(|e| format!("data.json constants.skillsPerOrbit: {e}"))?;
    let readme = String::from_utf8_lossy(&readme_bytes);
    let documented =
        readme_angles(&readme).ok_or("README.md: no table of the 16-slot orbits' angles")?;
    if documented != orbit_angles(16) {
        return Err(format!(
            "README.md's 16-slot angles {documented:?} differ from the rule's"
        ));
    }

    let source = crate::DirSource(bundle);
    let report = loom_data::bundle::validate::validate(&source);
    if !report.is_valid() {
        return Err(format!(
            "{}: the bundle does not validate",
            bundle.display()
        ));
    }
    let loaded = Bundle::load(&source).map_err(|e| format!("{}: {e}", bundle.display()))?;
    let tree_slots = &loaded
        .passives
        .as_ref()
        .ok_or("the bundle has no passive tree (format 1)")?
        .tree
        .orbit_slots;
    if *tree_slots != slots || radii.len() != slots.len() {
        return Err(format!(
            "the export's orbits ({slots:?}, {} radii) do not match the bundle's {tree_slots:?}",
            radii.len()
        ));
    }
    let layout = TreeLayout {
        orbit_radii: radii.clone(),
        orbit_angles: slots
            .iter()
            .map(|s| orbit_angles(usize::try_from(*s).unwrap_or(0)))
            .collect(),
        source: TreeExport {
            repository: pin.repository,
            tag: pin.tag.clone(),
            commit: pin.commit.clone(),
            files: vec![
                ExportFile {
                    path: "README.md".to_owned(),
                    sha256: hex(&Sha256::digest(&readme_bytes)),
                },
                ExportFile {
                    path: "data.json".to_owned(),
                    sha256: hex(&Sha256::digest(&data_bytes)),
                },
            ],
            angle_rule: ANGLE_RULE.to_owned(),
        },
    };
    if !layout.fits(&slots) {
        return Err("the layout does not fit the orbits".to_owned());
    }
    let mut layout_json = serde_json::to_vec_pretty(&layout).map_err(|e| e.to_string())?;
    layout_json.push(b'\n');

    // The copy: every listed file, the layout, the manifest with it.
    let mut manifest: Manifest = loaded.manifest.clone();
    manifest.files.retain(|f| f.path != TREE_LAYOUT);
    manifest.files.push(FileEntry {
        path: TREE_LAYOUT.to_owned(),
        sha256: hex(&Sha256::digest(&layout_json)),
        size: layout_json.len() as u64,
    });
    manifest.files.sort_by(|a, b| a.path.cmp(&b.path));
    let mut manifest_json =
        serde_json::to_vec_pretty(&serde_json::to_value(&manifest).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    manifest_json.push(b'\n');

    let partial = out.with_extension("partial");
    if partial.exists() {
        return Err(format!(
            "{}: already exists (an earlier run?)",
            partial.display()
        ));
    }
    let mut written = MemorySource::new();
    for f in &manifest.files {
        let bytes = if f.path == TREE_LAYOUT {
            layout_json.clone()
        } else {
            read(&bundle.join(&f.path))?
        };
        let target = partial.join(&f.path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&target, &bytes).map_err(|e| format!("{}: {e}", target.display()))?;
        written.insert(f.path.clone(), bytes);
    }
    std::fs::write(partial.join(MANIFEST), &manifest_json)
        .map_err(|e| format!("{}: {e}", partial.display()))?;
    written.insert(MANIFEST.to_owned(), manifest_json.clone());
    let report = loom_data::bundle::validate::validate(&written);
    if !report.is_valid() {
        return Err(format!(
            "the written bundle does not validate: {}",
            serde_json::to_string(&report).unwrap_or_default()
        ));
    }
    std::fs::rename(&partial, out).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(Summary {
        tag: pin.tag,
        commit: pin.commit,
        orbit_radii: radii,
        orbit_slots: slots,
        layout_sha256: hex(&Sha256::digest(&layout_json)),
        manifest_sha256: hex(&Sha256::digest(&manifest_json)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_angles_should_read_the_table_with_its_bold_cells() {
        let text = "x\nThe angle for positions in these orbits (2 & 3) are now:\n\n| Orbit Index | Angle |\n| --- | --- |\n| 0 | 0 |\n| 1 | 30 |\n| 2 | **45** |\n\n### next\n| 9 | 9 |\n";
        assert_eq!(readme_angles(text), Some(vec![0.0, 30.0, 45.0]));
        assert_eq!(readme_angles("nothing"), None);
    }
}
