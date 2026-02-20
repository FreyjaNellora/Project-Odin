// FEN4 parser and serializer for four-player chess.
//
// FEN4 format (chess.com inspired, adapted for Odin):
//   <rank14>/<rank13>/.../<rank1> <side> <castling> <en_passant> <halfmove> <fullmove>
//
// Ranks are serialized top-to-bottom (rank 14 first, rank 1 last).
// Each rank: pieces encoded with owner prefix (R=Red, B=Blue, Y=Yellow, G=Green)
// followed by piece letter. Numbers denote empty squares. Invalid corners
// are encoded as 'x' (3 per corner row).
//
// Side to move: r/b/y/g
// Castling: A/a=Red K/Q, B/b=Blue K/Q, C/c=Yellow K/Q, D/d=Green K/Q. "-" = none.
// En passant: file letter or "-"
// Halfmove/fullmove: decimal integers.

use super::board_struct::{
    Board, CASTLE_BLUE_KING, CASTLE_BLUE_QUEEN, CASTLE_GREEN_KING, CASTLE_GREEN_QUEEN,
    CASTLE_RED_KING, CASTLE_RED_QUEEN, CASTLE_YELLOW_KING, CASTLE_YELLOW_QUEEN,
};
use super::square::{file_char, is_valid_square, square_from, BOARD_SIZE};
use super::types::{Piece, PieceType, Player};

/// Errors that can occur during FEN4 parsing.
#[derive(Debug, PartialEq)]
pub enum Fen4Error {
    InvalidRankCount,
    InvalidPiece(String),
    InvalidSideToMove(String),
    InvalidCastling(String),
    InvalidEnPassant(String),
    InvalidNumber(String),
    TooFewFields,
}

impl std::fmt::Display for Fen4Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fen4Error::InvalidRankCount => write!(f, "expected 14 ranks"),
            Fen4Error::InvalidPiece(s) => write!(f, "invalid piece: {s}"),
            Fen4Error::InvalidSideToMove(s) => write!(f, "invalid side to move: {s}"),
            Fen4Error::InvalidCastling(s) => write!(f, "invalid castling: {s}"),
            Fen4Error::InvalidEnPassant(s) => write!(f, "invalid en passant: {s}"),
            Fen4Error::InvalidNumber(s) => write!(f, "invalid number: {s}"),
            Fen4Error::TooFewFields => write!(f, "expected 6 fields"),
        }
    }
}

/// Player prefix character in FEN4.
fn player_prefix(player: Player) -> char {
    match player {
        Player::Red => 'R',
        Player::Blue => 'B',
        Player::Yellow => 'Y',
        Player::Green => 'G',
    }
}

/// Parse a player prefix character.
fn parse_player_prefix(c: char) -> Option<Player> {
    match c {
        'R' => Some(Player::Red),
        'B' => Some(Player::Blue),
        'Y' => Some(Player::Yellow),
        'G' => Some(Player::Green),
        _ => None,
    }
}

impl Board {
    /// Parse a FEN4 string into a Board.
    pub fn from_fen4(fen: &str) -> Result<Board, Fen4Error> {
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() < 6 {
            return Err(Fen4Error::TooFewFields);
        }

        let mut board = Board::empty();

        // Parse ranks (top to bottom: rank 14 = index 13, down to rank 1 = index 0)
        let ranks: Vec<&str> = fields[0].split('/').collect();
        if ranks.len() != BOARD_SIZE {
            return Err(Fen4Error::InvalidRankCount);
        }

        for (rank_from_top, rank_str) in ranks.iter().enumerate() {
            let rank = (BOARD_SIZE - 1 - rank_from_top) as u8;
            let mut file: u8 = 0;
            let mut chars = rank_str.chars().peekable();

            while let Some(&c) = chars.peek() {
                if c == 'x' {
                    // Invalid corner square marker
                    chars.next();
                    file += 1;
                } else if c.is_ascii_digit() {
                    // Empty squares count (could be 1 or 2 digits for 10+)
                    chars.next();
                    let mut num_str = String::from(c);
                    if let Some(&next_c) = chars.peek() {
                        if next_c.is_ascii_digit() {
                            chars.next();
                            num_str.push(next_c);
                        }
                    }
                    let count: u8 = num_str
                        .parse()
                        .map_err(|_| Fen4Error::InvalidNumber(num_str))?;
                    file += count;
                } else if let Some(player) = parse_player_prefix(c) {
                    // Player prefix followed by piece type
                    chars.next();
                    let piece_char = chars
                        .next()
                        .ok_or_else(|| Fen4Error::InvalidPiece(format!("{c}")))?;
                    let piece_type = PieceType::from_fen_char(piece_char)
                        .ok_or_else(|| Fen4Error::InvalidPiece(format!("{c}{piece_char}")))?;

                    if let Some(sq) = square_from(file, rank) {
                        if is_valid_square(sq) {
                            board.place_piece(sq, Piece::new(piece_type, player));
                        }
                    }
                    file += 1;
                } else {
                    return Err(Fen4Error::InvalidPiece(format!("{c}")));
                }
            }
        }

        // Parse side to move
        let side = match fields[1] {
            "r" => Player::Red,
            "b" => Player::Blue,
            "y" => Player::Yellow,
            "g" => Player::Green,
            s => return Err(Fen4Error::InvalidSideToMove(s.to_string())),
        };
        board.set_side_to_move(side);

        // Parse castling rights
        let castling_str = fields[2];
        if castling_str != "-" {
            let mut rights: u8 = 0;
            for c in castling_str.chars() {
                match c {
                    'A' => rights |= CASTLE_RED_KING,
                    'a' => rights |= CASTLE_RED_QUEEN,
                    'B' => rights |= CASTLE_BLUE_KING,
                    'b' => rights |= CASTLE_BLUE_QUEEN,
                    'C' => rights |= CASTLE_YELLOW_KING,
                    'c' => rights |= CASTLE_YELLOW_QUEEN,
                    'D' => rights |= CASTLE_GREEN_KING,
                    'd' => rights |= CASTLE_GREEN_QUEEN,
                    _ => return Err(Fen4Error::InvalidCastling(castling_str.to_string())),
                }
            }
            board.set_castling_rights(rights);
        }

        // Parse en passant
        if fields[3] != "-" {
            let ep_file = fields[3]
                .chars()
                .next()
                .and_then(super::square::parse_file)
                .ok_or_else(|| Fen4Error::InvalidEnPassant(fields[3].to_string()))?;
            board.set_en_passant(Some(ep_file));
        }

        // Parse halfmove clock
        let hmc: u16 = fields[4]
            .parse()
            .map_err(|_| Fen4Error::InvalidNumber(fields[4].to_string()))?;
        board.set_halfmove_clock(hmc);

        // Parse fullmove number
        let fmn: u16 = fields[5]
            .parse()
            .map_err(|_| Fen4Error::InvalidNumber(fields[5].to_string()))?;
        board.set_fullmove_number(fmn);

        Ok(board)
    }

