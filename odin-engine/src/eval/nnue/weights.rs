// NNUE Weight Storage and I/O — Stage 14
//
// NnueWeights struct, .onnue binary format, random initialization, CRC32.
// Feature transformer has 4 separate weight matrices (one per perspective).

use std::io::{self, Write};
use std::path::Path;

use super::features::{
    FEATURES_PER_PERSPECTIVE, FT_OUT, HIDDEN_SIZE, BRS_OUTPUT, MCTS_OUTPUT,
};
use crate::board::PLAYER_COUNT;
use crate::util::SplitMix64;

// ---------------------------------------------------------------------------
// .onnue format constants
// ---------------------------------------------------------------------------

/// Magic bytes: "ONUE"
const ONNUE_MAGIC: [u8; 4] = [b'O', b'N', b'U', b'E'];

/// Format version.
const ONNUE_VERSION: u32 = 1;

/// Header size in bytes.
const HEADER_SIZE: usize = 48;

// ---------------------------------------------------------------------------
// NnueWeights
// ---------------------------------------------------------------------------

/// NNUE weight storage for the full network.
///
/// Architecture:
///   Input (4480 sparse) -> FT (256 dense) x4 perspectives  [int16]
///   Concat (4*256=1024) -> Hidden (32) [int8 weights, int32 bias]
///   Hidden (32) -> BRS scalar (1) [int8 weights, int32 bias]
///   Hidden (32) -> MCTS 4-vec (4) [int8 weights, int32 biases]
pub struct NnueWeights {
    /// Feature transformer weights: [4 perspectives][4480 features][256 neurons].
    /// Stored flat: perspective * (4480*256) + feature * 256 + neuron.
    pub ft_weights: Vec<i16>,

    /// Feature transformer biases: [4 perspectives][256 neurons].
    /// Stored flat: perspective * 256 + neuron.
    pub ft_biases: Vec<i16>,

    /// Hidden layer weights: [1024 inputs][32 neurons].
    /// Stored flat: input * 32 + neuron.
    pub hidden_weights: Vec<i8>,

    /// Hidden layer biases: [32 neurons].
    pub hidden_biases: Vec<i32>,

    /// BRS scalar head weights: [32 inputs].
    pub brs_weights: Vec<i8>,

    /// BRS scalar head bias.
    pub brs_bias: i32,

    /// MCTS value head weights: [32 inputs][4 outputs].
    /// Stored flat: input * 4 + output.
    pub mcts_weights: Vec<i8>,

    /// MCTS value head biases: [4 outputs].
    pub mcts_biases: Vec<i32>,
}

/// Error type for .onnue loading.
#[derive(Debug)]
pub enum NnueLoadError {
    Io(io::Error),
    InvalidMagic,
    UnsupportedVersion(u32),
    ArchitectureMismatch,
    ChecksumMismatch,
    UnexpectedEof,
}

impl From<io::Error> for NnueLoadError {
    fn from(e: io::Error) -> Self {
        NnueLoadError::Io(e)
    }
}

impl std::fmt::Display for NnueLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NnueLoadError::Io(e) => write!(f, "I/O error: {e}"),
            NnueLoadError::InvalidMagic => write!(f, "invalid .onnue magic bytes"),
            NnueLoadError::UnsupportedVersion(v) => write!(f, "unsupported version: {v}"),
            NnueLoadError::ArchitectureMismatch => write!(f, "architecture hash mismatch"),
            NnueLoadError::ChecksumMismatch => write!(f, "CRC32 checksum mismatch"),
            NnueLoadError::UnexpectedEof => write!(f, "unexpected end of file"),
        }
    }
}

