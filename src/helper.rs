//! Helper API for bitboard operations
//!
//! This module provides utility functions for working with bitboards,
//! including attack generation for various pieces.

use crate::types::{Bitboard, Color, Square};

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

// --- Move generation and attack calculation are not implemented here ---
// This crate is intended to be used only with the Reckless engine, which provides
// all move generation and attack calculation functionality. Do not use these stubs directly.

/// See Reckless engine for implementation.
#[allow(unused_variables)]
pub fn king_attacks(square: Square) -> Bitboard {
    panic!("king_attacks must be provided by Reckless engine");
}

/// See Reckless engine for implementation.
#[allow(unused_variables)]
pub fn knight_attacks(square: Square) -> Bitboard {
    panic!("knight_attacks must be provided by Reckless engine");
}

/// See Reckless engine for implementation.
#[allow(unused_variables)]
pub fn pawn_attacks(square: Square, color: Color) -> Bitboard {
    panic!("pawn_attacks must be provided by Reckless engine");
}

/// See Reckless engine for implementation.
#[allow(unused_variables)]
pub fn queen_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    panic!("queen_attacks must be provided by Reckless engine");
}

/// See Reckless engine for implementation.
#[allow(unused_variables)]
pub fn rook_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    panic!("rook_attacks must be provided by Reckless engine");
}

/// See Reckless engine for implementation.
#[allow(unused_variables)]
pub fn bishop_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    panic!("bishop_attacks must be provided by Reckless engine");
}
