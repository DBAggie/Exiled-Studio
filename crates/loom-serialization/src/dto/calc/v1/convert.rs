//! Engine result to v1 DTOs. One way only: the document is read as DTOs
//! (module docs).

use loom_calc as c;
use loom_domain as d;

use super::types::{
    CalcResult, FieldResult, IndeterminateReason, Node, NodeKind, Reasons, Source, StateKind,
    UnsupportedReason,
};
use crate::dto::v1::{Adapted, Game, NonFinite, Number};

/// A node's index. `NodeIndex` holds a `u32`, so the cast is lossless.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a NodeIndex is a u32 inside"
)]
const fn node_to(i: c::NodeIndex) -> u32 {
    i.get() as u32
}

fn index_to(i: usize) -> u64 {
    // `usize` is at most 64 bits on every target Rust supports.
    i as u64
}

fn number_to(n: f64) -> Number {
    if n.is_finite() {
        Number::Finite(n)
    } else {
        Number::NonFinite(NonFinite {
            f64_bits: format!("{:016x}", n.to_bits()),
        })
    }
}

const fn game_to(g: d::Game) -> Game {
    match g {
        d::Game::Poe1 => Game::Poe1,
        d::Game::Poe2 => Game::Poe2,
    }
}

pub(super) const fn state_to(s: c::StateKind) -> StateKind {
    match s {
        c::StateKind::Complete => StateKind::Complete,
        c::StateKind::Partial => StateKind::Partial,
        c::StateKind::Unsupported => StateKind::Unsupported,
        c::StateKind::Indeterminate => StateKind::Indeterminate,
    }
}

fn source_to(s: &c::Source) -> Source {
    match s {
        c::Source::Character => Source::Character,
        c::Source::Item {
            item,
            line,
            adapted,
        } => Source::Item {
            item: item.0,
            line: index_to(*line),
            adapted: adapted.map(|a| match a {
                d::Adapted::CaseFolded => Adapted::CaseFolded,
                d::Adapted::SplitSection => Adapted::SplitSection,
            }),
        },
        c::Source::ItemBase { item } => Source::ItemBase { item: item.0 },
        c::Source::Passive { node } => Source::Passive { node: node.0 },
        c::Source::Gem { group, gem } => Source::Gem {
            group: index_to(*group),
            gem: index_to(*gem),
        },
        c::Source::Config { input } => Source::Config {
            input: input.clone(),
        },
        c::Source::Enemy => Source::Enemy,
        c::Source::Rule(name) => Source::Rule((*name).to_owned()),
        c::Source::CustomModifier { block, line } => Source::CustomModifier {
            block: index_to(*block),
            line: index_to(*line),
        },
        c::Source::ConfigInputLine { input, line } => Source::ConfigInputLine {
            input: input.clone(),
            line: index_to(*line),
        },
    }
}

pub(super) fn unsupported_to(r: &c::UnsupportedReason) -> UnsupportedReason {
    match r {
        c::UnsupportedReason::UnmappedStat { stat, source } => UnsupportedReason::UnmappedStat {
            stat: stat.as_str().to_owned(),
            source: source_to(source),
        },
        c::UnsupportedReason::UnresolvedLine { source, reason } => {
            UnsupportedReason::UnresolvedLine {
                source: source_to(source),
                reason: (*reason).to_owned(),
            }
        }
        c::UnsupportedReason::ExcludedLine { source, exclusion } => {
            UnsupportedReason::ExcludedLine {
                source: source_to(source),
                exclusion: (*exclusion).to_owned(),
            }
        }
        c::UnsupportedReason::PendingLine { source } => UnsupportedReason::PendingLine {
            source: source_to(source),
        },
        c::UnsupportedReason::UnhandledKind { kind, source } => UnsupportedReason::UnhandledKind {
            kind: (*kind).to_owned(),
            source: source_to(source),
        },
        c::UnsupportedReason::UnmodelledCondition {
            condition,
            quantity,
            source,
        } => UnsupportedReason::UnmodelledCondition {
            condition: condition.as_str().to_owned(),
            quantity: quantity.to_string(),
            source: source_to(source),
        },
        c::UnsupportedReason::DependsOnNotComputed { field } => {
            UnsupportedReason::DependsOnNotComputed {
                field: field.to_string(),
            }
        }
        c::UnsupportedReason::NotComputed => UnsupportedReason::NotComputed,
        c::UnsupportedReason::UnmodelledGem { source } => UnsupportedReason::UnmodelledGem {
            source: source_to(source),
        },
        c::UnsupportedReason::UnmodelledConfig { input } => UnsupportedReason::UnmodelledConfig {
            input: input.clone(),
        },
        c::UnsupportedReason::UnmodelledChoice { choice, value } => {
            UnsupportedReason::UnmodelledChoice {
                choice: (*choice).to_owned(),
                value: value.clone(),
            }
        }
        c::UnsupportedReason::UnmodelledItem { item, slot, what } => {
            UnsupportedReason::UnmodelledItem {
                item: item.0,
                slot: slot.clone(),
                what: (*what).to_owned(),
            }
        }
        c::UnsupportedReason::UnmodelledPassive { source, what } => {
            UnsupportedReason::UnmodelledPassive {
                source: source_to(source),
                what: (*what).to_owned(),
            }
        }
        c::UnsupportedReason::UnreadLine { source } => UnsupportedReason::UnreadLine {
            source: source_to(source),
        },
        c::UnsupportedReason::ScaledLine { source, scaling } => UnsupportedReason::ScaledLine {
            source: source_to(source),
            scaling: (*scaling).to_owned(),
        },
        c::UnsupportedReason::UncheckedTreeVersion => UnsupportedReason::UncheckedTreeVersion,
        c::UnsupportedReason::UnmodelledSkill { source, what } => {
            UnsupportedReason::UnmodelledSkill {
                source: source_to(source),
                what: (*what).to_owned(),
            }
        }
    }
}

