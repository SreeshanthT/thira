use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

use serde::{Deserialize, Serialize};

/// Errors from constructing a [`Rational`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RationalError {
    /// The denominator was zero.
    ZeroDenominator,
    /// The value does not fit in `i64` numerator and denominator after reduction.
    Overflow,
}

impl fmt::Display for RationalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDenominator => f.write_str("denominator is zero"),
            Self::Overflow => f.write_str("rational value overflows i64"),
        }
    }
}

impl std::error::Error for RationalError {}

/// An exact fraction `num / den`, always stored reduced with `den > 0`.
///
/// Because the form is canonical, derived equality and hashing are exact:
/// `1/2 == 2/4`. Arithmetic is computed in `i128` and reduced; the checked
/// methods return `None` if the reduced result does not fit in `i64`, and the
/// operators panic in that case, like integer overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "RawRational", into = "RawRational")]
pub struct Rational {
    num: i64,
    den: i64,
}

/// Serialized form `{ "num": n, "den": d }`. Deserializing reduces it and rejects `den == 0`.
#[derive(Serialize, Deserialize)]
struct RawRational {
    num: i64,
    den: i64,
}

impl TryFrom<RawRational> for Rational {
    type Error = RationalError;

    fn try_from(raw: RawRational) -> Result<Self, Self::Error> {
        Self::new(raw.num, raw.den)
    }
}

impl From<Rational> for RawRational {
    fn from(r: Rational) -> Self {
        Self {
            num: r.num,
            den: r.den,
        }
    }
}

impl Rational {
    pub const ZERO: Self = Self { num: 0, den: 1 };
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// Creates `num / den`, reduced.
    pub fn new(num: i64, den: i64) -> Result<Self, RationalError> {
        if den == 0 {
            return Err(RationalError::ZeroDenominator);
        }
        Self::from_i128(i128::from(num), i128::from(den)).ok_or(RationalError::Overflow)
    }

    /// Const version of [`Rational::new`] for constants with small positive inputs.
    ///
    /// # Panics
    /// Panics (at compile time when used in a `const`) if `num < 0` or `den <= 0`.
    pub(crate) const fn new_const(num: i64, den: i64) -> Self {
        assert!(num >= 0 && den > 0, "new_const needs num >= 0 and den > 0");
        let (mut a, mut b) = (num, den);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        // a >= 1 here because den > 0.
        Self {
            num: num / a,
            den: den / a,
        }
    }

    /// Creates the whole number `n`.
    pub const fn from_integer(n: i64) -> Self {
        Self { num: n, den: 1 }
    }

    pub const fn numer(self) -> i64 {
        self.num
    }

    pub const fn denom(self) -> i64 {
        self.den
    }

    pub const fn is_zero(self) -> bool {
        self.num == 0
    }

    pub const fn is_negative(self) -> bool {
        self.num < 0
    }

    /// Largest integer less than or equal to the value.
    pub const fn floor(self) -> i64 {
        self.num.div_euclid(self.den)
    }

    /// Nearest integer, with halves rounded away from zero.
    pub fn round(self) -> i64 {
        let n = i128::from(self.num);
        let d = i128::from(self.den);
        let rounded = if n >= 0 {
            (2 * n + d) / (2 * d)
        } else {
            -((-2 * n + d) / (2 * d))
        };
        // |rounded| <= |num| + 1 fits in i64 unless num is i64::MIN with den 1, where it equals num.
        i64::try_from(rounded).unwrap_or(self.num)
    }

