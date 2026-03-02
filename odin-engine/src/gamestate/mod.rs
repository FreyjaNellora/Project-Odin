// Game state and rules — Stage 3
//
// GameState wraps Board + MoveGen with full game lifecycle:
// turns, check/checkmate/stalemate, elimination, scoring,
// DKW, terrain conversion, and game-over detection.

pub mod rules;
pub mod scoring;

use std::sync::Arc;

use crate::board::{Board, PieceStatus, Player};
use crate::movegen::{generate_legal, make_move, Move};

use rules::{
    check_claim_win, convert_to_dead, convert_to_terrain, determine_status_at_turn,
    generate_dkw_move, is_draw_by_fifty_moves, is_draw_by_repetition, kings_checked_by_move,
    remove_king, TurnDetermination,
};
use scoring::{
    capture_points, check_bonus_points, CHECKMATE_POINTS, DRAW_POINTS, STALEMATE_POINTS,
};

/// Status of a player in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStatus {
    /// Player is actively playing.
    Active,
    /// Player resigned/timed out, king wanders randomly.
    DeadKingWalking,
    /// Player is fully eliminated (no pieces or king stuck).
    Eliminated,
}

/// Reason a player was eliminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EliminationReason {
    Checkmate,
    Stalemate,
    Resignation,
    Timeout,
    DkwKingCaptured,
    DkwKingStuck,
}

/// Game mode — determines win conditions and game-over rules.
///
/// `FreeForAll`: Most points wins. Claim-win (21+ point lead with 2 active) enabled.
/// `LastKingStanding`: Last player alive wins. Points tracked for display but
/// do not affect game-over conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    FreeForAll,
    LastKingStanding,
}

/// Result of applying a move.
#[derive(Debug, Clone)]
pub struct MoveResult {
    pub mv: Move,
    pub points_scored: i32,
    pub eliminations: Vec<(Player, EliminationReason)>,
    pub dkw_moves: Vec<(Player, Move)>,
    pub game_ended: bool,
}

/// The full game state.
#[derive(Clone)]
pub struct GameState {
    board: Board,
    player_status: [PlayerStatus; 4],
    scores: [i32; 4],
    current_player: Player,
    elimination_order: Vec<Player>,
    position_history: Arc<Vec<u64>>,
    game_mode: GameMode,
    terrain_mode: bool,
    game_over: bool,
    winner: Option<Player>,
    rng_seed: u64,
}

impl GameState {
    /// Create a new game state from a board.
    pub fn new(board: Board, game_mode: GameMode, terrain_mode: bool) -> Self {
        let current_player = board.side_to_move();
        Self {
            board,
            player_status: [PlayerStatus::Active; 4],
            scores: [0; 4],
            current_player,
            elimination_order: Vec::new(),
            position_history: Arc::new(Vec::new()),
            game_mode,
            terrain_mode,
            game_over: false,
            winner: None,
            rng_seed: 0xDEADBEEF_CAFEBABE,
        }
    }

    /// Create a standard FFA game from the starting position.
    pub fn new_standard_ffa() -> Self {
        Self::new(Board::starting_position(), GameMode::FreeForAll, false)
    }

    /// Create a standard FFA game with terrain mode enabled.
    pub fn new_standard_ffa_terrain() -> Self {
        Self::new(Board::starting_position(), GameMode::FreeForAll, true)
    }

    /// Create a standard Last King Standing game from the starting position.
    pub fn new_standard_lks() -> Self {
        Self::new(
            Board::starting_position(),
            GameMode::LastKingStanding,
            false,
        )
    }

    /// Create a standard Last King Standing game with terrain mode enabled.
    pub fn new_standard_lks_terrain() -> Self {
        Self::new(Board::starting_position(), GameMode::LastKingStanding, true)
    }

    // --- Accessors ---

    pub fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn board_mut(&mut self) -> &mut Board {
        &mut self.board
    }

    pub fn player_status(&self, player: Player) -> PlayerStatus {
        self.player_status[player.index()]
    }

    pub fn score(&self, player: Player) -> i32 {
        self.scores[player.index()]
    }

    pub fn scores(&self) -> [i32; 4] {
        self.scores
    }

    pub fn current_player(&self) -> Player {
        self.current_player
    }

