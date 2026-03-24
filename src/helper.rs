//! Helper API for bitboard operations
//!
//! This module provides utility functions for working with bitboards,
//! including attack generation for various pieces.

use crate::types::{Bitboard, Square, Color};

/// Count set bits in a bitboard
pub fn pop_count(bb: Bitboard) -> u32 {
    crate::bitboard::pop_count(bb)
}

/// Get the least significant bit position
pub fn lsb(bb: Bitboard) -> Square {
    crate::bitboard::lsb(bb)
}

/// Pop and return the least significant bit
pub fn pop_lsb(bb: Bitboard) -> (Bitboard, Square) {
    crate::bitboard::pop_lsb(bb)
}

/// Generate king attack bitboard for a given square
pub fn king_attacks(square: Square) -> Bitboard {
    if square >= 64 {
        return 0;
    }

    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;
    let mut attacks = 0;

    for (df, dr) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        let nf = file + df;
        let nr = rank + dr;
        if (0..8).contains(&nf) && (0..8).contains(&nr) {
            attacks |= 1u64 << (nr * 8 + nf);
        }
    }

    attacks
}

/// Generate queen attack bitboard for a given square and occupancy
pub fn queen_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    rook_attacks(square, occupancy) | bishop_attacks(square, occupancy)
}

/// Generate rook attack bitboard for a given square and occupancy
pub fn rook_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    if square >= 64 {
        return 0;
    }

    let mut attacks = 0;
    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;

    for (df, dr) in [(0, 1), (0, -1), (1, 0), (-1, 0)] {
        let mut nf = file + df;
        let mut nr = rank + dr;

        while (0..8).contains(&nf) && (0..8).contains(&nr) {
            let sq = (nr * 8 + nf) as u8;
            let bit = 1u64 << sq;
            attacks |= bit;
            if occupancy & bit != 0 {
                break;
            }
            nf += df;
            nr += dr;
        }
    }

    attacks
}

/// Generate bishop attack bitboard for a given square and occupancy
pub fn bishop_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    if square >= 64 {
        return 0;
    }

    let mut attacks = 0;
    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;

    for (df, dr) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
        let mut nf = file + df;
        let mut nr = rank + dr;

        while (0..8).contains(&nf) && (0..8).contains(&nr) {
            let sq = (nr * 8 + nf) as u8;
            let bit = 1u64 << sq;
            attacks |= bit;
            if occupancy & bit != 0 {
                break;
            }
            nf += df;
            nr += dr;
        }
    }

    attacks
}

/// Generate knight attack bitboard for a given square
pub fn knight_attacks(square: Square) -> Bitboard {
    if square >= 64 {
        return 0;
    }

    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;
    let mut attacks = 0;

    for (df, dr) in [
        (-2, -1),
        (-2, 1),
        (-1, -2),
        (-1, 2),
        (1, -2),
        (1, 2),
        (2, -1),
        (2, 1),
    ] {
        let nf = file + df;
        let nr = rank + dr;
        if (0..8).contains(&nf) && (0..8).contains(&nr) {
            attacks |= 1u64 << (nr * 8 + nf);
        }
    }

    attacks
}

/// Generate pawn attack bitboard for a given square and color
pub fn pawn_attacks(square: Square, color: Color) -> Bitboard {
    if square >= 64 {
        return 0;
    }

    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;
    let direction = match color {
        Color::White => 1,
        Color::Black => -1,
    };

    let next_rank = rank + direction;
    if !(0..8).contains(&next_rank) {
        return 0;
    }

    let mut attacks = 0;
    for df in [-1, 1] {
        let nf = file + df;
        if (0..8).contains(&nf) {
            attacks |= 1u64 << (next_rank * 8 + nf);
        }
    }

    attacks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bb(squares: &[u8]) -> Bitboard {
        squares.iter().fold(0, |acc, &sq| acc | (1u64 << sq))
    }

    #[test]
    fn test_pop_count() {
        assert_eq!(pop_count(0), 0);
        assert_eq!(pop_count(0xFF), 8);
    }

    #[test]
    fn test_lsb() {
        assert_eq!(lsb(1), 0);
        assert_eq!(lsb(2), 1);
    }

    #[test]
    fn test_king_attacks_center_and_corner() {
        assert_eq!(king_attacks(27), bb(&[18, 19, 20, 26, 28, 34, 35, 36]));
        assert_eq!(king_attacks(0), bb(&[1, 8, 9]));
    }

    #[test]
    fn test_knight_attacks_center() {
        assert_eq!(knight_attacks(27), bb(&[10, 12, 17, 21, 33, 37, 42, 44]));
    }

    #[test]
    fn test_pawn_attacks() {
        assert_eq!(pawn_attacks(27, Color::White), bb(&[34, 36]));
        assert_eq!(pawn_attacks(27, Color::Black), bb(&[18, 20]));
        assert_eq!(pawn_attacks(8, Color::White), bb(&[17]));
        assert_eq!(pawn_attacks(55, Color::Black), bb(&[46]));
    }

    #[test]
    fn test_rook_attacks_with_blockers() {
        let occupancy = bb(&[11, 24, 30, 43]);
        let expected = bb(&[11, 19, 24, 25, 26, 28, 29, 30, 35, 43]);
        assert_eq!(rook_attacks(27, occupancy), expected);
    }

    #[test]
    fn test_bishop_attacks_with_blockers() {
        let occupancy = bb(&[9, 13, 41, 45]);
        let expected = bb(&[9, 13, 18, 20, 34, 36, 41, 45]);
        assert_eq!(bishop_attacks(27, occupancy), expected);
    }

    #[test]
    fn test_queen_attacks_combines_rook_and_bishop() {
        let occupancy = bb(&[11, 24, 30, 43, 9, 13, 41, 45]);
        assert_eq!(
            queen_attacks(27, occupancy),
            rook_attacks(27, occupancy) | bishop_attacks(27, occupancy)
        );
    }
}
