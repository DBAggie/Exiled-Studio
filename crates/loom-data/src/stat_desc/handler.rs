//! Variant handlers: what a handler after the format text does to a value.
//!
//! - **Numeric** handlers transform a stat value before formatting. Their
//!   semantics are a table ([`NUMERIC_HANDLERS`]); each entry records its
//!   [`Basis`]. `PobCited` entries reproduce
//!   `v2.67.2:src/Modules/StatDescriber.lua` `applySpecial` (rounding with
//!   `round` from `v2.67.2:src/Modules/Common.lua`, `floor(x * 10^d + 0.5)`;
//!   the value format `%d`, which truncates, unless PoB sets `fmt = "g"` or
//!   `".2f"`). `NameDerived` entries are not in PoB; their semantics are
//!   read from the handler name with the same conventions. Neither is game
//!   evidence: PR B2b's `render-check` compares the renderer with PoB's
//!   exported mod text, and its differences are categorized.
//! - **Lookup** handlers name a row of another table; the host supplies
//!   them through [`crate::stat_desc::render::Lookup`].
//! - **Display** handlers do not change values (`reminderstring`,
//!   `canonical_line`, `canonical_stat`: no-ops in `applySpecial`; `markup`:
//!   marks text that holds markup).
//! - Anything else is [`Handler::Unknown`]: kept, counted, and reported by
//!   the renderer, never ignored (`old_leech_percent`,
//!   `old_leech_permyriad`, `one_hundred_divide_by_value`: no reference and
//!   no unambiguous name).

/// Where a numeric handler's semantics come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Basis {
    /// Reproduces PoB v2.67.2 `applySpecial`.
    PobCited,
    /// Read from the handler name; not in PoB.
    NameDerived,
}

/// How a transformed value is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueFormat {
    /// C `%d`: truncated toward zero.
    Integer,
    /// C `%g`: six significant digits, trailing zeros removed.
    General,
    /// C `%.2f`.
    Fixed2,
}

/// A numeric handler: `value * num / den`, rounded to `round` decimals
/// (PoB `round`), times `then_mul`, plus `add`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumericHandler {
    /// Name as written in the game file.
    pub name: &'static str,
    /// Multiplier.
    pub num: i64,
    /// Divisor (never 0).
    pub den: i64,
    /// Decimals to round to after the division.
    pub round: Option<u8>,
    /// Multiplier after rounding.
    pub then_mul: i64,
    /// Addend after everything else.
    pub add: i64,
    /// The format this handler sets; `None` keeps the current one.
    pub format: Option<ValueFormat>,
    /// Source of the semantics.
    pub basis: Basis,
}

impl NumericHandler {
    /// Applies the handler to a value.
    #[must_use]
    pub fn apply(&self, value: f64) -> f64 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "handler constants are small integers"
        )]
        let (num, den, then_mul, add) = (
            self.num as f64,
            self.den as f64,
            self.then_mul as f64,
            self.add as f64,
        );
        let scaled = value * num / den;
        let rounded = self.round.map_or(scaled, |d| pob_round(scaled, d));
        rounded * then_mul + add
    }
}

/// PoB's `round(val, dec)`: `floor(val * 10^dec + 0.5) / 10^dec`.
#[must_use]
pub fn pob_round(value: f64, decimals: u8) -> f64 {
    let factor = 10_f64.powi(i32::from(decimals));
    (value * factor + 0.5).floor() / factor
}

const fn n(
    name: &'static str,
    num: i64,
    den: i64,
    round: Option<u8>,
    format: Option<ValueFormat>,
    basis: Basis,
) -> NumericHandler {
    NumericHandler {
        name,
        num,
        den,
        round,
        then_mul: 1,
        add: 0,
        format,
        basis,
    }
}

const fn with(mut h: NumericHandler, then_mul: i64, add: i64) -> NumericHandler {
    h.then_mul = then_mul;
    h.add = add;
    h
}

