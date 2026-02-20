// Board struct — the core board representation.
//
// 196-element array of Option<Piece>, plus per-player piece lists,
// per-player king square tracking, and Zobrist hash.

use super::square::{is_valid_square, square_from, Square, TOTAL_SQUARES};
use super::types::{Piece, PieceType, Player, PLAYER_COUNT};
use super::zobrist::ZobristKeys;

/// Maximum number of pieces a single player can have (16 in starting position).
pub const MAX_PIECES_PER_PLAYER: usize = 16;

/// Castling rights bitmask constants.
/// Each player has 2 bits: kingside (lower) and queenside (upper).
/// Red=bits 0-1, Blue=bits 2-3, Yellow=bits 4-5, Green=bits 6-7.
pub const CASTLE_RED_KING: u8 = 0x01;
pub const CASTLE_RED_QUEEN: u8 = 0x02;
pub const CASTLE_BLUE_KING: u8 = 0x04;
pub const CASTLE_BLUE_QUEEN: u8 = 0x08;
pub const CASTLE_YELLOW_KING: u8 = 0x10;
pub const CASTLE_YELLOW_QUEEN: u8 = 0x20;
pub const CASTLE_GREEN_KING: u8 = 0x40;
pub const CASTLE_GREEN_QUEEN: u8 = 0x80;

/// The four-player chess board.
#[derive(Clone)]
pub struct Board {
    /// 196-element flat array. `None` = empty or invalid corner.
    squares: [Option<Piece>; TOTAL_SQUARES],
    /// Per-player piece lists: (PieceType, Square) pairs.
    piece_lists: [Vec<(PieceType, Square)>; PLAYER_COUNT],
    /// Per-player king square (updated on every king move).
    king_squares: [Square; PLAYER_COUNT],
    /// Incrementally-maintained Zobrist hash.
    zobrist: u64,
    /// Castling rights: 8 bits, 2 per player.
    castling_rights: u8,
    /// En passant target square (the square a capturing pawn lands on).
    /// In 4PC, this is a full square index because Blue/Green pawns move
    /// along files, not ranks, so a file alone doesn't identify the target.
    en_passant: Option<Square>,
    /// Which player moves next.
    side_to_move: Player,
    /// Half-move clock for 50-move rule (resets on pawn move or capture).
    halfmove_clock: u16,
    /// Full-move number (increments after Green's turn).
    fullmove_number: u16,
    /// Shared Zobrist key table.
    zobrist_keys: &'static ZobristKeys,
}

/// Lazily-initialized global Zobrist key table.
fn global_zobrist_keys() -> &'static ZobristKeys {
    use std::sync::OnceLock;
    static KEYS: OnceLock<ZobristKeys> = OnceLock::new();
    KEYS.get_or_init(ZobristKeys::new)
}

impl Board {
    /// Create an empty board with no pieces.
    pub fn empty() -> Self {
        let keys = global_zobrist_keys();
        let mut board = Self {
            squares: [None; TOTAL_SQUARES],
            piece_lists: [vec![], vec![], vec![], vec![]],
            king_squares: [0; PLAYER_COUNT],
            zobrist: 0,
            castling_rights: 0,
            en_passant: None,
            side_to_move: Player::Red,
            halfmove_clock: 0,
            fullmove_number: 1,
            zobrist_keys: keys,
        };
        // Hash in the starting side to move
        board.zobrist ^= keys.side_to_move_key(Player::Red.index());
        board
    }

