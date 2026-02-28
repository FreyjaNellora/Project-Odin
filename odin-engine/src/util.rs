// Shared utilities — extracted for cross-module use.
//
// SplitMix64 PRNG: used by MCTS (Gumbel noise) and NNUE (random weight init).

/// SplitMix64 PRNG — fast, deterministic, no external dependency.
/// From the xoshiro family: high-quality 64-bit output.
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform float in open interval (0, 1). Never exactly 0 or 1.
    pub fn next_f64(&mut self) -> f64 {
        // Use 52 mantissa bits, shift into (0, 1) open interval.
        ((self.next_u64() >> 12) as f64 + 0.5) / (1u64 << 52) as f64
    }

    /// Random i16 (full range).
    pub fn next_i16(&mut self) -> i16 {
        self.next_u64() as i16
    }

    /// Random i8 (full range).
    pub fn next_i8(&mut self) -> i8 {
        self.next_u64() as i8
    }

    /// Random i32 (full range).
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
}
