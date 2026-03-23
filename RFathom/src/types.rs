//! Type definitions for RFathom

use crate::constants::*;
use crate::constants::{result_masks, result_shifts};

/// A bitboard representing piece positions
pub type Bitboard = u64;

/// A square on the chess board (0-63)
pub type Square = u8;

/// A chess move encoded as a 16-bit value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move(u16);

impl Move {
    /// Create a new move from raw value
    pub fn from_raw(value: u16) -> Self {
        Move(value)
    }

    /// Get raw move value
    pub fn raw(&self) -> u16 {
        self.0
    }

    /// Get the source square (6 bits, shifted right by 6)
    pub fn from_square(&self) -> Square {
        ((self.0 >> 6) & 0x3F) as u8
    }

    /// Get the destination square (6 bits)
    pub fn to_square(&self) -> Square {
        (self.0 & 0x3F) as u8
    }

    /// Get the promotion piece (3 bits, shifted right by 12)
    pub fn promotion(&self) -> Promotion {
        Promotion::from_u32(((self.0 >> 12) & 0x7) as u32).unwrap_or(Promotion::None)
    }

    /// Create a move from components
    pub fn new(from: Square, to: Square, promotion: Promotion) -> Self {
        let mut value = (to & 0x3F) as u16;
        value |= ((from & 0x3F) as u16) << 6;
        value |= ((promotion as u16) & 0x7) << 12;
        Move(value)
    }
}

/// Result of a tablebase probe
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeResult(u32);

impl ProbeResult {
    /// Special result indicating checkmate
    pub const CHECKMATE: ProbeResult = ProbeResult((WdlValue::Win as u32) << result_shifts::WDL_SHIFT);
    
    /// Special result indicating stalemate
    pub const STALEMATE: ProbeResult = ProbeResult((WdlValue::Draw as u32) << result_shifts::WDL_SHIFT);
    
    /// Special result indicating probe failure
    pub const FAILED: ProbeResult = ProbeResult(0xFFFF_FFFF);

    /// Create a new probe result from raw value
    pub fn from_raw(value: u32) -> Self {
        ProbeResult(value)
    }

    /// Get raw result value
    pub fn raw(&self) -> u32 {
        self.0
    }

    /// Check if the probe failed
    pub fn is_failed(&self) -> bool {
        self.0 == 0xFFFF_FFFF
    }

    /// Check if the position is checkmate
    pub fn is_checkmate(&self) -> bool {
        *self == Self::CHECKMATE
    }

    /// Check if the position is stalemate
    pub fn is_stalemate(&self) -> bool {
        *self == Self::STALEMATE
    }

    /// Extract the WDL value
    pub fn wdl(&self) -> Option<WdlValue> {
        let value = (self.0 & result_masks::WDL_MASK) >> result_shifts::WDL_SHIFT;
        WdlValue::from_u32(value)
    }

    /// Extract the destination square
    pub fn to_square(&self) -> Square {
        ((self.0 & result_masks::TO_MASK) >> result_shifts::TO_SHIFT) as u8
    }

    /// Extract the source square
    pub fn from_square(&self) -> Square {
        ((self.0 & result_masks::FROM_MASK) >> result_shifts::FROM_SHIFT) as u8
    }

    /// Extract the promotion piece
    pub fn promotion(&self) -> Promotion {
        let value = (self.0 & result_masks::PROMOTES_MASK) >> result_shifts::PROMOTES_SHIFT;
        Promotion::from_u32(value).unwrap_or(Promotion::None)
    }

    /// Check if the move is en passant
    pub fn is_en_passant(&self) -> bool {
        ((self.0 & result_masks::EP_MASK) >> result_shifts::EP_SHIFT) != 0
    }

    /// Extract the DTZ (Distance-To-Zero) value
    pub fn dtz(&self) -> i32 {
        let value = (self.0 & result_masks::DTZ_MASK) >> result_shifts::DTZ_SHIFT;
        // Sign extend from 12-bit value
        if value & 0x800 != 0 {
            (value | 0xFFFF_F000) as i32
        } else {
            value as i32
        }
    }

    /// Create a new result with WDL value
    pub fn with_wdl(mut self, wdl: WdlValue) -> Self {
        self.0 = (self.0 & !result_masks::WDL_MASK) | 
                 ((wdl as u32) << result_shifts::WDL_SHIFT & result_masks::WDL_MASK);
        self
    }

    /// Create a new result with destination square
    pub fn with_to(mut self, to: Square) -> Self {
        self.0 = (self.0 & !result_masks::TO_MASK) | 
                 ((to as u32) << result_shifts::TO_SHIFT & result_masks::TO_MASK);
        self
    }

    /// Create a new result with source square
    pub fn with_from(mut self, from: Square) -> Self {
        self.0 = (self.0 & !result_masks::FROM_MASK) | 
                 ((from as u32) << result_shifts::FROM_SHIFT & result_masks::FROM_MASK);
        self
    }

    /// Create a new result with promotion
    pub fn with_promotion(mut self, promotes: Promotion) -> Self {
        self.0 = (self.0 & !result_masks::PROMOTES_MASK) | 
                 ((promotes as u32) << result_shifts::PROMOTES_SHIFT & result_masks::PROMOTES_MASK);
        self
    }

    /// Create a new result with en passant flag
    pub fn with_ep(mut self, ep: bool) -> Self {
        self.0 = (self.0 & !result_masks::EP_MASK) | 
                 ((ep as u32) << result_shifts::EP_SHIFT & result_masks::EP_MASK);
        self
    }

    /// Create a new result with DTZ value
    pub fn with_dtz(mut self, dtz: i32) -> Self {
        let value = (dtz as u32) & 0xFFF; // 12-bit signed value
        self.0 = (self.0 & !result_masks::DTZ_MASK) | 
                 (value << result_shifts::DTZ_SHIFT & result_masks::DTZ_MASK);
        self
    }
}

/// Information about a root move with tablebase data
#[derive(Debug, Clone)]
pub struct RootMove {
    /// The move
    pub mv: Move,
    /// Principal variation
    pub pv: Vec<Move>,
    /// Tablebase score
    pub tb_score: i32,
    /// Tablebase rank
    pub tb_rank: i32,
}

impl RootMove {
    /// Create a new root move
    pub fn new(mv: Move) -> Self {
        RootMove {
            mv,
            pv: Vec::with_capacity(MAX_PLY),
            tb_score: 0,
            tb_rank: 0,
        }
    }
}

/// Collection of root moves with tablebase information
#[derive(Debug, Clone)]
pub struct RootMoves {
    /// List of moves
    pub moves: Vec<RootMove>,
}

impl RootMoves {
    /// Create a new empty root moves collection
    pub fn new() -> Self {
        RootMoves {
            moves: Vec::with_capacity(MAX_MOVES),
        }
    }

    /// Get the number of moves
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Add a move
    pub fn push(&mut self, mv: RootMove) {
        self.moves.push(mv);
    }

    /// Get an iterator over moves
    pub fn iter(&self) -> std::slice::Iter<'_, RootMove> {
        self.moves.iter()
    }
}

impl Default for RootMoves {
    fn default() -> Self {
        Self::new()
    }
}

/// Side to move
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    /// Create color from boolean (true = white, false = black)
    pub fn from_bool(b: bool) -> Self {
        if b { Color::White } else { Color::Black }
    }

    /// Convert to boolean (true = white, false = black)
    pub fn to_bool(self) -> bool {
        matches!(self, Color::White)
    }
}
