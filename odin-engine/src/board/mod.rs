// Board representation — Stage 1
//
// 14x14 board (196 total squares, 160 valid, 36 invalid corners).
// Flat array storage with per-player piece lists and Zobrist hashing.
// Coordinate system: files a-n (0-13), ranks 1-14 (0-13).
// Invalid corners: a1-c3, l1-n3, a12-c14, l12-n14.

mod board_struct;
mod fen4;
mod square;
mod types;
mod zobrist;

pub use board_struct::Board;
pub use square::{
    file_of, is_valid_square, rank_of, square_from, Square, BOARD_SIZE, INVALID_CORNER_COUNT,
    TOTAL_SQUARES, VALID_SQUARE_COUNT,
};
pub use types::{Piece, PieceStatus, PieceType, Player, PIECE_TYPE_COUNT, PLAYER_COUNT};
pub use zobrist::ZobristKeys;
