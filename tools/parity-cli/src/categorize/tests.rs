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
fn word_key_should_keep_a_dot_after_a_number_out_of_the_number() {
    assert_eq!(word_key("5.Foo"), "# foo");
    assert_eq!(word_key("Lasts 9.70 Seconds"), "lasts # seconds");
}
