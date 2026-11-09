//! Inverse of numeric handlers: the raw stat values that show as a number.
//!
//! The renderer shows one stat value `x` as
//! `format(h_n(...h_1(x)))`, where `h_1..h_n` are the stat's numeric
//! handlers in written order and `format` is the last format a handler set
//! (`%d` when none does; see [`super::render`]). [`invert_shown`] returns
//! every raw value that shows as a given text.
//!
//! **Why one interval:** each handler is `x * num / den`, optionally
//! rounded (`floor(y * 10^d + 0.5)`), times `then_mul`, plus `add`. That is
//! monotone with the sign of `num * then_mul / den`, and the value formats
//! (`%d` truncation, `%g`, `%.2f`) are non-decreasing. So the shown text is a
//! monotone function of `x`, and the values that show as one text form one
//! interval. It is found by binary search and both ends are checked by
//! rendering them, so a text the chain cannot produce (`1.5` where the
//! format is `%.2f`, `1.5` without a handler) has no interval.
//!
//! **Search domain:** stat values are 32-bit in game data (`Mods.Stat1Min`
//! to `Stat6Max` are `i32` in the vendored schema,
//! `tools/extractor/schema/schema.min.json`), so the search covers
//! `i32::MIN..=i32::MAX`.

use super::handler::{NumericHandler, ValueFormat};
use super::render::{StatValue, format_number};

/// Lowest raw value searched.
pub const RAW_MIN: i64 = i32::MIN as i64;
/// Highest raw value searched.
pub const RAW_MAX: i64 = i32::MAX as i64;

/// The text one raw value shows as through `handlers` (written order), as
/// the renderer writes a single value without a sign.
#[must_use]
pub fn shown_text(handlers: &[&NumericHandler], raw: i64) -> String {
    #[expect(
        clippy::cast_precision_loss,
        reason = "raw values are within i32, exact in f64"
    )]
    let mut value = raw as f64;
    let mut format = ValueFormat::Integer;
    for handler in handlers {
        value = handler.apply(value);
        if let Some(f) = handler.format {
            format = f;
        }
    }
    format_number(value, format)
}

/// Every raw value in `RAW_MIN..=RAW_MAX` that shows as `shown` through
/// `handlers` (written order), or `None` if none does. `shown` is one
/// number as written, with a leading `-` when negative (`"1.5"`, `"-10"`).
#[must_use]
pub fn invert_shown(handlers: &[&NumericHandler], shown: &str) -> Option<StatValue> {
    if handlers.is_empty() {
        // `%d` of the raw value: exact, and written as `%d` writes it (no
        // `+`, no leading zeros, no `-0`).
        let raw: i64 = shown.parse().ok()?;
        return ((RAW_MIN..=RAW_MAX).contains(&raw) && raw.to_string() == shown)
            .then_some(StatValue::exact(raw));
    }
    let target: f64 = shown.parse().ok().filter(|t: &f64| t.is_finite())?;
    // +1 when the shown value grows with the raw value, -1 when it shrinks.
    let direction = handlers.iter().fold(1.0_f64, |d, h| {
        let sign = h.num.signum() * h.then_mul.signum() * h.den.signum();
        if sign < 0 { -d } else { d }
    });
    let key = |raw: i64| -> f64 {
        shown_text(handlers, raw)
            .parse::<f64>()
            .map_or(f64::NAN, |v| v * direction)
    };
    let target = target * direction;
    // Keys are non-decreasing in `raw`.
    let min = first(RAW_MIN, RAW_MAX, |raw| key(raw) >= target)?;
    let max = match first(RAW_MIN, RAW_MAX, |raw| key(raw) > target) {
        Some(above) if above == RAW_MIN => return None,
        Some(above) => above - 1,
        None => RAW_MAX,
    };
    (min <= max && shown_text(handlers, min) == shown && shown_text(handlers, max) == shown)
        .then_some(StatValue { min, max })
}

/// The first value in `lo..=hi` where `pred` holds, for a `pred` that is
/// false and then true.
fn first(mut lo: i64, mut hi: i64, pred: impl Fn(i64) -> bool) -> Option<i64> {
    if !pred(hi) {
        return None;
    }
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if pred(mid) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    Some(lo)
}

