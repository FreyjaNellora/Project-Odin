// Move encoding, MoveUndo, and make/unmake.
//
// Move is a compact u32:
//   bits 0-7:   from_square
//   bits 8-15:  to_square
//   bits 16-19: piece_type (moving piece)
//   bits 20-23: captured_piece (NONE = 7)
//   bits 24-27: promotion (NONE = 7)
//   bits 28-30: flags
//
// Flags:
//   0 = normal
//   1 = double pawn push
//   2 = en passant capture
//   3 = kingside castle
//   4 = queenside castle

use crate::board::{
    file_of, rank_of, square_from, square_name, Board, Piece, PieceType, Player, Square,
    CASTLE_BLUE_KING, CASTLE_BLUE_QUEEN, CASTLE_GREEN_KING, CASTLE_GREEN_QUEEN, CASTLE_RED_KING,
    CASTLE_RED_QUEEN, CASTLE_YELLOW_KING, CASTLE_YELLOW_QUEEN,
};

/// Sentinel value for "no piece" in captured/promotion fields.
const PIECE_NONE: u32 = 7;

/// Move flags.
pub const FLAG_NORMAL: u32 = 0;
pub const FLAG_DOUBLE_PUSH: u32 = 1;
pub const FLAG_EN_PASSANT: u32 = 2;
pub const FLAG_CASTLE_KING: u32 = 3;
pub const FLAG_CASTLE_QUEEN: u32 = 4;

/// A compact chess move encoded in 32 bits.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Move(u32);

impl Move {
    /// Create a normal (non-special) move.
    pub fn new(from: Square, to: Square, piece_type: PieceType) -> Self {
        Self::encode(from, to, piece_type, PIECE_NONE, PIECE_NONE, FLAG_NORMAL)
    }

    /// Create a capture move.
    pub fn new_capture(
        from: Square,
        to: Square,
        piece_type: PieceType,
        captured: PieceType,
    ) -> Self {
        Self::encode(
            from,
            to,
            piece_type,
            captured.index() as u32,
            PIECE_NONE,
            FLAG_NORMAL,
        )
    }

    /// Create a promotion move (may also be a capture).
    pub fn new_promotion(
        from: Square,
        to: Square,
        captured: Option<PieceType>,
        promotion: PieceType,
    ) -> Self {
        let cap = captured.map_or(PIECE_NONE, |c| c.index() as u32);
        Self::encode(
            from,
            to,
            PieceType::Pawn,
            cap,
            promotion.index() as u32,
            FLAG_NORMAL,
        )
    }

    /// Create a double pawn push.
    pub fn new_double_push(from: Square, to: Square) -> Self {
        Self::encode(
            from,
            to,
            PieceType::Pawn,
            PIECE_NONE,
            PIECE_NONE,
            FLAG_DOUBLE_PUSH,
        )
    }

    /// Create an en passant capture.
    pub fn new_en_passant(from: Square, to: Square) -> Self {
        Self::encode(
            from,
            to,
            PieceType::Pawn,
            PieceType::Pawn.index() as u32,
            PIECE_NONE,
            FLAG_EN_PASSANT,
        )
    }

    /// Create a kingside castle move.
    pub fn new_castle_king(from: Square, to: Square) -> Self {
        Self::encode(
            from,
            to,
            PieceType::King,
            PIECE_NONE,
            PIECE_NONE,
            FLAG_CASTLE_KING,
        )
    }

    /// Create a queenside castle move.
    pub fn new_castle_queen(from: Square, to: Square) -> Self {
        Self::encode(
            from,
            to,
            PieceType::King,
            PIECE_NONE,
            PIECE_NONE,
            FLAG_CASTLE_QUEEN,
        )
    }

    fn encode(
        from: Square,
        to: Square,
        piece_type: PieceType,
        captured: u32,
        promotion: u32,
        flags: u32,
    ) -> Self {
        let bits = (from as u32)
            | ((to as u32) << 8)
            | ((piece_type.index() as u32) << 16)
            | (captured << 20)
            | (promotion << 24)
            | (flags << 28);
        Self(bits)
    }

