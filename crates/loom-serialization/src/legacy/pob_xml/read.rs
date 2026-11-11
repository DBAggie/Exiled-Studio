//! PoB XML to the domain Build (module docs).

use loom_domain::poe1::Poe1Choices;
use loom_domain::{
    AscendancyId, Attribute, Attributes, Build, BuildPart, Character, ClassId, Config, ConfigInput,
    ConfigSet, ConfigSetId, ConfigValue, Game, GameSpecific, Gem, GemId, ItemId, JewelSocket,
    MasteryEffect, NodeId, Number, OpaqueNode, Ordered, PassiveSpec, Passives, SkillId, SkillSet,
    SkillSetId, Skills, SlotId, SocketGroup, Sourced, SpecId, SpecPart, TreeVersion,
};
use roxmltree::{Node, NodeType};

use super::ImportError;

/// The content of a node kept as read; `None` for whitespace-only text
/// (layout between elements, which the export writes its own way).
pub(super) fn opaque(node: Node<'_, '_>) -> Option<OpaqueNode> {
    match node.node_type() {
        NodeType::Element => {
            // Mixed content (an element with text, such as an item's text
            // around `<ModRange>`): every text child is kept, whitespace
            // included, and the export writes it without layout.
            let mixed = is_mixed(node);
            Some(OpaqueNode::Element {
                name: node.tag_name().name().to_owned(),
                attributes: node
                    .attributes()
                    .map(|a| Attribute::new(a.name(), a.value()))
                    .collect(),
                children: node
                    .children()
                    .filter_map(|c| {
                        if mixed && c.is_text() {
                            Some(OpaqueNode::Text(c.text().unwrap_or("").to_owned()))
                        } else {
                            opaque(c)
                        }
                    })
                    .collect(),
            })
        }
        NodeType::Text => {
            let t = node.text().unwrap_or("");
            (!t.trim().is_empty()).then(|| OpaqueNode::Text(t.to_owned()))
        }
        NodeType::Comment => Some(OpaqueNode::Comment(node.text().unwrap_or("").to_owned())),
        NodeType::PI => node.pi().map(|pi| OpaqueNode::ProcessingInstruction {
            target: pi.target.to_owned(),
            data: pi.value.map(str::to_owned),
        }),
        NodeType::Root => None,
    }
}

/// The attributes of `node` not in `typed`, in order.
pub(super) fn rest(node: Node<'_, '_>, typed: &[&str]) -> Attributes {
    node.attributes()
        .filter(|a| !typed.contains(&a.name()))
        .map(|a| Attribute::new(a.name(), a.value()))
        .collect()
}

pub(super) fn text(node: Node<'_, '_>, name: &str) -> Option<String> {
    node.attribute(name).map(str::to_owned)
}

/// `Some(None)` when absent, `Some(Some(v))` when it parses, `None` when
/// present and not a number (the element is then kept opaque).
/// Whether an element has a text child that is not only whitespace.
pub(super) fn is_mixed(node: Node<'_, '_>) -> bool {
    node.children()
        .any(|c| c.is_text() && c.text().is_some_and(|t| !t.trim().is_empty()))
}

/// A number written in its canonical form (`+5` and `007` are not: the
/// export would write them differently, so their element is kept whole).
fn canonical<T: std::str::FromStr + ToString>(s: &str) -> Option<T> {
    s.parse::<T>().ok().filter(|v| v.to_string() == s)
}

/// `Some(None)` when absent, `Some(Some(v))` when it reads, `None` when
/// present and not a canonical number (the element is then kept whole).
pub(super) fn number_u32(node: Node<'_, '_>, name: &str) -> Option<Option<Sourced<u32>>> {
    match node.attribute(name) {
        None => Some(None),
        Some(s) => canonical(s).map(|v| Some(Sourced::read(v, s))),
    }
}

pub(super) fn required_u32(node: Node<'_, '_>, name: &str) -> Option<Sourced<u32>> {
    number_u32(node, name).flatten()
}

