//! Loaders from bytes: the extract manifest and one stat-description JSON
//! file (written by `tools/extractor/src/statdesc.rs`). The host reads the
//! files; this module never touches the filesystem (ADR-016).
//!
//! The JSON is untrusted: unknown fields are errors, and everything the
//! extractor guarantees (condition and handler counts, hands, placeholder
//! indexes) is checked again. Problems confined to one variant do not fail
//! the file (ADR-015): a format text that does not parse keeps its
//! [`TemplateError`], an unknown flag is kept in `unknown_flags`, and a
//! handler with a bad argument becomes [`Handler::Malformed`]; the renderer
//! reports each.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::template::{Template, TemplateError};
use super::{Condition, Description, Handler, StatDescFile, Variant};

/// Manifest formats this loader reads. Format 4 (Spike B PR B4x) adds the
/// `item_types` section; its stat-description section is the same as 3.
pub const SUPPORTED_MANIFEST_FORMATS: [u64; 2] = [3, 4];

/// Why a manifest or file did not load.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// Not valid JSON of the expected shape.
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// A manifest format this loader does not know.
    #[error("manifest format {0} is not supported (supported: {SUPPORTED_MANIFEST_FORMATS:?})")]
    UnsupportedFormat(u64),
    /// The data breaks an invariant of the extract format.
    #[error("{context}: {message}")]
    Invalid {
        /// Where (description line, variant line).
        context: String,
        /// What is wrong.
        message: String,
    },
}

impl LoadError {
    /// Stable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Json(_) => "DATA_STATDESC_JSON",
            Self::UnsupportedFormat(_) => "DATA_STATDESC_FORMAT",
            Self::Invalid { .. } => "DATA_STATDESC_INVALID",
        }
    }

    fn invalid(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid {
            context: context.into(),
            message: message.into(),
        }
    }
}

/// The parts of an extract manifest the stat-description loader needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatDescManifest {
    /// Manifest format.
    pub format: u64,
    /// `poe1` or `poe2`.
    pub game: String,
    /// Steam build of the install.
    pub steam_build_id: Option<String>,
    /// Every stat-description file by index path.
    pub files: BTreeMap<String, ManifestFile>,
    /// Handler use counts in English variants, by name.
    pub handlers: BTreeMap<String, usize>,
}

/// One stat-description file in the manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    /// `descriptions` or `filters`.
    pub kind: String,
    /// `ok`, `failed`, or `skipped`.
    pub status: String,
    /// Output path inside the extract directory, for parsed files.
    pub output: Option<String>,
    /// Description blocks parsed.
    pub descriptions: usize,
    /// English variants parsed.
    pub english_variants: usize,
}

#[derive(Deserialize)]
struct RawManifest {
    format: u64,
    game: String,
    #[serde(default)]
    steam_build_id: Option<serde_json::Value>,
    stat_descriptions: Option<RawStatDescSection>,
}

#[derive(Deserialize)]
struct RawStatDescSection {
    files: BTreeMap<String, RawManifestFile>,
    handlers: BTreeMap<String, usize>,
}

#[derive(Deserialize)]
struct RawManifestFile {
    kind: String,
    status: String,
    output: Option<String>,
    descriptions: usize,
    english_variants: usize,
}