use Basis::{NameDerived, PobCited};
use ValueFormat::{Fixed2, General};
const G: Option<ValueFormat> = Some(General);

/// Every numeric handler with known semantics, sorted by name.
pub const NUMERIC_HANDLERS: &[NumericHandler] = &[
    n("30%_of_value", 3, 10, None, None, PobCited),
    n("60%_of_value", 6, 10, None, None, PobCited),
    with(n("add_one", 1, 1, None, None, NameDerived), 1, 1),
    n(
        "deciseconds_to_seconds",
        1,
        10,
        None,
        Some(Fixed2),
        PobCited,
    ),
    n("divide_by_fifteen_0dp", 1, 15, None, None, PobCited),
    n("divide_by_fifty", 1, 50, None, G, NameDerived),
    n("divide_by_five", 1, 5, None, G, PobCited),
    n("divide_by_four", 1, 4, None, G, PobCited),
    n("divide_by_one_hundred", 1, 100, None, G, PobCited),
    n("divide_by_one_hundred_0dp", 1, 100, None, None, NameDerived),
    n("divide_by_one_hundred_1dp", 1, 100, Some(1), G, PobCited),
    n("divide_by_one_hundred_2dp", 1, 100, Some(2), G, PobCited),
    n(
        "divide_by_one_hundred_2dp_if_required",
        1,
        100,
        Some(2),
        G,
        PobCited,
    ),
    with(
        n(
            "divide_by_one_hundred_and_negate",
            1,
            100,
            Some(1),
            G,
            PobCited,
        ),
        -1,
        0,
    ),
    n("divide_by_one_thousand", 1, 1000, None, G, PobCited),
    n("divide_by_six", 1, 6, None, G, PobCited),
    n("divide_by_ten_0dp", 1, 10, None, None, PobCited),
    n("divide_by_ten_1dp", 1, 10, Some(1), G, PobCited),
    n("divide_by_ten_1dp_if_required", 1, 10, Some(1), G, PobCited),
    n(
        "divide_by_ten_thousand_1dp",
        1,
        10_000,
        Some(1),
        G,
        NameDerived,
    ),
    n("divide_by_three", 1, 3, None, G, NameDerived),
    n("divide_by_twelve", 1, 12, None, G, PobCited),
    n("divide_by_twenty", 1, 20, None, G, PobCited),
    with(
        n(
            "divide_by_twenty_then_double_0dp",
            1,
            20,
            Some(0),
            None,
            PobCited,
        ),
        2,
        0,
    ),
    n("divide_by_two_0dp", 1, 2, None, None, PobCited),
    n("double", 2, 1, None, None, PobCited),
    with(n("invert_chance", -1, 1, None, None, PobCited), 1, 100),
    n("locations_to_metres", 1, 10, None, G, PobCited),
    n("milliseconds_to_seconds", 1, 1000, None, G, PobCited),
    n("milliseconds_to_seconds_0dp", 1, 1000, None, None, PobCited),
    n("milliseconds_to_seconds_1dp", 1, 1000, Some(1), G, PobCited),
    n("milliseconds_to_seconds_2dp", 1, 1000, Some(2), G, PobCited),
    n(
        "milliseconds_to_seconds_2dp_if_required",
        1,
        1000,
        Some(2),
        G,
        PobCited,
    ),
    n(
        "milliseconds_to_seconds_halved",
        1,
        2000,
        None,
        G,
        NameDerived,
    ),
    with(
        n("multiplicative_damage_modifier", 1, 1, None, None, PobCited),
        1,
        100,
    ),
    with(
        n(
            "multiplicative_permyriad_damage_modifier",
            1,
            100,
            Some(1),
            G,
            PobCited,
        ),
        1,
        100,
    ),
    n("multiply_by_four", 4, 1, None, None, PobCited),
    n("multiply_by_one_hundred", 100, 1, None, None, NameDerived),
    n("multiply_by_ten", 10, 1, None, None, NameDerived),
    n("negate", -1, 1, None, None, PobCited),
    n("negate_and_double", -2, 1, None, None, PobCited),
    n("per_minute_to_per_second", 1, 60, Some(1), G, PobCited),
    n("per_minute_to_per_second_0dp", 1, 60, None, None, PobCited),
    n("per_minute_to_per_second_1dp", 1, 60, Some(1), G, PobCited),
    n("per_minute_to_per_second_2dp", 1, 60, Some(2), G, PobCited),
    n(
        "per_minute_to_per_second_2dp_if_required",
        1,
        60,
        Some(2),
        G,
        PobCited,
    ),
    n(
        "permyriad_per_minute_to_%_per_second",
        1,
        6000,
        Some(1),
        G,
        PobCited,
    ),
    with(n("plus_two_hundred", 1, 1, None, None, PobCited), 1, 200),
    with(n("subtract_one", 1, 1, None, None, NameDerived), 1, -1),
    n("times_one_point_five", 3, 2, None, None, PobCited),
    n("times_twenty", 20, 1, None, None, PobCited),
];

