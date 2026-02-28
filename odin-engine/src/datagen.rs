// datagen.rs — NNUE Training Data Feature Extraction (Stage 15)
//
// Reads JSONL training positions (from match.mjs datagen mode),
// replays each position from the move list, extracts HalfKP-4 features
// for all 4 perspectives, and writes binary training samples (.bin).
//
// Usage: odin-engine --datagen --input <file.jsonl> --output <file.bin>

use crate::board::{Board, Player};
use crate::eval::nnue::features::active_features;
use crate::gamestate::{GameMode, GameState, PlayerStatus};

use serde::Deserialize;
use std::io::Write;

/// Size of one binary training sample in bytes.
pub const SAMPLE_SIZE: usize = 556;

/// Maximum active features per perspective (padded to this size in binary output).
const MAX_FEATURES: usize = 64;

/// Bytes per perspective in the binary format: 1 (count) + 64 * 2 (u16 indices) = 129.
const PERSPECTIVE_BYTES: usize = 1 + MAX_FEATURES * 2;

// ---------------------------------------------------------------------------
// JSONL input record
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct TrainingSample {
    pub position_moves: String,
    pub ply: u32,
    pub side_to_move: String,
    pub score_cp: Option<f64>,
    pub v1: Option<f64>,
    pub v2: Option<f64>,
    pub v3: Option<f64>,
    pub v4: Option<f64>,
    pub depth: Option<u32>,
    pub game_id: Option<u32>,
    pub game_result: [f64; 4],
}

// ---------------------------------------------------------------------------
// Move replay
// ---------------------------------------------------------------------------

/// Replay a sequence of algebraic move strings from startpos.
///
/// Returns the resulting GameState, or an error if any move is illegal.
pub fn replay_moves(move_strs: &[&str]) -> Result<GameState, String> {
    let mut gs = GameState::new(Board::starting_position(), GameMode::FreeForAll, false);
    for (i, move_str) in move_strs.iter().enumerate() {
        if gs.is_game_over() {
            return Err(format!(
                "game over at move {} before applying '{}'",
                i, move_str
            ));
        }
        let legal = gs.legal_moves();
        let mv = legal
            .into_iter()
            .find(|m| m.to_algebraic() == *move_str)
            .ok_or_else(|| {
                format!(
                    "illegal or unrecognized move '{}' at index {}",
                    move_str, i
                )
            })?;
        gs.apply_move(mv);
    }
    Ok(gs)
}

// ---------------------------------------------------------------------------
// Binary sample extraction
// ---------------------------------------------------------------------------

/// Extract a 556-byte binary training sample from a replayed position.
///
/// Binary layout:
///   [0..516]   4 feature vectors (4 perspectives × 129 bytes each)
///              Per perspective: count:u8 + indices:[u16; 64] (LE, padded)
///   [516..518] brs_target: i16 (LE, centipawns clamped to i16 range)
///   [518..534] mcts_targets: [f32; 4] (LE, v1..v4)
///   [534..550] game_result: [f32; 4] (LE)
///   [550..552] ply: u16 (LE)
///   [552..556] game_id: u32 (LE)
pub fn extract_sample(gs: &GameState, sample: &TrainingSample) -> [u8; SAMPLE_SIZE] {
    let mut buf = [0u8; SAMPLE_SIZE];
    let board = gs.board();

    // 4 perspectives in absolute order: Red(0), Blue(1), Yellow(2), Green(3)
    for p in 0..4 {
        let perspective = Player::from_index(p).unwrap();
        let (features, count) = active_features(board, perspective);
        let base = p * PERSPECTIVE_BYTES;
        buf[base] = count as u8;
        for (i, &idx) in features.iter().enumerate().take(count) {
            let offset = base + 1 + i * 2;
            buf[offset] = (idx & 0xFF) as u8;
            buf[offset + 1] = (idx >> 8) as u8;
        }
    }

    // BRS target (i16, clamped)
    let brs_cp = sample.score_cp.unwrap_or(0.0) as i32;
    let brs_clamped = brs_cp.clamp(-30000, 30000) as i16;
    buf[516..518].copy_from_slice(&brs_clamped.to_le_bytes());

    // MCTS targets (4 × f32)
    let mcts = [
        sample.v1.unwrap_or(0.5) as f32,
        sample.v2.unwrap_or(0.5) as f32,
        sample.v3.unwrap_or(0.5) as f32,
        sample.v4.unwrap_or(0.5) as f32,
    ];
    for (i, &v) in mcts.iter().enumerate() {
        buf[518 + i * 4..522 + i * 4].copy_from_slice(&v.to_le_bytes());
    }

    // Game result (4 × f32)
    for (i, &v) in sample.game_result.iter().enumerate() {
        buf[534 + i * 4..538 + i * 4].copy_from_slice(&(v as f32).to_le_bytes());
    }

    // Metadata
    let ply = sample.ply as u16;
    buf[550..552].copy_from_slice(&ply.to_le_bytes());
    let game_id = sample.game_id.unwrap_or(0);
    buf[552..556].copy_from_slice(&game_id.to_le_bytes());

    buf
}

