//! PoE1 item rules, held here until the PoE1 data bundle carries them
//! (Phase 2 plan, step 3; decision 5: per-game rules are data, never
//! constants in shared crates).
//!
//! Scope by mod domain (Spike B plan, "Scope by mod domain"): the base's
//! `BaseItemTypes.ModDomain`. PoE1 values, from the vendored schema's
//! `ModDomains` enum (`indexing: 1`) and checked on the rows of PoE1
//! 26530219: 34 = `TINCTURE` (the 18 tincture bases, e.g. `Ironwood
//! Tincture`) uses `tincture_stat_descriptions`, 38 = `GRAFT` (the 17 graft
//! bases, e.g. `Fleshgraft`) uses `graft_stat_descriptions`, every other
//! domain uses `stat_descriptions`.
//!
//! The `Mods` filter's rules (`mod_filter` module docs give each rule and
//! its evidence) are written as `ModRules` data here; the enum values are
//! the vendored schema's `ModDomains` and `ModGenerationType` (both
//! `indexing: 1`) and `InfluenceTypes` (`indexing: 0`).

use loom_domain::ModSection;
use loom_parser::import::mods::{
    DomainMatch, FitClause, FitRule, LocalCarriers, ModRules, Unflagged,
};
use loom_parser::import::{ItemRules, TextRules};

/// The PoE1 item rules.
#[must_use]
pub fn item_rules() -> ItemRules {
    ItemRules {
        default_scope: "stat_descriptions".to_owned(),
        domain_scopes: vec![
            (34, "tincture_stat_descriptions".to_owned()),
            (38, "graft_stat_descriptions".to_owned()),
        ],
        mods: mod_rules(),
        text: TextRules {
            compositions: COMPOSITIONS
                .iter()
                .map(|(id, stat)| ((*id).to_owned(), (*stat).to_owned()))
                .collect(),
            property_ids: PROPERTY_IDS.iter().map(|i| (*i).to_owned()).collect(),
            label_ids: LABEL_IDS.iter().map(|i| (*i).to_owned()).collect(),
            granted_skill_wrapper: Some(GRAFT_WRAPPER_ID.to_owned()),
        },
    }
}

/// `ClientStrings` ids of item-display property texts (the categorizer's
/// `item-property` ids with numeric placeholders; Spike B PR B5d).
pub const PROPERTY_IDS: [&str; 8] = [
    "ItemDisplayFlaskBuffDuration1Second",
    "ItemDisplayFlaskBuffDurationNSeconds",
    "ItemDisplayChargesConsumptionOnUse1Charge",
    "ItemDisplayChargesConsumptionOnUseNCharges",
    "ItemDisplayCharges1Charge",
    "ItemDisplayChargesNCharges",
    "ItemDisplayChargesCurrentOfMax1",
    "ItemDisplayChargesCurrentOfMaxN",
];

/// `ClientStrings` ids of property labels PoB writes as `<label>: <number>`.
pub const LABEL_IDS: [&str; 1] = ["ZanaInfluencedItem"];

/// The `ClientStrings` id of the graft skill wrapper (`Uses level {1} {0}`;
/// Spike B PR B5g).
pub const GRAFT_WRAPPER_ID: &str = "BrequelGraftSkillString";

/// `ClientStrings` wrappers the resolver reads through (PR B5c): the id and
/// the stat the wrapper stands for (moved from `categorize`).
pub const COMPOSITIONS: [(&str, &str); 2] = [
    (
        "InfluenceStatConditionPresenceUniqueMonster",
        "local_influence_mod_requires_unique_monster_presence",
    ),
    (
        "InfluenceStatConditionPresenceCelestialBoss",
        "local_influence_mod_requires_celestial_boss_presence",
    ),
];

