//! From stat IDs to effects (ADR-015; Phase 3 plan, decisions 1, 2, 11).
//!
//! A game's rules collect a build's content (item lines, passives, gems)
//! and hand each stat to [`Ingest`], which asks the game's
//! [`StatMapping`] for its effects and adds them to the store. Anything
//! that cannot become effects becomes a reason, never a silent zero:
//!
//! - a stat the mapping does not know: `UnmappedStat`;
//! - a stat whose value is not one number: `UnsettledValue`;
//! - a typed resolution without a handler: `UnhandledKind`;
//! - an unresolved, pending, or excluded line: its own reason (lines of a
//!   variant the item does not select have no effect in game and give
//!   none).
//!
//! Which fields a reason can reach is not known here, so every reason
//! applies to every computed field (the conservative reading of decision
//! 1); later steps may narrow a reason to the fields it reaches.

use std::collections::BTreeMap;

use loom_domain::{
    Exclusion, ItemId, LineResolution, Resolved, StatId, StatLine, StatValue, TypedResolution,
};

use crate::condition::Conditions;
use crate::input::LineValues;
use crate::quantity::{Op, Quantity};
use crate::source::Source;
use crate::state::{IndeterminateReason, Reasons, UnsupportedReason};
use crate::store::{Effect, ModStore};

/// One effect a stat gives, per unit of the stat's value.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectTemplate {
    /// What it changes.
    pub quantity: Quantity,
    /// How it combines.
    pub op: Op,
    /// The effect's value per unit of the stat (for example `-1` for a
    /// "reduced" stat).
    pub scale: f64,
    /// What the scaled value is divided by: the effect is `value x scale /
    /// divisor`, one IEEE operation each, so a stat in hundredths (`/ 100`)
    /// gives the bits the game's rule gives, which a scale of `0.01` would
    /// not always. `1` for most stats.
    pub divisor: f64,
    /// When it applies.
    pub conditions: Conditions,
}

/// What the mapping says about a stat.
#[derive(Debug, Clone, PartialEq)]
pub enum StatMap {
    /// The stat's effects.
    Effects(Vec<EffectTemplate>),
    /// The stat is known and reaches no field the engine computes.
    NoEffect,
    /// The stat is not known.
    Unmapped,
}

/// What a handler says about a typed resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedMap {
    /// Its effects.
    Effects(Vec<Effect>),
    /// Known; it reaches no field the engine computes.
    NoEffect,
    /// No handler: the fields become `Partial` with `UnhandledKind`.
    Unhandled,
}

/// A game's stat-to-effect mapping (data where possible, code where
/// necessary; PoE1's lives in `loom_calc::poe1`).
pub trait StatMapping {
    /// The effects of `stat`.
    fn map(&self, stat: &StatId) -> StatMap;

    /// The effects of a typed resolution from `source`. The default has no
    /// handler.
    fn typed(&self, resolution: &TypedResolution, source: &Source) -> TypedMap {
        let _ = (resolution, source);
        TypedMap::Unhandled
    }
}

/// A stat value as one number: an exact value shown as itself, or the
/// assumed value of a hidden one. `None` otherwise.
///
/// A hidden value's assumption is accepted as the value on purpose: the
/// resolver records it only when the description's condition makes it
/// clear (`StatValue::Hidden`), so the field stays `Complete`. If a later
/// step finds assumptions that are guesses, they get their own reason.
#[must_use]
pub fn settle(value: &StatValue) -> Option<f64> {
    let raw = match value {
        StatValue::Exact { min, max } if min == max => *min,
        StatValue::Hidden {
            assumed: Some(value),
        } => *value,
        StatValue::Exact { .. }
        | StatValue::Ranged { .. }
        | StatValue::Hidden { assumed: None }
        | StatValue::Lookup(_) => return None,
    };
    // Stat values are small integers; the conversion is exact below 2^53.
    #[expect(
        clippy::cast_precision_loss,
        reason = "stat values are far below 2^53, where i64 to f64 is exact"
    )]
    Some(raw as f64)
}

/// The host's settled values of item lines, by item and line index
/// ([`CalcOptions::item_lines`](crate::CalcOptions::item_lines)).
pub type ItemLines = BTreeMap<ItemId, BTreeMap<usize, LineValues>>;

