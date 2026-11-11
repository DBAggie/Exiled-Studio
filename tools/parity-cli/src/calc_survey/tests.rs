//! Tests of `calc-survey` on the synthetic PoE1 bundle
//! (`crates/loom-data/tests/fixtures/bundle-poe1`) with skill, passive, and
//! base rows of the real skill types and jewel bases added, and small PoB
//! builds written here.

use std::collections::BTreeSet;
use std::path::Path;

use loom_data::bundle::Bundle;
use loom_domain::Build;
use serde_json::{Value, json};

use super::main_skill::{Problem, main_skill, type_expression};
use super::rules::{Archetype, Exclusion, Kind, Reason, Scope};
use super::{Input, Inputs, Request, Tables, inventory, not_carried_passives, run, verdict};
use crate::resolve_corpus::{BundleInputs, Tool};

const STEAM_BUILD: u64 = 26_530_219;

fn row<T: serde::de::DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}

/// An active skill, its granted effect, gem, and gem effect.
fn add_skill(b: &mut Bundle, id: &str, types: &[&str], manual: bool, minion: &[&str]) {
    let s = b.skills.as_mut().unwrap();
    s.active_skills.push(row(json!({
        "id": id.to_lowercase(), "name": id, "types": types, "minion_types": minion,
        "weapon_classes": [], "manually_cast": manual
    })));
    s.granted_effects.push(row(json!({
        "id": id, "active_skill": id.to_lowercase(), "cast_time": 1000,
        "stat_sets": [], "levels": [{"level": 1, "cost_multiplier": 100}], "support": false
    })));
    s.gems.push(row(
        json!({"base_item": format!("Gem/{id}"), "effects": [id]}),
    ));
    s.gem_effects.push(row(
        json!({"id": id, "name": id, "granted_effect": id, "tags": []}),
    ));
}

/// A support, its gem, and gem effect.
fn add_support(b: &mut Bundle, id: &str, allowed: &[&str], added: &[&str], excluded: &[&str]) {
    let s = b.skills.as_mut().unwrap();
    s.granted_effects.push(row(json!({
        "id": id, "support": true, "allowed_types": allowed, "added_types": added,
        "excluded_types": excluded, "cast_time": 0, "stat_sets": [], "levels": [{"level": 1, "cost_multiplier": 100}]
    })));
    s.gems.push(row(
        json!({"base_item": format!("Gem/{id}"), "effects": [id]}),
    ));
    s.gem_effects.push(row(
        json!({"id": id, "name": id, "granted_effect": id, "tags": []}),
    ));
}

