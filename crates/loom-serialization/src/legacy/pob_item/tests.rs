use super::*;

fn kinds(item: &Item) -> Vec<&'static str> {
    item.lines.iter().map(|l| l.kind.name()).collect()
}

fn modifier(item: &Item, text: &str) -> Modifier {
    item.modifiers()
        .find(|m| m.text == text)
        .cloned()
        .unwrap_or_else(|| panic!("no modifier {text:?} in {item:#?}"))
}

#[test]
fn read_should_split_a_rare_item_into_header_specs_and_sections() {
    let text = "
        Rarity: RARE
        Storm Thirst
        Thicket Bow
        Hunter Item
        Crafted: true
        Prefix: {range:0.5}LocalIncreaseSocketedGemLevel1
        Quality: 20
        LevelReq: 60
        Implicits: 2
        {crafted}Quality does not increase Physical Damage
        {range:0.5}(15-25)% increased Attack Speed
        +1 to Level of Socketed Gems
        {tags:chaos}{crafted}{range:0.5}+(18-20)% to Chaos Damage over Time Multiplier
        {fractured}19% increased Attack Speed
        Corrupted
    ";
    let item = read(text);
    assert_eq!(item.rarity, Some(Rarity::Rare));
    assert_eq!(item.name.as_deref(), Some("Storm Thirst"));
    assert_eq!(item.base.as_deref(), Some("Thicket Bow"));
    assert_eq!(item.implicits, Some(2));
    assert_eq!(
        kinds(&item),
        [
            "rarity",
            "name",
            "base",
            "influence",
            "spec",
            "spec",
            "spec",
            "spec",
            "spec",
            "modifier",
            "modifier",
            "modifier",
            "modifier",
            "modifier",
            "flag"
        ]
    );
    let sections: Vec<Section> = item.modifiers().map(|m| m.section).collect();
    assert_eq!(
        sections,
        [
            Section::Enchant,
            Section::Implicit,
            Section::Explicit,
            Section::Explicit,
            Section::Explicit
        ]
    );
    let m = modifier(&item, "+(18-20)% to Chaos Damage over Time Multiplier");
    assert!(m.has("crafted"));
    assert_eq!(m.value("range"), Some("0.5"));
    assert_eq!(m.value("tags"), Some("chaos"));
    assert!(modifier(&item, "19% increased Attack Speed").has("fractured"));
    assert!(item.modifiers().all(|m| m.selected));
}

#[test]
fn read_should_select_lines_of_the_selected_variant_and_alt_variants() {
    let text = "
        Rarity: UNIQUE
        Watcher's Eye
        Prismatic Jewel
        Variant: A
        Variant: B
        Variant: C
        Variant: D
        Selected Variant: 2
        Has Alt Variant: true
        Selected Alt Variant: 4
        Implicits: 0
        {variant:1}Line one
        {variant:2,3}Line two
        {variant:4}Line four
        Always here
    ";
    let item = read(text);
    let selected: Vec<(&str, bool)> = item
        .modifiers()
        .map(|m| (m.text.as_str(), m.selected))
        .collect();
    assert_eq!(
        selected,
        [
            ("Line one", false),
            ("Line two", true),
            ("Line four", true),
            ("Always here", true)
        ]
    );
}

#[test]
fn read_should_default_the_variant_to_the_last_like_parse_raw() {
    let text = "Rarity: UNIQUE\nX\nY\nVariant: Old\nVariant: New\nImplicits: 0\n{variant:1}Old line\n{variant:2}New line\n";
    let item = read(text);
    let selected: Vec<bool> = item.modifiers().map(|m| m.selected).collect();
    assert_eq!(selected, [false, true]);
    // A selection past the end is clamped to the last variant.
    let clamped = read(&text.replace("Implicits", "Selected Variant: 9\nImplicits"));
    let selected: Vec<bool> = clamped.modifiers().map(|m| m.selected).collect();
    assert_eq!(selected, [false, true]);
}

#[test]
fn read_should_not_select_variant_lines_of_an_item_without_variants() {
    let item = read("Rarity: UNIQUE\nX\nY\nImplicits: 0\n{variant:1}Line\n");
    assert!(!modifier(&item, "Line").selected);
}

#[test]
fn read_should_mark_variant_base_lines_and_their_selection() {
    let text = "
        Rarity: UNIQUE
        Some Unique
        Coral Ring
        Variant: Pre 3.0
        Variant: Current
        Selected Variant: 2
        {variant:1}Coral Amulet
        {variant:2}Coral Ring
        Implicits: 0
        +10 to maximum Life
    ";
    let item = read(text);
    let bases: Vec<(String, bool)> = item
        .lines
        .iter()
        .filter_map(|l| match &l.kind {
            LineKind::VariantBase { base, selected } => Some((base.clone(), *selected)),
            _ => None,
        })
        .collect();
    assert_eq!(
        bases,
        [
            ("Coral Amulet".to_owned(), false),
            ("Coral Ring".to_owned(), true)
        ]
    );
}