fn required_i32(node: Node<'_, '_>, name: &str) -> Option<Sourced<i32>> {
    let s = node.attribute(name)?;
    canonical(s).map(|v| Sourced::read(v, s))
}

fn boolean(s: &str) -> Option<bool> {
    match s {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

pub(super) fn children<'a, 'i>(node: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    node.children().filter(|c| opaque(*c).is_some())
}

/// Reads the build (module docs).
pub(super) fn build(doc: &roxmltree::Document<'_>) -> Result<Build, ImportError> {
    let root = doc.root_element();
    if root.tag_name().name() != "PathOfBuilding" {
        return Err(ImportError::Root(root.tag_name().name().to_owned()));
    }
    let mut b = Build::new(Game::Poe1);
    b.attributes = rest(root, &[]);
    let mut before = true;
    for n in doc.root().children() {
        if n == root {
            before = false;
        } else if let Some(o) = opaque(n) {
            if before {
                b.prolog.push(o);
            } else {
                b.epilog.push(o);
            }
        }
    }
    let mut layout = Ordered::new();
    let (mut character, mut tree, mut skills, mut items, mut config) =
        (false, false, false, false, false);
    for n in children(root) {
        let typed = match n.tag_name().name() {
            "Build" if n.is_element() && !character => character_of(n).map(|(c, choices)| {
                b.character = c;
                if let Some(choices) = choices {
                    b.set_game_specific(GameSpecific::Poe1(choices)).ok();
                }
                character = true;
                BuildPart::Character
            }),
            "Tree" if n.is_element() && !tree => passives(n).map(|p| {
                b.passives = p;
                tree = true;
                BuildPart::Passives
            }),
            "Skills" if n.is_element() && !skills => skills_of(n).map(|s| {
                b.skills = s;
                skills = true;
                BuildPart::Skills
            }),
            "Items" if n.is_element() && !items => super::items::items_of(n).map(|i| {
                b.items = i;
                items = true;
                BuildPart::Items
            }),
            "Config" if n.is_element() && !config => config_of(n).map(|c| {
                b.config = c;
                config = true;
                BuildPart::Config
            }),
            _ => None,
        };
        match typed {
            Some(part) => layout.push(part),
            None => {
                if let Some(o) = opaque(n) {
                    layout.push_opaque(o);
                }
            }
        }
    }
    b.layout = layout;
    Ok(b)
}

const CHARACTER: [&str; 7] = [
    "className",
    "ascendClassName",
    "level",
    "mainSocketGroup",
    "bandit",
    "pantheonMajorGod",
    "pantheonMinorGod",
];

fn character_of(n: Node<'_, '_>) -> Option<(Character, Option<Poe1Choices>)> {
    let choices = Poe1Choices {
        bandit: text(n, "bandit"),
        pantheon_major: text(n, "pantheonMajorGod"),
        pantheon_minor: text(n, "pantheonMinorGod"),
    };
    let any_choice = choices.bandit.is_some()
        || choices.pantheon_major.is_some()
        || choices.pantheon_minor.is_some();
    Some((
        Character {
            class: text(n, "className").map(ClassId::new),
            ascendancy: text(n, "ascendClassName").map(AscendancyId::new),
            level: number_u32(n, "level")?,
            main_socket_group: number_u32(n, "mainSocketGroup")?,
            attributes: rest(n, &CHARACTER),
            children: n.children().filter_map(opaque).collect(),
        },
        any_choice.then_some(choices),
    ))
}

fn passives(n: Node<'_, '_>) -> Option<Passives> {
    let mut specs = Ordered::new();
    let mut count = 0;
    for c in children(n) {
        match (c.tag_name().name(), c.is_element()) {
            ("Spec", true) => {
                count += 1;
                match spec(c, SpecId(count)) {
                    Some(s) => specs.push(s),
                    None => specs.push_opaque(opaque(c)?),
                }
            }
            _ => specs.push_opaque(opaque(c)?),
        }
    }
    Some(Passives {
        active: number_u32(n, "activeSpec")?.map(|s| SpecId(*s.value())),
        specs,
        attributes: rest(n, &["activeSpec"]),
    })
}

/// `1,2,3` (empty: none).
fn node_list(s: &str) -> Option<Vec<NodeId>> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    s.split(',')
        .map(|x| x.trim().parse().ok().map(NodeId))
        .collect()
}

/// `{node,effect},{node,effect}` (empty: none).
fn mastery_list(s: &str) -> Option<Vec<MasteryEffect>> {
    if s.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    let mut rest = s;
    loop {
        let body = rest.strip_prefix('{')?;
        let (pair, tail) = body.split_once('}')?;
        let (node, effect) = pair.split_once(',')?;
        out.push(MasteryEffect {
            node: NodeId(node.trim().parse().ok()?),
            effect: effect.trim().parse().ok()?,
        });
        if tail.is_empty() {
            return Some(out);
        }
        rest = tail.strip_prefix(',')?;
    }
}

const SPEC: [&str; 4] = ["title", "treeVersion", "nodes", "masteryEffects"];

fn spec(n: Node<'_, '_>, id: SpecId) -> Option<PassiveSpec> {
    let mut s = PassiveSpec::new(id);
    s.title = text(n, "title");
    s.tree_version = text(n, "treeVersion").map(TreeVersion::new);
    if let Some(v) = n.attribute("nodes") {
        s.nodes = Sourced::read(node_list(v)?, v);
    }
    if let Some(v) = n.attribute("masteryEffects") {
        s.mastery_effects = Sourced::read(mastery_list(v)?, v);
    }
    s.attributes = rest(n, &SPEC);
    let mut sockets_read = false;
    for c in children(n) {
        if c.tag_name().name() == "Sockets"
            && c.is_element()
            && !sockets_read
            && let Some(list) = sockets(c)
        {
            s.jewel_sockets = list;
            s.layout.push(SpecPart::JewelSockets);
            sockets_read = true;
            continue;
        }
        s.layout.push_opaque(opaque(c)?);
    }
    Some(s)
}

fn sockets(n: Node<'_, '_>) -> Option<Ordered<JewelSocket>> {
    if n.attributes().next().is_some() {
        return None;
    }
    let mut out = Ordered::new();
    for c in children(n) {
        if c.tag_name().name() == "Socket" && c.is_element() && children(c).next().is_none() {
            out.push(JewelSocket {
                node: NodeId(*required_u32(c, "nodeId")?.value()),
                // PoB writes `itemId="0"` for an empty socket.
                item: Some(*required_u32(c, "itemId")?.value())
                    .filter(|i| *i != 0)
                    .map(ItemId),
                attributes: rest(c, &["nodeId", "itemId"]),
            });
        } else {
            out.push_opaque(opaque(c)?);
        }
    }
    Some(out)
}

fn skills_of(n: Node<'_, '_>) -> Option<Skills> {
    let mut sets = Ordered::new();
    for c in children(n) {
        match (c.tag_name().name(), c.is_element()) {
            ("SkillSet", true) => match skill_set(c) {
                Some(s) => sets.push(s),
                None => sets.push_opaque(opaque(c)?),
            },
            _ => sets.push_opaque(opaque(c)?),
        }
    }
    Some(Skills {
        active: number_u32(n, "activeSkillSet")?.map(|s| SkillSetId(*s.value())),
        sets,
        attributes: rest(n, &["activeSkillSet"]),
    })
}

fn skill_set(n: Node<'_, '_>) -> Option<SkillSet> {
    let mut groups = Ordered::new();
    for c in children(n) {
        match (c.tag_name().name(), c.is_element()) {
            ("Skill", true) => match group(c) {
                Some(g) => groups.push(g),
                None => groups.push_opaque(opaque(c)?),
            },
            _ => groups.push_opaque(opaque(c)?),
        }
    }
    Some(SkillSet {
        id: SkillSetId(*required_u32(n, "id")?.value()),
        title: text(n, "title"),
        groups,
        attributes: rest(n, &["id", "title"]),
    })
}

const GROUP: [&str; 4] = ["label", "enabled", "slot", "mainActiveSkill"];

fn group(n: Node<'_, '_>) -> Option<SocketGroup> {
    let mut gems = Ordered::new();
    for c in children(n) {
        match (c.tag_name().name(), c.is_element()) {
            ("Gem", true) => match gem(c) {
                Some(g) => gems.push(g),
                None => gems.push_opaque(opaque(c)?),
            },
            _ => gems.push_opaque(opaque(c)?),
        }
    }
    let main = n
        .attribute("mainActiveSkill")
        .and_then(|v| canonical::<u32>(v).map(|x| Sourced::read(x, v)));
    let attributes = if main.is_some() {
        rest(n, &GROUP)
    } else {
        rest(n, &["label", "enabled", "slot"])
    };
    Some(SocketGroup {
        label: text(n, "label"),
        enabled: boolean(n.attribute("enabled")?)?,
        slot: text(n, "slot").map(SlotId::new),
        // PoB writes `mainActiveSkill="nil"` when it is unset (and reads it
        // as 1); a value that is not a number stays an untyped attribute.
        main_active_skill: main,
        gems,
        attributes,
    })
}

const GEM: [&str; 6] = [
    "gemId", "skillId", "nameSpec", "level", "quality", "enabled",
];

fn gem(n: Node<'_, '_>) -> Option<Gem> {
    if children(n).next().is_some() {
        return None;
    }
    Some(Gem {
        gem: text(n, "gemId").map(GemId::new),
        skill: text(n, "skillId").map(SkillId::new),
        name: text(n, "nameSpec"),
        level: required_u32(n, "level")?,
        quality: required_i32(n, "quality")?,
        enabled: boolean(n.attribute("enabled")?)?,
        attributes: rest(n, &GEM),
    })
}

fn config_of(n: Node<'_, '_>) -> Option<Config> {
    let mut sets = Ordered::new();
    for c in children(n) {
        match (c.tag_name().name(), c.is_element()) {
            ("ConfigSet", true) => match config_set(c) {
                Some(s) => sets.push(s),
                None => sets.push_opaque(opaque(c)?),
            },
            _ => sets.push_opaque(opaque(c)?),
        }
    }
    Some(Config {
        active: number_u32(n, "activeConfigSet")?.map(|s| ConfigSetId(*s.value())),
        sets,
        attributes: rest(n, &["activeConfigSet"]),
    })
}

fn config_set(n: Node<'_, '_>) -> Option<ConfigSet> {
    let mut inputs = Ordered::new();
    for c in children(n) {
        match (c.tag_name().name(), c.is_element()) {
            ("Input", true) => match input(c) {
                Some(i) => inputs.push(i),
                None => inputs.push_opaque(opaque(c)?),
            },
            _ => inputs.push_opaque(opaque(c)?),
        }
    }
    Some(ConfigSet {
        id: ConfigSetId(*required_u32(n, "id")?.value()),
        title: text(n, "title"),
        inputs,
        attributes: rest(n, &["id", "title"]),
    })
}

fn input(n: Node<'_, '_>) -> Option<ConfigInput> {
    if children(n).next().is_some() {
        return None;
    }
    let value = match (
        n.attribute("boolean"),
        n.attribute("number"),
        n.attribute("string"),
    ) {
        (Some(b), None, None) => ConfigValue::Boolean(boolean(b)?),
        (None, Some(x), None) => ConfigValue::Number(Sourced::read(Number(x.parse().ok()?), x)),
        (None, None, Some(s)) => ConfigValue::String(s.to_owned()),
        _ => return None,
    };
    Some(ConfigInput {
        name: text(n, "name")?,
        value,
        attributes: rest(n, &["name", "boolean", "number", "string"]),
    })
}
