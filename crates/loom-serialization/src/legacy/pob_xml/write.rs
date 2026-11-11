//! The domain Build to PoB XML (module docs): typed attributes first, then
//! the attributes kept as read, in order; untyped content where it was.

use loom_domain::{
    Attributes, Build, BuildPart, Character, Config, ConfigValue, Entry, GameSpecific, Gem,
    OpaqueNode, Ordered, PassiveSpec, Passives, Skills, Sourced, SpecPart,
};

/// An XML writer that indents with tabs, as PoB does.
#[derive(Default)]
struct Writer {
    out: String,
    /// Inside mixed content: no layout is written (the kept text carries
    /// its own whitespace).
    inline: usize,
}

fn escape(s: &str, attribute: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            '\n' if attribute => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' if attribute => out.push_str("&#9;"),
            c => out.push(c),
        }
    }
    out
}

/// An attribute list: typed ones (when set) and the kept ones.
type Attrs = Vec<(String, String)>;

fn put(attrs: &mut Attrs, name: &str, value: Option<&str>) {
    if let Some(v) = value {
        attrs.push((name.to_owned(), v.to_owned()));
    }
}

fn put_number<T: ToString>(attrs: &mut Attrs, name: &str, value: Option<&Sourced<T>>) {
    if let Some(v) = value {
        let text = v
            .source()
            .map_or_else(|| v.value().to_string(), str::to_owned);
        attrs.push((name.to_owned(), text));
    }
}

fn kept(attrs: &mut Attrs, rest: &Attributes) {
    attrs.extend(rest.iter().map(|a| (a.name.clone(), a.value.clone())));
}

impl Writer {
    /// A new line and indentation, only after a tag: text keeps its own
    /// whitespace (PoB item text is mixed with `<ModRange>` elements).
    fn indent(&mut self, depth: usize) {
        if self.inline > 0 || !self.out.ends_with('>') {
            return;
        }
        self.out.push('\n');
        for _ in 0..depth {
            self.out.push('\t');
        }
    }

    fn open(&mut self, depth: usize, name: &str, attrs: &Attrs) {
        self.indent(depth);
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            self.out.push(' ');
            self.out.push_str(k);
            self.out.push_str("=\"");
            self.out.push_str(&escape(v, true));
            self.out.push('"');
        }
    }

    /// An element whose children `body` writes; self-closing when it
    /// writes none.
    fn element(&mut self, depth: usize, name: &str, attrs: &Attrs, body: impl FnOnce(&mut Self)) {
        self.open(depth, name, attrs);
        self.out.push('>');
        let start = self.out.len();
        body(self);
        if self.out.len() == start {
            self.out.pop();
            self.out.push_str("/>");
        } else {
            self.indent(depth);
            self.out.push_str("</");
            self.out.push_str(name);
            self.out.push('>');
        }
    }

    fn opaque(&mut self, depth: usize, node: &OpaqueNode) {
        match node {
            OpaqueNode::Element {
                name,
                attributes,
                children,
            } => {
                let mut a = Attrs::new();
                kept(&mut a, attributes);
                let mixed = children.iter().any(|c| matches!(c, OpaqueNode::Text(_)));
                self.open(depth, name, &a);
                self.out.push('>');
                self.inline += usize::from(mixed);
                let start = self.out.len();
                for c in children {
                    self.opaque(depth + 1, c);
                }
                if self.out.len() == start {
                    self.out.pop();
                    self.out.push_str("/>");
                } else {
                    self.indent(depth);
                    self.out.push_str("</");
                    self.out.push_str(name);
                    self.out.push('>');
                }
                self.inline -= usize::from(mixed);
            }
            OpaqueNode::Text(t) => self.out.push_str(&escape(t, false)),
            OpaqueNode::Comment(t) => {
                self.indent(depth);
                self.out.push_str("<!--");
                self.out.push_str(t);
                self.out.push_str("-->");
            }
            OpaqueNode::ProcessingInstruction { target, data } => {
                self.indent(depth);
                self.out.push_str("<?");
                self.out.push_str(target);
                if let Some(d) = data {
                    self.out.push(' ');
                    self.out.push_str(d);
                }
                self.out.push_str("?>");
            }
        }
    }

    fn ordered<T>(&mut self, depth: usize, list: &Ordered<T>, typed: impl Fn(&mut Self, &T)) {
        for e in list.entries() {
            match e {
                Entry::Typed(t) => typed(self, t),
                Entry::Opaque(o) => self.opaque(depth, o),
            }
        }
    }
}

