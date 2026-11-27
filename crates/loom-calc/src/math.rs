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

/// The square root of `x` (IEEE-754 `squareRoot`, correctly rounded;
/// PoB's `m_sqrt`, P6-10b part 13).
#[must_use]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
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

/// `base` to the power `n` (`n >= 0`) by repeated multiplication, left to
/// right: only IEEE-exact operations, so every platform (wasm32 included)
/// gives the same bits. It may differ from C's `pow` (which PoB's Lua `^`
/// calls) in the last bit; a caller that floors or rounds the result
/// checks its values against PoB's (the monster armour table, P3-11).
#[must_use]
pub fn pow_int(base: f64, n: u32) -> f64 {
    let mut out = 1.0;
    for _ in 0..n {
        out *= base;
    }
    out
}

/// `base` to the power `exponent` (C's `pow`, which PoB's Lua `^` calls),
/// correctly rounded: the exact power rounded to the nearest double.
///
/// musl's `pow` (the `libm` crate) is within 1 ulp but not always
/// correctly rounded: `0.925 ^ 2.215714285714286` comes out 1 ulp below
/// the value PoB's LuaJIT gives on Windows and Linux (the open items'
/// game-truth lines, group 2). For a positive finite base and a result in
/// the normal range, the power is computed as `exp(exponent * log(base))`
/// in double-double arithmetic (about 100 bits, [`dd`]) and rounded once,
/// using only IEEE-exact operations and `libm`'s `log` as the first guess,
/// so every platform (wasm32 included) gives the same bits. It can differ
/// from the correctly rounded value only when the exact power lies within
/// about `2^-95` of a halfway point between two doubles. Every other input
/// (a zero, negative, or subnormal base, infinities, NaN, overflow,
/// subnormal results) goes to `libm::pow`.
#[must_use]
pub fn pow(base: f64, exponent: f64) -> f64 {
    if !(base >= f64::MIN_POSITIVE && base.is_finite() && exponent.is_finite()) {
        return libm::pow(base, exponent);
    }
    if exponent == 0.0 || base == 1.0 {
        return 1.0;
    }
    let t = dd::log(base).mul_f64(exponent);
    if !(t.hi > -708.0 && t.hi < 709.0) {
        return libm::pow(base, exponent);
    }
    dd::exp_rounded(t)
}

/// Double-double arithmetic for [`pow`]: a value is the unevaluated sum
/// `hi + lo` with `|lo| <= ulp(hi) / 2`. The algorithms are the QD
/// library's (Hida, Li, Bailey), with Dekker's split for the exact product
/// so no fused multiply-add is needed.
mod dd {
    /// A double-double value.
    #[derive(Clone, Copy, Debug)]
    pub(super) struct Dd {
        pub(super) hi: f64,
        pub(super) lo: f64,
    }

    /// `ln 2` in double-double.
    const LN2: Dd = Dd {
        hi: f64::from_bits(0x3FE6_2E42_FEFA_39EF),
        lo: f64::from_bits(0x3C7A_BC9E_3B39_803F),
    };

    /// The argument of `exp` is divided by `2^SQUARINGS` before the series
    /// and the result squared back.
    const SQUARINGS: u32 = 10;

    /// Terms of the series of `exp(r) - 1` for `|r| <= ln 2 / 2^11`.
    const TERMS: u32 = 10;

    fn two_sum(a: f64, b: f64) -> Dd {
        let s = a + b;
        let bb = s - a;
        Dd {
            hi: s,
            lo: (a - (s - bb)) + (b - bb),
        }
    }

    fn quick_two_sum(a: f64, b: f64) -> Dd {
        let s = a + b;
        Dd {
            hi: s,
            lo: b - (s - a),
        }
    }

    /// Dekker's split of `a` into two 26-bit halves.
    fn split(a: f64) -> (f64, f64) {
        let t = 134_217_729.0 * a;
        let hi = t - (t - a);
        (hi, a - hi)
    }

