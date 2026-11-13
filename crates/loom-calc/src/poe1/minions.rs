//! Minion modifiers that also affect you (Phase 3 plan, step 8; P3-8j):
//! "Increases and Reductions to Minion Damage also affect you" (Spiritual
//! Aid), "Increases and Reductions to Minion Attack Speed also affect you"
//! (Spiritual Command), and their kin.
//!
//! What PoB v2.67.2 does:
//! - `Modules/ModParser.lua` reads "Minions deal #% increased Damage",
//!   "Minions have #% increased Attack Speed", "... Cast Speed", and "...
//!   Attack and Cast Speed" as `MinionModifier` LIST modifiers holding
//!   `Damage` INC, `Speed` INC with `ModFlag.Attack`, with `ModFlag.Cast`,
//!   and with no flag (its "minion" word, 1068, and the speed words);
//! - it reads the redirects as flags (3455 to 3462):
//!   `MinionDamageAppliesToPlayer` (with "at #% of their value", also
//!   `ImprovedMinionDamageAppliesToPlayer` MAX #),
//!   `MinionAttackSpeedAppliesToPlayer`, `MinionCastSpeedAppliesToPlayer`;
//! - `Modules/CalcOffence.lua` 613 to 643, for the skill being computed:
//!   with a flag set, every `MinionModifier` of the kind becomes the
//!   player's own modifier at the multiplier (`Improved...` MAX, else
//!   100%), keeping its source and its tags: a `Damage` INC as `Damage`
//!   INC with its flags; a `Speed` INC with no flag or the Attack (Cast)
//!   flag as `Speed` INC with `ModFlag.Attack` (`ModFlag.Cast`). It does
//!   not floor them (only the life redirect, `CalcPerform.lua` 1566, which
//!   reaches no stat here, floors).
//!
//! **The engine.** [`STATS`] maps the minion stats to
//! [`Poe1Quantity::Minion`] (`Increased`) and the redirects to
//! [`Poe1Quantity::MinionToYou`] (`Flag`, with the percent of their value:
//! a redirect line has no value, so its stat value is not read).
//! [`minions`] runs once every source is in: for each kind with a
//! redirect, the largest percent counts (PoB's flag with its MAX), and
//! each minion effect of that kind becomes the character's (`Damage` or
//! `Speed` `Increased`, the speed for attacks or casts) at that percent,
//! with the minion effect's source and conditions. A minion effect without
//! a redirect changes no slice field: no minion is computed.
//!
//! A gem's own minion stats (Minion Damage, Minion Speed, Predator,
//! Infernal Legion) are minion effects too: only the main skill's gems are
//! read through the gem mapping ([`super::mapping::SkillMapping`]), so
//! they are the `MinionModifier` entries of PoB's `skillModList`, which
//! `CalcOffence.lua` converts with the rest (Absolution or Dominating Blow
//! supported by Minion Damage, with Spiritual Aid). Other groups' gems do
//! not reach this mapping, as their minion stats do not reach the skill in
//! PoB.
//!
//! The redirects PoB reads that the engine does not are [`DEPENDS_ON`]:
//! they stay reasons. Other minion stats PoB reads as `Damage` or `Speed`
//! INC (minion melee damage, damage on full or low life, ...) are
//! unmapped: reasons. Every redirect row is unconditional (a test), so
//! [`minions`] reads no condition on a redirect.

use super::Poe1Quantity;
use crate::condition::Flags;
use crate::ingest::Ingest;
use crate::quantity::{Op, Quantity};
use crate::store::Effect;

/// A kind of minion modifier the character can take on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MinionValue {
    /// Increased damage.
    Damage,
    /// Increased attack speed.
    AttackSpeed,
    /// Increased cast speed.
    CastSpeed,
}

impl MinionValue {
    /// Every kind.
    pub const ALL: [Self; 3] = [Self::Damage, Self::AttackSpeed, Self::CastSpeed];

    /// The kind's name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::AttackSpeed => "attack-speed",
            Self::CastSpeed => "cast-speed",
        }
    }

    /// The character's quantity and the flags the kind's effects take on.
    const fn target(self) -> (Quantity, Flags) {
        match self {
            Self::Damage => (Quantity::Damage, Flags::NONE),
            Self::AttackSpeed => (Quantity::Speed, Flags::ATTACK),
            Self::CastSpeed => (Quantity::Speed, Flags::CAST),
        }
    }
}

const fn minion(value: MinionValue) -> Poe1Quantity {
    Poe1Quantity::Minion(value)
}

