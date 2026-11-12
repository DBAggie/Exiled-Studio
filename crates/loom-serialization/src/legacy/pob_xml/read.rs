//! PoB XML to the domain Build (module docs).

use loom_domain::poe1::Poe1Choices;
use loom_domain::{
    AscendancyId, Attribute, Attributes, Build, BuildPart, Character, ClassId, Config, ConfigInput,
    ConfigInputKind, ConfigLayout, ConfigSet, ConfigSetId, ConfigValue, CustomModifierBlock,
    CustomModifierLine, Game, GameSpecific, Gem, GemId, ItemId, JewelSocket, LineResolution,
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

/// The `<Config>` element. With no `ConfigSet` child and some content, it
/// is PoB's layout before config sets: PoB loads every child into set 1
/// (PoB v2.67.2, `Classes/ConfigTab.lua`, `Load`, lines 888 to 902), read
/// here as one flat set. A mix of both layouts keeps its loose children
/// untyped (PoB's result then depends on their order).
fn config_of(n: Node<'_, '_>) -> Option<Config> {
    let active = number_u32(n, "activeConfigSet")?.map(|s| ConfigSetId(*s.value()));
    let attributes = rest(n, &["activeConfigSet"]);
    let is_set = |c: &Node<'_, '_>| c.is_element() && c.tag_name().name() == "ConfigSet";
    if children(n).next().is_some() && !children(n).any(|c| is_set(&c)) {
        let (inputs, custom_modifiers) = set_content(n)?;
        let mut sets = Ordered::new();
        sets.push(ConfigSet {
            id: ConfigSetId(1),
            title: None,
            inputs,
            custom_modifiers,
            attributes: Attributes::new(),
        });
        return Some(Config {
            active,
            sets,
            layout: ConfigLayout::Flat,
            attributes,
        });
    }
    let mut sets = Ordered::new();
    for c in children(n) {
        match config_set(c).filter(|_| is_set(&c)) {
            Some(s) => sets.push(s),
            None => sets.push_opaque(opaque(c)?),
        }
    }
    Some(Config {
        active,
        sets,
        layout: ConfigLayout::Sets,
        attributes,
    })
}

fn config_set(n: Node<'_, '_>) -> Option<ConfigSet> {
    let (inputs, custom_modifiers) = set_content(n)?;
    Some(ConfigSet {
        id: ConfigSetId(*required_u32(n, "id")?.value()),
        title: text(n, "title"),
        inputs,
        custom_modifiers,
        attributes: rest(n, &["id", "title"]),
    })
}

/// A set's inputs, placeholders, and custom modifier blocks.
fn set_content(n: Node<'_, '_>) -> Option<(Ordered<ConfigInput>, Vec<CustomModifierBlock>)> {
    // The blocks after the last other child are typed (PoB writes them
    // last); any other block stays opaque in place.
    let all: Vec<_> = children(n).collect();
    let first_block = all
        .iter()
        .rposition(|c| custom_modifier_block(*c).is_none())
        .map_or(0, |i| i + 1);
    let (others, blocks) = all.split_at_checked(first_block).unwrap_or((&all, &[]));
    let custom_modifiers = blocks
        .iter()
        .filter_map(|c| custom_modifier_block(*c))
        .collect();
    let mut inputs = Ordered::new();
    for &c in others {
        match (c.tag_name().name(), c.is_element()) {
            ("Input", true) => match input(c) {
                Some(i) => inputs.push(i),
                None => inputs.push_opaque(opaque(c)?),
            },
            ("Placeholder", true) => match placeholder(c) {
                Some(i) => inputs.push(i),
                None => inputs.push_opaque(opaque(c)?),
            },
            _ => inputs.push_opaque(opaque(c)?),
        }
    }
    Some((inputs, custom_modifiers))
}

/// A `CustomModifierBlock` (PoB v2.67.2, `Classes/ConfigTab.lua`, `Load`
/// and `Save`) with `enabled` written as `true` or `false` and only text
/// inside. Any other form stays opaque (PoB reads a missing `enabled` as
/// true and any other value as false).
///
/// PoB reads the text with its own XML reader (`runtime/lua/xml.lua`,
/// `ParseXML`): trimmed, the five named entities decoded, any other
/// reference read as nothing, and a CDATA section as a separate node. XML
/// decodes a character reference (`&#10;` a line break) and a CDATA
/// section as text, so a block whose raw text holds either is kept whole:
/// read here, its lines would differ from PoB's (ADR-015: kept, never
/// silently changed).
fn custom_modifier_block(n: Node<'_, '_>) -> Option<CustomModifierBlock> {
    if !n.is_element() || n.tag_name().name() != "CustomModifierBlock" {
        return None;
    }
    if n.children().any(|c| !c.is_text()) {
        return None;
    }
    // The raw content: after the first `>`, before the end tag. A raw `>`
    // in an attribute only adds the attributes' rest to the text checked,
    // never leaves content out. An element with no children has none.
    let raw = n.document().input_text().get(n.range()).unwrap_or_default();
    let content = raw
        .split_once('>')
        .and_then(|(_, after)| after.rsplit_once("</"))
        .map_or("", |(content, _)| content);
    let read_differently = n.has_children()
        && (content.contains("<![CDATA[")
            || content.split('&').skip(1).any(|r| {
                r.split_once(';')
                    .is_none_or(|(e, _)| named_entity(e).is_none())
            }));
    if read_differently {
        return None;
    }
    let enabled = match n.attribute("enabled")? {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    let body: String = n.children().filter_map(|c| c.text()).collect();
    Some(CustomModifierBlock {
        title: text(n, "title"),
        enabled,
        lines: custom_modifier_lines(&body)
            .map(|text| CustomModifierLine {
                text,
                resolution: LineResolution::Pending,
            })
            .collect(),
        text: body,
        attributes: rest(n, &["title", "enabled"]),
    })
}

/// The lines of a custom modifier block that PoB parses as modifiers
/// (PoB v2.67.2, `Classes/ConfigTab.lua`, `BuildModList`, lines 1106 to
/// 1128): each line with its colour codes removed (`StripEscapes`,
/// `HeadlessWrapper.lua` 75: `^` and a digit, then `^x` and six hex
/// digits) and trimmed of Lua's `%s`. An empty line is left out: it is no
/// modifier.
pub(super) fn custom_modifier_lines(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split('\n').filter_map(|line| {
        let line = strip_escapes(line);
        let line = line.trim_matches(is_lua_space);
        (!line.is_empty()).then(|| line.to_owned())
    })
}

/// Lua's `%s`: space, tab, line feed, vertical tab, form feed, carriage
/// return.
const fn is_lua_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r')
}

/// PoB's `StripEscapes`: every `^` and a digit removed, then every `^x`
/// and six hex digits, in two passes as `gsub` does.
fn strip_escapes(line: &str) -> String {
    let colour = strip(line, 2, |b| matches!(b, [b'^', d] if d.is_ascii_digit()));
    strip(
        &colour,
        8,
        |b| matches!(b, [b'^', b'x', hex @ ..] if hex.iter().all(u8::is_ascii_hexdigit)),
    )
}

/// `text` without every non-overlapping window of `len` bytes that
/// `matches`, left to right. Only ASCII windows match, and a multi-byte
/// character holds no ASCII byte, so the rest stays valid UTF-8.
fn strip(text: &str, len: usize, matches: impl Fn(&[u8]) -> bool) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        match bytes.get(i..i + len) {
            Some(window) if matches(window) => i += len,
            _ => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
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
        (None, None, Some(_)) => ConfigValue::String(pob_attribute(n, "string")?),
        _ => return None,
    };
    let name = text(n, "name")?;
    // PoB's legacy custom modifiers: one string of lines, which `Load`
    // moves into a block (`ConfigTab.lua` 872 to 884); whether they apply
    // is the calculations' concern.
    let lines = match &value {
        ConfigValue::String(s) if name == LEGACY_CUSTOM_MODS => custom_modifier_lines(s)
            .map(|text| CustomModifierLine {
                text,
                resolution: LineResolution::Pending,
            })
            .collect(),
        _ => Vec::new(),
    };
    Some(ConfigInput {
        name,
        value,
        kind: ConfigInputKind::Input,
        lines,
        attributes: rest(n, &["name", "boolean", "number", "string"]),
    })
}

