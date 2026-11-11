use loom_domain::poe1::Poe1Choices;
use loom_domain::{ConfigValue, Entry, GameSpecific, OpaqueNode};

use super::*;

const BASE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
	<Build level="90" className="Witch" ascendClassName="Elementalist" mainSocketGroup="1" bandit="None"/>
	<Tree activeSpec="1">
		<Spec treeVersion="3_29" nodes="1,2,3" masteryEffects="{2,7}">
			<URL>https://example.invalid/tree</URL>
			<Sockets>
				<Socket nodeId="5" itemId="0"/>
			</Sockets>
		</Spec>
	</Tree>
	<Skills activeSkillSet="1">
		<SkillSet id="1">
			<Skill enabled="true" slot="Weapon 1" mainActiveSkill="1">
				<Gem skillId="A" level="20" quality="0" enabled="true"/>
				<Gem skillId="B" level="20" quality="20" enabled="true"/>
			</Skill>
		</SkillSet>
	</Skills>
	<Items/>
	<Config activeConfigSet="1">
		<ConfigSet id="1" title="Default">
			<Input name="flag" boolean="true"/>
			<Input name="count" number="1.50"/>
			<Input name="choice" string="Fire"/>
		</ConfigSet>
	</Config>
</PathOfBuilding>
"#;

/// Imports, exports, and checks both round-trip properties.
fn round_trip(xml: &str) -> Build {
    let a = import(xml).unwrap();
    let out = export(&a);
    assert!(xml_equal(&out, xml).unwrap(), "export differs:\n{out}");
    assert_eq!(import(&out).unwrap(), a);
    a
}

#[test]
fn import_should_type_the_build_tree_skills_and_config() {
    let b = round_trip(BASE);
    let set = b.skills.sets.typed().next().unwrap();
    let group = set.groups.typed().next().unwrap();
    assert_eq!(
        (
            b.character.class.as_ref().map(|c| c.as_str()),
            b.character.level.as_ref().map(|l| *l.value()),
            b.passives.specs.typed().next().unwrap().nodes.value().len(),
            group.gems.typed().count(),
            b.config.sets.typed().next().unwrap().inputs.typed().count(),
        ),
        (Some("Witch"), Some(90), 3, 2, 3)
    );
}

#[test]
fn import_should_read_poe1_choices_into_game_specific() {
    let b = import(BASE).unwrap();
    assert_eq!(
        b.game_specific(),
        &GameSpecific::Poe1(Poe1Choices {
            bandit: Some("None".to_owned()),
            ..Poe1Choices::default()
        })
    );
}

