//! PoB build XML (Phase 2 plan, step 5; decisions 1, 2, 12): import into
//! the domain [`Build`] and semantic export back to XML.
//!
//! **Typed** (what the calculations need; P2-5a): the `<Build>` attributes
//! (class, ascendancy, level, main socket group; bandit and pantheon as
//! PoE1 choices), `<Tree>` with its `<Spec>`s (tree version, nodes,
//! mastery effects, jewel sockets), `<Skills>` with skill sets, socket
//! groups (`<Skill>`), and gems, and `<Config>` with config sets and
//! inputs. `<Items>` is typed in P2-5b; until then it is kept whole.
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
mod read;
mod write;

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests;

pub use equal::{NORMALIZATIONS, xml_equal};
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

/// Exports a build as a PoB build document.
#[must_use]
pub fn export(build: &Build) -> String {
    write::build(build)
}
