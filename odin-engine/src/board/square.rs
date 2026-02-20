// Square indexing and validity for the 14x14 four-player chess board.
//
// Files: a=0 through n=13. Ranks: 1=0 through 14=13.
// Index formula: rank * 14 + file (0..196).
// Four 3x3 corners are invalid (36 squares removed, 160 valid).
//
// Invalid corners (0-indexed file, rank):
//   Bottom-left:  files 0-2,  ranks 0-2   (a1-c3)
//   Bottom-right: files 11-13, ranks 0-2  (l1-n3)
//   Top-left:     files 0-2,  ranks 11-13 (a12-c14)
//   Top-right:    files 11-13, ranks 11-13 (l12-n14)

/// Board dimension (14x14).
pub const BOARD_SIZE: usize = 14;

/// Total squares in the 14x14 grid.
pub const TOTAL_SQUARES: usize = BOARD_SIZE * BOARD_SIZE;

/// Number of valid (playable) squares.
pub const VALID_SQUARE_COUNT: usize = 160;

/// Number of invalid corner squares (4 corners x 9 each).
pub const INVALID_CORNER_COUNT: usize = 36;

/// A square index (0..196). Not all indices are valid board squares.
pub type Square = u8;

/// Pre-computed validity lookup table. `true` = valid playable square.
static VALID_SQUARES: [bool; TOTAL_SQUARES] = {
    let mut table = [true; TOTAL_SQUARES];
    let mut rank = 0usize;
    while rank < BOARD_SIZE {
        let mut file = 0usize;
        while file < BOARD_SIZE {
            let idx = rank * BOARD_SIZE + file;
            // Bottom-left corner: files 0-2, ranks 0-2
            if file <= 2 && rank <= 2 {
                table[idx] = false;
            }
            // Bottom-right corner: files 11-13, ranks 0-2
            if file >= 11 && rank <= 2 {
                table[idx] = false;
            }
            // Top-left corner: files 0-2, ranks 11-13
            if file <= 2 && rank >= 11 {
                table[idx] = false;
            }
            // Top-right corner: files 11-13, ranks 11-13
            if file >= 11 && rank >= 11 {
                table[idx] = false;
            }
            file += 1;
        }
        rank += 1;
    }
    table
};

/// Extract the file (column, 0-13) from a square index.
#[inline]
pub fn file_of(sq: Square) -> u8 {
    sq % BOARD_SIZE as u8
}

/// Extract the rank (row, 0-13) from a square index.
#[inline]
pub fn rank_of(sq: Square) -> u8 {
    sq / BOARD_SIZE as u8
}

/// Create a square index from file and rank (both 0-indexed).
/// Returns `None` if out of bounds.
#[inline]
pub fn square_from(file: u8, rank: u8) -> Option<Square> {
    if file < BOARD_SIZE as u8 && rank < BOARD_SIZE as u8 {
        Some(rank * BOARD_SIZE as u8 + file)
    } else {
        None
    }
}

/// Check if a square index is a valid playable square.
#[inline]
pub fn is_valid_square(sq: Square) -> bool {
    (sq as usize) < TOTAL_SQUARES && VALID_SQUARES[sq as usize]
}

/// Iterator over all valid squares (0..196, skipping invalid corners).
pub fn valid_squares() -> impl Iterator<Item = Square> {
    (0..TOTAL_SQUARES as u8).filter(|&sq| is_valid_square(sq))
}

/// Convert file index to chess file letter (a-n).
pub fn file_char(file: u8) -> char {
    (b'a' + file) as char
}

/// Convert rank index to chess rank string (1-14).
pub fn rank_number(rank: u8) -> u8 {
    rank + 1
}

/// Parse a file character ('a'-'n') to file index (0-13).
pub fn parse_file(c: char) -> Option<u8> {
    if ('a'..='n').contains(&c) {
        Some(c as u8 - b'a')
    } else {
        None
    }
}

/// Parse a square string like "d4" or "k14" to a square index.
pub fn parse_square(s: &str) -> Option<Square> {
    let mut chars = s.chars();
    let file_c = chars.next()?;
    let file = parse_file(file_c)?;
    let rank_str: String = chars.collect();
    let rank_1based: u8 = rank_str.parse().ok()?;
    if !(1..=14).contains(&rank_1based) {
        return None;
    }
    let rank = rank_1based - 1;
    let sq = square_from(file, rank)?;
    if is_valid_square(sq) {
        Some(sq)
    } else {
        None
    }
}

