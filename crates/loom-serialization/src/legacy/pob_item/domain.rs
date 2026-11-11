//! Maps an item read from PoB's text onto the domain item (Phase 2 plan,
//! step 2a). Every line is kept with its source text and its kind; PoB-only
//! rules stay here:
//! - PoB's scourge, crucible, and class-requirement lists become
//!   [`ModSection::Other`] named `scourge`, `crucible`, `class-requirement`
//!   ([`Section::name`]);
//! - a modifier line is [`Exclusion::NotSelected`] when the variant
//!   selection leaves it out, else [`Exclusion::Custom`] with a `{custom}`
//!   prefix, else [`Exclusion::Section`] in the class-requirement list, else
//!   [`LineResolution::Pending`] (resolution runs later);
//! - an influence line `<display> Item` names the influence by `<display>`;
//! - only the alternate variants the item has (`Has Alt Variant ...`) are
//!   kept in the selection, as `ParseRaw` reads only those.

use loom_domain::{
    AltVariant, Exclusion, HeaderLine, InfluenceId, ItemBody, ItemLine, LineKind as DomainKind,
    LineResolution, ModSection, ModifierLine, Prefix as DomainPrefix, Rarity as DomainRarity,
    VariantSelection,
};

use super::{Item, Line, LineKind, Modifier, Rarity, Section};

/// The domain section of a PoB section.
#[must_use]
pub fn section(section: Section) -> ModSection {
    match section {
        Section::Enchant => ModSection::Enchant,
        Section::Implicit => ModSection::Implicit,
        Section::Explicit => ModSection::Explicit,
        Section::Scourge | Section::Crucible | Section::ClassRequirement => {
            ModSection::Other(section.name().to_owned())
        }
    }
}

/// The domain rarity of a PoB rarity.
#[must_use]
pub const fn rarity(rarity: Rarity) -> DomainRarity {
    match rarity {
        Rarity::Normal => DomainRarity::Normal,
        Rarity::Magic => DomainRarity::Magic,
        Rarity::Rare => DomainRarity::Rare,
        Rarity::Unique => DomainRarity::Unique,
        Rarity::Relic => DomainRarity::Relic,
    }
}

/// The domain item of an item read from PoB's text.
#[must_use]
pub fn to_domain(item: &Item) -> ItemBody {
    ItemBody {
        item_class: item.lines.iter().find_map(|l| match l.kind {
            LineKind::ItemClass => Some(after_key(&l.raw, "Item Class").to_owned()),
            _ => None,
        }),
        rarity: item.rarity.map(rarity),
        name: item.name.clone(),
        base: item.base.clone(),
        implicits: item.implicits,
        variants: VariantSelection {
            count: item.variants.count,
            selected: item.variants.selected,
            alternates: (0_u8..)
                .zip(item.variants.alt.iter())
                .filter(|(_, (has, _))| *has)
                .map(|(index, (_, selected))| AltVariant {
                    index,
                    selected: *selected,
                })
                .collect(),
        },
        lines: item.lines.iter().map(line).collect(),
    }
}

fn line(line: &Line) -> ItemLine {
    let kind = match &line.kind {
        LineKind::ItemClass => DomainKind::Header(HeaderLine::ItemClass),
        LineKind::Rarity => DomainKind::Header(HeaderLine::Rarity),
        LineKind::Separator => DomainKind::Header(HeaderLine::Separator),
        LineKind::Name => DomainKind::Header(HeaderLine::Name),
        LineKind::Base => DomainKind::Header(HeaderLine::Base),
        LineKind::Spec { key, known } => DomainKind::Spec {
            key: key.clone(),
            value: after_key(&line.raw, key).to_owned(),
            known: *known,
        },
        LineKind::VariantBase { base, selected } => DomainKind::VariantBase {
            base: base.clone(),
            selected: *selected,
        },
        LineKind::Influence => DomainKind::Influence(InfluenceId::new(
            line.raw.strip_suffix(" Item").unwrap_or(&line.raw),
        )),
        LineKind::Flag => DomainKind::Flag,
        LineKind::Modifier(m) => DomainKind::Modifier(modifier(m)),
        LineKind::Unrecognized => DomainKind::Unrecognized,
    };
    ItemLine {
        source: line.raw.clone(),
        kind,
    }
}