    fn two_prod(a: f64, b: f64) -> Dd {
        let p = a * b;
        let (ah, al) = split(a);
        let (bh, bl) = split(b);
        Dd {
            hi: p,
            lo: ((ah * bh - p) + ah * bl + al * bh) + al * bl,
        }
    }

    impl Dd {
        fn from_f64(x: f64) -> Self {
            Self { hi: x, lo: 0.0 }
        }

        fn neg(self) -> Self {
            Self {
                hi: -self.hi,
                lo: -self.lo,
            }
        }

        fn add(self, b: Self) -> Self {
            let s = two_sum(self.hi, b.hi);
            let t = two_sum(self.lo, b.lo);
            let r = quick_two_sum(s.hi, s.lo + t.hi);
            quick_two_sum(r.hi, r.lo + t.lo)
        }

        fn sub(self, b: Self) -> Self {
            self.add(b.neg())
        }

        fn mul(self, b: Self) -> Self {
            let p = two_prod(self.hi, b.hi);
            quick_two_sum(p.hi, p.lo + (self.hi * b.lo + self.lo * b.hi))
        }

        pub(super) fn mul_f64(self, b: f64) -> Self {
            let p = two_prod(self.hi, b);
            quick_two_sum(p.hi, p.lo + self.lo * b)
        }

        fn div_f64(self, b: f64) -> Self {
            let q1 = self.hi / b;
            let p = two_prod(q1, b);
            let s = two_sum(self.hi, -p.hi);
            let e = s.lo + self.lo - p.lo;
            let q2 = (s.hi + e) / b;
            quick_two_sum(q1, q2)
        }

        /// Times `2^n` (exact while the parts stay normal).
        fn scale(self, factor: f64) -> Self {
            Self {
                hi: self.hi * factor,
                lo: self.lo * factor,
            }
        }
    }

    /// `2^k` for `-1022 <= k <= 1023`.
    fn pow2(k: i32) -> f64 {
        let biased = u64::try_from(k + 1023).unwrap_or(1023);
        f64::from_bits(biased << 52)
    }