    /// Create the standard starting position.
    pub fn starting_position() -> Self {
        let mut board = Self::empty();

        // Red: south side. Back rank d1-k1 (rank 0, files 3-10).
        // Pieces: R N B Q K B N R. Pawns: d2-k2 (rank 1, files 3-10).
        let red_back_rank = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Queen,
            PieceType::King,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (i, &pt) in red_back_rank.iter().enumerate() {
            let file = (3 + i) as u8;
            board.place_piece(square_from(file, 0).unwrap(), Piece::new(pt, Player::Red));
            board.place_piece(
                square_from(file, 1).unwrap(),
                Piece::new(PieceType::Pawn, Player::Red),
            );
        }

        // Blue: west side. Back rank a4-a11 (file 0, ranks 3-10).
        // Pieces: R N B K Q B N R (K and Q swapped from Red's perspective).
        // Pawns: b4-b11 (file 1, ranks 3-10).
        let blue_back_rank = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::King,
            PieceType::Queen,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (i, &pt) in blue_back_rank.iter().enumerate() {
            let rank = (3 + i) as u8;
            board.place_piece(square_from(0, rank).unwrap(), Piece::new(pt, Player::Blue));
            board.place_piece(
                square_from(1, rank).unwrap(),
                Piece::new(PieceType::Pawn, Player::Blue),
            );
        }

        // Yellow: north side. Back rank d14-k14 (rank 13, files 3-10).
        // Pieces: R N B K Q B N R (mirrored from Red).
        // Pawns: d13-k13 (rank 12, files 3-10).
        let yellow_back_rank = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::King,
            PieceType::Queen,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (i, &pt) in yellow_back_rank.iter().enumerate() {
            let file = (3 + i) as u8;
            board.place_piece(
                square_from(file, 13).unwrap(),
                Piece::new(pt, Player::Yellow),
            );
            board.place_piece(
                square_from(file, 12).unwrap(),
                Piece::new(PieceType::Pawn, Player::Yellow),
            );
        }

        // Green: east side. Back rank n4-n11 (file 13, ranks 3-10).
        // Pieces: R N B Q K B N R (same layout as Red, from Green's perspective).
        // Pawns: m4-m11 (file 12, ranks 3-10).
        let green_back_rank = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Queen,
            PieceType::King,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (i, &pt) in green_back_rank.iter().enumerate() {
            let rank = (3 + i) as u8;
            board.place_piece(
                square_from(13, rank).unwrap(),
                Piece::new(pt, Player::Green),
            );
            board.place_piece(
                square_from(12, rank).unwrap(),
                Piece::new(PieceType::Pawn, Player::Green),
            );
        }

        // All castling rights available at start
        board.set_castling_rights(0xFF);

        board
    }

    // --- Accessors ---