/// `ModDomains` values used by the fit rules.
pub mod domain {
    /// `ITEM`.
    pub const ITEM: i64 = 1;
    /// `CRAFTED`.
    pub const CRAFTED: i64 = 9;
    /// `BASE_JEWEL`.
    pub const BASE_JEWEL: i64 = 10;
    /// `ABYSS_JEWEL`.
    pub const ABYSS_JEWEL: i64 = 13;
    /// `DELVE`.
    pub const DELVE: i64 = 16;
    /// `SYNTHESIS_A`, `SYNTHESIS_GLOBALS`, `SYNTHESIS_BONUS`.
    pub const SYNTHESIS: [i64; 3] = [18, 19, 20];
    /// `UNVEILED`.
    pub const UNVEILED: i64 = 28;
}

/// `ModGenerationType` values used by the fit rules.
pub mod generation {
    /// `PREFIX`.
    pub const PREFIX: i64 = 1;
    /// `SUFFIX`.
    pub const SUFFIX: i64 = 2;
    /// `UNIQUE`.
    pub const UNIQUE: i64 = 3;
    /// `CORRUPTED`.
    pub const CORRUPTED: i64 = 5;
    /// `ENCHANTMENT`.
    pub const ENCHANTMENT: i64 = 10;
    /// `SYNTHESIS_A`, `SYNTHESIS_GLOBALS`, `SYNTHESIS_BONUS`.
    pub const SYNTHESIS: [i64; 3] = [15, 16, 17];
    /// `FLASK_ENCHANTMENT_ENKINDLING`, `FLASK_ENCHANTMENT_INSTILLING`.
    pub const FLASK_ENCHANTMENT: [i64; 2] = [21, 22];
    /// `SCOURGE_UPSIDE`, `SCOURGE_DOWNSIDE`.
    pub const SCOURGE: [i64; 2] = [24, 25];
    /// `EXARCH_IMPLICIT`.
    pub const EXARCH: i64 = 28;
    /// `EATER_IMPLICIT`.
    pub const EATER: i64 = 29;
    /// `WEAPON_TREE`, `WEAPON_TREE_RECOMBINED`.
    pub const WEAPON_TREE: [i64; 2] = [31, 32];
}

/// `InfluenceTypes` value of rows without an influence (`NONE`).
pub const NO_INFLUENCE: i64 = 6;

/// Influence names (the domain's `InfluenceId`, PoB's `<display> Item`
/// without ` Item`) to `InfluenceTypes` (`SHAPER` 0, `ELDER` 1, `CRUSADER`
/// 2, `EYRIE` 3, `BASILISK` 4, `ADJUDICATOR` 5), after PoB's
/// `influenceInfo` (`v2.67.2:src/Modules/ItemTools.lua`: Redeemer is eyrie,
/// Hunter basilisk, Warlord adjudicator). The eldritch displays have no
/// `InfluenceTypes` value.
const INFLUENCE_TYPES: [(&str, i64); 6] = [
    ("Shaper", 0),
    ("Elder", 1),
    ("Crusader", 2),
    ("Redeemer", 3),
    ("Hunter", 4),
    ("Warlord", 5),
];

fn other(name: &str) -> ModSection {
    ModSection::Other(name.to_owned())
}

fn own() -> DomainMatch {
    DomainMatch {
        own: true,
        ..DomainMatch::default()
    }
}

fn clause(generations: &[i64], domains: DomainMatch) -> FitClause {
    FitClause {
        generations: generations.to_vec(),
        domains,
        ..FitClause::default()
    }
}

fn rule(
    section: ModSection,
    flag: Option<&str>,
    unique: Option<bool>,
    clauses: Vec<FitClause>,
) -> FitRule {
    FitRule {
        section,
        flag: flag.map(str::to_owned),
        unique,
        clauses,
    }
}

