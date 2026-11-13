//! Domain to DTO and back. Both directions keep every list in model order
//! and add nothing, so `from_dto(to_dto(build)) == build`.
//!
//! Domain to DTO cannot fail ([`super::Document::from_domain`] checks the
//! one invariant the document relies on first). DTO to domain checks what
//! the types cannot: the game-specific choices are of the build's game,
//! indices fit `usize`, non-finite numbers are well formed, and the item
//! lines' continuation links hold.

use loom_domain as d;

use super::DtoError;
use super::types::{Adapted, Exclusion};
use super::types::{
    AltVariant, Attribute, Build, BuildPart, Character, Config, ConfigInput, ConfigInputKind,
    ConfigLayout, ConfigSet, ConfigValue, CustomModifierBlock, CustomModifierLine, DataVersion,
    Entry, Game, GameSpecific, Gem, HeaderLine, Item, ItemBody, ItemLine, ItemPart, ItemSet, Items,
    ItemsEntry, JewelSocket, LineKind, LineResolution, MasteryEffect, ModSection, ModifierLine,
    NoBase, NonFinite, Number, OpaqueNode, PassiveSpec, Passives, Poe2Choices, Prefix,
    PropertyValue, Rarity, Resolved, SkillSet, Skills, SlotEntry, SocketGroup, Sourced, SpecPart,
    StatLine, StatValue, TypedResolution, UnresolvedReason, VariantSelection,
};

// ---- shared ----------------------------------------------------------------

fn attrs_to(a: &d::Attributes) -> Vec<Attribute> {
    a.iter()
        .map(|a| Attribute {
            name: a.name.clone(),
            value: a.value.clone(),
        })
        .collect()
}

fn attrs_from(a: Vec<Attribute>) -> d::Attributes {
    a.into_iter()
        .map(|a| d::Attribute::new(a.name, a.value))
        .collect()
}

fn opaque_to(n: &d::OpaqueNode) -> OpaqueNode {
    match n {
        d::OpaqueNode::Element {
            name,
            attributes,
            children,
        } => OpaqueNode::Element {
            name: name.clone(),
            attributes: attrs_to(attributes),
            children: children.iter().map(opaque_to).collect(),
        },
        d::OpaqueNode::Text(t) => OpaqueNode::Text(t.clone()),
        d::OpaqueNode::Comment(t) => OpaqueNode::Comment(t.clone()),
        d::OpaqueNode::ProcessingInstruction { target, data } => {
            OpaqueNode::ProcessingInstruction {
                target: target.clone(),
                data: data.clone(),
            }
        }
    }
}

fn opaque_from(n: OpaqueNode) -> d::OpaqueNode {
    match n {
        OpaqueNode::Element {
            name,
            attributes,
            children,
        } => d::OpaqueNode::Element {
            name,
            attributes: attrs_from(attributes),
            children: children.into_iter().map(opaque_from).collect(),
        },
        OpaqueNode::Text(t) => d::OpaqueNode::Text(t),
        OpaqueNode::Comment(t) => d::OpaqueNode::Comment(t),
        OpaqueNode::ProcessingInstruction { target, data } => {
            d::OpaqueNode::ProcessingInstruction { target, data }
        }
    }
}

fn ordered_to<T, U>(o: &d::Ordered<T>, f: impl Fn(&T) -> U) -> Vec<Entry<U>> {
    o.entries()
        .iter()
        .map(|e| match e {
            d::Entry::Typed(t) => Entry::Typed(f(t)),
            d::Entry::Opaque(n) => Entry::Opaque(opaque_to(n)),
        })
        .collect()
}

fn ordered_from<T, U>(
    v: Vec<Entry<T>>,
    f: impl Fn(T) -> Result<U, DtoError>,
) -> Result<d::Ordered<U>, DtoError> {
    v.into_iter()
        .map(|e| {
            Ok(match e {
                Entry::Typed(t) => d::Entry::Typed(f(t)?),
                Entry::Opaque(n) => d::Entry::Opaque(opaque_from(n)),
            })
        })
        .collect()
}

fn sourced_to<T, U>(s: &d::Sourced<T>, f: impl Fn(&T) -> U) -> Sourced<U> {
    Sourced {
        value: f(s.value()),
        source: s.source().map(str::to_owned),
    }
}