    /// Source square.
    #[inline]
    pub fn from_sq(self) -> Square {
        (self.0 & 0xFF) as Square
    }

    /// Target square.
    #[inline]
    pub fn to_sq(self) -> Square {
        ((self.0 >> 8) & 0xFF) as Square
    }

    /// Moving piece type.
    #[inline]
    pub fn piece_type(self) -> PieceType {
        let idx = ((self.0 >> 16) & 0xF) as usize;
        PieceType::ALL[idx]
    }

    /// Captured piece type, if any.
    #[inline]
    pub fn captured(self) -> Option<PieceType> {
        let idx = (self.0 >> 20) & 0xF;
        if idx == PIECE_NONE {
            None
        } else {
            Some(PieceType::ALL[idx as usize])
        }
    }

    /// Promotion piece type, if any.
    #[inline]
    pub fn promotion(self) -> Option<PieceType> {
        let idx = (self.0 >> 24) & 0xF;
        if idx == PIECE_NONE {
            None
        } else {
            Some(PieceType::ALL[idx as usize])
        }
    }

    /// Move flags.
    #[inline]
    pub fn flags(self) -> u32 {
        (self.0 >> 28) & 0x7
    }

    /// Whether this is a capture.
    #[inline]
    pub fn is_capture(self) -> bool {
        self.captured().is_some()
    }

    /// Whether this is a promotion.
    #[inline]
    pub fn is_promotion(self) -> bool {
        self.promotion().is_some()
    }

    /// Whether this is a castling move.
    #[inline]
    pub fn is_castle(self) -> bool {
        self.flags() == FLAG_CASTLE_KING || self.flags() == FLAG_CASTLE_QUEEN
    }

    /// Whether this is an en passant capture.
    #[inline]
    pub fn is_en_passant(self) -> bool {
        self.flags() == FLAG_EN_PASSANT
    }

    /// Whether this is a double pawn push.
    #[inline]
    pub fn is_double_push(self) -> bool {
        self.flags() == FLAG_DOUBLE_PUSH
    }

    /// Raw u32 encoding.
    #[inline]
    pub fn raw(self) -> u32 {
        self.0
    }

    /// Format move as algebraic string (e.g. "d2d4", "e7e8q").
    pub fn to_algebraic(self) -> String {
        let mut s = format!(
            "{}{}",
            square_name(self.from_sq()),
            square_name(self.to_sq())
        );
        if let Some(promo) = self.promotion() {
            s.push(promo.fen_char().to_ascii_lowercase());
        }
        s
    }
}

impl std::fmt::Debug for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Move({})", self.to_algebraic())
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_algebraic())
    }
}

/// State needed to undo a move (returned by make_move, consumed by unmake_move).
#[derive(Clone, Copy, Debug)]
pub struct MoveUndo {
    /// The captured piece (full Piece, not just type — includes owner).
    pub captured_piece: Option<Piece>,
    /// Previous castling rights.
    pub castling_rights: u8,
    /// Previous en passant square.
    pub en_passant: Option<Square>,
    /// Previous halfmove clock.
    pub halfmove_clock: u16,
    /// Zobrist hash before the move (for verification).
    pub zobrist_before: u64,
}

/// Castling configuration for one player.
struct CastlingConfig {
    king_sq: Square,
    kingside_rook_sq: Square,
    queenside_rook_sq: Square,
    king_target_ks: Square,
    rook_target_ks: Square,
    king_target_qs: Square,
    rook_target_qs: Square,
    kingside_bit: u8,
    queenside_bit: u8,
}