/// A line's stats with the host's rolled values in place of its ranged
/// ones; `None` (the line keeps its own values, so a ranged one stays
/// unsettled) when the host gives another number of stats or rolls a stat
/// that is not ranged or a value outside its range.
fn rolled(stats: &[StatLine], values: &[Option<i64>]) -> Option<Vec<StatLine>> {
    if stats.len() != values.len() {
        return None;
    }
    stats
        .iter()
        .zip(values)
        .map(|(s, v)| match (&s.value, v) {
            (_, None) => Some(s.clone()),
            (StatValue::Ranged { min, max }, Some(v)) if (*min..=*max).contains(v) => {
                Some(StatLine {
                    stat: s.stat.clone(),
                    value: StatValue::Exact { min: *v, max: *v },
                })
            }
            _ => None,
        })
        .collect()
}

/// Builds the modifier store from a build's content.
pub struct Ingest<'m> {
    mapping: &'m dyn StatMapping,
    item_lines: Option<&'m ItemLines>,
    store: ModStore,
    reasons: Reasons,
}

impl core::fmt::Debug for Ingest<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Ingest")
            .field("store", &self.store)
            .field("reasons", &self.reasons)
            .finish_non_exhaustive()
    }
}

impl<'m> Ingest<'m> {
    /// An empty store over `mapping`.
    #[must_use]
    pub fn new(mapping: &'m dyn StatMapping) -> Self {
        Self {
            mapping,
            item_lines: None,
            store: ModStore::new(),
            reasons: Reasons::new(),
        }
    }

    /// Takes the host's settled values of item lines ([`ItemLines`]):
    /// a `Rolled` line's ranged stats take the rolled values, a `Scaled`
    /// line gives `ScaledLine` and no effect.
    #[must_use]
    pub fn with_item_lines(mut self, item_lines: &'m ItemLines) -> Self {
        self.item_lines = Some(item_lines);
        self
    }

    /// Adds an effect the rules give directly (a character base value, an
    /// attribute's implicit effect).
    pub fn effect(&mut self, effect: Effect) {
        self.store.add(effect);
    }

    /// Records a reason that applies to every computed field.
    pub fn reason(&mut self, reason: UnsupportedReason) {
        self.reasons.partial.insert(reason);
    }

    /// Records a reason a value cannot be settled; it applies to every
    /// computed field.
    pub fn indeterminate(&mut self, reason: IndeterminateReason) {
        self.reasons.indeterminate.insert(reason);
    }

    /// Adds the effects of stat lines from `source`.
    pub fn stats(&mut self, source: &Source, stats: &[StatLine]) {
        let mapping = self.mapping;
        for line in stats {
            self.stat_with(mapping, source, line);
        }
    }

    /// [`Ingest::stats`] through another mapping (a gem's stats act only
    /// on the skill it supports).
    pub fn stats_with(&mut self, mapping: &dyn StatMapping, source: &Source, stats: &[StatLine]) {
        for line in stats {
            self.stat_with(mapping, source, line);
        }
    }

    /// Adds the effects of one stat line from `source`.
    pub fn stat(&mut self, source: &Source, line: &StatLine) {
        let mapping = self.mapping;
        self.stat_with(mapping, source, line);
    }

    fn stat_with(&mut self, mapping: &dyn StatMapping, source: &Source, line: &StatLine) {
        let templates = match mapping.map(&line.stat) {
            StatMap::NoEffect => return,
            StatMap::Unmapped => {
                self.reasons
                    .partial
                    .insert(UnsupportedReason::UnmappedStat {
                        stat: line.stat.clone(),
                        source: source.clone(),
                    });
                return;
            }
            StatMap::Effects(t) => t,
        };
        let Some(value) = settle(&line.value) else {
            self.reasons
                .indeterminate
                .insert(IndeterminateReason::UnsettledValue {
                    stat: line.stat.clone(),
                    source: source.clone(),
                });
            return;
        };
        for t in templates {
            self.store.add(Effect {
                quantity: t.quantity,
                op: t.op,
                value: value * t.scale / t.divisor,
                conditions: t.conditions,
                source: source.clone(),
            });
        }
    }

    /// Adds the effects of an item line by its resolution.
    pub fn item_line(&mut self, item: ItemId, line: usize, resolution: &LineResolution) {
        let mapping = self.mapping;
        self.item_line_with(mapping, item, line, resolution);
    }

