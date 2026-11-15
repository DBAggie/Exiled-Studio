use super::*;
use crate::legacy::pob_item::{self, Section};

#[test]
fn other_non_items_should_be_refused() {
    assert_eq!(
        convert("Item Class: Stackable Currency\nRarity: Currency\nChaos Orb\n--------\n"),
        Err(CopyError::NotAnItem("Currency".to_owned()))
    );
    assert_eq!(convert("  \n\n"), Err(CopyError::Empty));
    assert_eq!(
        convert("Some text\n--------\nmore"),
        Err(CopyError::NoRarity)
    );
}

#[test]
fn values_should_follow_parse_raws_rules() {
    // A named affix keeps its rolled values.
    assert_eq!(
        values("Adds 27(24-28) to 45(42-49) Physical Damage", true),
        "Adds 27 to 45 Physical Damage"
    );
    // One roll in range: the range with its fraction.
    assert_eq!(
        values("+40(30-40) to maximum Life", false),
        "{range:1}+(30-40) to maximum Life"
    );
    assert_eq!(
        values("31(20-40)% increased Charge Recovery", false),
        "{range:0.55}(20-40)% increased Charge Recovery"
    );
    // Out of range (a legacy roll): the value.
    assert_eq!(
        values("+45(30-40) to maximum Life", false),
        "+45 to maximum Life"
    );
    // Rolls at different fractions: every value.
    assert_eq!(
        values("Adds 40(40-55) to 59(56-65) Physical Damage", false),
        "Adds 40 to 59 Physical Damage"
    );
    // current(base) for a fixed value.
    assert_eq!(values("Grants 12(10) Life", false), "Grants 12 Life");
    // Enum ranges without digits go; `(...)` stays.
    assert_eq!(
        values(
            "Bathed in the blood of 5000(2000-10000) sacrificed in the name of Xibaqua (Ahuana-Doryani)",
            false
        ),
        "{range:0.375}Bathed in the blood of (2000-10000) sacrificed in the name of Xibaqua"
    );
    assert_eq!(
        values("Commanded under Akoya(...)", false),
        "Commanded under Akoya(...)"
    );
    // Decimals and a negative bound.
    assert_eq!(
        values("Regenerate 22.2(16.1-24) Life per second", false),
        "{range:0.772152}Regenerate (16.1-24) Life per second"
    );
    // The unscalable suffix stays after the values.
    assert_eq!(
        values("Historic - Unscalable Value", false),
        "Historic - Unscalable Value"
    );
}

#[test]
fn headers_should_read_as_parse_raw_reads_them() {
    let h = read_header(&clean(
        "{ Unique Modifier \u{2014} Life  \u{2014} 20% Increased }",
    ));
    assert_eq!(h.prefix, "{tags:life}");
    assert_eq!(h.info.kind, "Unique");
    assert_eq!(h.info.tags, ["Life"]);
    let h = read_header(&clean("{ Foulborn Unique Modifier }"));
    assert_eq!(h.prefix, "{mutated}");
    let h = read_header(&clean(
        "{ Searing Exarch Implicit Modifier (Grand) \u{2014} Elemental, Fire, Resistance }",
    ));
    assert_eq!(
        h.prefix,
        "{exarch}{implicit}{tags:elemental,fire,resistance}"
    );
    assert!(!h.named);
    let h = read_header(&clean(
        "{ Fractured Suffix Modifier \"of the Lightning\" (Tier: 2) \u{2014} Mana }",
    ));
    assert_eq!(h.prefix, "{fractured}{suffix}{tags:mana}");
    assert!(h.named);
    assert_eq!(h.info.tier, Some(2));
}

#[test]
fn reminders_should_match_parse_raws_test() {
    assert!(is_reminder(
        "(Conquered Passive Skills cannot be modified by other Jewels)"
    ));
    assert!(is_reminder(
        "(10% of Damage is taken from Mana before Life)"
    ));
    assert!(is_reminder("(10 of them)"));
    assert!(!is_reminder("(20-30)% increased Damage"));
    assert!(!is_reminder("Adds 1 to 2 Damage"));
}

#[test]
fn a_plain_copy_should_read_its_explicits_and_flavour_by_stage() {
    let text = "Item Class: Gloves\nRarity: Unique\nSoul Ascension\nCarnal Mitts\n--------\nEvasion Rating: 260 (augmented)\n--------\nItem Level: 87\n--------\nAttacks have +0.7% to Critical Strike Chance (implicit)\n--------\n+26% to Chaos Resistance\n--------\nA flavour line.\n--------\nCorrupted\n";
    let copy = convert(text).unwrap();
    let item = pob_item::read(&copy.text);
    assert_eq!(item.implicits, Some(1));
    let sections: Vec<Section> = item.modifiers().map(|m| m.section).collect();
    assert_eq!(sections, [Section::Implicit, Section::Explicit]);
    assert_eq!(copy.flavour, ["A flavour line."]);
    assert!(copy.text.contains("Corrupted\n"));
}

#[test]
fn ggg_markup_and_dashes_should_be_cleaned() {
    assert_eq!(clean("<rgb(255,0,0)>{Red} text"), "{Red} text");
    assert_eq!(
        clean("Grants [Onslaught|Onslaught] for [Mana]"),
        "Grants Onslaught for Mana"
    );
    assert_eq!(clean("a \u{2212}5 \u{2013} b"), "a -5 - b");
    assert_eq!(clean("M\u{e4}lstr\u{f6}m"), "Malstrom");
}
