//! P3-8n's rows (`loom_calc::poe1::conditions`, `poe1::stacks`,
//! `poe1::mapping`, `poe1::buffs`, `poe1::damage_taken`, `poe1::exposure`)
//! against the real bundle: each row's text is its stat's description, as
//! the rows' comments quote PoB's reading of it; every guard is a stat of
//! the bundle; and the mods carry their implicit tags (data schema 3).
//! Maintainer-run (the bundle is not in the repository):
//! `LOOM_BUNDLE_DIR=<bundle> cargo test -p loom-calc --test
//! conditions_stacks -- --ignored`.

#[expect(dead_code, reason = "only the real bundle's reader is used here")]
mod common;

use loom_calc::StatMapping;
use loom_calc::poe1::mapping::{NO_EFFECT, NO_EFFECT_DEPENDS_ON, Poe1Mapping};
use loom_calc::poe1::stacks::MODIFIERS;
use loom_domain::StatId;

/// The English text of each P3-8n row's stat (`stat_descriptions.txt`,
/// the gem files for the supports' stats), as PoB's patterns read them.
const TEXTS: &[(&str, &str)] = &[
    ("action_speed_-%", "{0}% increased Action Speed"),
    (
        "base_physical_damage_%_to_convert_to_fire",
        "{0}% of Physical Damage Converted to Fire Damage",
    ),
    (
        "cast_speed_+%_per_num_unique_spells_cast_recently",
        "{0}% increased Cast Speed for each different Non-Instant Spell you've Cast Recently",
    ),
    (
        "chance_to_deal_double_damage_%",
        "{0}% chance to deal Double Damage",
    ),
    (
        "consecrated_ground_effect_+%",
        "{0}% increased Effect of Consecrated Ground you create",
    ),
    (
        "consecrated_ground_enemy_damage_taken_+%",
        "Consecrated Ground you create applies {0}% increased Damage taken to Enemies",
    ),
    (
        "critical_strike_chance_against_enemies_on_full_life_+%",
        "{0}% increased Critical Strike Chance against Enemies that are on Full Life",
    ),
    (
        "critical_strike_multiplier_+_vs_unique_enemies",
        "+{0}% to Critical Strike Multiplier against Unique Enemies",
    ),
    (
        "damage_+%_if_enemy_shattered_recently",
        "{0}% increased Damage if you've Shattered an Enemy Recently",
    ),
    (
        "damage_+%_if_you_have_shocked_recently",
        "{0}% increased Damage if you have Shocked an Enemy Recently",
    ),
    (
        "gain_convergence_on_hitting_unique_enemy",
        "Gain Convergence when you Hit a Unique Enemy, no more than once every 8 seconds",
    ),
    ("gain_x_rage_on_melee_hit", "Gain {0} Rage on Melee Hit"),
    (
        "ignites_and_chill_apply_elemental_resistance_+",
        "Enemies Ignited or Chilled by you have {:+d}% to Elemental Resistances",
    ),
    (
        "local_elemental_damage_+%_per_2%_quality",
        "Grants {0}% increased Elemental Damage per 2% Quality",
    ),
    (
        "local_quality_does_not_increase_physical_damage",
        "Quality does not increase Physical Damage",
    ),
    (
        "maximum_intensify_stacks",
        "Spells which can gain Intensity have {0:+d} to maximum Intensity",
    ),
    (
        "maximum_life_+%_if_no_life_tags_on_body_armour",
        "{0}% increased maximum Life if there are no Life Modifiers on Equipped Body Armour",
    ),
    ("maximum_rage", "{0:+d} to Maximum Rage"),
    (
        "melee_damage_+%_at_close_range",
        "{0}% increased Melee Damage with Hits at Close Range",
    ),
    ("mod_granted_passive_hash", "Allocates {0}"),
    (
        "nearby_enemies_are_intimidated_while_you_have_rage",
        "Nearby Enemies are Intimidated while you have Rage",
    ),
    (
        "physical_staff_damage_+%",
        "{0}% increased Physical Damage with Staves",
    ),
    (
        "reduce_enemy_fire_resistance_with_weapons_%",
        "Damage with Weapons Penetrates {0}% Fire Resistance",
    ),
    (
        "spell_chance_to_deal_double_damage_%",
        "Spells have a {0}% chance to deal Double Damage",
    ),
    (
        "support_arcane_surge_spell_damage_+%_final_while_you_have_arcane_surge",
        "Supported Skills deal {0}% more Spell Damage while you have Arcane Surge",
    ),
    (
        "support_momentum_attack_speed_+%_per_stack",
        "Supported Skills have {0}% increased Attack Speed per Momentum",
    ),
    (
        "elemental_damage_+%_final_per_righteous_charge",
        "Supported Skills deal {0}% more Elemental Damage per Inspiration Charge",
    ),
    (
        "critical_strike_chance_+%_per_righteous_charge",
        "Supported Skills have {0}% increased Critical Strike Chance per Inspiration Charge",
    ),
];