impl NnueWeights {
    // Dimension constants for array sizing.
    const FT_WEIGHT_COUNT: usize = PLAYER_COUNT * FEATURES_PER_PERSPECTIVE * FT_OUT;
    const FT_BIAS_COUNT: usize = PLAYER_COUNT * FT_OUT;
    const HIDDEN_WEIGHT_COUNT: usize = PLAYER_COUNT * FT_OUT * HIDDEN_SIZE;
    const HIDDEN_BIAS_COUNT: usize = HIDDEN_SIZE;
    const BRS_WEIGHT_COUNT: usize = HIDDEN_SIZE;
    const MCTS_WEIGHT_COUNT: usize = HIDDEN_SIZE * MCTS_OUTPUT;
    const MCTS_BIAS_COUNT: usize = MCTS_OUTPUT;

    /// Create weights filled with deterministic pseudo-random values.
    /// Uses SplitMix64 for reproducibility.
    pub fn random(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);

        // Feature transformer: small range [-32, 31] to avoid accumulator saturation.
        let ft_weights: Vec<i16> = (0..Self::FT_WEIGHT_COUNT)
            .map(|_| (rng.next_i16() % 64).wrapping_sub(32))
            .collect();
        let ft_biases: Vec<i16> = (0..Self::FT_BIAS_COUNT)
            .map(|_| rng.next_i16() % 16)
            .collect();

        let hidden_weights: Vec<i8> = (0..Self::HIDDEN_WEIGHT_COUNT)
            .map(|_| rng.next_i8() % 32)
            .collect();
        let hidden_biases: Vec<i32> = (0..Self::HIDDEN_BIAS_COUNT)
            .map(|_| rng.next_i32() % 256)
            .collect();

        let brs_weights: Vec<i8> = (0..Self::BRS_WEIGHT_COUNT)
            .map(|_| rng.next_i8() % 32)
            .collect();
        let brs_bias = rng.next_i32() % 256;

        let mcts_weights: Vec<i8> = (0..Self::MCTS_WEIGHT_COUNT)
            .map(|_| rng.next_i8() % 32)
            .collect();
        let mcts_biases: Vec<i32> = (0..Self::MCTS_BIAS_COUNT)
            .map(|_| rng.next_i32() % 256)
            .collect();