impl StatDescManifest {
    /// Reads the stat-description part of an extract `manifest.json`.
    ///
    /// # Errors
    /// [`LoadError::UnsupportedFormat`] for an unknown format (checked
    /// first), [`LoadError::Json`] for bad JSON, [`LoadError::Invalid`] if
    /// the manifest has no stat-description section.
    pub fn from_json(bytes: &[u8]) -> Result<Self, LoadError> {
        #[derive(Deserialize)]
        struct Format {
            format: u64,
        }
        let Format { format } = serde_json::from_slice(bytes)?;
        if !SUPPORTED_MANIFEST_FORMATS.contains(&format) {
            return Err(LoadError::UnsupportedFormat(format));
        }
        let raw: RawManifest = serde_json::from_slice(bytes)?;
        let section = raw
            .stat_descriptions
            .ok_or_else(|| LoadError::invalid("manifest", "no stat_descriptions section"))?;
        let steam_build_id = raw.steam_build_id.and_then(|v| match v {
            serde_json::Value::String(s) => Some(s),
            serde_json::Value::Number(n) => Some(n.to_string()),
            _ => None,
        });
        Ok(Self {
            format: raw.format,
            game: raw.game,
            steam_build_id,
            files: section
                .files
                .into_iter()
                .map(|(path, f)| {
                    (
                        path,
                        ManifestFile {
                            kind: f.kind,
                            status: f.status,
                            output: f.output,
                            descriptions: f.descriptions,
                            english_variants: f.english_variants,
                        },
                    )
                })
                .collect(),
            handlers: section.handlers,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    descriptions: Vec<RawDescription>,
    #[expect(dead_code, reason = "read to keep the format strict; not used")]
    file_flags: Vec<String>,
    includes: Vec<String>,
    #[expect(dead_code, reason = "read to keep the format strict; not used")]
    languages: serde_json::Value,
    no_description: Vec<String>,
    source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDescription {
    handed: bool,
    line: usize,
    name: Option<String>,
    off_hand_stats: Option<Vec<String>>,
    stats: Vec<String>,
    variants: Vec<RawVariant>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawVariant {
    conditions: Vec<RawCondition>,
    flags: Vec<String>,
    handlers: Vec<RawHandler>,
    line: usize,
    placeholders: Vec<RawPlaceholder>,
    text: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum RawCondition {
    Range { max: Option<i64>, min: Option<i64> },
    Not { value: i64 },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHandler {
    name: String,
    #[serde(default)]
    reminder: Option<String>,
    #[serde(default)]
    stat: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlaceholder {
    format: String,
    index: Option<usize>,
}

impl StatDescFile {
    /// Loads one `stat_descriptions/**/*.json` file of an extract.
    ///
    /// # Errors
    /// [`LoadError::Json`] for bad JSON or unknown fields,
    /// [`LoadError::Invalid`] when a description breaks an invariant.
    pub fn from_json(bytes: &[u8]) -> Result<Self, LoadError> {
        let raw: RawFile = serde_json::from_slice(bytes)?;
        let descriptions = raw
            .descriptions
            .into_iter()
            .map(|d| description(&raw.source, d))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            source: raw.source,
            includes: raw.includes,
            descriptions,
            no_description: raw.no_description,
        })
    }
}

fn description(source: &str, raw: RawDescription) -> Result<Description, LoadError> {
    let context = format!("{source}: description at line {}", raw.line);
    let stats = raw.stats.len();
    if stats == 0 {
        return Err(LoadError::invalid(context, "no stats"));
    }
    match (&raw.off_hand_stats, raw.handed) {
        (Some(off), true) if off.len() == stats => {}
        (None, false) => {}
        _ => {
            return Err(LoadError::invalid(
                context,
                "off-hand stats do not match the handed flag",
            ));
        }
    }
    let variants = raw
        .variants
        .into_iter()
        .map(|v| variant(&context, stats, v))
        .collect::<Result<_, _>>()?;
    Ok(Description {
        name: raw.name,
        line: raw.line,
        stats: raw.stats,
        off_hand_stats: raw.off_hand_stats,
        variants,
    })
}

fn variant(context: &str, stats: usize, raw: RawVariant) -> Result<Variant, LoadError> {
    let context = format!("{context}, variant at line {}", raw.line);
    if raw.conditions.len() != stats {
        return Err(LoadError::invalid(
            context,
            format!("{} conditions for {stats} stats", raw.conditions.len()),
        ));
    }
    let mut gem_quality = false;
    let mut table_only = false;
    let mut unknown_flags = Vec::new();
    for flag in raw.flags {
        match flag.as_str() {
            "gem_quality" => gem_quality = true,
            "table_only" => table_only = true,
            _ => unknown_flags.push(flag),
        }
    }
    let handlers = raw
        .handlers
        .iter()
        .map(|h| {
            Handler::classify(&h.name, h.stat, h.reminder.as_deref(), stats).unwrap_or_else(|| {
                Handler::Malformed {
                    name: h.name.clone(),
                }
            })
        })
        .collect();
    let template = Template::parse(&raw.text, stats).and_then(|t| {
        let ours = t.placeholders();
        let theirs = raw
            .placeholders
            .iter()
            .map(|p| (p.index.unwrap_or(0), p.format.as_str()));
        if ours.eq(theirs) {
            Ok(t)
        } else {
            Err(TemplateError::PlaceholderMismatch)
        }
    });
    Ok(Variant {
        conditions: raw
            .conditions
            .into_iter()
            .map(|c| match c {
                RawCondition::Range { max, min } => Condition::Range { min, max },
                RawCondition::Not { value } => Condition::Not(value),
            })
            .collect(),
        gem_quality,
        table_only,
        unknown_flags,
        handlers,
        line: raw.line,
        text: raw.text,
        template,
    })
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;
    use crate::stat_desc::{Hand, LookupHandler};

    fn file(descriptions: &str) -> String {
        format!(
            r#"{{"descriptions": [{descriptions}], "file_flags": [], "includes": ["Metadata/A.txt"],
               "languages": {{}}, "no_description": ["hidden"], "source": "metadata/b.txt"}}"#
        )
    }

    const ONE: &str = r#"{"handed":false,"line":3,"name":null,"off_hand_stats":null,"stats":["s"],
        "variants":[{"conditions":[{"kind":"range","max":null,"min":1}],"flags":["gem_quality"],
        "handlers":[{"name":"negate","stat":1},{"name":"passive_hash","stat":1},{"name":"reminderstring","reminder":"R"}],
        "line":4,"placeholders":[{"format":"+d","index":0}],"text":"{0:+d}% [A|x]"},
        {"conditions":[{"kind":"not","value":0}],"flags":[],"handlers":[],"line":5,
        "placeholders":[{"format":"","index":null}],"text":"{} y"}]}"#;

    #[test]
    fn from_json_should_read_every_field() {
        let f = StatDescFile::from_json(file(ONE).as_bytes()).unwrap();
        assert_eq!(f.source, "metadata/b.txt");
        assert_eq!(f.includes, vec!["Metadata/A.txt"]);
        assert_eq!(f.no_description, vec!["hidden"]);
        let d = &f.descriptions[0];
        assert_eq!(d.stats_of(Hand::Off), None);
        let v = &d.variants[0];
        assert!(v.gem_quality && !v.table_only);
        assert_eq!(
            v.conditions,
            vec![Condition::Range {
                min: Some(1),
                max: None
            }]
        );
        assert_eq!(
            v.handlers[1],
            Handler::Lookup {
                handler: LookupHandler::PassiveHash,
                stat: 0
            }
        );
        assert!(v.template.is_ok());
        assert_eq!(d.variants[1].conditions, vec![Condition::Not(0)]);
        assert!(d.variants[1].template.is_ok());
    }

    #[test]
    fn from_json_should_keep_a_placeholder_mismatch_on_the_variant() {
        let text = file(ONE).replace(r#"{"format":"+d","index":0}"#, r#"{"format":"","index":0}"#);
        let f = StatDescFile::from_json(text.as_bytes()).unwrap();
        assert_eq!(
            f.descriptions[0].variants[0].template,
            Err(TemplateError::PlaceholderMismatch)
        );
    }

    #[test]
    fn from_json_should_reject_unknown_fields() {
        let text = file(ONE).replace(r#""line":4,"#, r#""line":4,"extra":1,"#);
        assert!(matches!(
            StatDescFile::from_json(text.as_bytes()),
            Err(LoadError::Json(_))
        ));
    }

    #[test]
    fn from_json_should_reject_a_wrong_condition_count() {
        let text = file(ONE).replace(
            r#"[{"kind":"not","value":0}]"#,
            r#"[{"kind":"not","value":0},{"kind":"not","value":1}]"#,
        );
        let err = StatDescFile::from_json(text.as_bytes()).unwrap_err();
        assert_eq!(err.code(), "DATA_STATDESC_INVALID");
    }

    #[test]
    fn from_json_should_keep_a_handler_with_a_bad_stat_as_malformed() {
        let text = file(ONE).replace(r#""name":"negate","stat":1"#, r#""name":"negate","stat":2"#);
        let f = StatDescFile::from_json(text.as_bytes()).unwrap();
        assert_eq!(
            f.descriptions[0].variants[0].handlers[0],
            Handler::Malformed {
                name: "negate".to_owned()
            }
        );
    }

    #[test]
    fn from_json_should_keep_an_unknown_flag_on_the_variant() {
        let text = file(ONE).replace(r#""flags":["gem_quality"]"#, r#""flags":["new_flag"]"#);
        let f = StatDescFile::from_json(text.as_bytes()).unwrap();
        assert_eq!(
            f.descriptions[0].variants[0].unknown_flags,
            vec!["new_flag"]
        );
    }

    #[test]
    fn from_json_should_reject_a_handed_description_without_off_hand_stats() {
        let text = file(ONE).replace(r#""handed":false"#, r#""handed":true"#);
        assert!(matches!(
            StatDescFile::from_json(text.as_bytes()),
            Err(LoadError::Invalid { .. })
        ));
    }

    #[test]
    fn from_json_should_not_panic_on_any_prefix() {
        let text = file(ONE);
        for (i, _) in text.char_indices() {
            let _ = StatDescFile::from_json(text.get(..i).unwrap().as_bytes());
        }
    }

    #[test]
    fn manifest_should_refuse_an_unknown_format_before_reading_more() {
        let err = StatDescManifest::from_json(br#"{"format": 2}"#).unwrap_err();
        assert_eq!(err.code(), "DATA_STATDESC_FORMAT");
    }

    #[test]
    fn manifest_should_read_files_and_handler_counts() {
        let m = StatDescManifest::from_json(
            br#"{"format": 3, "game": "poe1", "steam_build_id": "26530219", "tables": {},
                "stat_descriptions": {"dir": "x", "files": {"metadata/a.txt": {"kind": "descriptions",
                "status": "ok", "output": "stat_descriptions/a.txt.json", "descriptions": 2,
                "english_variants": 3, "includes": []}}, "handlers": {"negate": 4}}}"#,
        )
        .unwrap();
        assert_eq!(m.steam_build_id.as_deref(), Some("26530219"));
        assert_eq!(m.files["metadata/a.txt"].english_variants, 3);
        assert_eq!(m.handlers["negate"], 4);
    }
}
