//! Helper API for bitboard operations
//!
//! This module provides utility functions for working with bitboards,
//! including attack generation for various pieces.

use crate::types::{Bitboard, Color, Square};
use once_cell::sync::Lazy;

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

// --- Static attack tables for non-sliding pieces ---

const BOARD_SIZE: usize = 64;

fn king_attack_mask(square: Square) -> Bitboard {
    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;
    let mut attacks = 0u64;
    for dr in -1..=1 {
        for df in -1..=1 {
            if dr == 0 && df == 0 {
                continue;
            }
            let nf = file + df;
            let nr = rank + dr;
            if (0..8).contains(&nf) && (0..8).contains(&nr) {
                let sq = (nr * 8 + nf) as u8;
                attacks |= 1u64 << sq;
            }
        }
    }
    attacks
}

fn knight_attack_mask(square: Square) -> Bitboard {
    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;
    let mut attacks = 0u64;
    for (df, dr) in [
        (-2, -1),
        (-2, 1),
        (2, -1),
        (2, 1),
        (-1, -2),
        (-1, 2),
        (1, -2),
        (1, 2),
    ] {
        let nf = file + df;
        let nr = rank + dr;
        if (0..8).contains(&nf) && (0..8).contains(&nr) {
            let sq = (nr * 8 + nf) as u8;
            attacks |= 1u64 << sq;
        }
    }
    attacks
}

fn pawn_attack_mask(square: Square, color: Color) -> Bitboard {
    let file = (square % 8) as i8;
    let rank = (square / 8) as i8;
    let mut attacks = 0u64;
    let dirs = match color {
        Color::White => [(-1, 1), (1, 1)],
        Color::Black => [(-1, -1), (1, -1)],
    };
    for (df, dr) in dirs {
        let nf = file + df;
        let nr = rank + dr;
        if (0..8).contains(&nf) && (0..8).contains(&nr) {
            let sq = (nr * 8 + nf) as u8;
            attacks |= 1u64 << sq;
        }
    }
    attacks
}

pub static KING_ATTACKS: Lazy<[Bitboard; BOARD_SIZE]> = Lazy::new(|| {
    let mut table = [0u64; BOARD_SIZE];
    for sq in 0..BOARD_SIZE as u8 {
        table[sq as usize] = king_attack_mask(sq);
    }
    table
});

pub static KNIGHT_ATTACKS: Lazy<[Bitboard; BOARD_SIZE]> = Lazy::new(|| {
    let mut table = [0u64; BOARD_SIZE];
    for sq in 0..BOARD_SIZE as u8 {
        table[sq as usize] = knight_attack_mask(sq);
    }
    table
});

pub static PAWN_ATTACKS_WHITE: Lazy<[Bitboard; BOARD_SIZE]> = Lazy::new(|| {
    let mut table = [0u64; BOARD_SIZE];
    for sq in 0..BOARD_SIZE as u8 {
        table[sq as usize] = pawn_attack_mask(sq, Color::White);
    }
    table
});

pub static PAWN_ATTACKS_BLACK: Lazy<[Bitboard; BOARD_SIZE]> = Lazy::new(|| {
    let mut table = [0u64; BOARD_SIZE];
    for sq in 0..BOARD_SIZE as u8 {
        table[sq as usize] = pawn_attack_mask(sq, Color::Black);
    }
    table
});

/// Returns king attacks from a given square
pub fn king_attacks(square: Square) -> Bitboard {
    KING_ATTACKS[square as usize]
}

/// Returns knight attacks from a given square
pub fn knight_attacks(square: Square) -> Bitboard {
    KNIGHT_ATTACKS[square as usize]
}

/// Returns pawn attacks from a given square and color
pub fn pawn_attacks(square: Square, color: Color) -> Bitboard {
    match color {
        Color::White => PAWN_ATTACKS_WHITE[square as usize],
        Color::Black => PAWN_ATTACKS_BLACK[square as usize],
    }
}

// ...existing code...

// ...existing code...

// ...existing code...

// ...existing code...

// Sliding piece attacks (rook, bishop, queen) remain as-is for now.
pub fn queen_attacks(square: Square, occupancy: Bitboard) -> Bitboard {
    rook_attacks(square, occupancy) | bishop_attacks(square, occupancy)
}

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
