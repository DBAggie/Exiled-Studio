//! XML equality over parsed trees (Phase 2 plan, decision 12): two
//! documents are equal when their parsed trees are, under the closed list
//! of normalizations in [`NORMALIZATIONS`]. Parsing makes the XML
//! declaration, quoting, escaping, character references (`&#10;`
//! against a literal newline), and line endings (the parser reads CRLF as
//! LF) equal by construction.

use std::collections::BTreeMap;

use roxmltree::{Document, Node, NodeType};

/// Every normalization besides parsing, each with its reason. Each has a
/// test (`tests.rs`).
pub const NORMALIZATIONS: [(&str, &str); 2] = [
    (
        "attribute-order",
        "attributes compare as maps: XML gives their order no meaning, and the export writes typed attributes first",
    ),
    (
        "whitespace-between-elements",
        "whitespace-only text in an element with no other text is layout: the export indents its own way (in mixed content every text is compared)",
    ),
];

fn significant(n: &Node<'_, '_>) -> bool {
    match n.node_type() {
        NodeType::Text => n.text().is_some_and(|t| !t.trim().is_empty()),
        _ => true,
    }
}

fn node_equal(a: Node<'_, '_>, b: Node<'_, '_>) -> bool {
    if a.node_type() != b.node_type() {
        return false;
    }
    match a.node_type() {
        NodeType::Element => {
            let attrs = |n: Node<'_, '_>| -> BTreeMap<String, String> {
                n.attributes()
                    .map(|x| (x.name().to_owned(), x.value().to_owned()))
                    .collect()
            };
            a.tag_name().name() == b.tag_name().name()
                && attrs(a) == attrs(b)
                && children_equal(a, b)
        }
        NodeType::Text | NodeType::Comment => a.text() == b.text(),
        NodeType::PI => a.pi() == b.pi(),
        NodeType::Root => children_equal(a, b),
    }
}

/// Children compared: all of them in mixed content, else without the
/// whitespace-only text.
fn children_equal(a: Node<'_, '_>, b: Node<'_, '_>) -> bool {
    let mixed = |n: Node<'_, '_>| {
        n.children()
            .any(|c| c.is_text() && c.text().is_some_and(|t| !t.trim().is_empty()))
    };
    let all = mixed(a) || mixed(b);
    let ca: Vec<Node<'_, '_>> = a.children().filter(|c| all || significant(c)).collect();
    let cb: Vec<Node<'_, '_>> = b.children().filter(|c| all || significant(c)).collect();
    ca.len() == cb.len() && ca.into_iter().zip(cb).all(|(x, y)| node_equal(x, y))
}

/// Whether two XML documents are equal (module docs).
///
/// # Errors
/// The parse error of a document that is not XML.
pub fn xml_equal(a: &str, b: &str) -> Result<bool, roxmltree::Error> {
    let (da, db) = (Document::parse(a)?, Document::parse(b)?);
    Ok(node_equal(da.root(), db.root()))
}
