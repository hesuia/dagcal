use crate::error::EvalError;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Number {
    Rational(BigRational),
    Float(f64),
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rational(lhs), Self::Rational(rhs)) => lhs == rhs,
            (Self::Float(lhs), Self::Float(rhs)) => lhs == rhs,
            (lhs, rhs) => lhs.to_f64() == rhs.to_f64(),
        }
    }
}

impl Number {
    pub fn rational(numerator: impl Into<BigInt>, denominator: impl Into<BigInt>) -> Self {
        Self::Rational(BigRational::new(numerator.into(), denominator.into()))
    }

    pub fn integer(value: impl Into<BigInt>) -> Self {
        Self::Rational(BigRational::from_integer(value.into()))
    }

    pub fn from_f64(value: f64) -> Result<Self, EvalError> {
        if value.is_finite() {
            Ok(Self::Float(value))
        } else {
            Err(EvalError::Math("number is non-finite".to_string()))
        }
    }

    pub(crate) fn from_decimal_literal(input: &str) -> Option<Self> {
        let (mantissa, exponent) = split_exponent(input)?;
        let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits = format!("{whole}{fraction}");
        let numerator = if digits.is_empty() {
            BigInt::zero()
        } else {
            BigInt::from_str(&digits).ok()?
        };
        let scale = fraction.len() as i64 - exponent;
        if scale >= 0 {
            Some(Self::rational(numerator, pow10(scale as u32)))
        } else {
            Some(Self::integer(numerator * pow10((-scale) as u32)))
        }
    }

    pub(crate) fn from_based_literal(digits: &str, base: u32) -> Option<Self> {
        let (integer, fraction) = digits.split_once('.').unwrap_or((digits, ""));
        if integer.is_empty() && fraction.is_empty() {
            return None;
        }

        let base_int = BigInt::from(base);
        let mut numerator = BigInt::zero();
        for digit in integer.chars().chain(fraction.chars()) {
            numerator = numerator * &base_int + BigInt::from(digit.to_digit(base)?);
        }
        let denominator = base_int.pow(fraction.len() as u32);
        Some(Self::rational(numerator, denominator))
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Self::Rational(value) => value.to_f64().unwrap_or_else(|| {
                if value.is_negative() {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                }
            }),
            Self::Float(value) => *value,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Self::Rational(value) => value.is_zero(),
            Self::Float(value) => *value == 0.0,
        }
    }

    pub fn is_finite(&self) -> bool {
        match self {
            Self::Rational(_) => true,
            Self::Float(value) => value.is_finite(),
        }
    }

    pub(crate) fn finite(self, message: impl FnOnce() -> String) -> Result<Self, EvalError> {
        if self.is_finite() {
            Ok(self)
        } else {
            Err(EvalError::Math(message()))
        }
    }

    pub(crate) fn pow(self, rhs: Self) -> Result<Self, EvalError> {
        match (&self, &rhs) {
            (Self::Rational(base), Self::Rational(exponent)) if exponent.is_integer() => {
                let exponent = exponent.to_integer();
                let Some(exponent) = exponent.to_i32() else {
                    return Self::Float(self.to_f64().powf(rhs.to_f64()))
                        .finite(|| "power operation produced non-finite result".to_string());
                };
                Ok(Self::Rational(pow_rational(base, exponent)))
            }
            _ => Self::Float(self.to_f64().powf(rhs.to_f64()))
                .finite(|| "power operation produced non-finite result".to_string()),
        }
    }

    pub(crate) fn abs(&self) -> Self {
        match self {
            Self::Rational(value) => Self::Rational(value.abs()),
            Self::Float(value) => Self::Float(value.abs()),
        }
    }

    pub(crate) fn recip(&self) -> Self {
        match self {
            Self::Rational(value) => Self::Rational(value.recip()),
            Self::Float(value) => Self::Float(value.recip()),
        }
    }
}

impl From<i32> for Number {
    fn from(value: i32) -> Self {
        Self::integer(value)
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::integer(value)
    }
}

impl From<usize> for Number {
    fn from(value: usize) -> Self {
        Self::integer(value)
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl fmt::Display for Number {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rational(value) if value.is_integer() => {
                write!(formatter, "{}", value.to_integer())
            }
            _ => write!(formatter, "{}", self.to_f64()),
        }
    }
}

impl Add for Number {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Rational(lhs), Self::Rational(rhs)) => Self::Rational(lhs + rhs),
            (lhs, rhs) => Self::Float(lhs.to_f64() + rhs.to_f64()),
        }
    }
}

impl Sub for Number {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Rational(lhs), Self::Rational(rhs)) => Self::Rational(lhs - rhs),
            (lhs, rhs) => Self::Float(lhs.to_f64() - rhs.to_f64()),
        }
    }
}

