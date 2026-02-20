// Move generation + attack query API — Stage 2
//
// Pre-computed attack tables, attack query API (is_square_attacked_by,
// attackers_of), pseudo-legal and legal move generation, make/unmake,
// and perft validation.

mod attacks;
mod generate;
mod moves;
mod tables;

pub use attacks::{attackers_of, is_in_check, is_square_attacked_by};
pub use generate::{generate_legal, generate_pseudo_legal, perft, perft_divide};
pub use moves::{
    make_move, unmake_move, Move, MoveUndo, FLAG_CASTLE_KING, FLAG_CASTLE_QUEEN, FLAG_DOUBLE_PUSH,
    FLAG_EN_PASSANT, FLAG_NORMAL,
};
pub use tables::{
    global_attack_tables, is_diagonal, is_orthogonal, DIR_EAST, DIR_NORTH, DIR_NORTHEAST,
    DIR_NORTHWEST, DIR_SOUTH, DIR_SOUTHEAST, DIR_SOUTHWEST, DIR_WEST, NUM_DIRECTIONS,
};