    /// [`Ingest::item_line`] with another mapping for the line's stats and
    /// typed resolutions: an item whose local stats act on the item itself
    /// (an armour piece's own defences) maps them to its own quantities and
    /// leaves the others to the game's mapping.
    pub fn item_line_with(
        &mut self,
        mapping: &dyn StatMapping,
        item: ItemId,
        line: usize,
        resolution: &LineResolution,
    ) {
        let at = |adapted| Source::Item {
            item,
            line,
            adapted,
        };
        match resolution {
            // Not part of the item in game, or carried by an earlier line.
            LineResolution::Excluded(Exclusion::NotSelected) | LineResolution::Continues { .. } => {
            }
            LineResolution::Excluded(e @ (Exclusion::Custom | Exclusion::Section)) => {
                self.reason(UnsupportedReason::ExcludedLine {
                    source: at(None),
                    exclusion: e.name(),
                });
            }
            LineResolution::Pending => {
                self.reason(UnsupportedReason::PendingLine { source: at(None) });
            }
            LineResolution::Unresolved(r) => {
                self.reason(UnsupportedReason::UnresolvedLine {
                    source: at(None),
                    reason: r.name(),
                });
            }
            LineResolution::Resolved { result, adapted } => {
                let source = at(*adapted);
                let host = self
                    .item_lines
                    .and_then(|lines| lines.get(&item))
                    .and_then(|lines| lines.get(&line));
                if let Some(LineValues::Scaled(scaling)) = host {
                    self.reason(UnsupportedReason::ScaledLine {
                        source,
                        scaling: scaling.name(),
                    });
                    return;
                }
                match result {
                    Resolved::Stats(stats) => {
                        let rolled = match host {
                            Some(LineValues::Rolled(values)) => rolled(stats, values),
                            _ => None,
                        };
                        for s in rolled.as_deref().unwrap_or(stats) {
                            self.stat_with(mapping, &source, s);
                        }
                    }
                    Resolved::Typed(t) => self.typed_with(mapping, &source, t),
                }
            }
        }
    }

    /// Adds the effects of line `line` of custom modifier block `block`
    /// of the active config set: a character-wide modifier, through the
    /// game's mapping (as PoB adds the parsed line to the config's
    /// modifiers). A line without a usable resolution is a reason, as an
    /// item line's is: nothing selects or continues a custom line, so an
    /// exclusion is a reason too, and a continuation (which the build
    /// document refuses) is an unread line.
    pub fn custom_line(&mut self, block: usize, line: usize, resolution: &LineResolution) {
        let source = Source::CustomModifier { block, line };
        match resolution {
            LineResolution::Excluded(e) => self.reason(UnsupportedReason::ExcludedLine {
                source,
                exclusion: e.name(),
            }),
            LineResolution::Continues { .. } => {
                self.reason(UnsupportedReason::UnreadLine { source });
            }
            LineResolution::Pending => self.reason(UnsupportedReason::PendingLine { source }),
            LineResolution::Unresolved(r) => self.reason(UnsupportedReason::UnresolvedLine {
                source,
                reason: r.name(),
            }),
            LineResolution::Resolved { result, .. } => match result {
                Resolved::Stats(stats) => {
                    for s in stats {
                        self.stat(&source, s);
                    }
                }
                Resolved::Typed(t) => self.typed(&source, t),
            },
        }
    }

    /// Adds the effects of a typed resolution from `source`.
    pub fn typed(&mut self, source: &Source, resolution: &TypedResolution) {
        let mapping = self.mapping;
        self.typed_with(mapping, source, resolution);
    }

    fn typed_with(
        &mut self,
        mapping: &dyn StatMapping,
        source: &Source,
        resolution: &TypedResolution,
    ) {
        match mapping.typed(resolution, source) {
            TypedMap::Effects(effects) => {
                for e in effects {
                    self.store.add(e);
                }
            }
            TypedMap::NoEffect => {}
            TypedMap::Unhandled => self.reason(UnsupportedReason::UnhandledKind {
                kind: resolution.kind(),
                source: source.clone(),
            }),
        }
    }

    /// The store and the reasons that apply to every computed field.
    #[must_use]
    pub fn finish(self) -> (ModStore, Reasons) {
        (self.store, self.reasons)
    }
}

