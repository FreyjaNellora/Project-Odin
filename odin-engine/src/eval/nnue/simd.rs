// NNUE SIMD Acceleration — Stage 19
//
// AVX2 implementations for accumulator operations, SCReLU activation,
// and hidden layer MatVec. Falls back to scalar on non-AVX2 hardware.
//
// All SIMD functions have identical signatures to their scalar counterparts
// and produce bit-for-bit identical results.

use super::features::{FT_OUT, HIDDEN_SIZE, QA};

// ---------------------------------------------------------------------------
// Runtime feature detection (one-time check)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
fn has_avx2() -> bool {
    is_x86_feature_detected!("avx2")
}

#[cfg(not(target_arch = "x86_64"))]
fn has_avx2() -> bool {
    false
}

/// Cached AVX2 availability. Checked once at first use.
static AVX2_AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

#[inline]
fn use_avx2() -> bool {
    *AVX2_AVAILABLE.get_or_init(has_avx2)
}

// ---------------------------------------------------------------------------
// Accumulator add/sub (256 x i16 saturating)
// ---------------------------------------------------------------------------

/// Add a feature's weight column to accumulator values (saturating i16).
#[inline]
pub fn accumulator_add(values: &mut [i16; FT_OUT], weights: &[i16]) {
    #[cfg(target_arch = "x86_64")]
    {
        if use_avx2() {
            // SAFETY: AVX2 verified available, slices are FT_OUT=256 elements,
            // 256 * 2 bytes = 512 bytes, divisible by 32.
            unsafe { accumulator_add_avx2(values, weights) };
            return;
        }
    }
    accumulator_add_scalar(values, weights);
}

/// Subtract a feature's weight column from accumulator values (saturating i16).
#[inline]
pub fn accumulator_sub(values: &mut [i16; FT_OUT], weights: &[i16]) {
    #[cfg(target_arch = "x86_64")]
    {
        if use_avx2() {
            unsafe { accumulator_sub_avx2(values, weights) };
            return;
        }
    }
    accumulator_sub_scalar(values, weights);
}

// Scalar fallback
#[inline]
fn accumulator_add_scalar(values: &mut [i16; FT_OUT], weights: &[i16]) {
    for j in 0..FT_OUT {
        values[j] = values[j].saturating_add(weights[j]);
    }
}

#[inline]
fn accumulator_sub_scalar(values: &mut [i16; FT_OUT], weights: &[i16]) {
    for j in 0..FT_OUT {
        values[j] = values[j].saturating_sub(weights[j]);
    }
}