impl Mul for Number {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Rational(lhs), Self::Rational(rhs)) => Self::Rational(lhs * rhs),
            (lhs, rhs) => Self::Float(lhs.to_f64() * rhs.to_f64()),
        }
    }
}

impl Div for Number {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Rational(lhs), Self::Rational(rhs)) => Self::Rational(lhs / rhs),
            (lhs, rhs) => Self::Float(lhs.to_f64() / rhs.to_f64()),
        }
    }
}

impl Rem for Number {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Rational(lhs), Self::Rational(rhs)) => {
                let quotient = (&lhs / &rhs).trunc();
                Self::Rational(lhs - rhs * quotient)
            }
            (lhs, rhs) => Self::Float(lhs.to_f64() % rhs.to_f64()),
        }
    }
}

impl Neg for Number {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            Self::Rational(value) => Self::Rational(-value),
            Self::Float(value) => Self::Float(-value),
        }
    }
}

fn split_exponent(input: &str) -> Option<(&str, i64)> {
    if let Some((mantissa, exponent)) = input.split_once(['e', 'E']) {
        Some((mantissa, exponent.parse().ok()?))
    } else {
        Some((input, 0))
    }
}

fn pow10(exponent: u32) -> BigInt {
    BigInt::from(10_u8).pow(exponent)
}