    pub fn is_game_over(&self) -> bool {
        self.game_over
    }

    pub fn winner(&self) -> Option<Player> {
        self.winner
    }

    pub fn elimination_order(&self) -> &[Player] {
        &self.elimination_order
    }

    pub fn active_player_count(&self) -> usize {
        self.player_status
            .iter()
            .filter(|&&s| s == PlayerStatus::Active)
            .count()
    }

    pub fn is_player_active(&self, player: Player) -> bool {
        self.player_status[player.index()] == PlayerStatus::Active
    }

    pub fn terrain_mode(&self) -> bool {
        self.terrain_mode
    }

    /// Generate legal moves for the current player.
    pub fn legal_moves(&mut self) -> Vec<Move> {
        self.board.set_side_to_move(self.current_player);
        generate_legal(&mut self.board)
    }

    /// Check if position is a draw by repetition.
    pub fn is_draw_by_repetition(&self) -> bool {
        is_draw_by_repetition(&self.position_history, self.board.zobrist())
    }

    /// Zobrist hash history of all positions played so far.
    /// Used by the search to detect in-search repetitions against game history.
    pub fn position_history(&self) -> &[u64] {
        &self.position_history
    }

    /// Arc-wrapped position history for zero-cost sharing with search.
    pub fn position_history_arc(&self) -> &Arc<Vec<u64>> {
        &self.position_history
    }

    /// Check if position is a draw by 50-move rule.
    pub fn is_draw_by_fifty_moves(&self) -> bool {
        is_draw_by_fifty_moves(self.board.halfmove_clock())
    }

    // --- Game loop ---

    /// Apply a move to the game state. This is the central method.
    ///
    /// Flow:
    /// 1. Make the move on the board
    /// 2. Score capture points
    /// 3. Score check bonus
    /// 4. Record position history
    /// 5. Advance to next active player
    /// 6. Process DKW instant moves (must run before checkmate detection)
    /// 7. Check checkmate/stalemate for next player (chain eliminations)
    /// 8. Check game-over conditions
    pub fn apply_move(&mut self, mv: Move) -> MoveResult {
        assert!(!self.game_over, "game is already over");
        assert_eq!(
            self.board.side_to_move(),
            self.current_player,
            "board side_to_move must match current_player"
        );

        let mover = self.current_player;
        let mut result = MoveResult {
            mv,
            points_scored: 0,
            eliminations: Vec::new(),
            dkw_moves: Vec::new(),
            game_ended: false,
        };

        // 1. Capture info BEFORE make_move (need the piece that's about to be removed)
        //    For en passant, the captured piece is on a different square — MoveUndo handles this.
        let undo = make_move(&mut self.board, mv);

        // 2. Score capture points
        if let Some(captured) = undo.captured_piece {
            let pts = capture_points(captured.piece_type, captured.status);
            self.scores[mover.index()] += pts;
            result.points_scored += pts;

            // Check if a DKW king was captured
            if captured.piece_type == crate::board::PieceType::King
                && captured.status == PieceStatus::Dead
            {
                let dead_player = captured.owner;
                self.eliminate_player(dead_player, EliminationReason::DkwKingCaptured, &mut result);
            }
        }

        // 3. Score check bonus
        let checked_kings = kings_checked_by_move(&self.board, mover, &self.player_status);
        let bonus = check_bonus_points(checked_kings.len());
        if bonus > 0 {
            self.scores[mover.index()] += bonus;
            result.points_scored += bonus;
        }

        // 4. Record position in history
        Arc::make_mut(&mut self.position_history).push(self.board.zobrist());

        // 5. Advance to next active player
        if let Some(next) = self.next_active_player(mover) {
            self.current_player = next;
            self.board.set_side_to_move(next);
        } else {
            // No active players left
            self.end_game(&mut result);
        }

        // 6. Process DKW instant moves first — board must reflect DKW positions
        //    before checkmate/stalemate detection runs (a DKW piece that wanders
        //    away could be the only piece the next player could capture).
        if !self.game_over {
            self.process_dkw_moves(&mut result);
        }

        // 7. Check checkmate/stalemate for the next player (now sees post-DKW board)
        if !self.game_over {
            self.check_elimination_chain(&mut result);
        }

        // 8. Check game-over conditions
        if !self.game_over {
            self.check_game_over(&mut result);
        }

        result.game_ended = self.game_over;
        result
    }

