//! Bitboard utility functions

use crate::types::{Bitboard, Square};

/// Count the number of set bits in a bitboard
#[inline]
pub fn pop_count(bb: Bitboard) -> u32 {
    bb.count_ones()
}

/// Get the index of the least significant bit
#[inline]
pub fn lsb(bb: Bitboard) -> Square {
    bb.trailing_zeros() as Square
}

/// Pop (remove and return) the least significant bit
#[inline]
pub fn pop_lsb(bb: Bitboard) -> (Bitboard, Square) {
    let square = lsb(bb);
    (bb & (bb - 1), square)
}

/// Get a bitboard with only the least significant bit set
#[inline]
pub fn isolate_lsb(bb: Bitboard) -> Bitboard {
    bb & bb.wrapping_neg()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pop_count() {
        assert_eq!(pop_count(0), 0);
        assert_eq!(pop_count(1), 1);
        assert_eq!(pop_count(0xFF), 8);
        assert_eq!(pop_count(0xFFFF_FFFF_FFFF_FFFF), 64);
    }

    #[test]
    fn test_lsb() {
        assert_eq!(lsb(1), 0);
        assert_eq!(lsb(2), 1);
        assert_eq!(lsb(0x100), 8);
    }

    #[test]
    fn test_pop_lsb() {
        let (bb, sq) = pop_lsb(0b1010);
        assert_eq!(sq, 1);
        assert_eq!(bb, 0b1000);
    }
}
