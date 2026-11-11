//! PoE2 item rules for the PoE2 data bundle (Phase 2 plan, step 3b).
//!
//! **Hypothesis, not measured.** PoE2 item import is backlog (ADR-005), so
//! no PoE2 item has been resolved with these rules. They use the vendored
//! dat schema's enums, which the schema marks valid for both games
//! (`ModDomains` and `ModGenerationType`, `validFor: 3`), with only the
//! fit rules both games share: explicit affixes and unique mods in the
//! base's domain, unique and corrupted implicits, and enchantments. PoE2
//! has no PoE1-only section, no stat-description scope by mod domain that
//! has been checked, and no client-composed line or item-property text
//! named yet. Measure and extend them when PoE2 item import starts.

use loom_domain::ModSection;
use loom_parser::import::mods::{
    DomainMatch, FitClause, FitRule, LocalCarriers, ModRules, Unflagged,
};
use loom_parser::import::{ItemRules, TextRules};

use crate::poe1::{domain, generation};

fn own(generations: &[i64]) -> FitClause {
    FitClause {
        generations: generations.to_vec(),
        domains: DomainMatch {
            own: true,
            ..DomainMatch::default()
        },
        ..FitClause::default()
    }
}

/// The PoE2 item rules (module docs).
#[must_use]
pub fn item_rules() -> ItemRules {
    use generation as g;
    let unflagged_implicit = Unflagged {
        section: ModSection::Implicit,
        flags: Vec::new(),
    };
    ItemRules {
        default_scope: "stat_descriptions".to_owned(),
        domain_scopes: Vec::new(),
        mods: ModRules {
            fits: vec![
                FitRule {
                    section: ModSection::Explicit,
                    flag: None,
                    unique: Some(true),
                    clauses: vec![own(&[g::UNIQUE])],
                },
                FitRule {
                    section: ModSection::Explicit,
                    flag: None,
                    unique: None,
                    clauses: vec![FitClause {
                        essence_class: true,
                        ..own(&[g::PREFIX, g::SUFFIX])
                    }],
                },
                FitRule {
                    section: ModSection::Implicit,
                    flag: None,
                    unique: None,
                    clauses: vec![own(&[g::UNIQUE, g::CORRUPTED])],
                },
                FitRule {
                    section: ModSection::Enchant,
                    flag: None,
                    unique: None,
                    clauses: vec![FitClause {
                        domains: DomainMatch {
                            own: true,
                            fixed: vec![domain::ITEM],
                            related: Vec::new(),
                        },
                        ..own(&[g::ENCHANTMENT, g::UNIQUE])
                    }],
                },
            ],
            spawn_exclusion: unflagged_implicit.clone(),
            spawn_excluded_generations: Vec::new(),
            base_implicit: unflagged_implicit,
            no_spawn_flags: Vec::new(),
            local_carriers: LocalCarriers {
                weapon: "weapon".to_owned(),
                item: "armour".to_owned(),
            },
            sections: Vec::new(),
            influences: Vec::new(),
        },
        text: TextRules::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_rules_should_validate() {
        assert_eq!(item_rules().mods.validate(), Ok(()));
    }
}