/// PoB's legacy custom modifier input (before `CustomModifierBlock`).
const LEGACY_CUSTOM_MODS: &str = "customMods";

/// The value of attribute `name` as PoB reads it (PoB v2.67.2,
/// `runtime/lua/xml.lua`, `parseAttribs` and `decodeContent`): the raw
/// text between the quotes, with its line breaks, tabs, and carriage
/// returns (XML would turn each into a space), `&lt;`, `&gt;`, `&amp;`,
/// `&apos;`, and `&quot;` decoded, and any other reference (`&#10;`)
/// removed. PoB writes line breaks raw (`encodeContent` escapes only those
/// five characters), so a multi-line value round-trips only this way.
///
/// `None` (the element is then kept whole) when the attribute is absent,
/// when its raw text, read as XML reads it, is not the parser's value
/// (the raw text is then not where it was looked for), or when it holds a
/// raw `>`: XML allows one, but PoB's tag pattern ends the tag there
/// (`xml.lua` 52), so PoB does not read the value at all. Also `None`
/// when it holds a reference PoB reads as nothing (`&#10;`): PoB drops it
/// silently, so the element is kept whole and stays a reason rather than
/// losing that text without a trace (ADR-015).
fn pob_attribute(n: Node<'_, '_>, name: &str) -> Option<String> {
    let a = n.attribute_node(name)?;
    let raw = n.document().input_text().get(a.range_value())?;
    let dropped = raw.split('&').skip(1).any(|r| {
        r.split_once(';')
            .is_none_or(|(e, _)| named_entity(e).is_none())
    });
    (!raw.contains('>') && !dropped && xml_attribute_value(raw)? == a.value())
        .then(|| pob_decode(raw))
}

