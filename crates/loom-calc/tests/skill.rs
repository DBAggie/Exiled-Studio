//! The skill model (Phase 3 plan, step 9a) over the synthetic PoE1 bundle:
//! the main skill as PoB picks it, its supports, and each gem instance's
//! stats at its level and quality (`loom_calc::poe1::skill`).

mod common;

use common::bundle;
use loom_calc::CalcData;
use loom_calc::poe1::skill::{
    InstanceProblem, SkillProblem, instance, instances, main_skill, type_expression,
};
use loom_data::bundle::skills::{LevelStat, QualityStat};
use loom_domain::{
    Attribute, Build, Game, Gem, GemId, Ordered, SkillId, SkillSet, SkillSetId, SocketGroup,
    Sourced,
};

fn gem(gem_id: Option<&str>, skill: &str, level: u32, quality: i32) -> Gem {
    Gem {
        gem: gem_id.map(GemId::new),
        skill: Some(SkillId::new(skill)),
        name: None,
        level: Sourced::new(level),
        quality: Sourced::new(quality),
        enabled: true,
        attributes: Vec::new(),
    }
}

fn bolt(level: u32, quality: i32) -> Gem {
    gem(Some("Test/Gems/Bolt"), "TestBolt", level, quality)
}

fn support() -> Gem {
    gem(Some("Test/Gems/SupportDamage"), "SupportTestDamage", 1, 0)
}

fn group(gems: Vec<Gem>) -> SocketGroup {
    let mut g = SocketGroup {
        label: None,
        enabled: true,
        slot: None,
        main_active_skill: None,
        gems: Ordered::new(),
        attributes: Vec::new(),
    };
    for gem in gems {
        g.gems.push(gem);
    }
    g
}

fn build(groups: Vec<SocketGroup>, main: u32) -> Build {
    let mut b = Build::new(Game::Poe1);
    let mut set = SkillSet {
        id: SkillSetId(1),
        title: None,
        groups: Ordered::new(),
        attributes: Vec::new(),
    };
    for g in groups {
        set.groups.push(g);
    }
    b.skills.sets.push(set);
    b.character.main_socket_group = Some(Sourced::new(main));
    b
}

#[test]
fn the_main_skill_should_be_the_main_groups_active_gem_with_its_supports() {
    let data = bundle();
    let b = build(
        vec![group(vec![support()]), group(vec![support(), bolt(2, 0)])],
        2,
    );
    let m = main_skill(&b, &data).unwrap();
    assert_eq!(m.group, 2);
    assert_eq!(m.effect().id, "TestBolt");
    assert_eq!(m.active_gem.gem, 1, "the gem's place in the group");
    assert_eq!(m.active.map(|a| a.name.as_str()), Some("Test Bolt"));
    let supports: Vec<_> = m.supports.iter().map(|s| s.effect.id.as_str()).collect();
    assert_eq!(supports, ["SupportTestDamage"]);
    assert!(m.unapplied.is_empty());
    assert!(m.types.contains("TestSpell"));
    assert!(!m.item_granted);
}

#[test]
fn the_main_group_should_be_clamped_and_a_missing_gem_named() {
    let data = bundle();
    // mainSocketGroup 9 of 1 group: the last.
    let b = build(vec![group(vec![bolt(1, 0)])], 9);
    assert_eq!(main_skill(&b, &data).unwrap().group, 1);
    let b = build(
        vec![group(vec![gem(Some("Test/Gems/Nothing"), "Nothing", 1, 0)])],
        1,
    );
    assert_eq!(
        main_skill(&b, &data).unwrap_err(),
        SkillProblem::UnknownGem {
            gem: "Test/Gems/Nothing".to_owned()
        }
    );
    assert_eq!(
        main_skill(&build(vec![group(vec![support()])], 1), &data).unwrap_err(),
        SkillProblem::NoActiveSkill
    );
    assert_eq!(
        main_skill(&Build::new(Game::Poe1), &data).unwrap_err(),
        SkillProblem::NoSkillSet
    );
}

#[test]
fn an_item_granted_skill_should_be_found_by_its_skill_id() {
    let data = bundle();
    let mut g = group(vec![gem(None, "TestStrike", 1, 0), support()]);
    g.attributes
        .push(Attribute::new("source", "Item:1:Test Ring"));
    let m = main_skill(&build(vec![g], 1), &data).unwrap();
    assert_eq!(m.effect().id, "TestStrike");
    assert!(m.item_granted);
    // The support is not gems-only and allows attacks: it applies.
    assert_eq!(m.supports.len(), 1);
}

