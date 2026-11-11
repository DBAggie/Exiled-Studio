//! The unknown-content mutation set (Phase 2 plan, steps 5 and 8; gate 2):
//! content the adapter does not know, injected into a PoB build, must
//! survive XML to JSON to XML.
//!
//! **Targets:** the first element of each distinct element path
//! (`PathOfBuilding/Skills/SkillSet/Skill/Gem`), in document order, so
//! every section and every kind of element is mutated once, however
//! large the build.
//!
//! **Mutations** ([`Kind`]), each on its own copy of the document:
//! an unknown attribute; an unknown element as the first and as the last
//! child (on the root, an unknown section); a comment before the element
//! (on the root, before the root element); a processing instruction after
//! it (on the root, after the root element); an unknown element right
//! after it, so between two typed siblings where there is a next one (not
//! on the root); and on `Items/Item`, a second, earlier item text with an
//! element between the two texts (PoB reads the last one).
//!
//! The values carry characters that need escaping (`< & > "`, tab,
//! newline) so the writer's escaping is tested too.
//!
//! **Writer:** the document is written back from its parsed tree, with
//! every text node as read (whitespace too) and the mutation added. The
//! unmutated copy must be [`xml_equal`] to the source; [`mutations`]
//! refuses the document otherwise.

use std::collections::BTreeSet;

use loom_serialization::legacy::pob_xml::xml_equal;
use roxmltree::{Document, Node, NodeId, NodeType};

/// Name of every injected element.
pub const ELEMENT: &str = "LoomMutation";
/// Name of the injected attribute.
pub const ATTRIBUTE: &str = "loomMutation";
/// The injected attribute's value.
const VALUE: &str = "a < b & \"c\" 'd'\tline\nend";
/// The injected elements' text.
const TEXT: &str = "text < & > ]]> end";
/// The earlier item text of [`Kind::ExtraItemText`].
const ITEM_TEXT: &str = "\nRarity: NORMAL\nLoom Mutation Earlier Text\n";

/// One kind of mutation (module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// An unknown attribute on the element.
    Attribute,
    /// An unknown element as the first child.
    FirstChild,
    /// An unknown element as the last child.
    LastChild,
    /// A comment right before the element.
    CommentBefore,
    /// A processing instruction right after the element.
    InstructionAfter,
    /// An unknown element right after the element (not on the root).
    ElementAfter,
    /// A second, earlier text in an `Items/Item`.
    ExtraItemText,
}

impl Kind {
    /// Every kind, in report order.
    pub const ALL: [Self; 7] = [
        Self::Attribute,
        Self::FirstChild,
        Self::LastChild,
        Self::CommentBefore,
        Self::InstructionAfter,
        Self::ElementAfter,
        Self::ExtraItemText,
    ];

    /// Stable name (report keys).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Attribute => "attribute",
            Self::FirstChild => "first-child",
            Self::LastChild => "last-child",
            Self::CommentBefore => "comment-before",
            Self::InstructionAfter => "instruction-after",
            Self::ElementAfter => "element-after",
            Self::ExtraItemText => "extra-item-text",
        }
    }

    /// Whether the kind applies to `node` at `path`.
    fn applies(self, node: Node<'_, '_>, path: &str) -> bool {
        let is_root = node.parent().is_some_and(|p| p.is_root());
        match self {
            Self::Attribute => node.attribute(ATTRIBUTE).is_none(),
            Self::FirstChild | Self::LastChild | Self::CommentBefore | Self::InstructionAfter => {
                true
            }
            Self::ElementAfter => !is_root,
            Self::ExtraItemText => path.ends_with("/Items/Item"),
        }
    }
}

/// One mutated copy of a document.
#[derive(Debug)]
pub struct Mutation {
    /// The kind.
    pub kind: Kind,
    /// The target element's path (element names joined with `/`).
    pub path: String,
    /// The mutated document.
    pub xml: String,
}

/// Why no mutation set was made.
#[derive(Debug)]
pub enum Error {
    /// The document is not XML.
    Xml(roxmltree::Error),
    /// The writer's unmutated copy is not equal to the document (a harness
    /// fault, never the adapter's).
    Writer,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Xml(e) => write!(f, "not XML: {e}"),
            Self::Writer => f.write_str("the mutation writer does not reproduce the document"),
        }
    }
}