/// A handler that names a row of another table. The table each needs was
/// read from the vendored schema (`tools/extractor/schema/schema.min.json`)
/// in PR B1; see the Spike B context file. Confirmation against real
/// values is PR B2b work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LookupHandler {
    /// `affliction_reward_type`.
    AfflictionRewardType,
    /// `display_indexable_non_active_support`.
    DisplayIndexableNonActiveSupport,
    /// `display_indexable_skill`.
    DisplayIndexableSkill,
    /// `display_indexable_support`.
    DisplayIndexableSupport,
    /// `mages_legacy_index` (PoE2).
    MagesLegacyIndex,
    /// `mod_value_to_item_class`.
    ModValueToItemClass,
    /// `passive_hash`.
    PassiveHash,
    /// `passive_keystone_index` (PoE2).
    PassiveKeystoneIndex,
    /// `specific_skill` (PoE2).
    SpecificSkill,
    /// `tempest_mod_text`.
    TempestModText,
    /// `tree_expansion_jewel_passive`.
    TreeExpansionJewelPassive,
    /// `ultimatum_wager_type_hash` (PoE2).
    UltimatumWagerTypeHash,
    /// `weapon_tree_unique_base_type_name`.
    WeaponTreeUniqueBaseTypeName,
}

impl LookupHandler {
    /// Every lookup handler.
    pub const ALL: [Self; 13] = [
        Self::AfflictionRewardType,
        Self::DisplayIndexableNonActiveSupport,
        Self::DisplayIndexableSkill,
        Self::DisplayIndexableSupport,
        Self::MagesLegacyIndex,
        Self::ModValueToItemClass,
        Self::PassiveHash,
        Self::PassiveKeystoneIndex,
        Self::SpecificSkill,
        Self::TempestModText,
        Self::TreeExpansionJewelPassive,
        Self::UltimatumWagerTypeHash,
        Self::WeaponTreeUniqueBaseTypeName,
    ];

    /// The name as written in the game file.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AfflictionRewardType => "affliction_reward_type",
            Self::DisplayIndexableNonActiveSupport => "display_indexable_non_active_support",
            Self::DisplayIndexableSkill => "display_indexable_skill",
            Self::DisplayIndexableSupport => "display_indexable_support",
            Self::MagesLegacyIndex => "mages_legacy_index",
            Self::ModValueToItemClass => "mod_value_to_item_class",
            Self::PassiveHash => "passive_hash",
            Self::PassiveKeystoneIndex => "passive_keystone_index",
            Self::SpecificSkill => "specific_skill",
            Self::TempestModText => "tempest_mod_text",
            Self::TreeExpansionJewelPassive => "tree_expansion_jewel_passive",
            Self::UltimatumWagerTypeHash => "ultimatum_wager_type_hash",
            Self::WeaponTreeUniqueBaseTypeName => "weapon_tree_unique_base_type_name",
        }
    }

    /// The lookup handler with this name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|h| h.name() == name)
    }
}

