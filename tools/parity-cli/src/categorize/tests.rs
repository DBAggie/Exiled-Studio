use super::*;

#[test]
fn word_key_should_replace_values_and_drop_case_and_punctuation() {
    assert_eq!(
        word_key("+(17-19)% to Fire Damage over Time Multiplier"),
        "# to fire damage over time multiplier"
    );
    assert_eq!(
        word_key(" # % to Fire Damage over Time Multiplier"),
        "# to fire damage over time multiplier"
    );
    assert_eq!(word_key("Lasts 3.70 Seconds"), "lasts # seconds");
    assert_eq!(
        word_key("Consumes 15 of 40 Charges on use"),
        "consumes # of # charges on use"
    );
    assert_eq!(word_key("Assassin's Mark"), "assassin s mark");
    assert_eq!(word_key("% increased Damage"), "increased damage");
    assert_eq!(word_key(""), "");
}

#[test]
fn client_template_should_turn_placeholders_into_hashes() {
    assert_eq!(
        word_key(&client_template("Uses level {1} {0}")),
        "uses level #"
    );
    assert_eq!(
        word_key(&client_template(
            "While a Unique Enemy is in your Presence, {0}"
        )),
        "while a unique enemy is in your presence #"
    );
}

#[test]
fn glob_should_let_a_hash_take_one_or_more_words() {
    let p = ["uses", "level", "#"];
    assert!(glob(&p, &["uses", "level", "#", "call", "the", "pyre"]));
    assert!(glob(&p, &["uses", "level", "#"]));
    assert!(!glob(&p, &["uses", "level"]));
    assert!(!glob(&p, &["uses", "levels", "#"]));
    assert!(glob(
        &["#", "charges"],
        &["currently", "has", "#", "charges"]
    ));
}

#[test]
fn quoted_values_should_skip_keys_and_assignments() {
    let text = "\t[\"Ancestral Cry\"] = {\n\t\t\"Attacks deal 35% increased Damage \",\n\
                name = \"Totem Damage\", stats = { \"4% increased maximum Life\" },";
    assert_eq!(
        quoted_values(text),
        [
            "Attacks deal 35% increased Damage ",
            "4% increased maximum Life"
        ]
    );
}

#[test]
fn unique_lines_should_read_blocks_without_prefixes() {
    let text = "return {\n[[\nThe Anvil\nAmber Amulet\nVariant: Pre 1.3.0\nImplicits: 1\n\
                {variant:1}{tags:x}10% Chance to Block\n]],\n}";
    // Name, base, and spec lines are not mod text.
    assert_eq!(unique_lines(text), ["10% Chance to Block"]);
}

#[test]
fn pob_text_should_read_each_kind_of_file_and_record_a_hash() {
    let files = vec![
        (
            "ModFlask.lua".to_owned(),
            "\t[\"F1\"] = { type = \"Prefix\", affix = \"x\", \"25% reduced Charges used\", level = 1 },\n".to_owned(),
        ),
        ("ModCache.lua".to_owned(), "c[\"Never read\"]={}".to_owned()),
        ("Uniques/ring.lua".to_owned(), "[[\nName\nRing\n+1 to Sockets\n]]".to_owned()),
        ("Gems.lua".to_owned(), "\"Not read\"".to_owned()),
    ];
    let p = PobText::read("v1", files);
    assert!(p.keys.contains("# reduced charges used"));
    assert!(p.keys.contains("# to sockets"));
    assert!(!p.keys.contains("never read") && !p.keys.contains("not read"));
    assert_eq!(p.source.files.len(), 2);
    assert_eq!(p.source.sha256.len(), 64);
}