// ---------------------------------------------------------------------------
// CLI entry point
// ---------------------------------------------------------------------------

fn find_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// Parse a player name string to a Player enum.
fn parse_player(name: &str) -> Result<Player, String> {
    match name {
        "Red" => Ok(Player::Red),
        "Blue" => Ok(Player::Blue),
        "Yellow" => Ok(Player::Yellow),
        "Green" => Ok(Player::Green),
        _ => Err(format!("unknown player: '{}'", name)),
    }
}

/// Run the datagen pipeline: read JSONL → replay positions → extract features → write .bin.
pub fn run(args: &[String]) -> Result<(), String> {
    let input_path = find_arg(args, "--input").ok_or("--input <file.jsonl> required")?;
    let output_path = find_arg(args, "--output").ok_or("--output <file.bin> required")?;

    let input = std::fs::read_to_string(&input_path)
        .map_err(|e| format!("cannot read '{}': {}", input_path, e))?;

    let mut out = std::fs::File::create(&output_path)
        .map_err(|e| format!("cannot create '{}': {}", output_path, e))?;

    let mut total = 0u64;
    let mut skipped = 0u64;

    for (line_num, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let sample: TrainingSample = serde_json::from_str(line).map_err(|e| {
            format!("line {}: parse error: {}", line_num + 1, e)
        })?;

        // Skip positions with null v1-v4 (forced moves, no MCTS output)
        if sample.v1.is_none()
            || sample.v2.is_none()
            || sample.v3.is_none()
            || sample.v4.is_none()
        {
            skipped += 1;
            continue;
        }

        // Replay moves to reconstruct position
        let move_strs: Vec<&str> = if sample.position_moves.is_empty() {
            vec![]
        } else {
            sample.position_moves.split_whitespace().collect()
        };

        let gs = match replay_moves(&move_strs) {
            Ok(gs) => gs,
            Err(e) => {
                eprintln!("line {}: skip ({})", line_num + 1, e);
                skipped += 1;
                continue;
            }
        };

        // Skip if side_to_move is eliminated
        let stm = parse_player(&sample.side_to_move)?;
        if gs.player_status(stm) == PlayerStatus::Eliminated {
            skipped += 1;
            continue;
        }

        let buf = extract_sample(&gs, &sample);
        out.write_all(&buf)
            .map_err(|e| format!("write error: {}", e))?;
        total += 1;
    }

    eprintln!(
        "datagen: {} samples written, {} skipped",
        total, skipped
    );
    eprintln!(
        "datagen: output = {} ({} bytes = {} * {})",
        output_path,
        total * SAMPLE_SIZE as u64,
        total,
        SAMPLE_SIZE
    );

    Ok(())
}
