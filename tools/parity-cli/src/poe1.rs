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

use loom_parser::import::ItemRules;

/// The PoE1 item rules.
#[must_use]
pub fn item_rules() -> ItemRules {
    ItemRules {
        default_scope: "stat_descriptions".to_owned(),
        domain_scopes: vec![
            (34, "tincture_stat_descriptions".to_owned()),
            (38, "graft_stat_descriptions".to_owned()),
        ],
    }
}