/// A raw attribute text read as XML reads it: line ends normalised, then
/// each line break and tab a space, then references decoded (a character
/// reference is not normalised). `None` for a reference XML does not
/// define.
fn xml_attribute_value(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '&' => {
                let reference: String = chars.by_ref().take_while(|&c| c != ';').collect();
                out.push(xml_reference(&reference)?);
            }
            '\r' => {
                chars.next_if_eq(&'\n');
                out.push(' ');
            }
            '\n' | '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    Some(out)
}

/// An XML reference's character: a predefined entity, or a decimal or
/// hexadecimal character reference.
fn xml_reference(reference: &str) -> Option<char> {
    if let Some(named) = named_entity(reference) {
        return Some(named);
    }
    let code = match reference.strip_prefix("#x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => reference.strip_prefix('#')?.parse().ok()?,
    };
    char::from_u32(code)
}

/// The five entities PoB decodes.
fn named_entity(name: &str) -> Option<char> {
    match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "apos" => Some('\''),
        "quot" => Some('"'),
        _ => None,
    }
}

/// PoB's `decodeContent`: each `&` up to the next `;` replaced by the
/// named entity, or by nothing when it is any other; an `&` with no `;`
/// after it stays.
fn pob_decode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some((before, after)) = rest.split_once('&') {
        out.push_str(before);
        match after.split_once(';') {
            Some((name, tail)) => {
                out.extend(named_entity(name));
                rest = tail;
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// A `Placeholder` with a number (PoB v2.67.2, `Classes/ConfigTab.lua`,
/// `Load`). Any other form stays opaque: PoB loads a `string` placeholder
/// as the input's value, not as a placeholder.
fn placeholder(n: Node<'_, '_>) -> Option<ConfigInput> {
    if children(n).next().is_some() {
        return None;
    }
    let x = n.attribute("number")?;
    Some(ConfigInput {
        name: text(n, "name")?,
        value: ConfigValue::Number(Sourced::read(Number(x.parse().ok()?), x)),
        kind: ConfigInputKind::Placeholder,
        lines: Vec::new(),
        attributes: rest(n, &["name", "number"]),
    })
}