        Self {
            ft_weights,
            ft_biases,
            hidden_weights,
            hidden_biases,
            brs_weights,
            brs_bias,
            mcts_weights,
            mcts_biases,
        }
    }

    /// Total parameter count.
    pub fn param_count(&self) -> usize {
        self.ft_weights.len()
            + self.ft_biases.len()
            + self.hidden_weights.len()
            + self.hidden_biases.len()
            + self.brs_weights.len()
            + 1 // brs_bias
            + self.mcts_weights.len()
            + self.mcts_biases.len()
    }

    /// Save weights to .onnue binary format.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + Self::body_size());

        // Header (48 bytes)
        buf.extend_from_slice(&ONNUE_MAGIC);
        buf.extend_from_slice(&ONNUE_VERSION.to_le_bytes());
        buf.extend_from_slice(&architecture_hash());
        buf.extend_from_slice(&(FEATURES_PER_PERSPECTIVE as u32).to_le_bytes());
        buf.extend_from_slice(&(FT_OUT as u32).to_le_bytes());

        // Body: FT weights (i16)
        for &w in &self.ft_weights {
            buf.extend_from_slice(&w.to_le_bytes());
        }
        // FT biases (i16)
        for &b in &self.ft_biases {
            buf.extend_from_slice(&b.to_le_bytes());
        }
        // Hidden weights (i8)
        for &w in &self.hidden_weights {
            buf.push(w as u8);
        }
        // Hidden biases (i32)
        for &b in &self.hidden_biases {
            buf.extend_from_slice(&b.to_le_bytes());
        }
        // BRS weights (i8)
        for &w in &self.brs_weights {
            buf.push(w as u8);
        }
        // BRS bias (i32)
        buf.extend_from_slice(&self.brs_bias.to_le_bytes());
        // MCTS weights (i8)
        for &w in &self.mcts_weights {
            buf.push(w as u8);
        }
        // MCTS biases (i32)
        for &b in &self.mcts_biases {
            buf.extend_from_slice(&b.to_le_bytes());
        }

        // Footer: CRC32 of header + body
        let checksum = crc32(&buf);
        buf.extend_from_slice(&checksum.to_le_bytes());

        let mut file = std::fs::File::create(path)?;
        file.write_all(&buf)?;
        Ok(())
    }

    /// Load weights from .onnue binary format.
    pub fn load(path: &Path) -> Result<Self, NnueLoadError> {
        let data = std::fs::read(path)?;

        if data.len() < HEADER_SIZE + 4 {
            return Err(NnueLoadError::UnexpectedEof);
        }

        // Verify magic
        if data[0..4] != ONNUE_MAGIC {
            return Err(NnueLoadError::InvalidMagic);
        }

        // Verify version
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if version != ONNUE_VERSION {
            return Err(NnueLoadError::UnsupportedVersion(version));
        }

        // Verify architecture hash
        let stored_hash = &data[8..40];
        let expected_hash = architecture_hash();
        if stored_hash != expected_hash {
            return Err(NnueLoadError::ArchitectureMismatch);
        }

        // Verify CRC32
        let payload_end = data.len() - 4;
        let stored_crc = u32::from_le_bytes([
            data[payload_end],
            data[payload_end + 1],
            data[payload_end + 2],
            data[payload_end + 3],
        ]);
        let computed_crc = crc32(&data[..payload_end]);
        if stored_crc != computed_crc {
            return Err(NnueLoadError::ChecksumMismatch);
        }

        // Parse body
        let expected_size = HEADER_SIZE + Self::body_size() + 4;
        if data.len() != expected_size {
            return Err(NnueLoadError::UnexpectedEof);
        }

        let mut offset = HEADER_SIZE;

        let ft_weights = read_i16_vec(&data, &mut offset, Self::FT_WEIGHT_COUNT);
        let ft_biases = read_i16_vec(&data, &mut offset, Self::FT_BIAS_COUNT);
        let hidden_weights = read_i8_vec(&data, &mut offset, Self::HIDDEN_WEIGHT_COUNT);
        let hidden_biases = read_i32_vec(&data, &mut offset, Self::HIDDEN_BIAS_COUNT);
        let brs_weights = read_i8_vec(&data, &mut offset, Self::BRS_WEIGHT_COUNT);
        let brs_bias = read_i32(&data, &mut offset);
        let mcts_weights = read_i8_vec(&data, &mut offset, Self::MCTS_WEIGHT_COUNT);
        let mcts_biases = read_i32_vec(&data, &mut offset, Self::MCTS_BIAS_COUNT);

        Ok(Self {
            ft_weights,
            ft_biases,
            hidden_weights,
            hidden_biases,
            brs_weights,
            brs_bias,
            mcts_weights,
            mcts_biases,
        })
    }

    /// Compute the total body size in bytes (excluding header and footer).
    fn body_size() -> usize {
        Self::FT_WEIGHT_COUNT * 2      // i16
        + Self::FT_BIAS_COUNT * 2      // i16
        + Self::HIDDEN_WEIGHT_COUNT    // i8
        + Self::HIDDEN_BIAS_COUNT * 4  // i32
        + Self::BRS_WEIGHT_COUNT       // i8
        + 4                            // brs_bias i32
        + Self::MCTS_WEIGHT_COUNT      // i8
        + Self::MCTS_BIAS_COUNT * 4    // i32
    }
}

// ---------------------------------------------------------------------------
// Architecture hash
// ---------------------------------------------------------------------------

/// Compute a deterministic 32-byte architecture hash.
/// Changes when network dimensions change, preventing weight mismatch.
fn architecture_hash() -> [u8; 32] {
    // Hash the architecture descriptor string using repeated FNV-1a.
    let descriptor = format!(
        "HalfKP4-{}-{}-{}-{}-{}",
        FEATURES_PER_PERSPECTIVE, FT_OUT, HIDDEN_SIZE, BRS_OUTPUT, MCTS_OUTPUT
    );
    let bytes = descriptor.as_bytes();

    let mut hash = [0u8; 32];
    // Fill 32 bytes by running FNV-1a with different seeds per 8-byte chunk.
    for chunk_idx in 0..4 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325_u64.wrapping_add(chunk_idx as u64);
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let h_bytes = h.to_le_bytes();
        hash[chunk_idx * 8..chunk_idx * 8 + 8].copy_from_slice(&h_bytes);
    }
    hash
}