const fn to_you(value: MinionValue, percent: u16) -> Poe1Quantity {
    Poe1Quantity::MinionToYou(value, percent)
}

/// One stat: its quantities and the scale per unit of its value.
type MinionRow = (&'static str, &'static [(Poe1Quantity, f64)]);

/// The minion stats and the redirects, sorted by stat ID (module docs).
/// Each description is the game's (`stat_descriptions.txt`):
/// - "Minions deal #% increased Damage", "Minions have #% increased Attack
///   Speed", "... Cast Speed", "... Attack and Cast Speed";
/// - "Increases and Reductions to Minion Damage also affect you" (its
///   value 1, so 100%), "... at 150% of their value", "Increases and
///   Reductions to Minion Attack Speed also affect you", "... Cast Speed
///   ...".
pub const STATS: [MinionRow; 8] = [
    (
        "additive_minion_damage_modifiers_apply_to_you_at_150%_value",
        &[(to_you(MinionValue::Damage, 150), 1.0)],
    ),
    (
        "additive_modifiers_to_minion_attack_speed_also_affect_you",
        &[(to_you(MinionValue::AttackSpeed, 100), 1.0)],
    ),
    (
        "additive_modifiers_to_minion_cast_speed_also_affect_you",
        &[(to_you(MinionValue::CastSpeed, 100), 1.0)],
    ),
    (
        "minion_attack_and_cast_speed_+%",
        &[
            (minion(MinionValue::AttackSpeed), 1.0),
            (minion(MinionValue::CastSpeed), 1.0),
        ],
    ),
    (
        "minion_attack_speed_+%",
        &[(minion(MinionValue::AttackSpeed), 1.0)],
    ),
    (
        "minion_cast_speed_+%",
        &[(minion(MinionValue::CastSpeed), 1.0)],
    ),
    ("minion_damage_+%", &[(minion(MinionValue::Damage), 1.0)]),
    (
        "minion_damage_increases_and_reductions_also_affects_you",
        &[(to_you(MinionValue::Damage, 100), 1.0)],
    ),
];

/// Redirects PoB reads that the engine does not, sorted: they must stay
/// reasons while minion modifiers are only redirected by [`STATS`]. "...
/// also affect Dominating Blow and Absolution at #% of their value"
/// (`ModParser.lua` 3457: Dominating Blow and Absolution only), the skill
/// stat that applies them to all damage (`Data/SkillStatMap.lua` 905),
/// "Modifiers to Minion Damage also affect you", which PoB does not read
/// at all (a line the engine would apply where PoB does not: a divergence
/// to settle first), and `additive_minion_damage_modifiers_apply_to_you_at_%_value`,
/// which has no description, so no line resolves to it.
pub const DEPENDS_ON: &[&str] = &[
    "active_skill_additive_minion_damage_modifiers_apply_to_all_damage_at_%_value",
    "additive_minion_damage_modifiers_apply_to_you_at_%_value",
    "dominating_blow_and_absolution_additive_minion_damage_modifiers_apply_to_you_at_150%_value",
    "modifiers_to_minion_damage_also_affect_you",
];

/// The effects of a stat of [`STATS`]: `(quantity, op, scale)`.
#[must_use]
pub fn effects(stat: &str) -> Option<Vec<(Quantity, Op, f64)>> {
    let i = STATS.binary_search_by(|(id, _)| (*id).cmp(stat)).ok()?;
    Some(
        STATS[i]
            .1
            .iter()
            .map(|&(q, scale)| {
                let op = match q {
                    Poe1Quantity::MinionToYou(..) => Op::Flag,
                    _ => Op::Increased,
                };
                (Quantity::Poe1(q), op, scale)
            })
            .collect(),
    )
}

/// Whether `quantity` is a minion modifier (no field of the character
/// reads it but [`minions`]).
#[must_use]
pub const fn is_minion_quantity(quantity: Quantity) -> bool {
    matches!(
        quantity,
        Quantity::Poe1(Poe1Quantity::Minion(_) | Poe1Quantity::MinionToYou(..))
    )
}