    /// Get the piece on a square, if any.
    #[inline]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.squares[sq as usize]
    }

    /// Get the piece list for a player: (PieceType, Square) pairs.
    pub fn piece_list(&self, player: Player) -> &[(PieceType, Square)] {
        &self.piece_lists[player.index()]
    }

    /// Get the king square for a player.
    #[inline]
    pub fn king_square(&self, player: Player) -> Square {
        self.king_squares[player.index()]
    }

    /// Current Zobrist hash.
    #[inline]
    pub fn zobrist(&self) -> u64 {
        self.zobrist
    }

    /// Current castling rights (8-bit mask).
    #[inline]
    pub fn castling_rights(&self) -> u8 {
        self.castling_rights
    }

    /// Current en passant target square, if any.
    #[inline]
    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    /// Current side to move.
    #[inline]
    pub fn side_to_move(&self) -> Player {
        self.side_to_move
    }

    /// Half-move clock (for 50-move rule).
    #[inline]
    pub fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }

    /// Full-move number.
    #[inline]
    pub fn fullmove_number(&self) -> u16 {
        self.fullmove_number
    }

    /// Reference to the Zobrist key table.
    pub fn zobrist_keys(&self) -> &ZobristKeys {
        self.zobrist_keys
    }

    // --- Mutators ---

    /// Place a piece on a square. Updates array, piece list, king square, and Zobrist hash.
    /// The square must be valid and empty.
    pub fn place_piece(&mut self, sq: Square, piece: Piece) {
        debug_assert!(is_valid_square(sq), "cannot place piece on invalid square");
        debug_assert!(
            self.squares[sq as usize].is_none(),
            "square already occupied"
        );

        self.squares[sq as usize] = Some(piece);

        // Update Zobrist hash
        let key = self
            .zobrist_keys
            .piece_key(sq, piece.piece_type.index(), piece.owner.index());
        self.zobrist ^= key;

        // Update piece list
        self.piece_lists[piece.owner.index()].push((piece.piece_type, sq));

        // Track king square
        if piece.piece_type == PieceType::King {
            self.king_squares[piece.owner.index()] = sq;
        }
    }

    /// Remove a piece from a square. Updates array, piece list, and Zobrist hash.
    /// Returns the removed piece. The square must contain a piece.
    pub fn remove_piece(&mut self, sq: Square) -> Piece {
        let piece = self.squares[sq as usize].expect("remove_piece: square is empty");

        self.squares[sq as usize] = None;

        // Update Zobrist hash (XOR is its own inverse)
        let key = self
            .zobrist_keys
            .piece_key(sq, piece.piece_type.index(), piece.owner.index());
        self.zobrist ^= key;

        // Update piece list — find and remove the entry
        let list = &mut self.piece_lists[piece.owner.index()];
        if let Some(pos) = list
            .iter()
            .position(|&(pt, s)| pt == piece.piece_type && s == sq)
        {
            list.swap_remove(pos);
        }

        piece
    }

    /// Move a piece from one square to another (no capture handling).
    /// For captures, call remove_piece on the target first.
    pub fn move_piece(&mut self, from: Square, to: Square) {
        let piece = self.remove_piece(from);
        self.place_piece(to, piece);
    }

    /// Set castling rights. Updates Zobrist hash for the change.
    pub fn set_castling_rights(&mut self, new_rights: u8) {
        let old_rights = self.castling_rights;
        if old_rights != new_rights {
            self.zobrist ^= self.zobrist_keys.castling_key(old_rights);
            self.zobrist ^= self.zobrist_keys.castling_key(new_rights);
            self.castling_rights = new_rights;
        }
    }

    /// Set the en passant target square. Updates Zobrist hash.
    pub fn set_en_passant(&mut self, sq: Option<Square>) {
        // XOR out old en passant
        if let Some(old_sq) = self.en_passant {
            self.zobrist ^= self.zobrist_keys.en_passant_key(old_sq);
        }
        // XOR in new en passant
        if let Some(new_sq) = sq {
            self.zobrist ^= self.zobrist_keys.en_passant_key(new_sq);
        }
        self.en_passant = sq;
    }

    /// Set the side to move. Updates Zobrist hash.
    pub fn set_side_to_move(&mut self, player: Player) {
        let old = self.side_to_move;
        if old != player {
            self.zobrist ^= self.zobrist_keys.side_to_move_key(old.index());
            self.zobrist ^= self.zobrist_keys.side_to_move_key(player.index());
            self.side_to_move = player;
        }
    }

    /// Set the halfmove clock.
    pub fn set_halfmove_clock(&mut self, clock: u16) {
        self.halfmove_clock = clock;
    }

    /// Set the fullmove number.
    pub fn set_fullmove_number(&mut self, num: u16) {
        self.fullmove_number = num;
    }

    /// Compute the full Zobrist hash from scratch (for verification).
    pub fn compute_full_hash(&self) -> u64 {
        let mut hash = 0u64;

        // Piece-square keys
        for sq in 0..TOTAL_SQUARES as u8 {
            if let Some(piece) = self.piece_at(sq) {
                hash ^=
                    self.zobrist_keys
                        .piece_key(sq, piece.piece_type.index(), piece.owner.index());
            }
        }

        // Castling rights
        hash ^= self.zobrist_keys.castling_key(self.castling_rights);

        // En passant
        if let Some(ep_sq) = self.en_passant {
            hash ^= self.zobrist_keys.en_passant_key(ep_sq);
        }

        // Side to move
        hash ^= self
            .zobrist_keys
            .side_to_move_key(self.side_to_move.index());

        hash
    }

    /// Verify that the incremental Zobrist hash matches full recomputation.
    pub fn verify_zobrist(&self) -> bool {
        self.zobrist == self.compute_full_hash()
    }

    /// Verify piece list sync: array and piece lists agree.
    pub fn verify_piece_lists(&self) -> bool {
        for &player in &Player::ALL {
            let list = &self.piece_lists[player.index()];

            // Every piece in the list must exist on the board
            for &(pt, sq) in list {
                match self.piece_at(sq) {
                    Some(piece) if piece.piece_type == pt && piece.owner == player => {}
                    _ => return false,
                }
            }

            // Every piece on the board for this player must be in the list
            for sq in 0..TOTAL_SQUARES as u8 {
                if let Some(piece) = self.piece_at(sq) {
                    if piece.owner == player
                        && !list
                            .iter()
                            .any(|&(pt, s)| pt == piece.piece_type && s == sq)
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Change a piece's status in-place (e.g. Alive → Dead for DKW, Alive → Terrain).
    /// Does NOT affect the Zobrist hash — hash keys are indexed by (square, piece_type, owner)
    /// and do not include a status dimension.
    pub fn set_piece_status(&mut self, sq: Square, new_status: super::types::PieceStatus) {
        if let Some(ref mut piece) = self.squares[sq as usize] {
            piece.status = new_status;
        }
    }

    /// Count total pieces on the board.
    pub fn piece_count(&self) -> usize {
        self.piece_lists.iter().map(|l| l.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::super::square::{file_of, rank_of, square_from};
    use super::*;

    #[test]
    fn test_empty_board() {
        let board = Board::empty();
        assert_eq!(board.piece_count(), 0);
        assert_eq!(board.side_to_move(), Player::Red);
        assert_eq!(board.castling_rights(), 0);
        assert!(board.en_passant().is_none());
        assert!(board.verify_zobrist());
    }

    #[test]
    fn test_starting_position_piece_count() {
        let board = Board::starting_position();
        // 4 players x 16 pieces each = 64 pieces
        assert_eq!(board.piece_count(), 64);
    }

    #[test]
    fn test_starting_position_kings() {
        let board = Board::starting_position();
        // Red king: e1 (file 4, rank 0) — but looking at piece order: R N B Q K B N R
        // d1=R, e1=N, f1=B, g1=Q, h1=K, i1=B, j1=N, k1=R
        // So Red king is at file 7 (h), rank 0 => h1
        let red_king_sq = board.king_square(Player::Red);
        assert_eq!(file_of(red_king_sq), 7);
        assert_eq!(rank_of(red_king_sq), 0);

        // Blue king: a7 (file 0, rank 6) — R N B K Q B N R
        // a4=R, a5=N, a6=B, a7=K, a8=Q, a9=B, a10=N, a11=R
        let blue_king_sq = board.king_square(Player::Blue);
        assert_eq!(file_of(blue_king_sq), 0);
        assert_eq!(rank_of(blue_king_sq), 6);

        // Yellow king: g14 (file 6, rank 13) — R N B K Q B N R
        // d14=R, e14=N, f14=B, g14=K, h14=Q, i14=B, j14=N, k14=R
        let yellow_king_sq = board.king_square(Player::Yellow);
        assert_eq!(file_of(yellow_king_sq), 6);
        assert_eq!(rank_of(yellow_king_sq), 13);

        // Green king: n8 (file 13, rank 7) — R N B Q K B N R
        // n4=R, n5=N, n6=B, n7=Q, n8=K, n9=B, n10=N, n11=R
        let green_king_sq = board.king_square(Player::Green);
        assert_eq!(file_of(green_king_sq), 13);
        assert_eq!(rank_of(green_king_sq), 7);
    }

    #[test]
    fn test_starting_position_zobrist_valid() {
        let board = Board::starting_position();
        assert!(board.verify_zobrist());
    }

    #[test]
    fn test_starting_position_piece_lists_valid() {
        let board = Board::starting_position();
        assert!(board.verify_piece_lists());
    }

    #[test]
    fn test_starting_position_castling() {
        let board = Board::starting_position();
        assert_eq!(board.castling_rights(), 0xFF);
    }

    #[test]
    fn test_place_remove_piece() {
        let mut board = Board::empty();
        let sq = square_from(5, 5).unwrap();
        let piece = Piece::new(PieceType::Pawn, Player::Red);

        board.place_piece(sq, piece);
        assert_eq!(board.piece_at(sq), Some(piece));
        assert_eq!(board.piece_count(), 1);
        assert!(board.verify_zobrist());
        assert!(board.verify_piece_lists());

        let hash_with_piece = board.zobrist();

        let removed = board.remove_piece(sq);
        assert_eq!(removed, piece);
        assert_eq!(board.piece_at(sq), None);
        assert_eq!(board.piece_count(), 0);
        assert!(board.verify_zobrist());
        assert!(board.verify_piece_lists());

        // Hash should differ with vs without piece
        assert_ne!(hash_with_piece, board.zobrist());
    }

    #[test]
    fn test_move_piece_updates_correctly() {
        let mut board = Board::empty();
        let from = square_from(5, 5).unwrap();
        let to = square_from(5, 6).unwrap();
        let piece = Piece::new(PieceType::Pawn, Player::Red);

        board.place_piece(from, piece);
        board.move_piece(from, to);

        assert_eq!(board.piece_at(from), None);
        assert_eq!(board.piece_at(to), Some(piece));
        assert!(board.verify_zobrist());
        assert!(board.verify_piece_lists());
    }

    #[test]
    fn test_zobrist_changes_on_side_to_move() {
        let mut board = Board::empty();
        let hash1 = board.zobrist();

        board.set_side_to_move(Player::Blue);
        let hash2 = board.zobrist();
        assert_ne!(hash1, hash2);
        assert!(board.verify_zobrist());

        board.set_side_to_move(Player::Red);
        assert_eq!(board.zobrist(), hash1);
    }

    #[test]
    fn test_zobrist_changes_on_castling() {
        let mut board = Board::empty();
        let hash1 = board.zobrist();

        board.set_castling_rights(0xFF);
        let hash2 = board.zobrist();
        assert_ne!(hash1, hash2);
        assert!(board.verify_zobrist());
    }

    #[test]
    fn test_zobrist_changes_on_en_passant() {
        let mut board = Board::empty();
        let hash1 = board.zobrist();

        // Use a valid square as ep target (f3 = file 5, rank 2)
        let ep_sq = square_from(5, 2).unwrap();
        board.set_en_passant(Some(ep_sq));
        let hash2 = board.zobrist();
        assert_ne!(hash1, hash2);
        assert!(board.verify_zobrist());

        board.set_en_passant(None);
        assert_eq!(board.zobrist(), hash1);
    }

    #[test]
    fn test_per_player_piece_lists() {
        let board = Board::starting_position();
        for &player in &Player::ALL {
            assert_eq!(board.piece_list(player).len(), 16);
        }
    }

    #[test]
    fn test_red_back_rank_pieces() {
        let board = Board::starting_position();
        // d1=Rook, e1=Knight, f1=Bishop, g1=Queen, h1=King, i1=Bishop, j1=Knight, k1=Rook
        let expected = [
            (3, PieceType::Rook),
            (4, PieceType::Knight),
            (5, PieceType::Bishop),
            (6, PieceType::Queen),
            (7, PieceType::King),
            (8, PieceType::Bishop),
            (9, PieceType::Knight),
            (10, PieceType::Rook),
        ];
        for (file, pt) in expected {
            let sq = square_from(file, 0).unwrap();
            let piece = board.piece_at(sq).expect("piece should exist");
            assert_eq!(piece.piece_type, pt, "wrong piece at file {file} rank 0");
            assert_eq!(piece.owner, Player::Red);
        }
    }
}