/// Get the castling details for a player, reading initial positions from the board.
/// Supports both standard and Chess960 starting positions.
fn castling_config(player: Player, board: &Board) -> CastlingConfig {
    let (king_sq, ks_rook, qs_rook) = board.castling_starts()[player.index()];
    match player {
        Player::Red => CastlingConfig {
            king_sq,
            kingside_rook_sq: ks_rook,
            queenside_rook_sq: qs_rook,
            king_target_ks: square_from(9, 0).unwrap(),
            rook_target_ks: square_from(8, 0).unwrap(),
            king_target_qs: square_from(5, 0).unwrap(),
            rook_target_qs: square_from(6, 0).unwrap(),
            kingside_bit: CASTLE_RED_KING,
            queenside_bit: CASTLE_RED_QUEEN,
        },
        Player::Blue => CastlingConfig {
            king_sq,
            kingside_rook_sq: ks_rook,
            queenside_rook_sq: qs_rook,
            king_target_ks: square_from(0, 4).unwrap(),
            rook_target_ks: square_from(0, 5).unwrap(),
            king_target_qs: square_from(0, 8).unwrap(),
            rook_target_qs: square_from(0, 7).unwrap(),
            kingside_bit: CASTLE_BLUE_KING,
            queenside_bit: CASTLE_BLUE_QUEEN,
        },
        Player::Yellow => CastlingConfig {
            king_sq,
            kingside_rook_sq: ks_rook,
            queenside_rook_sq: qs_rook,
            king_target_ks: square_from(4, 13).unwrap(),
            rook_target_ks: square_from(5, 13).unwrap(),
            king_target_qs: square_from(8, 13).unwrap(),
            rook_target_qs: square_from(7, 13).unwrap(),
            kingside_bit: CASTLE_YELLOW_KING,
            queenside_bit: CASTLE_YELLOW_QUEEN,
        },
        Player::Green => CastlingConfig {
            king_sq,
            kingside_rook_sq: ks_rook,
            queenside_rook_sq: qs_rook,
            king_target_ks: square_from(13, 9).unwrap(),
            rook_target_ks: square_from(13, 8).unwrap(),
            king_target_qs: square_from(13, 5).unwrap(),
            rook_target_qs: square_from(13, 6).unwrap(),
            kingside_bit: CASTLE_GREEN_KING,
            queenside_bit: CASTLE_GREEN_QUEEN,
        },
    }
}

/// Apply a move to the board. Returns undo information for unmake_move.
pub fn make_move(board: &mut Board, mv: Move) -> MoveUndo {
    let player = board.side_to_move();
    let from = mv.from_sq();
    let to = mv.to_sq();

    // Save undo state
    let undo = MoveUndo {
        captured_piece: None, // Will be set below if capture
        castling_rights: board.castling_rights(),
        en_passant: board.en_passant(),
        halfmove_clock: board.halfmove_clock(),
        zobrist_before: board.zobrist(),
    };

    // Clear en passant (will be set again if double push)
    board.set_en_passant(None);

    let mut captured_piece = None;

    match mv.flags() {
        FLAG_EN_PASSANT => {
            // Remove the captured pawn (not on the target square).
            // find_ep_captured_pawn_sq returns None only if generate.rs
            // let an invalid EP move through — should not happen after the
            // validation gate added in generate.rs, but guard defensively.
            let captured_pawn_sq = find_ep_captured_pawn_sq(board, to, player)
                .expect("make_move EP: no enemy pawn near ep_target (invalid EP move generated)");
            captured_piece = Some(board.remove_piece(captured_pawn_sq));
            board.move_piece(from, to);
        }
        FLAG_CASTLE_KING => {
            let config = castling_config(player, board);
            // Chess960: king/rook may overlap destinations. Remove both first, then place.
            let king_piece = board.remove_piece(from);
            let rook_piece = board.remove_piece(config.kingside_rook_sq);
            board.place_piece(config.king_target_ks, king_piece);
            board.place_piece(config.rook_target_ks, rook_piece);
        }
        FLAG_CASTLE_QUEEN => {
            let config = castling_config(player, board);
            // Chess960: king/rook may overlap destinations. Remove both first, then place.
            let king_piece = board.remove_piece(from);
            let rook_piece = board.remove_piece(config.queenside_rook_sq);
            board.place_piece(config.king_target_qs, king_piece);
            board.place_piece(config.rook_target_qs, rook_piece);
        }
        _ => {
            // Normal move, capture, double push, promotion
            if mv.is_capture() {
                captured_piece = Some(board.remove_piece(to));
            }

            if mv.is_promotion() {
                // Remove the pawn
                board.remove_piece(from);
                // Place the promoted piece
                let promo_type = mv.promotion().unwrap();
                board.place_piece(to, Piece::new(promo_type, player));
            } else {
                board.move_piece(from, to);
            }

            if mv.is_double_push() {
                // Set en passant target square (the square the pawn passed through)
                let ep_sq = en_passant_target_sq(from, to, player);
                board.set_en_passant(Some(ep_sq));
            }
        }
    }

    // Update castling rights
    update_castling_rights(board, from, to);

    // Update halfmove clock
    if mv.piece_type() == PieceType::Pawn || mv.is_capture() {
        board.set_halfmove_clock(0);
    } else {
        board.set_halfmove_clock(board.halfmove_clock() + 1);
    }

    // Advance side to move
    let next_player = player.next();
    board.set_side_to_move(next_player);

    // Update fullmove number (increments after Green's turn)
    if player == Player::Green {
        board.set_fullmove_number(board.fullmove_number() + 1);
    }

    MoveUndo {
        captured_piece,
        ..undo
    }
}