    /// Serialize the board to FEN4 string.
    pub fn to_fen4(&self) -> String {
        let mut result = String::new();

        // Ranks top-to-bottom
        for rank_from_top in 0..BOARD_SIZE {
            let rank = (BOARD_SIZE - 1 - rank_from_top) as u8;
            if rank_from_top > 0 {
                result.push('/');
            }

            let mut empty_count = 0u8;

            for file in 0..BOARD_SIZE as u8 {
                if let Some(sq) = square_from(file, rank) {
                    if !is_valid_square(sq) {
                        // Flush empty count before corner
                        if empty_count > 0 {
                            result.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        result.push('x');
                    } else if let Some(piece) = self.piece_at(sq) {
                        // Flush empty count before piece
                        if empty_count > 0 {
                            result.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        result.push(player_prefix(piece.owner));
                        result.push(piece.piece_type.fen_char());
                    } else {
                        empty_count += 1;
                    }
                }
            }

            // Flush trailing empty count
            if empty_count > 0 {
                result.push_str(&empty_count.to_string());
            }
        }

        // Side to move
        result.push(' ');
        result.push(match self.side_to_move() {
            Player::Red => 'r',
            Player::Blue => 'b',
            Player::Yellow => 'y',
            Player::Green => 'g',
        });

        // Castling
        result.push(' ');
        let c = self.castling_rights();
        if c == 0 {
            result.push('-');
        } else {
            if c & CASTLE_RED_KING != 0 {
                result.push('A');
            }
            if c & CASTLE_RED_QUEEN != 0 {
                result.push('a');
            }
            if c & CASTLE_BLUE_KING != 0 {
                result.push('B');
            }
            if c & CASTLE_BLUE_QUEEN != 0 {
                result.push('b');
            }
            if c & CASTLE_YELLOW_KING != 0 {
                result.push('C');
            }
            if c & CASTLE_YELLOW_QUEEN != 0 {
                result.push('c');
            }
            if c & CASTLE_GREEN_KING != 0 {
                result.push('D');
            }
            if c & CASTLE_GREEN_QUEEN != 0 {
                result.push('d');
            }
        }

        // En passant
        result.push(' ');
        match self.en_passant() {
            Some(file) => result.push(file_char(file)),
            None => result.push('-'),
        }

        // Halfmove clock
        result.push(' ');
        result.push_str(&self.halfmove_clock().to_string());

        // Fullmove number
        result.push(' ');
        result.push_str(&self.fullmove_number().to_string());

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starting_position_fen4_roundtrip() {
        let board = Board::starting_position();
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).expect("should parse");
        let fen2 = parsed.to_fen4();
        assert_eq!(fen, fen2, "FEN4 round-trip failed");
    }

    #[test]
    fn test_starting_position_fen4_zobrist_roundtrip() {
        let board = Board::starting_position();
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).expect("should parse");
        assert_eq!(
            board.zobrist(),
            parsed.zobrist(),
            "Zobrist mismatch after FEN4 round-trip"
        );
    }

    #[test]
    fn test_starting_position_fen4_piece_count() {
        let board = Board::starting_position();
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).expect("should parse");
        assert_eq!(parsed.piece_count(), 64);
    }

    #[test]
    fn test_empty_board_fen4_roundtrip() {
        let board = Board::empty();
        let fen = board.to_fen4();
        let parsed = Board::from_fen4(&fen).expect("should parse");
        assert_eq!(board.zobrist(), parsed.zobrist());
        assert_eq!(parsed.piece_count(), 0);
    }

    #[test]
    fn test_fen4_side_to_move() {
        let mut board = Board::empty();
        board.set_side_to_move(Player::Blue);
        let fen = board.to_fen4();
        assert!(fen.contains(" b "), "FEN should contain side=b");
        let parsed = Board::from_fen4(&fen).expect("should parse");
        assert_eq!(parsed.side_to_move(), Player::Blue);
    }

    #[test]
    fn test_fen4_en_passant() {
        let mut board = Board::empty();
        board.set_en_passant(Some(5)); // file f
        let fen = board.to_fen4();
        assert!(fen.contains(" f "), "FEN should contain ep=f");
        let parsed = Board::from_fen4(&fen).expect("should parse");
        assert_eq!(parsed.en_passant(), Some(5));
    }

    #[test]
    fn test_fen4_invalid_input() {
        assert!(Board::from_fen4("").is_err());
        assert!(Board::from_fen4("bad input").is_err());
    }
}