/// The `Mods` filter's PoE1 rules (`mod_filter` module docs, "Fit").
fn mod_rules() -> ModRules {
    use generation as g;
    let affix = [g::PREFIX, g::SUFFIX];
    let eldritch_flags = vec!["exarch".to_owned(), "eater".to_owned()];
    ModRules {
        fits: vec![
            rule(
                ModSection::Explicit,
                Some("crafted"),
                None,
                vec![FitClause {
                    craft_class: true,
                    ..clause(
                        &affix,
                        DomainMatch {
                            fixed: vec![domain::CRAFTED],
                            ..DomainMatch::default()
                        },
                    )
                }],
            ),
            rule(
                ModSection::Explicit,
                None,
                Some(true),
                vec![clause(&[g::UNIQUE], own())],
            ),
            rule(
                ModSection::Explicit,
                None,
                None,
                vec![FitClause {
                    essence_class: true,
                    ..clause(
                        &affix,
                        DomainMatch {
                            own: true,
                            fixed: Vec::new(),
                            related: vec![
                                (
                                    domain::DELVE,
                                    vec![domain::ITEM, domain::BASE_JEWEL, domain::ABYSS_JEWEL],
                                ),
                                (domain::UNVEILED, vec![domain::ITEM]),
                            ],
                        },
                    )
                }],
            ),
            rule(
                ModSection::Implicit,
                Some("exarch"),
                None,
                vec![clause(&[g::EXARCH], own())],
            ),
            rule(
                ModSection::Implicit,
                Some("eater"),
                None,
                vec![clause(&[g::EATER], own())],
            ),
            rule(
                ModSection::Implicit,
                None,
                None,
                vec![
                    clause(&[g::UNIQUE, g::CORRUPTED, g::EXARCH, g::EATER], own()),
                    clause(
                        &g::SYNTHESIS,
                        DomainMatch {
                            fixed: domain::SYNTHESIS.to_vec(),
                            ..DomainMatch::default()
                        },
                    ),
                ],
            ),
            rule(
                ModSection::Enchant,
                None,
                None,
                vec![clause(
                    &[
                        g::ENCHANTMENT,
                        g::UNIQUE,
                        g::FLASK_ENCHANTMENT[0],
                        g::FLASK_ENCHANTMENT[1],
                    ],
                    DomainMatch {
                        own: true,
                        fixed: vec![domain::ITEM],
                        related: Vec::new(),
                    },
                )],
            ),
            rule(
                other("scourge"),
                None,
                None,
                vec![clause(&g::SCOURGE, own())],
            ),
            rule(
                other("crucible"),
                None,
                None,
                vec![clause(&g::WEAPON_TREE, own())],
            ),
            // `class-requirement`: no rule, so no row fits.
        ],
        spawn_exclusion: Unflagged {
            section: ModSection::Implicit,
            flags: eldritch_flags.clone(),
        },
        spawn_excluded_generations: vec![g::EXARCH, g::EATER],
        base_implicit: Unflagged {
            section: ModSection::Implicit,
            flags: eldritch_flags,
        },
        no_spawn_flags: vec!["crafted".to_owned()],
        sections: ["scourge", "crucible", "class-requirement"]
            .map(str::to_owned)
            .to_vec(),
        local_carriers: LocalCarriers {
            weapon: "weapon".to_owned(),
            item: "armour".to_owned(),
        },
        influences: INFLUENCE_TYPES
            .iter()
            .map(|(n, v)| ((*n).to_owned(), *v))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use loom_serialization::legacy::pob_item::{Section, domain_section};

    use super::*;

    /// The PoE1 rules survive the bundle's JSON form (`data/item_rules.json`).
    #[test]
    fn item_rules_should_round_trip_through_bundle_json() {
        let rules = item_rules();
        let json = serde_json::to_string(&rules).unwrap();
        assert_eq!(serde_json::from_str::<ItemRules>(&json).unwrap(), rules);
    }

    #[test]
    fn mod_rules_should_validate() {
        assert_eq!(item_rules().mods.validate(), Ok(()));
    }

    /// Every PoB section but class requirement has a fit rule, so no
    /// section's lines silently fit no row.
    #[test]
    fn mod_rules_should_cover_every_pob_section_but_class_requirement() {
        let rules = item_rules().mods;
        let covered: Vec<&str> = [
            Section::Enchant,
            Section::Scourge,
            Section::ClassRequirement,
            Section::Implicit,
            Section::Crucible,
            Section::Explicit,
        ]
        .into_iter()
        .filter(|s| rules.fits.iter().any(|r| r.section == domain_section(*s)))
        .map(Section::name)
        .collect();
        assert_eq!(
            covered,
            ["enchant", "scourge", "implicit", "crucible", "explicit"]
        );
    }
}
