//! What-if queries: the computed fields of the open build across its
//! stages or across one swept input, for the UI's charts.
//!
//! Each point is the build with one stage chosen or one value set, written
//! into a copy as the command of the same change writes it
//! (`use-loadout`, `edit-gem`, `set-config-input`), and calculated as any
//! build is. Nothing is committed: the build, its version, and its history
//! stay as they are. It runs on the caller's thread with the latest
//! snapshot, never on the writer, as an item edit's preview does
//! ([`crate::preview`]).
//!
//! None of these changes touches an item's lines, so every copy keeps the
//! state's range ends ([`State::ends`]), as the commands do.

use loom_domain::{Build, ConfigValue, Number, Sourced};
use loom_serialization::dto::app::v1 as api;

use crate::data::GameData;
use crate::error::{AppError, Code};
use crate::loadout::{self, Loadout};
use crate::session::{self, State};
use crate::view;

/// The most points one query calculates (each is a full calculation).
pub const MAX_POINTS: u32 = 41;

/// The highest character level.
const MAX_CHARACTER_LEVEL: u32 = 100;

/// What one point changes in the copy.
#[derive(Debug, Clone)]
enum Change {
    /// Chooses a stage.
    Stage(Loadout),
    /// Sets a gem's level.
    GemLevel { group: u32, gem: u32, level: u32 },
    /// Sets the character's level.
    CharacterLevel(u32),
    /// Sets a config number.
    ConfigNumber(f64),
}

/// A point before its calculation.
#[derive(Debug, Clone)]
struct Planned {
    label: String,
    x: f64,
    change: Change,
}

fn refused(message: impl Into<String>) -> AppError {
    AppError::new(Code::WhatIf, message)
}

/// Refuses more than [`MAX_POINTS`] points.
fn check_count(count: u64) -> Result<(), AppError> {
    if count > u64::from(MAX_POINTS) {
        return Err(refused(format!(
            "{count} points; a what-if query has at most {MAX_POINTS}"
        )));
    }
    Ok(())
}