/// Writes the build as PoB XML.
pub(super) fn build(b: &Build) -> String {
    let mut w = Writer::default();
    w.out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    for p in &b.prolog {
        w.opaque(0, p);
    }
    let mut root = Attrs::new();
    kept(&mut root, &b.attributes);
    w.element(0, "PathOfBuilding", &root, |w| {
        w.ordered(1, &b.layout, |w, part| match part {
            BuildPart::Character => character(w, b),
            BuildPart::Passives => passives(w, &b.passives),
            BuildPart::Skills => skills(w, &b.skills),
            BuildPart::Items => {
                let mut a = Attrs::new();
                kept(&mut a, &b.items.attributes);
                w.element(1, "Items", &a, |_| {});
            }
            BuildPart::Config => config(w, &b.config),
        });
    });
    for p in &b.epilog {
        w.opaque(0, p);
    }
    w.out.push('\n');
    w.out
}

fn character(w: &mut Writer, b: &Build) {
    let c: &Character = &b.character;
    let mut a = Attrs::new();
    put(&mut a, "className", c.class.as_ref().map(|x| x.as_str()));
    put(
        &mut a,
        "ascendClassName",
        c.ascendancy.as_ref().map(|x| x.as_str()),
    );
    put_number(&mut a, "level", c.level.as_ref());
    put_number(&mut a, "mainSocketGroup", c.main_socket_group.as_ref());
    if let GameSpecific::Poe1(p) = b.game_specific() {
        put(&mut a, "bandit", p.bandit.as_deref());
        put(&mut a, "pantheonMajorGod", p.pantheon_major.as_deref());
        put(&mut a, "pantheonMinorGod", p.pantheon_minor.as_deref());
    }
    kept(&mut a, &c.attributes);
    w.element(1, "Build", &a, |w| {
        for ch in &c.children {
            w.opaque(2, ch);
        }
    });
}

fn passives(w: &mut Writer, p: &Passives) {
    let mut a = Attrs::new();
    put(
        &mut a,
        "activeSpec",
        p.active.map(|s| s.0.to_string()).as_deref(),
    );
    kept(&mut a, &p.attributes);
    w.element(1, "Tree", &a, |w| w.ordered(2, &p.specs, spec));
}

/// A list attribute: its source text, else the value written fresh; an
/// empty list without source text is not written.
fn list_text<T>(v: &Sourced<Vec<T>>, item: impl Fn(&T) -> String) -> Option<String> {
    match v.source() {
        Some(s) => Some(s.to_owned()),
        None if v.value().is_empty() => None,
        None => Some(v.value().iter().map(item).collect::<Vec<_>>().join(",")),
    }
}