/// The synthetic bundle with the rows the tests use.
fn bundle() -> Bundle {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/loom-data/tests/fixtures/bundle-poe1");
    let mut b = crate::load_bundle(&dir).unwrap();
    add_skill(
        &mut b,
        "Bolt",
        &["Spell", "Damage", "Projectile", "Totemable"],
        true,
        &[],
    );
    add_skill(&mut b, "Strike", &["Attack", "Melee"], true, &[]);
    add_skill(&mut b, "Aura", &["Spell", "Aura", "Buff"], true, &[]);
    add_skill(&mut b, "Brand", &["Spell", "Damage", "Brand"], true, &[]);
    add_skill(
        &mut b,
        "Zombie",
        &["Spell", "Damage", "Minion"],
        true,
        &["Attack"],
    );
    add_skill(&mut b, "Echo", &["Spell", "Damage"], false, &[]);
    add_skill(&mut b, "Vaal", &["Spell", "Damage", "Vaal"], true, &[]);
    add_support(
        &mut b,
        "SupportTotem",
        &["Spell", "Totemable", "AND"],
        &["SummonsTotem"],
        &[],
    );
    add_support(&mut b, "SupportTrap", &["Spell"], &["Trapped"], &[]);
    add_support(&mut b, "SupportMine", &["Spell"], &["RemoteMined"], &[]);
    add_support(&mut b, "SupportTrigger", &["Spell"], &["Triggered"], &[]);
    add_support(
        &mut b,
        "SupportMirage",
        &["MirageArcherCanUse"],
        &["Duration"],
        &[],
    );
    add_support(&mut b, "SupportNoSpell", &[], &["Fire"], &["Spell"]);
    // A Vaal gem: the Vaal skill, then the plain one.
    let s = b.skills.as_mut().unwrap();
    s.gems.push(row(
        json!({"base_item": "Gem/VaalBolt", "effects": ["VaalBoltEffect", "BoltAlt"]}),
    ));
    s.gem_effects.push(row(json!({
        "id": "VaalBoltEffect", "name": "", "granted_effect": "Vaal",
        "additional_granted_effects": ["Bolt"], "tags": []
    })));
    s.gem_effects.push(row(
        json!({"id": "BoltAlt", "name": "", "granted_effect": "Strike", "tags": []}),
    ));
    let p = b.passives.as_mut().unwrap();
    for (node, keystone, socket) in [
        (300, false, false),
        (301, true, false),
        (400, false, true),
        (401, false, true),
        (500, false, false),
    ] {
        p.passives.push(row(json!({
            "id": format!("p{node}"), "node": node, "name": "", "keystone": keystone,
            "jewel_socket": socket, "stats": [{"stat": format!("stat_{node}"), "value": 1}]
        })));
    }
    for (id, name, tags) in [
        (
            super::rules::TIMELESS_JEWEL_BASE,
            "Timeless Jewel",
            vec!["jewel"],
        ),
        (
            "Metadata/Items/Jewels/JewelPassiveTreeExpansionLarge",
            "Large Cluster Jewel",
            vec!["jewel", "expansion_jewel_large"],
        ),
        (
            "Metadata/Items/Jewels/JewelStr",
            "Crimson Jewel",
            vec!["jewel"],
        ),
    ] {
        b.base_items.push(row(
            json!({"id": id, "name": name, "domain": 11, "item_class": "Jewel", "tags": tags}),
        ));
    }
    b
}

fn scope(exclusions: &[&str]) -> Scope {
    let mechanics: Vec<Value> = exclusions
        .iter()
        .map(|e| json!({"id": e, "reason": "test"}))
        .collect();
    Scope::read(
        json!({
            "format": 1, "game": "poe1", "steam_build": STEAM_BUILD, "status": "proposed",
            "tree_version": "3_29", "tree_evidence": "test", "minimum_per_skill": 2,
            "skills": {"spell": null, "attack": null}, "excluded_mechanics": mechanics
        })
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}

/// A build: `groups` are `<Skill>` elements of skill set 1; `spec` the
/// `<Spec>` attributes and children; `items` the `<Items>` content.
fn xml(main_group: u32, groups: &str, spec: &str, items: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<PathOfBuilding>
	<Build level="90" className="Witch" ascendClassName="None" mainSocketGroup="{main_group}" bandit="None"/>
	<Tree activeSpec="1">
		{spec}
	</Tree>
	<Skills activeSkillSet="1">
		<SkillSet id="1">
{groups}
		</SkillSet>
	</Skills>
	<Items activeItemSet="1">
{items}
	</Items>
	<Config activeConfigSet="1">
		<ConfigSet id="1">
			<Input name="enemyIsBoss" string="Pinnacle"/>
		</ConfigSet>
	</Config>
</PathOfBuilding>
"#
    )
}

const SPEC: &str = r#"<Spec treeVersion="3_29" nodes="300,301"/>"#;

fn gem(id: &str) -> String {
    format!(
        r#"<Gem gemId="Gem/{id}" variantId="{id}" skillId="{id}" level="20" quality="0" enabled="true"/>"#
    )
}