/// Adds the minion modifiers that also affect the character (module
/// docs). Runs after every other source has been ingested.
pub fn minions(ingest: &mut Ingest<'_>) {
    for kind in MinionValue::ALL {
        // The largest percent (PoB's flag, with `Improved...` MAX).
        // Every redirect row is unconditional: a conditional one would be
        // a new row, which must come with its condition here.
        let percent = ingest
            .effects()
            .iter()
            .filter_map(|e| match (e.quantity, e.op) {
                (Quantity::Poe1(Poe1Quantity::MinionToYou(k, p)), Op::Flag) if k == kind => Some(p),
                _ => None,
            })
            .max();
        let Some(percent) = percent else { continue };
        let percent = f64::from(percent);
        let (quantity, flags) = kind.target();
        let added: Vec<Effect> = ingest
            .effects()
            .iter()
            .filter(|e| e.quantity == Quantity::Poe1(minion(kind)) && e.op == Op::Increased)
            .map(|e| Effect {
                quantity,
                op: Op::Increased,
                value: e.value * percent / 100.0,
                conditions: e.conditions.clone().with_all(flags),
                source: e.source.clone(),
            })
            .collect();
        for e in added {
            ingest.effect(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition::Conditions;
    use crate::ingest::{StatMap, StatMapping};
    use crate::poe1::ArmourSlot;
    use crate::poe1::mapping::{PieceMapping, Poe1Mapping, SkillMapping, WeaponMapping};
    use crate::source::Source;
    use loom_domain::StatId;

    #[test]
    fn stats_should_be_sorted_and_unique() {
        assert!(STATS.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(DEPENDS_ON.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn the_character_mapping_should_read_every_row() {
        for (stat, row) in STATS {
            let StatMap::Effects(effects) = Poe1Mapping.map(&StatId::new(stat)) else {
                panic!("{stat} is not mapped");
            };
            assert_eq!(effects.len(), row.len(), "{stat}");
            for (t, &(q, scale)) in effects.iter().zip(row) {
                assert_eq!(t.quantity, Quantity::Poe1(q), "{stat}");
                assert!((t.scale - scale).abs() < f64::EPSILON, "{stat}");
                assert_eq!(t.conditions, Conditions::none(), "{stat}");
            }
        }
    }

    #[test]
    fn a_main_skill_gems_minion_stats_should_be_minion_effects() {
        for (stat, _) in STATS {
            assert_eq!(
                SkillMapping.map(&StatId::new(stat)),
                Poe1Mapping.map(&StatId::new(stat)),
                "{stat}"
            );
        }
    }

    #[test]
    fn a_gems_minion_damage_should_follow_the_redirect() {
        let gem = Source::Gem { group: 0, gem: 1 };
        let got = redirected(vec![
            effect(DAMAGE, Op::Increased, 30.0, gem.clone()),
            effect(to_you(MinionValue::Damage, 100), Op::Flag, 0.0, passive(1)),
        ]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].quantity, Quantity::Damage);
        assert!((got[0].value - 30.0).abs() < 1e-9);
        assert_eq!(got[0].source, gem);
    }

    #[test]
    fn other_minion_damage_stats_should_stay_unmapped() {
        for stat in ["minion_melee_damage_+%", "minion_damage_+%_on_full_life"] {
            assert_eq!(
                Poe1Mapping.map(&StatId::new(stat)),
                StatMap::Unmapped,
                "{stat}"
            );
        }
    }

    fn passive(node: u32) -> Source {
        Source::Passive {
            node: loom_domain::NodeId(node),
        }
    }

    fn effect(quantity: Poe1Quantity, op: Op, value: f64, source: Source) -> Effect {
        Effect {
            quantity: Quantity::Poe1(quantity),
            op,
            value,
            conditions: Conditions::none(),
            source,
        }
    }

    /// The character's `Damage` and `Speed` effects after [`minions`].
    fn redirected(effects: Vec<Effect>) -> Vec<Effect> {
        let mut ingest = Ingest::new(&Poe1Mapping);
        for e in effects {
            ingest.effect(e);
        }
        minions(&mut ingest);
        ingest
            .effects()
            .iter()
            .filter(|e| matches!(e.quantity, Quantity::Damage | Quantity::Speed))
            .cloned()
            .collect()
    }

    const DAMAGE: Poe1Quantity = Poe1Quantity::Minion(MinionValue::Damage);
    const ATTACK: Poe1Quantity = Poe1Quantity::Minion(MinionValue::AttackSpeed);
    const CAST: Poe1Quantity = Poe1Quantity::Minion(MinionValue::CastSpeed);

    #[test]
    fn minion_modifiers_without_a_redirect_should_reach_nothing() {
        assert!(
            redirected(vec![
                effect(DAMAGE, Op::Increased, 20.0, passive(1)),
                effect(ATTACK, Op::Increased, 10.0, passive(1)),
                effect(CAST, Op::Increased, 10.0, passive(1)),
            ])
            .is_empty()
        );
    }

    #[test]
    fn the_largest_damage_redirect_should_count_once() {
        let got = redirected(vec![
            effect(DAMAGE, Op::Increased, 20.0, passive(1)),
            effect(DAMAGE, Op::Increased, -4.0, passive(2)),
            effect(to_you(MinionValue::Damage, 100), Op::Flag, 0.0, passive(3)),
            effect(to_you(MinionValue::Damage, 150), Op::Flag, 1.0, passive(4)),
            effect(to_you(MinionValue::Damage, 100), Op::Flag, 1.0, passive(5)),
        ]);
        let want = [(20.0 * 1.5, passive(1)), (-4.0 * 1.5, passive(2))];
        assert_eq!(got.len(), want.len());
        for (e, (value, source)) in got.iter().zip(want) {
            assert_eq!(e.quantity, Quantity::Damage);
            assert_eq!(e.op, Op::Increased);
            assert!((e.value - value).abs() < 1e-9, "{e:?}");
            assert_eq!(e.conditions, Conditions::none());
            assert_eq!(e.source, source);
        }
    }

    #[test]
    fn each_speed_redirect_should_give_its_own_kind_only() {
        let got = redirected(vec![
            effect(ATTACK, Op::Increased, 10.0, passive(1)),
            effect(CAST, Op::Increased, 7.0, passive(2)),
            effect(DAMAGE, Op::Increased, 20.0, passive(3)),
            effect(
                to_you(MinionValue::AttackSpeed, 100),
                Op::Flag,
                0.0,
                passive(4),
            ),
        ]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].quantity, Quantity::Speed);
        assert!((got[0].value - 10.0).abs() < 1e-9);
        assert_eq!(
            got[0].conditions,
            Conditions::none().with_all(Flags::ATTACK)
        );
        assert_eq!(got[0].source, passive(1));

        let got = redirected(vec![
            effect(ATTACK, Op::Increased, 10.0, passive(1)),
            effect(CAST, Op::Increased, 7.0, passive(2)),
            effect(
                to_you(MinionValue::CastSpeed, 100),
                Op::Flag,
                0.0,
                passive(4),
            ),
        ]);
        assert_eq!(got.len(), 1);
        assert!((got[0].value - 7.0).abs() < 1e-9);
        assert_eq!(got[0].conditions, Conditions::none().with_all(Flags::CAST));
    }

    #[test]
    fn attack_and_cast_speed_should_reach_both_redirects() {
        let StatMap::Effects(templates) =
            Poe1Mapping.map(&StatId::new("minion_attack_and_cast_speed_+%"))
        else {
            panic!("not mapped");
        };
        let mut effects: Vec<Effect> = templates
            .iter()
            .map(|t| Effect {
                quantity: t.quantity,
                op: t.op,
                value: 6.0,
                conditions: t.conditions.clone(),
                source: passive(1),
            })
            .collect();
        effects.push(effect(
            to_you(MinionValue::AttackSpeed, 100),
            Op::Flag,
            0.0,
            passive(2),
        ));
        effects.push(effect(
            to_you(MinionValue::CastSpeed, 100),
            Op::Flag,
            0.0,
            passive(2),
        ));
        let got = redirected(effects);
        let flags: Vec<Flags> = got.iter().map(|e| e.conditions.all).collect();
        assert_eq!(flags, [Flags::ATTACK, Flags::CAST]);
        assert!(got.iter().all(|e| (e.value - 6.0).abs() < 1e-9));
    }

    #[test]
    fn a_minion_modifiers_conditions_should_be_kept() {
        let key = crate::condition::ConditionKey::new("UsingStaff");
        let mut e = effect(ATTACK, Op::Increased, 10.0, passive(1));
        e.conditions = Conditions::none().with_state(key.clone(), true);
        let got = redirected(vec![
            e,
            effect(
                to_you(MinionValue::AttackSpeed, 100),
                Op::Flag,
                0.0,
                passive(2),
            ),
        ]);
        assert_eq!(
            got[0].conditions,
            Conditions::none()
                .with_state(key, true)
                .with_all(Flags::ATTACK)
        );
    }

    #[test]
    fn the_redirects_left_should_stay_unmapped() {
        let piece = PieceMapping {
            slot: ArmourSlot::Helmet,
        };
        let mappings: [&dyn StatMapping; 4] = [&Poe1Mapping, &SkillMapping, &piece, &WeaponMapping];
        for &stat in DEPENDS_ON {
            for m in mappings {
                assert_eq!(m.map(&StatId::new(stat)), StatMap::Unmapped, "{stat}");
            }
        }
    }
}