fn sourced_from<T, U>(
    s: Sourced<T>,
    f: impl Fn(T) -> Result<U, DtoError>,
) -> Result<d::Sourced<U>, DtoError> {
    let value = f(s.value)?;
    Ok(match s.source {
        Some(source) => d::Sourced::read(value, source),
        None => d::Sourced::new(value),
    })
}

fn copy<T: Copy>(v: &T) -> T {
    *v
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "passed where a fallible conversion is expected"
)]
const fn ok<T>(v: T) -> Result<T, DtoError> {
    Ok(v)
}

fn index_to(i: usize) -> u64 {
    // `usize` is at most 64 bits on every target Rust supports.
    i as u64
}

fn index_from(i: u64) -> Result<usize, DtoError> {
    usize::try_from(i).map_err(|_| DtoError::OutOfRange(i))
}

fn number_to(n: &d::Number) -> Number {
    if n.0.is_finite() {
        Number::Finite(n.0)
    } else {
        Number::NonFinite(NonFinite {
            f64_bits: format!("{:016x}", n.0.to_bits()),
        })
    }
}

fn number_from(n: Number) -> Result<d::Number, DtoError> {
    match n {
        Number::Finite(v) => Ok(d::Number(v)),
        Number::NonFinite(NonFinite { f64_bits }) => {
            let bits = (f64_bits.len() == 16
                && f64_bits
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
            .then(|| u64::from_str_radix(&f64_bits, 16).ok())
            .flatten()
            .map(f64::from_bits)
            // A finite value must be written as a number: one form each.
            .filter(|v| !v.is_finite());
            bits.map(d::Number).ok_or(DtoError::Number(f64_bits))
        }
    }
}

// ---- build -----------------------------------------------------------------

const fn game_to(g: d::Game) -> Game {
    match g {
        d::Game::Poe1 => Game::Poe1,
        d::Game::Poe2 => Game::Poe2,
    }
}

const fn game_from(g: &Game) -> d::Game {
    match g {
        Game::Poe1 => d::Game::Poe1,
        Game::Poe2 => d::Game::Poe2,
    }
}

const fn part_to(p: &d::BuildPart) -> BuildPart {
    match p {
        d::BuildPart::Character => BuildPart::Character,
        d::BuildPart::Passives => BuildPart::Passives,
        d::BuildPart::Skills => BuildPart::Skills,
        d::BuildPart::Items => BuildPart::Items,
        d::BuildPart::Config => BuildPart::Config,
    }
}

const fn part_from(p: BuildPart) -> Result<d::BuildPart, DtoError> {
    Ok(match p {
        BuildPart::Character => d::BuildPart::Character,
        BuildPart::Passives => d::BuildPart::Passives,
        BuildPart::Skills => d::BuildPart::Skills,
        BuildPart::Items => d::BuildPart::Items,
        BuildPart::Config => d::BuildPart::Config,
    })
}

pub(super) fn build_to(b: &d::Build) -> Build {
    let c = &b.character;
    Build {
        game: game_to(b.game()),
        game_specific: match b.game_specific() {
            d::GameSpecific::Poe1(p) => GameSpecific::Poe1(p.into()),
            d::GameSpecific::Poe2(d::Poe2Choices {}) => GameSpecific::Poe2(Poe2Choices {}),
        },
        data: b.data.as_ref().map(|v| DataVersion {
            game: game_to(v.game),
            schema_version: v.schema_version,
            steam_build: v.steam_build,
        }),
        character: Character {
            class: c.class.as_ref().map(|i| i.as_str().to_owned()),
            ascendancy: c.ascendancy.as_ref().map(|i| i.as_str().to_owned()),
            level: c.level.as_ref().map(|s| sourced_to(s, copy)),
            main_socket_group: c.main_socket_group.as_ref().map(|s| sourced_to(s, copy)),
            attributes: attrs_to(&c.attributes),
            children: c.children.iter().map(opaque_to).collect(),
        },
        passives: passives_to(&b.passives),
        skills: skills_to(&b.skills),
        items: items_to(&b.items),
        config: config_to(&b.config),
        layout: ordered_to(&b.layout, part_to),
        attributes: attrs_to(&b.attributes),
        prolog: b.prolog.iter().map(opaque_to).collect(),
        epilog: b.epilog.iter().map(opaque_to).collect(),
    }
}

pub(super) fn build_from(b: Build) -> Result<d::Build, DtoError> {
    let game = game_from(&b.game);
    let mut out = d::Build::new(game);
    let choices = match b.game_specific {
        GameSpecific::Poe1(p) => d::GameSpecific::Poe1(p.into()),
        GameSpecific::Poe2(Poe2Choices {}) => d::GameSpecific::Poe2(d::Poe2Choices {}),
    };
    out.set_game_specific(choices).map_err(DtoError::Domain)?;
    out.data = b.data.map(|v| d::DataVersion {
        game: game_from(&v.game),
        schema_version: v.schema_version,
        steam_build: v.steam_build,
    });
    let c = b.character;
    out.character = d::Character {
        class: c.class.map(d::ClassId::new),
        ascendancy: c.ascendancy.map(d::AscendancyId::new),
        level: c.level.map(|s| sourced_from(s, ok)).transpose()?,
        main_socket_group: c
            .main_socket_group
            .map(|s| sourced_from(s, ok))
            .transpose()?,
        attributes: attrs_from(c.attributes),
        children: c.children.into_iter().map(opaque_from).collect(),
    };
    out.passives = passives_from(b.passives)?;
    out.skills = skills_from(b.skills)?;
    out.items = items_from(b.items)?;
    out.config = config_from(b.config)?;
    out.layout = ordered_from(b.layout, part_from)?;
    out.attributes = attrs_from(b.attributes);
    out.prolog = b.prolog.into_iter().map(opaque_from).collect();
    out.epilog = b.epilog.into_iter().map(opaque_from).collect();
    Ok(out)
}

// ---- passives --------------------------------------------------------------

fn passives_to(p: &d::Passives) -> Passives {
    Passives {
        active: p.active.map(|i| i.0),
        specs: ordered_to(&p.specs, |s| PassiveSpec {
            id: s.id.0,
            title: s.title.clone(),
            tree_version: s.tree_version.as_ref().map(|v| v.as_str().to_owned()),
            nodes: sourced_to(&s.nodes, |n| n.iter().map(|n| n.0).collect()),
            mastery_effects: sourced_to(&s.mastery_effects, |m| {
                m.iter()
                    .map(|m| MasteryEffect {
                        node: m.node.0,
                        effect: m.effect,
                    })
                    .collect()
            }),
            jewel_sockets: ordered_to(&s.jewel_sockets, |j| JewelSocket {
                node: j.node.0,
                item: j.item.map(|i| i.0),
                attributes: attrs_to(&j.attributes),
            }),
            layout: ordered_to(&s.layout, |p| match p {
                d::SpecPart::JewelSockets => SpecPart::JewelSockets,
            }),
            attributes: attrs_to(&s.attributes),
        }),
        attributes: attrs_to(&p.attributes),
    }
}

fn passives_from(p: Passives) -> Result<d::Passives, DtoError> {
    Ok(d::Passives {
        active: p.active.map(d::SpecId),
        specs: ordered_from(p.specs, |s| {
            let mut spec = d::PassiveSpec::new(d::SpecId(s.id));
            spec.title = s.title;
            spec.tree_version = s.tree_version.map(d::TreeVersion::new);
            spec.nodes = sourced_from(s.nodes, |n| Ok(n.into_iter().map(d::NodeId).collect()))?;
            spec.mastery_effects = sourced_from(s.mastery_effects, |m| {
                Ok(m.into_iter()
                    .map(|m| d::MasteryEffect {
                        node: d::NodeId(m.node),
                        effect: m.effect,
                    })
                    .collect())
            })?;
            spec.jewel_sockets = ordered_from(s.jewel_sockets, |j| {
                Ok(d::JewelSocket {
                    node: d::NodeId(j.node),
                    item: j.item.map(d::ItemId),
                    attributes: attrs_from(j.attributes),
                })
            })?;
            spec.layout = ordered_from(s.layout, |p| {
                Ok(match p {
                    SpecPart::JewelSockets => d::SpecPart::JewelSockets,
                })
            })?;
            spec.attributes = attrs_from(s.attributes);
            Ok(spec)
        })?,
        attributes: attrs_from(p.attributes),
    })
}

// ---- skills ----------------------------------------------------------------

fn skills_to(s: &d::Skills) -> Skills {
    Skills {
        active: s.active.map(|i| i.0),
        sets: ordered_to(&s.sets, |set| SkillSet {
            id: set.id.0,
            title: set.title.clone(),
            groups: ordered_to(&set.groups, |g| SocketGroup {
                label: g.label.clone(),
                enabled: g.enabled,
                slot: g.slot.as_ref().map(|s| s.as_str().to_owned()),
                main_active_skill: g.main_active_skill.as_ref().map(|s| sourced_to(s, copy)),
                gems: ordered_to(&g.gems, |gem| Gem {
                    gem: gem.gem.as_ref().map(|i| i.as_str().to_owned()),
                    skill: gem.skill.as_ref().map(|i| i.as_str().to_owned()),
                    name: gem.name.clone(),
                    level: sourced_to(&gem.level, copy),
                    quality: sourced_to(&gem.quality, copy),
                    enabled: gem.enabled,
                    attributes: attrs_to(&gem.attributes),
                }),
                attributes: attrs_to(&g.attributes),
            }),
            attributes: attrs_to(&set.attributes),
        }),
        attributes: attrs_to(&s.attributes),
    }
}

fn skills_from(s: Skills) -> Result<d::Skills, DtoError> {
    Ok(d::Skills {
        active: s.active.map(d::SkillSetId),
        sets: ordered_from(s.sets, |set| {
            Ok(d::SkillSet {
                id: d::SkillSetId(set.id),
                title: set.title,
                groups: ordered_from(set.groups, |g| {
                    Ok(d::SocketGroup {
                        label: g.label,
                        enabled: g.enabled,
                        slot: g.slot.map(d::SlotId::new),
                        main_active_skill: g
                            .main_active_skill
                            .map(|s| sourced_from(s, ok))
                            .transpose()?,
                        gems: ordered_from(g.gems, |gem| {
                            Ok(d::Gem {
                                gem: gem.gem.map(d::GemId::new),
                                skill: gem.skill.map(d::SkillId::new),
                                name: gem.name,
                                level: sourced_from(gem.level, ok)?,
                                quality: sourced_from(gem.quality, ok)?,
                                enabled: gem.enabled,
                                attributes: attrs_from(gem.attributes),
                            })
                        })?,
                        attributes: attrs_from(g.attributes),
                    })
                })?,
                attributes: attrs_from(set.attributes),
            })
        })?,
        attributes: attrs_from(s.attributes),
    })
}

// ---- items -----------------------------------------------------------------

fn items_to(i: &d::Items) -> Items {
    Items {
        active_set: i.active_set.map(|i| i.0),
        entries: ordered_to(&i.entries, |e| match e {
            d::ItemsEntry::Item(item) => ItemsEntry::Item(item_to(item)),
            d::ItemsEntry::Set(set) => ItemsEntry::Set(ItemSet {
                id: set.id.0,
                title: set.title.clone(),
                slots: ordered_to(&set.slots, |s| SlotEntry {
                    slot: s.slot.as_str().to_owned(),
                    item: s.item.map(|i| i.0),
                    attributes: attrs_to(&s.attributes),
                }),
                attributes: attrs_to(&set.attributes),
            }),
        }),
        attributes: attrs_to(&i.attributes),
    }
}

fn items_from(i: Items) -> Result<d::Items, DtoError> {
    Ok(d::Items {
        active_set: i.active_set.map(d::ItemSetId),
        entries: ordered_from(i.entries, |e| {
            Ok(match e {
                ItemsEntry::Item(item) => d::ItemsEntry::Item(item_from(item)?),
                ItemsEntry::Set(set) => d::ItemsEntry::Set(d::ItemSet {
                    id: d::ItemSetId(set.id),
                    title: set.title,
                    slots: ordered_from(set.slots, |s| {
                        Ok(d::SlotEntry {
                            slot: d::SlotId::new(s.slot),
                            item: s.item.map(d::ItemId),
                            attributes: attrs_from(s.attributes),
                        })
                    })?,
                    attributes: attrs_from(set.attributes),
                }),
            })
        })?,
        attributes: attrs_from(i.attributes),
    })
}

fn item_to(item: &d::Item) -> Item {
    let b = &item.body;
    Item {
        id: item.id.0,
        body: ItemBody {
            item_class: b.item_class.clone(),
            rarity: b.rarity.map(|r| match r {
                d::Rarity::Normal => Rarity::Normal,
                d::Rarity::Magic => Rarity::Magic,
                d::Rarity::Rare => Rarity::Rare,
                d::Rarity::Unique => Rarity::Unique,
                d::Rarity::Relic => Rarity::Relic,
            }),
            name: b.name.clone(),
            base: b.base.clone(),
            implicits: b.implicits.map(index_to),
            variants: VariantSelection {
                count: b.variants.count,
                selected: b.variants.selected,
                alternates: b
                    .variants
                    .alternates
                    .iter()
                    .map(|a| AltVariant {
                        index: a.index,
                        selected: a.selected,
                    })
                    .collect(),
            },
            lines: b
                .lines
                .iter()
                .map(|l| ItemLine {
                    source: l.source.clone(),
                    kind: kind_to(&l.kind),
                })
                .collect(),
        },
        layout: ordered_to(&item.layout, |p| match p {
            d::ItemPart::Body => ItemPart::Body,
        }),
        text_spacing: item.text_spacing.clone(),
        attributes: attrs_to(&item.attributes),
    }
}

fn item_from(item: Item) -> Result<d::Item, DtoError> {
    let b = item.body;
    let body = d::ItemBody {
        item_class: b.item_class,
        rarity: b.rarity.map(|r| match r {
            Rarity::Normal => d::Rarity::Normal,
            Rarity::Magic => d::Rarity::Magic,
            Rarity::Rare => d::Rarity::Rare,
            Rarity::Unique => d::Rarity::Unique,
            Rarity::Relic => d::Rarity::Relic,
        }),
        name: b.name,
        base: b.base,
        implicits: b.implicits.map(index_from).transpose()?,
        variants: d::VariantSelection {
            count: b.variants.count,
            selected: b.variants.selected,
            alternates: b
                .variants
                .alternates
                .into_iter()
                .map(|a| d::AltVariant {
                    index: a.index,
                    selected: a.selected,
                })
                .collect(),
        },
        lines: b
            .lines
            .into_iter()
            .map(|l| {
                Ok(d::ItemLine {
                    source: l.source,
                    kind: kind_from(l.kind)?,
                })
            })
            .collect::<Result<_, DtoError>>()?,
    };
    body.check_links().map_err(|e| DtoError::Item {
        item: item.id,
        error: e,
    })?;
    Ok(d::Item {
        id: d::ItemId(item.id),
        body,
        layout: ordered_from(item.layout, |p| {
            Ok(match p {
                ItemPart::Body => d::ItemPart::Body,
            })
        })?,
        text_spacing: item.text_spacing,
        attributes: attrs_from(item.attributes),
    })
}

fn kind_to(k: &d::LineKind) -> LineKind {
    match k {
        d::LineKind::Header(h) => LineKind::Header(match h {
            d::HeaderLine::ItemClass => HeaderLine::ItemClass,
            d::HeaderLine::Rarity => HeaderLine::Rarity,
            d::HeaderLine::Separator => HeaderLine::Separator,
            d::HeaderLine::Name => HeaderLine::Name,
            d::HeaderLine::Base => HeaderLine::Base,
        }),
        d::LineKind::Spec { key, value, known } => LineKind::Spec {
            key: key.clone(),
            value: value.clone(),
            known: *known,
        },
        d::LineKind::VariantBase { base, selected } => LineKind::VariantBase {
            base: base.clone(),
            selected: *selected,
        },
        d::LineKind::Influence(i) => LineKind::Influence(i.as_str().to_owned()),
        d::LineKind::Flag => LineKind::Flag,
        d::LineKind::Modifier(m) => LineKind::Modifier(ModifierLine {
            text: m.text.clone(),
            prefixes: m
                .prefixes
                .iter()
                .map(|p| Prefix {
                    key: p.key.clone(),
                    value: p.value.clone(),
                })
                .collect(),
            section: match &m.section {
                d::ModSection::Enchant => ModSection::Enchant,
                d::ModSection::Implicit => ModSection::Implicit,
                d::ModSection::Explicit => ModSection::Explicit,
                d::ModSection::Other(s) => ModSection::Other(s.clone()),
            },
            selected: m.selected,
            resolution: resolution_to(&m.resolution),
        }),
        d::LineKind::Unrecognized => LineKind::Unrecognized,
    }
}

fn kind_from(k: LineKind) -> Result<d::LineKind, DtoError> {
    Ok(match k {
        LineKind::Header(h) => d::LineKind::Header(match h {
            HeaderLine::ItemClass => d::HeaderLine::ItemClass,
            HeaderLine::Rarity => d::HeaderLine::Rarity,
            HeaderLine::Separator => d::HeaderLine::Separator,
            HeaderLine::Name => d::HeaderLine::Name,
            HeaderLine::Base => d::HeaderLine::Base,
        }),
        LineKind::Spec { key, value, known } => d::LineKind::Spec { key, value, known },
        LineKind::VariantBase { base, selected } => d::LineKind::VariantBase { base, selected },
        LineKind::Influence(i) => d::LineKind::Influence(d::InfluenceId::new(i)),
        LineKind::Flag => d::LineKind::Flag,
        LineKind::Modifier(m) => d::LineKind::Modifier(d::ModifierLine {
            text: m.text,
            prefixes: m
                .prefixes
                .into_iter()
                .map(|p| d::Prefix {
                    key: p.key,
                    value: p.value,
                })
                .collect(),
            section: match m.section {
                ModSection::Enchant => d::ModSection::Enchant,
                ModSection::Implicit => d::ModSection::Implicit,
                ModSection::Explicit => d::ModSection::Explicit,
                ModSection::Other(s) => d::ModSection::Other(s),
            },
            selected: m.selected,
            resolution: resolution_from(m.resolution)?,
        }),
        LineKind::Unrecognized => d::LineKind::Unrecognized,
    })
}

// ---- resolutions -----------------------------------------------------------

fn stats_to(s: &[d::StatLine]) -> Vec<StatLine> {
    s.iter()
        .map(|s| StatLine {
            stat: s.stat.as_str().to_owned(),
            value: match &s.value {
                d::StatValue::Exact { min, max } => StatValue::Exact {
                    min: *min,
                    max: *max,
                },
                d::StatValue::Ranged { min, max } => StatValue::Ranged {
                    min: *min,
                    max: *max,
                },
                d::StatValue::Hidden { assumed } => StatValue::Hidden { assumed: *assumed },
                d::StatValue::Lookup(keys) => StatValue::Lookup(keys.clone()),
            },
        })
        .collect()
}

fn stats_from(s: Vec<StatLine>) -> Vec<d::StatLine> {
    s.into_iter()
        .map(|s| d::StatLine {
            stat: d::StatId::new(s.stat),
            value: match s.value {
                StatValue::Exact { min, max } => d::StatValue::Exact { min, max },
                StatValue::Ranged { min, max } => d::StatValue::Ranged { min, max },
                StatValue::Hidden { assumed } => d::StatValue::Hidden { assumed },
                StatValue::Lookup(keys) => d::StatValue::Lookup(keys),
            },
        })
        .collect()
}

fn resolution_to(r: &d::LineResolution) -> LineResolution {
    match r {
        d::LineResolution::Pending => LineResolution::Pending,
        d::LineResolution::Excluded(e) => LineResolution::Excluded(match e {
            d::Exclusion::NotSelected => Exclusion::NotSelected,
            d::Exclusion::Custom => Exclusion::Custom,
            d::Exclusion::Section => Exclusion::Section,
        }),
        d::LineResolution::Resolved { result, adapted } => LineResolution::Resolved {
            result: match result {
                d::Resolved::Stats(s) => Resolved::Stats(stats_to(s)),
                d::Resolved::Typed(t) => Resolved::Typed(match t {
                    d::TypedResolution::GrantedSkill { source, gem, level } => {
                        TypedResolution::GrantedSkill {
                            source: source.clone(),
                            gem: gem.as_str().to_owned(),
                            level: *level,
                        }
                    }
                    d::TypedResolution::ItemProperty { property, values } => {
                        TypedResolution::ItemProperty {
                            property: property.clone(),
                            values: values
                                .iter()
                                .map(|v| PropertyValue {
                                    slot: v.slot,
                                    text: v.text.clone(),
                                })
                                .collect(),
                        }
                    }
                    d::TypedResolution::CatalystQuality {
                        quality_type,
                        stats,
                        quality,
                    } => TypedResolution::CatalystQuality {
                        quality_type: quality_type.clone(),
                        stats: stats_to(stats),
                        quality: quality.clone(),
                    },
                    d::TypedResolution::CustomNote => TypedResolution::CustomNote,
                    d::TypedResolution::CustomPattern { pattern, value } => {
                        TypedResolution::CustomPattern {
                            pattern: pattern.clone(),
                            value: value.clone(),
                        }
                    }
                }),
            },
            adapted: adapted.map(|a| match a {
                d::Adapted::CaseFolded => Adapted::CaseFolded,
                d::Adapted::SplitSection => Adapted::SplitSection,
            }),
        },
        d::LineResolution::Continues { line } => LineResolution::Continues {
            line: index_to(*line),
        },
        d::LineResolution::Unresolved(u) => LineResolution::Unresolved(match u {
            d::UnresolvedReason::NoTemplate => UnresolvedReason::NoTemplate,
            d::UnresolvedReason::NoVariantFits => UnresolvedReason::NoVariantFits,
            d::UnresolvedReason::OnlyShadowed => UnresolvedReason::OnlyShadowed,
            d::UnresolvedReason::Ambiguous(c) => UnresolvedReason::Ambiguous(
                c.iter()
                    .map(|s| s.iter().map(|s| s.as_str().to_owned()).collect())
                    .collect(),
            ),
            d::UnresolvedReason::NoBase(n) => UnresolvedReason::NoBase(match n {
                d::NoBase::Unnamed(r) => NoBase::Unnamed(r.clone()),
                d::NoBase::NotFound => NoBase::NotFound,
                d::NoBase::ScopeConflict => NoBase::ScopeConflict,
            }),
        }),
    }
}

fn resolution_from(r: LineResolution) -> Result<d::LineResolution, DtoError> {
    Ok(match r {
        LineResolution::Pending => d::LineResolution::Pending,
        LineResolution::Excluded(e) => d::LineResolution::Excluded(match e {
            Exclusion::NotSelected => d::Exclusion::NotSelected,
            Exclusion::Custom => d::Exclusion::Custom,
            Exclusion::Section => d::Exclusion::Section,
        }),
        LineResolution::Resolved { result, adapted } => d::LineResolution::Resolved {
            result: match result {
                Resolved::Stats(s) => d::Resolved::Stats(stats_from(s)),
                Resolved::Typed(t) => d::Resolved::Typed(match t {
                    TypedResolution::GrantedSkill { source, gem, level } => {
                        d::TypedResolution::GrantedSkill {
                            source,
                            gem: d::GemId::new(gem),
                            level,
                        }
                    }
                    TypedResolution::ItemProperty { property, values } => {
                        d::TypedResolution::ItemProperty {
                            property,
                            values: values
                                .into_iter()
                                .map(|v| d::PropertyValue {
                                    slot: v.slot,
                                    text: v.text,
                                })
                                .collect(),
                        }
                    }
                    TypedResolution::CatalystQuality {
                        quality_type,
                        stats,
                        quality,
                    } => d::TypedResolution::CatalystQuality {
                        quality_type,
                        stats: stats_from(stats),
                        quality,
                    },
                    TypedResolution::CustomNote => d::TypedResolution::CustomNote,
                    TypedResolution::CustomPattern { pattern, value } => {
                        d::TypedResolution::CustomPattern { pattern, value }
                    }
                }),
            },
            adapted: adapted.map(|a| match a {
                Adapted::CaseFolded => d::Adapted::CaseFolded,
                Adapted::SplitSection => d::Adapted::SplitSection,
            }),
        },
        LineResolution::Continues { line } => d::LineResolution::Continues {
            line: index_from(line)?,
        },
        LineResolution::Unresolved(u) => d::LineResolution::Unresolved(match u {
            UnresolvedReason::NoTemplate => d::UnresolvedReason::NoTemplate,
            UnresolvedReason::NoVariantFits => d::UnresolvedReason::NoVariantFits,
            UnresolvedReason::OnlyShadowed => d::UnresolvedReason::OnlyShadowed,
            UnresolvedReason::Ambiguous(c) => d::UnresolvedReason::Ambiguous(
                c.into_iter()
                    .map(|s| s.into_iter().map(d::StatId::new).collect())
                    .collect(),
            ),
            UnresolvedReason::NoBase(n) => d::UnresolvedReason::NoBase(match n {
                NoBase::Unnamed(r) => d::NoBase::Unnamed(r),
                NoBase::NotFound => d::NoBase::NotFound,
                NoBase::ScopeConflict => d::NoBase::ScopeConflict,
            }),
        }),
    })
}

// ---- config ----------------------------------------------------------------

fn config_to(c: &d::Config) -> Config {
    Config {
        active: c.active.map(|i| i.0),
        sets: ordered_to(&c.sets, |set| ConfigSet {
            id: set.id.0,
            title: set.title.clone(),
            inputs: ordered_to(&set.inputs, |i| ConfigInput {
                name: i.name.clone(),
                value: match &i.value {
                    d::ConfigValue::Boolean(b) => ConfigValue::Boolean(*b),
                    d::ConfigValue::Number(n) => ConfigValue::Number(sourced_to(n, number_to)),
                    d::ConfigValue::String(s) => ConfigValue::String(s.clone()),
                },
                kind: match i.kind {
                    d::ConfigInputKind::Input => ConfigInputKind::Input,
                    d::ConfigInputKind::Placeholder => ConfigInputKind::Placeholder,
                },
                lines: lines_to(&i.lines),
                attributes: attrs_to(&i.attributes),
            }),
            custom_modifiers: set
                .custom_modifiers
                .iter()
                .map(|b| CustomModifierBlock {
                    title: b.title.clone(),
                    enabled: b.enabled,
                    text: b.text.clone(),
                    lines: lines_to(&b.lines),
                    attributes: attrs_to(&b.attributes),
                })
                .collect(),
            attributes: attrs_to(&set.attributes),
        }),
        layout: match c.layout {
            d::ConfigLayout::Sets => ConfigLayout::Sets,
            d::ConfigLayout::Flat => ConfigLayout::Flat,
        },
        attributes: attrs_to(&c.attributes),
    }
}

fn lines_to(lines: &[d::CustomModifierLine]) -> Vec<CustomModifierLine> {
    lines
        .iter()
        .map(|l| CustomModifierLine {
            text: l.text.clone(),
            resolution: resolution_to(&l.resolution),
        })
        .collect()
}

fn lines_from(lines: Vec<CustomModifierLine>) -> Result<Vec<d::CustomModifierLine>, DtoError> {
    lines
        .into_iter()
        .map(|l| {
            Ok(d::CustomModifierLine {
                text: l.text,
                resolution: resolution_from(l.resolution)?,
            })
        })
        .collect()
}

fn config_from(c: Config) -> Result<d::Config, DtoError> {
    Ok(d::Config {
        active: c.active.map(d::ConfigSetId),
        sets: ordered_from(c.sets, |set| {
            Ok(d::ConfigSet {
                id: d::ConfigSetId(set.id),
                title: set.title,
                inputs: ordered_from(set.inputs, |i| {
                    Ok(d::ConfigInput {
                        name: i.name,
                        value: match i.value {
                            ConfigValue::Boolean(b) => d::ConfigValue::Boolean(b),
                            ConfigValue::Number(n) => {
                                d::ConfigValue::Number(sourced_from(n, number_from)?)
                            }
                            ConfigValue::String(s) => d::ConfigValue::String(s),
                        },
                        kind: match i.kind {
                            ConfigInputKind::Input => d::ConfigInputKind::Input,
                            ConfigInputKind::Placeholder => d::ConfigInputKind::Placeholder,
                        },
                        lines: lines_from(i.lines)?,
                        attributes: attrs_from(i.attributes),
                    })
                })?,
                custom_modifiers: set
                    .custom_modifiers
                    .into_iter()
                    .map(|b| {
                        Ok(d::CustomModifierBlock {
                            title: b.title,
                            enabled: b.enabled,
                            text: b.text,
                            lines: lines_from(b.lines)?,
                            attributes: attrs_from(b.attributes),
                        })
                    })
                    .collect::<Result<_, DtoError>>()?,
                attributes: attrs_from(set.attributes),
            })
        })?,
        layout: match c.layout {
            ConfigLayout::Sets => d::ConfigLayout::Sets,
            ConfigLayout::Flat => d::ConfigLayout::Flat,
        },
        attributes: attrs_from(c.attributes),
    })
    .and_then(|config: d::Config| {
        config.check_custom_lines().map_err(DtoError::Domain)?;
        config.check_layout().map_err(DtoError::Domain)?;
        Ok(config)
    })
}