#[cfg(test)]
mod tests {
    use loom_domain::{Adapted, GemId, UnresolvedReason};

    use super::*;

    /// Knows `life` (base) and `reduced_life` (increased, scale -1),
    /// and `ignored` (no effect).
    struct Map;

    impl StatMapping for Map {
        fn map(&self, stat: &StatId) -> StatMap {
            let t = |op, scale, divisor| EffectTemplate {
                quantity: Quantity::Life,
                op,
                scale,
                divisor,
                conditions: Conditions::none(),
            };
            match stat.as_str() {
                "life" => StatMap::Effects(vec![t(Op::Base, 1.0, 1.0)]),
                "reduced_life" => StatMap::Effects(vec![t(Op::Increased, -1.0, 1.0)]),
                "life_hundredths" => StatMap::Effects(vec![t(Op::Base, 1.0, 100.0)]),
                "ignored" => StatMap::NoEffect,
                _ => StatMap::Unmapped,
            }
        }
    }

    fn line(stat: &str, value: StatValue) -> StatLine {
        StatLine {
            stat: StatId::new(stat),
            value,
        }
    }

    fn exact(v: i64) -> StatValue {
        StatValue::Exact { min: v, max: v }
    }

    #[test]
    fn mapped_stats_should_become_scaled_effects() {
        let mut i = Ingest::new(&Map);
        i.stats(
            &Source::Character,
            &[line("life", exact(12)), line("reduced_life", exact(10))],
        );
        i.stats_with(
            &Map,
            &Source::Character,
            &[line("life_hundredths", exact(7))],
        );
        let (store, reasons) = i.finish();
        assert!(reasons.is_empty());
        assert_eq!(store.len(), 3);
        assert_eq!(store.effects()[1].value, -10.0);
        // Divided, as the game's rule: 7 / 100, not 7 x 0.01.
        assert_eq!(
            store.effects()[2].value.to_bits(),
            (7.0_f64 / 100.0).to_bits()
        );
    }

    #[test]
    fn an_unmapped_stat_should_give_a_reason_not_a_zero() {
        let mut i = Ingest::new(&Map);
        i.stat(&Source::Character, &line("mystery", exact(5)));
        i.stat(&Source::Character, &line("ignored", exact(5)));
        let (store, reasons) = i.finish();
        assert!(store.is_empty());
        assert_eq!(
            reasons.partial.into_iter().collect::<Vec<_>>(),
            vec![UnsupportedReason::UnmappedStat {
                stat: StatId::new("mystery"),
                source: Source::Character
            }]
        );
    }

    #[test]
    fn settle_should_take_one_number_only() {
        assert_eq!(settle(&exact(3)), Some(3.0));
        assert_eq!(settle(&StatValue::Hidden { assumed: Some(-2) }), Some(-2.0));
        assert_eq!(settle(&StatValue::Exact { min: 1, max: 2 }), None);
        assert_eq!(settle(&StatValue::Ranged { min: 1, max: 2 }), None);
        assert_eq!(settle(&StatValue::Hidden { assumed: None }), None);
        assert_eq!(settle(&StatValue::Lookup(vec![1])), None);
    }

    #[test]
    fn an_unsettled_value_should_be_indeterminate() {
        let mut i = Ingest::new(&Map);
        i.stat(
            &Source::Character,
            &line("life", StatValue::Ranged { min: 1, max: 9 }),
        );
        let (store, reasons) = i.finish();
        assert!(store.is_empty());
        assert_eq!(reasons.indeterminate.len(), 1);
    }

