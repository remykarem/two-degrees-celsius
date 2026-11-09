use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Money in integer cents. All arithmetic is checked; overflow is a bug and panics.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cents(pub i64);

impl Cents {
    pub const ZERO: Cents = Cents(0);

    pub fn from_dollars(d: f64) -> Cents {
        Cents::from_f64(d * 100.0)
    }
    /// Round a float number of cents. Panics on non-finite input (a model bug).
    pub fn from_f64(c: f64) -> Cents {
        assert!(c.is_finite(), "non-finite money value");
        assert!(c.abs() < 9.0e18, "money value out of range");
        Cents(c.round() as i64)
    }
    pub fn dollars(self) -> f64 {
        self.0 as f64 / 100.0
    }
    pub fn as_f64(self) -> f64 {
        self.0 as f64
    }
    pub fn checked_add(self, o: Cents) -> Option<Cents> {
        self.0.checked_add(o.0).map(Cents)
    }
    pub fn checked_sub(self, o: Cents) -> Option<Cents> {
        self.0.checked_sub(o.0).map(Cents)
    }
    /// Scale by a float factor, rounding to the nearest cent.
    pub fn scale(self, f: f64) -> Cents {
        Cents::from_f64(self.0 as f64 * f)
    }
    pub fn max(self, o: Cents) -> Cents {
        Cents(self.0.max(o.0))
    }
    pub fn min(self, o: Cents) -> Cents {
        Cents(self.0.min(o.0))
    }
    pub fn is_negative(self) -> bool {
        self.0 < 0
    }
    /// Sum of an iterator of Cents.
    pub fn sum<I: IntoIterator<Item = Cents>>(it: I) -> Cents {
        it.into_iter().fold(Cents::ZERO, |a, b| a + b)
    }
}

impl Add for Cents {
    type Output = Cents;
    fn add(self, o: Cents) -> Cents {
        self.checked_add(o).expect("money overflow")
    }
}
impl Sub for Cents {
    type Output = Cents;
    fn sub(self, o: Cents) -> Cents {
        self.checked_sub(o).expect("money overflow")
    }
}
impl Neg for Cents {
    type Output = Cents;
    fn neg(self) -> Cents {
        Cents(self.0.checked_neg().expect("money overflow"))
    }
}
impl AddAssign for Cents {
    fn add_assign(&mut self, o: Cents) {
        *self = *self + o;
    }
}
impl SubAssign for Cents {
    fn sub_assign(&mut self, o: Cents) {
        *self = *self - o;
    }
}

macro_rules! f64_newtype {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub f64);
        impl $name {
            pub const ZERO: $name = $name(0.0);
            pub fn get(self) -> f64 { self.0 }
            pub fn max(self, o: $name) -> $name { $name(self.0.max(o.0)) }
            pub fn min(self, o: $name) -> $name { $name(self.0.min(o.0)) }
            pub fn is_finite(self) -> bool { self.0.is_finite() }
        }
        impl Add for $name { type Output = $name; fn add(self, o: $name) -> $name { $name(self.0 + o.0) } }
        impl Sub for $name { type Output = $name; fn sub(self, o: $name) -> $name { $name(self.0 - o.0) } }
        impl AddAssign for $name { fn add_assign(&mut self, o: $name) { self.0 += o.0; } }
        impl SubAssign for $name { fn sub_assign(&mut self, o: $name) { self.0 -= o.0; } }
        impl Mul<f64> for $name { type Output = $name; fn mul(self, f: f64) -> $name { $name(self.0 * f) } }
        impl Div<f64> for $name { type Output = $name; fn div(self, f: f64) -> $name { $name(self.0 / f) } }
    };
}

f64_newtype!(
    /// Tonnes of CO2.
    Tonnes
);
f64_newtype!(
    /// Physical units: goods, food, capital, housing, natural stocks.
    Units
);
f64_newtype!(
    /// Number of people (cohort weights, employment).
    People
);
f64_newtype!(
    /// Dimensionless rates, shares, indices in model units.
    Rate
);

/// Division where the zero case is explicit.
pub fn safe_div(num: f64, den: f64, if_zero: f64) -> f64 {
    if den == 0.0 || !den.is_finite() {
        if_zero
    } else {
        num / den
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents_arithmetic() {
        let a = Cents::from_dollars(12.34);
        assert_eq!(a, Cents(1234));
        assert_eq!(a + Cents(66), Cents(1300));
        assert_eq!(a.scale(0.5), Cents(617));
        assert!(Cents(i64::MAX).checked_add(Cents(1)).is_none());
    }

    #[test]
    #[should_panic]
    fn cents_overflow_panics() {
        let _ = Cents(i64::MAX) + Cents(1);
    }

    #[test]
    fn safe_div_zero() {
        assert_eq!(safe_div(1.0, 0.0, 7.0), 7.0);
        assert_eq!(safe_div(6.0, 3.0, 7.0), 2.0);
    }
}
