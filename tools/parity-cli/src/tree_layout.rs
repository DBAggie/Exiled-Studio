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
//! It adds the tree's sprites too (passive tree plan, T-a;
//! `loom_data::bundle::tree_sprites`): the sheets of [`CATEGORIES`] at
//! the zoom level [`ZOOM`], copied unmodified from the export's `assets/`
//! into `trees/assets/`, and the index `data/passive_tree_sprites.json`
//! (each category's sprites, and the icons of the bundle's tree nodes
//! that a copied sheet has). The sheets live only in the written bundle,
//! never in the repository (ADR-005 Amendment 2).
//!
//! The output is a copy of the bundle with those files added and the
//! manifest's file list updated (nothing else changes), in a new folder.

use std::path::Path;

use std::collections::{BTreeMap, BTreeSet};

use loom_data::bundle::tree_layout::{
    ANGLE_RULE, ExportFile, TREE_LAYOUT, TreeExport, TreeLayout, orbit_angles,
};
use loom_data::bundle::tree_sprites::{
    NodeArt, SPRITES_DIR, Sprite, SpriteCategory, TREE_SPRITES, TreeSprites,
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

/// The export's zoom level whose sheets the bundle carries: the
/// fourth of five (`0.3835`), sharp at the tree screen's usual zooms and
/// about a third of the largest level's bytes.
pub const ZOOM: &str = "0.3835";

/// The export's sprite categories the tree screen draws: the passives'
/// icons (allocated and not), their frames (the jewel sockets' too), and
/// the masteries' icons (unconnected, connected, chosen).
pub const CATEGORIES: [&str; 10] = [
    "frame",
    "keystoneActive",
    "keystoneInactive",
    "masteryActiveSelected",
    "masteryConnected",
    "masteryInactive",
    "normalActive",
    "normalInactive",
    "notableActive",
    "notableInactive",
];

/// What the run did.
#[derive(Serialize)]
pub struct Summary {
    tag: String,
    commit: String,
    orbit_radii: Vec<f64>,
    orbit_slots: Vec<i64>,
    layout_sha256: String,
    sprites_sha256: String,
    sheets: Vec<String>,
    nodes_with_art: usize,
    icons_without_sprite: usize,
    manifest_sha256: String,
}

#[derive(Deserialize)]
struct ExportSheet {
    filename: String,
    w: u32,
    h: u32,
    coords: BTreeMap<String, ExportRect>,
}

#[derive(Deserialize)]
struct ExportRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

/// The sprite index, its sheets by bundle path, and how many icons no
/// copied sheet has.
type Sprites = (TreeSprites, BTreeMap<String, Vec<u8>>, usize);

/// The index and the sheets (bundle path, bytes) from the export's
/// `data.json` and `assets/`: the categories of [`CATEGORIES`] at
/// [`ZOOM`], and the art of each node in `nodes`. Returns the icons it
/// left out because no copied sheet has them, too.
fn sprites(
    data: &Value,
    export: &Path,
    nodes: &BTreeSet<u32>,
    source: TreeExport,
) -> Result<Sprites, String> {
    let zoom: f64 = ZOOM.parse().map_err(|_| "ZOOM")?;
    let mut categories = Vec::new();
    let mut sheets = BTreeMap::new();
    for name in CATEGORIES {
        let sheet: ExportSheet = serde_json::from_value(data["sprites"][name][ZOOM].clone())
            .map_err(|e| format!("data.json sprites.{name}.{ZOOM}: {e}"))?;
        // `https://.../passive-skill/skills-3.jpg?64320931`: the file name.
        let file = sheet
            .filename
            .split('?')
            .next()
            .and_then(|u| u.rsplit('/').next())
            .filter(|f| !f.is_empty() && !f.starts_with('.'))
            .ok_or_else(|| format!("{name}: file {}", sheet.filename))?
            .to_owned();
        let path = format!("{SPRITES_DIR}{file}");
        if let std::collections::btree_map::Entry::Vacant(e) = sheets.entry(path) {
            e.insert(read(&export.join("assets").join(&file))?);
        }
        categories.push(SpriteCategory {
            name: name.to_owned(),
            file,
            width: sheet.w,
            height: sheet.h,
            sprites: sheet
                .coords
                .into_iter()
                .map(|(name, r)| Sprite {
                    name,
                    x: r.x,
                    y: r.y,
                    w: r.w,
                    h: r.h,
                })
                .collect(),
        });
    }
    let known: BTreeSet<&str> = categories
        .iter()
        .flat_map(|c| c.sprites.iter().map(|s| s.name.as_str()))
        .collect();
    let mut missing = 0;
    let mut keep = |v: &Value| -> Option<String> {
        let icon = v.as_str()?;
        if known.contains(icon) {
            Some(icon.to_owned())
        } else {
            missing += 1;
            None
        }
    };
    let mut art = Vec::new();
    if let Some(all) = data["nodes"].as_object() {
        for (id, n) in all {
            let Some(node) = id.parse::<u32>().ok().filter(|n| nodes.contains(n)) else {
                continue;
            };
            let a = NodeArt {
                node,
                icon: keep(&n["icon"]),
                inactive_icon: keep(&n["inactiveIcon"]),
                active_icon: keep(&n["activeIcon"]),
            };
            if a.icon.is_some() || a.inactive_icon.is_some() || a.active_icon.is_some() {
                art.push(a);
            }
        }
    }
    art.sort_by_key(|a| a.node);
    let mut source = source;
    for path in sheets.keys() {
        let file = path.trim_start_matches(SPRITES_DIR);
        source.files.push(ExportFile {
            path: format!("assets/{file}"),
            sha256: hex(&Sha256::digest(&sheets[path])),
        });
    }
    let index = TreeSprites {
        zoom,
        categories,
        nodes: art,
        source,
    };
    let problems = index.problems();
    if !problems.is_empty() {
        return Err(format!("the sprite index: {problems:?}"));
    }
    Ok((index, sheets, missing))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn read(path: &Path) -> Result<Vec<u8>, String> {
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
    let tree_nodes: BTreeSet<u32> = loaded
        .passives
        .as_ref()
        .map(|p| {
            p.tree
                .groups
                .iter()
                .flat_map(|g| g.nodes.iter())
                .filter_map(|n| u32::try_from(n.node).ok())
                .collect()
        })
        .unwrap_or_default();
    let (index, sheets, icons_without_sprite) =
        sprites(&data, export, &tree_nodes, layout.source.clone())?;
    let mut sprites_json = serde_json::to_vec_pretty(&index).map_err(|e| e.to_string())?;
    sprites_json.push(b'\n');
    let mut added: BTreeMap<String, Vec<u8>> = sheets;
    added.insert(TREE_LAYOUT.to_owned(), layout_json.clone());
    added.insert(TREE_SPRITES.to_owned(), sprites_json.clone());

    let manifest_json = write_copy(
        bundle,
        loaded.manifest.clone(),
        &added,
        |path| path.starts_with(SPRITES_DIR),
        out,
    )?;
    Ok(Summary {
        tag: pin.tag,
        commit: pin.commit,
        orbit_radii: radii,
        orbit_slots: slots,
        layout_sha256: hex(&Sha256::digest(&layout_json)),
        sprites_sha256: hex(&Sha256::digest(&sprites_json)),
        sheets: index.sheet_paths(),
        nodes_with_art: index.nodes.len(),
        icons_without_sprite,
        manifest_sha256: hex(&Sha256::digest(&manifest_json)),
    })
}

/// Writes a copy of the bundle folder `bundle` (its manifest `manifest`)
/// into `out`, which must not exist: every listed file but those `drop`
/// names and those in `added`, then `added`, with the manifest's file list
/// updated; through `<out>.partial`, renamed when the copy validates.
/// Returns the written manifest's bytes.
pub(crate) fn write_copy(
    bundle: &Path,
    mut manifest: Manifest,
    added: &BTreeMap<String, Vec<u8>>,
    drop: impl Fn(&str) -> bool,
    out: &Path,
) -> Result<Vec<u8>, String> {
    if out.exists() {
        return Err(format!("{}: already exists", out.display()));
    }
    manifest
        .files
        .retain(|f| !added.contains_key(&f.path) && !drop(&f.path));
    for (path, bytes) in added {
        manifest.files.push(FileEntry {
            path: path.clone(),
            sha256: hex(&Sha256::digest(bytes)),
            size: bytes.len() as u64,
        });
    }
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
        let bytes = match added.get(&f.path) {
            Some(b) => b.clone(),
            None => read(&bundle.join(&f.path))?,
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
    Ok(manifest_json)
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