fn game(pob: bool) -> GameText {
    let mut descriptions: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    descriptions
        .entry("# to maximum life".to_owned())
        .or_default()
        .insert("item".to_owned(), 7);
    descriptions
        .entry("# increased monster damage".to_owned())
        .or_default()
        .insert("map".to_owned(), 9);
    let rule = |id: &str, kind, words: &str| ClientRule {
        id: id.to_owned(),
        kind,
        words: words.split(' ').map(str::to_owned).collect(),
    };
    let mut pob_text = PobText::default();
    pob_text.source.tag = "v2.4.0".to_owned();
    pob_text
        .keys
        .insert("immune to curses during flask effect".to_owned());
    GameText {
        descriptions,
        wild: Vec::new(),
        client: vec![
            rule(
                "ItemDisplayFlaskBuffDurationNSeconds",
                ClientKind::Property,
                "lasts # seconds",
            ),
            rule(
                "InfluenceStatConditionPresenceUniqueMonster",
                ClientKind::Modifier,
                "while a unique enemy is in your presence #",
            ),
        ],
        pob: if pob { vec![pob_text] } else { Vec::new() },
        colour_code_in_game_text: false,
        compositions: Vec::new(),
        compositions_missing: Vec::new(),
    }
}

fn cat(game: &GameText, outcome: &str, text: &str) -> Category {
    let scope: BTreeSet<String> = ["item".to_owned()].into();
    categorize(
        game,
        LineFacts {
            outcome,
            text,
            detail: &[],
            scope: &scope,
            split_with: None,
        },
    )
}

#[test]
fn categorize_should_apply_each_rule() {
    let g = game(true);
    let t = "unresolved/no-template";
    let name = |o: &str, text: &str| cat(&g, o, text).name;
    assert_eq!(name(t, "^1Note to self"), "special-case/author-note");
    assert_eq!(
        name("unknown-base/tie", "+1 to maximum Life"),
        "special-case/unknown-base"
    );
    assert_eq!(
        name("ambiguous", "12% increased Speed"),
        "parser-gap/ambiguous"
    );
    assert_eq!(
        name("unresolved/only-shadowed", "x"),
        "parser-gap/only-shadowed"
    );
    assert_eq!(name(t, "Memory Strands: 63"), "special-case/item-property");
    assert_eq!(
        cat(&g, t, "Note: buy a better one").evidence,
        "pob-note-spec"
    );
    assert_eq!(name(t, "Lasts 3.70 Seconds"), "special-case/item-property");
    assert_eq!(
        name(
            t,
            "While a Unique Enemy is in your Presence, 11% increased Cooldown Recovery Rate"
        ),
        "parser-gap/client-string"
    );
    assert_eq!(name(t, "+132 to Maximum Life"), "parser-gap/near-miss");
    assert_eq!(
        name(t, "12% increased Monster Damage"),
        "data-gap/other-scope"
    );
    assert_eq!(
        name(t, "Immune to Curses during Flask effect"),
        "data-gap/legacy-text"
    );
    assert_eq!(
        name(t, "Use jeweler orbs at the bench"),
        "special-case/author-note"
    );
    assert_eq!(name("unrecognized-line", "x"), "uncategorized");
    assert_eq!(cat(&g, t, "Use jeweler orbs").evidence, "no-game-text");
    assert!(cat(&g, t, "Use jeweler orbs").excluded());
    assert!(!cat(&g, t, "+132 to Maximum Life").excluded());
    assert!(cat(&g, t, "Immune to Curses during Flask effect").excluded());
    // Only the named categories are excluded, not the `data-gap/` family.
    for name in [
        "data-gap/other-scope",
        "parser-gap/near-miss",
        "uncategorized",
    ] {
        assert!(!Category::new(name, "").excluded(), "{name}");
    }
}

#[test]
fn categorize_should_tell_handler_gaps_from_near_misses() {
    let g = game(true);
    let scope = BTreeSet::new();
    let name = |reason: &str| {
        let detail = [reason.to_owned()];
        categorize(
            &g,
            LineFacts {
                outcome: "unresolved/no-variant-fits",
                text: "x",
                detail: &detail,
                scope: &scope,
                split_with: None,
            },
        )
        .name
    };
    assert_eq!(name("unknown-handler"), "parser-gap/unsupported-handler");
    assert_eq!(name("literal-mismatch"), "parser-gap/near-miss");
}

#[test]
fn categorize_should_name_a_line_split_across_sections() {
    let g = game(true);
    let scope = BTreeSet::new();
    let c = categorize(
        &g,
        LineFacts {
            outcome: "unresolved/no-template",
            text: "Added Small Passive Skills grant: 12% increased Trap Damage",
            detail: &[],
            scope: &scope,
            split_with: Some(9),
        },
    );
    assert_eq!(c.name, "parser-gap/split-section");
}