fn pow_rational(value: &BigRational, exponent: i32) -> BigRational {
    if exponent == 0 {
        return BigRational::one();
    }

    let power = exponent.unsigned_abs();
    let result = BigRational::new(value.numer().pow(power), value.denom().pow(power));
    if exponent < 0 { result.recip() } else { result }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_float_close(actual: Number, expected: f64) {
        match actual {
            Number::Float(actual) => {
                assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
            }
            other => panic!("expected float, got {other:?}"),
        }
    }

    fn assert_rational(actual: Number, numerator: i64, denominator: i64) {
        assert_eq!(actual, Number::rational(numerator, denominator));
    }

    #[test]
    fn constructors_create_reduced_rationals_and_floats() {
        assert_eq!(Number::integer(42), Number::rational(42, 1));
        assert_eq!(Number::rational(6, 8), Number::rational(3, 4));
        assert_eq!(Number::from(7_i32), Number::rational(7, 1));
        assert_eq!(Number::from(7_i64), Number::rational(7, 1));
        assert_eq!(Number::from(7_usize), Number::rational(7, 1));
        assert_eq!(Number::from(0.25), Number::Float(0.25));
        assert_eq!(Number::from_f64(0.25), Ok(Number::Float(0.25)));
    }

    #[test]
    fn from_f64_rejects_non_finite_values() {
        assert!(matches!(
            Number::from_f64(f64::NAN),
            Err(EvalError::Math(message)) if message == "number is non-finite"
        ));
        assert!(matches!(
            Number::from_f64(f64::INFINITY),
            Err(EvalError::Math(message)) if message == "number is non-finite"
        ));
    }

    #[test]
    fn parses_decimal_literals_exactly() {
        assert_rational(Number::from_decimal_literal("10").unwrap(), 10, 1);
        assert_rational(Number::from_decimal_literal("4.2").unwrap(), 21, 5);
        assert_rational(Number::from_decimal_literal(".5").unwrap(), 1, 2);
        assert_rational(Number::from_decimal_literal("1.").unwrap(), 1, 1);
        assert_rational(Number::from_decimal_literal("1e3").unwrap(), 1000, 1);
        assert_rational(Number::from_decimal_literal("1e-3").unwrap(), 1, 1000);
        assert_rational(Number::from_decimal_literal("2.5E-1").unwrap(), 1, 4);
        assert_rational(Number::from_decimal_literal("12.3400").unwrap(), 617, 50);
    }

    #[test]
    fn rejects_invalid_decimal_literals() {
        assert_eq!(Number::from_decimal_literal("abc"), None);
        assert_eq!(Number::from_decimal_literal("1e"), None);
        assert_eq!(Number::from_decimal_literal("1e+"), None);
        assert_eq!(Number::from_decimal_literal("1.2.3"), None);
    }

    #[test]
    fn parses_based_literals_exactly() {
        assert_rational(Number::from_based_literal("1001.1101", 2).unwrap(), 157, 16);
        assert_rational(Number::from_based_literal(".1", 2).unwrap(), 1, 2);
        assert_rational(Number::from_based_literal("10.4", 8).unwrap(), 17, 2);
        assert_rational(Number::from_based_literal("A.F", 16).unwrap(), 175, 16);
        assert_rational(Number::from_based_literal("ff.", 16).unwrap(), 255, 1);
    }

    #[test]
    fn rejects_invalid_based_literals() {
        assert_eq!(Number::from_based_literal(".", 2), None);
        assert_eq!(Number::from_based_literal("102", 2), None);
        assert_eq!(Number::from_based_literal("8", 8), None);
        assert_eq!(Number::from_based_literal("G", 16), None);
    }

    #[test]
    fn rational_arithmetic_stays_exact() {
        assert_rational(Number::rational(1, 10) + Number::rational(2, 10), 3, 10);
        assert_rational(Number::rational(5, 6) - Number::rational(1, 3), 1, 2);
        assert_rational(Number::rational(2, 3) * Number::rational(9, 4), 3, 2);
        assert_rational(Number::rational(2, 3) / Number::rational(4, 5), 5, 6);
        assert_rational(-Number::rational(1, 3), -1, 3);
    }

    #[test]
    fn rational_remainder_matches_truncated_quotient_semantics() {
        assert_rational(Number::rational(7, 3) % Number::from(1), 1, 3);
        assert_rational(Number::rational(7, 3) % Number::rational(2, 3), 1, 3);
        assert_rational(Number::rational(-7, 3) % Number::from(1), -1, 3);
    }

    #[test]
    fn mixed_arithmetic_returns_float() {
        assert_float_close(Number::rational(1, 2) + Number::Float(0.25), 0.75);
        assert_float_close(Number::Float(1.0) - Number::rational(1, 4), 0.75);
        assert_float_close(Number::Float(2.0) * Number::rational(3, 4), 1.5);
        assert_float_close(Number::rational(3, 2) / Number::Float(2.0), 0.75);
        assert_float_close(Number::Float(5.5) % Number::from(2), 1.5);
    }

    #[test]
    fn integer_rational_powers_stay_exact() {
        assert_eq!(
            Number::rational(2, 3).pow(Number::from(3)).unwrap(),
            Number::rational(8, 27)
        );
        assert_eq!(
            Number::rational(2, 3).pow(Number::from(-2)).unwrap(),
            Number::rational(9, 4)
        );
        assert_eq!(
            Number::rational(2, 3).pow(Number::from(0)).unwrap(),
            Number::from(1)
        );
    }

    #[test]
    fn non_integer_or_float_powers_return_float() {
        assert_float_close(Number::from(9).pow(Number::rational(1, 2)).unwrap(), 3.0);
        assert_float_close(Number::Float(2.0).pow(Number::from(3)).unwrap(), 8.0);
    }

    #[test]
    fn non_finite_power_results_are_errors() {
        assert!(matches!(
            Number::Float(1e308).pow(Number::from(2)),
            Err(EvalError::Math(message))
                if message == "power operation produced non-finite result"
        ));
    }

    #[test]
    fn zero_and_finite_checks_cover_both_variants() {
        assert!(Number::from(0).is_zero());
        assert!(Number::Float(0.0).is_zero());
        assert!(!Number::rational(1, 2).is_zero());
        assert!(Number::rational(1, 2).is_finite());
        assert!(Number::Float(1.0).is_finite());
        assert!(!Number::Float(f64::NAN).is_finite());
        assert!(!Number::Float(f64::INFINITY).is_finite());
    }

    #[test]
    fn finite_wraps_non_finite_values_with_supplied_message() {
        assert_eq!(
            Number::rational(1, 2).finite(|| "unused".to_string()),
            Ok(Number::rational(1, 2))
        );
        assert!(matches!(
            Number::Float(f64::NAN).finite(|| "bad value".to_string()),
            Err(EvalError::Math(message)) if message == "bad value"
        ));
    }

    #[test]
    fn abs_and_recip_preserve_variant_semantics() {
        assert_eq!(Number::rational(-3, 4).abs(), Number::rational(3, 4));
        assert_eq!(Number::rational(2, 3).recip(), Number::rational(3, 2));
        assert_eq!(Number::Float(-2.5).abs(), Number::Float(2.5));
        assert_eq!(Number::Float(4.0).recip(), Number::Float(0.25));
    }

    #[test]
    fn display_keeps_integer_rationals_compact_and_uses_decimal_output_otherwise() {
        assert_eq!(Number::from(42).to_string(), "42");
        assert_eq!(Number::rational(1, 2).to_string(), "0.5");
        assert_eq!(Number::Float(1.25).to_string(), "1.25");
    }

    #[test]
    fn equality_compares_cross_variant_numeric_values_by_float_conversion() {
        assert_eq!(Number::rational(1, 2), Number::Float(0.5));
        assert_eq!(Number::rational(1, 3), Number::Float(1.0 / 3.0));
        assert_ne!(Number::rational(1, 3), Number::Float(0.3));
    }

    #[test]
    fn to_f64_converts_large_rationals_to_signed_infinity_when_needed() {
        let huge_positive = Number::rational(BigInt::from(10_u8).pow(400), 1);
        let huge_negative = Number::rational(-BigInt::from(10_u8).pow(400), 1);

        assert_eq!(huge_positive.to_f64(), f64::INFINITY);
        assert_eq!(huge_negative.to_f64(), f64::NEG_INFINITY);
    }
}
