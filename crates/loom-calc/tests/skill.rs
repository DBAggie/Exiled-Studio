//! The skill model (Phase 3 plan, step 9a) over the synthetic PoE1 bundle:
//! the main skill as PoB picks it, its supports, and each gem instance's
//! stats at its level and quality (`loom_calc::poe1::skill`).

mod common;

use common::bundle;
use loom_calc::CalcData;
use std::collections::BTreeSet;

use loom_calc::poe1::Poe1Mapping;
use loom_calc::poe1::gem_levels::{GemLevels, LEVEL_STATS};
use loom_calc::poe1::skill::{
    InstanceProblem, KEYWORD_TYPES, SkillProblem, group_skills, instance, instances, keyword_flags,
    main_skill, type_expression,
};
use loom_calc::{Conditions, Effect, Flags, Source, StatMap, StatMapping};
use loom_data::bundle::skills::{LevelStat, QualityStat};
use loom_domain::{
    Attribute, Build, Game, Gem, GemId, Ordered, SkillId, SkillSet, SkillSetId, SocketGroup,
    Sourced, StatId,
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
    let m = main_skill(&b, &data, &GemLevels::none()).unwrap();
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

/// P3-8h: each of the five type names gives its keyword, several give
/// several, and other types none.
#[test]
fn keyword_flags_should_follow_the_skills_types() {
    let types =
        |names: &[&str]| -> BTreeSet<String> { names.iter().map(|n| (*n).to_owned()).collect() };
    for (name, t) in KEYWORD_TYPES {
        assert_eq!(
            keyword_flags(&types(&["Spell", name])),
            Flags::skill_keyword(t),
            "{name}"
        );
    }
    assert_eq!(
        keyword_flags(&types(&["Attack", "Fire", "Lightning"])),
        Flags::SKILL_FIRE | Flags::SKILL_LIGHTNING
    );
    assert_eq!(
        keyword_flags(&types(&["Spell", "Area", "cold", "Elemental"])),
        Flags::NONE
    );
}

/// A type a support adds gives its keyword (PoB's `skillTypes` after the
/// supports' `addSkillTypes`).
#[test]
fn a_type_a_support_adds_should_give_its_keyword() {
    let mut data = bundle();
    let base = main_skill(
        &build(vec![group(vec![bolt(1, 0), support()])], 1),
        &data,
        &GemLevels::none(),
    )
    .unwrap()
    .keyword_flags();
    assert_eq!(base, Flags::NONE);
    let skills = data.skills.as_mut().unwrap();
    for e in &mut skills.granted_effects {
        if e.id == "SupportTestDamage" {
            e.added_types = vec!["Cold".to_owned()];
        }
    }
    let m = main_skill(
        &build(vec![group(vec![bolt(1, 0), support()])], 1),
        &data,
        &GemLevels::none(),
    )
    .unwrap();
    assert!(m.types.contains("Cold"));
    assert_eq!(m.keyword_flags(), Flags::SKILL_COLD);
}

#[test]
fn the_main_group_should_be_clamped_and_a_missing_gem_named() {
    let data = bundle();
    // mainSocketGroup 9 of 1 group: the last.
    let b = build(vec![group(vec![bolt(1, 0)])], 9);
    assert_eq!(main_skill(&b, &data, &GemLevels::none()).unwrap().group, 1);
    let b = build(
        vec![group(vec![gem(Some("Test/Gems/Nothing"), "Nothing", 1, 0)])],
        1,
    );
    assert_eq!(
        main_skill(&b, &data, &GemLevels::none()).unwrap_err(),
        SkillProblem::UnknownGem {
            gem: "Test/Gems/Nothing".to_owned()
        }
    );
    assert_eq!(
        main_skill(
            &build(vec![group(vec![support()])], 1),
            &data,
            &GemLevels::none()
        )
        .unwrap_err(),
        SkillProblem::NoActiveSkill
    );
    assert_eq!(
        main_skill(&Build::new(Game::Poe1), &data, &GemLevels::none()).unwrap_err(),
        SkillProblem::NoSkillSet
    );
}

#[test]
fn an_item_granted_skill_should_be_found_by_its_skill_id() {
    let data = bundle();
    let mut g = group(vec![gem(None, "TestStrike", 1, 0), support()]);
    g.attributes
        .push(Attribute::new("source", "Item:1:Test Ring"));
    let m = main_skill(&build(vec![g], 1), &data, &GemLevels::none()).unwrap();
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
    // The implicit stat counts 1 (PoB's `level[index] or 1`). Base damage
    // (effectiveness interpolation) is the game's resolved value: 6 to 12
    // at level 2.
    let i = instance(effect, 2, 20, &data);
    assert_eq!(i.level, Some(2));
    assert_eq!(
        i.stats,
        [
            ("test_base_damage_max".to_owned(), 12),
            ("test_base_damage_min".to_owned(), 6),
            ("test_damage_per_level".to_owned(), 7),
            ("test_quality_cast_speed".to_owned(), 10),
            ("test_spell_flag".to_owned(), 1)
        ]
    );
    assert!(i.problems.is_empty());
    // No quality: no quality stat.
    let i = instance(effect, 1, 0, &data);
    assert_eq!(
        i.stats,
        [
            ("test_base_damage_max".to_owned(), 9),
            ("test_base_damage_min".to_owned(), 4),
            ("test_damage_per_level".to_owned(), 6),
            ("test_spell_flag".to_owned(), 1)
        ]
    );
    // The level rows PoB uses for a gem's level.
    let row = loom_calc::poe1::skill::stat_set_level(effect, 9, &data).unwrap();
    assert_eq!((row.level, row.spell_crit_chance), (2, 600));
    assert_eq!(
        loom_calc::poe1::skill::effect_level(effect, 1).map(|l| l.level),
        Some(1)
    );
}

#[test]
fn a_gem_level_outside_the_effect_should_be_clamped_as_pob_does() {
    let data = bundle();
    let effect = data.granted_effect("TestBolt").unwrap();
    assert_eq!(instance(effect, 21, 0, &data).level, Some(2));
    assert_eq!(instance(effect, 0, 0, &data).level, Some(1));
    // An effect without levels: no level PoB could use.
    let strike = data.granted_effect("TestStrike").unwrap();
    let mut empty = strike.clone();
    empty.levels.clear();
    let i = instance(&empty, 1, 0, &data);
    assert!(i.problems.contains(&InstanceProblem::NoLevel));
    assert_eq!(i.level, None);
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
    // Only the first stat set counts (PoB exports only it); a later one
    // is flagged.
    let mut several = effect.clone();
    several.stat_sets = vec!["TestBolt".to_owned(), "SupportTestDamage".to_owned()];
    let i = instance(&several, 1, 0, &data);
    assert!(i.problems.contains(&InstanceProblem::SeveralStatSets));
    assert_eq!(
        i.stats,
        instance(&effect, 1, 0, &data).stats,
        "the second set adds nothing"
    );
    let mut missing = effect.clone();
    missing.stat_sets = vec!["Nowhere".to_owned()];
    let i = instance(&missing, 1, 0, &data);
    assert_eq!(
        i.problems,
        [InstanceProblem::MissingStatSet {
            id: "Nowhere".to_owned()
        }]
    );
}

#[test]
fn the_main_skills_instances_should_use_each_gems_level_and_quality() {
    let data = bundle();
    let b = build(vec![group(vec![bolt(2, 20), support()])], 1);
    let m = main_skill(&b, &data, &GemLevels::none()).unwrap();
    assert_eq!((m.active_gem.level, m.active_gem.quality), (2, 20));
    let is = instances(&m, &data);
    assert_eq!(is.len(), 2);
    assert_eq!(
        (is[0].effect.id.as_str(), is[0].level, is[0].quality),
        ("TestBolt", Some(2), 20)
    );
    assert_eq!(is[1].effect.id.as_str(), "SupportTestDamage");
}

/// Gem level totals as the PoE1 mapping turns `stats` into effects
/// (P3-8i).
fn levels(stats: &[(&str, f64)]) -> GemLevels {
    let mut effects = Vec::new();
    for (stat, value) in stats {
        let StatMap::Effects(templates) = Poe1Mapping.map(&StatId::new(*stat)) else {
            panic!("{stat} is not mapped");
        };
        for t in templates {
            assert_eq!(t.conditions, Conditions::none(), "{stat}");
            effects.push(Effect {
                quantity: t.quantity,
                op: t.op,
                value: value * t.scale / t.divisor,
                conditions: t.conditions,
                source: Source::Rule("test"),
            });
        }
    }
    GemLevels::from_effects(&effects)
}

/// The synthetic bundle with real gem tags: Test Bolt a cold spell skill
/// gem, the support a critical support gem.
fn tagged() -> loom_data::bundle::Bundle {
    let mut data = bundle();
    let tags = |t: &[&str]| t.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let effects = data
        .skills
        .as_mut()
        .map_or(&mut [][..], |s| &mut s.gem_effects[..]);
    assert!(!effects.is_empty());
    for e in effects {
        e.tags = match e.id.as_str() {
            "TestBolt" => tags(&["intelligence", "grants_active_skill", "spell", "cold"]),
            _ => tags(&["support", "critical", "intelligence"]),
        };
    }
    data
}

/// PoB's `applyGemMods` (P3-8i): every gem whose tags match a line's
/// keywords gains its levels, an active gem and a support alike; the level
/// is then clamped only where the gem's rows are read.
#[test]
fn gem_level_modifiers_should_raise_the_gems_they_match() {
    let data = tagged();
    let all = levels(&[
        ("cold_skill_gem_level_+", 1.0),
        ("spell_skill_gem_level_+", 1.0),
        ("fire_skill_gem_level_+", 4.0),
        ("critical_support_gem_level_+", 2.0),
        ("minion_skill_gem_level_+", 3.0),
    ]);
    let b = build(vec![group(vec![bolt(1, 0), support()])], 1);
    let m = main_skill(&b, &data, &all).unwrap();
    assert_eq!((m.active_gem.level, m.active_gem.bonus), (3, 2));
    assert_eq!((m.supports[0].level, m.supports[0].bonus), (3, 2));
    // Test Bolt has levels 1 and 2: its instance clamps 3 to 2, as
    // `calcLib.validateGemLevel` does.
    let is = instances(&m, &data);
    assert_eq!(is[0].level, Some(2));
    assert_eq!(is[1].level, Some(1));
    // The same totals reach every group's gems.
    let skills = group_skills(
        b.skills
            .sets
            .typed()
            .next()
            .unwrap()
            .groups
            .typed()
            .next()
            .unwrap(),
        &[],
        &data,
        &all,
    )
    .unwrap();
    assert_eq!(skills[0].active_gem.level, 3);
    assert_eq!(skills[0].supports[0].level, 3);
    // Without modifiers, the levels the build writes.
    let m = main_skill(&b, &data, &GemLevels::none()).unwrap();
    assert_eq!((m.active_gem.level, m.active_gem.bonus), (1, 0));
    // A negative total lowers the level.
    let m = main_skill(&b, &data, &levels(&[("all_skill_gem_level_+", -1.0)])).unwrap();
    assert_eq!(m.active_gem.level, 0);
    assert_eq!(instances(&m, &data)[0].level, Some(1));
}

/// A skill an item grants (only `skillId`) has no gem in PoB
/// (`data.gemForSkill` is keyed by the granted effect's table), so it gains
/// no level (P3-8i).
#[test]
fn an_item_granted_skill_should_gain_no_gem_levels() {
    let data = tagged();
    let mut g = group(vec![gem(None, "TestStrike", 1, 0), support()]);
    g.attributes
        .push(Attribute::new("source", "Item:1:Test Ring"));
    let all = levels(&[
        ("all_skill_gem_level_+", 2.0),
        ("critical_support_gem_level_+", 1.0),
    ]);
    let m = main_skill(&build(vec![g], 1), &data, &all).unwrap();
    assert_eq!((m.active_gem.level, m.active_gem.bonus), (1, 0));
    // The support is a gem: it gains its level.
    assert_eq!(m.supports[0].level, 2);
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

/// The real PoE1 bundle's Ice Nova and Smite at level 20 and quality 20,
/// against PoB v2.67.2's own skill data (`Data/Skills/act_int.lua`
/// `IceNova`, `act_str.lua` `Smite`: `qualityStats`, `constantStats`,
/// the level-20 row, and `stats` with PoB's `level[index] or 1`).
/// Maintainer-run: `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc
/// --test skill -- --ignored`.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_gems_should_have_pobs_stats() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    let stats = |id: &str| {
        let effect = data.granted_effect(id).unwrap();
        let i = instance(effect, 20, 20, &data);
        assert!(i.problems.is_empty(), "{id}: {:?}", i.problems);
        i.stats
    };
    let s = |v: &[(&str, i64)]| {
        v.iter()
            .map(|(k, n)| ((*k).to_owned(), *n))
            .collect::<Vec<_>>()
    };
    // Ice Nova: radius 0.15 x 20 = 3, level 4, constant 26; freeze 25;
    // `is_area_damage` 1; base damage 1122 to 1683 (resolved; PoB's
    // effectiveness formula at level requirement 70 gives the same).
    assert_eq!(
        stats("IceNova"),
        s(&[
            ("active_skill_base_area_of_effect_radius", 33),
            ("base_chance_to_freeze_%", 25),
            ("is_area_damage", 1),
            ("spell_maximum_base_cold_damage", 1683),
            ("spell_minimum_base_cold_damage", 1122),
        ])
    );
    // Smite: area damage 0.5 x 20 = 10 and the level's -20; the quality
    // display stat 1 x 20; shock 19; radii 6 + 15 and 6 + 36; the
    // constants; three implicit flags.
    assert_eq!(
        stats("Smite"),
        s(&[
            ("active_skill_area_damage_+%_final", -10),
            ("active_skill_base_area_of_effect_radius", 21),
            ("active_skill_base_secondary_area_of_effect_radius", 42),
            ("active_skill_secondary_area_of_effect_description_mode", 4),
            ("base_chance_to_shock_%_from_skill", 19),
            ("base_skill_effect_duration", 4000),
            ("base_smite_number_of_targets", 1),
            ("console_skill_dont_chase", 1),
            ("maximum_added_lightning_damage_from_skill", 220),
            ("minimum_added_lightning_damage_from_skill", 12),
            ("quality_display_active_skill_area_damage_is_gem", 1),
            (
                "quality_display_active_skill_area_damage_quality_negated_from_gem",
                20
            ),
            ("skill_physical_damage_%_to_convert_to_lightning", 50),
            ("smite_lightning_target_range", 50),
            ("visual_hit_effect_elemental_is_holy", 1),
        ])
    );
}

