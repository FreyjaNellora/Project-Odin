// swarm_eval.rs — Tactical leaf assessment replacing quiescence search.
//
// Single-pass tactical assessment at leaf nodes:
// 1. Hanging piece detection (attacked, undefended)
// 2. Chain walk: static capture sequence resolution
// 3. Commitment check: material overextension on contested squares
//
// Returns a centipawn adjustment from root_player's perspective.

use crate::board::{Board, PieceType, Player, Square};
use crate::eval::values::PIECE_EVAL_VALUES;
use crate::gamestate::{GameState, PlayerStatus};
use crate::movegen::is_square_attacked_by;

/// Tactical adjustment at leaf nodes instead of qsearch.
/// Returns centipawn adjustment from root_player's perspective.
pub fn swarm_leaf_eval(gs: &GameState, root_player: Player) -> i16 {
    let board = gs.board();
    let mut adjustment: i16 = 0;

    // ── 1. Hanging piece detection ──
    let mut root_hanging_penalty: i16 = 0;
    let mut opp_hanging_bonus: i16 = 0;

    for player in Player::ALL.iter().copied() {
        if gs.player_status(player) != PlayerStatus::Active {
            continue;
        }

        for &(piece_type, sq) in board.piece_list(player) {
            if piece_type == PieceType::King {
                continue;
            }

            let piece_val = PIECE_EVAL_VALUES[piece_type.index()];
            let mut is_attacked = false;

            for opp in Player::ALL.iter().copied() {
                if opp == player { continue; }
                if gs.player_status(opp) != PlayerStatus::Active { continue; }
                if is_square_attacked_by(sq, opp, board) {
                    is_attacked = true;
                    break;
                }
            }

            if !is_attacked {
                continue;
            }

            // Defended by own pieces?
            let is_defended = is_square_attacked_by(sq, player, board);

            if !is_defended {
                // Hanging — full value at risk
                if player == root_player {
                    root_hanging_penalty += piece_val;
                } else {
                    opp_hanging_bonus += piece_val / 3;
                }
            }
        }
    }

    adjustment -= root_hanging_penalty;
    adjustment += opp_hanging_bonus;

    // ── 2. Chain walk (simplified) ──
    // For root_player's attacked pieces, check if trade is losing
    for &(piece_type, sq) in board.piece_list(root_player) {
        if piece_type == PieceType::King { continue; }

        let piece_val = PIECE_EVAL_VALUES[piece_type.index()];
        let mut cheapest_attacker_val = i16::MAX;

        for opp in Player::ALL.iter().copied() {
            if opp == root_player { continue; }
            if gs.player_status(opp) != PlayerStatus::Active { continue; }
            // Check if this opponent attacks the square
            if !is_square_attacked_by(sq, opp, board) { continue; }
            // Find their cheapest attacker by scanning their pieces
            for &(opp_pt, opp_sq) in board.piece_list(opp) {
                if opp_pt == PieceType::King { continue; }
                let opp_val = PIECE_EVAL_VALUES[opp_pt.index()];
                if opp_val < cheapest_attacker_val {
                    // Verify this specific piece actually attacks our square
                    // (is_square_attacked_by already confirmed SOME piece does)
                    // For speed, use the cheapest piece of attacking player as estimate
                    cheapest_attacker_val = opp_val;
                }
            }
        }

        if cheapest_attacker_val < i16::MAX && cheapest_attacker_val < piece_val {
            // Attacked by cheaper piece — potential losing trade
            let net_loss = piece_val - cheapest_attacker_val;
            let is_defended = is_square_attacked_by(sq, root_player, board);
            if is_defended {
                adjustment -= net_loss / 4; // Discounted — defended, may not trade
            } else {
                adjustment -= net_loss / 2; // More serious — hanging to cheaper piece
            }
        }
    }

    // ── 3. Commitment / overextension ──
    let mut total_material: i16 = 0;
    let mut contested_material: i16 = 0;

    for &(piece_type, sq) in board.piece_list(root_player) {
        let val = PIECE_EVAL_VALUES[piece_type.index()];
        total_material += val;

        for opp in Player::ALL.iter().copied() {
            if opp == root_player { continue; }
            if gs.player_status(opp) != PlayerStatus::Active { continue; }
            if is_square_attacked_by(sq, opp, board) {
                contested_material += val;
                break;
            }
        }
    }

    if total_material > 0 {
        let ratio = (contested_material as i32 * 100) / total_material as i32;
        if ratio > 40 {
            adjustment -= ((ratio - 40) / 5) as i16;
        }
    }

    adjustment
}