pub(super) fn indeterminate_to(r: &c::IndeterminateReason) -> IndeterminateReason {
    match r {
        c::IndeterminateReason::ConflictingOverride { quantity } => {
            IndeterminateReason::ConflictingOverride {
                quantity: quantity.to_string(),
            }
        }
        c::IndeterminateReason::UnsettledValue { stat, source } => {
            IndeterminateReason::UnsettledValue {
                stat: stat.as_str().to_owned(),
                source: source_to(source),
            }
        }
        c::IndeterminateReason::MissingData { table, key } => IndeterminateReason::MissingData {
            table: (*table).to_owned(),
            key: key.clone(),
        },
        c::IndeterminateReason::TreeVersion { build, data } => IndeterminateReason::TreeVersion {
            build: build.clone(),
            data: data.clone(),
        },
    }
}

fn reasons_to<'a>(
    unsupported: impl IntoIterator<Item = &'a c::UnsupportedReason>,
    indeterminate: impl IntoIterator<Item = &'a c::IndeterminateReason>,
) -> Reasons {
    Reasons {
        unsupported: unsupported.into_iter().map(unsupported_to).collect(),
        indeterminate: indeterminate.into_iter().map(indeterminate_to).collect(),
    }
}

fn field_to(field: c::Field, r: &c::FieldResult) -> FieldResult {
    let (reason, lists) = match &r.state {
        c::FieldState::Complete => (None, reasons_to([], [])),
        c::FieldState::Partial(p) => (None, reasons_to(p, [])),
        c::FieldState::Unsupported(u) => (Some(unsupported_to(u)), reasons_to([], [])),
        c::FieldState::Indeterminate {
            reasons: unsettled,
            partial,
        } => (None, reasons_to(partial, unsettled)),
    };
    FieldResult {
        field: field.to_string(),
        value: r.value.map(number_to),
        node: r.node.map(node_to),
        state: state_to(r.state.kind()),
        reason,
        unsupported: lists.unsupported,
        indeterminate: lists.indeterminate,
    }
}

fn term_to(t: c::Term) -> String {
    match t {
        c::Term::Base => "base".to_owned(),
        c::Term::Increased => "increased".to_owned(),
        c::Term::More => "more".to_owned(),
        c::Term::Override => "override".to_owned(),
        c::Term::Result => "result".to_owned(),
        c::Term::Step(name) => format!("step.{name}"),
    }
}

fn node_kind_to(n: &c::Node) -> (NodeKind, Option<c::Term>) {
    match n {
        c::Node::Contribution {
            source,
            quantity,
            op,
            ..
        } => (
            NodeKind::Contribution {
                source: source_to(source),
                quantity: quantity.to_string(),
                op: op.name().to_owned(),
            },
            None,
        ),
        c::Node::Constant { label, .. } => (
            NodeKind::Constant {
                label: (*label).to_owned(),
            },
            None,
        ),
        c::Node::Sum { term, .. } => (NodeKind::Sum, Some(*term)),
        c::Node::Product { term, .. } => (NodeKind::Product, Some(*term)),
        c::Node::PercentFactor { term, .. } => (NodeKind::PercentFactor, Some(*term)),
        c::Node::Any { term, .. } => (NodeKind::Any, Some(*term)),
        c::Node::Override { .. } => (NodeKind::Override, None),
        c::Node::Floor { term, .. } => (NodeKind::Floor, Some(*term)),
        c::Node::RoundHalfUp { term, .. } => (NodeKind::RoundHalfUp, Some(*term)),
        c::Node::Trunc { term, .. } => (NodeKind::Trunc, Some(*term)),
        c::Node::Min { term, .. } => (NodeKind::Min, Some(*term)),
        c::Node::Max { term, .. } => (NodeKind::Max, Some(*term)),
        c::Node::Difference { term, .. } => (NodeKind::Difference, Some(*term)),
        c::Node::Quotient { term, .. } => (NodeKind::Quotient, Some(*term)),
        c::Node::Power { term, .. } => (NodeKind::Power, Some(*term)),
    }
}

fn node_to_dto(n: &c::Node, value: f64) -> Node {
    let (kind, term) = node_kind_to(n);
    Node {
        kind,
        term: term.map(term_to),
        children: n.children().into_iter().map(node_to).collect(),
        value: number_to(value),
    }
}

pub(super) fn result_to(r: &c::CalcResult) -> CalcResult {
    CalcResult {
        game: game_to(r.game),
        state: state_to(r.state),
        reasons: reasons_to(&r.reasons.partial, &r.reasons.indeterminate),
        fields: r.fields.iter().map(|(f, v)| field_to(*f, v)).collect(),
        graph: r.graph.iter().map(|(_, n, v)| node_to_dto(n, v)).collect(),
    }
}