#[test]
fn read_should_consume_an_unknown_rarity_line_and_keep_the_default() {
    let item = read("Rarity: GEM\nN\nB\nImplicits: 0\n");
    assert_eq!(item.rarity, None);
    assert_eq!(item.name.as_deref(), Some("N"));
    assert_eq!(item.base.as_deref(), Some("B"));
}

#[test]
fn read_should_take_the_last_variant_prefix_and_select_nothing_for_an_empty_one() {
    let item = read(
        "Rarity: UNIQUE\nN\nB\nVariant: A\nVariant: B\nSelected Variant: 2\nImplicits: 0\n{variant:2}{variant:1}Last wins\n{variant:}Empty\n",
    );
    assert!(!modifier(&item, "Last wins").selected);
    assert!(!modifier(&item, "Empty").selected);
    assert_eq!(modifier(&item, "Empty").variants(), Some(vec![]));
}

#[test]
fn unsupported_features_should_name_each_feature_the_reader_skips() {
    let item = read(
        "Rarity: UNIQUE\nEnergy Blade\nB\nVersion: 1\nHas Variants: x\nImplicits: 0\n{group:1}{modGroup:a}A\n{ Prefix Modifier \"X\" }\n12(10-15)% increased Y\n(Reminder text)\nRequirements:\n--------\nPlain line\n",
    );
    let got = unsupported_features(&item);
    for feature in [
        "energy-blade-base",
        "spec/Version",
        "spec/Has Variants",
        "prefix/group",
        "prefix/modGroup",
        "advanced-copy-line",
        "value-then-range",
        "reminder-text",
        "requirements-line",
        "body-separator",
    ] {
        assert_eq!(got.get(feature), Some(&1), "{feature}: {got:?}");
    }
    assert!(
        unsupported_features(&read("Rarity: RARE\nN\nB\nImplicits: 0\n+(1-2) to X\n")).is_empty()
    );
}

#[test]
fn read_should_read_a_repeated_base_line_as_a_base() {
    let item = read(
        "Rarity: UNIQUE\nN\nTimeless Jewel\nVariant: A\nSelected Variant: 1\nTimeless Jewel\nImplicits: 0\n",
    );
    assert_eq!(
        kinds(&item),
        ["rarity", "name", "base", "spec", "spec", "base", "spec"]
    );
}

#[test]
fn read_should_take_the_base_from_the_name_line_for_magic_items() {
    let item = read(
        "Rarity: MAGIC\nHeated Thicket Bow of Skill\nImplicits: 0\n10% increased Attack Speed\n",
    );
    assert_eq!(item.rarity, Some(Rarity::Magic));
    assert_eq!(item.base, None);
    assert_eq!(item.name.as_deref(), Some("Heated Thicket Bow of Skill"));
    assert_eq!(kinds(&item), ["rarity", "name", "spec", "modifier"]);
}

#[test]
fn read_should_not_take_a_base_line_for_unidentified_items() {
    let item = read("Rarity: RARE\nThicket Bow\nUnidentified\nImplicits: 0\n");
    assert_eq!(item.base, None);
    assert_eq!(kinds(&item), ["rarity", "name", "flag", "spec"]);
}

#[test]
fn read_should_count_crafted_implicits_as_enchants_until_the_count_is_reached() {
    let text = "Rarity: RARE\nA\nB\nImplicits: 3\n{crafted}C1\nI1\n{crafted}C2\n{crafted}C3\nE1\n{enchant}N1\n{scourge}S1\n{crucible}K1\nRequires Class Witch\n";
    let item = read(text);
    let got: Vec<(&str, Section)> = item
        .modifiers()
        .map(|m| (m.text.as_str(), m.section))
        .collect();
    assert_eq!(
        got,
        [
            ("C1", Section::Enchant),
            ("I1", Section::Implicit),
            ("C2", Section::Enchant),
            ("C3", Section::Explicit),
            ("E1", Section::Explicit),
            ("N1", Section::Enchant),
            ("S1", Section::Scourge),
            ("K1", Section::Crucible),
            ("Requires Class Witch", Section::ClassRequirement),
        ]
    );
}