#[test]
fn categorize_should_not_call_text_a_note_without_a_pob_source() {
    let g = game(false);
    assert_eq!(
        cat(&g, "unresolved/no-template", "Use jeweler orbs").name,
        "uncategorized"
    );
}

#[test]
fn colour_codes_should_be_found_only_with_a_digit_or_x() {
    assert!(has_colour_code("a ^1b"));
    assert!(has_colour_code("^xFF0000"));
    assert!(!has_colour_code("2^n"));
}

#[test]
fn client_kind_should_match_exact_ids_and_prefixes() {
    assert_eq!(
        client_kind("ItemDisplayChargesNCharges"),
        Some(ClientKind::Property)
    );
    assert_eq!(
        client_kind("BrequelGraftSkillString"),
        Some(ClientKind::Modifier)
    );
    assert_eq!(client_kind("BrequelGraftSkillStringX"), None);
    assert_eq!(client_kind("ItemPopupCorrupted"), None);
}

#[test]
fn categorize_should_apply_the_colour_rule_to_no_template_lines_only() {
    let g = game(true);
    assert_eq!(cat(&g, "ambiguous", "^1x").name, "parser-gap/ambiguous");
    let mut tainted = game(true);
    tainted.colour_code_in_game_text = true;
    // With a colour escape in game text the rule has no evidence; the line
    // falls to the other rules (no source holds it).
    assert_eq!(
        cat(&tainted, "unresolved/no-template", "^1Buy this").evidence,
        "no-game-text"
    );
}

#[test]
fn categorize_should_need_a_number_for_a_spec_shaped_item_property() {
    let g = game(true);
    assert_eq!(
        cat(&g, "unresolved/no-template", "Quality: +20%").name,
        "special-case/item-property"
    );
    assert_eq!(
        cat(&g, "unresolved/no-template", "Quality: very high").evidence,
        "no-game-text"
    );
    assert!(numeric_value("63") && numeric_value("+20%") && numeric_value("-1.5"));
    assert!(!numeric_value("") && !numeric_value("1.") && !numeric_value("x"));
}

#[test]
fn categorize_should_let_a_lookup_placeholder_match_a_name() {
    let mut g = game(true);
    g.wild.push((
        ["grants", "level", "#", "skill"]
            .iter()
            .map(|w| (*w).to_owned())
            .collect(),
        "item".to_owned(),
        3,
    ));
    let c = cat(
        &g,
        "unresolved/no-template",
        "Grants Level 30 Reckoning Skill",
    );
    assert_eq!(
        (c.name, c.evidence.as_str()),
        ("parser-gap/near-miss", "item:3")
    );
}

#[test]
fn categorize_should_call_a_handler_gap_only_when_every_rejection_is_one() {
    let g = game(true);
    let scope = BTreeSet::new();
    let detail = ["unknown-handler".to_owned(), "literal-mismatch".to_owned()];
    let c = categorize(
        &g,
        LineFacts {
            outcome: "unresolved/no-variant-fits",
            text: "x",
            detail: &detail,
            scope: &scope,
            split_with: None,
        },
    );
    assert_eq!(c.name, "parser-gap/near-miss");
}

#[test]
fn client_kind_should_not_take_other_item_display_ids() {
    assert_eq!(client_kind("ItemDisplayArmour"), None);
    assert_eq!(client_kind("ItemDisplayRequires"), None);
}

#[test]
fn strip_braces_should_drop_closed_prefixes_only() {
    assert_eq!(strip_braces("{variant:1}{tags:x}10% Block"), "10% Block");
    assert_eq!(strip_braces("{variant:1}"), "");
    assert_eq!(strip_braces("{open 10% Block"), "{open 10% Block");
    assert_eq!(strip_braces("a {b} c"), "a {b} c");
}

#[test]
fn block_lines_should_skip_name_base_specs_and_empty_lines() {
    let block =
        "\n  The Anvil \n Amber Amulet\n\nRequires Level 45\n{variant:1}\n  +1 to Strength \n";
    let lines: Vec<String> = block_lines(block).collect();
    assert_eq!(lines, ["+1 to Strength"]);
}

#[test]
fn unique_lines_should_read_every_block_and_stop_at_an_unclosed_one() {
    let text = "[[\nA\nB\nx1\n]] [[\nC\nD\nx2\n]] [[\nE\nF\nx3\n";
    assert_eq!(unique_lines(text), ["x1", "x2"]);
}