/// Every stat P3-8n maps or guards: a misspelt ID cannot pass as a row
/// or as `Unmapped`.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_p3_8n_stats_should_be_the_bundles() {
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    let texts: Vec<(&str, &str)> = data
        .stat_descriptions
        .files()
        .flat_map(|(_, f)| &f.descriptions)
        .flat_map(|d| {
            d.stats.iter().flat_map(move |s| {
                d.variants
                    .iter()
                    .map(move |v| (s.as_str(), v.text.as_str()))
            })
        })
        .collect();
    for &(stat, text) in TEXTS {
        assert!(
            texts.iter().any(|&(s, t)| s == stat && t == text),
            "{stat}: {text}"
        );
        // A weapon's local lines are `WeaponMapping`'s rows.
        let mapping: &dyn StatMapping = if stat.starts_with("local_") {
            &loom_calc::poe1::mapping::WeaponMapping
        } else {
            &Poe1Mapping
        };
        assert!(
            !matches!(
                mapping.map(&StatId::new(stat)),
                loom_calc::StatMap::Unmapped
            ),
            "{stat}: not a row"
        );
    }
    let known = |id: &str| data.stats.iter().any(|s| s.id == id);
    let guards = NO_EFFECT_DEPENDS_ON
        .iter()
        .map(|(s, _)| *s)
        .chain(
            loom_calc::poe1::buffs::ARCANE_SURGE_DEPENDS_ON
                .iter()
                .copied(),
        )
        .chain(MODIFIERS.iter().map(|(s, ..)| *s))
        .chain(NO_EFFECT.iter().map(|(s, _)| *s));
    let missing: Vec<_> = guards.filter(|id| !known(id)).collect();
    assert!(missing.is_empty(), "{missing:?}");
}

/// The mods carry their implicit tags (data schema 3): the life mods have
/// `life`, and the Life Mastery's effect is the stat the state reads.
#[test]
#[ignore = "needs the real PoE1 bundle (LOOM_BUNDLE_DIR)"]
#[expect(
    clippy::disallowed_methods,
    reason = "a maintainer-run test reads the bundle named by the environment"
)]
fn real_mods_should_carry_implicit_tags() {
    use loom_calc::CalcData;
    let dir = std::env::var("LOOM_BUNDLE_DIR").expect("LOOM_BUNDLE_DIR");
    let data = common::bundle_at(std::path::Path::new(&dir));
    assert!(data.manifest.data_schema >= loom_calc::input::IMPLICIT_TAGS_SCHEMA);
    let life = data
        .mods
        .iter()
        .find(|m| m.id == "IncreasedLife1")
        .expect("IncreasedLife1");
    assert!(life.implicit_tags.iter().any(|t| t == "life"));
    let by_tag = data.mods_by_tag("life").expect("tags");
    assert!(
        by_tag
            .tagged
            .iter()
            .any(|m| m.contains(&"base_maximum_life".to_owned()))
    );
    let mastery = data.mastery_effect_stats(34242).expect("Life2");
    assert!(
        mastery
            .iter()
            .any(|s| s.stat == "maximum_life_+%_if_no_life_tags_on_body_armour")
    );
}