#[test]
fn export_should_write_a_number_by_its_source_text() {
    let b = import(BASE).unwrap();
    let input = b
        .config
        .sets
        .typed()
        .next()
        .unwrap()
        .inputs
        .typed()
        .nth(1)
        .unwrap();
    assert!(matches!(&input.value, ConfigValue::Number(n) if n.source() == Some("1.50")));
    assert!(export(&b).contains(r#"number="1.50""#));
}

#[test]
fn unknown_attribute_should_survive() {
    round_trip(&BASE.replace("<Build level", r#"<Build futureThing="x" level"#));
}

#[test]
fn unknown_element_between_gems_should_stay_between_them() {
    let xml = BASE.replace(
        r#"<Gem skillId="B""#,
        r#"<Future kind="new"><Inner/></Future><Gem skillId="B""#,
    );
    let b = round_trip(&xml);
    let group = b
        .skills
        .sets
        .typed()
        .next()
        .unwrap()
        .groups
        .typed()
        .next()
        .unwrap();
    let shape: Vec<bool> = group
        .gems
        .entries()
        .iter()
        .map(|e| matches!(e, Entry::Typed(_)))
        .collect();
    assert_eq!(shape, [true, false, true]);
}

#[test]
fn unknown_section_comment_and_instruction_should_survive() {
    let xml = BASE
        .replace("<Items/>", "<Items/><Party>p</Party><!-- note --><?app x?>")
        .replace(
            "<PathOfBuilding>",
            "<!-- before --><PathOfBuilding future=\"1\">",
        );
    let b = round_trip(&xml);
    assert_eq!((b.prolog.len(), b.attributes.len()), (1, 1));
}

#[test]
fn typed_element_that_does_not_read_should_be_kept_whole() {
    let xml = BASE.replace(r#"skillId="A" level="20""#, r#"skillId="A" level="twenty""#);
    let b = round_trip(&xml);
    let group = b
        .skills
        .sets
        .typed()
        .next()
        .unwrap()
        .groups
        .typed()
        .next()
        .unwrap();
    assert!(matches!(
        &group.gems.entries()[0],
        Entry::Opaque(OpaqueNode::Element { name, .. }) if name == "Gem"
    ));
}

#[test]
fn malformed_xml_should_give_import_xml() {
    assert_eq!(
        import("<PathOfBuilding>").err().map(|e| e.code()),
        Some("IMPORT_XML")
    );
}

#[test]
fn another_root_should_give_import_root() {
    assert_eq!(
        import("<Other/>").err().map(|e| e.code()),
        Some("IMPORT_ROOT")
    );
}

#[test]
fn xml_equal_should_ignore_attribute_order() {
    assert!(xml_equal(r#"<a x="1" y="2"/>"#, r#"<a y="2" x="1"/>"#).unwrap());
}

#[test]
fn xml_equal_should_ignore_whitespace_between_elements() {
    assert!(xml_equal("<a>\n\t<b/>\n</a>", "<a><b/></a>").unwrap());
}

#[test]
fn xml_equal_should_compare_whitespace_in_mixed_content() {
    assert!(!xml_equal("<a>x<b/> <c/></a>", "<a>x<b/><c/></a>").unwrap());
}

#[test]
fn mixed_content_whitespace_should_survive_the_round_trip() {
    let xml = BASE.replace(
        "<Items/>",
        "<Items><Item id=\"1\">A<B/> <C/>\n</Item></Items>",
    );
    round_trip(&xml);
}

#[test]
fn empty_socket_should_have_no_item() {
    let b = import(BASE).unwrap();
    let spec = b.passives.specs.typed().next().unwrap();
    assert_eq!(spec.jewel_sockets.typed().next().unwrap().item, None);
}

#[test]
fn non_canonical_number_should_keep_its_element_whole() {
    let xml = BASE.replace(r#"<SkillSet id="1">"#, r#"<SkillSet id="+1">"#);
    let b = round_trip(&xml);
    assert_eq!(kept_whole(&b), ["Skills/SkillSet"]);
}

#[test]
fn kept_whole_should_list_typed_elements_that_did_not_read() {
    let xml = BASE.replace(r#"skillId="A" level="20""#, r#"skillId="A" level="x""#);
    let b = import(&xml).unwrap();
    assert_eq!(kept_whole(&b), ["Skills/SkillSet/Skill/Gem"]);
}

#[test]
fn kept_whole_should_be_empty_when_everything_reads() {
    assert_eq!(kept_whole(&import(BASE).unwrap()), [] as [&str; 0]);
}

#[test]
fn xml_equal_should_see_a_changed_value_or_text() {
    assert!(!xml_equal(r#"<a x="1"/>"#, r#"<a x="2"/>"#).unwrap());
    assert!(!xml_equal("<a>x</a>", "<a>y</a>").unwrap());
}

#[test]
fn normalizations_should_be_the_closed_list() {
    let names: Vec<&str> = NORMALIZATIONS.iter().map(|(n, _)| *n).collect();
    assert_eq!(names, ["attribute-order", "whitespace-between-elements"]);
}

#[test]
fn mixed_text_and_elements_should_keep_their_text() {
    let xml = BASE.replace(
        "<Items/>",
        "<Items><Item id=\"1\">\n\t\t\tRarity: RARE\n\t\t\tA\n\t\t\t<ModRange id=\"1\" range=\"0.5\"/>\n\t\t</Item></Items>",
    );
    round_trip(&xml);
}

const ITEMS: &str = r#"<Items activeItemSet="1">
		<Item id="1" variant="2">
			Rarity: RARE
			Doom Loop
			Two-Stone Ring
			Implicits: 0
			+20 to maximum Life
			<ModRange id="1" range="0.5"/>
		</Item>
		<!-- between items -->
		<Item id="2">old text<ModRange id="1" range="1"/>
			Rarity: MAGIC
			Plain Ring
		</Item>
		<ItemSet id="1" useSecondWeaponSet="false">
			<Slot name="Ring 1" itemId="1" future="x"/>
			<Slot name="Ring 2" itemId="0"/>
			<SocketIdURL nodeId="1" name="J" itemPbURL=""/>
		</ItemSet>
	</Items>"#;

fn with_items() -> String {
    BASE.replace("<Items/>", ITEMS)
}

#[test]
fn items_should_be_typed_and_round_trip() {
    let b = round_trip(&with_items());
    let first = b.items.items().next().unwrap();
    assert_eq!(
        (
            b.items.items().count(),
            b.items.sets().count(),
            first.body.lines.len(),
            first.body.name.as_deref(),
            kept_whole(&b).len(),
        ),
        (2, 1, 5, Some("Doom Loop"), 0)
    );
}

#[test]
fn item_body_should_be_the_last_non_blank_text() {
    let b = import(&with_items()).unwrap();
    let second = b.items.items().nth(1).unwrap();
    // "old text" stays in place; the item reads the later text.
    assert_eq!(
        (second.body.name.as_deref(), &second.layout.entries()[0]),
        (
            Some("Plain Ring"),
            &Entry::Opaque(OpaqueNode::Text("old text".to_owned()))
        )
    );
}

#[test]
fn empty_slot_should_have_no_item() {
    let b = import(&with_items()).unwrap();
    let set = b.items.sets().next().unwrap();
    let items: Vec<Option<u32>> = set.slots.typed().map(|s| s.item.map(|i| i.0)).collect();
    assert_eq!(items, [Some(1), None]);
}

#[test]
fn item_export_should_come_from_the_lines() {
    let mut b = import(&with_items()).unwrap();
    let mut entries: Vec<loom_domain::Entry<loom_domain::ItemsEntry>> =
        b.items.entries.entries().to_vec();
    if let Entry::Typed(loom_domain::ItemsEntry::Item(item)) = &mut entries[0] {
        item.body.lines[4].source = "+21 to maximum Life".to_owned();
    }
    b.items.entries = entries.into_iter().collect();
    let out = export(&b);
    assert!(out.contains("+21 to maximum Life") && !out.contains("+20 to maximum Life"));
}

#[test]
fn nil_main_active_skill_should_stay_an_untyped_attribute() {
    let xml = BASE.replace(r#"mainActiveSkill="1""#, r#"mainActiveSkill="nil""#);
    let b = round_trip(&xml);
    let group = b
        .skills
        .sets
        .typed()
        .next()
        .unwrap()
        .groups
        .typed()
        .next()
        .unwrap();
    assert_eq!(
        (group.main_active_skill.is_none(), kept_whole(&b).len()),
        (true, 0)
    );
}
