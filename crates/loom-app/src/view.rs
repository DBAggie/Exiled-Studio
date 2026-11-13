//! Projections of the domain onto the API DTOs (ADR-004: queries return
//! typed projections; the UI never reads the domain).

use loom_calc::CalcResult;
use loom_domain::{
    Build, DataVersion, Item, ItemId, ItemLine, LineKind, LineResolution, QuarantineReason,
    Resolved, StatValue, UnresolvedReason,
};
use loom_serialization::dto::app::v1 as api;

/// The data version's view.
#[must_use]
pub fn data_version(v: &DataVersion) -> api::DataVersionView {
    api::DataVersionView {
        game: v.game.name().to_owned(),
        schema_version: v.schema_version,
        steam_build: v.steam_build,
    }
}

/// The slot each item is in, in the active item set (else the first).
fn slot_of(build: &Build, id: ItemId) -> Option<String> {
    let set = build
        .items
        .sets()
        .find(|s| Some(s.id) == build.items.active_set)
        .or_else(|| build.items.sets().next())?;
    set.slots
        .typed()
        .find(|s| s.item == Some(id))
        .map(|s| s.slot.as_str().to_owned())
}

/// The main skill's gem names (the build's main socket group in the
/// active skill set), if it has one.
fn main_skill(build: &Build) -> Option<String> {
    let index = usize::try_from(*build.character.main_socket_group.as_ref()?.value()).ok()?;
    let set = build
        .skills
        .sets
        .typed()
        .find(|s| Some(s.id) == build.skills.active)
        .or_else(|| build.skills.sets.typed().next())?;
    let group = set.groups.typed().nth(index.checked_sub(1)?)?;
    let names: Vec<&str> = group
        .gems
        .typed()
        .filter_map(|g| g.name.as_deref())
        .collect();
    (!names.is_empty()).then(|| names.join(", "))
}

fn warning_code(line: &ItemLine) -> Option<&'static str> {
    let reason = match &line.kind {
        LineKind::Unrecognized => QuarantineReason::UnrecognizedLine,
        LineKind::Spec { known: false, .. } => QuarantineReason::UnknownSpec,
        LineKind::Modifier(m) => match &m.resolution {
            LineResolution::Pending => QuarantineReason::Pending,
            LineResolution::Unresolved(r) => QuarantineReason::Unresolved(r.clone()),
            _ => return None,
        },
        _ => return None,
    };
    Some(reason.code())
}