#[test]
fn file_lines_should_choose_the_reader_by_path() {
    assert_eq!(
        file_lines("Uniques/ring.lua", "[[\nN\nB\n+1 to Sockets\n]]"),
        Some(vec!["+1 to Sockets".to_owned()])
    );
    assert_eq!(
        file_lines("EnchantmentHelmet.lua", "\"Enchant line\""),
        Some(vec!["Enchant line".to_owned()])
    );
    assert_eq!(
        file_lines("ClusterJewels.lua", "\"Cluster line\""),
        Some(vec!["Cluster line".to_owned()])
    );
    assert_eq!(file_lines("ModCache.lua", "c[\"x\"]={}"), None);
    assert_eq!(file_lines("Gems.lua", "\"x\""), None);
    // The `Uniques/` test is on the path, the others on the file name.
    assert_eq!(file_lines("Other/Gems.lua", "\"x\""), None);
}

#[test]
fn insert_keys_should_split_written_newlines_and_skip_empty_keys() {
    let mut p = PobText::default();
    p.insert_keys(&[
        "+1 to Life\\n2% Block".to_owned(),
        "12 %".to_owned(),
        String::new(),
    ]);
    let keys: Vec<&str> = p.keys.iter().map(String::as_str).collect();
    assert_eq!(keys, ["#", "# block", "# to life"]);
}

#[test]
fn is_pattern_should_need_a_placeholder_and_a_word() {
    assert!(is_pattern("grants level # skill"));
    assert!(!is_pattern("#"));
    assert!(!is_pattern("grants level skill"));
}

#[test]
fn client_rule_should_take_named_ids_with_a_literal_word() {
    let rule = client_rule("BrequelGraftSkillString", "Uses level {1} {0}").unwrap();
    assert_eq!(rule.kind, ClientKind::Modifier);
    assert_eq!(rule.words, ["uses", "level", "#"]);
    assert!(client_rule("ItemDisplayHasEffect", "{0}").is_none());
    assert!(client_rule("ItemPopupCorrupted", "Corrupted").is_none());
}

#[test]
fn outcome_category_should_leave_only_no_template_lines_to_the_text_rules() {
    let scope = BTreeSet::new();
    let facts = |outcome| LineFacts {
        outcome,
        text: "x",
        detail: &[],
        scope: &scope,
        split_with: None,
    };
    assert!(outcome_category(facts("unresolved/no-template")).is_none());
    assert_eq!(
        outcome_category(facts("unknown-base/tie")),
        Some(Category::new("special-case/unknown-base", "tie"))
    );
    assert_eq!(
        outcome_category(facts("unresolved/other")).map(|c| c.name),
        Some("uncategorized")
    );
    // No rejection reason at all is not a handler gap.
    assert_eq!(no_variant_category(&[]).name, "parser-gap/near-miss");
}

#[test]
fn spec_helpers_should_read_colour_escapes_and_property_keys() {
    assert!(starts_with_colour_code("^1x") && starts_with_colour_code("^xFF"));
    assert!(!starts_with_colour_code("^") && !starts_with_colour_code("a^1"));
    assert!(is_property_spec("Quality", "Quality: +20%"));
    assert!(is_property_spec(
        "Quality (Life Modifiers)",
        "Quality (Life Modifiers): 10"
    ));
    assert!(!is_property_spec("Quality", "Quality: high"));
    assert!(!is_property_spec("Unknown Key", "Unknown Key: 3"));
}

#[test]
fn categorize_should_put_a_split_line_before_the_note_rule() {
    let g = game(true);
    let scope = BTreeSet::new();
    let c = categorize(
        &g,
        LineFacts {
            outcome: "unresolved/no-template",
            text: "Note: x",
            detail: &[],
            scope: &scope,
            split_with: Some(2),
        },
    );
    assert_eq!(c.name, "parser-gap/split-section");
}

#[test]
fn word_key_should_keep_a_dot_after_a_number_out_of_the_number() {
    assert_eq!(word_key("5.Foo"), "# foo");
    assert_eq!(word_key("Lasts 9.70 Seconds"), "lasts # seconds");
}