/// Undo a move, restoring the board to its previous state.
pub fn unmake_move(board: &mut Board, mv: Move, undo: MoveUndo) {
    let from = mv.from_sq();
    let to = mv.to_sq();

    // Restore side to move first (need to know who moved)
    let next_player = board.side_to_move();
    // The player who made this move is the one before the current side to move
    let player = next_player.prev();
    board.set_side_to_move(player);

    // Restore fullmove number
    if player == Player::Green {
        board.set_fullmove_number(board.fullmove_number() - 1);
    }

    match mv.flags() {
        FLAG_EN_PASSANT => {
            // Move pawn back
            board.move_piece(to, from);
            // Restore captured pawn — use the captured piece's owner (the real pusher),
            // not player.prev() which fails when a player has been eliminated.
            if let Some(cap) = undo.captured_piece {
                let captured_pawn_sq = en_passant_captured_sq(to, cap.owner);
                board.place_piece(captured_pawn_sq, cap);
            }
        }
        FLAG_CASTLE_KING => {
            let config = castling_config(player, board);
            // Chess960: remove both, then place at original squares.
            let king_piece = board.remove_piece(config.king_target_ks);
            let rook_piece = board.remove_piece(config.rook_target_ks);
            board.place_piece(from, king_piece);
            board.place_piece(config.kingside_rook_sq, rook_piece);
        }
        FLAG_CASTLE_QUEEN => {
            let config = castling_config(player, board);
            // Chess960: remove both, then place at original squares.
            let king_piece = board.remove_piece(config.king_target_qs);
            let rook_piece = board.remove_piece(config.rook_target_qs);
            board.place_piece(from, king_piece);
            board.place_piece(config.queenside_rook_sq, rook_piece);
        }
        _ => {
            if mv.is_promotion() {
                // Remove the promoted piece
                board.remove_piece(to);
                // Put the pawn back
                board.place_piece(from, Piece::new(PieceType::Pawn, player));
            } else {
                board.move_piece(to, from);
            }

            // Restore captured piece
            if let Some(cap) = undo.captured_piece {
                board.place_piece(to, cap);
            }
        }
    }

    // Restore state
    board.set_castling_rights(undo.castling_rights);
    board.set_en_passant(undo.en_passant);
    board.set_halfmove_clock(undo.halfmove_clock);
}

/// Compute the en passant target square (the square the pawn passed through).
fn en_passant_target_sq(from: Square, to: Square, _player: Player) -> Square {
    // The ep target is the midpoint between from and to
    let from_file = file_of(from) as i8;
    let from_rank = rank_of(from) as i8;
    let to_file = file_of(to) as i8;
    let to_rank = rank_of(to) as i8;
    let mid_file = ((from_file + to_file) / 2) as u8;
    let mid_rank = ((from_rank + to_rank) / 2) as u8;
    square_from(mid_file, mid_rank).unwrap()
}

