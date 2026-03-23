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
    // TODO: Implement king attack generation
    // This would typically use a pre-computed lookup table
    let _ = square;
    0
}

/// Generate queen attack bitboard for a given square and occupancy
pub fn queen_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    rook_attacks(square, occupancy) | bishop_attacks(square, occupancy)
}

/// Generate rook attack bitboard for a given square and occupancy
pub fn rook_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    // TODO: Implement rook attack generation
    // This would typically use magic bitboards or similar technique
    let _ = (square, occupancy);
    0
}

/// Generate bishop attack bitboard for a given square and occupancy
pub fn bishop_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    // TODO: Implement bishop attack generation
    // This would typically use magic bitboards or similar technique
    let _ = (square, occupancy);
    0
}

/// Generate knight attack bitboard for a given square
pub fn knight_attacks(square: Square) -> Bitboard {
    // TODO: Implement knight attack generation
    // This would typically use a pre-computed lookup table
    let _ = square;
    0
}

/// Generate pawn attack bitboard for a given square and color
pub fn pawn_attacks(square: Square, color: Color) -> Bitboard {
    // TODO: Implement pawn attack generation
    // This would typically use a pre-computed lookup table
    let _ = (square, color);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