    /// Resign a player. Triggers DKW or terrain conversion.
    pub fn resign_player(&mut self, player: Player) -> MoveResult {
        let mut result = MoveResult {
            mv: Move::new(0, 0, crate::board::PieceType::Pawn), // dummy
            points_scored: 0,
            eliminations: Vec::new(),
            dkw_moves: Vec::new(),
            game_ended: false,
        };

        if self.terrain_mode {
            self.eliminate_player(player, EliminationReason::Resignation, &mut result);
        } else {
            // DKW mode: pieces go dead, king wanders
            self.start_dkw(player, EliminationReason::Resignation);
        }

        // If the resigned player was current, advance
        if self.current_player == player {
            if let Some(next) = self.next_active_player(player) {
                self.current_player = next;
                self.board.set_side_to_move(next);
                self.check_elimination_chain(&mut result);
            } else {
                self.end_game(&mut result);
            }
        }

        if !self.game_over {
            self.check_game_over(&mut result);
        }

        result.game_ended = self.game_over;
        result
    }

    /// Timeout a player. Same behavior as resign.
    pub fn timeout_player(&mut self, player: Player) -> MoveResult {
        let mut result = MoveResult {
            mv: Move::new(0, 0, crate::board::PieceType::Pawn),
            points_scored: 0,
            eliminations: Vec::new(),
            dkw_moves: Vec::new(),
            game_ended: false,
        };

        if self.terrain_mode {
            self.eliminate_player(player, EliminationReason::Timeout, &mut result);
        } else {
            self.start_dkw(player, EliminationReason::Timeout);
        }

        if self.current_player == player {
            if let Some(next) = self.next_active_player(player) {
                self.current_player = next;
                self.board.set_side_to_move(next);
                self.check_elimination_chain(&mut result);
            } else {
                self.end_game(&mut result);
            }
        }

        if !self.game_over {
            self.check_game_over(&mut result);
        }

        result.game_ended = self.game_over;
        result
    }

    /// Called from the protocol when the current player has no legal moves.
    ///
    /// Runs checkmate/stalemate detection, DKW moves, and game-over check without
    /// requiring a move to be made. Returns a MoveResult describing all state changes.
    pub fn handle_no_legal_moves(&mut self) -> MoveResult {
        let mut result = MoveResult {
            mv: Move::new(0, 0, crate::board::PieceType::Pawn), // dummy — no move made
            points_scored: 0,
            eliminations: Vec::new(),
            dkw_moves: Vec::new(),
            game_ended: false,
        };
        if !self.game_over {
            self.check_elimination_chain(&mut result);
        }
        if !self.game_over {
            self.process_dkw_moves(&mut result);
        }
        if !self.game_over {
            self.check_game_over(&mut result);
        }
        result.game_ended = self.game_over;
        result
    }

    // --- Internal methods ---

    /// Find the next active player after `after`, skipping eliminated/DKW players.
    /// Returns None if no active player exists.
    fn next_active_player(&self, after: Player) -> Option<Player> {
        let mut candidate = after.next();
        for _ in 0..4 {
            if self.player_status[candidate.index()] == PlayerStatus::Active {
                return Some(candidate);
            }
            candidate = candidate.next();
        }
        None
    }

    /// Check for checkmate/stalemate chain starting from current_player.
    /// When one player is eliminated, the next in line might also be mated.
    fn check_elimination_chain(&mut self, result: &mut MoveResult) {
        loop {
            if self.game_over {
                break;
            }

            let player = self.current_player;
            if self.player_status[player.index()] != PlayerStatus::Active {
                break;
            }

            match determine_status_at_turn(&mut self.board, player) {
                TurnDetermination::HasMoves => break,
                TurnDetermination::Checkmate => {
                    // Award checkmate points to the previous active player
                    // (the one who delivered the mating position)
                    if let Some(prev) = self.prev_active_player(player) {
                        self.scores[prev.index()] += CHECKMATE_POINTS;
                    }
                    self.eliminate_player(player, EliminationReason::Checkmate, result);

                    // Advance to next active player and continue chain
                    if let Some(next) = self.next_active_player(player) {
                        self.current_player = next;
                        self.board.set_side_to_move(next);
                    } else {
                        self.end_game(result);
                        break;
                    }
                }
                TurnDetermination::Stalemate => {
                    // Stalemated player gets 20 points
                    self.scores[player.index()] += STALEMATE_POINTS;
                    self.eliminate_player(player, EliminationReason::Stalemate, result);

                    if let Some(next) = self.next_active_player(player) {
                        self.current_player = next;
                        self.board.set_side_to_move(next);
                    } else {
                        self.end_game(result);
                        break;
                    }
                }
            }
        }
    }