#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "tests index fixed synthetic data; a bad index should panic"
)]
mod tests {
    use super::*;
    use crate::stat_desc::handler::NUMERIC_HANDLERS;
    use crate::stat_desc::render::{NoLookup, VariantFilter, render_description};
    use crate::stat_desc::{Condition, Description, Hand, Handler, Template, Variant};

    fn numeric(name: &str) -> &'static NumericHandler {
        NUMERIC_HANDLERS.iter().find(|h| h.name == name).unwrap()
    }

    fn one_stat(handlers: Vec<Handler>) -> Description {
        Description {
            name: None,
            line: 1,
            stats: vec!["s".to_owned()],
            off_hand_stats: None,
            variants: vec![Variant {
                conditions: vec![Condition::Range {
                    min: None,
                    max: None,
                }],
                gem_quality: false,
                table_only: false,
                unknown_flags: Vec::new(),
                handlers,
                line: 1,
                text: "{0}".to_owned(),
                template: Template::parse("{0}", 1),
            }],
        }
    }

    /// Raw values for the per-handler checks: small values, both signs,
    /// and values around rounding steps of every divisor in the table.
    fn raws() -> Vec<i64> {
        let mut out: Vec<i64> = (-130..=130).collect();
        for base in [599, 600, 601, 999, 1000, 1001, 5999, 6000, 9999, 10_000] {
            out.extend([base, -base]);
        }
        out.extend([RAW_MIN, RAW_MIN + 1, RAW_MAX - 1, RAW_MAX]);
        out
    }

    #[test]
    fn shown_text_should_equal_the_renderer_for_every_numeric_handler() {
        for h in NUMERIC_HANDLERS {
            let d = one_stat(vec![Handler::Numeric {
                handler: h,
                stat: 0,
            }]);
            for raw in [-1234, -7, 0, 3, 15, 1234] {
                let rendered = render_description(
                    &d,
                    Hand::Main,
                    &[StatValue::exact(raw)],
                    &NoLookup,
                    VariantFilter::default(),
                );
                if raw == 0 {
                    continue; // the renderer checks conditions, not zero
                }
                assert_eq!(
                    rendered.map(|(_, l)| l),
                    Ok(vec![shown_text(&[h], raw)]),
                    "{} {raw}",
                    h.name
                );
            }
        }
    }

    #[test]
    fn shown_text_should_equal_the_renderer_for_handler_chains() {
        let chains = [
            ["negate", "divide_by_one_hundred"],
            ["divide_by_ten_1dp", "double"],
            ["per_minute_to_per_second", "deciseconds_to_seconds"],
        ];
        for names in chains {
            let chain: Vec<_> = names.iter().map(|n| numeric(n)).collect();
            let d = one_stat(
                chain
                    .iter()
                    .map(|h| Handler::Numeric {
                        handler: h,
                        stat: 0,
                    })
                    .collect(),
            );
            for raw in [-1234, -7, 3, 15, 1234] {
                let rendered = render_description(
                    &d,
                    Hand::Main,
                    &[StatValue::exact(raw)],
                    &NoLookup,
                    VariantFilter::default(),
                );
                assert_eq!(
                    rendered.map(|(_, l)| l),
                    Ok(vec![shown_text(&chain, raw)]),
                    "{names:?} {raw}"
                );
            }
        }
    }

    /// Per-handler inverse: the interval holds the raw value, both ends
    /// show the same text, and it is maximal (the neighbours do not).
    #[test]
    fn invert_should_give_the_maximal_interval_for_every_numeric_handler() {
        for h in NUMERIC_HANDLERS {
            for raw in raws() {
                let text = shown_text(&[h], raw);
                let found = invert_shown(&[h], &text);
                let Some(found) = found else {
                    panic!("{}: {raw} shows as {text}, not inverted", h.name);
                };
                assert!(found.min <= raw && raw <= found.max, "{} {raw}", h.name);
                if found.min > RAW_MIN {
                    assert_ne!(shown_text(&[h], found.min - 1), text, "{} {raw}", h.name);
                }
                if found.max < RAW_MAX {
                    assert_ne!(shown_text(&[h], found.max + 1), text, "{} {raw}", h.name);
                }
            }
        }
    }

    #[test]
    fn invert_should_be_exact_without_a_handler() {
        assert_eq!(invert_shown(&[], "25"), Some(StatValue::exact(25)));
        assert_eq!(invert_shown(&[], "-3"), Some(StatValue::exact(-3)));
        assert_eq!(invert_shown(&[], "1.5"), None);
        for bad in ["007", "-0", "+5", "5.0"] {
            assert_eq!(invert_shown(&[], bad), None, "{bad}");
        }
    }

    #[test]
    fn invert_should_widen_for_rounding_and_truncating_handlers() {
        // round(x / 10, 1): 1.5 shows for 15 only; %d of x / 100 shows 5
        // for 500..=599 and -500..=-599 truncates to -5.
        assert_eq!(
            invert_shown(&[numeric("divide_by_ten_1dp")], "1.5"),
            Some(StatValue::exact(15))
        );
        assert_eq!(
            invert_shown(&[numeric("divide_by_one_hundred_0dp")], "5"),
            Some(StatValue { min: 500, max: 599 })
        );
        assert_eq!(
            invert_shown(&[numeric("divide_by_one_hundred_0dp")], "-5"),
            Some(StatValue {
                min: -599,
                max: -500
            })
        );
        // per_minute_to_per_second: round(x / 60, 1) = 1.7 for 99..=104.
        assert_eq!(
            invert_shown(&[numeric("per_minute_to_per_second")], "1.7"),
            Some(StatValue { min: 99, max: 104 })
        );
    }

    #[test]
    fn invert_should_follow_decreasing_handlers() {
        assert_eq!(
            invert_shown(&[numeric("negate")], "10"),
            Some(StatValue::exact(-10))
        );
        assert_eq!(
            invert_shown(&[numeric("invert_chance")], "70"),
            Some(StatValue::exact(30))
        );
    }

    #[test]
    fn invert_should_chain_handlers_in_written_order() {
        let chain = [numeric("negate"), numeric("divide_by_one_hundred")];
        assert_eq!(shown_text(&chain, -150), "1.5");
        assert_eq!(invert_shown(&chain, "1.5"), Some(StatValue::exact(-150)));
    }

    #[test]
    fn invert_should_reject_text_the_format_cannot_write() {
        let h = [numeric("deciseconds_to_seconds")];
        assert_eq!(invert_shown(&h, "1.50"), Some(StatValue::exact(15)));
        assert_eq!(invert_shown(&h, "1.5"), None);
        assert_eq!(invert_shown(&[], "+5"), None);
        for bad in ["", "x", "inf", "NaN", "1e999", "--1"] {
            assert_eq!(invert_shown(&[], bad), None, "{bad}");
        }
    }

    #[test]
    fn invert_should_reject_values_outside_the_raw_domain() {
        assert_eq!(invert_shown(&[], "99999999999"), None);
        assert_eq!(
            invert_shown(&[], &RAW_MAX.to_string()),
            Some(StatValue::exact(RAW_MAX))
        );
    }

    #[test]
    fn invert_should_not_panic_on_any_chain_and_text() {
        // Deterministic generator (xorshift64); no external crate.
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let texts = ["0", "-0", "1", "1.5", "-2.25", "1e+06", "", "+3", "0.000"];
        for _ in 0..2_000 {
            let len = usize::try_from(next() % 4).unwrap();
            let chain: Vec<_> = (0..len)
                .map(|_| {
                    let i = usize::try_from(next()).unwrap() % NUMERIC_HANDLERS.len();
                    &NUMERIC_HANDLERS[i]
                })
                .collect();
            let text = texts[usize::try_from(next()).unwrap() % texts.len()];
            if let Some(found) = invert_shown(&chain, text) {
                assert!(found.min <= found.max);
                assert_eq!(shown_text(&chain, found.min), text);
            }
        }
    }
}