fn value_text(v: &StatValue) -> String {
    match v {
        StatValue::Exact { min, max } | StatValue::Ranged { min, max } if min == max => {
            min.to_string()
        }
        StatValue::Exact { min, max } | StatValue::Ranged { min, max } => format!("{min} to {max}"),
        StatValue::Hidden { assumed: Some(a) } => format!("hidden ({a})"),
        StatValue::Hidden { assumed: None } => "hidden".to_owned(),
        StatValue::Lookup(values) => values
            .iter()
            .map(i64::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// The numbers in a line's text, outside `{...}` prefixes.
fn numbers(text: &str) -> Vec<api::NumberSpan> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut depth = 0_u32;
    while let Some(&c) = chars.get(i) {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 && c.is_ascii_digit() => {
                let start = i;
                while chars
                    .get(i)
                    .is_some_and(|c| c.is_ascii_digit() || *c == '.')
                {
                    i += 1;
                }
                // A trailing dot ends a sentence, not a number.
                let mut end = i;
                if chars.get(end - 1) == Some(&'.') {
                    end -= 1;
                }
                let (Ok(s), Ok(e)) = (u32::try_from(start), u32::try_from(end)) else {
                    break;
                };
                out.push(api::NumberSpan {
                    start: s,
                    end: e,
                    text: chars.get(start..end).unwrap_or_default().iter().collect(),
                });
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// One line's view.
#[must_use]
pub fn line_view(index: usize, line: &ItemLine) -> api::LineView {
    let mut view = api::LineView {
        index: u32::try_from(index).unwrap_or(u32::MAX),
        text: line.source.clone(),
        kind: line.kind.name().to_owned(),
        section: None,
        status: api::LineStatus::Other,
        code: warning_code(line).map(str::to_owned),
        stats: Vec::new(),
        typed: None,
        adapted: None,
        numbers: numbers(&line.source),
    };
    if view.code.is_some() {
        view.status = api::LineStatus::Warning;
    }
    if let LineKind::Modifier(m) = &line.kind {
        view.section = Some(m.section.name().to_owned());
        match &m.resolution {
            LineResolution::Resolved { result, adapted } => {
                view.status = if adapted.is_some() {
                    api::LineStatus::Adapted
                } else {
                    api::LineStatus::Matched
                };
                view.adapted = adapted.map(|a| a.name().to_owned());
                match result {
                    Resolved::Stats(stats) => {
                        view.stats = stats
                            .iter()
                            .map(|s| api::StatView {
                                id: s.stat.as_str().to_owned(),
                                value: value_text(&s.value),
                            })
                            .collect();
                    }
                    Resolved::Typed(t) => view.typed = Some(t.kind().to_owned()),
                }
            }
            LineResolution::Continues { .. } => view.status = api::LineStatus::Continues,
            LineResolution::Excluded(_) => view.status = api::LineStatus::Excluded,
            LineResolution::Pending | LineResolution::Unresolved(_) => {}
        }
        if let LineResolution::Unresolved(UnresolvedReason::Ambiguous(options)) = &m.resolution {
            view.stats = options
                .iter()
                .flatten()
                .map(|s| api::StatView {
                    id: s.as_str().to_owned(),
                    value: "?".to_owned(),
                })
                .collect();
        }
    }
    view
}

/// One item's view.
#[must_use]
pub fn item_view(build: &Build, item: &Item) -> api::ItemView {
    api::ItemView {
        id: item.id.0,
        name: item.body.name.clone(),
        base: item.body.base.clone(),
        rarity: item.body.rarity.map(|r| r.name().to_owned()),
        slot: slot_of(build, item.id),
        lines: item
            .body
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| line_view(i, l))
            .collect(),
    }
}

/// The build's view.
#[must_use]
pub fn build_view(build: &Build, name: String) -> api::BuildView {
    let mut counts = api::LineCounts {
        modifiers: 0,
        matched: 0,
        adapted: 0,
        warnings: 0,
        excluded: 0,
    };
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    for item in build.items.items() {
        let mut item_warnings = 0;
        for (i, line) in item.body.lines.iter().enumerate() {
            let view = line_view(i, line);
            if let LineKind::Modifier(_) = line.kind {
                counts.modifiers += 1;
            }
            match view.status {
                api::LineStatus::Matched => counts.matched += 1,
                api::LineStatus::Adapted => counts.adapted += 1,
                api::LineStatus::Excluded => counts.excluded += 1,
                api::LineStatus::Warning => {
                    counts.warnings += 1;
                    item_warnings += 1;
                }
                api::LineStatus::Continues | api::LineStatus::Other => {}
            }
            if let Some(code) = view.code {
                warnings.push(api::WarningView {
                    item: item.id.0,
                    item_name: item.body.name.clone(),
                    line: view.index,
                    text: view.text,
                    code,
                });
            }
        }
        items.push(api::ItemSummary {
            id: item.id.0,
            name: item.body.name.clone(),
            base: item.body.base.clone(),
            rarity: item.body.rarity.map(|r| r.name().to_owned()),
            slot: slot_of(build, item.id),
            warnings: item_warnings,
        });
    }
    api::BuildView {
        name,
        game: build.game().name().to_owned(),
        class: build
            .character
            .class
            .as_ref()
            .map(|c| c.as_str().to_owned()),
        ascendancy: build
            .character
            .ascendancy
            .as_ref()
            .map(|a| a.as_str().to_owned()),
        level: build.character.level.as_ref().map(|l| *l.value()),
        main_skill: main_skill(build),
        data_version: build.data.as_ref().map(data_version),
        completeness: build.completeness().name().to_owned(),
        lines: counts,
        items,
        warnings,
    }
}

/// Each field of a calc result as `(name, finite value, state)`.
#[must_use]
pub fn fields(result: &CalcResult) -> Vec<(String, Option<f64>, String)> {
    result
        .fields
        .iter()
        .map(|(field, f)| {
            let value = f.value.filter(|v| v.is_finite());
            (field.to_string(), value, f.state.kind().name().to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_should_skip_prefixes_and_sentence_dots() {
        let n = numbers("{range:0.5}+(10-20.5) to maximum Life.");
        let texts: Vec<&str> = n.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["10", "20.5"]);
        assert_eq!(n[0].start, 13);
    }
}