    /// Find the previous active player (the one whose move led to this position).
    fn prev_active_player(&self, before: Player) -> Option<Player> {
        let mut candidate = before.prev();
        for _ in 0..4 {
            if self.player_status[candidate.index()] == PlayerStatus::Active
                || self.player_status[candidate.index()] == PlayerStatus::DeadKingWalking
            {
                return Some(candidate);
            }
            candidate = candidate.prev();
        }
        None
    }

    /// Eliminate a player.
    fn eliminate_player(
        &mut self,
        player: Player,
        reason: EliminationReason,
        result: &mut MoveResult,
    ) {
        self.player_status[player.index()] = PlayerStatus::Eliminated;
        self.elimination_order.push(player);
        result.eliminations.push((player, reason));

        // Convert pieces based on mode
        if self.terrain_mode {
            convert_to_terrain(&mut self.board, player);
        } else {
            // For checkmate/stalemate: just remove the king, pieces stay as-is
            // (they're already dead if DKW, or alive if direct checkmate)
            // For direct checkmate of an active player, convert to dead then remove king
            if self.player_status[player.index()] == PlayerStatus::Eliminated {
                // If this was a DKW player, just remove king
                // If this was an Active player being eliminated, convert pieces to dead
                convert_to_dead(&mut self.board, player);
                remove_king(&mut self.board, player);
            }
        }
    }

    /// Start DKW for a player (resign/timeout in non-terrain mode).
    fn start_dkw(&mut self, player: Player, _reason: EliminationReason) {
        self.player_status[player.index()] = PlayerStatus::DeadKingWalking;
        convert_to_dead(&mut self.board, player);
    }

    /// Process DKW instant moves for all DKW players.
    fn process_dkw_moves(&mut self, result: &mut MoveResult) {
        // Process each DKW player in turn order starting from current_player
        let mut candidate = self.current_player.next();
        for _ in 0..4 {
            if candidate == self.current_player {
                break;
            }
            if self.player_status[candidate.index()] == PlayerStatus::DeadKingWalking {
                if let Some(dkw_mv) =
                    generate_dkw_move(&mut self.board, candidate, &mut self.rng_seed)
                {
                    // Save side_to_move, make DKW move, restore
                    let saved_stm = self.board.side_to_move();
                    self.board.set_side_to_move(candidate);
                    let _undo = make_move(&mut self.board, dkw_mv);
                    // Note: we don't unmake DKW moves. They're permanent.
                    self.board.set_side_to_move(saved_stm);
                    result.dkw_moves.push((candidate, dkw_mv));
                } else {
                    // DKW king is stuck — eliminate
                    self.eliminate_player(candidate, EliminationReason::DkwKingStuck, result);
                }
            }
            candidate = candidate.next();
        }
    }

    /// Check if the game should end.
    fn check_game_over(&mut self, result: &mut MoveResult) {
        // Count active players
        let active_count = self.active_player_count();

        // Check draw conditions
        if self.is_draw_by_repetition() || self.is_draw_by_fifty_moves() {
            // Award draw points to all active players
            for &player in &Player::ALL {
                if self.player_status[player.index()] == PlayerStatus::Active {
                    self.scores[player.index()] += DRAW_POINTS;
                }
            }
            self.game_over = true;
            result.game_ended = true;
            return;
        }

        if active_count <= 1 {
            self.end_game(result);
            return;
        }

        // Check claim win (21+ lead with exactly 2 active) — FFA only.
        // In LKS, points are tracked for display but don't trigger game-over.
        if self.game_mode == GameMode::FreeForAll {
            if let Some(winner) = check_claim_win(&self.scores, &self.player_status) {
                self.game_over = true;
                self.winner = Some(winner);
                result.game_ended = true;
            }
        }
    }