/// A swept number as text: up to six decimals, without trailing zeros.
fn number_label(x: f64) -> String {
    let text = format!("{x:.6}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// The request's points, or why it is refused. Calculates nothing.
fn plan(build: &Build, request: &api::WhatIfRequest) -> Result<Vec<Planned>, AppError> {
    match &request.vary {
        api::WhatIfVary::Stages => {
            let stages = loadout::loadouts(build);
            check_count(u64::try_from(stages.len()).unwrap_or(u64::MAX))?;
            Ok((0_u32..)
                .zip(stages)
                .map(|(i, l)| Planned {
                    label: l.title.clone(),
                    x: f64::from(i),
                    change: Change::Stage(l),
                })
                .collect())
        }
        api::WhatIfVary::GemLevel {
            group,
            gem,
            from,
            to,
        } => {
            let (group, gem, from, to) = (*group, *gem, *from, *to);
            let found = session::group_of(build, group).map_err(|e| refused(e.message))?;
            if found.gems.typed().nth(gem as usize).is_none() {
                return Err(refused(format!("no gem {gem}")));
            }
            if from == 0 || from > to {
                return Err(refused(format!(
                    "gem levels {from} to {to}: from at least 1 and not above to"
                )));
            }
            check_count(u64::from(to - from) + 1)?;
            Ok((from..=to)
                .map(|level| Planned {
                    label: level.to_string(),
                    x: f64::from(level),
                    change: Change::GemLevel { group, gem, level },
                })
                .collect())
        }
        api::WhatIfVary::CharacterLevel { from, to, step } => {
            let (from, to, step) = (*from, *to, *step);
            if step == 0 || from == 0 || from > to || to > MAX_CHARACTER_LEVEL {
                return Err(refused(format!(
                    "character levels {from} to {to} by {step}: from 1 to \
                     {MAX_CHARACTER_LEVEL}, from not above to, a step of at least 1"
                )));
            }
            check_count(u64::from((to - from) / step) + 1)?;
            Ok((0..=(to - from) / step)
                .map(|i| {
                    let level = from + i * step;
                    Planned {
                        label: level.to_string(),
                        x: f64::from(level),
                        change: Change::CharacterLevel(level),
                    }
                })
                .collect())
        }
        api::WhatIfVary::ConfigNumber {
            name,
            from,
            to,
            steps,
        } => {
            let (from, to, steps) = (*from, *to, *steps);
            let active = loadout::active_config_set(build);
            let numeric = build
                .config
                .sets
                .typed()
                .find(|s| Some(s.id) == active)
                .and_then(|s| s.inputs.typed().find(|i| i.name == *name))
                .is_some_and(|i| matches!(i.value, ConfigValue::Number(_)));
            if !numeric {
                return Err(refused(format!("no numeric config input {name}")));
            }
            if !from.is_finite() || !to.is_finite() || from >= to {
                return Err(refused(format!(
                    "config values {from} to {to}: finite, from below to"
                )));
            }
            if steps < 2 {
                return Err(refused(format!("{steps} steps; at least 2")));
            }
            check_count(u64::from(steps))?;
            let last = steps - 1;
            Ok((0..steps)
                .map(|i| {
                    // The ends exactly; the others evenly between them.
                    let x = if i == 0 {
                        from
                    } else if i == last {
                        to
                    } else {
                        from + (to - from) * f64::from(i) / f64::from(last)
                    };
                    Planned {
                        label: number_label(x),
                        x,
                        change: Change::ConfigNumber(x),
                    }
                })
                .collect())
        }
    }
}

/// The calculation's own cancel error when `cancel` says so (the engine
/// polls first, so nothing is calculated).
fn poll(state: &State, data: &GameData, cancel: &dyn Fn() -> bool) -> Result<(), AppError> {
    if cancel() {
        session::calculate(state, data, &|| true).map(|_| ())
    } else {
        Ok(())
    }
}

/// The open build across its stages or one swept input
/// ([`api::WhatIfRequest`]): each point calculated on a copy of the build,
/// polling `cancel` between the points and in each calculation.
///
/// # Errors
/// `APP_GAME_MISMATCH`, `APP_DATA_VERSION` when the data cannot calculate
/// the build; `APP_WHAT_IF` for more than [`MAX_POINTS`] points, a range
/// that is not one, or a group, gem, or numeric config input the build
/// does not have; the calculation's codes (`CALC_CANCELLED` when
/// cancelled), the first failing point's for the whole query.
pub fn what_if(
    state: &State,
    data: &GameData,
    request: &api::WhatIfRequest,
    cancel: &dyn Fn() -> bool,
) -> Result<api::WhatIfSeries, AppError> {
    session::check_data(&state.build, data)?;
    let planned = plan(&state.build, request)?;
    let name = match &request.vary {
        api::WhatIfVary::ConfigNumber { name, .. } => name.as_str(),
        _ => "",
    };
    // The main skill a stage's skill set switch carries over, as
    // `use-loadout` carries it.
    let previous = matches!(request.vary, api::WhatIfVary::Stages)
        .then(|| crate::main_skill::current_skills(&state.build, data))
        .unwrap_or_default();
    let mut points = Vec::with_capacity(planned.len());
    for p in planned {
        poll(state, data, cancel)?;
        let mut build = state.build.clone();
        let current = match p.change {
            // The active stage is the build as it is.
            Change::Stage(l) if loadout::is_active(&build, &l) => true,
            Change::Stage(l) => {
                let before = build.skills.active;
                loadout::apply(&mut build, &l);
                if build.skills.active != before {
                    session::main_after_switch(&mut build, &state.ends, data, &previous);
                }
                false
            }
            Change::GemLevel { group, gem, level } => {
                session::set_gem(&mut build, group, gem, Some(level), None, None)?;
                build == state.build
            }
            Change::CharacterLevel(level) => {
                match &mut build.character.level {
                    Some(l) if *l.value() == level => {}
                    Some(l) => l.set(level),
                    None => build.character.level = Some(Sourced::new(level)),
                }
                build == state.build
            }
            Change::ConfigNumber(x) => {
                session::write_config_input(
                    &mut build,
                    name,
                    ConfigValue::Number(Sourced::new(Number(x))),
                );
                build == state.build
            }
        };
        let point = State {
            build,
            ends: state.ends.clone(),
        };
        let result = session::calculate(&point, data, cancel)?;
        points.push(api::WhatIfPoint {
            label: p.label,
            x: p.x,
            current,
            fields: view::fields(&result)
                .into_iter()
                .map(|(field, value, state)| api::WhatIfField {
                    field,
                    value,
                    state,
                })
                .collect(),
        });
    }
    Ok(api::WhatIfSeries { points })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_labels_should_drop_trailing_zeros_and_float_noise() {
        assert_eq!(number_label(20.0), "20");
        assert_eq!(number_label(-60.0), "-60");
        assert_eq!(number_label(0.1 + 0.2), "0.3");
        assert_eq!(number_label(2.5), "2.5");
        assert_eq!(number_label(-0.0), "0");
        assert_eq!(number_label(100.0), "100");
    }
}