fn group(gems: &[&str]) -> String {
    let body: String = gems.iter().map(|g| gem(g)).collect();
    format!(r#"<Skill enabled="true" mainActiveSkill="1">{body}</Skill>"#)
}

fn import(text: &str) -> Build {
    loom_serialization::legacy::pob_xml::import(text).unwrap()
}

/// The reasons of a build with one group of `gems`.
fn reasons_of(b: &Bundle, gems: &[&str]) -> Vec<Reason> {
    reasons(b, &xml(1, &group(gems), SPEC, ""), &scope(&[]))
}

fn reasons(b: &Bundle, text: &str, scope: &Scope) -> Vec<Reason> {
    let t = Tables::new(b).unwrap();
    let build = import(text);
    let main = main_skill(&build, &t);
    let names: Vec<&str> = b.base_items.iter().map(|x| x.name.as_str()).collect();
    let not_carried: BTreeSet<i64> = [500].into();
    verdict(&build, &main, &t, scope, &not_carried, &names)
}

#[test]
fn a_plain_spell_and_attack_are_in_scope() {
    let b = bundle();
    assert_eq!(reasons_of(&b, &["Bolt"]), vec![]);
    assert_eq!(reasons_of(&b, &["Strike"]), vec![]);
    let t = Tables::new(&b).unwrap();
    let m = main_skill(&import(&xml(1, &group(&["Strike"]), SPEC, "")), &t).unwrap();
    assert_eq!(Kind::of(&m.types), Some(Kind::Attack));
}

#[test]
fn each_archetype_rule() {
    let b = bundle();
    let one = |gems: &[&str], rule: Archetype| {
        assert_eq!(
            reasons_of(&b, gems),
            vec![Reason::Archetype(rule)],
            "{gems:?}"
        );
    };
    one(&["Bolt", "SupportTotem"], Archetype::Totem);
    one(&["Bolt", "SupportTrap"], Archetype::Trap);
    one(&["Bolt", "SupportMine"], Archetype::Mine);
    one(&["Bolt", "SupportTrigger"], Archetype::Triggered);
    one(&["Brand"], Archetype::Brand);
    one(&["Echo"], Archetype::NotManuallyCast);
    one(&["Aura"], Archetype::NotHit);
    // Minion: the Minion type and minion types.
    one(&["Zombie"], Archetype::Minion);
}

#[test]
fn mirage_needs_a_skill_it_supports() {
    let mut b = bundle();
    // The support applies only to skills of the type it requires.
    assert_eq!(reasons_of(&b, &["Strike", "SupportMirage"]), vec![]);
    let s = b.skills.as_mut().unwrap();
    s.active_skills
        .iter_mut()
        .find(|a| a.id == "strike")
        .unwrap()
        .types
        .push("MirageArcherCanUse".to_owned());
    assert_eq!(
        reasons_of(&b, &["Strike", "SupportMirage"]),
        vec![Reason::Archetype(Archetype::Mirage)]
    );
}

#[test]
fn supports_apply_by_type_expression() {
    let b = bundle();
    let t = Tables::new(&b).unwrap();
    // Totem requires Spell AND Totemable: Strike is neither.
    let m = main_skill(
        &import(&xml(
            1,
            &group(&["Strike", "SupportTotem", "SupportNoSpell"]),
            SPEC,
            "",
        )),
        &t,
    )
    .unwrap();
    let ids = |v: &[&loom_data::bundle::skills::GrantedEffect]| {
        v.iter().map(|g| g.id.clone()).collect::<Vec<_>>()
    };
    assert_eq!(ids(&m.supports), ["SupportNoSpell"]);
    assert_eq!(ids(&m.unapplied), ["SupportTotem"]);
    assert!(
        m.types.contains("Fire"),
        "an applied support adds its types"
    );
    // Excluded types: SupportNoSpell excludes spells.
    let m = main_skill(
        &import(&xml(1, &group(&["Bolt", "SupportNoSpell"]), SPEC, "")),
        &t,
    )
    .unwrap();
    assert!(m.supports.is_empty());
}

#[test]
fn type_expressions_are_postfix() {
    let types: BTreeSet<String> = ["Spell", "Totemable"].map(str::to_owned).into();
    let e = |v: &[&str]| {
        type_expression(
            &v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
            &types,
            &[],
        )
    };
    assert!(e(&["Spell"]));
    assert!(!e(&["Attack"]));
    assert!(e(&["Spell", "Totemable", "AND"]));
    assert!(!e(&["Spell", "Attack", "AND"]));
    assert!(e(&["Attack", "Spell", "OR"]));
    assert!(e(&["Attack", "NOT"]));
    assert!(!e(&["Spell", "NOT"]));
    // Any true value left on the stack matches (PoB's rule).
    assert!(e(&["Attack", "Spell"]));
    assert!(!e(&[]));
    let minion = ["Attack".to_owned()];
    assert!(type_expression(&["Attack".to_owned()], &types, &minion));
}

#[test]
fn main_group_and_skill_are_clamped_and_follow_the_active_set() {
    let b = bundle();
    let t = Tables::new(&b).unwrap();
    let two = format!("{}{}", group(&["Bolt"]), group(&["Strike"]));
    let pick = |text: &str| main_skill(&import(text), &t).map(|m| (m.group, m.effect.id.clone()));
    assert_eq!(pick(&xml(2, &two, SPEC, "")), Ok((2, "Strike".to_owned())));
    assert_eq!(pick(&xml(9, &two, SPEC, "")), Ok((2, "Strike".to_owned())));
    assert_eq!(pick(&xml(0, &two, SPEC, "")), Ok((1, "Bolt".to_owned())));
    // The active skill set decides the groups.
    let sets = xml(1, &group(&["Bolt"]), SPEC, "").replace(
        r#"<Skills activeSkillSet="1">"#,
        &format!(
            r#"<Skills activeSkillSet="2"><SkillSet id="2">{}</SkillSet>"#,
            group(&["Strike"])
        ),
    );
    assert_eq!(pick(&sets), Ok((1, "Strike".to_owned())));
    // mainActiveSkill is clamped to the group's active skills.
    let two_actives =
        group(&["Bolt", "Strike"]).replace(r#"mainActiveSkill="1""#, r#"mainActiveSkill="5""#);
    assert_eq!(
        pick(&xml(1, &two_actives, SPEC, "")),
        Ok((1, "Strike".to_owned()))
    );
    // A disabled gem is not an active skill.
    let disabled =
        group(&["Bolt", "Strike"]).replacen(r#"enabled="true"/>"#, r#"enabled="false"/>"#, 1);
    assert_eq!(
        pick(&xml(1, &disabled, SPEC, "")),
        Ok((1, "Strike".to_owned()))
    );
}

#[test]
fn gems_are_found_by_variant_or_skill() {
    let b = bundle();
    let t = Tables::new(&b).unwrap();
    let pick = |g: &str| {
        let text = xml(
            1,
            &format!(r#"<Skill enabled="true">{g}</Skill>"#),
            SPEC,
            "",
        );
        main_skill(&import(&text), &t).map(|m| m.effect.id.clone())
    };
    // variantId picks the gem effect; skillId alone picks by granted effect.
    let vaal = |v: &str, extra: &str| {
        format!(r#"<Gem gemId="Gem/VaalBolt" {v} level="1" quality="0" enabled="true" {extra}/>"#)
    };
    assert_eq!(
        pick(&vaal(r#"variantId="BoltAlt""#, "")),
        Ok("Strike".to_owned())
    );
    assert_eq!(
        pick(&vaal(r#"skillId="Strike""#, "")),
        Ok("Strike".to_owned())
    );
    // A skill an item grants has no gemId.
    assert_eq!(
        pick(r#"<Gem skillId="Bolt" level="1" quality="0" enabled="true"/>"#),
        Ok("Bolt".to_owned())
    );
    // Not in the bundle.
    assert_eq!(
        pick(r#"<Gem gemId="Gem/Gone" variantId="Gone" level="1" quality="0" enabled="true"/>"#),
        Err(Problem::UnknownGem {
            gem: "Gem/Gone".to_owned()
        })
    );
    assert_eq!(
        pick(&vaal(r#"variantId="Missing""#, "")),
        Err(Problem::UnknownGem {
            gem: "Gem/VaalBolt".to_owned()
        })
    );
}

#[test]
fn global_effects_switched_off_make_the_pick_uncertain_only_when_it_matters() {
    let b = bundle();
    let t = Tables::new(&b).unwrap();
    let pick = |main: u32, globals: &str| {
        let g = format!(
            r#"<Skill enabled="true" mainActiveSkill="{main}"><Gem gemId="Gem/VaalBolt" variantId="VaalBoltEffect" level="1" quality="0" enabled="true" {globals}/></Skill>"#
        );
        main_skill(&import(&xml(1, &g, SPEC, "")), &t).map(|m| m.effect.id.clone())
    };
    // Default: enableGlobal1 on, enableGlobal2 off, so Bolt may or may not count.
    assert_eq!(pick(1, ""), Ok("Vaal".to_owned()));
    assert_eq!(pick(2, ""), Err(Problem::Uncertain));
    assert_eq!(pick(2, r#"enableGlobal2="true""#), Ok("Bolt".to_owned()));
    // Vaal off: index 2 is Bolt whether or not Vaal counts (clamped).
    assert_eq!(
        pick(2, r#"enableGlobal1="false" enableGlobal2="true""#),
        Ok("Bolt".to_owned())
    );
    assert_eq!(
        pick(1, r#"enableGlobal1="false" enableGlobal2="true""#),
        Err(Problem::Uncertain)
    );
}

#[test]
fn no_main_skill_problems() {
    let b = bundle();
    let t = Tables::new(&b).unwrap();
    let none = import(
        &xml(1, "", SPEC, "")
            .replace(r#"<SkillSet id="1">"#, "")
            .replace("</SkillSet>", ""),
    );
    assert_eq!(main_skill(&none, &t).err(), Some(Problem::NoSkillSet));
    let empty = import(&xml(1, r#"<Skill enabled="true"/>"#, SPEC, ""));
    assert_eq!(main_skill(&empty, &t).err(), Some(Problem::NoActiveSkill));
    let supports_only = import(&xml(1, &group(&["SupportTrap"]), SPEC, ""));
    assert_eq!(
        main_skill(&supports_only, &t).err(),
        Some(Problem::NoActiveSkill)
    );
    assert_eq!(
        reasons(
            &b,
            &xml(1, r#"<Skill enabled="true"/>"#, SPEC, ""),
            &scope(&[])
        ),
        vec![Reason::MainSkill("no-active-skill")]
    );
}

#[test]
fn tree_version_and_unknown_passives() {
    let b = bundle();
    let g = group(&["Bolt"]);
    let r = |spec: &str| reasons(&b, &xml(1, &g, spec, ""), &scope(&[]));
    assert_eq!(
        r(r#"<Spec treeVersion="3_28" nodes="300"/>"#),
        vec![Reason::TreeVersion]
    );
    assert_eq!(r(r#"<Spec nodes="300"/>"#), vec![Reason::TreeVersion]);
    assert_eq!(
        r(r#"<Spec treeVersion="3_29" nodes="300,65600"/>"#),
        vec![Reason::UnknownPassive]
    );
    assert_eq!(
        r(r#"<Spec treeVersion="3_29" nodes="300" masteryEffects="{300,9}"/>"#),
        vec![Reason::UnknownPassive]
    );
    // The synthetic bundle's mastery effect hash is 4242.
    assert_eq!(
        r(r#"<Spec treeVersion="3_29" nodes="300" masteryEffects="{300,4242}"/>"#),
        vec![]
    );
    // The active spec is the one read.
    let two = r#"<Spec treeVersion="3_28" nodes="300"/></Tree><Tree activeSpec="2">"#;
    assert_eq!(r(two).len(), 1);
    let active_second = xml(
        1,
        &g,
        r#"<Spec treeVersion="3_28" nodes="300"/><Spec treeVersion="3_29" nodes="300"/>"#,
        "",
    )
    .replace(r#"<Tree activeSpec="1">"#, r#"<Tree activeSpec="2">"#);
    assert_eq!(reasons(&b, &active_second, &scope(&[])), vec![]);
}

fn jewel(id: u32, name: &str, base: &str) -> String {
    format!("<Item id=\"{id}\">\nRarity: UNIQUE\n{name}\n{base}\n</Item>")
}

#[test]
fn each_excluded_mechanic_when_allocated_and_listed() {
    let b = bundle();
    let g = group(&["Bolt"]);
    let all = scope(&["timeless-jewel", "cluster-jewel", "not-carried-passive"]);
    let items = format!(
        "{}{}{}<ItemSet id=\"1\"/>",
        jewel(1, "Glorious Vanity", "Timeless Jewel"),
        jewel(2, "Voices", "Large Cluster Jewel"),
        jewel(3, "Plain", "Crimson Jewel")
    );
    let spec = |nodes: &str, socket: u32, item: u32| {
        format!(
            r#"<Spec treeVersion="3_29" nodes="{nodes}"><Sockets><Socket nodeId="{socket}" itemId="{item}"/></Sockets></Spec>"#
        )
    };
    let r = |spec: &str, scope: &Scope| reasons(&b, &xml(1, &g, spec, &items), scope);
    assert_eq!(
        r(&spec("300,400", 400, 1), &all),
        vec![Reason::Excluded(Exclusion::TimelessJewel)]
    );
    assert_eq!(
        r(&spec("300,400", 400, 2), &all),
        vec![Reason::Excluded(Exclusion::ClusterJewel)]
    );
    assert_eq!(r(&spec("300,400", 400, 3), &all), vec![]);
    // A socket that is not allocated does not count.
    assert_eq!(r(&spec("300", 400, 1), &all), vec![]);
    // An allocated passive a not-carried rule names.
    assert_eq!(
        r(&spec("300,500", 400, 0), &all),
        vec![Reason::Excluded(Exclusion::NotCarriedPassive)]
    );
    // A mechanic the scope file does not list does not exclude.
    assert_eq!(r(&spec("300,400,500", 400, 1), &scope(&[])), vec![]);
    assert_eq!(
        r(&spec("300,400,500", 400, 1), &scope(&["cluster-jewel"])),
        vec![]
    );
}

#[test]
fn scope_file_checks() {
    let base = || {
        json!({
            "format": 1, "game": "poe1", "steam_build": 1, "status": "proposed",
            "tree_version": "3_29", "tree_evidence": "e", "minimum_per_skill": 3,
            "skills": {"spell": null, "attack": "Strike"},
            "excluded_mechanics": [{"id": "timeless-jewel", "reason": "r"}]
        })
    };
    let read = |v: Value| Scope::read(v.to_string().as_bytes());
    let s = read(base()).unwrap();
    assert_eq!(s.skills.of(Kind::Attack), Some("Strike"));
    assert!(s.excludes(Exclusion::TimelessJewel) && !s.excludes(Exclusion::ClusterJewel));
    let with = |k: &str, v: Value| {
        let mut b = base();
        b[k] = v;
        read(b)
    };
    assert!(with("format", json!(2)).is_err());
    assert!(with("game", json!("poe2")).is_err());
    assert!(with("status", json!("done")).is_err());
    assert!(with("minimum_per_skill", json!(0)).is_err());
    assert!(with("tree_evidence", json!(" ")).is_err());
    assert!(
        with(
            "excluded_mechanics",
            json!([{"id": "unknown", "reason": "r"}])
        )
        .is_err()
    );
    assert!(
        with(
            "excluded_mechanics",
            json!([{"id": "cluster-jewel", "reason": ""}])
        )
        .is_err()
    );
    assert!(
        with(
            "excluded_mechanics",
            json!([{"id": "cluster-jewel", "reason": "r"}, {"id": "cluster-jewel", "reason": "r"}])
        )
        .is_err()
    );
    assert!(with("extra", json!(1)).is_err());
}

#[test]
fn not_carried_passives_come_from_not_carried_passive_rules() {
    let rules = json!({
        "steam_build": 7,
        "rules": [
            {"id": "a", "category": "not-carried", "table": "passives", "keys": ["1", "2"]},
            {"id": "b", "category": "patch-delta", "table": "passives", "keys": ["3"]},
            {"id": "c", "category": "not-carried", "table": "skills", "keys": ["4"]}
        ]
    })
    .to_string();
    assert_eq!(
        not_carried_passives(rules.as_bytes(), 7).unwrap(),
        [1, 2].into()
    );
    assert!(not_carried_passives(rules.as_bytes(), 8).is_err());
    let bad = rules.replace(r#"["1","2"]"#, r#"["x"]"#);
    assert!(not_carried_passives(bad.as_bytes(), 7).is_err());
}

fn repo(path: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap()
}

#[test]
fn the_committed_scope_and_rules_files_read() {
    let scope = Scope::read(&repo("docs/phase-3/calc-scope-poe1.json")).unwrap();
    let rules = repo("docs/phase-3/data-parity-rules-poe1.json");
    let nodes = not_carried_passives(&rules, scope.steam_build).unwrap();
    // P3-2: 147 bloodline and 15 buff or skill passives.
    assert_eq!(nodes.len(), 162);
}

#[test]
fn the_committed_slice_fields_are_well_formed() {
    let v: Value = serde_json::from_slice(&repo("docs/phase-3/slice-fields.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&repo("tools/oracle/manifest.json")).unwrap();
    let oracle: BTreeSet<&str> = manifest["adapter"]["breakdowns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_str().unwrap())
        .collect();
    assert_eq!(v["format"], 1);
    let mut ids = BTreeSet::new();
    let mut breakdowns = BTreeSet::new();
    for f in v["fields"].as_array().unwrap() {
        let id = f["id"].as_str().unwrap();
        assert!(ids.insert(id), "{id} twice");
        assert!(f["pob_key"].is_string(), "{id}");
        assert!(
            matches!(f["applies"].as_str(), Some("always" | "attack" | "spell")),
            "{id}"
        );
        match f["comparison"]["rule"].as_str() {
            Some("exact") => {}
            Some("abs" | "rel") => {
                assert!(f["comparison"]["tolerance"].is_number(), "{id}");
                assert!(
                    f["comparison"]["reason"]
                        .as_str()
                        .is_some_and(|r| !r.is_empty()),
                    "{id}"
                );
            }
            other => panic!("{id}: comparison {other:?}"),
        }
        if let Some(b) = f.get("breakdown") {
            let key = b["pob_key"].as_str().unwrap();
            assert!(
                oracle.contains(key),
                "{id}: breakdown {key} is not one the oracle writes"
            );
            assert!(
                matches!(
                    b["shape"].as_str(),
                    Some("chain" | "slots" | "min-max-total")
                ),
                "{id}"
            );
            assert!(b["source"].is_string(), "{id}");
            breakdowns.insert(key);
        }
    }
    assert_eq!(
        breakdowns, oracle,
        "every oracle breakdown has its slice field"
    );
}

#[test]
fn inventory_reads_items_in_use_jewels_skills_and_config() {
    let b = bundle();
    let t = Tables::new(&b).unwrap();
    let items = format!(
        "{}{}{}<ItemSet id=\"1\"><Slot name=\"Weapon 1\" itemId=\"1\"/><Slot name=\"Weapon 1 Swap\" itemId=\"2\"/></ItemSet>",
        jewel(1, "A", "Crimson Jewel"),
        jewel(2, "B", "Crimson Jewel"),
        jewel(3, "C", "Crimson Jewel")
    );
    let spec = r#"<Spec treeVersion="3_29" nodes="300,301,400" masteryEffects="{300,4242}"><Sockets><Socket nodeId="400" itemId="3"/><Socket nodeId="401" itemId="2"/></Sockets></Spec>"#;
    let groups = format!(
        "{}{}{}",
        group(&["Bolt", "SupportTrap"]),
        group(&["Aura"]),
        group(&["Strike"]).replace(
            r#"enabled="true" mainActiveSkill"#,
            r#"enabled="false" mainActiveSkill"#
        )
    );
    let build = import(&xml(1, &groups, spec, &items));
    let inv = inventory::inventory(&build, &t, Some(1));
    // Weapon 1 and the allocated socket's jewel; not the swap slot nor the
    // unallocated socket.
    assert_eq!(inv.items, 2);
    assert_eq!(inv.nodes, 3);
    assert_eq!(inv.keystones, ["p301".to_owned()].into());
    assert_eq!(inv.mastery_effects, 1);
    assert_eq!(inv.passive_stats.get("stat_300"), Some(&1));
    assert!(inv.passive_stats.contains_key("test_mastery_stat"));
    assert_eq!(inv.other_skills.len(), 1);
    assert_eq!(
        inv.other_skills.get("aura"),
        Some(&["Aura".to_owned()].into())
    );
    assert_eq!(inv.config, ["enemyIsBoss".to_owned()].into());
    // The second weapon set swaps the weapon slots.
    let swapped = import(&xml(
        1,
        &groups,
        spec,
        &items.replace(
            r#"<ItemSet id="1">"#,
            r#"<ItemSet id="1" useSecondWeaponSet="true">"#,
        ),
    ));
    let inv2 = inventory::inventory(&swapped, &t, Some(1));
    assert_eq!(inv2.items, 2);
    assert!(inv.size() > 0);
}

fn request<'a, 'r, 'd, 'b>(
    resolver: &'r loom_parser::import::Resolver<'d, 'b>,
    data: &'d loom_parser::import::BundleData<'b>,
    tables: &'a Tables<'b>,
    scope: &'a Scope,
    not_carried: &'a BTreeSet<i64>,
) -> Request<'a, 'r, 'd, 'b> {
    Request {
        resolver,
        data,
        tables,
        scope,
        not_carried,
        anonymous: false,
        inputs: Inputs {
            tool: Tool::default(),
            bundle: BundleInputs {
                game: "poe1".to_owned(),
                steam_build: Some(STEAM_BUILD),
                index_sha256: String::new(),
            },
            scope_sha256: String::new(),
            data_rules_sha256: String::new(),
            anonymous: false,
        },
    }
}

#[test]
fn run_ranks_candidates_and_checks_the_chosen_minimum() {
    let b = bundle();
    let t = Tables::new(&b).unwrap();
    // The item pipeline builds from the bundle as validated; the rows the
    // tests add are read through the tables only.
    let plain = crate::load_bundle(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/loom-data/tests/fixtures/bundle-poe1"),
    )
    .unwrap();
    let data = loom_parser::import::BundleData::new(&plain);
    let resolver = loom_parser::import::Resolver::new(&data).unwrap();
    let not_carried = BTreeSet::new();
    let input = |path: &str, text: String| Input {
        path: path.to_owned(),
        bytes: text.into_bytes(),
    };
    let bolt = xml(1, &group(&["Bolt"]), SPEC, "");
    let inputs = vec![
        input("a.xml", bolt.clone()),
        input("dup.xml", bolt.clone()),
        input("b.xml", bolt.replace("level=\"90\"", "level=\"91\"")),
        input("c.xml", xml(1, &group(&["Strike"]), SPEC, "")),
        input(
            "d.xml",
            xml(
                1,
                &group(&["Strike"]),
                r#"<Spec treeVersion="3_28" nodes="300"/>"#,
                "",
            ),
        ),
        input("e.xml", xml(1, &group(&["Bolt", "SupportTotem"]), SPEC, "")),
        input("f.xml", "not xml".to_owned()),
    ];
    let mut s = scope(&[]);
    let (summary, report) = run(&inputs, &request(&resolver, &data, &t, &s, &not_carried));
    assert_eq!(
        (summary.builds, summary.duplicates, summary.in_scope),
        (6, 1, 3)
    );
    assert!(summary.pass, "nothing chosen yet");
    let spell = &summary.candidates["spell"];
    assert_eq!(spell[0].skill, "Bolt");
    assert_eq!(spell[0].in_scope, ["a.xml", "b.xml"]);
    // The totem build fails an archetype rule, so it is no candidate.
    assert_eq!(spell[0].archetype_builds, 2);
    let attack = &summary.candidates["attack"];
    assert_eq!(
        (attack[0].archetype_builds, attack[0].in_scope.len()),
        (2, 1)
    );
    assert_eq!(summary.reasons.get("tree:version"), Some(&1));
    assert_eq!(summary.reasons.get("archetype:totem"), Some(&1));
    assert!(summary.reasons.keys().any(|k| k.starts_with("import:")));
    assert_eq!(summary.states.get("in-scope"), Some(&3));
    assert_eq!(report.inventories.len(), 6);
    assert_eq!(report.reach_in_scope.builds, 3);

    // Chosen skills: Bolt has 2 (the minimum), Strike 1.
    s.skills.spell = Some("Bolt".to_owned());
    let (summary, report) = run(&inputs, &request(&resolver, &data, &t, &s, &not_carried));
    assert!(summary.pass);
    assert_eq!(report.reach_in_scope.builds, 2);
    s.skills.attack = Some("Strike".to_owned());
    let (summary, _) = run(&inputs, &request(&resolver, &data, &t, &s, &not_carried));
    assert!(!summary.pass);
    assert!(!summary.chosen["attack"].meets_minimum);
    assert!(summary.chosen["spell"].meets_minimum);
    // Two runs give the same bytes.
    let again = run(&inputs, &request(&resolver, &data, &t, &s, &not_carried));
    assert_eq!(
        serde_json::to_string(&summary).unwrap(),
        serde_json::to_string(&again.0).unwrap()
    );
}