// ---------------------------------------------------------------------------
// CRC32 (IEEE 802.3)
// ---------------------------------------------------------------------------

/// CRC32 lookup table (IEEE polynomial 0xEDB88320).
static CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0u32;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};

/// Compute CRC32 checksum over a byte slice.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        let idx = ((crc ^ byte as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ CRC32_TABLE[idx];
    }
    !crc
}

// ---------------------------------------------------------------------------
// Binary read helpers
// ---------------------------------------------------------------------------

fn read_i16_vec(data: &[u8], offset: &mut usize, count: usize) -> Vec<i16> {
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        let val = i16::from_le_bytes([data[*offset], data[*offset + 1]]);
        *offset += 2;
        v.push(val);
    }
    v
}

fn read_i8_vec(data: &[u8], offset: &mut usize, count: usize) -> Vec<i8> {
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        v.push(data[*offset] as i8);
        *offset += 1;
    }
    v
}

fn read_i32_vec(data: &[u8], offset: &mut usize, count: usize) -> Vec<i32> {
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        let val = i32::from_le_bytes([
            data[*offset],
            data[*offset + 1],
            data[*offset + 2],
            data[*offset + 3],
        ]);
        *offset += 4;
        v.push(val);
    }
    v
}

fn read_i32(data: &[u8], offset: &mut usize) -> i32 {
    let val = i32::from_le_bytes([
        data[*offset],
        data[*offset + 1],
        data[*offset + 2],
        data[*offset + 3],
    ]);
    *offset += 4;
    val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_deterministic() {
        let w1 = NnueWeights::random(42);
        let w2 = NnueWeights::random(42);
        assert_eq!(w1.ft_weights, w2.ft_weights);
        assert_eq!(w1.ft_biases, w2.ft_biases);
        assert_eq!(w1.hidden_weights, w2.hidden_weights);
        assert_eq!(w1.hidden_biases, w2.hidden_biases);
        assert_eq!(w1.brs_weights, w2.brs_weights);
        assert_eq!(w1.brs_bias, w2.brs_bias);
        assert_eq!(w1.mcts_weights, w2.mcts_weights);
        assert_eq!(w1.mcts_biases, w2.mcts_biases);
    }

    #[test]
    fn test_random_different_seeds() {
        let w1 = NnueWeights::random(42);
        let w2 = NnueWeights::random(43);
        assert_ne!(w1.ft_weights[..100], w2.ft_weights[..100]);
    }

    #[test]
    fn test_param_count() {
        let w = NnueWeights::random(0);
        // 4*4480*256 + 4*256 + 4*256*32 + 32 + 32 + 1 + 32*4 + 4
        let expected = NnueWeights::FT_WEIGHT_COUNT
            + NnueWeights::FT_BIAS_COUNT
            + NnueWeights::HIDDEN_WEIGHT_COUNT
            + NnueWeights::HIDDEN_BIAS_COUNT
            + NnueWeights::BRS_WEIGHT_COUNT
            + 1
            + NnueWeights::MCTS_WEIGHT_COUNT
            + NnueWeights::MCTS_BIAS_COUNT;
        assert_eq!(w.param_count(), expected);
    }

    #[test]
    fn test_architecture_hash_stable() {
        let h1 = architecture_hash();
        let h2 = architecture_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_crc32_known() {
        // CRC32 of empty data should be 0
        assert_eq!(crc32(b""), 0);
        // CRC32 of "123456789" = 0xCBF43926 (IEEE standard test vector)
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