/// The real PoE1 bundle's skill types name every keyword type as
/// `KEYWORD_TYPES` spells it, and Ice Nova is a cold skill, Smite a
/// lightning one (P3-8h). Maintainer-run, as above.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_skill_types_should_name_the_keyword_types() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    let skills = data.skills.as_ref().unwrap();
    for (name, _) in KEYWORD_TYPES {
        assert!(
            skills.skill_types.iter().any(|t| t.id == name),
            "{name} is no skill type"
        );
    }
    let flags = |id: &str| {
        let active = data.active_skill(id).unwrap();
        keyword_flags(&active.types.iter().cloned().collect())
    };
    assert_eq!(flags("ice_nova"), Flags::SKILL_COLD);
    assert_eq!(flags("smite"), Flags::SKILL_LIGHTNING);
}

/// The real PoE1 bundle's gem colours (P3-12i): every granted effect has
/// one, 1 to 4, and the in-scope skills and their gems have PoB's
/// (`Data/Skills/*.lua` `color`). Maintainer-run, as above.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_gems_should_have_pobs_colours() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    let effects = &data.skills.as_ref().unwrap().granted_effects;
    assert!(
        effects
            .iter()
            .all(|e| e.attribute.is_some_and(|a| (1..=4).contains(&a)))
    );
    let colour = |id: &str| data.granted_effect(id).unwrap().attribute;
    for (id, c) in [
        ("IceNova", 3),
        ("Smite", 1),
        ("Precision", 2),
        ("Haste", 2),
        ("SupportControlledDestruction", 3),
        ("SupportFasterAttacks", 2),
        ("SupportMeleePhysicalDamage", 1),
    ] {
        assert_eq!(colour(id), Some(c), "{id}");
    }
}