fn modifier(m: &Modifier) -> ModifierLine {
    let resolution = if !m.selected {
        LineResolution::Excluded(Exclusion::NotSelected)
    } else if m.has("custom") {
        LineResolution::Excluded(Exclusion::Custom)
    } else if m.section == Section::ClassRequirement {
        LineResolution::Excluded(Exclusion::Section)
    } else {
        LineResolution::Pending
    };
    ModifierLine {
        text: m.text.clone(),
        prefixes: m
            .prefixes
            .iter()
            .map(|p| DomainPrefix::new(p.key.clone(), p.value.as_deref()))
            .collect(),
        section: section(m.section),
        selected: m.selected,
        resolution,
    }
}

/// The value of a `Key: value` or `Key value` line: the text after `key`
/// and any `:` and spaces; the whole line when it does not start with
/// `key`.
fn after_key<'a>(raw: &'a str, key: &str) -> &'a str {
    raw.strip_prefix(key)
        .map_or(raw, |rest| rest.trim_start_matches(':').trim_start())
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use loom_domain::{Exclusion, LineKind, LineResolution, ModSection, Rarity};

    use super::to_domain;
    use crate::legacy::pob_item::read;

    const RING: &str = "Item Class: Rings
Rarity: RARE
Doom Loop
Two-Stone Ring
Unique ID: abc
Item Level: 84
Shaper Item
Variant: One
Variant: Two
Selected Variant: 1
Has Alt Variant: true
Selected Alt Variant: 1
Implicits: 1
{crafted}+12% to Fire and Cold Resistances
{variant:2}+5 to Strength
{custom}My own note
{scourge}+10 to Dexterity
+20 to maximum Life
Corrupted";

    fn body() -> loom_domain::ItemBody {
        to_domain(&read(RING))
    }

    #[test]
    fn to_domain_should_keep_every_source_line_in_order() {
        let sources: Vec<String> = body().lines.into_iter().map(|l| l.source).collect();
        let expected: Vec<&str> = RING.lines().collect();
        assert_eq!(sources, expected);
    }

    #[test]
    fn to_domain_should_keep_the_line_kind_names() {
        let item = read(RING);
        let pob: Vec<&str> = item.lines.iter().map(|l| l.kind.name()).collect();
        let domain = body();
        let mapped: Vec<&str> = domain.lines.iter().map(|l| l.kind.name()).collect();
        assert_eq!(mapped, pob);
    }

    #[test]
    fn to_domain_should_read_the_header_fields() {
        let b = body();
        assert_eq!(
            (
                b.item_class.as_deref(),
                b.rarity,
                b.name.as_deref(),
                b.base.as_deref(),
                b.implicits
            ),
            (
                Some("Rings"),
                Some(Rarity::Rare),
                Some("Doom Loop"),
                Some("Two-Stone Ring"),
                Some(1)
            )
        );
    }

    #[test]
    fn to_domain_should_keep_spec_values() {
        let b = body();
        assert_eq!(
            b.lines[5].kind,
            LineKind::Spec {
                key: "Item Level".to_owned(),
                value: "84".to_owned(),
                known: true,
            }
        );
    }

    #[test]
    fn to_domain_should_name_influences_by_display() {
        let names: Vec<String> = body().influences().map(ToString::to_string).collect();
        assert_eq!(names, ["Shaper"]);
    }

    #[test]
    fn to_domain_should_keep_alternates_the_item_has() {
        let v = body().variants;
        assert_eq!(
            (
                v.count,
                v.selected,
                v.alternates.len(),
                v.alternates[0].index,
                v.alternates[0].selected
            ),
            (2, Some(1), 1, 0, Some(1))
        );
    }

    #[test]
    fn to_domain_should_set_exclusions_and_sections() {
        let found: Vec<(String, LineResolution)> = body()
            .modifiers()
            .map(|(_, m)| (m.section.name().to_owned(), m.resolution.clone()))
            .collect();
        assert_eq!(
            found,
            [
                ("enchant".to_owned(), LineResolution::Pending),
                (
                    "explicit".to_owned(),
                    LineResolution::Excluded(Exclusion::NotSelected)
                ),
                (
                    "explicit".to_owned(),
                    LineResolution::Excluded(Exclusion::Custom)
                ),
                ("scourge".to_owned(), LineResolution::Pending),
                ("explicit".to_owned(), LineResolution::Pending),
            ]
        );
    }

    #[test]
    fn to_domain_should_exclude_class_requirement_lines_as_section() {
        let b = to_domain(&read(
            "Rarity: RARE
A
B
Implicits: 0
Requires Class Witch",
        ));
        let (_, m) = b.modifiers().next().unwrap();
        assert_eq!(
            (&m.section, &m.resolution),
            (
                &ModSection::Other("class-requirement".to_owned()),
                &LineResolution::Excluded(Exclusion::Section)
            )
        );
    }
}