/// Convert a square index to a string like "d4" or "k14".
pub fn square_name(sq: Square) -> String {
    let file = file_of(sq);
    let rank = rank_of(sq);
    format!("{}{}", file_char(file), rank_number(rank))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_square_count() {
        let count = (0..TOTAL_SQUARES as u8)
            .filter(|&sq| is_valid_square(sq))
            .count();
        assert_eq!(count, VALID_SQUARE_COUNT);
    }

    #[test]
    fn test_invalid_corner_count() {
        let invalid = (0..TOTAL_SQUARES as u8)
            .filter(|&sq| !is_valid_square(sq))
            .count();
        assert_eq!(invalid, INVALID_CORNER_COUNT);
    }

    #[test]
    fn test_bottom_left_corner_invalid() {
        // a1-c3: files 0-2, ranks 0-2
        for file in 0..=2u8 {
            for rank in 0..=2u8 {
                let sq = square_from(file, rank).unwrap();
                assert!(!is_valid_square(sq), "({file},{rank}) should be invalid");
            }
        }
    }

    #[test]
    fn test_bottom_right_corner_invalid() {
        // l1-n3: files 11-13, ranks 0-2
        for file in 11..=13u8 {
            for rank in 0..=2u8 {
                let sq = square_from(file, rank).unwrap();
                assert!(!is_valid_square(sq), "({file},{rank}) should be invalid");
            }
        }
    }

    #[test]
    fn test_top_left_corner_invalid() {
        // a12-c14: files 0-2, ranks 11-13
        for file in 0..=2u8 {
            for rank in 11..=13u8 {
                let sq = square_from(file, rank).unwrap();
                assert!(!is_valid_square(sq), "({file},{rank}) should be invalid");
            }
        }
    }

    #[test]
    fn test_top_right_corner_invalid() {
        // l12-n14: files 11-13, ranks 11-13
        for file in 11..=13u8 {
            for rank in 11..=13u8 {
                let sq = square_from(file, rank).unwrap();
                assert!(!is_valid_square(sq), "({file},{rank}) should be invalid");
            }
        }
    }

    #[test]
    fn test_center_squares_valid() {
        // The center 8x8 area (files 3-10, ranks 3-10) should all be valid
        for file in 3..=10u8 {
            for rank in 3..=10u8 {
                let sq = square_from(file, rank).unwrap();
                assert!(is_valid_square(sq), "({file},{rank}) should be valid");
            }
        }
    }

    #[test]
    fn test_edge_adjacent_to_corner_valid() {
        // d1 (file 3, rank 0) should be valid — just outside bottom-left corner
        let sq = square_from(3, 0).unwrap();
        assert!(is_valid_square(sq));

        // a4 (file 0, rank 3) should be valid — just outside bottom-left corner
        let sq = square_from(0, 3).unwrap();
        assert!(is_valid_square(sq));
    }

    #[test]
    fn test_file_rank_roundtrip() {
        for file in 0..14u8 {
            for rank in 0..14u8 {
                let sq = square_from(file, rank).unwrap();
                assert_eq!(file_of(sq), file);
                assert_eq!(rank_of(sq), rank);
            }
        }
    }

    #[test]
    fn test_square_out_of_bounds() {
        assert!(square_from(14, 0).is_none());
        assert!(square_from(0, 14).is_none());
        assert!(!is_valid_square(196));
    }

    #[test]
    fn test_parse_square_valid() {
        assert_eq!(parse_square("d1"), Some(square_from(3, 0).unwrap()));
        assert_eq!(parse_square("a4"), Some(square_from(0, 3).unwrap()));
        assert_eq!(parse_square("n11"), Some(square_from(13, 10).unwrap()));
        assert_eq!(parse_square("k14"), Some(square_from(10, 13).unwrap()));
    }

    #[test]
    fn test_parse_square_invalid_corner() {
        assert_eq!(parse_square("a1"), None); // invalid corner
        assert_eq!(parse_square("c3"), None); // invalid corner
        assert_eq!(parse_square("n14"), None); // invalid corner
    }

    #[test]
    fn test_square_name_roundtrip() {
        for sq in valid_squares() {
            let name = square_name(sq);
            let parsed = parse_square(&name);
            assert_eq!(parsed, Some(sq), "roundtrip failed for {name}");
        }
    }
}
