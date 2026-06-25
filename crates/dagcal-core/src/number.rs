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