#[test]
fn a_gem_instance_should_have_its_quality_level_and_constant_stats() {
    let data = bundle();
    let effect = data.granted_effect("TestBolt").unwrap();
    // Level 2, quality 20: quality 0.5 x 20 = 10; level stat 2 and the
    // constant 5 on the same stat: 7.
    let i = instance(effect, 2, 20, &data);
    assert_eq!(i.level, 2);
    assert_eq!(
        i.stats,
        [
            ("test_damage_per_level".to_owned(), 7),
            ("test_quality_cast_speed".to_owned(), 10)
        ]
    );
    assert_eq!(i.implicit, ["test_spell_flag"]);
    assert!(i.problems.is_empty());
    // No quality: no quality stat.
    let i = instance(effect, 1, 0, &data);
    assert_eq!(i.stats, [("test_damage_per_level".to_owned(), 6)]);
}

#[test]
fn a_gem_level_outside_the_effect_should_be_clamped_as_pob_does() {
    let data = bundle();
    let effect = data.granted_effect("TestBolt").unwrap();
    assert_eq!(instance(effect, 21, 0, &data).level, 2);
    assert_eq!(instance(effect, 0, 0, &data).level, 1);
    // An effect without levels: no level PoB could use.
    let strike = data.granted_effect("TestStrike").unwrap();
    let mut empty = strike.clone();
    empty.levels.clear();
    let i = instance(&empty, 1, 0, &data);
    assert!(i.problems.contains(&InstanceProblem::NoLevel));
}

#[test]
fn quality_stats_should_truncate_the_double_product_as_pob_does() {
    let data = bundle();
    let mut effect = data.granted_effect("TestBolt").unwrap().clone();
    // 0.29 x 100 as doubles is 28.999999999999996: PoB's `math.modf` gives
    // 28, where integer arithmetic would give 29.
    effect.quality = vec![QualityStat {
        stat: "q".to_owned(),
        per_mille: 290,
    }];
    let i = instance(&effect, 1, 100, &data);
    assert!(i.stats.contains(&("q".to_owned(), 28)), "{:?}", i.stats);
}

#[test]
fn interpolated_level_stats_and_missing_stat_sets_should_be_problems() {
    let mut data = bundle();
    {
        let skills = data.skills.as_mut().unwrap();
        let set = skills
            .stat_sets
            .iter_mut()
            .find(|s| s.id == "TestBolt")
            .unwrap();
        set.levels[0].stats.push(LevelStat {
            stat: "linear".to_owned(),
            value: 3,
            interpolation: Some(2),
        });
    }
    let effect = data.granted_effect("TestBolt").unwrap().clone();
    let i = instance(&effect, 1, 0, &data);
    assert_eq!(
        i.problems,
        [InstanceProblem::Interpolated {
            stat: "linear".to_owned()
        }]
    );
    let mut missing = effect.clone();
    missing.stat_sets = vec!["Nowhere".to_owned(), "TestBolt".to_owned()];
    let i = instance(&missing, 1, 0, &data);
    assert!(i.problems.contains(&InstanceProblem::SeveralStatSets));
    assert!(i.problems.contains(&InstanceProblem::MissingStatSet {
        id: "Nowhere".to_owned()
    }));
}

#[test]
fn the_main_skills_instances_should_use_each_gems_level_and_quality() {
    let data = bundle();
    let b = build(vec![group(vec![bolt(2, 20), support()])], 1);
    let m = main_skill(&b, &data).unwrap();
    let is = instances(&b, &m, &data);
    assert_eq!(is.len(), 2);
    assert_eq!(
        (is[0].effect.id.as_str(), is[0].level, is[0].quality),
        ("TestBolt", 2, 20)
    );
    assert_eq!(is[1].effect.id.as_str(), "SupportTestDamage");
}

#[test]
fn type_expressions_should_be_postfix() {
    let types: std::collections::BTreeSet<String> = ["A".to_owned()].into();
    let e = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert!(type_expression(&e(&["A"]), &types, &[]));
    assert!(!type_expression(&e(&["A", "NOT"]), &types, &[]));
    assert!(type_expression(&e(&["B", "A", "OR"]), &types, &[]));
    assert!(!type_expression(&e(&["B", "A", "AND"]), &types, &[]));
    assert!(!type_expression(&[], &types, &[]));
}