    /// End the game, determine winner.
    ///
    /// FFA: winner = active player with highest score.
    /// LKS: winner = last standing player (ignores scores).
    fn end_game(&mut self, result: &mut MoveResult) {
        self.game_over = true;
        result.game_ended = true;

        match self.game_mode {
            GameMode::LastKingStanding => {
                // Last standing player wins, regardless of score.
                self.winner = Player::ALL
                    .iter()
                    .copied()
                    .find(|&p| self.player_status[p.index()] == PlayerStatus::Active);
            }
            GameMode::FreeForAll => {
                // Active player with the highest score wins.
                let mut best_player = None;
                let mut best_score = i32::MIN;
                for &player in &Player::ALL {
                    if self.player_status[player.index()] == PlayerStatus::Active
                        && self.scores[player.index()] > best_score
                    {
                        best_score = self.scores[player.index()];
                        best_player = Some(player);
                    }
                }
                self.winner = best_player;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{square_from, Board, Piece, PieceType, Player};

    #[test]
    fn test_new_standard_ffa() {
        let gs = GameState::new_standard_ffa();
        assert_eq!(gs.current_player(), Player::Red);
        assert!(!gs.is_game_over());
        assert_eq!(gs.active_player_count(), 4);
        for &p in &Player::ALL {
            assert_eq!(gs.score(p), 0);
            assert!(gs.is_player_active(p));
        }
    }

    #[test]
    fn test_next_active_player_all_active() {
        let gs = GameState::new_standard_ffa();
        assert_eq!(gs.next_active_player(Player::Red), Some(Player::Blue));
        assert_eq!(gs.next_active_player(Player::Green), Some(Player::Red));
    }

    #[test]
    fn test_next_active_player_skip_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Blue.index()] = PlayerStatus::Eliminated;
        assert_eq!(gs.next_active_player(Player::Red), Some(Player::Yellow));
    }

    #[test]
    fn test_next_active_player_skip_two_eliminated() {
        let mut gs = GameState::new_standard_ffa();
        gs.player_status[Player::Blue.index()] = PlayerStatus::Eliminated;
        gs.player_status[Player::Yellow.index()] = PlayerStatus::Eliminated;
        assert_eq!(gs.next_active_player(Player::Red), Some(Player::Green));
    }

    #[test]
    fn test_next_active_player_none_active() {
        let mut gs = GameState::new_standard_ffa();
        for i in 0..4 {
            gs.player_status[i] = PlayerStatus::Eliminated;
        }
        assert_eq!(gs.next_active_player(Player::Red), None);
    }

    #[test]
    fn test_apply_move_basic() {
        let mut gs = GameState::new_standard_ffa();
        let moves = gs.legal_moves();
        assert!(!moves.is_empty());

        let mv = moves[0];
        let result = gs.apply_move(mv);
        assert_eq!(result.mv, mv);
        assert_eq!(gs.current_player(), Player::Blue);
    }

    #[test]
    fn test_apply_move_advances_turns() {
        let mut gs = GameState::new_standard_ffa();
        // Make moves for all 4 players
        for expected in &[Player::Blue, Player::Yellow, Player::Green, Player::Red] {
            let moves = gs.legal_moves();
            gs.apply_move(moves[0]);
            assert_eq!(gs.current_player(), *expected);
        }
    }

    #[test]
    fn test_scoring_on_capture() {
        let mut board = Board::empty();
        // Red pawn on e5, Blue pawn on f6 — Red can capture
        board.place_piece(
            square_from(4, 4).unwrap(),
            Piece::new(PieceType::Pawn, Player::Red),
        );
        board.place_piece(
            square_from(5, 5).unwrap(),
            Piece::new(PieceType::Pawn, Player::Blue),
        );
        // Kings
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.place_piece(
            square_from(0, 6).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.place_piece(
            square_from(6, 13).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.place_piece(
            square_from(13, 7).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Red);

        let mut gs = GameState::new(board, GameMode::FreeForAll, false);
        let moves = gs.legal_moves();

        // Find the capture move
        let capture = moves.iter().find(|m| m.is_capture()).unwrap();
        let result = gs.apply_move(*capture);
        assert_eq!(result.points_scored, 1); // Pawn = 1 point
        assert_eq!(gs.score(Player::Red), 1);
    }

    #[test]
    fn test_terrain_mode_flag() {
        let gs = GameState::new_standard_ffa_terrain();
        assert!(gs.terrain_mode());
    }

    #[test]
    fn test_handle_no_legal_moves_checkmate() {
        // Red king at h1 (7,0), Green queen at i2 (8,1) giving check along SW diagonal.
        // Blue bishop at c8 (2,7) protects the queen on the SE diagonal (c8→…→i2).
        // Blue rook at g5 (6,4) covers g1 (6,0) via file g — the only other escape square.
        //
        // Escape analysis from h1:
        //   g1 (6,0): rook at g5 covers via file g   → illegal
        //   h2 (7,1): queen at i2 covers via rank 1   → illegal
        //   g2 (6,1): queen at i2 covers via rank 1   → illegal
        //   i1 (8,0): queen at i2 covers via file i   → illegal
        //   i2 (8,1): queen is there, protected by bishop → illegal capture
        // h1 is in check (queen on SW diagonal). No escape → CHECKMATE.
        let mut board = Board::empty();
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.place_piece(
            square_from(8, 1).unwrap(),
            Piece::new(PieceType::Queen, Player::Green),
        );
        board.place_piece(
            square_from(2, 7).unwrap(),
            Piece::new(PieceType::Bishop, Player::Blue),
        );
        board.place_piece(
            square_from(6, 4).unwrap(),
            Piece::new(PieceType::Rook, Player::Blue),
        );
        // Remaining kings placed on valid non-corner squares
        board.place_piece(
            square_from(3, 13).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.place_piece(
            square_from(6, 13).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.place_piece(
            square_from(13, 6).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Red);

        let mut gs = GameState::new(board, GameMode::FreeForAll, false);
        assert!(
            gs.legal_moves().is_empty(),
            "Red should have no legal moves"
        );

        let result = gs.handle_no_legal_moves();

        assert!(
            result
                .eliminations
                .iter()
                .any(|(p, r)| *p == Player::Red && *r == EliminationReason::Checkmate),
            "Red should be eliminated by checkmate"
        );
        assert_ne!(
            gs.current_player(),
            Player::Red,
            "current player should have advanced past Red"
        );
    }

    #[test]
    fn test_handle_no_legal_moves_stalemate() {
        // Red king at h1 (7,0), not in check, no legal moves.
        //
        // Three Blue rooks box in the king:
        //   f2 (5,1): covers rank 1 → h2 (7,1), g2 (6,1), i2 (8,1)
        //   g3 (6,2): covers file g → g1 (6,0)
        //   i4 (8,3): covers file i → i1 (8,0)
        //
        // h1 (7,0) is NOT on rank 1, file g, or file i → not in check.
        // All five escape squares covered → STALEMATE.
        let mut board = Board::empty();
        board.place_piece(
            square_from(7, 0).unwrap(),
            Piece::new(PieceType::King, Player::Red),
        );
        board.place_piece(
            square_from(5, 1).unwrap(),
            Piece::new(PieceType::Rook, Player::Blue),
        );
        board.place_piece(
            square_from(6, 2).unwrap(),
            Piece::new(PieceType::Rook, Player::Blue),
        );
        board.place_piece(
            square_from(8, 3).unwrap(),
            Piece::new(PieceType::Rook, Player::Blue),
        );
        // Remaining kings on valid non-corner squares
        board.place_piece(
            square_from(3, 13).unwrap(),
            Piece::new(PieceType::King, Player::Blue),
        );
        board.place_piece(
            square_from(6, 13).unwrap(),
            Piece::new(PieceType::King, Player::Yellow),
        );
        board.place_piece(
            square_from(13, 6).unwrap(),
            Piece::new(PieceType::King, Player::Green),
        );
        board.set_side_to_move(Player::Red);

        let mut gs = GameState::new(board, GameMode::FreeForAll, false);
        assert!(
            gs.legal_moves().is_empty(),
            "Red should have no legal moves"
        );

        let result = gs.handle_no_legal_moves();

        assert!(
            result
                .eliminations
                .iter()
                .any(|(p, r)| *p == Player::Red && *r == EliminationReason::Stalemate),
            "Red should be eliminated by stalemate"
        );
    }
}