    #[test]
    fn item_lines_should_give_effects_or_reasons_by_resolution() {
        let item = ItemId(3);
        let mut i = Ingest::new(&Map);
        i.item_line(
            item,
            0,
            &LineResolution::Resolved {
                result: Resolved::Stats(vec![line("life", exact(40))]),
                adapted: Some(Adapted::CaseFolded),
            },
        );
        i.item_line(item, 1, &LineResolution::Excluded(Exclusion::NotSelected));
        i.item_line(item, 2, &LineResolution::Continues { line: 0 });
        i.item_line(item, 3, &LineResolution::Excluded(Exclusion::Custom));
        i.item_line(item, 4, &LineResolution::Excluded(Exclusion::Section));
        i.item_line(item, 5, &LineResolution::Pending);
        i.item_line(
            item,
            6,
            &LineResolution::Unresolved(UnresolvedReason::NoTemplate),
        );
        i.item_line(
            item,
            7,
            &LineResolution::Resolved {
                result: Resolved::Typed(TypedResolution::GrantedSkill {
                    source: "row".into(),
                    gem: GemId::new("gem"),
                    level: 1,
                }),
                adapted: None,
            },
        );
        let (store, reasons) = i.finish();
        assert_eq!(store.len(), 1);
        assert_eq!(
            store.effects()[0].source,
            Source::Item {
                item,
                line: 0,
                adapted: Some(Adapted::CaseFolded)
            },
            "the adapted flag reaches provenance"
        );
        let names: Vec<_> = reasons
            .partial
            .iter()
            .map(UnsupportedReason::name)
            .collect();
        assert_eq!(
            names,
            [
                "unresolved-line",
                "excluded-line",
                "excluded-line",
                "pending-line",
                "unhandled-kind"
            ]
        );
        assert!(reasons.indeterminate.is_empty());
    }

    fn ranged_line(stats: Vec<StatLine>) -> LineResolution {
        LineResolution::Resolved {
            result: Resolved::Stats(stats),
            adapted: None,
        }
    }

    #[test]
    fn the_hosts_rolled_values_should_settle_ranged_stats() {
        let item = ItemId(5);
        let lines: ItemLines = BTreeMap::from([(
            item,
            BTreeMap::from([
                // Rolled: the ranged stat takes 14, the exact one its own.
                (0, LineValues::Rolled(vec![None, Some(14)])),
                // Another number of stats: ignored, stays unsettled.
                (1, LineValues::Rolled(vec![Some(14)])),
                // Outside the range: ignored.
                (2, LineValues::Rolled(vec![None, Some(30)])),
                (3, LineValues::Scaled(crate::input::Scaling::Catalyst)),
            ]),
        )]);
        let pair = || {
            vec![
                line("life", exact(3)),
                line("life", StatValue::Ranged { min: 10, max: 20 }),
            ]
        };
        let mut i = Ingest::new(&Map).with_item_lines(&lines);
        i.item_line(item, 0, &ranged_line(pair()));
        i.item_line(item, 1, &ranged_line(pair()));
        i.item_line(item, 2, &ranged_line(pair()));
        i.item_line(item, 3, &ranged_line(pair()));
        // A line the host does not list keeps its values.
        i.item_line(item, 4, &ranged_line(vec![line("life", exact(2))]));
        let (store, reasons) = i.finish();
        let values: Vec<f64> = store.effects().iter().map(|e| e.value).collect();
        // Line 0 (3, 14); lines 1 and 2 their exact stat only; line 4.
        assert_eq!(values, [3.0, 14.0, 3.0, 3.0, 2.0]);
        assert_eq!(reasons.indeterminate.len(), 2, "lines 1 and 2 unsettled");
        assert_eq!(
            reasons.partial.into_iter().collect::<Vec<_>>(),
            vec![UnsupportedReason::ScaledLine {
                source: Source::Item {
                    item,
                    line: 3,
                    adapted: None
                },
                scaling: "catalyst"
            }]
        );
    }

    #[test]
    fn a_typed_handler_should_add_its_effects() {
        struct Handles;
        impl StatMapping for Handles {
            fn map(&self, _: &StatId) -> StatMap {
                StatMap::Unmapped
            }
            fn typed(&self, _: &TypedResolution, source: &Source) -> TypedMap {
                TypedMap::Effects(vec![Effect {
                    quantity: Quantity::Mana,
                    op: Op::Base,
                    value: 1.0,
                    conditions: Conditions::none(),
                    source: source.clone(),
                }])
            }
        }
        let mut i = Ingest::new(&Handles);
        i.effect(Effect {
            quantity: Quantity::Life,
            op: Op::Base,
            value: 2.0,
            conditions: Conditions::none(),
            source: Source::Rule("r"),
        });
        i.typed(
            &Source::Enemy,
            &TypedResolution::GrantedSkill {
                source: "row".into(),
                gem: GemId::new("gem"),
                level: 1,
            },
        );
        let (store, reasons) = i.finish();
        assert_eq!(store.len(), 2);
        assert!(reasons.is_empty());
        assert!(format!("{:?}", Ingest::new(&Handles)).starts_with("Ingest"));
    }
}