    /// `exp(a) = 2^k * exp(r)` with `r = a - k ln 2`, unrounded, and `k`.
    /// `|a| < 709`, so `k` is within `[-1022, 1023]`.
    fn exp_parts(a: Dd) -> (Dd, i32) {
        let k = (a.hi / LN2.hi).round();
        let r = a.sub(LN2.mul_f64(k));
        let r = r.scale(pow2(-i32::try_from(SQUARINGS).unwrap_or(0)));
        // s = exp(r) - 1 by its series.
        let mut term = r;
        let mut s = r;
        for n in 2..=TERMS {
            term = term.mul(r).div_f64(f64::from(n));
            s = s.add(term);
        }
        // (1 + s)^2 - 1 = 2s + s^2, without losing s's low bits to the 1.
        for _ in 0..SQUARINGS {
            s = s.scale(2.0).add(s.mul(s));
        }
        // `k` is a whole number within the range above.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "k is a whole number in [-1022, 1023]"
        )]
        let k = k as i32;
        (Dd::from_f64(1.0).add(s), k)
    }

    /// `exp(a)` rounded to the nearest double, for `-708 < a.hi < 709`.
    pub(super) fn exp_rounded(a: Dd) -> f64 {
        let (m, k) = exp_parts(a);
        // `m` is normalised, so `m.hi` is `m.hi + m.lo` rounded; the
        // scaling by `2^k` is exact for a normal result. 2^k is split so
        // each factor stays a normal double.
        let half = k / 2;
        (m.hi + m.lo) * pow2(half) * pow2(k - half)
    }

    fn exp(a: Dd) -> Dd {
        let (m, k) = exp_parts(a);
        let half = k / 2;
        m.scale(pow2(half)).scale(pow2(k - half))
    }

    /// `ln x` for a positive normal `x`: `x = m 2^e` with `m` in `[1, 2)`,
    /// `ln m` from `libm`'s `log` refined by one Newton step
    /// `y + m exp(-y) - 1` (which squares its error), plus `e ln 2`.
    pub(super) fn log(x: f64) -> Dd {
        let bits = x.to_bits();
        let biased = i32::try_from(bits >> 52).unwrap_or(1023);
        let m = f64::from_bits((bits & 0x000F_FFFF_FFFF_FFFF) | 0x3FF0_0000_0000_0000);
        let y = Dd::from_f64(libm::log(m));
        let correction = exp(y.neg()).mul_f64(m).sub(Dd::from_f64(1.0));
        y.add(correction).add(LN2.mul_f64(f64::from(biased - 1023)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bits of `pow` written once (CI runs this on Windows and Linux).
    #[test]
    fn pow_should_give_the_written_bits() {
        let cases: [(f64, f64, u64); 4] = [
            (2.0, 10.0, 1024.0_f64.to_bits()),
            (9.0, 0.5, 3.0_f64.to_bits()),
            // PoB's hit chance: `(evasion / 5) ^ 0.9`, the default enemy's.
            // The MSVC runtime gives the same bits (CPython's `**`).
            (2028.6, 0.9, 0x408D_9A36_576C_22D4),
            (0.0, 0.9, 0.0_f64.to_bits()),
        ];
        for (i, (base, exponent, bits)) in cases.into_iter().enumerate() {
            let value = pow(base, exponent);
            assert_eq!(value.to_bits(), bits, "case {i}: {value}");
        }
    }

    /// Powers `libm::pow` rounds the wrong way, each with the exact power
    /// rounded once (Python's `decimal` at 80 digits): `pow` gives these
    /// bits. The first is the boots fixture's ignite (the open items'
    /// game-truth lines, group 2), whose value PoB's LuaJIT gives too.
    #[test]
    fn pow_should_be_correctly_rounded() {
        let cases: [(u64, u64, u64); 7] = [
            (
                0x3FED_9999_9999_999A,
                0x4001_B9C8_6953_6204,
                0x3FEA_EC63_3EB0_2651,
            ),
            (
                0x50BD_13DC_4E60_44FF,
                0xBFDF_4BE4_1CC4_4CE4,
                0x37B7_048A_A773_3AB5,
            ),
            (
                0x3FEC_CCCC_CCCC_CCCD,
                0x400A_369C_96C9_00B2,
                0x3FE6_A866_6007_5C95,
            ),
            (
                0x3FEE_6666_6666_6666,
                0x4037_15A4_900A_0CBB,
                0x3FD3_95F1_EA18_178F,
            ),
            (
                0x4127_1BD0_4E31_80B9,
                0xBFC7_C4CD_F3C7_93E0,
                0x3FB4_B9BE_1FC4_3E81,
            ),
            (
                0x3FBE_DD06_8970_8AE4,
                0x3FE2_34A3_6EE4_BA08,
                0x3FD3_34EB_752A_02A7,
            ),
            (
                0x4123_C411_0480_60BA,
                0x3FE8_0000_0000_0000,
                0x40D6_4BBF_0621_4EFD,
            ),
        ];
        for (i, (base, exponent, bits)) in cases.into_iter().enumerate() {
            let value = pow(f64::from_bits(base), f64::from_bits(exponent));
            assert_eq!(value.to_bits(), bits, "case {i}: {value}");
        }
    }

    /// Inputs outside the double-double path keep C's results.
    #[test]
    fn pow_should_keep_the_edge_cases() {
        assert_eq!(pow(0.0, 2.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(pow(-2.0, 3.0), -8.0);
        assert!(pow(-2.0, 0.5).is_nan());
        assert_eq!(pow(5.0, 0.0), 1.0);
        assert_eq!(pow(1.0, f64::INFINITY), 1.0);
        assert_eq!(pow(10.0, 400.0), f64::INFINITY);
        assert_eq!(pow(10.0, -400.0), 0.0);
        // A large base: the square root, correctly rounded. (`f64::MAX`'s
        // root lies `2^-109` past a halfway point, a case beyond the
        // double-double's precision, as `pow` says.)
        assert_eq!(pow(1e300, 0.5).to_bits(), 1e300_f64.sqrt().to_bits());
        assert_eq!(pow(2.0, -1021.0), f64::MIN_POSITIVE * 2.0);
    }

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