/// Pawn forward direction delta per player: (file_delta, rank_delta).
const PAWN_FORWARD: [(i8, i8); 4] = [
    (0, 1),  // Red: +rank
    (1, 0),  // Blue: +file
    (0, -1), // Yellow: -rank
    (-1, 0), // Green: -file
];

/// Compute the square of the pawn captured by en passant.
/// The captured pawn is the one that double-stepped PAST the ep target.
/// Its location is: ep_target + pushing_player's forward direction.
fn en_passant_captured_sq(ep_target: Square, pushing_player: Player) -> Square {
    let file = file_of(ep_target) as i8;
    let rank = rank_of(ep_target) as i8;
    let (df, dr) = PAWN_FORWARD[pushing_player.index()];
    square_from((file + df) as u8, (rank + dr) as u8).unwrap()
}

/// Find the square of the pawn captured by en passant by scanning the board.
/// Returns None if no capturable enemy pawn exists near ep_target — this
/// can happen when the current player itself pushed the pawn (self-EP, invalid)
/// or when ep_sq is stale.
///
/// Made pub so generate.rs can use it to validate before generating EP moves.
pub fn find_ep_captured_pawn_sq(
    board: &Board,
    ep_target: Square,
    capturing_player: Player,
) -> Option<Square> {
    let file = file_of(ep_target) as i8;
    let rank = rank_of(ep_target) as i8;
    for pidx in 0..4 {
        let candidate = Player::from_index(pidx).unwrap();
        if candidate == capturing_player {
            continue;
        }
        let (df, dr) = PAWN_FORWARD[candidate.index()];
        let cf = file + df;
        let cr = rank + dr;
        if cf >= 0 && cf < 14 && cr >= 0 && cr < 14 {
            if let Some(sq) = square_from(cf as u8, cr as u8) {
                if let Some(piece) = board.piece_at(sq) {
                    if piece.piece_type == PieceType::Pawn && piece.owner == candidate {
                        return Some(sq);
                    }
                }
            }
        }
    }
    None
}

/// Update castling rights based on piece movement.
fn update_castling_rights(board: &mut Board, from: Square, to: Square) {
    let mut rights = board.castling_rights();
    if rights == 0 {
        return;
    }

    // Check each player's castling squares
    for &player in &Player::ALL {
        let config = castling_config(player, board);

        // King moved -> lose both rights
        if from == config.king_sq {
            rights &= !(config.kingside_bit | config.queenside_bit);
        }

        // Rook moved or captured -> lose that side's right
        if from == config.kingside_rook_sq || to == config.kingside_rook_sq {
            rights &= !config.kingside_bit;
        }
        if from == config.queenside_rook_sq || to == config.queenside_rook_sq {
            rights &= !config.queenside_bit;
        }
    }

    board.set_castling_rights(rights);
}

/// Get the castling configuration (public for use by move generation).
pub fn get_castling_config(
    player: Player,
    board: &Board,
) -> (
    Square, // king_sq
    Square, // ks_rook
    Square, // qs_rook
    Square, // king_target_ks
    Square, // rook_target_ks
    Square, // king_target_qs
    Square, // rook_target_qs
    u8,     // ks_bit
    u8,     // qs_bit
) {
    let c = castling_config(player, board);
    (
        c.king_sq,
        c.kingside_rook_sq,
        c.queenside_rook_sq,
        c.king_target_ks,
        c.rook_target_ks,
        c.king_target_qs,
        c.rook_target_qs,
        c.kingside_bit,
        c.queenside_bit,
    )
}

