//! Constants and configuration values for RFathom

/// Maximum number of moves in a position (including sentinel)
pub const MAX_MOVES: usize = 193;

/// Maximum number of captures
pub const MAX_CAPTURES: usize = 64;

/// Maximum ply depth for principal variation
pub const MAX_PLY: usize = 256;

/// Castling rights flags
pub mod castling {
    /// White king-side castling
    pub const WHITE_KING_SIDE: u32 = 0x1;
    /// White queen-side castling
    pub const WHITE_QUEEN_SIDE: u32 = 0x2;
    /// Black king-side castling
    pub const BLACK_KING_SIDE: u32 = 0x4;
    /// Black queen-side castling
    pub const BLACK_QUEEN_SIDE: u32 = 0x8;
}

/// WDL (Win-Draw-Loss) values
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WdlValue {
    Loss = 0,
    BlessedLoss = 1,
    Draw = 2,
    CursedWin = 3,
    Win = 4,
}

impl WdlValue {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(WdlValue::Loss),
            1 => Some(WdlValue::BlessedLoss),
            2 => Some(WdlValue::Draw),
            3 => Some(WdlValue::CursedWin),
            4 => Some(WdlValue::Win),
            _ => None,
        }
    }
}

/// Promotion piece types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Promotion {
    None = 0,
    Queen = 1,
    Rook = 2,
    Bishop = 3,
    Knight = 4,
}

impl Promotion {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(Promotion::None),
            1 => Some(Promotion::Queen),
            2 => Some(Promotion::Rook),
            3 => Some(Promotion::Bishop),
            4 => Some(Promotion::Knight),
            _ => None,
        }
    }
}

/// Bit masks for extracting fields from result values
pub(crate) mod result_masks {
    pub const WDL_MASK: u32 = 0x0000_000F;
    pub const TO_MASK: u32 = 0x0000_03F0;
    pub const FROM_MASK: u32 = 0x0000_FC00;
    pub const PROMOTES_MASK: u32 = 0x0007_0000;
    pub const EP_MASK: u32 = 0x0008_0000;
    pub const DTZ_MASK: u32 = 0xFFF0_0000;
}

/// Bit shift amounts for result fields
pub(crate) mod result_shifts {
    pub const WDL_SHIFT: u32 = 0;
    pub const TO_SHIFT: u32 = 4;
    pub const FROM_SHIFT: u32 = 10;
    pub const PROMOTES_SHIFT: u32 = 16;
    pub const EP_SHIFT: u32 = 19;
    pub const DTZ_SHIFT: u32 = 20;
}
