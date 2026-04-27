use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

fn gcd(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Exact rational number. Denominator is always positive; stored in lowest terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rational {
    pub numer: i64,
    pub denom: i64,
}

impl Rational {
    pub fn new(n: i64, d: i64) -> Self {
        assert!(d != 0, "Rational::new: denominator cannot be zero");
        if n == 0 {
            return Rational { numer: 0, denom: 1 };
        }
        let sign = if d < 0 { -1i64 } else { 1 };
        let n = n * sign;
        let d = d * sign;
        let g = gcd(n.abs(), d);
        Rational {
            numer: n / g,
            denom: d / g,
        }
    }

    pub fn zero() -> Self {
        Rational { numer: 0, denom: 1 }
    }
    pub fn one() -> Self {
        Rational { numer: 1, denom: 1 }
    }
    pub fn neg_one() -> Self {
        Rational {
            numer: -1,
            denom: 1,
        }
    }

    pub fn is_zero(self) -> bool {
        self.numer == 0
    }
    pub fn is_one(self) -> bool {
        self.numer == 1 && self.denom == 1
    }
    pub fn is_neg_one(self) -> bool {
        self.numer == -1 && self.denom == 1
    }
    pub fn is_integer(self) -> bool {
        self.denom == 1
    }
    pub fn is_positive(self) -> bool {
        self.numer > 0
    }
    pub fn is_negative(self) -> bool {
        self.numer < 0
    }

    pub fn to_f64(self) -> f64 {
        self.numer as f64 / self.denom as f64
    }

    pub fn abs(self) -> Self {
        Rational {
            numer: self.numer.abs(),
            denom: self.denom,
        }
    }

    pub fn recip(self) -> Self {
        if self.numer < 0 {
            Rational {
                numer: -self.denom,
                denom: -self.numer,
            }
        } else {
            Rational {
                numer: self.denom,
                denom: self.numer,
            }
        }
    }

    /// Integer power with overflow checking. Returns None on overflow.
    pub fn checked_pow_int(self, n: i32) -> Option<Self> {
        if n == 0 {
            return Some(Rational::one());
        }
        if n < 0 {
            return self.recip().checked_pow_int(-n);
        }
        let mut result_n: i64 = 1;
        let mut result_d: i64 = 1;
        for _ in 0..n {
            result_n = result_n.checked_mul(self.numer)?;
            result_d = result_d.checked_mul(self.denom)?;
        }
        Some(Rational::new(result_n, result_d))
    }
}

impl Neg for Rational {
    type Output = Self;
    fn neg(self) -> Self {
        Rational {
            numer: -self.numer,
            denom: self.denom,
        }
    }
}

impl Add for Rational {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let n = self.numer * rhs.denom + rhs.numer * self.denom;
        let d = self.denom * rhs.denom;
        Rational::new(n, d)
    }
}

impl Sub for Rational {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl Mul for Rational {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Rational::new(self.numer * rhs.numer, self.denom * rhs.denom)
    }
}

impl Div for Rational {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        self * rhs.recip()
    }
}

impl From<i64> for Rational {
    fn from(n: i64) -> Self {
        Rational { numer: n, denom: 1 }
    }
}

impl From<i32> for Rational {
    fn from(n: i32) -> Self {
        Rational {
            numer: n as i64,
            denom: 1,
        }
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denom == 1 {
            write!(f, "{}", self.numer)
        } else {
            write!(f, "{}/{}", self.numer, self.denom)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_arithmetic() {
        let half = Rational::new(1, 2);
        let third = Rational::new(1, 3);
        assert_eq!(half + third, Rational::new(5, 6));
        assert_eq!(half * third, Rational::new(1, 6));
        assert_eq!(half - third, Rational::new(1, 6));
        assert_eq!(half / third, Rational::new(3, 2));
    }

    #[test]
    fn reduction() {
        assert_eq!(Rational::new(4, 6), Rational::new(2, 3));
        assert_eq!(Rational::new(-4, -6), Rational::new(2, 3));
        assert_eq!(Rational::new(-4, 6), Rational::new(-2, 3));
    }

    #[test]
    fn pow() {
        assert_eq!(
            Rational::new(2, 3).checked_pow_int(2),
            Some(Rational::new(4, 9))
        );
        assert_eq!(
            Rational::new(2, 3).checked_pow_int(-1),
            Some(Rational::new(3, 2))
        );
        assert_eq!(
            Rational::new(2, 1).checked_pow_int(0),
            Some(Rational::one())
        );
    }
}