/// Squares that must be empty for castling (Chess960-compatible).
///
/// Computes the union of:
///   - King's travel path (start → destination, excluding start)
///   - Rook's travel path (start → destination, excluding start)
///   - Minus the king and rook starting squares (they vacate)
///
/// In standard chess this produces the same result as "between king and rook."
/// In Chess960, the king/rook may travel paths that don't overlap with each other.
pub fn castling_empty_squares(player: Player, kingside: bool, board: &Board) -> Vec<Square> {
    let config = castling_config(player, board);
    let (king_start, rook_start, king_dest, rook_dest) = if kingside {
        (
            config.king_sq,
            config.kingside_rook_sq,
            config.king_target_ks,
            config.rook_target_ks,
        )
    } else {
        (
            config.king_sq,
            config.queenside_rook_sq,
            config.king_target_qs,
            config.rook_target_qs,
        )
    };

    let mut must_be_empty: Vec<Square> = Vec::with_capacity(12);

    // King's travel path: all squares from king_start to king_dest (inclusive of dest)
    walk_path_inclusive(&mut must_be_empty, king_start, king_dest);

    // Rook's travel path: all squares from rook_start to rook_dest (inclusive of dest)
    walk_path_inclusive(&mut must_be_empty, rook_start, rook_dest);

    // Remove king and rook themselves (they will vacate their squares)
    must_be_empty.retain(|&sq| sq != king_start && sq != rook_start);

    // Deduplicate (paths may overlap)
    must_be_empty.sort_unstable();
    must_be_empty.dedup();

    must_be_empty
}

/// Walk from `from` towards `to`, adding each intermediate square and `to` itself.
/// Does NOT add `from`. If `from == to`, adds nothing.
fn walk_path_inclusive(out: &mut Vec<Square>, from: Square, to: Square) {
    if from == to {
        return;
    }
    let (ff, fr) = (file_of(from) as i8, rank_of(from) as i8);
    let (tf, tr) = (file_of(to) as i8, rank_of(to) as i8);
    let df = (tf - ff).signum();
    let dr = (tr - fr).signum();
    let mut f = ff + df;
    let mut r = fr + dr;
    // Walk until we've passed `to`
    while (f, r) != (tf + df, tr + dr) {
        out.push(square_from(f as u8, r as u8).unwrap());
        f += df;
        r += dr;
    }
}