// AVX2 implementation
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accumulator_add_avx2(values: &mut [i16; FT_OUT], weights: &[i16]) {
    use std::arch::x86_64::*;
    // Process 16 x i16 per iteration. FT_OUT=256, so 16 iterations.
    let mut i = 0;
    while i < FT_OUT {
        let v = _mm256_loadu_si256(values[i..].as_ptr() as *const __m256i);
        let w = _mm256_loadu_si256(weights[i..].as_ptr() as *const __m256i);
        let r = _mm256_adds_epi16(v, w);
        _mm256_storeu_si256(values[i..].as_mut_ptr() as *mut __m256i, r);
        i += 16;
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn accumulator_sub_avx2(values: &mut [i16; FT_OUT], weights: &[i16]) {
    use std::arch::x86_64::*;
    let mut i = 0;
    while i < FT_OUT {
        let v = _mm256_loadu_si256(values[i..].as_ptr() as *const __m256i);
        let w = _mm256_loadu_si256(weights[i..].as_ptr() as *const __m256i);
        let r = _mm256_subs_epi16(v, w);
        _mm256_storeu_si256(values[i..].as_mut_ptr() as *mut __m256i, r);
        i += 16;
    }
}

// ---------------------------------------------------------------------------
// SCReLU activation: clamp(x, 0, QA)^2, converting i16 → i32
// ---------------------------------------------------------------------------

/// Apply SCReLU activation to `input` (256 x i16), writing i32 results to `output`.
/// output[j] = clamp(input[j], 0, QA)^2
#[inline]
pub fn screlu_activate(input: &[i16; FT_OUT], output: &mut [i32], offset: usize) {
    #[cfg(target_arch = "x86_64")]
    {
        if use_avx2() {
            unsafe { screlu_activate_avx2(input, output, offset) };
            return;
        }
    }
    screlu_activate_scalar(input, output, offset);
}

#[inline]
fn screlu_activate_scalar(input: &[i16; FT_OUT], output: &mut [i32], offset: usize) {
    let qa = QA as i32;
    for j in 0..FT_OUT {
        let clamped = (input[j] as i32).clamp(0, qa);
        output[offset + j] = clamped * clamped;
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn screlu_activate_avx2(input: &[i16; FT_OUT], output: &mut [i32], offset: usize) {
    use std::arch::x86_64::*;

    let zero = _mm256_setzero_si256();
    let qa_vec = _mm256_set1_epi16(QA);

    // Process 16 i16 → 16 i32 per iteration (split into two 8xi32 groups).
    let mut j = 0;
    while j < FT_OUT {
        // Load 16 x i16
        let v = _mm256_loadu_si256(input[j..].as_ptr() as *const __m256i);
        // Clamp to [0, QA]
        let clamped = _mm256_min_epi16(_mm256_max_epi16(v, zero), qa_vec);

        // Unpack to i32 and square.
        // AVX2 _mm256_cvtepi16_epi32 takes a 128-bit input (__m128i).
        // Extract low 128 bits and high 128 bits of the 256-bit clamped vector.
        let lo128 = _mm256_castsi256_si128(clamped);
        let hi128 = _mm256_extracti128_si256(clamped, 1);

        // Sign-extend i16 → i32
        let lo_i32 = _mm256_cvtepi16_epi32(lo128); // 8 x i32
        let hi_i32 = _mm256_cvtepi16_epi32(hi128); // 8 x i32

        // Square: result = val * val
        let lo_sq = _mm256_mullo_epi32(lo_i32, lo_i32);
        let hi_sq = _mm256_mullo_epi32(hi_i32, hi_i32);

        // Store 8 + 8 = 16 i32 values
        _mm256_storeu_si256(output[offset + j..].as_mut_ptr() as *mut __m256i, lo_sq);
        _mm256_storeu_si256(
            output[offset + j + 8..].as_mut_ptr() as *mut __m256i,
            hi_sq,
        );

        j += 16;
    }
}

// ---------------------------------------------------------------------------
// Hidden layer MatVec: 32 neurons × 1024 inputs
// ---------------------------------------------------------------------------

/// Compute hidden layer output using transposed weights.
///
/// `weights_t` layout: [HIDDEN_SIZE][FT_OUT * 4] = [32][1024] (row-major per neuron).
/// `activated_u8`: the SCReLU output divided by QA, as u8 (range [0, 255]).
/// `biases`: [HIDDEN_SIZE] i32 biases.
/// `output`: [HIDDEN_SIZE] i32 result after ClippedReLU.
#[inline]
pub fn hidden_layer_forward(
    weights_t: &[i8],
    activated_u8: &[u8],
    biases: &[i32],
    output: &mut [i32; HIDDEN_SIZE],
) {
    #[cfg(target_arch = "x86_64")]
    {
        if use_avx2() {
            unsafe { hidden_layer_forward_avx2(weights_t, activated_u8, biases, output) };
            return;
        }
    }
    hidden_layer_forward_scalar(weights_t, activated_u8, biases, output);
}

const INPUT_SIZE: usize = FT_OUT * 4; // 1024

#[inline]
fn hidden_layer_forward_scalar(
    weights_t: &[i8],
    activated_u8: &[u8],
    biases: &[i32],
    output: &mut [i32; HIDDEN_SIZE],
) {
    for h in 0..HIDDEN_SIZE {
        let mut acc = biases[h];
        let row_offset = h * INPUT_SIZE;
        for i in 0..INPUT_SIZE {
            acc += weights_t[row_offset + i] as i32 * activated_u8[i] as i32;
        }
        // ClippedReLU
        output[h] = acc.max(0);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn hidden_layer_forward_avx2(
    weights_t: &[i8],
    activated_u8: &[u8],
    biases: &[i32],
    output: &mut [i32; HIDDEN_SIZE],
) {
    use std::arch::x86_64::*;

    for h in 0..HIDDEN_SIZE {
        let mut acc = _mm256_setzero_si256();
        let row_offset = h * INPUT_SIZE;

        // Process 32 elements per iteration: u8 inputs × i8 weights
        // _mm256_maddubs_epi16: unsigned × signed → 16 x i16 (horizontal pairs)
        // _mm256_madd_epi16: i16 pairs → 8 x i32 (horizontal pairs)
        let mut i = 0;
        while i < INPUT_SIZE {
            // Load 32 x u8 inputs and 32 x i8 weights
            let inp = _mm256_loadu_si256(activated_u8[i..].as_ptr() as *const __m256i);
            let wt = _mm256_loadu_si256(weights_t[row_offset + i..].as_ptr() as *const __m256i);

            // Multiply unsigned × signed → 16 x i16
            let prod16 = _mm256_maddubs_epi16(inp, wt);

            // Horizontal pair add i16 → 8 x i32
            let prod32 = _mm256_madd_epi16(prod16, _mm256_set1_epi16(1));

            // Accumulate
            acc = _mm256_add_epi32(acc, prod32);

            i += 32;
        }

        // Horizontal sum of 8 x i32 → single i32
        // Step 1: Add high 128 to low 128
        let hi128 = _mm256_extracti128_si256(acc, 1);
        let lo128 = _mm256_castsi256_si128(acc);
        let sum128 = _mm_add_epi32(lo128, hi128); // 4 x i32

        // Step 2: Horizontal add within 128 bits
        let shuf = _mm_shuffle_epi32(sum128, 0b_01_00_11_10); // swap pairs
        let sum64 = _mm_add_epi32(sum128, shuf); // 2 unique i32
        let shuf2 = _mm_shuffle_epi32(sum64, 0b_00_00_00_01);
        let sum32 = _mm_add_epi32(sum64, shuf2);

        let total = _mm_cvtsi128_si32(sum32) + biases[h];

        // ClippedReLU
        output[h] = total.max(0);
    }

    // Prevent AVX→SSE transition penalty
    _mm256_zeroupper();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulator_add_simd_matches_scalar() {
        let mut values_simd = [0i16; FT_OUT];
        let mut values_scalar = [0i16; FT_OUT];
        let mut weights = [0i16; FT_OUT];

        // Fill with test data
        for i in 0..FT_OUT {
            values_simd[i] = (i as i16 * 7) % 500 - 250;
            values_scalar[i] = values_simd[i];
            weights[i] = (i as i16 * 13) % 100 - 50;
        }

        accumulator_add(&mut values_simd, &weights);
        accumulator_add_scalar(&mut values_scalar, &weights);

        assert_eq!(values_simd, values_scalar, "SIMD add must match scalar");
    }

    #[test]
    fn test_accumulator_sub_simd_matches_scalar() {
        let mut values_simd = [0i16; FT_OUT];
        let mut values_scalar = [0i16; FT_OUT];
        let mut weights = [0i16; FT_OUT];

        for i in 0..FT_OUT {
            values_simd[i] = (i as i16 * 7) % 500 - 250;
            values_scalar[i] = values_simd[i];
            weights[i] = (i as i16 * 13) % 100 - 50;
        }

        accumulator_sub(&mut values_simd, &weights);
        accumulator_sub_scalar(&mut values_scalar, &weights);

        assert_eq!(values_simd, values_scalar, "SIMD sub must match scalar");
    }

    #[test]
    fn test_accumulator_add_saturation() {
        let mut values = [i16::MAX - 10; FT_OUT];
        let weights = [20i16; FT_OUT];

        accumulator_add(&mut values, &weights);

        // Should saturate at i16::MAX, not wrap
        for &v in &values {
            assert_eq!(v, i16::MAX);
        }
    }

    #[test]
    fn test_accumulator_sub_saturation() {
        let mut values = [i16::MIN + 10; FT_OUT];
        let weights = [20i16; FT_OUT];

        accumulator_sub(&mut values, &weights);

        for &v in &values {
            assert_eq!(v, i16::MIN);
        }
    }

    #[test]
    fn test_screlu_simd_matches_scalar() {
        let mut input = [0i16; FT_OUT];
        for i in 0..FT_OUT {
            input[i] = (i as i16 * 3) % 600 - 200; // range: -200 to 399
        }

        let mut out_simd = vec![0i32; FT_OUT];
        let mut out_scalar = vec![0i32; FT_OUT];

        screlu_activate(&input, &mut out_simd, 0);
        screlu_activate_scalar(&input, &mut out_scalar, 0);

        assert_eq!(out_simd, out_scalar, "SCReLU SIMD must match scalar");
    }

    #[test]
    fn test_screlu_clamping() {
        let mut input = [0i16; FT_OUT];
        input[0] = -100; // Should clamp to 0
        input[1] = 0; // Edge: 0^2 = 0
        input[2] = QA; // Edge: 255^2 = 65025
        input[3] = QA + 100; // Should clamp to QA

        let mut output = vec![0i32; FT_OUT];
        screlu_activate(&input, &mut output, 0);

        assert_eq!(output[0], 0);
        assert_eq!(output[1], 0);
        assert_eq!(output[2], (QA as i32) * (QA as i32));
        assert_eq!(output[3], (QA as i32) * (QA as i32));
    }

    #[test]
    fn test_hidden_layer_simd_matches_scalar() {
        let input_size = FT_OUT * 4;

        // Create deterministic test data
        let mut weights_t = vec![0i8; HIDDEN_SIZE * input_size];
        let mut activated_u8 = vec![0u8; input_size];
        let biases = vec![100i32; HIDDEN_SIZE];

        for i in 0..weights_t.len() {
            weights_t[i] = ((i * 7 + 3) % 60) as i8 - 30;
        }
        for i in 0..activated_u8.len() {
            activated_u8[i] = ((i * 11 + 5) % 200) as u8;
        }

        let mut out_simd = [0i32; HIDDEN_SIZE];
        let mut out_scalar = [0i32; HIDDEN_SIZE];

        hidden_layer_forward(&weights_t, &activated_u8, &biases, &mut out_simd);
        hidden_layer_forward_scalar(&weights_t, &activated_u8, &biases, &mut out_scalar);

        assert_eq!(
            out_simd, out_scalar,
            "Hidden layer SIMD must match scalar"
        );
    }

    #[test]
    fn test_hidden_layer_clipped_relu() {
        let input_size = FT_OUT * 4;
        let weights_t = vec![0i8; HIDDEN_SIZE * input_size];
        let activated_u8 = vec![0u8; input_size];
        let biases = vec![-100i32; HIDDEN_SIZE]; // Negative biases → should be clamped to 0

        let mut output = [0i32; HIDDEN_SIZE];
        hidden_layer_forward(&weights_t, &activated_u8, &biases, &mut output);

        for &v in &output {
            assert_eq!(v, 0, "Negative values should be clamped to 0 by ClippedReLU");
        }
    }
}
