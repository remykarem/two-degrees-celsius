//! Seeded PCG32 random number generator. No dependencies, no global state.
#![forbid(unsafe_code)]

const MULT: u64 = 6_364_136_223_846_793_005;

/// PCG-XSH-RR 64/32 (O'Neill). Deterministic and identical on every target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    /// Seed with a value and a stream selector.
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut r = Rng {
            state: 0,
            inc: (stream << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    /// Rebuild from a saved `(state, inc)` pair.
    pub fn from_parts(state: u64, inc: u64) -> Self {
        Rng {
            state,
            inc: inc | 1,
        }
    }

    /// The `(state, inc)` pair, for saving.
    pub fn parts(&self) -> (u64, u64) {
        (self.state, self.inc)
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    pub fn next_u64(&mut self) -> u64 {
        (u64::from(self.next_u32()) << 32) | u64::from(self.next_u32())
    }

    /// Uniform in [0, 1) with 53 bits of precision.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// True with probability `p` (clamped to [0, 1]).
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < p
    }

    /// Uniform integer in [0, bound). Returns 0 when bound is 0.
    pub fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % bound;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42, 54);
        let mut b = Rng::new(42, 54);
        for _ in 0..1000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn reference_vector() {
        // Reference values from the PCG32 demo (seed 42, stream 54).
        let mut r = Rng::new(42, 54);
        let got: Vec<u32> = (0..6).map(|_| r.next_u32()).collect();
        assert_eq!(
            got,
            vec![0xa15c02b7, 0x7b47f409, 0xba1d3330, 0x83d2f293, 0xbfa4784b, 0xcbed606e]
        );
    }

    #[test]
    fn f64_in_unit_interval() {
        let mut r = Rng::new(1, 1);
        for _ in 0..10_000 {
            let x = r.next_f64();
            assert!((0.0..1.0).contains(&x));
        }
    }

    #[test]
    fn below_is_in_range_and_roundtrip() {
        let mut r = Rng::new(7, 3);
        for _ in 0..1000 {
            assert!(r.below(10) < 10);
        }
        let (s, i) = r.parts();
        let mut r2 = Rng::from_parts(s, i);
        assert_eq!(r.next_u32(), r2.next_u32());
    }
}