fn spec(w: &mut Writer, s: &PassiveSpec) {
    let mut a = Attrs::new();
    put(&mut a, "title", s.title.as_deref());
    put(
        &mut a,
        "treeVersion",
        s.tree_version.as_ref().map(|t| t.as_str()),
    );
    put(
        &mut a,
        "nodes",
        list_text(&s.nodes, |n| n.0.to_string()).as_deref(),
    );
    put(
        &mut a,
        "masteryEffects",
        list_text(&s.mastery_effects, |m| {
            format!("{{{},{}}}", m.node.0, m.effect)
        })
        .as_deref(),
    );
    kept(&mut a, &s.attributes);
    w.element(2, "Spec", &a, |w| {
        w.ordered(3, &s.layout, |w, part| match part {
            SpecPart::JewelSockets => w.element(3, "Sockets", &Attrs::new(), |w| {
                w.ordered(4, &s.jewel_sockets, |w, j| {
                    let mut a = vec![("nodeId".to_owned(), j.node.0.to_string())];
                    // No item: PoB's `itemId="0"`.
                    let item = j.item.map_or_else(|| "0".to_owned(), |i| i.0.to_string());
                    put(&mut a, "itemId", Some(&item));
                    kept(&mut a, &j.attributes);
                    w.element(4, "Socket", &a, |_| {});
                });
            }),
        });
    });
}

fn bool_text(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

fn skills(w: &mut Writer, s: &Skills) {
    let mut a = Attrs::new();
    put(
        &mut a,
        "activeSkillSet",
        s.active.map(|x| x.0.to_string()).as_deref(),
    );
    kept(&mut a, &s.attributes);
    w.element(1, "Skills", &a, |w| {
        w.ordered(2, &s.sets, |w, set| {
            let mut a = vec![("id".to_owned(), set.id.0.to_string())];
            put(&mut a, "title", set.title.as_deref());
            kept(&mut a, &set.attributes);
            w.element(2, "SkillSet", &a, |w| {
                w.ordered(3, &set.groups, |w, g| {
                    let mut a = Attrs::new();
                    put(&mut a, "label", g.label.as_deref());
                    put(&mut a, "enabled", Some(bool_text(g.enabled)));
                    put(&mut a, "slot", g.slot.as_ref().map(|x| x.as_str()));
                    put_number(&mut a, "mainActiveSkill", g.main_active_skill.as_ref());
                    kept(&mut a, &g.attributes);
                    w.element(3, "Skill", &a, |w| w.ordered(4, &g.gems, gem));
                });
            });
        });
    });
}

fn gem(w: &mut Writer, g: &Gem) {
    let mut a = Attrs::new();
    put(&mut a, "gemId", g.gem.as_ref().map(|x| x.as_str()));
    put(&mut a, "skillId", g.skill.as_ref().map(|x| x.as_str()));
    put(&mut a, "nameSpec", g.name.as_deref());
    put_number(&mut a, "level", Some(&g.level));
    put_number(&mut a, "quality", Some(&g.quality));
    put(&mut a, "enabled", Some(bool_text(g.enabled)));
    kept(&mut a, &g.attributes);
    w.element(4, "Gem", &a, |_| {});
}

fn config(w: &mut Writer, c: &Config) {
    let mut a = Attrs::new();
    put(
        &mut a,
        "activeConfigSet",
        c.active.map(|x| x.0.to_string()).as_deref(),
    );
    kept(&mut a, &c.attributes);
    w.element(1, "Config", &a, |w| {
        w.ordered(2, &c.sets, |w, set| {
            let mut a = vec![("id".to_owned(), set.id.0.to_string())];
            put(&mut a, "title", set.title.as_deref());
            kept(&mut a, &set.attributes);
            w.element(2, "ConfigSet", &a, |w| {
                w.ordered(3, &set.inputs, |w, i| {
                    let mut a = vec![("name".to_owned(), i.name.clone())];
                    match &i.value {
                        ConfigValue::Boolean(b) => put(&mut a, "boolean", Some(bool_text(*b))),
                        ConfigValue::Number(n) => {
                            let text = n
                                .source()
                                .map_or_else(|| n.value().0.to_string(), str::to_owned);
                            put(&mut a, "number", Some(&text));
                        }
                        ConfigValue::String(s) => put(&mut a, "string", Some(s)),
                    }
                    kept(&mut a, &i.attributes);
                    w.element(3, "Input", &a, |_| {});
                });
            });
        });
    });
}
