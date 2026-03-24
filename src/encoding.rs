//! Position canonicalization and deterministic encoding.

use crate::types::{Bitboard, Color, Square};

const MAX_TB_PIECES: u32 = 7;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PositionInput {
    pub(crate) white: Bitboard,
    pub(crate) black: Bitboard,
    pub(crate) kings: Bitboard,
    pub(crate) queens: Bitboard,
    pub(crate) rooks: Bitboard,
    pub(crate) bishops: Bitboard,
    pub(crate) knights: Bitboard,
    pub(crate) pawns: Bitboard,
    pub(crate) ep: Square,
    pub(crate) turn: Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedPosition {
    pub(crate) material_key: String,
    pub(crate) key: u64,
    /// True if white and black were swapped to produce the canonical key.
    /// Callers must flip WDL (Win<->Loss) and negate DTZ when this is set.
    pub(crate) color_flipped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EncodeError {
    SideOverlap,
    KingsNotTwo,
    InvalidPieceUnion,
    TooManyPieces,
}

pub(crate) fn encode_position(input: PositionInput) -> Result<EncodedPosition, EncodeError> {
    validate_position(input)?;

    // Syzygy file names use a canonical ordering: the side with more pieces
    // comes first; if equal, the side with stronger pieces comes first.
    // When we flip to reach canonical form we must record it so callers can
    // invert the WDL result (Win<->Loss) and negate DTZ.
    let white_strength = material_strength(input, input.white);
    let black_strength = material_strength(input, input.black);
    let color_flipped = black_strength > white_strength;

    let canonical = if color_flipped {
        flip_colors(input)
    } else {
        input
    };

    let material_key = material_key(canonical);
    let key = hash_position(canonical);

    Ok(EncodedPosition {
        material_key,
        key,
        color_flipped,
    })
}

/// Swap white/black sides so the position can be re-encoded canonically.
fn flip_colors(input: PositionInput) -> PositionInput {
    PositionInput {
        white: input.black,
        black: input.white,
        kings: input.kings,
        queens: input.queens,
        rooks: input.rooks,
        bishops: input.bishops,
        knights: input.knights,
        pawns: input.pawns,
        ep: input.ep,
        turn: match input.turn {
            Color::White => Color::Black,
            Color::Black => Color::White,
        },
    }
}

/// Compute a material strength score for a side's pieces.
/// Higher values mean a stronger/larger army (used only for canonical ordering).
fn material_strength(input: PositionInput, side: Bitboard) -> u32 {
    let piece_count =
        (input.kings | input.queens | input.rooks | input.bishops | input.knights | input.pawns)
            .count_ones();
    let own_count =
        ((input.kings | input.queens | input.rooks | input.bishops | input.knights | input.pawns)
            & side)
            .count_ones();
    // Primary: piece count. Secondary: piece quality (Q>R>B>N>P).
    let quality = (input.queens & side).count_ones() * 9
        + (input.rooks & side).count_ones() * 5
        + (input.bishops & side).count_ones() * 3
        + (input.knights & side).count_ones() * 3
        + (input.pawns & side).count_ones();
    let _ = piece_count;
    own_count * 100 + quality
}

fn validate_position(input: PositionInput) -> Result<(), EncodeError> {
    if (input.white & input.black) != 0 {
        return Err(EncodeError::SideOverlap);
    }

    if input.kings.count_ones() != 2 {
        return Err(EncodeError::KingsNotTwo);
    }

    let pieces =
        input.kings | input.queens | input.rooks | input.bishops | input.knights | input.pawns;

    if pieces != (input.white | input.black) {
        return Err(EncodeError::InvalidPieceUnion);
    }

    if pieces.count_ones() > MAX_TB_PIECES {
        return Err(EncodeError::TooManyPieces);
    }

    Ok(())
}

fn material_key(input: PositionInput) -> String {
    let w = side_material(input, input.white);
    let b = side_material(input, input.black);
    format!("{}v{}", w, b).to_ascii_lowercase()
}

fn side_material(input: PositionInput, side: Bitboard) -> String {
    let mut s = String::from("K");
    append_repeated(&mut s, 'Q', (input.queens & side).count_ones());
    append_repeated(&mut s, 'R', (input.rooks & side).count_ones());
    append_repeated(&mut s, 'B', (input.bishops & side).count_ones());
    append_repeated(&mut s, 'N', (input.knights & side).count_ones());
    append_repeated(&mut s, 'P', (input.pawns & side).count_ones());
    s
}

fn append_repeated(out: &mut String, ch: char, count: u32) {
    for _ in 0..count {
        out.push(ch);
    }
}

fn hash_position(input: PositionInput) -> u64 {
    // Stable FNV-1a style hash over the raw position representation.
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    fn mix(h: &mut u64, v: u64) {
        *h ^= v;
        *h = h.wrapping_mul(0x0000_0001_0000_01b3);
    }

    mix(&mut h, input.white);
    mix(&mut h, input.black);
    mix(&mut h, input.kings);
    mix(&mut h, input.queens);
    mix(&mut h, input.rooks);
    mix(&mut h, input.bishops);
    mix(&mut h, input.knights);
    mix(&mut h, input.pawns);
    mix(&mut h, input.ep as u64);
    mix(&mut h, input.turn as u64);

    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_encoding() {
        let pos = PositionInput {
            white: 0x11,
            black: 0x80,
            kings: 0x81,
            queens: 0x10,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: 0,
            ep: 0,
            turn: Color::White,
        };
        let a = encode_position(pos).expect("position should encode");
        let b = encode_position(pos).expect("position should encode");
        assert_eq!(a, b);
        assert_eq!(a.material_key, "kqvk");
        assert!(!a.color_flipped);
    }

    #[test]
    fn canonical_key_when_black_is_stronger() {
        // Black has the queen (bit 4), white just has king (bit 0), black king (bit 7).
        // pieces = kings | queens = 0x91 but we need exactly 2 kings.
        // white=king on a1 (bit 0), black=king on h1 (bit 7) + queen on e1 (bit 4).
        let white = 0x01u64; // bit 0 = a1
        let black = 0x90u64; // bit 4 = e1, bit 7 = h1
        let kings = 0x81u64; // bit 0 = white king, bit 7 = black king
        let queens = 0x10u64; // bit 4 = black queen
                              // validate: kings | queens = 0x91, white | black = 0x91 ✓, kings.count = 2 ✓
        let pos = PositionInput {
            white,
            black,
            kings,
            queens,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: 0,
            ep: 0,
            turn: Color::White,
        };
        let enc = encode_position(pos).expect("position should encode");
        assert_eq!(enc.material_key, "kqvk");
        assert!(enc.color_flipped);
    }

    #[test]
    fn rejects_overlapping_sides() {
        let pos = PositionInput {
            white: 0x1,
            black: 0x1,
            kings: 0x1,
            queens: 0,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: 0,
            ep: 0,
            turn: Color::White,
        };
        let err = encode_position(pos).expect_err("position should be invalid");
        assert_eq!(err, EncodeError::SideOverlap);
    }

    #[test]
    fn rejects_missing_or_extra_kings() {
        let pos = PositionInput {
            white: 0x11,
            black: 0x80,
            kings: 0x01,
            queens: 0x10,
            rooks: 0,
            bishops: 0,
            knights: 0,
            pawns: 0,
            ep: 0,
            turn: Color::White,
        };
        let err = encode_position(pos).expect_err("position should be invalid");
        assert_eq!(err, EncodeError::KingsNotTwo);
    }
}
