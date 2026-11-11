//! The audited math module (Phase 3 plan, decision 7).
//!
//! Values that reach an output go through IEEE-754 operations whose result
//! is exactly specified (correctly rounded `+ - * /` and `sqrt`, `floor`,
//! `ceil`, `trunc`, comparisons), so Windows, Linux, and wasm32 give the
//! same bits. Any other function (powers, logarithms, trigonometry) is
//! added here only with a deterministic implementation and a test of its
//! bits, never called from `std` elsewhere in the engine.
//!
//! The rounding functions here are the rounding points the engine may use;
//! each use in a phase names where the game rounds.

/// Powers of ten that are exact in `f64`, for [`round_half_up_to`].
const POW10: [f64; 7] = [1.0, 10.0, 100.0, 1_000.0, 10_000.0, 100_000.0, 1_000_000.0];

/// The largest number of decimals [`round_half_up_to`] takes.
pub const MAX_DECIMALS: u8 = 6;

/// The greatest integer not above `x` (IEEE-754 `roundToIntegralTowardNegative`).
#[must_use]
pub fn floor(x: f64) -> f64 {
    x.floor()
}

/// The least integer not below `x` (IEEE-754 `roundToIntegralTowardPositive`).
#[must_use]
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

/// `x` without its fraction (IEEE-754 `roundToIntegralTowardZero`).
#[must_use]
pub fn trunc(x: f64) -> f64 {
    x.trunc()
}

/// Rounds half up: `floor(x + 0.5)`. This is not `f64::round` (half away
/// from zero): `-2.5` gives `-2`. Both the addition and the floor are
/// exactly specified.
#[must_use]
pub fn round_half_up(x: f64) -> f64 {
    floor(x + 0.5)
}

/// Rounds half up at `decimals` decimals: `floor(x * 10^d + 0.5) / 10^d`.
/// Each step is one correctly rounded operation, so the bits are the same
/// on every platform. `None` above [`MAX_DECIMALS`].
#[must_use]
pub fn round_half_up_to(x: f64, decimals: u8) -> Option<f64> {
    let p = *POW10.get(usize::from(decimals))?;
    Some(round_half_up(x * p) / p)
}

/// The smaller of two numbers; `a` when they are equal (so `-0.0` and
/// `0.0` keep their order). NaN is not handled: the engine never produces
/// it on an output path.
#[must_use]
pub fn min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// The larger of two numbers; `a` when they are equal.
#[must_use]
pub fn max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// `1 + x / 100`: the factor of an increase or a "more" of `x` percent.
#[must_use]
pub fn percent_factor(x: f64) -> f64 {
    1.0 + x / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bits written once; CI runs this on Windows and Linux, so a platform
    /// that rounds differently fails here.
    #[test]
    fn math_should_give_the_written_bits() {
        let cases: [(f64, u64); 12] = [
            (floor(2.5), 2.0_f64.to_bits()),
            (floor(-2.5), (-3.0_f64).to_bits()),
            (ceil(2.1), 3.0_f64.to_bits()),
            (trunc(-2.7), (-2.0_f64).to_bits()),
            (round_half_up(2.5), 3.0_f64.to_bits()),
            (round_half_up(-2.5), (-2.0_f64).to_bits()),
            // The largest double below 0.5: `x + 0.5` rounds to 1, so half
            // up gives 1, as PoB's `m_floor(x + 0.5)` does.
            (round_half_up(0.499_999_999_999_999_94), 1.0_f64.to_bits()),
            (percent_factor(184.0), 0x4006_B851_EB85_1EB8),
            (percent_factor(-30.0), 0x3FE6_6666_6666_6666),
            (
                round_half_up_to(2.845, 2).unwrap_or(f64::NAN),
                2.85_f64.to_bits(),
            ),
            (
                round_half_up_to(1.005, 2).unwrap_or(f64::NAN),
                0x3FF0_0000_0000_0000,
            ),
            (0.1 + 0.2, 0x3FD3_3333_3333_3334),
        ];
        for (i, (value, bits)) in cases.into_iter().enumerate() {
            assert_eq!(value.to_bits(), bits, "case {i}: {value}");
        }
    }

    #[test]
    fn round_half_up_to_should_refuse_too_many_decimals() {
        assert_eq!(round_half_up_to(1.0, MAX_DECIMALS + 1), None);
        assert_eq!(round_half_up_to(1.234_567, MAX_DECIMALS), Some(1.234_567));
    }

    #[test]
    fn min_and_max_should_keep_the_first_on_equal_values() {
        assert_eq!(min(-0.0, 0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(max(0.0, -0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(min(1.0, 2.0), 1.0);
        assert_eq!(max(1.0, 2.0), 2.0);
    }
}