/// A handler of a variant. Stat numbers are 0-based here (the file and the
/// extract JSON write them 1-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handler {
    /// Transforms the value of stat `stat`.
    Numeric {
        /// Semantics.
        handler: &'static NumericHandler,
        /// 0-based stat index.
        stat: usize,
    },
    /// Names a row of another table by the value of stat `stat`.
    Lookup {
        /// Which lookup.
        handler: LookupHandler,
        /// 0-based stat index.
        stat: usize,
    },
    /// `reminderstring <id>`: adds reminder text (not rendered).
    Reminder(String),
    /// `canonical_line`: display only.
    CanonicalLine,
    /// `markup`: display only (the text holds markup).
    Markup,
    /// `canonical_stat <n>`: display only.
    CanonicalStat {
        /// 0-based stat index.
        stat: usize,
    },
    /// A known or unknown handler whose argument is missing or out of range.
    Malformed {
        /// Name as written.
        name: String,
    },
    /// A handler with no known semantics, kept as written.
    Unknown {
        /// Name as written.
        name: String,
        /// 0-based stat index, if it has one.
        stat: Option<usize>,
    },
}

/// Coarse kind of a handler, for counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HandlerKind {
    /// [`Handler::Numeric`] with [`Basis::PobCited`].
    NumericPobCited,
    /// [`Handler::Numeric`] with [`Basis::NameDerived`].
    NumericNameDerived,
    /// [`Handler::Lookup`].
    Lookup,
    /// Display-only handlers.
    Display,
    /// [`Handler::Unknown`].
    Unknown,
    /// [`Handler::Malformed`].
    Malformed,
}

impl HandlerKind {
    /// Stable name for reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NumericPobCited => "numeric-pob-cited",
            Self::NumericNameDerived => "numeric-name-derived",
            Self::Lookup => "lookup",
            Self::Display => "display",
            Self::Unknown => "unknown",
            Self::Malformed => "malformed",
        }
    }
}

impl Handler {
    /// Classifies a handler as written: `name`, its 1-based stat, and its
    /// reminder ID. `None` if a required argument is missing or out of
    /// range for `stats` stats.
    #[must_use]
    pub fn classify(
        name: &str,
        stat: Option<usize>,
        reminder: Option<&str>,
        stats: usize,
    ) -> Option<Self> {
        let index = match stat {
            Some(s) if (1..=stats).contains(&s) => Some(s - 1),
            Some(_) => return None,
            None => None,
        };
        Some(match name {
            "reminderstring" => Self::Reminder(reminder?.to_owned()),
            "canonical_line" => Self::CanonicalLine,
            "markup" => Self::Markup,
            "canonical_stat" => Self::CanonicalStat { stat: index? },
            _ => {
                if let Some(handler) = NUMERIC_HANDLERS.iter().find(|h| h.name == name) {
                    Self::Numeric {
                        handler,
                        stat: index?,
                    }
                } else if let Some(handler) = LookupHandler::from_name(name) {
                    Self::Lookup {
                        handler,
                        stat: index?,
                    }
                } else {
                    Self::Unknown {
                        name: name.to_owned(),
                        stat: index,
                    }
                }
            }
        })
    }

