/// xorshift32: tiny, fast on ARM7, and reproducible from a single seed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        // A zero state would stay zero forever.
        Self(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform-ish value in `0..n` (`n` > 0). Bias is irrelevant at these ranges.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }

    /// True with probability `1 / n`.
    pub fn one_in(&mut self, n: u32) -> bool {
        self.below(n) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn zero_seed_does_not_stick() {
        let mut rng = Rng::new(0);
        assert_ne!(rng.next_u32(), 0);
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = Rng::new(7);
        assert!((0..1000).all(|_| rng.below(5) < 5));
    }
}
