//! Content the model does not type, kept as read and in place (ADR-003,
//! ADR-005; Phase 2 plan, decision 1). An exporter writes it back
//! unchanged.
//!
//! [`Ordered`] is a list of typed entries with opaque content between
//! them, so an unknown element between two gems stays between those gems.

/// One attribute, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// Name.
    pub name: String,
    /// Value.
    pub value: String,
}

impl Attribute {
    /// An attribute.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Attributes a typed node does not type, in source order.
pub type Attributes = Vec<Attribute>;

/// A node of untyped content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpaqueNode {
    /// An element: name, attributes in order, children in order.
    Element {
        /// Name.
        name: String,
        /// Attributes, in source order.
        attributes: Attributes,
        /// Children, in source order.
        children: Vec<OpaqueNode>,
    },
    /// Text.
    Text(String),
    /// A comment.
    Comment(String),
    /// A processing instruction.
    ProcessingInstruction {
        /// Target.
        target: String,
        /// Data, if any.
        data: Option<String>,
    },
}

impl OpaqueNode {
    /// An element with no attributes and no children.
    #[must_use]
    pub fn element(name: impl Into<String>) -> Self {
        Self::Element {
            name: name.into(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }
}

/// One entry of an [`Ordered`] list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry<T> {
    /// A typed entry.
    Typed(T),
    /// Untyped content at this position.
    Opaque(OpaqueNode),
}

/// Typed entries with opaque content kept in place between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ordered<T> {
    entries: Vec<Entry<T>>,
}

impl<T> Default for Ordered<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> Ordered<T> {
    /// An empty list.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Appends a typed entry.
    pub fn push(&mut self, value: T) {
        self.entries.push(Entry::Typed(value));
    }

    /// Appends opaque content.
    pub fn push_opaque(&mut self, node: OpaqueNode) {
        self.entries.push(Entry::Opaque(node));
    }

    /// Every entry, in order.
    #[must_use]
    pub fn entries(&self) -> &[Entry<T>] {
        &self.entries
    }

    /// The typed entries, in order.
    pub fn typed(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().filter_map(|e| match e {
            Entry::Typed(t) => Some(t),
            Entry::Opaque(_) => None,
        })
    }

    /// The typed entries, in order, mutably.
    pub fn typed_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.entries.iter_mut().filter_map(|e| match e {
            Entry::Typed(t) => Some(t),
            Entry::Opaque(_) => None,
        })
    }

    /// The opaque entries, in order.
    pub fn opaque(&self) -> impl Iterator<Item = &OpaqueNode> {
        self.entries.iter().filter_map(|e| match e {
            Entry::Typed(_) => None,
            Entry::Opaque(n) => Some(n),
        })
    }
}

impl<T> FromIterator<Entry<T>> for Ordered<T> {
    fn from_iter<I: IntoIterator<Item = Entry<T>>>(iter: I) -> Self {
        Self {
            entries: iter.into_iter().collect(),
        }
    }
}