/// Each gem level row's keywords are the words of the stat's English
/// description between "Level of all" and "Gems", lower-cased, as PoB's
/// `"([%+%-]%d+)%%? to (%a+) of all (.+) gems"` reads them (P3-8i), and
/// every keyword names a real gem tag or one of `calcLib.gemIsType`'s
/// words. Maintainer-run, as above.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_gem_level_stats_should_have_pobs_keywords() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    let texts: Vec<(&str, &str)> = data
        .stat_descriptions
        .files()
        .flat_map(|(_, f)| &f.descriptions)
        .filter(|d| d.stats.len() == 1)
        .flat_map(|d| {
            d.variants
                .iter()
                .map(move |v| (d.stats[0].as_str(), v.text.as_str()))
        })
        .collect();
    let tags: BTreeSet<&str> = data
        .skills
        .as_ref()
        .unwrap()
        .gem_tags
        .iter()
        .map(|t| t.id.as_str())
        .collect();
    // PoB reads a phrase that is a gem's name as that gem
    // (`gemIdLookup`, `ModParser.lua` 3282): no row's phrase may be one.
    let skills = data.skills.as_ref().unwrap();
    let gem_names: BTreeSet<String> = skills
        .gems
        .iter()
        .filter_map(|g| data.base_items.iter().find(|b| b.id == g.base_item))
        .map(|b| b.name.to_lowercase())
        .chain(skills.gem_effects.iter().map(|e| e.name.to_lowercase()))
        .filter(|n| !n.is_empty())
        .collect();
    assert!(gem_names.contains("ice nova"));
    for (stat, words) in LEVEL_STATS {
        assert!(!gem_names.contains(&words.join(" ")), "{stat}");
        let found: BTreeSet<Vec<String>> = texts
            .iter()
            .filter(|(s, _)| *s == stat)
            .map(|(_, text)| {
                let rest = text
                    .split_once(" to Level of all ")
                    .unwrap_or_else(|| panic!("{stat}: {text}"))
                    .1;
                let phrase = rest
                    .strip_suffix(" Gems")
                    .unwrap_or_else(|| panic!("{stat}: {text}"));
                phrase.split(' ').map(str::to_lowercase).collect()
            })
            .collect();
        let want: Vec<String> = words.iter().map(|w| (*w).to_owned()).collect();
        assert_eq!(found, BTreeSet::from([want]), "{stat}");
        for w in words {
            assert!(
                tags.contains(w) || ["skill", "non-exceptional"].contains(w),
                "{stat}: {w}"
            );
        }
    }
}