/// Squares the king passes through during castling (including from and to).
/// If king_start == king_dest (Chess960 edge case), returns just [king_start].
pub fn castling_king_path(player: Player, kingside: bool, board: &Board) -> Vec<Square> {
    let config = castling_config(player, board);
    let (from, to) = if kingside {
        (config.king_sq, config.king_target_ks)
    } else {
        (config.king_sq, config.king_target_qs)
    };

    if from == to {
        return vec![from];
    }

    let from_file = file_of(from) as i8;
    let from_rank = rank_of(from) as i8;
    let to_file = file_of(to) as i8;
    let to_rank = rank_of(to) as i8;

    let df = (to_file - from_file).signum();
    let dr = (to_rank - from_rank).signum();

    let mut path = vec![from];
    let mut f = from_file + df;
    let mut r = from_rank + dr;
    while (f, r) != (to_file + df, to_rank + dr) {
        path.push(square_from(f as u8, r as u8).unwrap());
        f += df;
        r += dr;
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::{square_from, Board, Piece, PieceType, Player};

    #[test]
    fn test_move_encoding_roundtrip() {
        let from = square_from(4, 1).unwrap();
        let to = square_from(4, 3).unwrap();
        let mv = Move::new(from, to, PieceType::Pawn);
        assert_eq!(mv.from_sq(), from);
        assert_eq!(mv.to_sq(), to);
        assert_eq!(mv.piece_type(), PieceType::Pawn);
        assert_eq!(mv.captured(), None);
        assert_eq!(mv.promotion(), None);
        assert_eq!(mv.flags(), FLAG_NORMAL);
    }

    #[test]
    fn test_move_capture_encoding() {
        let from = square_from(4, 4).unwrap();
        let to = square_from(5, 5).unwrap();
        let mv = Move::new_capture(from, to, PieceType::Bishop, PieceType::Pawn);
        assert!(mv.is_capture());
        assert_eq!(mv.captured(), Some(PieceType::Pawn));
        assert!(!mv.is_promotion());
    }

    #[test]
    fn test_move_promotion_encoding() {
        let from = square_from(4, 7).unwrap();
        let to = square_from(4, 8).unwrap();
        let mv = Move::new_promotion(from, to, None, PieceType::PromotedQueen);
        assert!(mv.is_promotion());
        assert_eq!(mv.promotion(), Some(PieceType::PromotedQueen));
        assert!(!mv.is_capture());
    }

    #[test]
    fn test_make_unmake_simple_move() {
        let mut board = Board::starting_position();
        let hash_before = board.zobrist();
        let from = square_from(4, 1).unwrap(); // e2
        let to = square_from(4, 3).unwrap(); // e4

        let mv = Move::new_double_push(from, to);
        let undo = make_move(&mut board, mv);

        // Pawn should be on e4 now
        assert!(board.piece_at(from).is_none());
        assert_eq!(board.piece_at(to).unwrap().piece_type, PieceType::Pawn);
        assert!(board.verify_zobrist());
        assert!(board.verify_piece_lists());

        // Unmake
        unmake_move(&mut board, mv, undo);
        assert_eq!(board.zobrist(), hash_before);
        assert!(board.verify_zobrist());
        assert!(board.verify_piece_lists());
        assert_eq!(board.piece_at(from).unwrap().piece_type, PieceType::Pawn);
        assert!(board.piece_at(to).is_none());
    }

    #[test]
    fn test_make_unmake_capture() {
        let mut board = Board::empty();
        let pawn_sq = square_from(5, 4).unwrap();
        let target_sq = square_from(6, 5).unwrap();
        board.place_piece(pawn_sq, Piece::new(PieceType::Pawn, Player::Red));
        board.place_piece(target_sq, Piece::new(PieceType::Knight, Player::Blue));
        let hash_before = board.zobrist();

        let mv = Move::new_capture(pawn_sq, target_sq, PieceType::Pawn, PieceType::Knight);
        let undo = make_move(&mut board, mv);

        assert!(board.piece_at(pawn_sq).is_none());
        assert_eq!(board.piece_at(target_sq).unwrap().owner, Player::Red);
        assert!(board.verify_zobrist());

        unmake_move(&mut board, mv, undo);
        assert_eq!(board.zobrist(), hash_before);
        assert_eq!(
            board.piece_at(target_sq).unwrap().piece_type,
            PieceType::Knight
        );
        assert_eq!(board.piece_at(target_sq).unwrap().owner, Player::Blue);
    }

    #[test]
    fn test_castling_empty_squares_red_kingside() {
        let board = Board::starting_position();
        let empty = castling_empty_squares(Player::Red, true, &board);
        // Red kingside: king h1 (7,0) -> j1, rook k1 (10,0) -> i1
        // King path: i1, j1. Rook path: i1. Union minus king/rook starts: i1, j1
        assert_eq!(empty.len(), 2);
        assert!(empty.contains(&square_from(8, 0).unwrap())); // i1
        assert!(empty.contains(&square_from(9, 0).unwrap())); // j1
    }

    #[test]
    fn test_castling_empty_squares_red_queenside() {
        let board = Board::starting_position();
        let empty = castling_empty_squares(Player::Red, false, &board);
        // Red queenside: king h1 (7,0) -> f1, rook d1 (3,0) -> g1
        // King path: g1, f1. Rook path: e1, f1, g1. Union minus king/rook: e1, f1, g1
        assert_eq!(empty.len(), 3);
        assert!(empty.contains(&square_from(4, 0).unwrap())); // e1
        assert!(empty.contains(&square_from(5, 0).unwrap())); // f1
        assert!(empty.contains(&square_from(6, 0).unwrap())); // g1
    }

    #[test]
    fn test_castling_king_path_red_kingside() {
        let board = Board::starting_position();
        let path = castling_king_path(Player::Red, true, &board);
        // King h1 -> j1: h1, i1, j1
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], square_from(7, 0).unwrap()); // h1
        assert_eq!(path[1], square_from(8, 0).unwrap()); // i1
        assert_eq!(path[2], square_from(9, 0).unwrap()); // j1
    }
}