#[test]
fn read_should_list_unknown_prefixes_and_strip_flag_words_and_unscalable() {
    let item = read(
        "Rarity: RARE\nA\nB\nImplicits: 0\n{custom}{eater}{exarch}{mutated}{weird:1}Text here\nAdds 1 (implicit) thing - Unscalable Value\nKeeps (Capital) words\n",
    );
    let m = modifier(&item, "Text here");
    assert!(m.has("custom") && m.has("eater") && m.has("exarch") && m.has("mutated"));
    assert_eq!(m.unknown_prefixes(), ["weird"]);
    let m = modifier(&item, "Adds 1 thing");
    assert!(m.has("implicit") && m.has("unscalable"));
    assert!(m.unknown_prefixes().is_empty());
    assert_eq!(m.section, Section::Implicit);
    let _ = modifier(&item, "Keeps (Capital) words");
}

#[test]
fn strip_prefixes_should_remove_groups_anywhere_and_keep_an_unclosed_brace() {
    let (text, prefixes) = strip_prefixes("{range:0.5}(1-2) to {tags:a,b}x {open");
    assert_eq!(text, "(1-2) to x {open");
    let keys: Vec<(&str, Option<&str>)> = prefixes
        .iter()
        .map(|p| (p.key.as_str(), p.value.as_deref()))
        .collect();
    assert_eq!(keys, [("range", Some("0.5")), ("tags", Some("a,b"))]);
}

#[test]
fn read_should_report_lines_it_cannot_place_before_the_implicits() {
    let item = read(
        "Rarity: RARE\nA\nB\nSomething odd\nWeird Key: 1\nFoil Unique (Verdant)\nImplicits: 0\n",
    );
    assert_eq!(
        kinds(&item),
        [
            "rarity",
            "name",
            "base",
            "unrecognized",
            "spec-unknown",
            "flag",
            "spec"
        ]
    );
}

#[test]
fn read_should_make_every_line_unrecognized_or_header_without_implicits() {
    let item = read("Rarity: RARE\nA\nB\n+10 to maximum Life\n");
    assert_eq!(kinds(&item), ["rarity", "name", "base", "unrecognized"]);
    assert_eq!(item.implicits, None);
}

#[test]
fn read_should_default_to_unique_without_a_rarity_line() {
    let item = read("Some Name\nSome Base\nImplicits: 0\nLine\n");
    assert_eq!(item.rarity, None);
    assert_eq!(item.base.as_deref(), Some("Some Base"));
    assert_eq!(kinds(&item), ["name", "base", "spec", "modifier"]);
}

#[test]
fn magic_base_should_prefer_exact_then_longest_and_report_ties() {
    let bases = ["Ring", "Coral Ring", "Two-Stone Ring", "Thicket Bow", "Bow"];
    assert_eq!(
        magic_base("Superior Thicket Bow", bases),
        BaseMatch::One("Thicket Bow".to_owned())
    );
    assert_eq!(
        magic_base("Heated Coral Ring of Ire", bases),
        BaseMatch::One("Coral Ring".to_owned())
    );
    assert_eq!(magic_base("Nothing", bases), BaseMatch::None);
    assert_eq!(
        magic_base("Abc Def", ["Abc", "Def", "Abc"]),
        BaseMatch::Tie(vec!["Abc".to_owned(), "Def".to_owned()])
    );
}

#[test]
fn base_name_should_remove_parse_raw_prefixes() {
    assert_eq!(base_name("Superior Synthesised Ring"), "Ring");
    assert_eq!(base_name("Vestigial Ring"), "Ring");
    assert_eq!(base_name("Ring"), "Ring");
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(n).unwrap()).unwrap()
    }
}

#[test]
fn read_should_never_panic_and_should_give_every_line_a_kind() {
    let pieces = [
        "Rarity: ",
        "RARE",
        "UNIQUE",
        "MAGIC",
        "\n",
        "\n",
        "Implicits: ",
        "2",
        "{",
        "}",
        "{variant:",
        "1,2",
        "{range:",
        "0.5",
        "Variant: ",
        "Selected Variant: ",
        "Has Alt Variant: true",
        "Selected Alt Variant Two: ",
        " (",
        ")",
        "implicit",
        "crafted",
        ": ",
        "Requires ",
        "Class",
        "Corrupted",
        "Foil Unique",
        " Item",
        "Hunter",
        "--------",
        "é",
        "語",
        "-",
        "+",
        "(1-2)",
        " - Unscalable Value",
        "Item Class: ",
        " ",
        "\t",
        "\r",
    ];
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for _ in 0..20_000 {
        let n = rng.below(40);
        let text: String = (0..n).map(|_| pieces[rng.below(pieces.len())]).collect();
        let item = read(&text);
        let non_blank = text.split('\n').filter(|l| !l.trim().is_empty()).count();
        assert_eq!(item.lines.len(), non_blank, "{text:?}");
    }
}