    /// The name as written.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Numeric { handler, .. } => handler.name,
            Self::Lookup { handler, .. } => handler.name(),
            Self::Reminder(_) => "reminderstring",
            Self::CanonicalLine => "canonical_line",
            Self::Markup => "markup",
            Self::CanonicalStat { .. } => "canonical_stat",
            Self::Unknown { name, .. } | Self::Malformed { name } => name,
        }
    }

    /// Coarse kind, for counts.
    #[must_use]
    pub const fn kind(&self) -> HandlerKind {
        match self {
            Self::Numeric { handler, .. } => match handler.basis {
                Basis::PobCited => HandlerKind::NumericPobCited,
                Basis::NameDerived => HandlerKind::NumericNameDerived,
            },
            Self::Lookup { .. } => HandlerKind::Lookup,
            Self::Reminder(_) | Self::CanonicalLine | Self::Markup | Self::CanonicalStat { .. } => {
                HandlerKind::Display
            }
            Self::Unknown { .. } => HandlerKind::Unknown,
            Self::Malformed { .. } => HandlerKind::Malformed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numeric(name: &str) -> &'static NumericHandler {
        NUMERIC_HANDLERS.iter().find(|h| h.name == name).unwrap()
    }

    #[test]
    fn numeric_handlers_should_be_sorted_unique_and_never_divide_by_zero() {
        let names: Vec<_> = NUMERIC_HANDLERS.iter().map(|h| h.name).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(names, sorted);
        assert!(NUMERIC_HANDLERS.iter().all(|h| h.den != 0));
    }

    #[test]
    fn lookup_names_should_round_trip_and_not_overlap_numeric_names() {
        for h in LookupHandler::ALL {
            assert_eq!(LookupHandler::from_name(h.name()), Some(h));
            assert!(NUMERIC_HANDLERS.iter().all(|n| n.name != h.name()));
        }
    }

    #[test]
    fn apply_should_follow_pob_rounding_for_dp_handlers() {
        // round(15 / 10, 1) = 1.5; round(-25 / 10, 0) uses floor(x + 0.5).
        assert!((numeric("divide_by_ten_1dp").apply(15.0) - 1.5).abs() < 1e-12);
        assert!((numeric("per_minute_to_per_second").apply(100.0) - 1.7).abs() < 1e-12);
        assert!((pob_round(-2.5, 0) - -2.0).abs() < 1e-12);
    }

    #[test]
    fn apply_should_round_before_negating_and_doubling() {
        assert!((numeric("divide_by_one_hundred_and_negate").apply(125.0) - -1.3).abs() < 1e-12);
        assert!((numeric("divide_by_twenty_then_double_0dp").apply(30.0) - 4.0).abs() < 1e-12);
    }

    #[test]
    fn apply_should_add_after_scaling() {
        assert!((numeric("invert_chance").apply(30.0) - 70.0).abs() < 1e-12);
        assert!((numeric("multiplicative_damage_modifier").apply(-20.0) - 80.0).abs() < 1e-12);
    }

    #[test]
    fn classify_should_sort_handlers_into_kinds() {
        let kind =
            |name, stat, reminder| Handler::classify(name, stat, reminder, 2).map(|h| h.kind());
        assert_eq!(
            kind("negate", Some(1), None),
            Some(HandlerKind::NumericPobCited)
        );
        assert_eq!(
            kind("add_one", Some(2), None),
            Some(HandlerKind::NumericNameDerived)
        );
        assert_eq!(
            kind("passive_hash", Some(1), None),
            Some(HandlerKind::Lookup)
        );
        assert_eq!(
            kind("reminderstring", None, Some("R")),
            Some(HandlerKind::Display)
        );
        assert_eq!(kind("markup", None, None), Some(HandlerKind::Display));
        assert_eq!(
            kind("old_leech_percent", Some(1), None),
            Some(HandlerKind::Unknown)
        );
    }

    #[test]
    fn classify_should_reject_missing_or_out_of_range_arguments() {
        assert_eq!(Handler::classify("negate", None, None, 1), None);
        assert_eq!(Handler::classify("negate", Some(2), None, 1), None);
        assert_eq!(Handler::classify("negate", Some(0), None, 1), None);
        assert_eq!(Handler::classify("reminderstring", None, None, 1), None);
    }

    #[test]
    fn classify_should_keep_an_unknown_handler_with_its_stat() {
        assert_eq!(
            Handler::classify("brand_new", Some(1), None, 1),
            Some(Handler::Unknown {
                name: "brand_new".to_owned(),
                stat: Some(0)
            })
        );
    }
}
