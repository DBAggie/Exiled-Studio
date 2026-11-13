//! PoB build XML (Phase 2 plan, step 5; decisions 1, 2, 12): import into
//! the domain [`Build`] and semantic export back to XML.
//!
//! **Typed** (what the calculations need; P2-5a): the `<Build>` attributes
//! (class, ascendancy, level, main socket group; bandit and pantheon as
//! PoE1 choices), `<Tree>` with its `<Spec>`s (tree version, nodes,
//! mastery effects, jewel sockets), `<Skills>` with skill sets, socket
//! groups (`<Skill>`), and gems, and `<Config>` with config sets and
//! inputs; `<Items>` with items, item sets, and slots (P2-5b; module
//! `items`).
//!
//! **Untyped, kept in place:** every other element, attribute, comment,
//! and processing instruction, including content before and after the
//! root element ([`loom_domain::OpaqueNode`]). An element whose typed
//! fields do not read (a number that is not one, a boolean that is not
//! `true` or `false`, a required attribute missing, an element where an
//! attribute-only one is expected) is kept whole instead (plan,
//! "Fallback"), so nothing is lost. Typed values keep their source text
//! and the export writes it back while the value is unchanged.
//!
//! Whitespace-only text between elements is layout and is not kept; the
//! export indents with tabs. Two documents compare with [`xml_equal`].
//! PoB's format is PoE1's (`<PathOfBuilding>`); PoE2 build import is
//! backlog (ADR-005).

mod equal;
mod items;
mod read;
mod write;

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;

pub use equal::{NORMALIZATIONS, xml_equal};
pub use write::item_text;
use loom_domain::Build;

/// Why a document did not import.
#[derive(Debug)]
pub enum ImportError {
    /// Not well-formed XML.
    Xml(roxmltree::Error),
    /// The root element is not `PathOfBuilding`.
    Root(String),
}

impl ImportError {
    /// Stable code (ADR-014).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Xml(_) => "IMPORT_XML",
            Self::Root(_) => "IMPORT_ROOT",
        }
    }
}

impl core::fmt::Display for ImportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Xml(e) => write!(f, "{}: {e}", self.code()),
            Self::Root(name) => write!(f, "{}: root element {name:?}", self.code()),
        }
    }
}

impl core::error::Error for ImportError {}

/// Imports a PoB build document.
///
/// # Errors
/// [`ImportError`] when the text is not XML or not a PoB build.
pub fn import(xml: &str) -> Result<Build, ImportError> {
    let doc = roxmltree::Document::parse(xml).map_err(ImportError::Xml)?;
    read::build(&doc)
}

/// Typed elements kept whole because their fields did not read (the
/// plan's fallback), by path (`Skills/SkillSet/Skill/Gem`), in document
/// order: the import's quarantine list for typed content (ADR-015: kept
/// and shown, never silently ignored). Code `IMPORT_KEPT_WHOLE`.
#[must_use]
pub fn kept_whole(build: &Build) -> Vec<String> {
    use loom_domain::{BuildPart, Entry, OpaqueNode};
    fn named<'a, T>(
        list: &'a loom_domain::Ordered<T>,
        name: &str,
    ) -> impl Iterator<Item = ()> + 'a {
        let name = name.to_owned();
        list.opaque().filter_map(move |o| match o {
            OpaqueNode::Element { name: n, .. } if *n == name => Some(()),
            _ => None,
        })
    }
    let mut out = Vec::new();
    for e in build.layout.entries() {
        match e {
            Entry::Opaque(OpaqueNode::Element { name, .. })
                if ["Build", "Tree", "Skills", "Items", "Config"].contains(&name.as_str()) =>
            {
                out.push(name.clone());
            }
            Entry::Typed(BuildPart::Passives) => {
                out.extend(named(&build.passives.specs, "Spec").map(|()| "Tree/Spec".to_owned()));
                for spec in build.passives.specs.typed() {
                    out.extend(
                        named(&spec.jewel_sockets, "Socket")
                            .map(|()| "Tree/Spec/Sockets/Socket".to_owned()),
                    );
                }
            }
            Entry::Typed(BuildPart::Skills) => {
                out.extend(
                    named(&build.skills.sets, "SkillSet").map(|()| "Skills/SkillSet".to_owned()),
                );
                for set in build.skills.sets.typed() {
                    out.extend(
                        named(&set.groups, "Skill").map(|()| "Skills/SkillSet/Skill".to_owned()),
                    );
                    for group in set.groups.typed() {
                        out.extend(
                            named(&group.gems, "Gem")
                                .map(|()| "Skills/SkillSet/Skill/Gem".to_owned()),
                        );
                    }
                }
            }
            Entry::Typed(BuildPart::Items) => {
                for o in build.items.entries.opaque() {
                    if let OpaqueNode::Element { name, .. } = o
                        && (name == "Item" || name == "ItemSet")
                    {
                        out.push(format!("Items/{name}"));
                    }
                }
                for set in build.items.sets() {
                    out.extend(named(&set.slots, "Slot").map(|()| "Items/ItemSet/Slot".to_owned()));
                }
            }
            Entry::Typed(BuildPart::Config) => {
                out.extend(
                    named(&build.config.sets, "ConfigSet").map(|()| "Config/ConfigSet".to_owned()),
                );
                // PoB's layout before config sets has no set element.
                let set_path = match build.config.layout {
                    loom_domain::ConfigLayout::Sets => "Config/ConfigSet",
                    loom_domain::ConfigLayout::Flat => "Config",
                };
                for set in build.config.sets.typed() {
                    for name in ["Input", "Placeholder", "CustomModifierBlock"] {
                        out.extend(named(&set.inputs, name).map(|()| format!("{set_path}/{name}")));
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Exports a build as a PoB build document.
#[must_use]
pub fn export(build: &Build) -> String {
    write::build(build)
}
