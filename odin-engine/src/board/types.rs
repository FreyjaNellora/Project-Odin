// Core types for the four-player chess board.
//
// Player: Red, Blue, Yellow, Green (clockwise turn order).
// PieceType: Pawn, Knight, Bishop, Rook, Queen, King, PromotedQueen.
// PieceStatus: Alive, Dead (DKW), Terrain.

/// Number of players in four-player chess.
pub const PLAYER_COUNT: usize = 4;

/// Number of distinct piece types.
pub const PIECE_TYPE_COUNT: usize = 7;

/// The four players. Turn order: Red -> Blue -> Yellow -> Green (clockwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Player {
    Red = 0,
    Blue = 1,
    Yellow = 2,
    Green = 3,
}

impl Player {
    /// All players in turn order.
    pub const ALL: [Player; PLAYER_COUNT] =
        [Player::Red, Player::Blue, Player::Yellow, Player::Green];

    /// Array index for this player (0-3).
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Next player in turn order (wraps around).
    #[inline]
    pub fn next(self) -> Player {
        match self {
            Player::Red => Player::Blue,
            Player::Blue => Player::Yellow,
            Player::Yellow => Player::Green,
            Player::Green => Player::Red,
        }
    }

    /// Player from index (0-3).
    pub fn from_index(idx: usize) -> Option<Player> {
        match idx {
            0 => Some(Player::Red),
            1 => Some(Player::Blue),
            2 => Some(Player::Yellow),
            3 => Some(Player::Green),
            _ => None,
        }
    }
}

/// Chess piece types. PromotedQueen is distinct from Queen (worth 1 point
/// on capture in FFA scoring, but moves identically and has 900cp eval value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
    PromotedQueen = 6,
}

impl PieceType {
    /// All piece types.
    pub const ALL: [PieceType; PIECE_TYPE_COUNT] = [
        PieceType::Pawn,
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
        PieceType::King,
        PieceType::PromotedQueen,
    ];

    /// Array index for this piece type (0-6).
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// FEN character for this piece type (uppercase).
    pub fn fen_char(self) -> char {
        match self {
            PieceType::Pawn => 'P',
            PieceType::Knight => 'N',
            PieceType::Bishop => 'B',
            PieceType::Rook => 'R',
            PieceType::Queen => 'Q',
            PieceType::King => 'K',
            PieceType::PromotedQueen => 'W', // "War queen" — distinct from Q
        }
    }

    /// Parse from FEN character (case-insensitive for the type).
    pub fn from_fen_char(c: char) -> Option<PieceType> {
        match c.to_ascii_uppercase() {
            'P' => Some(PieceType::Pawn),
            'N' => Some(PieceType::Knight),
            'B' => Some(PieceType::Bishop),
            'R' => Some(PieceType::Rook),
            'Q' => Some(PieceType::Queen),
            'K' => Some(PieceType::King),
            'W' => Some(PieceType::PromotedQueen),
            _ => None,
        }
    }
}

/// Piece status on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PieceStatus {
    /// Normal active piece.
    Alive = 0,
    /// Dead King Walking — grey piece, worth 0 points on capture.
    Dead = 1,
    /// Terrain — immovable, uncapturable wall (Terrain mode only).
    Terrain = 2,
}

/// A piece on the board: type + owner + status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    pub piece_type: PieceType,
    pub owner: Player,
    pub status: PieceStatus,
}

impl Piece {
    /// Create a new alive piece.
    #[inline]
    pub fn new(piece_type: PieceType, owner: Player) -> Self {
        Self {
            piece_type,
            owner,
            status: PieceStatus::Alive,
        }
    }

    /// Whether this piece is alive (not dead/terrain).
    #[inline]
    pub fn is_alive(self) -> bool {
        self.status == PieceStatus::Alive
    }

    /// Whether this piece is terrain (immovable wall).
    #[inline]
    pub fn is_terrain(self) -> bool {
        self.status == PieceStatus::Terrain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_turn_order() {
        assert_eq!(Player::Red.next(), Player::Blue);
        assert_eq!(Player::Blue.next(), Player::Yellow);
        assert_eq!(Player::Yellow.next(), Player::Green);
        assert_eq!(Player::Green.next(), Player::Red);
    }

    #[test]
    fn test_player_index_roundtrip() {
        for &player in &Player::ALL {
            assert_eq!(Player::from_index(player.index()), Some(player));
        }
    }

    #[test]
    fn test_piece_type_fen_roundtrip() {
        for &pt in &PieceType::ALL {
            let c = pt.fen_char();
            assert_eq!(PieceType::from_fen_char(c), Some(pt));
        }
    }

    #[test]
    fn test_piece_creation() {
        let piece = Piece::new(PieceType::King, Player::Red);
        assert_eq!(piece.piece_type, PieceType::King);
        assert_eq!(piece.owner, Player::Red);
        assert!(piece.is_alive());
        assert!(!piece.is_terrain());
    }

    #[test]
    fn test_promoted_queen_is_distinct() {
        assert_ne!(PieceType::Queen, PieceType::PromotedQueen);
        assert_ne!(
            PieceType::Queen.fen_char(),
            PieceType::PromotedQueen.fen_char()
        );
    }
}