/// The mutation set of a document (module docs), in document order and
/// [`Kind::ALL`] order per target.
///
/// # Errors
/// [`Error`] when the document is not XML or the writer cannot reproduce
/// it.
pub fn mutations(xml: &str) -> Result<Vec<Mutation>, Error> {
    let doc = Document::parse(xml).map_err(Error::Xml)?;
    let plain = write(&doc, None);
    if !xml_equal(&plain, xml).map_err(Error::Xml)? {
        return Err(Error::Writer);
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for node in doc.descendants().filter(Node::is_element) {
        let path = path(node);
        if !seen.insert(path.clone()) {
            continue;
        }
        for kind in Kind::ALL {
            if kind.applies(node, &path) {
                out.push(Mutation {
                    kind,
                    path: path.clone(),
                    xml: write(&doc, Some((node.id(), kind))),
                });
            }
        }
    }
    Ok(out)
}

/// Element names from the root element down to `node`, joined with `/`.
fn path(node: Node<'_, '_>) -> String {
    let mut names: Vec<&str> = node
        .ancestors()
        .filter(Node::is_element)
        .map(|n| n.tag_name().name())
        .collect();
    names.reverse();
    names.join("/")
}

type Target = Option<(NodeId, Kind)>;

/// Writes the document with the mutation `target`, if any.
fn write(doc: &Document<'_>, target: Target) -> String {
    let mut out = String::new();
    for child in doc.root().children() {
        around(child, target, &mut out);
    }
    out
}

/// A node with what a mutation puts before and after it.
fn around(node: Node<'_, '_>, target: Target, out: &mut String) {
    let kind = target.and_then(|(id, kind)| (id == node.id()).then_some(kind));
    if kind == Some(Kind::CommentBefore) {
        out.push_str("<!-- loom mutation: comment before -->");
    }
    node_text(node, kind, target, out);
    match kind {
        Some(Kind::InstructionAfter) => out.push_str("<?loom-mutation after=\"1\"?>"),
        Some(Kind::ElementAfter) => element(Kind::ElementAfter, out),
        _ => {}
    }
}

fn node_text(node: Node<'_, '_>, kind: Option<Kind>, target: Target, out: &mut String) {
    match node.node_type() {
        NodeType::Root => {
            for child in node.children() {
                around(child, target, out);
            }
        }
        NodeType::Element => {
            let name = node.tag_name().name();
            out.push('<');
            out.push_str(name);
            for a in node.attributes() {
                attribute(a.name(), a.value(), out);
            }
            if kind == Some(Kind::Attribute) {
                attribute(ATTRIBUTE, VALUE, out);
            }
            let adds = matches!(
                kind,
                Some(Kind::FirstChild | Kind::LastChild | Kind::ExtraItemText)
            );
            if !node.has_children() && !adds {
                out.push_str("/>");
                return;
            }
            out.push('>');
            match kind {
                Some(Kind::ExtraItemText) => {
                    escape_text(ITEM_TEXT, out);
                    element(Kind::ExtraItemText, out);
                }
                Some(Kind::FirstChild) => element(Kind::FirstChild, out),
                _ => {}
            }
            for child in node.children() {
                around(child, target, out);
            }
            if kind == Some(Kind::LastChild) {
                element(Kind::LastChild, out);
            }
            out.push_str("</");
            out.push_str(name);
            out.push('>');
        }
        NodeType::Text => escape_text(node.text().unwrap_or(""), out),
        NodeType::Comment => {
            out.push_str("<!--");
            out.push_str(node.text().unwrap_or(""));
            out.push_str("-->");
        }
        NodeType::PI => {
            if let Some(pi) = node.pi() {
                out.push_str("<?");
                out.push_str(pi.target);
                if let Some(value) = pi.value {
                    out.push(' ');
                    out.push_str(value);
                }
                out.push_str("?>");
            }
        }
    }
}

/// An injected element, with an attribute, a child, and text.
fn element(kind: Kind, out: &mut String) {
    out.push('<');
    out.push_str(ELEMENT);
    attribute("kind", kind.name(), out);
    attribute(ATTRIBUTE, VALUE, out);
    out.push_str("><Inner/>");
    escape_text(TEXT, out);
    out.push_str("</");
    out.push_str(ELEMENT);
    out.push('>');
}

fn attribute(name: &str, value: &str, out: &mut String) {
    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn escape_text(text: &str, out: &mut String) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}