    /// Approximate value as `f64`, for display only. Never use it for edit math.
    #[allow(clippy::cast_precision_loss)]
    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let (a, b, c, d) = self.wide(rhs);
        Self::from_i128(a * d + c * b, b * d)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let (a, b, c, d) = self.wide(rhs);
        Self::from_i128(a * d - c * b, b * d)
    }

    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let (a, b, c, d) = self.wide(rhs);
        Self::from_i128(a * c, b * d)
    }

    /// Returns `None` on overflow or division by zero.
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.is_zero() {
            return None;
        }
        let (a, b, c, d) = self.wide(rhs);
        Self::from_i128(a * d, b * c)
    }

    fn wide(self, rhs: Self) -> (i128, i128, i128, i128) {
        (
            i128::from(self.num),
            i128::from(self.den),
            i128::from(rhs.num),
            i128::from(rhs.den),
        )
    }

    /// Reduces `num / den` (with `den != 0`) and narrows to `i64`.
    ///
    /// Products of two `i64` values always fit in `i128`, so callers can
    /// compute one level of products without overflow.
    fn from_i128(num: i128, den: i128) -> Option<Self> {
        debug_assert!(den != 0);
        let g = gcd(num.unsigned_abs(), den.unsigned_abs());
        // g divides both magnitudes, so the quotients are at most the inputs.
        let g = i128::try_from(g).ok()?;
        let (mut num, mut den) = (num / g, den / g);
        if den < 0 {
            num = num.checked_neg()?;
            den = den.checked_neg()?;
        }
        Some(Self {
            num: i64::try_from(num).ok()?,
            den: i64::try_from(den).ok()?,
        })
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

impl Default for Rational {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<i64> for Rational {
    fn from(n: i64) -> Self {
        Self::from_integer(n)
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        // Denominators are positive, so cross-multiplying keeps the order.
        let (a, b, c, d) = self.wide(*other);
        (a * d).cmp(&(c * b))
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

macro_rules! impl_op {
    ($trait:ident, $method:ident, $checked:ident, $what:literal) => {
        impl $trait for Rational {
            type Output = Self;

            /// # Panics
            #[doc = concat!("Panics if the ", $what, " overflows `i64` after reduction.")]
            fn $method(self, rhs: Self) -> Self {
                self.$checked(rhs)
                    .expect(concat!("rational ", $what, " overflow"))
            }
        }
    };
}

impl_op!(Add, add, checked_add, "addition");
impl_op!(Sub, sub, checked_sub, "subtraction");
impl_op!(Mul, mul, checked_mul, "multiplication");

impl Div for Rational {
    type Output = Self;

    /// # Panics
    /// Panics on division by zero or if the quotient overflows `i64`.
    fn div(self, rhs: Self) -> Self {
        assert!(!rhs.is_zero(), "rational division by zero");
        self.checked_div(rhs).expect("rational division overflow")
    }
}

impl Neg for Rational {
    type Output = Self;

    /// # Panics
    /// Panics if the numerator is `i64::MIN`.
    fn neg(self) -> Self {
        Self {
            num: self.num.checked_neg().expect("rational negation overflow"),
            den: self.den,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Rational {
        Rational::new(n, d).expect("valid rational")
    }

    #[test]
    fn new_reduces_and_normalizes_sign() {
        assert_eq!(r(2, 4), r(1, 2));
        assert_eq!(r(1, -2), r(-1, 2));
        assert_eq!(r(-3, -6), r(1, 2));
        assert_eq!(r(0, -5), Rational::ZERO);
        assert_eq!(r(0, 7).denom(), 1);
    }

    #[test]
    fn zero_denominator_is_rejected() {
        assert_eq!(Rational::new(1, 0), Err(RationalError::ZeroDenominator));
    }

    #[test]
    fn arithmetic() {
        assert_eq!(r(1, 2) + r(1, 3), r(5, 6));
        assert_eq!(r(1, 2) - r(1, 3), r(1, 6));
        assert_eq!(r(2, 3) * r(3, 4), r(1, 2));
        assert_eq!(r(1, 2) / r(1, 4), r(2, 1));
        assert_eq!(-r(1, 2), r(-1, 2));
    }

    #[test]
    fn ordering_uses_exact_values() {
        assert!(r(1001, 30000) < r(1, 29));
        assert!(r(-1, 2) < r(-1, 3));
        assert_eq!(r(2, 4).cmp(&r(1, 2)), Ordering::Equal);
    }

    #[test]
    fn floor_and_round() {
        assert_eq!(r(7, 2).floor(), 3);
        assert_eq!(r(-7, 2).floor(), -4);
        assert_eq!(r(7, 2).round(), 4);
        assert_eq!(r(-7, 2).round(), -4);
        assert_eq!(r(10, 3).round(), 3);
        assert_eq!(Rational::from_integer(i64::MIN).round(), i64::MIN);
    }

    #[test]
    fn overflow_is_reported_not_wrapped() {
        let big = Rational::from_integer(i64::MAX);
        assert_eq!(big.checked_add(Rational::ONE), None);
        assert_eq!(big.checked_mul(Rational::from_integer(2)), None);
        assert_eq!(Rational::ONE.checked_div(Rational::ZERO), None);
        // Large intermediate products that reduce back into range are fine.
        assert_eq!(big.checked_mul(r(2, 2)), Some(big));
    }

    #[test]
    fn display() {
        assert_eq!(r(3, 1).to_string(), "3");
        assert_eq!(r(-1001, 30000).to_string(), "-1001/30000");
    }
}
